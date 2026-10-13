//! BIP-39 mnemonic codes and the "missing word" recovery used in the prologue.
//!
//! A 12-word mnemonic encodes 128 bits of entropy (ENT) plus a 4-bit checksum
//! (CS = first ENT/32 bits of SHA-256(ENT)): 12 words x 11 bits = 132 bits.
//! Because only 4 bits are redundancy, a single unknown word leaves about
//! 2048 / 16 = 128 candidates; extra knowledge (a visible first letter, a key
//! fingerprint written on a sticky note) narrows it to one.

use std::collections::HashMap;
use std::sync::LazyLock;

use hmac::Hmac;
use sha2::{Digest, Sha256, Sha512};

use crate::bip32::{ExtendedKey, master_fingerprint};
use crate::bytes::{bits_to_bytes, bytes_to_bits};
use crate::drbg::Drbg;

/// The official English wordlist (SHA-256 2f5eed53...4dbda).
pub static WORDLIST: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    let w: Vec<&'static str> = include_str!("english.txt").lines().collect();
    assert_eq!(w.len(), 2048);
    w
});

static INDEX: LazyLock<HashMap<&'static str, usize>> =
    LazyLock::new(|| WORDLIST.iter().enumerate().map(|(i, w)| (*w, i)).collect());

pub fn is_word(w: &str) -> bool {
    INDEX.contains_key(w.trim().to_lowercase().as_str())
}

pub fn word_index(w: &str) -> Option<usize> {
    INDEX.get(w.trim().to_lowercase().as_str()).copied()
}

pub fn word_bits(w: &str) -> Result<String, String> {
    let i = word_index(w).ok_or_else(|| format!("not a BIP-39 word: {w}"))?;
    Ok(format!("{i:011b}"))
}

#[derive(Clone, Debug)]
pub struct ChecksumReport {
    /// All 11*words bits.
    pub bits: String,
    pub entropy_bits: String,
    /// The checksum as written in the words.
    pub checksum_bits: String,
    /// What SHA-256(entropy) says it should be.
    pub expected_bits: String,
    pub entropy: Vec<u8>,
    pub ok: bool,
}

pub fn checksum_report<S: AsRef<str>>(words: &[S]) -> Result<ChecksumReport, String> {
    if words.len() % 3 != 0 || words.len() < 12 {
        return Err("mnemonic length must be 12, 15, 18, 21 or 24".into());
    }
    let bits = words.iter().map(|w| word_bits(w.as_ref())).collect::<Result<String, _>>()?;
    let cs_len = words.len() / 3;
    let entropy_bits = bits[..bits.len() - cs_len].to_string();
    let checksum_bits = bits[bits.len() - cs_len..].to_string();
    let entropy = bits_to_bytes(&entropy_bits)?;
    let expected_bits = bytes_to_bits(&Sha256::digest(&entropy))[..cs_len].to_string();
    let ok = checksum_bits == expected_bits;
    Ok(ChecksumReport { bits, entropy_bits, checksum_bits, expected_bits, entropy, ok })
}

pub fn checksum_ok<S: AsRef<str>>(words: &[S]) -> bool {
    checksum_report(words).map(|r| r.ok).unwrap_or(false)
}

/// Every word that makes the checksum valid when placed at `missing`.
pub fn checksum_candidates(words: &[Option<&str>], missing: usize) -> Vec<&'static str> {
    let mut trial: Vec<&str> = words.iter().map(|w| w.unwrap_or("abandon")).collect();
    let mut out = Vec::new();
    for &w in WORDLIST.iter() {
        trial[missing] = w;
        if checksum_ok(&trial) {
            out.push(w);
        }
    }
    out
}

#[derive(Clone, Debug)]
pub struct Funnel {
    pub total: usize,
    pub checksum_valid: Vec<&'static str>,
    pub letter_matches: Vec<&'static str>,
}

pub fn recovery_funnel(words: &[Option<&str>], missing: usize, first_letter: char) -> Funnel {
    let checksum_valid = checksum_candidates(words, missing);
    let first = first_letter.to_ascii_lowercase();
    let letter_matches = checksum_valid.iter().copied().filter(|w| w.starts_with(first)).collect();
    Funnel { total: WORDLIST.len(), checksum_valid, letter_matches }
}

