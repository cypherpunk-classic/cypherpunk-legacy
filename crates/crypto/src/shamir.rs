//! Shamir secret sharing (1979) and Feldman verifiable secret sharing (1987).
//!
//! The dealer picks a random polynomial of degree t-1 with f(0) = secret and
//! hands out shares (i, f(i)). Any t shares determine f by Lagrange
//! interpolation; any t-1 shares are consistent with *every* possible secret.
//!
//! Here the field is Z_n with n the order of secp256k1, so the secret can be a
//! signing key and the dealer can publish Feldman commitments C_j = a_j * G.
//! Share (i, y) is genuine iff  y*G == C_0 + i*C_1 + i^2*C_2 + ...

use num_traits::{One, Signed, Zero};

use crate::bytes::{Big, big, gcd, mod_inv, modulo};
use crate::drbg::Drbg;
use crate::secp::{N, Pt, infinity, mod_n, mul, mul_pt};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Share {
    pub x: Big,
    pub y: Big,
}

pub fn eval_poly(coeffs: &[Big], x: &Big, p: &Big) -> Big {
    coeffs.iter().rev().fold(Big::zero(), |acc, c| modulo(&(acc * x + c), p))
}

#[derive(Clone, Debug)]
pub struct Dealing {
    /// a_0 = secret
    pub coeffs: Vec<Big>,
    pub shares: Vec<Share>,
    /// C_j = a_j * G (only over the curve order)
    pub commitments: Vec<Pt>,
}

pub fn deal(secret: &Big, threshold: usize, count: usize, rng: &mut Drbg, p: &Big) -> Dealing {
    let mut coeffs = vec![modulo(secret, p)];
    for _ in 1..threshold {
        coeffs.push(rng.scalar(p));
    }
    let shares = (1..=count).map(|i| Share { x: big(i as i64), y: eval_poly(&coeffs, &big(i as i64), p) }).collect();
    let commitments = if p == &*N { coeffs.iter().map(mul).collect() } else { Vec::new() };
    Dealing { coeffs, shares, commitments }
}

#[derive(Clone, Debug)]
pub struct LagrangeTerm {
    pub x: Big,
    /// prod x_j (as a plain integer, reduced fraction)
    pub num: Big,
    /// prod (x_j - x_i) (as a plain integer, reduced fraction, positive)
    pub den: Big,
    /// num/den mod p
    pub lambda: Big,
}

/// Lagrange basis coefficients at x = 0, both as fractions and reduced mod p.
pub fn lagrange_at_zero(xs: &[Big], p: &Big) -> Vec<LagrangeTerm> {
    xs.iter()
        .enumerate()
        .map(|(i, xi)| {
            let mut num = Big::one();
            let mut den = Big::one();
            for (j, xj) in xs.iter().enumerate() {
                if j == i {
                    continue;
                }
                num *= xj;
                den *= xj - xi;
            }
            let g = gcd(&num, &den);
            if g > Big::one() {
                num /= &g;
                den /= &g;
            }
            if den.is_negative() {
                num = -num;
                den = -den;
            }
            let lambda = modulo(&(&num * mod_inv(&modulo(&den, p), p).expect("distinct x")), p);
            LagrangeTerm { x: xi.clone(), num, den, lambda }
        })
        .collect()
}

pub fn reconstruct(shares: &[Share], p: &Big) -> Big {
    let xs: Vec<Big> = shares.iter().map(|s| s.x.clone()).collect();
    lagrange_at_zero(&xs, p).iter().zip(shares).fold(Big::zero(), |acc, (t, s)| modulo(&(acc + &t.lambda * &s.y), p))
}

pub struct FeldmanCheck {
    pub lhs: Pt,
    pub rhs: Pt,
    pub ok: bool,
}

