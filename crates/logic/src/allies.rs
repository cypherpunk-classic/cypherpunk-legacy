//! Allies and the web of trust.
//!
//! Recruiting someone means checking their fingerprint and signing their key
//! with your identity key: a real BIP-340 certification, kept in the save.
//! Allies have signed some keys themselves. Which of those keys you accept
//! depends on how much you trust each signer, and the validity is computed
//! with GnuPG's rules (`cpl_crypto::wot`).

use cpl_crypto::bip340::Sig;
use cpl_crypto::bytes::{from_hex, to_hex};
use cpl_crypto::wot::{Node, OwnerTrust, Validity, certify, validity, verify_cert};

use crate::state::{GameState, StateEvent};

/// Someone who can join you.
#[derive(Clone, Copy, Debug)]
pub struct AllyDef {
    pub id: &'static str,
    pub name: &'static str,
    /// the user id on their key
    pub uid: &'static str,
    /// one line for the Network tab
    pub role: &'static str,
    /// keys this person has already signed
    pub vouches: &'static [&'static str],
}

pub const ALLIES: &[AllyDef] = &[
    AllyDef { id: "mara", name: "Mara", uid: "Mara <mara@wharf.radio>", role: "radio operator, the Wharf", vouches: &["juno", "rosa", "kestrel"] },
    AllyDef { id: "felix", name: "Felix", uid: "Felix <felix@exchange>", role: "switchboard, the Exchange", vouches: &["mara", "theo", "kestrel"] },
    AllyDef { id: "juno", name: "Juno", uid: "Juno <juno@commons.park>", role: "gardener, the Commons", vouches: &["felix", "wen", "kestrel"] },
    AllyDef { id: "ibrahim", name: "Ibrahim", uid: "Ibrahim <audit@mint>", role: "auditor, the Mint", vouches: &["felix", "lin"] },
    AllyDef { id: "lin", name: "Lin", uid: "Lin <lin@masquerade>", role: "remailer operator, the Masquerade", vouches: &["ada"] },
    AllyDef { id: "ada", name: "Ada", uid: "Ada <ada@observatory>", role: "astronomer, the Observatory", vouches: &["ibrahim"] },
    AllyDef { id: "rosa", name: "Rosa", uid: "Rosa <rosa@harbourlight.news>", role: "reporter, the Harbour Light", vouches: &["mara"] },
    AllyDef { id: "theo", name: "Theo", uid: "Theo <theo@exchange.records>", role: "records clerk, the Exchange", vouches: &["felix"] },
    AllyDef { id: "wen", name: "Wen", uid: "Wen <wen@commons.grocer>", role: "grocer, the Commons", vouches: &["juno"] },
    AllyDef { id: "c1", name: "Dock worker", uid: "Bao <bao@wharf>", role: "dock worker, the Wharf", vouches: &[] },
    AllyDef { id: "c2", name: "Fisher", uid: "Ines <ines@wharf>", role: "fisher, the Wharf", vouches: &[] },
    AllyDef { id: "c3", name: "Student", uid: "Kai <kai@plaza>", role: "student, Legacy Plaza", vouches: &[] },
    AllyDef { id: "c4", name: "Gardener", uid: "Odile <odile@commons.park>", role: "gardener, the Commons", vouches: &[] },
    AllyDef { id: "c5", name: "Courier", uid: "Ravi <ravi@avenue>", role: "courier, the avenues", vouches: &[] },
];

/// Keys you meet but never shake hands with.
#[derive(Clone, Copy, Debug)]
pub struct StrangerDef {
    pub id: &'static str,
    pub uid: &'static str,
    /// shown once this flag is set
    pub met: &'static str,
    /// who signed it (ids of allies, or of people you will never trust)
    pub signed_by: &'static [&'static str],
}

pub const STRANGERS: &[StrangerDef] = &[
    StrangerDef { id: "kestrel", uid: "Kestrel <kestrel@nym>", met: "ch5.kestrel", signed_by: &[] },
    StrangerDef { id: "kestrel-dla", uid: "Kestrel <kestrel@nym>", met: "ch5.kestrel", signed_by: &["notary"] },
    StrangerDef { id: "notary", uid: "DLA Key Notary <notary@dla.gov>", met: "ch5.kestrel", signed_by: &[] },
];

pub fn ally_def(id: &str) -> Option<&'static AllyDef> {
    ALLIES.iter().find(|a| a.id == id)
}

pub fn trust_from(n: u8) -> OwnerTrust {
    match n {
        1 => OwnerTrust::Never,
        2 => OwnerTrust::Marginal,
        3 => OwnerTrust::Full,
        _ => OwnerTrust::Unknown,
    }
}

pub fn trust_to(t: OwnerTrust) -> u8 {
    match t {
        OwnerTrust::Never => 1,
        OwnerTrust::Marginal => 2,
        OwnerTrust::Full | OwnerTrust::Ultimate => 3,
        OwnerTrust::Unknown => 0,
    }
}

/// The web as you see it: nodes (you first), certifications, validity.
pub struct WebView {
    pub nodes: Vec<Node>,
    pub ids: Vec<String>,
    pub certs: Vec<(usize, usize)>,
    pub validity: Vec<Validity>,
}

impl WebView {
    pub fn index(&self, id: &str) -> Option<usize> {
        self.ids.iter().position(|i| i == id)
    }

    pub fn valid(&self, id: &str) -> bool {
        self.index(id).is_some_and(|i| self.validity[i].is_valid())
    }
}

