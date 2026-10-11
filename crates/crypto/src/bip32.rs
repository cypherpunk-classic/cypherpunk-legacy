//! BIP-32 hierarchical deterministic keys, written out (private derivation only).
//!
//!   master:  I = HMAC-SHA512("Bitcoin seed", seed),  k = I_L,  c = I_R
//!   child i: I = HMAC-SHA512(c, 0x00 || k || i)      (hardened, i >= 2^31)
//!            I = HMAC-SHA512(c, K || i)              (normal, K = k*G compressed)
//!            k_i = I_L + k mod n,  c_i = I_R
//!   fingerprint = first 4 bytes of HASH160(K) = RIPEMD160(SHA256(K))

use hmac::{Hmac, KeyInit, Mac};
use ripemd::Ripemd160;
use sha2::{Digest, Sha256, Sha512};

use crate::bytes::{Big, big_to_bytes, bytes_to_big, to_hex};
use crate::secp::{N, compressed, mod_n, mul};

pub const HARDENED: u32 = 0x8000_0000;

#[derive(Clone, Debug)]
pub struct ExtendedKey {
    pub depth: u8,
    pub parent_fingerprint: [u8; 4],
    pub child_number: u32,
    pub chain_code: [u8; 32],
    pub secret: Big,
}

fn hmac_sha512(key: &[u8], parts: &[&[u8]]) -> [u8; 64] {
    let mut mac = <Hmac<Sha512> as KeyInit>::new_from_slice(key).expect("any key length");
    for p in parts {
        mac.update(p);
    }
    mac.finalize().into_bytes().into()
}

pub fn hash160(data: &[u8]) -> [u8; 20] {
    Ripemd160::digest(Sha256::digest(data)).into()
}

impl ExtendedKey {
    pub fn master(seed: &[u8]) -> Self {
        let i = hmac_sha512(b"Bitcoin seed", &[seed]);
        let secret = bytes_to_big(&i[..32]);
        assert!(secret > Big::from(0) && secret < *N, "invalid master key (probability 2^-127)");
        Self { depth: 0, parent_fingerprint: [0; 4], child_number: 0, chain_code: i[32..].try_into().unwrap(), secret }
    }

    pub fn public_key(&self) -> [u8; 33] {
        compressed(&mul(&self.secret))
    }

    pub fn fingerprint(&self) -> [u8; 4] {
        hash160(&self.public_key())[..4].try_into().unwrap()
    }

    pub fn private_key(&self) -> [u8; 32] {
        big_to_bytes(&self.secret, 32).try_into().unwrap()
    }

    pub fn child(&self, index: u32) -> Self {
        let i = if index >= HARDENED {
            hmac_sha512(&self.chain_code, &[&[0u8], &self.private_key(), &index.to_be_bytes()])
        } else {
            hmac_sha512(&self.chain_code, &[&self.public_key(), &index.to_be_bytes()])
        };
        let il = bytes_to_big(&i[..32]);
        assert!(il < *N, "invalid child (probability 2^-127)");
        Self {
            depth: self.depth + 1,
            parent_fingerprint: self.fingerprint(),
            child_number: index,
            chain_code: i[32..].try_into().unwrap(),
            secret: mod_n(&(il + &self.secret)),
        }
    }

    /// Derive along a path such as `m/86'/0'/0'/0/0` (`'` or `h` marks hardened steps).
    pub fn derive(&self, path: &str) -> Result<Self, String> {
        let mut k = self.clone();
        for (n, part) in path.split('/').enumerate() {
            if n == 0 {
                if part != "m" {
                    return Err(format!("path must start with m: {path}"));
                }
                continue;
            }
            let (num, hardened) = match part.strip_suffix(['\'', 'h', 'H']) {
                Some(p) => (p, true),
                None => (part, false),
            };
            let idx: u32 = num.parse().map_err(|_| format!("bad path component {part}"))?;
            if idx >= HARDENED {
                return Err(format!("index too large: {part}"));
            }
            k = k.child(if hardened { idx + HARDENED } else { idx });
        }
        Ok(k)
    }