/// Feldman check: y*G == sum_j x^j * C_j. Returns both sides for display.
pub fn feldman_check(share: &Share, commitments: &[Pt]) -> FeldmanCheck {
    let lhs = mul(&share.y);
    let mut rhs = infinity();
    let mut x_pow = Big::one();
    for c in commitments {
        rhs += mul_pt(&x_pow, c);
        x_pow = mod_n(&(x_pow * &share.x));
    }
    FeldmanCheck { ok: lhs == rhs, lhs, rhs }
}

// ---------------------------------------------------------------------------
// The toy field GF(31): small enough to draw every point on screen.
// ---------------------------------------------------------------------------

pub const TOY_P: i64 = 31;

fn md(a: i64, p: i64) -> i64 {
    a.rem_euclid(p)
}

fn inv(a: i64, p: i64) -> i64 {
    crate::bytes::inv_mod_i64(a, p).expect("invertible")
}

/// Coefficients [a0, a1, a2] of the unique quadratic through three points mod p.
pub fn quadratic_through(points: &[(i64, i64); 3], p: i64) -> [i64; 3] {
    let mut coeffs = [0i64; 3];
    for (i, &(xi, yi)) in points.iter().enumerate() {
        // basis polynomial L_i(x) = prod_{j != i} (x - x_j) / (x_i - x_j), low degree first
        let mut denom = 1i64;
        let mut basis: Vec<i64> = vec![1];
        for (j, &(xj, _)) in points.iter().enumerate() {
            if j == i {
                continue;
            }
            denom = md(denom * (xi - xj), p);
            let mut next = vec![0i64; basis.len() + 1];
            for (k, &c) in basis.iter().enumerate() {
                next[k] = md(next[k] - c * xj, p);
                next[k + 1] = md(next[k + 1] + c, p);
            }
            basis = next;
        }
        let scale = md(yi * inv(denom, p), p);
        for (k, &c) in basis.iter().enumerate() {
            coeffs[k] = md(coeffs[k] + c * scale, p);
        }
    }
    coeffs
}

/// For two known shares and a guessed secret s, the curve that would explain them.
pub fn curve_for_guess(s: i64, known: &[(i64, i64); 2], p: i64) -> [i64; 3] {
    quadratic_through(&[(0, s), known[0], known[1]], p)
}

pub fn eval_toy(coeffs: &[i64; 3], x: i64, p: i64) -> i64 {
    md(coeffs[0] + coeffs[1] * x + coeffs[2] * x * x, p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn threshold_reconstruction_and_feldman() {
        let mut r = Drbg::new(&[9u8; 32], "shamir");
        let secret = r.scalar(&N);
        let d = deal(&secret, 3, 5, &mut r, &N);
        assert_eq!(reconstruct(&d.shares[0..3], &N), secret);
        assert_eq!(reconstruct(&[d.shares[4].clone(), d.shares[1].clone(), d.shares[3].clone()], &N), secret);
        assert_ne!(reconstruct(&d.shares[0..2], &N), secret);
        for s in &d.shares {
            assert!(feldman_check(s, &d.commitments).ok);
        }
        let forged = Share { x: big(3), y: r.scalar(&N) };
        assert!(!feldman_check(&forged, &d.commitments).ok);
        let terms = lagrange_at_zero(&[big(1), big(2), big(3)], &N);
        assert_eq!((terms[0].num.clone(), terms[0].den.clone()), (big(3), big(1)));
        assert_eq!((terms[1].num.clone(), terms[1].den.clone()), (big(-3), big(1)));
    }

    #[test]
    fn toy_quadratic() {
        let c = [5i64, 7, 11];
        let pts = [(1, eval_toy(&c, 1, TOY_P)), (2, eval_toy(&c, 2, TOY_P)), (4, eval_toy(&c, 4, TOY_P))];
        assert_eq!(quadratic_through(&pts, TOY_P), c);
        assert_eq!(curve_for_guess(5, &[pts[0], pts[1]], TOY_P), c);
    }
}
