//! Chaumian e-cash with blind Diffie–Hellman key exchange (BDHKE).
//!
//! David Chaum's 1982 idea: a bank signs a coin it cannot see, so when the
//! coin comes back it cannot tell whose it was. This is the elliptic-curve
//! form David Wagner proposed on the Cypherpunks list in 1996 and that the
//! Cashu and Fedimint mints use today:
//!
//! ```text
//! wallet:  Y  = hash_to_curve(secret)
//!          B_ = Y + r·G                 (r: blinding factor, kept)
//! mint:    C_ = k·B_                    (k: the mint's key for this amount)
//! wallet:  C  = C_ − r·K   = k·Y        (K = k·G is public)
//! mint:    later accepts (secret, C) iff C == k·hash_to_curve(secret)
//! ```
//!
//! The mint sees `B_` at withdrawal and `(secret, C)` at deposit. Without `r`
//! the two are unrelated points: for *every* withdrawal and *every* deposit
//! there is a blinding factor that would connect them, so the mint's records
//! cannot link them. A DLEQ proof (Chaum–Pedersen) lets the wallet check that
//! the mint signed with its published key and not with a per-customer key
//! chosen to tag the coin.

use std::collections::BTreeSet;

use sha2::{Digest, Sha256};

use crate::bytes::{Big, bytes_to_big, from_hex, to_hex};
use crate::drbg::Drbg;
use crate::secp::{N, Pt, compressed, mod_n, mul, mul_pt, point_from_bytes, point_from_hex};

/// Domain separator for hashing a secret onto the curve (the one Cashu uses).
pub const DOMAIN: &[u8] = b"Secp256k1_HashToCurve_Cashu_";

/// Map a secret to a curve point nobody knows the discrete log of:
/// SHA-256 the secret with the domain tag, then try `02 ‖ SHA-256(h ‖ counter)`
/// as a compressed point until one is on the curve (about half are).
pub fn hash_to_curve(secret: &[u8]) -> Pt {
    let msg_hash: [u8; 32] = Sha256::new().chain_update(DOMAIN).chain_update(secret).finalize().into();
    for counter in 0u32..65_536 {
        let x: [u8; 32] = Sha256::new().chain_update(msg_hash).chain_update(counter.to_le_bytes()).finalize().into();
        let mut enc = [0u8; 33];
        enc[0] = 0x02;
        enc[1..].copy_from_slice(&x);
        if let Some(p) = point_from_bytes(&enc) {
            return p;
        }
    }
    unreachable!("no point found in 65536 tries")
}

/// B_ = Y + r·G
pub fn blind(secret: &[u8], r: &Big) -> Pt {
    hash_to_curve(secret) + mul(r)
}

/// C = C_ − r·K
pub fn unblind(c_blind: &Pt, r: &Big, k_pub: &Pt) -> Pt {
    *c_blind - mul_pt(r, k_pub)
}

/// The denominations the mint issues (powers of two, like every Cashu mint).
pub const AMOUNTS: [u64; 6] = [1, 2, 4, 8, 16, 32];

/// Split an amount into denominations, largest first.
pub fn split_amount(mut amount: u64) -> Vec<u64> {
    let mut out = Vec::new();
    for a in AMOUNTS.iter().rev() {
        while amount >= *a {
            out.push(*a);
            amount -= a;
        }
    }
    out
}

/// A Chaum–Pedersen proof that log_G(K) = log_B(C_) without revealing k.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dleq {
    pub e: Big,
    pub s: Big,
}

fn dleq_hash(r1: &Pt, r2: &Pt, k_pub: &Pt, c_blind: &Pt) -> Big {
    let mut h = Sha256::new();
    for p in [r1, r2, k_pub, c_blind] {
        h.update(compressed(p));
    }
    mod_n(&bytes_to_big(&h.finalize()))
}

/// Prove that `c_blind = k·b_blind` and `k_pub = k·G` use the same k.
pub fn dleq_prove(k: &Big, b_blind: &Pt, c_blind: &Pt, rng: &mut Drbg) -> Dleq {
    let p = rng.scalar(&N);
    let r1 = mul(&p);
    let r2 = mul_pt(&p, b_blind);
    let e = dleq_hash(&r1, &r2, &mul(k), c_blind);
    Dleq { s: mod_n(&(p + &e * k)), e }
}

/// Check a DLEQ proof: R1 = s·G − e·K, R2 = s·B_ − e·C_, and e must hash them.
pub fn dleq_verify(proof: &Dleq, k_pub: &Pt, b_blind: &Pt, c_blind: &Pt) -> bool {
    let r1 = mul(&proof.s) - mul_pt(&proof.e, k_pub);
    let r2 = mul_pt(&proof.s, b_blind) - mul_pt(&proof.e, c_blind);
    dleq_hash(&r1, &r2, k_pub, c_blind) == proof.e
}

