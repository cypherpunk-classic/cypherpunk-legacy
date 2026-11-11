//! The web of trust: key fingerprints, key IDs, certifications, and validity
//! computed the way PGP and GnuPG do it.
//!
//! Phil Zimmermann's answer (PGP, 1991) to "how do I know this key is really
//! hers?" was not a certificate authority but people: you sign the keys of
//! people you have checked in person, and you decide whose signatures you are
//! willing to rely on. A key is valid for you if you signed it, or if it is
//! signed by one key you trust fully, or by three you trust marginally, along
//! a chain at most five long (GnuPG's defaults).
//!
//! Fingerprints are the 20-byte digest of a version byte, the creation time
//! and the public key, like OpenPGP v4 (which uses SHA-1; SHA-256 truncated
//! here). The *short key ID* is the end of the fingerprint. Real short IDs were
//! 8 hex digits, and in 2014 the Evil32 project generated a colliding key for
//! every key in the strong set: anyone who checked short IDs instead of
//! fingerprints could be fooled. This city's keyserver uses 4-digit short IDs,
//! so a collision takes about 65,536 hashes instead of 4 billion. The lesson is
//! the same: compare the whole fingerprint.

use sha2::{Digest, Sha256};

use crate::bip340::{Sig, sign, verify};
use crate::bytes::{Big, to_hex};
use crate::secp::tagged_hash;

pub const FP_BYTES: usize = 20;
/// Bytes of the fingerprint used as a short ID (4 hex digits in this city).
pub const SHORT_ID_BYTES: usize = 2;
/// Bytes of the fingerprint used as a long ID (16 hex digits).
pub const LONG_ID_BYTES: usize = 8;

pub type Fingerprint = [u8; FP_BYTES];

/// v4-style fingerprint: H(0x04 ‖ created ‖ public key), first 20 bytes.
pub fn fingerprint(xonly: &[u8; 32], created: u32) -> Fingerprint {
    let h = Sha256::new().chain_update([0x04u8]).chain_update(created.to_be_bytes()).chain_update(xonly).finalize();
    h[..FP_BYTES].try_into().unwrap()
}

/// Upper-case hex in groups of four, the way fingerprints are printed on slips.
pub fn fp_display(fp: &Fingerprint) -> String {
    let h = to_hex(fp).to_uppercase();
    h.as_bytes().chunks(4).map(|c| std::str::from_utf8(c).unwrap()).collect::<Vec<_>>().join(" ")
}

pub fn short_id(fp: &Fingerprint) -> String {
    to_hex(&fp[FP_BYTES - SHORT_ID_BYTES..]).to_uppercase()
}

pub fn long_id(fp: &Fingerprint) -> String {
    to_hex(&fp[FP_BYTES - LONG_ID_BYTES..]).to_uppercase()
}

/// Search creation times for a key whose short ID matches `target`'s (the
/// Evil32 trick: keep your own key, vary what else goes into the fingerprint).
/// Returns (created, fingerprint) and the number of hashes it took.
pub fn collide_short_id(target: &Fingerprint, xonly: &[u8; 32], start: u32) -> (u32, Fingerprint, u64) {
    let want = &target[FP_BYTES - SHORT_ID_BYTES..];
    let mut created = start;
    let mut tries = 0u64;
    loop {
        tries += 1;
        let fp = fingerprint(xonly, created);
        if &fp[FP_BYTES - SHORT_ID_BYTES..] == want && fp != *target {
            return (created, fp, tries);
        }
        created = created.wrapping_add(1);
    }
}

/// The message a certification signs: who vouches, for which key, for which name.
pub fn cert_message(signer: &Fingerprint, target: &Fingerprint, uid: &str) -> [u8; 32] {
    tagged_hash("CPL/certification", &[signer, target, uid.as_bytes()])
}

/// Sign someone's key with your secret key.
pub fn certify(signer_secret: &Big, signer: &Fingerprint, target: &Fingerprint, uid: &str) -> Sig {
    let mut aux = [0u8; 32];
    aux[..FP_BYTES].copy_from_slice(target);
    sign(&cert_message(signer, target, uid), signer_secret, Some(aux))
}

pub fn verify_cert(sig: &Sig, signer_xonly: &[u8; 32], signer: &Fingerprint, target: &Fingerprint, uid: &str) -> bool {
    verify(sig, &cert_message(signer, target, uid), signer_xonly)
}

/// How much you rely on a key's owner to check other people's keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum OwnerTrust {
    Unknown,
    Never,
    Marginal,
    Full,
    /// your own key
    Ultimate,
}

