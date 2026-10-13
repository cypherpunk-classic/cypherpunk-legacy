//! The Schnorr identification protocol, a Sigma protocol for "I know x with X = x*G".
//!
//!   Prover                         Verifier
//!   r random, R = r*G   --R-->
//!                       <--c--     c random
//!   s = r + c*x         --s-->     accept iff s*G == R + c*X
//!
//! Completeness: s*G = r*G + c*x*G = R + c*X.
//! Special soundness: two accepting transcripts (R, c1, s1), (R, c2, s2) with
//!   c1 != c2 give x = (s1 - s2)/(c1 - c2): whoever can answer two challenges
//!   for one commitment knows x. (The extractor is the nonce-reuse attack.)
//! Honest-verifier zero knowledge: pick c and s first, set R = s*G - c*X; the
//!   simulated transcript has exactly the distribution of a real one.
//! Fiat–Shamir: replace the verifier with c = H(R || X || context).

use num_traits::Zero;

use crate::bytes::{Big, bytes_to_big, concat};
use crate::drbg::Drbg;
use crate::secp::{N, Pt, compressed, inv_n, mod_n, mul, mul_pt, tagged_hash};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transcript {
    pub r_point: Pt,
    pub c: Big,
    pub s: Big,
}

pub fn commit(rng: &mut Drbg) -> (Big, Pt) {
    let r = rng.scalar(&N);
    let rp = mul(&r);
    (r, rp)
}

pub fn respond(r: &Big, c: &Big, x: &Big) -> Big {
    mod_n(&(r + c * x))
}

pub fn verify_transcript(x_pub: &Pt, t: &Transcript) -> bool {
    mul(&t.s) == t.r_point + mul_pt(&t.c, x_pub)
}

/// A transcript produced without the secret. Indistinguishable from a real one.
pub fn simulate(x_pub: &Pt, c: &Big, s: &Big) -> Transcript {
    Transcript { r_point: mul(s) - mul_pt(c, x_pub), c: mod_n(c), s: mod_n(s) }
}

pub fn extract(t1: &Transcript, t2: &Transcript) -> Result<Big, String> {
    if t1.r_point != t2.r_point {
        return Err("transcripts must share the commitment".into());
    }
    if mod_n(&(&t1.c - &t2.c)).is_zero() {
        return Err("challenges must differ".into());
    }
    Ok(mod_n(&((&t1.s - &t2.s) * inv_n(&(&t1.c - &t2.c)))))
}

/// A cheater who does not know x bets on the challenge bit in advance:
///  - bets 0: R = r*G and answers s = r (works only if c = 0)
///  - bets 1: picks s, sets R = s*G - X   (works only if c = 1)
pub fn cheater_commit(x_pub: &Pt, bet: u8, rng: &mut Drbg) -> (Pt, Big) {
    let s = rng.scalar(&N);
    let r = if bet == 0 { mul(&s) } else { mul(&s) - x_pub };
    (r, s)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FsOptions {
    pub include_key: bool,
    pub include_context: bool,
}

impl Default for FsOptions {
    fn default() -> Self {
        Self { include_key: true, include_context: true }
    }
}

pub fn fs_challenge(r: &Pt, x_pub: &Pt, context: &str, opts: FsOptions) -> Big {
    let rc = compressed(r);
    let xc = compressed(x_pub);
    let mut parts: Vec<&[u8]> = vec![&rc];
    if opts.include_key {
        parts.push(&xc);
    }
    if opts.include_context {
        parts.push(context.as_bytes());
    }
    let joined = concat(&parts);
    mod_n(&bytes_to_big(&tagged_hash("CypherpunkLegacy/fiat-shamir", &[&joined])))
}

pub fn fs_prove(x: &Big, x_pub: &Pt, context: &str, rng: &mut Drbg, opts: FsOptions) -> Transcript {
    let (r, rp) = commit(rng);
    let c = fs_challenge(&rp, x_pub, context, opts);
    Transcript { r_point: rp, s: respond(&r, &c, x), c }
}

pub fn fs_verify(x_pub: &Pt, context: &str, t: &Transcript, opts: FsOptions) -> bool {
    t.c == fs_challenge(&t.r_point, x_pub, context, opts) && verify_transcript(x_pub, t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sigma_protocol_properties() {
        let mut rng = Drbg::new(&[3u8; 32], "sigma");
        let x = rng.scalar(&N);
        let xp = mul(&x);
        let (r, rp) = commit(&mut rng);
        let c1 = rng.scalar(&N);
        let c2 = rng.scalar(&N);
        let t1 = Transcript { r_point: rp, c: c1.clone(), s: respond(&r, &c1, &x) };
        let t2 = Transcript { r_point: rp, c: c2.clone(), s: respond(&r, &c2, &x) };
        assert!(verify_transcript(&xp, &t1));
        assert_eq!(extract(&t1, &t2).unwrap(), x);
        let sim = simulate(&xp, &rng.scalar(&N), &rng.scalar(&N));
        assert!(verify_transcript(&xp, &sim));
        let t = fs_prove(&x, &xp, "ctx", &mut rng, FsOptions::default());
        assert!(fs_verify(&xp, "ctx", &t, FsOptions::default()));
        assert!(!fs_verify(&xp, "other", &t, FsOptions::default()));
        let weak = FsOptions { include_key: true, include_context: false };
        let tw = fs_prove(&x, &xp, "ctx", &mut rng, weak);
        assert!(fs_verify(&xp, "anything", &tw, weak));
    }
}