/// A spendable token: an amount, a secret and the mint's unblinded signature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub amount: u64,
    /// hex-encoded secret (32 random bytes)
    pub secret: String,
    /// C = k·hash_to_curve(secret), compressed, hex
    pub c: String,
}

impl Token {
    pub fn secret_bytes(&self) -> Vec<u8> {
        from_hex(&self.secret).unwrap_or_default()
    }

    pub fn c_point(&self) -> Option<Pt> {
        point_from_hex(&self.c)
    }

    /// Y, the curve point the secret hashes to (what the mint keeps on its spent list).
    pub fn y(&self) -> Pt {
        hash_to_curve(&self.secret_bytes())
    }
}

/// A withdrawal in progress, as the wallet holds it.
#[derive(Clone, Debug)]
pub struct Pending {
    pub amount: u64,
    pub secret: Vec<u8>,
    pub r: Big,
    pub b_blind: Pt,
}

/// Start a withdrawal: a fresh secret and blinding factor, and the blinded point to send.
pub fn prepare(amount: u64, rng: &mut Drbg) -> Pending {
    let secret = rng.bytes(32);
    let r = rng.scalar(&N);
    let b_blind = blind(&secret, &r);
    Pending { amount, secret, r, b_blind }
}

/// Finish a withdrawal with the mint's blind signature.
pub fn finish(p: &Pending, c_blind: &Pt, k_pub: &Pt) -> Token {
    let c = unblind(c_blind, &p.r, k_pub);
    Token { amount: p.amount, secret: to_hex(&p.secret), c: to_hex(&compressed(&c)) }
}

/// What the mint wrote down, in order. This is all a subpoena can get.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LedgerEntry {
    /// a blinded point it signed, for this amount (and, if it logs, a name)
    Issued { amount: u64, b_blind: String, customer: Option<String> },
    /// a token it accepted: the secret and signature it was shown
    Redeemed { amount: u64, secret: String, c: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RedeemError {
    UnknownAmount,
    BadSignature,
    DoubleSpend,
}

impl std::fmt::Display for RedeemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            RedeemError::UnknownAmount => "the mint does not issue that amount",
            RedeemError::BadSignature => "C is not k·hash_to_curve(secret): not a coin this mint signed",
            RedeemError::DoubleSpend => "this secret is already on the spent list",
        })
    }
}

/// A mint: one key per denomination, a spent list, and a ledger.
#[derive(Clone, Debug)]
pub struct Mint {
    keys: Vec<(u64, Big)>,
    pub spent: BTreeSet<[u8; 33]>,
    pub ledger: Vec<LedgerEntry>,
    /// A "compliant" mint writes the customer's name next to every issue.
    pub logs_customers: bool,
}

impl Mint {
    pub fn new(rng: &mut Drbg) -> Self {
        let keys = AMOUNTS.iter().map(|a| (*a, rng.scalar(&N))).collect();
        Self { keys, spent: BTreeSet::new(), ledger: Vec::new(), logs_customers: false }
    }

    fn key(&self, amount: u64) -> Option<&Big> {
        self.keys.iter().find(|(a, _)| *a == amount).map(|(_, k)| k)
    }

    /// K for an amount (published).
    pub fn public_key(&self, amount: u64) -> Option<Pt> {
        self.key(amount).map(mul)
    }

    /// Sign a blinded point; returns C_ and a DLEQ proof.
    pub fn sign(&mut self, amount: u64, b_blind: &Pt, customer: &str, rng: &mut Drbg) -> Option<(Pt, Dleq)> {
        let k = self.key(amount)?.clone();
        let c_blind = mul_pt(&k, b_blind);
        let proof = dleq_prove(&k, b_blind, &c_blind, rng);
        self.ledger.push(LedgerEntry::Issued { amount, b_blind: to_hex(&compressed(b_blind)), customer: self.logs_customers.then(|| customer.to_string()) });
        Some((c_blind, proof))
    }

    /// Would this token be accepted (without spending it)?
    pub fn check(&self, t: &Token) -> Result<(), RedeemError> {
        let k = self.key(t.amount).ok_or(RedeemError::UnknownAmount)?;
        let c = t.c_point().ok_or(RedeemError::BadSignature)?;
        let y = t.y();
        if mul_pt(k, &y) != c {
            return Err(RedeemError::BadSignature);
        }
        if self.spent.contains(&compressed(&y)) {
            return Err(RedeemError::DoubleSpend);
        }
        Ok(())
    }

    /// Accept a token and put its Y on the spent list.
    pub fn redeem(&mut self, t: &Token) -> Result<(), RedeemError> {
        self.check(t)?;
        self.spent.insert(compressed(&t.y()));
        self.ledger.push(LedgerEntry::Redeemed { amount: t.amount, secret: t.secret.clone(), c: t.c.clone() });
        Ok(())
    }

