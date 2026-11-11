//! Hash chains: an append-only ledger where rewriting history shows.
//!
//! Each entry's hash covers the previous entry's hash, so changing any line
//! changes every hash after it. A forger who also recomputes all the later
//! hashes produces a chain that checks out on its own, which is why Haber and
//! Stornetta (1991) published their chain's latest hash where nobody could
//! quietly change it: from 1995 their company, Surety, printed it every week
//! in the classified ads of *The New York Times*. Bitcoin's blocks are chained
//! the same way.

use sha2::{Digest, Sha256};

use crate::bytes::to_hex;

pub type Hash = [u8; 32];

pub const GENESIS: Hash = [0u8; 32];

/// h_i = SHA-256(h_{i-1} ‖ i ‖ text_i)
pub fn link(prev: &Hash, index: u32, text: &str) -> Hash {
    Sha256::new().chain_update(prev).chain_update(index.to_be_bytes()).chain_update(text.as_bytes()).finalize().into()
}

/// Hashes for every entry.
pub fn chain(texts: &[String]) -> Vec<Hash> {
    let mut prev = GENESIS;
    texts
        .iter()
        .enumerate()
        .map(|(i, t)| {
            prev = link(&prev, i as u32, t);
            prev
        })
        .collect()
}

/// The first entry whose recorded hash does not match its text and predecessor.
pub fn first_break(texts: &[String], recorded: &[Hash]) -> Option<usize> {
    let mut prev = GENESIS;
    for (i, t) in texts.iter().enumerate() {
        let h = link(&prev, i as u32, t);
        if recorded.get(i) != Some(&h) {
            return Some(i);
        }
        prev = recorded[i];
    }
    None
}

pub fn short(h: &Hash) -> String {
    to_hex(&h[..4])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_show_and_rewrites_need_anchors() {
        let mut texts: Vec<String> = (0..8).map(|i| format!("day {i}: 12 crates of oranges")).collect();
        let honest = chain(&texts);
        assert_eq!(first_break(&texts, &honest), None);
        // a clumsy edit breaks the chain at the edited line
        texts[3] = "day 3: 12 crates of oranges, 2 rifles".into();
        assert_eq!(first_break(&texts, &honest), Some(3));
        // a careful forger recomputes everything after it: the chain checks out...
        let forged = chain(&texts);
        assert_eq!(first_break(&texts, &forged), None);
        // ...but its head no longer matches the hash published before the edit
        assert_ne!(forged.last(), honest.last());
        assert_eq!(forged[2], honest[2]);
        assert_ne!(forged[4], honest[4]);
    }
}
