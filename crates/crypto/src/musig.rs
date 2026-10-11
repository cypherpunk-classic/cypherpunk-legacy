//! Key aggregation for multi-signatures.
//!
//! Naive aggregation Q = P1 + P2 is broken by the rogue-key attack: whoever
//! announces their key last can announce P2 = P_attacker - P1, making
//! Q = P_attacker, a key they alone control.
//!
//! BIP-327 (MuSig2) KeyAgg weights every key by a coefficient that depends on
//! the whole key list:  a_i = H_coef(L || P_i),  L = H_list(P_1 || ... || P_u),
//! so changing your own key changes everybody's coefficients and the attacker
//! is left chasing a moving target. (The second distinct key gets a_i = 1, a
//! BIP-327 optimisation that keeps the scheme secure.)

use num_traits::One;

use crate::bytes::{Big, bytes_to_big};
use crate::secp::{Pt, compressed, infinity, is_infinity, mod_n, mul_pt, point_from_bytes, tagged_hash};

pub fn naive_aggregate(keys: &[Pt]) -> Pt {
    keys.iter().fold(infinity(), |acc, p| acc + p)
}

/// The rogue key: announce P_attacker - sum(other keys).
pub fn rogue_key(target: &Pt, others: &[Pt]) -> Pt {
    *target - naive_aggregate(others)
}

pub fn hash_keys(pks: &[[u8; 33]]) -> [u8; 32] {
    let joined: Vec<u8> = pks.concat();
    tagged_hash("KeyAgg list", &[&joined])
}

pub fn second_key(pks: &[[u8; 33]]) -> [u8; 33] {
    let first = pks[0];
    pks.iter().copied().find(|pk| pk != &first).unwrap_or([0u8; 33])
}

pub fn key_agg_coeff(pks: &[[u8; 33]], pk: &[u8; 33]) -> Big {
    if pk == &second_key(pks) {
        return Big::one();
    }
    let l = hash_keys(pks);
    mod_n(&bytes_to_big(&tagged_hash("KeyAgg coefficient", &[&l, pk])))
}

#[derive(Clone, Debug)]
pub struct KeyAggResult {
    pub q: Pt,
    pub coefficients: Vec<Big>,
    pub l: [u8; 32],
}

/// BIP-327 KeyAgg over 33-byte compressed public keys (order matters).
pub fn key_agg(pks: &[[u8; 33]]) -> Result<KeyAggResult, String> {
    let l = hash_keys(pks);
    let coefficients: Vec<Big> = pks.iter().map(|pk| key_agg_coeff(pks, pk)).collect();
    let mut q = infinity();
    for (pk, a) in pks.iter().zip(&coefficients) {
        let p = point_from_bytes(pk).ok_or("invalid public key")?;
        q += mul_pt(a, &p);
    }
    if is_infinity(&q) {
        return Err("aggregate key is infinity".into());
    }
    Ok(KeyAggResult { q, coefficients, l })
}

pub fn key_agg_points(keys: &[Pt]) -> Result<KeyAggResult, String> {
    key_agg(&keys.iter().map(compressed).collect::<Vec<_>>())
}

pub struct RogueAttempt {
    pub announced: Pt,
    pub q: Pt,
    pub hits_target: bool,
}

/// What happens if an attacker tries the naive rogue-key trick against KeyAgg:
/// they announce P2 = T - P1, but the aggregate is a1*P1 + a2*P2, not P1 + P2.
/// Iterating "fix it up with the new coefficient" never converges.
pub fn rogue_attempt_against_key_agg(p1: &Pt, target: &Pt, rounds: usize) -> Vec<RogueAttempt> {
    let mut out = Vec::new();
    let mut a1 = Big::one();
    for _ in 0..rounds {
        let announced = *target - mul_pt(&a1, p1);
        let res = key_agg_points(&[*p1, announced]).expect("non-degenerate");
        out.push(RogueAttempt { announced, q: res.q, hits_target: res.q == *target });
        a1 = res.coefficients[0].clone();
    }
    out
}
