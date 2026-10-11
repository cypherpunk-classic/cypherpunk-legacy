//! BIP-340 Schnorr signatures over secp256k1.
//!
//!   sign:   R = k*G (k negated so R has even y), e = H_challenge(R.x || P.x || m) mod n,
//!           s = k + e*d mod n                    signature = R.x || s
//!   verify: s*G == R + e*P
//!
//! Real signing and verification go through `k256::schnorr` (deterministic
//! nonce derived from the key, aux randomness and the message).
//! [`sign_with_nonce`] exists only so the game can reproduce a broken device
//! that reuses k, the bug that leaks the key.

use k256::schnorr::{Signature, SigningKey, VerifyingKey};

use crate::bytes::{Big, big_to_bytes, bytes_to_big, concat, to_hex};
use crate::drbg::random_bytes;
use crate::secp::{N, Pt, has_even_y, inv_n, lift_x, mod_n, mul, mul_pt, tagged_hash, x_only};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sig {
    /// R.x, 32 bytes
    pub r: [u8; 32],
    pub s: Big,
}

impl Sig {
    pub fn bytes(&self) -> [u8; 64] {
        concat(&[&self.r, &big_to_bytes(&self.s, 32)]).try_into().unwrap()
    }

    pub fn hex(&self) -> String {
        to_hex(&self.bytes())
    }

    pub fn parse(b: &[u8]) -> Result<Self, String> {
        if b.len() != 64 {
            return Err("signature must be 64 bytes".into());
        }
        Ok(Self { r: b[..32].try_into().unwrap(), s: bytes_to_big(&b[32..]) })
    }
}

/// BIP-340 secret key normalisation: the key whose public point has even y.
pub fn even_key(d: &Big) -> Big {
    let p = mul(d);
    if has_even_y(&p) { mod_n(d) } else { &*N - mod_n(d) }
}

/// e = int(hash_BIP0340/challenge(R.x || P.x || m)) mod n
pub fn challenge(rx: &[u8], px: &[u8], m: &[u8]) -> Big {
    mod_n(&bytes_to_big(&tagged_hash("BIP0340/challenge", &[rx, px, m])))
}

/// Standard BIP-340 signing (k256). Without aux randomness, fresh random bytes are used.
pub fn sign(m: &[u8], secret_key: &Big, aux_rand: Option<[u8; 32]>) -> Sig {
    let sk_bytes: [u8; 32] = big_to_bytes(&mod_n(secret_key), 32).try_into().unwrap();
    let sk = SigningKey::from_bytes(&sk_bytes.into()).expect("valid secret key");
    let aux = aux_rand.unwrap_or_else(random_bytes::<32>);
    let sig = sk.sign_raw(m, &aux).expect("signing succeeds");
    Sig::parse(&sig.to_bytes()).unwrap()
}

/// Standard BIP-340 verification (k256).
pub fn verify_bytes(sig: &[u8], m: &[u8], pub_x: &[u8]) -> bool {
    let Ok(vk) = VerifyingKey::from_slice(pub_x) else { return false };
    let Ok(sig) = Signature::try_from(sig) else { return false };
    vk.verify_raw(m, &sig).is_ok()
}

pub fn verify(sig: &Sig, m: &[u8], pub_x: &[u8]) -> bool {
    verify_bytes(&sig.bytes(), m, pub_x)
}

pub fn public_key(secret_key: &Big) -> [u8; 32] {
    x_only(&mul(secret_key))
}

/// Sign with a caller-chosen nonce k. Never do this outside a museum of broken devices.
pub fn sign_with_nonce(m: &[u8], secret_key: &Big, k0: &Big) -> Sig {
    let d = even_key(secret_key);
    let p = mul(&d);
    let mut k = mod_n(k0);
    let r = mul(&k);
    if !has_even_y(&r) {
        k = &*N - k;
    }
    let rx = x_only(&r);
    let e = challenge(&rx, &x_only(&p), m);
    Sig { r: rx, s: mod_n(&(k + e * d)) }
}

/// The pieces of the verification equation, for showing on screen.
pub struct Explained {
    pub e: Big,
    pub lhs: Pt,
    pub rhs: Pt,
    pub ok: bool,
}

pub fn explain_verify(sig: &Sig, m: &[u8], pub_x: &[u8]) -> Option<Explained> {
    let p = lift_x(pub_x)?;
    let r = lift_x(&sig.r)?;
    let e = challenge(&sig.r, pub_x, m);
    let lhs = mul(&sig.s);
    let rhs = r + mul_pt(&e, &p);
    Some(Explained { e, ok: lhs == rhs, lhs, rhs })
}

#[derive(Clone, Debug)]
pub struct NonceReuseSolution {
    pub e1: Big,
    pub e2: Big,
    pub s_diff: Big,
    pub e_diff: Big,
    pub e_diff_inv: Big,
    pub d: Big,
}

/// Two signatures with the same R under the same key:
///   s1 = k + e1*d,  s2 = k + e2*d   =>   d = (s1 - s2) / (e1 - e2)  mod n
pub fn recover_from_nonce_reuse(pub_x: &[u8], m1: &[u8], sig1: &Sig, m2: &[u8], sig2: &Sig) -> Result<NonceReuseSolution, String> {
    if sig1.r != sig2.r {
        return Err("signatures do not share a nonce".into());
    }
    let e1 = challenge(&sig1.r, pub_x, m1);
    let e2 = challenge(&sig2.r, pub_x, m2);
    if e1 == e2 {
        return Err("identical challenges".into());
    }
    let s_diff = mod_n(&(&sig1.s - &sig2.s));
    let e_diff = mod_n(&(&e1 - &e2));
    let e_diff_inv = inv_n(&e_diff);
    let d = mod_n(&(&s_diff * &e_diff_inv));
    Ok(NonceReuseSolution { e1, e2, s_diff, e_diff, e_diff_inv, d })
}