impl GameState {
    pub fn is_ally(&self, id: &str) -> bool {
        self.data.allies.iter().any(|a| a == id)
    }

    /// Sign someone's key and welcome them. Returns false if already an ally.
    pub fn recruit(&mut self, id: &str) -> bool {
        if self.is_ally(id) || ally_def(id).is_none() {
            return false;
        }
        let w = self.world();
        let me = w.key_of("you");
        let them = w.key_of(id);
        let uid = ally_def(id).map(|a| a.uid).unwrap_or(id);
        let sig = certify(&me.secret, &me.fp, &them.fp, uid);
        self.data.certs.insert(id.to_string(), sig.hex());
        self.data.allies.push(id.to_string());
        self.data.trust.entry(id.to_string()).or_insert(2);
        self.set(&format!("ally.{id}"), true);
        self.events.push(StateEvent::Ally(id.to_string()));
        true
    }

    /// Someone leaves the city (or the web): their certification goes with them.
    pub fn lose_ally(&mut self, id: &str) -> bool {
        if !self.is_ally(id) {
            return false;
        }
        self.data.allies.retain(|a| a != id);
        self.data.certs.remove(id);
        self.data.trust.remove(id);
        self.set(&format!("ally.{id}"), false);
        true
    }

    /// Check one of your certifications (it is a real signature).
    pub fn cert_ok(&self, id: &str) -> bool {
        let Some(hex) = self.data.certs.get(id) else { return false };
        let Some(sig) = from_hex(hex).ok().and_then(|b| Sig::parse(&b).ok()) else { return false };
        let w = self.world();
        let me = w.key_of("you");
        let them = w.key_of(id);
        let uid = ally_def(id).map(|a| a.uid).unwrap_or(id);
        verify_cert(&sig, &me.xonly, &me.fp, &them.fp, uid)
    }

    pub fn owner_trust(&self, id: &str) -> OwnerTrust {
        trust_from(self.data.trust.get(id).copied().unwrap_or(0))
    }

    pub fn set_trust(&mut self, id: &str, t: OwnerTrust) {
        self.data.trust.insert(id.to_string(), trust_to(t));
    }

    /// Everyone on your web: you, your allies, and the strangers you have met.
    pub fn web(&self) -> WebView {
        let w = self.world();
        let mut ids: Vec<String> = vec!["you".into()];
        ids.extend(self.data.allies.iter().cloned());
        for s in STRANGERS {
            if self.flag(s.met) {
                ids.push(s.id.into());
            }
        }
        // allies vouch only for people already on the web
        let nodes: Vec<Node> = ids
            .iter()
            .map(|id| {
                let k = w.key_of(id);
                let uid = ally_def(id).map(|a| a.uid.to_string()).or_else(|| STRANGERS.iter().find(|s| s.id == id).map(|s| s.uid.to_string())).unwrap_or_else(|| format!("{} <you>", self.handle()));
                let trust = if id == "you" { OwnerTrust::Ultimate } else { self.owner_trust(id) };
                Node { uid, fp: k.fp, trust }
            })
            .collect();
        let idx = |id: &str| ids.iter().position(|i| i == id);
        let mut certs = Vec::new();
        for (t, id) in ids.iter().enumerate().skip(1) {
            if self.data.certs.contains_key(id) {
                certs.push((0, t));
            }
        }
        for a in ALLIES {
            let Some(s) = idx(a.id) else { continue };
            for v in a.vouches {
                if let Some(t) = idx(v) {
                    certs.push((s, t));
                }
            }
        }
        for st in STRANGERS {
            let Some(t) = idx(st.id) else { continue };
            for signer in st.signed_by {
                if let Some(s) = idx(signer) {
                    certs.push((s, t));
                }
            }
        }
        let validity = validity(&nodes, &certs);
        WebView { nodes, ids, certs, validity }
    }

    pub fn ally_count(&self) -> usize {
        self.data.allies.len()
    }

    /// How fast the Dragnet's trace rises on you (1 = no protection).
    pub fn opsec_factor(&self) -> f32 {
        let mut f = 1.0;
        for (item, m) in [("sealed-messenger", 0.8), ("keyring", 0.8), ("faraday", 0.6), ("onion-route", 0.7), ("mesh-radio", 0.9)] {
            if self.has(item) {
                f *= m;
            }
        }
        if self.wallet_open() {
            f *= 0.85;
        }
        f * (1.0 - 0.02 * self.ally_count() as f32).max(0.75)
    }

    /// Fingerprint of a key, formatted (for dialogue and terminals).
    pub fn fingerprint_of(&self, id: &str) -> String {
        to_hex(&self.world().key_of(id).fp).to_uppercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recruiting_signs_and_trust_decides_strangers() {
        let mut s = GameState::new();
        s.new_game("t", 0, Some("allies"));
        for id in ["mara", "felix", "juno"] {
            assert!(s.recruit(id));
            assert!(s.cert_ok(id));
        }
        assert!(!s.recruit("mara"));
        s.set("ch5.kestrel", true);
        // default trust: marginal for the three friends who signed Kestrel
        let w = s.web();
        assert!(w.valid("mara") && w.valid("kestrel"));
        assert!(!w.valid("kestrel-dla"));
        // distrust one of them and Kestrel is only partly valid
        s.set_trust("juno", OwnerTrust::Never);
        assert!(!s.web().valid("kestrel"));
        // trust one fully and it is valid again
        s.set_trust("felix", OwnerTrust::Full);
        assert!(s.web().valid("kestrel"));
        assert!(s.opsec_factor() < 1.0);
    }
}
