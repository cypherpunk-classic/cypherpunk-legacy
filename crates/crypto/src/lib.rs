//! The cryptography behind *Cypherpunk Legacy*.
//!
//! Everything in this crate is pure Rust with no game or engine code, so it can
//! be tested against the official vectors and read on its own as reference
//! material. Elliptic-curve arithmetic comes from RustCrypto's `k256`, hashes
//! and MACs from `sha2`/`hmac`/`pbkdf2`/`ripemd`/`hkdf`, AES from `aes`, `ctr`
//! and `aes-gcm`, and the post-quantum schemes from `ml-kem` and `ml-dsa`. The
//! protocol logic on top (BIP-39 recovery funnels, BIP-340 nonce reuse,
//! BIP-327 KeyAgg, BIP-341 tweaks, Shamir/Feldman, Sigma protocols, LLL,
//! knapsacks, LWE, Chaumian e-cash, onion routing, Hashcash, the Clipper LEAF,
//! the web of trust, lamp-board pads) is written out in full here, because that logic *is* the
//! lesson.

pub mod aead;
pub mod bech32;
pub mod bip32;
pub mod bip340;
pub mod bip39;
pub mod bytes;
pub mod clipper;
pub mod dh;
pub mod dots;
pub mod drbg;
pub mod ecash;
pub mod hashcash;
pub mod hashchain;
pub mod knapsack;
pub mod lamps;
pub mod lattice;
pub mod lwe;
pub mod musig;
pub mod ocr;
pub mod onion;
pub mod otp;
pub mod pq;
pub mod secp;
pub mod shamir;
pub mod sigma;
pub mod taproot;
pub mod wot;

pub use bytes::{Big, big};
pub use drbg::Drbg;
pub use secp::Pt;
