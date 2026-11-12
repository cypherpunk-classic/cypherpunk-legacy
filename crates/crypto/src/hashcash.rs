//! Hashcash: proof-of-work postage (Adam Back, 1997).
//!
//! Back posted it to the Cypherpunks list to make spam and remailer flooding
//! expensive: before a server accepts your message, show it a string whose
//! hash starts with k zero bits. Finding one takes about 2^k hash evaluations;
//! checking it takes one. A person sending a few messages hardly notices; a
//! spammer sending a million pays a million times. Bitcoin's mining is the
//! same puzzle.
//!
//! The stamp format is Hashcash version 1,
//! `1:bits:date:resource:ext:rand:counter`, hashed with SHA-256 here (the
//! original used SHA-1).

use sha2::{Digest, Sha256};

use crate::bytes::to_hex;
use crate::drbg::Drbg;

/// Number of leading zero bits of a hash.
pub fn leading_zero_bits(h: &[u8]) -> u32 {
    let mut n = 0;
    for b in h {
        if *b == 0 {
            n += 8;
        } else {
            n += b.leading_zeros();
            break;
        }
    }
    n
}

pub fn stamp_hash(stamp: &str) -> [u8; 32] {
    Sha256::digest(stamp.as_bytes()).into()
}

/// The stamp without its counter: `1:bits:date:resource::rand:`.
pub fn header(bits: u32, date: &str, resource: &str, rand: &str) -> String {
    format!("1:{bits}:{date}:{resource}::{rand}:")
}

/// A fresh random field for a stamp (base64-like, 12 characters).
pub fn rand_field(rng: &mut Drbg) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    (0..12).map(|_| ALPHABET[rng.int(64)] as char).collect()
}

/// Parsed fields of a stamp.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stamp {
    pub bits: u32,
    pub date: String,
    pub resource: String,
    pub rand: String,
    pub counter: String,
}

pub fn parse(stamp: &str) -> Option<Stamp> {
    let f: Vec<&str> = stamp.split(':').collect();
    if f.len() != 7 || f[0] != "1" {
        return None;
    }
    Some(Stamp { bits: f[1].parse().ok()?, date: f[2].into(), resource: f[3].into(), rand: f[5].into(), counter: f[6].into() })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StampError {
    Malformed,
    WrongResource,
    /// claims more bits than it has
    NotEnoughWork { claimed: u32, actual: u32 },
    /// asks for less than the server requires
    TooCheap { claimed: u32, required: u32 },
    Replayed,
}

impl std::fmt::Display for StampError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StampError::Malformed => write!(f, "not a version-1 stamp"),
            StampError::WrongResource => write!(f, "the stamp was made for a different address"),
            StampError::NotEnoughWork { claimed, actual } => write!(f, "claims {claimed} zero bits, its hash has {actual}"),
            StampError::TooCheap { claimed, required } => write!(f, "{claimed} bits offered, {required} required"),
            StampError::Replayed => write!(f, "this stamp was already spent"),
        }
    }
}

/// Check a stamp for a resource at a required difficulty (the server's side: one hash).
pub fn verify(stamp: &str, resource: &str, required: u32, spent: &[String]) -> Result<u32, StampError> {
    let s = parse(stamp).ok_or(StampError::Malformed)?;
    if s.resource != resource {
        return Err(StampError::WrongResource);
    }
    let actual = leading_zero_bits(&stamp_hash(stamp));
    if actual < s.bits {
        return Err(StampError::NotEnoughWork { claimed: s.bits, actual });
    }
    if s.bits < required {
        return Err(StampError::TooCheap { claimed: s.bits, required });
    }
    if spent.iter().any(|x| x == stamp) {
        return Err(StampError::Replayed);
    }
    Ok(actual)
}

/// A resumable minting search, so a game can spend a slice of every frame on it.
#[derive(Clone, Debug)]
pub struct Minter {
    pub header: String,
    pub bits: u32,
    pub counter: u64,
    pub best: u32,
    pub found: Option<String>,
}

impl Minter {
    pub fn new(bits: u32, date: &str, resource: &str, rng: &mut Drbg) -> Self {
        Self { header: header(bits, date, resource, &rand_field(rng)), bits, counter: 0, best: 0, found: None }
    }

    /// Try up to `budget` counters. Returns true once a stamp is found.
    pub fn step(&mut self, budget: u64) -> bool {
        if self.found.is_some() {
            return true;
        }
        let base = Sha256::new().chain_update(self.header.as_bytes());
        let mut buf = [0u8; 16];
        for _ in 0..budget {
            let c = hex_u64(self.counter, &mut buf);
            let h: [u8; 32] = base.clone().chain_update(c).finalize().into();
            let z = leading_zero_bits(&h);
            self.best = self.best.max(z);
            if z >= self.bits {
                self.found = Some(format!("{}{:x}", self.header, self.counter));
                return true;
            }
            self.counter += 1;
        }
        false
    }

    pub fn tries(&self) -> u64 {
        self.counter + u64::from(self.found.is_some())
    }
}

/// Lower-case hex of `v` without leading zeros, written into `buf`.
fn hex_u64(v: u64, buf: &mut [u8; 16]) -> &[u8] {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut i = 16;
    let mut x = v;
    loop {
        i -= 1;
        buf[i] = DIGITS[(x & 15) as usize];
        x >>= 4;
        if x == 0 {
            break;
        }
    }
    &buf[i..]
}

/// Expected number of hashes to find a k-bit stamp.
pub fn expected_work(bits: u32) -> f64 {
    2f64.powi(bits as i32)
}

pub fn hash_hex(stamp: &str) -> String {
    to_hex(&stamp_hash(stamp))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_zero_bits() {
        assert_eq!(leading_zero_bits(&[0, 0, 0x1f]), 19);
        assert_eq!(leading_zero_bits(&[0x80]), 0);
        assert_eq!(leading_zero_bits(&[0, 1]), 15);
    }

    #[test]
    fn mint_and_verify() {
        let mut rng = Drbg::new(b"seed", "hashcash");
        let mut m = Minter::new(12, "261111", "board@cryptopolis", &mut rng);
        while !m.step(1000) {}
        let stamp = m.found.clone().unwrap();
        assert!(leading_zero_bits(&stamp_hash(&stamp)) >= 12);
        assert_eq!(verify(&stamp, "board@cryptopolis", 12, &[]).map(|b| b >= 12), Ok(true));
        assert_eq!(verify(&stamp, "other@cryptopolis", 12, &[]), Err(StampError::WrongResource));
        assert!(matches!(verify(&stamp, "board@cryptopolis", 20, &[]), Err(StampError::TooCheap { .. })));
        assert_eq!(verify(&stamp, "board@cryptopolis", 12, std::slice::from_ref(&stamp)), Err(StampError::Replayed));
        // lying about the work is caught by the one hash the server computes
        let liar = stamp.replacen(":12:", ":30:", 1);
        assert!(matches!(verify(&liar, "board@cryptopolis", 12, &[]), Err(StampError::NotEnoughWork { .. })));
    }
}