pub fn entropy_to_mnemonic(entropy: &[u8]) -> String {
    assert!(entropy.len() % 4 == 0 && (16..=32).contains(&entropy.len()), "bad entropy length");
    let cs_len = entropy.len() * 8 / 32;
    let mut bits = bytes_to_bits(entropy);
    bits.push_str(&bytes_to_bits(&Sha256::digest(entropy))[..cs_len]);
    (0..bits.len() / 11)
        .map(|i| WORDLIST[usize::from_str_radix(&bits[11 * i..11 * i + 11], 2).unwrap()])
        .collect::<Vec<_>>()
        .join(" ")
}

/// BIP-39 seed: PBKDF2-HMAC-SHA512(mnemonic, "mnemonic" || passphrase, 2048 rounds, 64 bytes).
///
/// (BIP-39 asks for NFKD normalisation first; every phrase and passphrase in the
/// game is plain ASCII, for which NFKD is the identity.)
pub fn seed_from_mnemonic(phrase: &str, passphrase: &str) -> [u8; 64] {
    let salt = format!("mnemonic{passphrase}");
    let mut out = [0u8; 64];
    pbkdf2::pbkdf2::<Hmac<Sha512>>(phrase.as_bytes(), salt.as_bytes(), 2048, &mut out).expect("valid length");
    out
}

pub fn is_valid_mnemonic(phrase: &str) -> bool {
    let words: Vec<&str> = phrase.split_whitespace().collect();
    checksum_ok(&words)
}

#[derive(Clone, Debug)]
pub struct DerivedKey {
    pub path: String,
    pub private_key: [u8; 32],
    /// 33-byte compressed
    pub public_key: [u8; 33],
    /// 32-byte BIP-340 key
    pub x_only: [u8; 32],
    pub xpub: String,
}

pub fn derive_key(seed: &[u8], path: &str) -> DerivedKey {
    let k = ExtendedKey::master(seed).derive(path).expect("valid path");
    let public_key = k.public_key();
    DerivedKey {
        path: path.to_string(),
        private_key: k.private_key(),
        public_key,
        x_only: public_key[1..].try_into().unwrap(),
        xpub: k.xpub(),
    }
}

/// Standard BIP-86 single-key Taproot paths.
pub const BIP86_ACCOUNT: &str = "m/86'/0'/0'";
pub const BIP86_FIRST: &str = "m/86'/0'/0'/0/0";

pub fn descriptor(seed: &[u8]) -> String {
    let fp = master_fingerprint(seed);
    let acct = derive_key(seed, BIP86_ACCOUNT);
    format!("tr([{fp}/86h/0h/0h]{}/0/*)", acct.xpub)
}

#[derive(Clone, Debug)]
pub struct GrandpaSeed {
    pub words: Vec<&'static str>,
    pub phrase: String,
    pub missing_index: usize,
    pub first_letter: char,
    pub fingerprint: String,
    pub funnel: Funnel,
}

/// Generate a mnemonic whose recovery funnel is interesting: the visible first
/// letter must leave between 3 and 9 checksum-valid candidates, so the
/// fingerprint on the sticky note is genuinely needed to pick the right one.
pub fn generate_grandpa_seed(rng: &mut Drbg) -> GrandpaSeed {
    loop {
        let entropy = rng.bytes(16);
        let phrase = entropy_to_mnemonic(&entropy);
        let words: Vec<&'static str> = phrase.split(' ').map(|w| WORDLIST[word_index(w).unwrap()]).collect();
        let missing_index = 3 + rng.int(7); // somewhere in the middle of the letter
        let first_letter = words[missing_index].chars().next().unwrap();
        let masked: Vec<Option<&str>> = words.iter().enumerate().map(|(i, w)| (i != missing_index).then_some(*w)).collect();
        let funnel = recovery_funnel(&masked, missing_index, first_letter);
        if funnel.letter_matches.len() < 3 || funnel.letter_matches.len() > 9 {
            continue;
        }
        let fingerprint = master_fingerprint(&seed_from_mnemonic(&phrase, ""));
        return GrandpaSeed { words, phrase, missing_index, first_letter, fingerprint, funnel };
    }
}
