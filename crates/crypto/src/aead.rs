//! Encryption alone vs. authenticated encryption.
//!
//! AES-CTR turns AES into a stream cipher: C = P xor AES_K(counter, counter+1, ...).
//! Flip bit i of the ciphertext and bit i of the plaintext flips, silently.
//! AES-GCM is CTR plus a GHASH authentication tag over the ciphertext; any
//! change makes decryption fail instead of producing an attacker-chosen message.

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use ctr::cipher::{KeyIvInit, StreamCipher};
use hkdf::Hkdf;
use sha2::{Digest, Sha256};

use crate::bytes::{ascii_to_bytes, bytes_to_ascii, to_hex};

type Aes256Ctr = ctr::Ctr128BE<aes::Aes256>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CtrTicket {
    /// 16-byte initial counter block
    pub nonce: [u8; 16],
    pub ct: Vec<u8>,
}

fn ctr_apply(key: &[u8; 32], nonce: &[u8; 16], data: &mut [u8]) {
    let mut c = Aes256Ctr::new_from_slices(key, nonce).expect("key and nonce sizes");
    c.apply_keystream(data);
}

pub fn ctr_encrypt(key: &[u8; 32], nonce: &[u8; 16], plaintext: &str) -> CtrTicket {
    let mut ct = ascii_to_bytes(plaintext);
    ctr_apply(key, nonce, &mut ct);
    CtrTicket { nonce: *nonce, ct }
}

pub fn ctr_decrypt(key: &[u8; 32], t: &CtrTicket) -> String {
    let mut pt = t.ct.clone();
    ctr_apply(key, &t.nonce, &mut pt);
    bytes_to_ascii(&pt, '?')
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GcmBox {
    /// 12 bytes
    pub nonce: [u8; 12],
    /// ciphertext || 16-byte tag
    pub ct: Vec<u8>,
}

pub fn gcm_encrypt(key: &[u8; 32], nonce: &[u8; 12], plaintext: &[u8], aad: &[u8]) -> GcmBox {
    let cipher = Aes256Gcm::new_from_slice(key).expect("32-byte key");
    let n = Nonce::from(*nonce);
    let ct = cipher.encrypt(&n, Payload { msg: plaintext, aad }).expect("encryption");
    GcmBox { nonce: *nonce, ct }
}

pub fn gcm_decrypt(key: &[u8; 32], b: &GcmBox, aad: &[u8]) -> Result<Vec<u8>, String> {
    let cipher = Aes256Gcm::new_from_slice(key).expect("32-byte key");
    let n = Nonce::from(b.nonce);
    cipher.decrypt(&n, Payload { msg: &b.ct, aad }).map_err(|_| "authentication tag mismatch".to_string())
}

pub fn gcm_encrypt_text(key: &[u8; 32], nonce: &[u8; 12], text: &str) -> GcmBox {
    gcm_encrypt(key, nonce, text.as_bytes(), b"")
}

pub fn gcm_decrypt_text(key: &[u8; 32], b: &GcmBox) -> Result<String, String> {
    gcm_decrypt(key, b, b"").map(|p| String::from_utf8_lossy(&p).into_owned())
}

/// Flip one bit of a byte array (returns a copy). Bit 0 is the top bit of byte 0.
pub fn flip_bit(b: &[u8], bit_index: usize) -> Vec<u8> {
    let mut out = b.to_vec();
    out[bit_index >> 3] ^= 0x80 >> (bit_index & 7);
    out
}

/// The malleability attack in one line: C' = C xor (P xor P').
pub fn forge_ctr(t: &CtrTicket, known: &str, wanted: &str) -> Result<CtrTicket, String> {
    if known.chars().count() != wanted.chars().count() {
        return Err("same length only".into());
    }
    let delta: Vec<u8> = ascii_to_bytes(known).iter().zip(ascii_to_bytes(wanted)).map(|(a, b)| a ^ b).collect();
    let ct = t.ct.iter().enumerate().map(|(i, b)| b ^ delta.get(i).copied().unwrap_or(0)).collect();
    Ok(CtrTicket { nonce: t.nonce, ct })
}

/// Derive a 32-byte symmetric key from arbitrary secret material (HKDF-SHA256).
pub fn kdf(secret: &[u8], info: &str) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(Some(b"cypherpunk-legacy"), secret);
    let mut okm = [0u8; 32];
    hk.expand(info.as_bytes(), &mut okm).expect("32 bytes is a valid HKDF length");
    okm
}

pub fn sha256_hex(b: &[u8]) -> String {
    to_hex(&Sha256::digest(b))
}
