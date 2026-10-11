//! Everything cryptographic about *this* playthrough, derived from the save's
//! seed: Grandpa's mnemonic, the Vault key and its shares, Warden's leaky
//! signatures, Grandpa's Taproot address... Values are computed on first use
//! and cached for the life of the save.

use std::cell::OnceCell;

use cpl_crypto::aead::{GcmBox, gcm_encrypt_text, kdf};
use cpl_crypto::bip39::{BIP86_FIRST, DerivedKey, GrandpaSeed, derive_key, descriptor, generate_grandpa_seed, seed_from_mnemonic};
use cpl_crypto::bip340::{Sig, even_key, sign, sign_with_nonce};
use cpl_crypto::bytes::{Big, big, bytes_to_big, from_hex};
use cpl_crypto::drbg::Drbg;
use cpl_crypto::secp::{N, Pt, mul, scalar_to_hex, x_only};
use cpl_crypto::shamir::{Share, deal};
use cpl_crypto::taproot::{LEAF_VERSION_TAPSCRIPT, Leaf, TweakResult, leaf_single_sig, leaf_timelocked, leaf_two_of_two, tap_branch_hash, tap_leaf_hash, tweak_key};

use crate::state::SaveData;

pub struct GrandpaKeys {
    pub mnemonic: GrandpaSeed,
    pub seed: [u8; 64],
    pub key: DerivedKey,
    pub secret: Big,
    pub p: Pt,
    pub descriptor: String,
}

pub struct VaultWorld {
    pub d: Big,
    pub v: Pt,
    /// genuine shares 1..5
    pub shares: Vec<Share>,
    /// what the DLA left behind the brick
    pub forged3: Share,
    pub commitments: Vec<Pt>,
    pub letter: GcmBox,
    pub letter_key: [u8; 32],
}

pub struct Identity {
    pub x: Big,
    pub p: Pt,
    pub xonly: [u8; 32],
}

pub struct WardenWorld {
    pub d: Big,
    pub p: [u8; 32],
    pub m1: &'static str,
    pub m2: &'static str,
    pub sig1: Sig,
    pub sig2: Sig,
}

pub struct Custody {
    pub d: Big,
    pub p: Pt,
}

pub struct TaprootWorld {
    pub leaves: Vec<Leaf>,
    pub correct: (&'static str, &'static str),
    pub target: TweakResult,
}

pub struct CheckpointKey {
    pub x: Big,
    pub p: Pt,
    pub r: Big,
}

pub const CH1_P1: &str = "NIGHTJAR TO MARA: THE VAULT KEY IS SPLIT FIVE WAYS.";
pub const CH1_P2: &str = "MARA TO NIGHTJAR: KEEP YOUR SHARE AWAY FROM RELAYS.";

pub const GRANDPA_LETTER: &str = "{{handle}},

If the Vault opened for you, then Mara, Felix and Juno trusted you, and you learned to trust the math instead of the messenger. Good. That was the whole lesson, really.

Here is what I could not write on paper. In the spring, the Directorate seized the Mint and sealed our Archive behind one of Warden's keys: thirty years of letters, code, arguments, mistakes. The things we wrote so that the next people would not have to start from zero. They call it \"lawful access\". It is a locked door with one key, and the key is theirs.

Ibrahim will show you why Warden's key is not as safe as Warden thinks. Lin will teach you to walk through their checkpoints without handing over your name. And Ada knows what is coming after all of this, when today's keys stop being enough.

I made a key for you the day you were born. It is in this vault. It was always yours.

Nonce knows the way. Feed her once. Only once. She will tell you otherwise.

— E.";

pub const LOCKTIME_2030: i64 = 1_893_456_000; // 2030-01-01T00:00:00Z

pub const KNAPSACK_MESSAGE: &str = "HARVEST GOES LIVE AT DAWN. ALL ARCHIVE TRAFFIC IS RECORDED.";

pub const WARDEN_M1: &str = "DLA ORDER 0417: SEAL THE ARCHIVE AT THE MINT. AUTH: WARDEN";
pub const WARDEN_M2: &str = "DLA ORDER 0418: DOUBLE THE CHECKPOINTS AT THE MASQUERADE. AUTH: WARDEN";

pub struct World {
    seed: Vec<u8>,
    handle: String,
    grandpa: OnceCell<GrandpaKeys>,
    gate: OnceCell<([u8; 32], [u8; 16])>,
    vault: OnceCell<VaultWorld>,
    identity: OnceCell<Identity>,
    warden: OnceCell<WardenWorld>,
    custody: OnceCell<Custody>,
    taproot: OnceCell<TaprootWorld>,
    checkpoint: OnceCell<CheckpointKey>,
}

/// A scalar as 64 hex digits.
pub fn hex32(x: &Big) -> String {
    scalar_to_hex(x)
}

impl World {
    pub fn new(data: &SaveData) -> Self {
        Self {
            seed: from_hex(&data.seed).unwrap_or_else(|_| vec![0; 32]),
            handle: if data.handle.is_empty() { "anon".into() } else { data.handle.clone() },
            grandpa: OnceCell::new(),
            gate: OnceCell::new(),
            vault: OnceCell::new(),
            identity: OnceCell::new(),
            warden: OnceCell::new(),
            custody: OnceCell::new(),
            taproot: OnceCell::new(),
            checkpoint: OnceCell::new(),
        }
    }

