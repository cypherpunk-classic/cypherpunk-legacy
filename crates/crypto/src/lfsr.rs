//! Linear-feedback shift registers and the Berlekamp–Massey algorithm.
//!
//! An LFSR of length L makes each new bit the XOR of some of the last L:
//! `s[t] = c1·s[t-1] ⊕ c2·s[t-2] ⊕ … ⊕ cL·s[t-L]`. With the right taps
//! (a primitive connection polynomial) the sequence runs 2^L − 1 bits before
//! it repeats and looks random to the eye. It is not: it is linear, and
//! Berlekamp–Massey (Berlekamp 1968, Massey 1969) recovers the shortest
//! register that makes any sequence from just 2L of its bits. XOR several
//! registers together and the lengths add, so the attack only needs more
//! bits. That is why LFSRs are never used as keystreams on their own (A5/1,
//! CSS and Bluetooth's E0 combined them with non-linear steps, and were
//! still broken).

/// A register in Fibonacci form: `taps[i]` is c_{i+1}; `state[0]` is the newest bit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lfsr {
    pub taps: Vec<u8>,
    pub state: Vec<u8>,
}

impl Lfsr {
    /// `taps` and `state` are the same length L; `state` must not be all zero.
    pub fn new(taps: Vec<u8>, state: Vec<u8>) -> Self {
        assert_eq!(taps.len(), state.len());
        Self { taps, state }
    }

    pub fn len(&self) -> usize {
        self.taps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.taps.is_empty()
    }

    /// The next bit, and the register steps on.
    pub fn next_bit(&mut self) -> u8 {
        let b = next_from(&self.taps, &self.state);
        self.state.insert(0, b);
        self.state.truncate(self.taps.len());
        b
    }

    /// Steps until the state comes back (0 if it never does from here).
    pub fn period(&self) -> usize {
        let mut r = self.clone();
        for n in 1..=(1usize << self.len()) {
            r.next_bit();
            if r.state == self.state {
                return n;
            }
        }
        0
    }
}

/// The bit a recurrence with these taps makes after `recent` (newest first).
pub fn next_from(taps: &[u8], recent: &[u8]) -> u8 {
    taps.iter().zip(recent).fold(0, |acc, (c, s)| acc ^ (c & s))
}

/// Taps (c1..cL, cL = 1) of every register of length `l` that runs the full
/// 2^l − 1 bits: the primitive connection polynomials, found by stepping them.
pub fn primitive_taps(l: usize) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    for mask in 0..(1u32 << (l - 1)) {
        let mut taps: Vec<u8> = (0..l - 1).map(|i| ((mask >> i) & 1) as u8).collect();
        taps.push(1);
        let mut start = vec![0u8; l];
        start[0] = 1;
        if Lfsr::new(taps.clone(), start).period() == (1 << l) - 1 {
            out.push(taps);
        }
    }
    out
}

/// How many bits of `seq` (oldest first) disagree with the recurrence `taps`,
/// over every position that has a full window behind it.
pub fn mismatches(taps: &[u8], seq: &[u8]) -> usize {
    let l = taps.len();
    (l..seq.len())
        .filter(|&t| {
            let recent: Vec<u8> = (1..=l).map(|i| seq[t - i]).collect();
            next_from(taps, &recent) != seq[t]
        })
        .count()
}

/// Predict the bit after `seq` (oldest first) with `taps`.
pub fn predict(taps: &[u8], seq: &[u8]) -> Option<u8> {
    let l = taps.len();
    if seq.len() < l {
        return None;
    }
    let recent: Vec<u8> = (1..=l).map(|i| seq[seq.len() - i]).collect();
    Some(next_from(taps, &recent))
}

/// Berlekamp–Massey over GF(2): the linear complexity L of `seq` (oldest
/// first) and the taps c1..cL of the shortest register that makes it.
pub fn berlekamp_massey(seq: &[u8]) -> (usize, Vec<u8>) {
    let n = seq.len();
    let mut c = vec![0u8; n + 1];
    let mut b = vec![0u8; n + 1];
    c[0] = 1;
    b[0] = 1;
    let (mut l, mut m) = (0usize, 1usize);
    for i in 0..n {
        let mut d = seq[i];
        for j in 1..=l {
            d ^= c[j] & seq[i - j];
        }
        if d == 0 {
            m += 1;
        } else if 2 * l <= i {
            let t = c.clone();
            for j in 0..=n - m {
                c[j + m] ^= b[j];
            }
            l = i + 1 - l;
            b = t;
            m = 1;
        } else {
            for j in 0..=n - m {
                c[j + m] ^= b[j];
            }
            m += 1;
        }
    }
    (l, c[1..=l].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(r: &mut Lfsr, n: usize) -> Vec<u8> {
        (0..n).map(|_| r.next_bit()).collect()
    }

    #[test]
    fn primitive_registers_run_their_full_period() {
        assert_eq!(primitive_taps(4).len(), 2, "x^4+x+1 and x^4+x^3+1");
        assert_eq!(primitive_taps(5).len(), 6);
        for taps in primitive_taps(5) {
            assert_eq!(Lfsr::new(taps, vec![1, 0, 0, 1, 1]).period(), 31);
        }
    }

    #[test]
    fn berlekamp_massey_needs_two_l_bits() {
        for taps in primitive_taps(5) {
            let mut r = Lfsr::new(taps.clone(), vec![0, 1, 1, 0, 1]);
            let s = run(&mut r, 40);
            let (l, c) = berlekamp_massey(&s[..10]);
            assert_eq!((l, &c), (5, &taps));
            assert_eq!(mismatches(&c, &s), 0);
            assert_eq!(predict(&c, &s), Some(r.clone().next_bit()));
        }
    }

    #[test]
    fn two_registers_xored_are_still_linear_just_longer() {
        let (t4, t5) = (primitive_taps(4)[0].clone(), primitive_taps(5)[2].clone());
        let mut a = Lfsr::new(t4, vec![1, 0, 1, 1]);
        let mut b = Lfsr::new(t5, vec![1, 1, 0, 0, 1]);
        let s: Vec<u8> = (0..60).map(|_| a.next_bit() ^ b.next_bit()).collect();
        let (l, c) = berlekamp_massey(&s[..18]);
        assert_eq!(l, 9, "the lengths add");
        assert_eq!(mismatches(&c, &s), 0, "18 bits pin it down");
        // too few bits: a shorter register that fits them and fails later
        let (l8, c8) = berlekamp_massey(&s[..10]);
        assert!(l8 < 9 && mismatches(&c8, &s) > 0, "{l8}");
    }
}
