//! Key escrow and its failure: the Clipper chip's LEAF, and Matt Blaze's 1994
//! attack on it.
//!
//! In 1993 the US government proposed the Clipper chip: strong encryption for
//! phones, with a copy of every session key for the government. Each call
//! carried a 128-bit Law Enforcement Access Field (LEAF): the chip's unit ID,
//! the session key encrypted under that chip's escrowed unit key, and a 16-bit
//! checksum, all encrypted under a family key shared by every chip. The
//! receiving chip refused to talk unless the LEAF's checksum matched.
//!
//! Blaze noticed that 16 bits is not much. A sender can feed the receiving
//! chip random LEAFs until one passes, about 2^16 tries (42 minutes on the
//! hardware of the day), and then talk with a LEAF that decrypts to garbage:
//! the escrow agents get a unit ID that does not exist and no key.
//!
//! The real chip used the classified SKIPJACK cipher; AES-256 and HMAC-SHA256
//! stand in for it here. The flaw is the 16-bit check, and that is exact.

use aes::Aes256;
use aes::cipher::{BlockCipherDecrypt, BlockCipherEncrypt, KeyInit};
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::aead::kdf;
use crate::bytes::to_hex;
use crate::drbg::Drbg;

pub const LEAF_BYTES: usize = 16;
pub const CHECKSUM_BITS: u32 = 16;

fn aes_block(key: &[u8; 32], block: [u8; 16], decrypt: bool) -> [u8; 16] {
    let c = Aes256::new_from_slice(key).expect("32-byte key");
    let mut b = aes::Block::from(block);
    if decrypt {
        c.decrypt_block(&mut b);
    } else {
        c.encrypt_block(&mut b);
    }
    b.as_slice().try_into().unwrap()
}

/// The 16-bit checksum over the session key and the call's IV.
pub fn checksum(session_key: &[u8; 10], iv: &[u8; 8]) -> [u8; 2] {
    let mut m = <Hmac<Sha256> as KeyInit>::new_from_slice(session_key).expect("any key length");
    m.update(iv);
    let out = m.finalize().into_bytes();
    [out[0], out[1]]
}

fn wrap_mask(unit_key: &[u8; 32]) -> [u8; 10] {
    kdf(unit_key, "clipper unit wrap")[..10].try_into().unwrap()
}

/// One escrowed chip.
#[derive(Clone, Debug)]
pub struct Chip {
    pub unit_id: u32,
    pub unit_key: [u8; 32],
}

/// What the escrow agencies hold: the family key and every chip's unit key.
#[derive(Clone, Debug)]
pub struct Escrow {
    pub family_key: [u8; 32],
    pub chips: Vec<Chip>,
}

/// The LEAF's contents before family-key encryption.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeafFields {
    pub unit_id: u32,
    pub wrapped_key: [u8; 10],
    pub checksum: [u8; 2],
}

impl LeafFields {
    fn to_block(&self) -> [u8; 16] {
        let mut b = [0u8; 16];
        b[..4].copy_from_slice(&self.unit_id.to_be_bytes());
        b[4..14].copy_from_slice(&self.wrapped_key);
        b[14..].copy_from_slice(&self.checksum);
        b
    }

    fn from_block(b: &[u8; 16]) -> Self {
        Self { unit_id: u32::from_be_bytes(b[..4].try_into().unwrap()), wrapped_key: b[4..14].try_into().unwrap(), checksum: [b[14], b[15]] }
    }
}

impl Escrow {
    pub fn new(rng: &mut Drbg, units: usize) -> Self {
        let family_key = rng.array();
        let chips = (0..units).map(|_| Chip { unit_id: 0x0100_0000 | (rng.int(0xff_ffff) as u32), unit_key: rng.array() }).collect();
        Self { family_key, chips }
    }

    /// The LEAF an honest chip attaches to a call.
    pub fn honest_leaf(&self, chip: &Chip, session_key: &[u8; 10], iv: &[u8; 8]) -> [u8; 16] {
        let mask = wrap_mask(&chip.unit_key);
        let mut wrapped = [0u8; 10];
        for i in 0..10 {
            wrapped[i] = session_key[i] ^ mask[i];
        }
        let f = LeafFields { unit_id: chip.unit_id, wrapped_key: wrapped, checksum: checksum(session_key, iv) };
        aes_block(&self.family_key, f.to_block(), false)
    }