    /// Withdraw `amount` for a customer in one go (wallet and mint in the same
    /// process: used for rewards and by tests). Verifies every DLEQ proof.
    pub fn withdraw(&mut self, amount: u64, customer: &str, rng: &mut Drbg) -> Vec<Token> {
        let mut out = Vec::new();
        for a in split_amount(amount) {
            let p = prepare(a, rng);
            let (c_blind, proof) = self.sign(a, &p.b_blind, customer, rng).expect("known amount");
            let k_pub = self.public_key(a).expect("known amount");
            assert!(dleq_verify(&proof, &k_pub, &p.b_blind, &c_blind), "mint proof");
            out.push(finish(&p, &c_blind, &k_pub));
        }
        out
    }
}

/// Total value of a set of tokens.
pub fn total(tokens: &[Token]) -> u64 {
    tokens.iter().map(|t| t.amount).sum()
}

/// Choose tokens covering at least `amount` (largest first). Returns their indices.
pub fn select(tokens: &[Token], amount: u64) -> Option<Vec<usize>> {
    let mut idx: Vec<usize> = (0..tokens.len()).collect();
    idx.sort_by(|a, b| tokens[*b].amount.cmp(&tokens[*a].amount));
    let mut picked = Vec::new();
    let mut sum = 0;
    for i in idx {
        if sum >= amount {
            break;
        }
        picked.push(i);
        sum += tokens[i].amount;
    }
    (sum >= amount).then_some(picked)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_to_curve_is_deterministic_and_separated() {
        let a = hash_to_curve(b"test");
        assert_eq!(a, hash_to_curve(b"test"));
        assert_ne!(a, hash_to_curve(b"tesu"));
    }

    #[test]
    fn blind_sign_unblind_verifies() {
        let mut rng = Drbg::new(b"seed", "ecash");
        let mut mint = Mint::new(&mut rng);
        let p = prepare(8, &mut rng);
        let (c_blind, proof) = mint.sign(8, &p.b_blind, "alice", &mut rng).unwrap();
        let k_pub = mint.public_key(8).unwrap();
        assert!(dleq_verify(&proof, &k_pub, &p.b_blind, &c_blind));
        let t = finish(&p, &c_blind, &k_pub);
        assert_eq!(mint.check(&t), Ok(()));
        // unblinding with the wrong factor gives a token the mint rejects
        let wrong = Token { c: to_hex(&compressed(&unblind(&c_blind, &(p.r.clone() + 1), &k_pub))), ..t.clone() };
        assert_eq!(mint.check(&wrong), Err(RedeemError::BadSignature));
        // spend once, then never again
        assert_eq!(mint.redeem(&t), Ok(()));
        assert_eq!(mint.redeem(&t), Err(RedeemError::DoubleSpend));
        // a token for the wrong denomination fails
        let other = Token { amount: 4, ..t };
        assert_eq!(mint.check(&other), Err(RedeemError::BadSignature));
    }

    #[test]
    fn mint_records_do_not_contain_the_token() {
        let mut rng = Drbg::new(b"seed", "ledger");
        let mut mint = Mint::new(&mut rng);
        let tokens = mint.withdraw(13, "alice", &mut rng);
        assert_eq!(total(&tokens), 13);
        assert_eq!(tokens.iter().map(|t| t.amount).collect::<Vec<_>>(), vec![8, 4, 1]);
        for t in &tokens {
            mint.redeem(t).unwrap();
        }
        // nothing the mint saw at issue time appears at redeem time
        let issued: Vec<&String> = mint.ledger.iter().filter_map(|e| if let LedgerEntry::Issued { b_blind, .. } = e { Some(b_blind) } else { None }).collect();
        for t in &tokens {
            assert!(!issued.contains(&&t.c));
            assert!(!issued.contains(&&to_hex(&compressed(&t.y()))));
        }
    }

    #[test]
    fn a_tagging_mint_is_caught_by_dleq() {
        let mut rng = Drbg::new(b"seed", "tag");
        let mint = Mint::new(&mut rng);
        let k_pub = mint.public_key(4).unwrap();
        let p = prepare(4, &mut rng);
        // the mint signs with a different, per-customer key
        let k_tag = rng.scalar(&N);
        let c_blind = mul_pt(&k_tag, &p.b_blind);
        let proof = dleq_prove(&k_tag, &p.b_blind, &c_blind, &mut rng);
        assert!(!dleq_verify(&proof, &k_pub, &p.b_blind, &c_blind));
    }

    #[test]
    fn select_covers_the_amount() {
        let mut rng = Drbg::new(b"seed", "select");
        let mut mint = Mint::new(&mut rng);
        let tokens = mint.withdraw(21, "x", &mut rng);
        let pick = select(&tokens, 6).unwrap();
        assert!(pick.iter().map(|i| tokens[*i].amount).sum::<u64>() >= 6);
        assert!(select(&tokens, 22).is_none());
    }
}