impl OwnerTrust {
    pub fn label(self) -> &'static str {
        match self {
            OwnerTrust::Unknown => "unknown",
            OwnerTrust::Never => "never",
            OwnerTrust::Marginal => "marginal",
            OwnerTrust::Full => "full",
            OwnerTrust::Ultimate => "ultimate",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Node {
    pub uid: String,
    pub fp: Fingerprint,
    pub trust: OwnerTrust,
}

/// Why a key is (or is not) valid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Validity {
    /// your own key
    Ultimate,
    /// valid at this distance from you
    Valid { depth: u32, full: usize, marginal: usize },
    /// signed, but not by enough trusted keys
    Partial { full: usize, marginal: usize },
    Unknown,
}

impl Validity {
    pub fn is_valid(&self) -> bool {
        matches!(self, Validity::Ultimate | Validity::Valid { .. })
    }
}

pub const MARGINALS_NEEDED: usize = 3;
pub const COMPLETES_NEEDED: usize = 1;
pub const MAX_DEPTH: u32 = 5;

/// Compute validity for every node given certifications (signer, target).
pub fn validity(nodes: &[Node], certs: &[(usize, usize)]) -> Vec<Validity> {
    let n = nodes.len();
    let mut out: Vec<Validity> = nodes.iter().map(|k| if k.trust == OwnerTrust::Ultimate { Validity::Ultimate } else { Validity::Unknown }).collect();
    for depth in 1..=MAX_DEPTH {
        let mut changed = false;
        let snapshot = out.clone();
        for t in 0..n {
            if snapshot[t].is_valid() {
                continue;
            }
            let (mut full, mut marginal) = (0, 0);
            for &(s, tt) in certs {
                if tt != t || s == t || !snapshot[s].is_valid() {
                    continue;
                }
                match nodes[s].trust {
                    OwnerTrust::Ultimate | OwnerTrust::Full => full += 1,
                    OwnerTrust::Marginal => marginal += 1,
                    _ => {}
                }
            }
            if full >= COMPLETES_NEEDED || marginal >= MARGINALS_NEEDED {
                out[t] = Validity::Valid { depth, full, marginal };
                changed = true;
            } else if full + marginal > 0 {
                out[t] = Validity::Partial { full, marginal };
            }
        }
        if !changed {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drbg::Drbg;
    use crate::secp::{N, mul, x_only};

    fn node(uid: &str, rng: &mut Drbg, trust: OwnerTrust) -> (Node, Big, [u8; 32]) {
        let sk = rng.scalar(&N);
        let x = x_only(&mul(&sk));
        (Node { uid: uid.into(), fp: fingerprint(&x, 1_000_000), trust }, sk, x)
    }

    #[test]
    fn fingerprints_and_ids() {
        let fp = fingerprint(&[7; 32], 42);
        assert_eq!(fp_display(&fp).len(), 40 + 9);
        assert_eq!(short_id(&fp).len(), 4);
        assert_eq!(long_id(&fp).len(), 16);
        assert!(long_id(&fp).ends_with(&short_id(&fp)));
        assert_ne!(fingerprint(&[7; 32], 43), fp);
    }

    #[test]
    fn short_ids_collide_full_fingerprints_do_not() {
        let target = fingerprint(&[1; 32], 500);
        let (created, fp, tries) = collide_short_id(&target, &[2; 32], 0);
        assert_eq!(short_id(&fp), short_id(&target));
        assert_ne!(fp, target);
        assert_eq!(fingerprint(&[2; 32], created), fp);
        assert!(tries < 2_000_000);
    }

    #[test]
    fn certifications_verify() {
        let mut rng = Drbg::new(b"seed", "wot");
        let (a, ask, ax) = node("you", &mut rng, OwnerTrust::Ultimate);
        let (b, _, _) = node("felix", &mut rng, OwnerTrust::Full);
        let sig = certify(&ask, &a.fp, &b.fp, &b.uid);
        assert!(verify_cert(&sig, &ax, &a.fp, &b.fp, &b.uid));
        assert!(!verify_cert(&sig, &ax, &a.fp, &b.fp, "mallory"));
    }

    #[test]
    fn gnupg_rules() {
        let mut rng = Drbg::new(b"seed", "wot2");
        let t = |r: &mut Drbg, u: &str, tr| node(u, r, tr).0;
        let nodes = vec![
            t(&mut rng, "you", OwnerTrust::Ultimate), // 0
            t(&mut rng, "felix", OwnerTrust::Full),   // 1
            t(&mut rng, "mara", OwnerTrust::Marginal), // 2
            t(&mut rng, "juno", OwnerTrust::Marginal), // 3
            t(&mut rng, "rosa", OwnerTrust::Marginal), // 4
            t(&mut rng, "wen", OwnerTrust::Unknown),  // 5: signed by three marginals
            t(&mut rng, "theo", OwnerTrust::Unknown), // 6: signed by felix only
            t(&mut rng, "eve", OwnerTrust::Unknown),  // 7: signed by two marginals
        ];
        let certs = vec![(0, 1), (0, 2), (0, 3), (0, 4), (2, 5), (3, 5), (4, 5), (1, 6), (2, 7), (3, 7)];
        let v = validity(&nodes, &certs);
        assert_eq!(v[0], Validity::Ultimate);
        assert!(v[1].is_valid() && v[2].is_valid());
        assert_eq!(v[5], Validity::Valid { depth: 2, full: 0, marginal: 3 });
        assert_eq!(v[6], Validity::Valid { depth: 2, full: 1, marginal: 0 });
        assert_eq!(v[7], Validity::Partial { full: 0, marginal: 2 });
    }
}