    /// The receiving chip's check: decrypt with the family key, recompute the checksum.
    pub fn chip_accepts(&self, leaf: &[u8; 16], session_key: &[u8; 10], iv: &[u8; 8]) -> bool {
        LeafFields::from_block(&aes_block(&self.family_key, *leaf, true)).checksum == checksum(session_key, iv)
    }

    /// What a wiretap with a court order recovers from a LEAF.
    pub fn wiretap(&self, leaf: &[u8; 16]) -> Result<(u32, [u8; 10]), String> {
        let f = LeafFields::from_block(&aes_block(&self.family_key, *leaf, true));
        let Some(chip) = self.chips.iter().find(|c| c.unit_id == f.unit_id) else {
            return Err(format!("unit {:08X} is not in the escrow database", f.unit_id));
        };
        let mask = wrap_mask(&chip.unit_key);
        let mut sk = [0u8; 10];
        for i in 0..10 {
            sk[i] = f.wrapped_key[i] ^ mask[i];
        }
        Ok((f.unit_id, sk))
    }

    /// Decrypt a LEAF's fields (what the escrow agents see).
    pub fn open_leaf(&self, leaf: &[u8; 16]) -> LeafFields {
        LeafFields::from_block(&aes_block(&self.family_key, *leaf, true))
    }
}

/// Blaze's search, resumable: try random LEAFs against the receiving chip.
#[derive(Clone, Debug)]
pub struct LeafSearch {
    pub tries: u64,
    pub found: Option<[u8; 16]>,
    rng: Drbg,
}

impl LeafSearch {
    pub fn new(label: &str) -> Self {
        Self { tries: 0, found: None, rng: Drbg::new(label.as_bytes(), "leaf search") }
    }

    /// Test up to `budget` random LEAFs. Returns true once one passes.
    pub fn step(&mut self, escrow: &Escrow, session_key: &[u8; 10], iv: &[u8; 8], budget: u64) -> bool {
        if self.found.is_some() {
            return true;
        }
        let want = checksum(session_key, iv);
        let c = Aes256::new_from_slice(&escrow.family_key).expect("32-byte key");
        for _ in 0..budget {
            let leaf: [u8; 16] = self.rng.array();
            self.tries += 1;
            let mut b = aes::Block::from(leaf);
            c.decrypt_block(&mut b);
            if b[14] == want[0] && b[15] == want[1] {
                self.found = Some(leaf);
                return true;
            }
        }
        false
    }
}

/// Expected tries to pass a checksum of `bits` bits by chance.
pub fn expected_tries(bits: u32) -> f64 {
    2f64.powi(bits as i32)
}

pub fn leaf_hex(leaf: &[u8; 16]) -> String {
    to_hex(leaf)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (Escrow, [u8; 10], [u8; 8]) {
        let mut rng = Drbg::new(b"seed", "clipper");
        let e = Escrow::new(&mut rng, 4);
        (e, rng.array(), rng.array())
    }

    #[test]
    fn honest_leaf_is_accepted_and_escrowed() {
        let (e, sk, iv) = setup();
        let leaf = e.honest_leaf(&e.chips[1], &sk, &iv);
        assert!(e.chip_accepts(&leaf, &sk, &iv));
        assert_eq!(e.wiretap(&leaf), Ok((e.chips[1].unit_id, sk)));
        // a different call (new IV) needs a new LEAF
        assert!(!e.chip_accepts(&leaf, &sk, &[9; 8]));
    }

    #[test]
    fn blaze_forgery_passes_the_chip_but_not_the_wiretap() {
        let (e, sk, iv) = setup();
        let mut s = LeafSearch::new("test");
        while !s.step(&e, &sk, &iv, 4096) {}
        let leaf = s.found.unwrap();
        assert!(e.chip_accepts(&leaf, &sk, &iv));
        assert!(s.tries < 1_000_000);
        match e.wiretap(&leaf) {
            Err(_) => {}
            Ok((_, got)) => assert_ne!(got, sk, "the forged LEAF must not give up the key"),
        }
    }

    #[test]
    fn zero_leaf_is_rejected() {
        let (e, sk, iv) = setup();
        assert!(!e.chip_accepts(&[0; 16], &sk, &iv) || checksum(&sk, &iv) == e.open_leaf(&[0; 16]).checksum);
    }
}
