//! Things that end up on your keyring. Details are computed from the save.

use cpl_crypto::bytes::to_hex;
use cpl_crypto::bip39::BIP86_FIRST;
use cpl_crypto::secp::x_only;

use crate::state::GameState;
use crate::world::hex32;

#[derive(Clone, Copy, Debug)]
pub struct ItemDef {
    pub id: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
}

pub const ITEMS: &[ItemDef] = &[
    ItemDef { id: "letter", name: "Grandpa's letter", desc: "Twelve words in blue ink. One of them drowned in a coffee ring." },
    ItemDef { id: "descriptor", name: "Sticky note", desc: "A wallet descriptor in Grandpa's handwriting. The fingerprint is underlined twice." },
    ItemDef { id: "gateway-key", name: "Gateway identity", desc: "Grandpa's Taproot key, derived from the recovered seed at m/86'/0'/0'/0/0." },
    ItemDef { id: "share1", name: "Share #1 (Mara)", desc: "A Shamir share of the Vault key, written on a waterproof card." },
    ItemDef { id: "share2", name: "Share #2 (Felix)", desc: "A Shamir share of the Vault key, read out over a secured line." },
    ItemDef { id: "share3", name: "Share #3 (dead drop)", desc: "Found behind a loose brick in the Commons. The chalk mark looked fresh." },
    ItemDef { id: "share4", name: "Share #4 (dead drop)", desc: "Found in the hollow of the old oak in the Commons." },
    ItemDef { id: "vault-key", name: "Vault key", desc: "Reconstructed from three good shares. Opens the Legacy vault." },
    ItemDef { id: "identity", name: "Your identity key", desc: "A key Grandpa generated for you long ago and kept in the vault." },
    ItemDef { id: "warden-key", name: "Warden's signing key", desc: "Recovered from two signatures that shared a nonce." },
    ItemDef { id: "archive", name: "The Archive", desc: "Decades of cypherpunk writing and code, released from the Mint." },
    ItemDef { id: "checkpoint-key", name: "Checkpoint master key", desc: "Extracted by rewinding a careless prover." },
    ItemDef { id: "pq-keys", name: "ML-KEM keypair", desc: "A post-quantum key for the Legacy capsule." },
];

pub fn item_def(id: &str) -> Option<&'static ItemDef> {
    ITEMS.iter().find(|i| i.id == id)
}

/// Live details for a held item, as (label, value) rows.
pub fn item_details(id: &str, s: &GameState) -> Vec<(String, String)> {
    let w = s.world();
    match id {
        "letter" => {
            let g = &w.grandpa().mnemonic;
            let words: Vec<String> = g
                .words
                .iter()
                .enumerate()
                .map(|(i, word)| if i == g.missing_index && !s.flag("pro.seed") { format!("{}▒▒▒", &word[..1]) } else { word.to_string() })
                .collect();
            vec![("words".into(), words.join(" "))]
        }
        "descriptor" => vec![("descriptor".into(), w.grandpa().descriptor.clone())],
        "gateway-key" => vec![("path".into(), BIP86_FIRST.into()), ("x-only key".into(), to_hex(&x_only(&w.grandpa().p)))],
        "share1" | "share2" | "share3" | "share4" => {
            let i: usize = id[5..].parse().unwrap();
            let v = w.vault();
            let sh = if i == 3 { &v.forged3 } else { &v.shares[i - 1] };
            vec![("x".into(), sh.x.to_string()), ("y".into(), hex32(&sh.y))]
        }
        "vault-key" => vec![("public key".into(), to_hex(&x_only(&w.vault().v)))],
        "identity" => vec![("x-only key".into(), to_hex(&w.identity().xonly))],
        "warden-key" => vec![("public key".into(), to_hex(&w.warden().p))],
        _ => Vec::new(),
    }
}