    fn serialize(&self, version: u32, key: &[u8; 33]) -> String {
        let mut payload = Vec::with_capacity(78);
        payload.extend_from_slice(&version.to_be_bytes());
        payload.push(self.depth);
        payload.extend_from_slice(&self.parent_fingerprint);
        payload.extend_from_slice(&self.child_number.to_be_bytes());
        payload.extend_from_slice(&self.chain_code);
        payload.extend_from_slice(key);
        base58check(&payload)
    }

    /// Extended public key, mainnet (`xpub...`).
    pub fn xpub(&self) -> String {
        self.serialize(0x0488_B21E, &self.public_key())
    }

    /// Extended private key, mainnet (`xprv...`).
    pub fn xprv(&self) -> String {
        let mut key = [0u8; 33];
        key[1..].copy_from_slice(&self.private_key());
        self.serialize(0x0488_ADE4, &key)
    }
}

const B58: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

pub fn base58(data: &[u8]) -> String {
    let zeros = data.iter().take_while(|&&b| b == 0).count();
    let mut digits: Vec<u8> = Vec::new(); // little-endian base-58 digits
    for &byte in &data[zeros..] {
        let mut carry = byte as u32;
        for d in digits.iter_mut() {
            carry += (*d as u32) << 8;
            *d = (carry % 58) as u8;
            carry /= 58;
        }
        while carry > 0 {
            digits.push((carry % 58) as u8);
            carry /= 58;
        }
    }
    let mut s = String::with_capacity(zeros + digits.len());
    s.extend(std::iter::repeat_n('1', zeros));
    s.extend(digits.iter().rev().map(|&d| B58[d as usize] as char));
    s
}

pub fn base58check(payload: &[u8]) -> String {
    let check = Sha256::digest(Sha256::digest(payload));
    let mut v = payload.to_vec();
    v.extend_from_slice(&check[..4]);
    base58(&v)
}

/// Master key fingerprint as 8 hex digits, the way descriptors write it.
pub fn master_fingerprint(seed: &[u8]) -> String {
    to_hex(&ExtendedKey::master(seed).fingerprint())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytes::from_hex;

    #[test]
    fn bip32_test_vector_1() {
        let seed = from_hex("000102030405060708090a0b0c0d0e0f").unwrap();
        let m = ExtendedKey::master(&seed);
        assert_eq!(
            m.xpub(),
            "xpub661MyMwAqRbcFtXgS5sYJABqqG9YLmC4Q1Rdap9gSE8NqtwybGhePY2gZ29ESFjqJoCu1Rupje8YtGqsefD265TMg7usUDFdp6W1EGMcet8"
        );
        assert_eq!(
            m.xprv(),
            "xprv9s21ZrQH143K3QTDL4LXw2F7HEK3wJUD2nW2nRk4stbPy6cq3jPPqjiChkVvvNKmPGJxWUtg6LnF5kejMRNNU3TGtRBeJgk33yuGBxrMPHi"
        );
        let k = m.derive("m/0'/1/2'/2/1000000000").unwrap();
        assert_eq!(
            k.xpub(),
            "xpub6H1LXWLaKsWFhvm6RVpEL9P4KfRZSW7abD2ttkWP3SSQvnyA8FSVqNTEcYFgJS2UaFcxupHiYkro49S8yGasTvXEYBVPamhGW6cFJodrTHy"
        );
        assert_eq!(
            k.xprv(),
            "xprvA41z7zogVVwxVSgdKUHDy1SKmdb533PjDz7J6N6mV6uS3ze1ai8FHa8kmHScGpWmj4WggLyQjgPie1rFSruoUihUZREPSL39UNdE3BBDu76"
        );
    }

    #[test]
    fn base58_leading_zeros() {
        assert_eq!(base58(&[0, 0, 1]), "112");
        assert_eq!(base58(b""), "");
    }
}
