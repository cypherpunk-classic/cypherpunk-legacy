//! Deterministic random bit generator for *puzzle content*.
//!
//! Every save file carries a random 32-byte seed. Each puzzle derives its own
//! stream with a label (`"ch1/gate"`, `"warden"`, ...) so that a puzzle looks
//! the same every time the same save is loaded, while two players see different
//! keys, pads and shares. Output block i is SHA-256(seed || "|label|" || i).
//!
//! This is a content generator, not a key generator: the only real secret in
//! the game (the key you create in the epilogue) comes from the OS RNG.

use num_traits::One;
use sha2::{Digest, Sha256};

use crate::bytes::{Big, bytes_to_big};

#[derive(Clone)]
pub struct Drbg {
    counter: u32,
    buf: Vec<u8>,
    prefix: Vec<u8>,
}

impl Drbg {
    pub fn new(seed: &[u8], label: &str) -> Self {
        let mut prefix = seed.to_vec();
        prefix.extend_from_slice(format!("|{label}|").as_bytes());
        Self { counter: 0, buf: Vec::new(), prefix }
    }

    /// A stream seeded from the operating system's RNG.
    pub fn fresh(label: &str) -> Self {
        Self::new(&random_bytes::<32>(), label)
    }

    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(n);
        while out.len() < n {
            if self.buf.is_empty() {
                let mut h = Sha256::new();
                h.update(&self.prefix);
                h.update(self.counter.to_be_bytes());
                self.counter += 1;
                self.buf = h.finalize().to_vec();
            }
            let take = (n - out.len()).min(self.buf.len());
            out.extend(self.buf.drain(..take));
        }
        out
    }

    pub fn array<const L: usize>(&mut self) -> [u8; L] {
        self.bytes(L).try_into().expect("length")
    }

    /// Uniform integer in `[0, n)` by rejection sampling.
    pub fn below(&mut self, n: &Big) -> Big {
        assert!(n > &Big::from(0), "bound must be positive");
        let bits = n.bits();
        let nbytes = bits.div_ceil(8) as usize;
        let mask: Big = (Big::one() << bits) - 1;
        loop {
            let x = bytes_to_big(&self.bytes(nbytes)) & &mask;
            if &x < n {
                return x;
            }
        }
    }

    /// Uniform integer in `[lo, hi]`.
    pub fn range(&mut self, lo: &Big, hi: &Big) -> Big {
        lo + self.below(&(hi - lo + 1))
    }

    /// Uniform `usize` in `[0, n)` for small n.
    pub fn int(&mut self, n: usize) -> usize {
        let v = self.below(&Big::from(n));
        usize::try_from(v).expect("small")
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.int(items.len())]
    }

    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.int(i + 1);
            items.swap(i, j);
        }
    }

    /// Non-zero scalar modulo n.
    pub fn scalar(&mut self, n: &Big) -> Big {
        Big::one() + self.below(&(n - 1))
    }
}

/// Bytes from the operating system's (or browser's) cryptographic RNG.
pub fn random_bytes<const L: usize>() -> [u8; L] {
    let mut out = [0u8; L];
    getrandom::fill(&mut out).expect("system randomness unavailable");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_label_separated() {
        let mut a = Drbg::new(&[7u8; 32], "x");
        let mut b = Drbg::new(&[7u8; 32], "x");
        let mut c = Drbg::new(&[7u8; 32], "y");
        let (x, y, z) = (a.bytes(50), b.bytes(50), c.bytes(50));
        assert_eq!(x, y);
        assert_ne!(x, z);
        let n = Big::from(1000);
        for _ in 0..200 {
            assert!(a.below(&n) < n);
        }
    }
}
