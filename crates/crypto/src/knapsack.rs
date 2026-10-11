//! The Merkle–Hellman knapsack cryptosystem (1978) and the lattice attack that
//! killed it.
//!
//! Private key: a superincreasing sequence w (each term exceeds the sum of all
//! before it), a modulus q > sum(w) and a multiplier r coprime to q.
//! Public key: a_i = r * w_i mod q.   Encrypt bits x: c = sum x_i * a_i.
//! Decrypt: c * r^-1 mod q = sum x_i * w_i, which is easy to solve greedily.
//!
//! Shamir broke the basic scheme in 1982; Lagarias–Odlyzko (1985) and Coster,
//! Joux, LaMacchia, Odlyzko, Schnorr and Stern (1992) showed that *any*
//! low-density subset sum falls to lattice reduction. With the CJLOSS basis
//!   b_i     = (0, .., 2, .., 0,  N*a_i)     i = 1..n
//!   b_{n+1} = (1, .., 1, .., 1,  N*c)
//! the vector sum x_i*b_i - b_{n+1} = (2x_1 - 1, ..., 2x_n - 1, 0) has entries
//! +-1, length sqrt(n), and LLL tends to find it.

use num_traits::{One, Zero};

use crate::bytes::{Big, big, gcd, mod_inv, modulo};
use crate::drbg::Drbg;
use crate::lattice::{Basis, LllResult, lll};

#[derive(Clone, Debug)]
pub struct MhPrivate {
    pub w: Vec<Big>,
    pub q: Big,
    pub r: Big,
}

#[derive(Clone, Debug)]
pub struct MhKeys {
    pub private: MhPrivate,
    pub public: Vec<Big>,
}

pub fn mh_keygen(n: usize, rng: &mut Drbg, start_bits: u32, step_bits: u32) -> MhKeys {
    let mut w = Vec::with_capacity(n);
    let mut sum = Big::zero();
    for i in 0..n {
        let next = if i == 0 {
            rng.range(&(Big::one() << (start_bits - 1)), &(Big::one() << start_bits))
        } else {
            &sum + rng.range(&Big::one(), &(Big::one() << step_bits))
        };
        sum += &next;
        w.push(next);
    }
    let q = &sum + rng.range(&Big::one(), &sum);
    let r = loop {
        let r = rng.range(&(&q / 4), &(&q - 1));
        if gcd(&r, &q).is_one() {
            break r;
        }
    };
    let public = w.iter().map(|wi| modulo(&(&r * wi), &q)).collect();
    MhKeys { private: MhPrivate { w, q, r }, public }
}

pub fn mh_encrypt(public: &[Big], bits: &[u8]) -> Big {
    public.iter().zip(bits).filter(|(_, b)| **b == 1).fold(Big::zero(), |acc, (a, _)| acc + a)
}

pub fn mh_decrypt(private: &MhPrivate, c: &Big) -> Result<Vec<u8>, String> {
    let mut t = modulo(&(c * mod_inv(&private.r, &private.q).unwrap()), &private.q);
    let mut bits = vec![0u8; private.w.len()];
    for i in (0..private.w.len()).rev() {
        if t >= private.w[i] {
            bits[i] = 1;
            t -= &private.w[i];
        }
    }
    if !t.is_zero() {
        return Err("not a valid ciphertext".into());
    }
    Ok(bits)
}

pub fn density(public: &[Big]) -> f64 {
    let max_bits = public.iter().map(|a| a.bits()).max().unwrap_or(1);
    public.len() as f64 / max_bits as f64
}

pub fn text_to_bits(text: &str) -> Vec<u8> {
    text.chars().flat_map(|ch| {
        let code = ch as u32 & 0xff;
        (0..8).rev().map(move |i| ((code >> i) & 1) as u8)
    }).collect()
}

pub fn bits_to_text(bits: &[u8]) -> String {
    bits.chunks_exact(8).map(|c| c.iter().fold(0u32, |acc, &b| (acc << 1) | b as u32)).map(|code| char::from_u32(code).unwrap_or('?')).collect()
}

/// Split text into blocks of n bits (n multiple of 8), padding with spaces.
pub fn encrypt_text(public: &[Big], text: &str) -> Vec<Big> {
    let chars_per_block = public.len() / 8;
    let mut padded: String = text.to_string();
    while padded.chars().count() % chars_per_block != 0 {
        padded.push(' ');
    }
    let chars: Vec<char> = padded.chars().collect();
    chars.chunks(chars_per_block).map(|block| mh_encrypt(public, &text_to_bits(&block.iter().collect::<String>()))).collect()
}

