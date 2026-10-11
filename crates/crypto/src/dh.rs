//! Finite-field Diffie–Hellman in prime-order subgroups of Z_p^*, with p = 2q + 1
//! a safe prime and g = 4 (a quadratic residue, hence of order q).
//!
//!   Alice: a, A = g^a      Bob: b, B = g^b      shared K = B^a = A^b = g^(ab)
//!
//! Nothing in the exchange says *who* sent A or B. A relay that swaps both for
//! its own g^m ends up sharing g^(am) with Alice and g^(bm) with Bob: the
//! man-in-the-middle attack. The cure is authentication (signatures, Chapter 4).
//!
//! All the groups in the game fit in 64 bits, so plain `u64` arithmetic (with
//! 128-bit products) is used throughout.

use std::collections::HashMap;

use sha2::{Digest, Sha256};

use crate::bytes::{Big, pow_mod_u64};
use crate::drbg::Drbg;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Group {
    pub name: &'static str,
    pub p: u64,
    pub q: u64,
    pub g: u64,
}

/// Small enough to do by hand (or by a very patient cat).
pub const TOY_GROUP: Group = Group { name: "toy", p: 1019, q: 509, g: 4 };

/// 31-bit subgroup order: breakable in milliseconds with baby-step giant-step.
pub const WEAK_GROUP: Group = Group { name: "weak-33", p: 4_294_992_683, q: 2_147_496_341, g: 4 };

/// 64-bit safe prime: big enough that the MITM is the easier attack in-game.
pub const DRONE_GROUP: Group = Group { name: "drone-64", p: 18_446_741_874_686_296_103, q: 9_223_370_937_343_148_051, g: 4 };

impl Group {
    pub fn pow(&self, base: u64, e: u64) -> u64 {
        pow_mod_u64(base, e % self.q, self.p)
    }

    pub fn public(&self, secret: u64) -> u64 {
        self.pow(self.g, secret)
    }

    pub fn is_element(&self, y: u64) -> bool {
        y > 1 && y < self.p && pow_mod_u64(y, self.q, self.p) == 1
    }

    pub fn keypair(&self, rng: &mut Drbg) -> Keypair {
        let secret = rng.range(&Big::from(2), &Big::from(self.q - 1));
        let secret = u64::try_from(secret).expect("fits");
        Keypair { secret, public: self.public(secret) }
    }

    pub fn shared(&self, my_secret: u64, their_public: u64) -> u64 {
        self.pow(their_public, my_secret)
    }

    /// Session key = SHA-256(big-endian bytes of K).
    pub fn session_key(&self, k: u64) -> [u8; 32] {
        let len = (64 - self.p.leading_zeros()).div_ceil(8) as usize;
        let bytes = k.to_be_bytes();
        Sha256::digest(&bytes[8 - len..]).into()
    }

    pub fn bits(&self) -> u32 {
        64 - self.q.leading_zeros()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Keypair {
    pub secret: u64,
    pub public: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BsgsPhase {
    Baby,
    Giant,
    Done,
}

/// Baby-step giant-step, runnable a slice at a time so a UI can show progress.
///
/// Solve g^x = y in a group of order q with ~2*sqrt(q) multiplications and
/// sqrt(q) memory. Write x = i*m + j with m = ceil(sqrt q): store g^j for all
/// j, then walk y * (g^-m)^i until a stored value appears.
pub struct Bsgs {
    pub group: Group,
    pub m: u64,
    pub phase: BsgsPhase,
    table: HashMap<u64, u64>,
    j: u64,
    e: u64,
    i: u64,
    gamma: u64,
    factor: u64,
    pub giants_total: u64,
    pub result: Option<u64>,
}

impl Bsgs {
    pub fn new(group: Group, y: u64, m: u64) -> Self {
        let m = m.max(1);
        let factor = pow_mod_u64(group.g, group.q - (m % group.q), group.p); // g^(-m)
        Self {
            group,
            m,
            phase: BsgsPhase::Baby,
            table: HashMap::with_capacity(m.min(1 << 22) as usize),
            j: 0,
            e: 1,
            i: 0,
            gamma: y % group.p,
            factor,
            giants_total: group.q.div_ceil(m),
            result: None,
        }
    }

    /// Do up to `budget` steps of work. Returns true when finished.
    pub fn step(&mut self, budget: u64) -> bool {
        let p = self.group.p;
        let mut left = budget;
        while left > 0 {
            left -= 1;
            match self.phase {
                BsgsPhase::Baby => {
                    if self.j >= self.m {
                        self.phase = BsgsPhase::Giant;
                        continue;
                    }
                    self.table.entry(self.e).or_insert(self.j);
                    self.e = crate::bytes::mul_mod_u64(self.e, self.group.g, p);
                    self.j += 1;
                }
                BsgsPhase::Giant => {
                    if self.i > self.giants_total {
                        self.phase = BsgsPhase::Done;
                        return true;
                    }
                    if let Some(&j) = self.table.get(&self.gamma) {
                        let x = ((self.i as u128 * self.m as u128 + j as u128) % self.group.q as u128) as u64;
                        self.result = Some(x);
                        self.phase = BsgsPhase::Done;
                        return true;
                    }
                    self.gamma = crate::bytes::mul_mod_u64(self.gamma, self.factor, p);
                    self.i += 1;
                }
                BsgsPhase::Done => return true,
            }
        }
        self.phase == BsgsPhase::Done
    }

    pub fn baby_fraction(&self) -> f32 {
        (self.j as f64 / self.m as f64).min(1.0) as f32
    }

    pub fn giant_fraction(&self) -> f32 {
        if self.phase == BsgsPhase::Done {
            return 1.0;
        }
        (self.i as f64 / self.giants_total.max(1) as f64).min(1.0) as f32
    }

    pub fn baby_steps(&self) -> u64 {
        self.j
    }

    pub fn giant_steps(&self) -> u64 {
        self.i + 1
    }

    pub fn run(mut self) -> Option<u64> {
        while !self.step(1 << 20) {}
        self.result
    }
}

/// ceil(sqrt(q)), the balanced choice of m.
pub fn sqrt_ceil(q: u64) -> u64 {
    let mut r = (q as f64).sqrt() as u64;
    while r * r < q {
        r += 1;
    }
    while r > 0 && (r - 1) * (r - 1) >= q {
        r -= 1;
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytes::{big, is_probable_prime};

    #[test]
    fn groups_are_safe_primes() {
        for g in [TOY_GROUP, WEAK_GROUP, DRONE_GROUP] {
            assert!(is_probable_prime(&big(g.p as i64)) || g.p > i64::MAX as u64);
            assert_eq!(g.p, 2 * g.q + 1);
            assert!(is_probable_prime(&Big::from(g.q)));
            assert!(is_probable_prime(&Big::from(g.p)));
            assert_eq!(pow_mod_u64(g.g, g.q, g.p), 1);
        }
    }

    #[test]
    fn exchange_and_bsgs() {
        let mut r = Drbg::new(&[1u8; 32], "dh");
        let a = TOY_GROUP.keypair(&mut r);
        let b = TOY_GROUP.keypair(&mut r);
        assert_eq!(TOY_GROUP.shared(a.secret, b.public), TOY_GROUP.shared(b.secret, a.public));
        let secret = 1_234_567_891 % WEAK_GROUP.q;
        let y = WEAK_GROUP.public(secret);
        let x = Bsgs::new(WEAK_GROUP, y, sqrt_ceil(WEAK_GROUP.q)).run().unwrap();
        assert_eq!(WEAK_GROUP.public(x), y);
        assert_eq!(x, secret);
    }
}
