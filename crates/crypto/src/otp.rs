//! One-time pads and the two-time pad.
//!
//! C = P xor K is perfectly secret if K is uniformly random, as long as the
//! message, and never used again (Shannon, 1949). Reuse the pad and
//!   C1 xor C2 = (P1 xor K) xor (P2 xor K) = P1 xor P2,
//! the key cancels and what is left is two plaintexts tangled together.
//! Guessing a word in one ("crib") reveals the same span of the other.

use crate::bytes::{ascii_to_bytes, bytes_to_ascii, xor};
use crate::drbg::Drbg;

#[derive(Clone, Debug)]
pub struct TwoTimePad {
    pub pad: Vec<u8>,
    pub c1: Vec<u8>,
    pub c2: Vec<u8>,
    /// c1 xor c2 = p1 xor p2
    pub x: Vec<u8>,
    pub p1: String,
    pub p2: String,
}

pub fn make_two_time_pad(p1: &str, p2: &str, rng: &mut Drbg) -> TwoTimePad {
    assert_eq!(p1.len(), p2.len(), "messages must have equal length");
    let pad = rng.bytes(p1.len());
    let c1 = xor(&ascii_to_bytes(p1), &pad);
    let c2 = xor(&ascii_to_bytes(p2), &pad);
    let x = xor(&c1, &c2);
    TwoTimePad { pad, c1, c2, x, p1: p1.into(), p2: p2.into() }
}

/// What a guess for one message implies for the other at the same positions.
pub fn partner(x: &[u8], guess: &str, offset: usize) -> String {
    let end = (offset + guess.chars().count()).min(x.len());
    if offset >= end {
        return String::new();
    }
    bytes_to_ascii(&xor(&x[offset..end], &ascii_to_bytes(guess)), '·')
}

fn plausible(ch: char) -> bool {
    ch.is_ascii_uppercase() || ch.is_ascii_digit() || " .,'!?:;-".contains(ch)
}

/// Share of characters in a decoded fragment that are capitals, digits or punctuation.
pub fn plausibility(fragment: &str) -> f64 {
    let n = fragment.chars().count();
    if n == 0 {
        return 0.0;
    }
    fragment.chars().filter(|&c| plausible(c)).count() as f64 / n as f64
}

#[derive(Clone, Debug)]
pub struct CribHit {
    pub offset: usize,
    pub partner_text: String,
    pub score: f64,
}

/// Slide a crib along the whole XOR stream (the classic "crib drag").
pub fn crib_drag(x: &[u8], crib: &str) -> Vec<CribHit> {
    let len = crib.chars().count();
    if len == 0 || len > x.len() {
        return Vec::new();
    }
    (0..=x.len() - len)
        .map(|offset| {
            let partner_text = partner(x, crib, offset);
            let score = plausibility(&partner_text);
            CribHit { offset, partner_text, score }
        })
        .collect()
}