/// Scaling factor N for the last column; it must exceed sqrt(n)/2.
pub fn cj_scale(n: usize) -> Big {
    big(((n as f64).sqrt().ceil() as i64 + 1) * 4)
}

pub fn cj_basis(public: &[Big], c: &Big, scale: &Big) -> Basis {
    let n = public.len();
    let mut rows: Basis = Vec::with_capacity(n + 1);
    for (i, a) in public.iter().enumerate() {
        let mut row = vec![Big::zero(); n + 1];
        row[i] = big(2);
        row[n] = scale * a;
        rows.push(row);
    }
    let mut last = vec![Big::one(); n];
    last.push(scale * c);
    rows.push(last);
    rows
}

/// Look for a row of the form (+-1, ..., +-1, 0) that decodes to a valid solution.
pub fn decode_solution(reduced: &Basis, public: &[Big], c: &Big) -> Option<(Vec<u8>, usize)> {
    let n = public.len();
    let one = Big::one();
    let minus = -Big::one();
    for (r, v) in reduced.iter().enumerate() {
        if !v[n].is_zero() {
            continue;
        }
        if !v[..n].iter().all(|x| x == &one || x == &minus) {
            continue;
        }
        for sign in [one.clone(), minus.clone()] {
            let bits: Vec<u8> = v[..n].iter().map(|x| u8::from(&(x * &sign) == &one)).collect();
            if &mh_encrypt(public, &bits) == c {
                return Some((bits, r));
            }
        }
    }
    None
}

#[derive(Clone, Debug)]
pub struct AttackResult {
    pub bits: Option<Vec<u8>>,
    pub row: Option<usize>,
    pub lll: LllResult,
    pub attempts: usize,
}

/// A deterministic reordering of the basis rows (the lattice is unchanged).
fn rotate_rows(b: &Basis, k: usize) -> Basis {
    let len = b.len();
    let step = 2 * k + 1;
    let mut used = vec![false; len];
    let mut out = Vec::with_capacity(len);
    let mut i = (k * 7) % len;
    while out.len() < len {
        while used[i] {
            i = (i + 1) % len;
        }
        used[i] = true;
        out.push(b[i].clone());
        i = (i + step) % len;
    }
    out
}

/// Reduce the CJLOSS lattice and read off the plaintext bits. LLL is not an SVP
/// oracle, so on an unlucky block we retry from a reordered (but equivalent)
/// starting basis, which steers LLL down a different path.
pub fn attack_block(public: &[Big], c: &Big, attempts: usize) -> AttackResult {
    let base = cj_basis(public, c, &cj_scale(public.len()));
    let mut last = None;
    for a in 0..attempts {
        let start = if a == 0 { base.clone() } else { rotate_rows(&base, a) };
        let res = lll(&start, (99, 100), 1).expect("CJLOSS basis is independent");
        if let Some((bits, row)) = decode_solution(&res.basis, public, c) {
            return AttackResult { bits: Some(bits), row: Some(row), lll: res, attempts: a + 1 };
        }
        last = Some(res);
    }
    AttackResult { bits: None, row: None, lll: last.unwrap(), attempts }
}

#[derive(Clone, Debug)]
pub struct KnapsackPuzzle {
    pub keys: MhKeys,
    pub message: String,
    pub blocks: Vec<Big>,
}

/// Generate a key and ciphertext for which the lattice attack is known to work
/// on every block (LLL is deterministic, so checking once is enough).
pub fn make_knapsack_puzzle(rng: &mut Drbg, message: &str, n: usize) -> KnapsackPuzzle {
    for _ in 0..20 {
        let keys = mh_keygen(n, rng, 24, 8);
        let blocks = encrypt_text(&keys.public, message);
        if blocks.iter().all(|c| attack_block(&keys.public, c, 4).bits.is_some()) {
            return KnapsackPuzzle { keys, message: message.to_string(), blocks };
        }
    }
    panic!("could not generate an attackable knapsack instance")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merkle_hellman_roundtrip_and_attack() {
        let mut r = Drbg::new(&[5u8; 32], "knapsack");
        let keys = mh_keygen(32, &mut r, 24, 8);
        let blocks = encrypt_text(&keys.public, "LATTICE!");
        let mut bits = Vec::new();
        for c in &blocks {
            bits.extend(mh_decrypt(&keys.private, c).unwrap());
        }
        assert_eq!(bits_to_text(&bits), "LATTICE!");
        assert!(density(&keys.public) < 0.9);
        let res = attack_block(&keys.public, &blocks[0], 4);
        assert_eq!(bits_to_text(res.bits.as_ref().unwrap()), "LATT");
    }
}