    pub fn rng(&self, label: &str) -> Drbg {
        Drbg::new(&self.seed, label)
    }

    pub fn handle(&self) -> &str {
        &self.handle
    }

    /// The playthrough's seed, as stored in the save.
    pub fn seed_hex(&self) -> String {
        cpl_crypto::bytes::to_hex(&self.seed)
    }

    pub fn grandpa(&self) -> &GrandpaKeys {
        self.grandpa.get_or_init(|| {
            let mnemonic = generate_grandpa_seed(&mut self.rng("grandpa/mnemonic"));
            let seed = seed_from_mnemonic(&mnemonic.phrase, "");
            let key = derive_key(&seed, BIP86_FIRST);
            let secret = bytes_to_big(&key.private_key);
            let p = mul(&secret);
            let descriptor = descriptor(&seed);
            GrandpaKeys { mnemonic, seed, key, secret, p, descriptor }
        })
    }

    /// The boathouse ticket gate's AES key and counter block (chapter 1).
    pub fn gate_key(&self) -> &([u8; 32], [u8; 16]) {
        self.gate.get_or_init(|| {
            let mut r = self.rng("ch1/gate");
            (r.array(), r.array())
        })
    }

    pub fn vault(&self) -> &VaultWorld {
        self.vault.get_or_init(|| {
            let mut r = self.rng("vault");
            let d = r.scalar(&N);
            let dealing = deal(&d, 3, 5, &mut r, &N);
            let forged3 = Share { x: big(3), y: r.scalar(&N) };
            let letter_key = kdf(&from_hex(&hex32(&d)).unwrap(), "legacy-letter");
            let nonce: [u8; 12] = r.array();
            let letter = gcm_encrypt_text(&letter_key, &nonce, &GRANDPA_LETTER.replace("{{handle}}", &self.handle));
            VaultWorld { v: dealing.commitments[0], d, shares: dealing.shares, forged3, commitments: dealing.commitments, letter, letter_key }
        })
    }

    /// The player's own key, generated by Grandpa long ago (and kept in the Vault).
    pub fn identity(&self) -> &Identity {
        self.identity.get_or_init(|| {
            let x = even_key(&self.rng("identity").scalar(&N));
            let p = mul(&x);
            Identity { xonly: x_only(&p), x, p }
        })
    }

    pub fn warden(&self) -> &WardenWorld {
        self.warden.get_or_init(|| {
            let mut r = self.rng("warden");
            let d = even_key(&r.scalar(&N));
            let k = r.scalar(&N);
            let sig1 = sign_with_nonce(WARDEN_M1.as_bytes(), &d, &k);
            let sig2 = sign_with_nonce(WARDEN_M2.as_bytes(), &d, &k);
            WardenWorld { p: x_only(&mul(&d)), d, m1: WARDEN_M1, m2: WARDEN_M2, sig1, sig2 }
        })
    }

    pub fn custody(&self) -> &Custody {
        self.custody.get_or_init(|| {
            let d = self.rng("custody").scalar(&N);
            Custody { p: mul(&d), d }
        })
    }

    pub fn taproot(&self) -> &TaprootWorld {
        self.taproot.get_or_init(|| {
            let mut r = self.rng("taproot");
            let mut k = || x_only(&mul(&r.scalar(&N)));
            let (mara, juno, felix) = (k(), k(), k());
            let leaves = vec![
                leaf_two_of_two("recovery", "Recovery: Mara and Juno together", &mara, "mara", &juno, "juno"),
                leaf_timelocked("heir", "Inheritance: after 1 Jan 2030, the heir alone", LOCKTIME_2030, &self.identity().xonly, "heir"),
                leaf_single_sig("escrow", "Escrow: the Directorate alone", &self.warden().p, "warden"),
                leaf_timelocked("felix", "Fallback: after 1 Jan 2030, Felix alone", LOCKTIME_2030, &felix, "felix"),
            ];
            let root = tap_branch_hash(&tap_leaf_hash(&leaves[0].script, LEAF_VERSION_TAPSCRIPT), &tap_leaf_hash(&leaves[1].script, LEAF_VERSION_TAPSCRIPT));
            let target = tweak_key(&x_only(&self.grandpa().p), Some(&root), "bc").expect("valid key");
            TaprootWorld { leaves, correct: ("recovery", "heir"), target }
        })
    }

    pub fn checkpoint_key(&self) -> &CheckpointKey {
        self.checkpoint.get_or_init(|| {
            let mut rng = self.rng("checkpoint");
            let x = rng.scalar(&N);
            CheckpointKey { p: mul(&x), x, r: rng.scalar(&N) }
        })
    }
}

/// A plain BIP-340 signature by `secret` over a UTF-8 message.
pub fn sign_as(secret: &Big, msg: &str) -> Sig {
    sign(msg.as_bytes(), secret, None)
}
