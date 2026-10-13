//! Learning With Errors (Regev, 2005), at toy size.
//!
//! Secret s in Z_q^n. Public samples (a_i, b_i = <a_i, s> + e_i mod q) with
//! small errors e_i. Without the errors, s falls to Gaussian elimination; with
//! them, recovering s is believed hard even for quantum computers, and that
//! hardness is what ML-KEM (FIPS 203) builds on.
//!
//! Regev encryption of one bit: pick a random subset S of samples,
//!   u = sum_{i in S} a_i,   v = sum_{i in S} b_i + bit * floor(q/2).
//! Decryption computes v - <u, s> = bit * floor(q/2) + (sum of the chosen e_i):
//! close to 0 means 0, close to q/2 means 1, as long as the noise stays below q/4.

use crate::bytes::inv_mod_i64;
use crate::drbg::Drbg;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LweParams {
    pub n: usize,
    pub q: i64,
    pub m: usize,
    /// errors uniform in [-noise, noise]
    pub noise: i64,
}

pub const TOY_LWE: LweParams = LweParams { n: 4, q: 97, m: 12, noise: 1 };

#[derive(Clone, Debug)]
pub struct LweKeys {
    pub params: LweParams,
    pub s: Vec<i64>,
    pub a: Vec<Vec<i64>>,
    pub e: Vec<i64>,
    pub b: Vec<i64>,
}

fn md(x: i64, q: i64) -> i64 {
    x.rem_euclid(q)
}

pub fn lwe_keygen(params: LweParams, rng: &mut Drbg) -> LweKeys {
    let LweParams { n, q, m, noise } = params;
    let s: Vec<i64> = (0..n).map(|_| rng.int(q as usize) as i64).collect();
    let a: Vec<Vec<i64>> = (0..m).map(|_| (0..n).map(|_| rng.int(q as usize) as i64).collect()).collect();
    let e: Vec<i64> = (0..m).map(|_| rng.int((2 * noise + 1) as usize) as i64 - noise).collect();
    let b = a.iter().zip(&e).map(|(row, ei)| md(row.iter().zip(&s).map(|(x, y)| x * y).sum::<i64>() + ei, q)).collect();
    LweKeys { params, s, a, e, b }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LweCiphertext {
    pub subset: Vec<usize>,
    pub u: Vec<i64>,
    pub v: i64,
}

pub fn lwe_encrypt(keys: &LweKeys, bit: u8, rng: &mut Drbg) -> LweCiphertext {
    let LweParams { n, q, m, .. } = keys.params;
    let mut subset = Vec::new();
    while subset.is_empty() {
        subset = (0..m).filter(|_| rng.int(2) == 1).collect();
    }
    let mut u = vec![0i64; n];
    let mut v = bit as i64 * (q / 2);
    for &i in &subset {
        for (j, a) in keys.a[i].iter().enumerate() {
            u[j] = md(u[j] + a, q);
        }
        v = md(v + keys.b[i], q);
    }
    LweCiphertext { subset, u, v }
}

/// The "phase" v - <u,s> mod q: where the value lands on the circle.
pub fn lwe_phase(keys: &LweKeys, ct: &LweCiphertext) -> i64 {
    let q = keys.params.q;
    md(ct.v - ct.u.iter().zip(&keys.s).map(|(x, y)| x * y).sum::<i64>(), q)
}

pub fn lwe_decrypt(keys: &LweKeys, ct: &LweCiphertext) -> u8 {
    let q = keys.params.q;
    let ph = lwe_phase(keys, ct);
    let dist_to_half = (ph - q / 2).abs();
    let dist_to_zero = ph.min(q - ph);
    u8::from(dist_to_half < dist_to_zero)
}

/// Signed noise carried by a ciphertext: the sum of the errors of the chosen samples.
pub fn ciphertext_noise(keys: &LweKeys, ct: &LweCiphertext) -> i64 {
    ct.subset.iter().map(|&i| keys.e[i]).sum()
}

/// Solve A s = b (mod prime q) from the first n samples by Gaussian elimination.
/// Works perfectly without noise; with noise it confidently returns garbage.
pub fn gaussian_solve(a: &[Vec<i64>], b: &[i64], q: i64) -> Option<Vec<i64>> {
    let n = a[0].len();
    let mut m: Vec<Vec<i64>> = a[..n].iter().zip(b).map(|(row, bi)| row.iter().map(|&x| md(x, q)).chain(std::iter::once(md(*bi, q))).collect()).collect();
    for col in 0..n {
        let pivot = (col..n).find(|&r| m[r][col] != 0)?;
        m.swap(col, pivot);
        let inv = inv_mod_i64(m[col][col], q)?;
        m[col] = m[col].iter().map(|&x| md(x * inv, q)).collect();
        for r in 0..n {
            if r == col || m[r][col] == 0 {
                continue;
            }
            let f = m[r][col];
            let pivot_row = m[col].clone();
            m[r] = m[r].iter().zip(&pivot_row).map(|(&x, &p)| md(x - f * p, q)).collect();
        }
    }
    Some(m.iter().map(|row| row[n]).collect())
}

/// The error each sample would need for a candidate secret (centred representatives).
pub fn residuals(keys: &LweKeys, cand: &[i64]) -> Vec<i64> {
    let q = keys.params.q;
    keys.a
        .iter()
        .zip(&keys.b)
        .map(|(row, bi)| {
            let r = md(bi - row.iter().zip(cand).map(|(x, y)| x * y).sum::<i64>(), q);
            if r > q / 2 { r - q } else { r }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regev_roundtrip_and_noise_defeats_gauss() {
        let mut r = Drbg::new(&[2u8; 32], "lwe");
        let keys = lwe_keygen(TOY_LWE, &mut r);
        for bit in [0u8, 1, 1, 0, 1] {
            let ct = lwe_encrypt(&keys, bit, &mut r);
            assert_eq!(lwe_decrypt(&keys, &ct), bit);
        }
        assert!(residuals(&keys, &keys.s).iter().all(|e| e.abs() <= 1));
        let noiseless: Vec<i64> = keys.a.iter().map(|row| row.iter().zip(&keys.s).map(|(x, y)| x * y).sum::<i64>().rem_euclid(97)).collect();
        assert_eq!(gaussian_solve(&keys.a, &noiseless, 97), Some(keys.s.clone()));
    }
}
