//! The save file and everything that changes as you play: flags, keyring,
//! codex, objective, chapter, settings. Persistence (file or localStorage) is
//! the engine's job; this module only (de)serialises.

use std::collections::BTreeMap;
use std::rc::Rc;

use cpl_crypto::bytes::{from_hex, to_hex};
use cpl_crypto::drbg::{Drbg, random_bytes};
use serde::{Deserialize, Serialize};

use crate::Facing;
use crate::world::World;

pub const SAVE_VERSION: u32 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum MapId {
    Cabin,
    City,
}

impl MapId {
    pub fn name(self) -> &'static str {
        match self {
            MapId::Cabin => "cabin",
            MapId::City => "city",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FlagValue {
    Bool(bool),
    Num(f64),
    Str(String),
}

impl FlagValue {
    pub fn truthy(&self) -> bool {
        match self {
            FlagValue::Bool(b) => *b,
            FlagValue::Num(n) => *n != 0.0 && !n.is_nan(),
            FlagValue::Str(s) => !s.is_empty(),
        }
    }
}

impl From<bool> for FlagValue {
    fn from(b: bool) -> Self {
        FlagValue::Bool(b)
    }
}

impl From<f64> for FlagValue {
    fn from(n: f64) -> Self {
        FlagValue::Num(n)
    }
}

impl From<&str> for FlagValue {
    fn from(s: &str) -> Self {
        FlagValue::Str(s.to_string())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pos {
    pub x: f32,
    pub y: f32,
    #[serde(with = "facing_serde")]
    pub facing: Facing,
}

mod facing_serde {
    use super::Facing;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(f: &Facing, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(f.name())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Facing, D::Error> {
        let s = String::deserialize(d)?;
        Ok(Facing::parse(&s).unwrap_or(Facing::Down))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Solved {
    pub at: f64,
    pub attempts: u32,
}

/// An eCash token in the wallet (see `cpl_crypto::ecash::Token`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenData {
    pub amount: u64,
    pub secret: String,
    pub c: String,
}

/// Numbers the credits show at the end.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Stats {
    /// times the Dragnet flagged you
    pub flagged: u32,
    /// credits earned and spent
    pub earned: u64,
    pub spent: u64,
    /// payments the mint accepted without knowing who paid
    pub payments: u32,
    pub stamps_minted: u32,
    pub deep_dives: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveData {
    pub version: u32,
    /// 32-byte hex seed for all puzzle content in this playthrough.
    pub seed: String,
    pub handle: String,
    pub look: usize,
    pub scene: MapId,
    pub pos: Option<Pos>,
    #[serde(default)]
    pub flags: BTreeMap<String, FlagValue>,
    #[serde(default)]
    pub items: Vec<String>,
    #[serde(default)]
    pub codex: Vec<String>,
    #[serde(default)]
    pub objective: String,
    #[serde(default)]
    pub chapter: u32,
    #[serde(default)]
    pub play_ms: f64,
    #[serde(default)]
    pub created_at: f64,
    #[serde(default)]
    pub updated_at: f64,
    #[serde(default)]
    pub solved: BTreeMap<String, Solved>,
    /// eCash tokens (blind-signed by the Commons Mint).
    #[serde(default)]
    pub wallet: Vec<TokenData>,
    /// Plain credit notes with your name on them, not yet swapped for eCash.
    #[serde(default)]
    pub ious: u64,
    /// The Commons Mint's spent list (hex of each spent token's Y).
    #[serde(default)]
    pub mint_spent: Vec<String>,
    /// Counter that makes every withdrawal's secrets fresh.
    #[serde(default)]
    pub wallet_nonce: u64,
    /// Hashcash stamps in hand.
    #[serde(default)]
    pub stamps: Vec<String>,
    /// Allies, in the order they joined.
    #[serde(default)]
    pub allies: Vec<String>,
    /// Owner trust you assigned (0 unknown, 1 never, 2 marginal, 3 full), by key id.
    #[serde(default)]
    pub trust: BTreeMap<String, u8>,
    /// Your certifications: key id → BIP-340 signature (hex) over its fingerprint.
    #[serde(default)]
    pub certs: BTreeMap<String, String>,
    /// Deep dives completed (puzzle ids).
    #[serde(default)]
    pub badges: Vec<String>,
    /// Punched cards in your pocket (the Wharf's signal boards).
    #[serde(default)]
    pub cards: Vec<crate::signals::Card>,
    /// Stencils in the glyph book, by glyph id.
    #[serde(default)]
    pub glyphs: Vec<String>,
    #[serde(default)]
    pub stats: Stats,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub music: f32,
    pub sfx: f32,
    /// characters per second, 0 = instant
    pub text_speed: f32,
    pub crt: bool,
    pub reduced_motion: bool,
    /// UI text scale
    pub ui_scale: f32,
    /// Guided (steps with hints) or Minimal (steps only).
    pub hints: bool,
    /// The Dragnet raises your trace and can flag you (off: sensors only show).
    pub dragnet: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { music: 0.5, sfx: 0.7, text_speed: 60.0, crt: true, reduced_motion: false, ui_scale: 1.0, hints: true, dragnet: true }
    }
}

/// Things the UI reacts to (toasts, HUD flashes, sounds).
#[derive(Clone, Debug, PartialEq)]
pub enum StateEvent {
    Flag(String),
    Item { id: String, added: bool },
    Codex(String),
    Objective(String),
    Chapter(u32),
    /// credits arrived (positive) or left (negative), and why
    Credits(i64, String),
    Ally(String),
    Saved,
}

/// Wall-clock time in whole milliseconds (whole, so saves round-trip exactly through JSON).
pub fn now_ms() -> f64 {
    web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map(|d| d.as_millis() as f64).unwrap_or(0.0)
}

pub struct GameState {
    pub data: SaveData,
    pub settings: Settings,
    pub events: Vec<StateEvent>,
    world: Rc<World>,
    session_start: f64,
    /// Set when the save should be written by the engine.
    pub dirty_save: bool,
    pub dirty_settings: bool,
    /// The Dragnet's trace on you, 0 to 100 (set by the engine, not saved).
    pub trace: f32,
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

impl GameState {
    pub fn new() -> Self {
        let data = Self::blank("anon", 0, None);
        let world = Rc::new(World::new(&data));
        Self { data, settings: Settings::default(), events: Vec::new(), world, session_start: now_ms(), dirty_save: false, dirty_settings: false, trace: 0.0 }
    }

    pub fn blank(handle: &str, look: usize, seed: Option<String>) -> SaveData {
        let now = now_ms();
        SaveData {
            version: SAVE_VERSION,
            seed: seed.unwrap_or_else(|| to_hex(&random_bytes::<32>())),
            handle: handle.to_string(),
            look,
            scene: MapId::Cabin,
            pos: None,
            flags: BTreeMap::new(),
            items: Vec::new(),
            codex: Vec::new(),
            objective: String::new(),
            chapter: 0,
            play_ms: 0.0,
            created_at: now,
            updated_at: now,
            solved: BTreeMap::new(),
            wallet: Vec::new(),
            ious: 0,
            mint_spent: Vec::new(),
            wallet_nonce: 0,
            stamps: Vec::new(),
            allies: Vec::new(),
            trust: BTreeMap::new(),
            certs: BTreeMap::new(),
            badges: Vec::new(),
            cards: Vec::new(),
            glyphs: Vec::new(),
            stats: Stats::default(),
        }
    }

    /// Start a new game. `seed_phrase` forces deterministic content (tests, `?seed=`).
    pub fn new_game(&mut self, handle: &str, look: usize, seed_phrase: Option<&str>) {
        let seed = seed_phrase.map(|p| to_hex(&Drbg::new(p.as_bytes(), "seed").bytes(32)));
        self.data = Self::blank(handle, look, seed);
        self.world = Rc::new(World::new(&self.data));
        self.session_start = now_ms();
        self.events.clear();
        self.trace = 0.0;
        self.save();
    }

    /// Replace the current game with loaded data.
    pub fn load(&mut self, mut d: SaveData) -> bool {
        if from_hex(&d.seed).map(|b| b.len() != 32).unwrap_or(true) {
            return false;
        }
        d.version = SAVE_VERSION;
        self.data = d;
        self.world = Rc::new(World::new(&self.data));
        self.session_start = now_ms();
        self.events.clear();
        true
    }

    pub fn parse_save(json: &str) -> Option<SaveData> {
        let d: SaveData = serde_json::from_str(json).ok()?;
        (d.version >= 2 && from_hex(&d.seed).map(|b| b.len() == 32).unwrap_or(false)).then_some(d)
    }

    pub fn save_json(&self) -> String {
        serde_json::to_string_pretty(&self.data).unwrap()
    }

    /// Mark the save for writing (the engine persists it at the end of the frame).
    pub fn save(&mut self) {
        let now = now_ms();
        self.data.play_ms += (now - self.session_start).max(0.0);
        self.session_start = now;
        self.data.updated_at = now;
        self.dirty_save = true;
        self.events.push(StateEvent::Saved);
    }

    pub fn world(&self) -> Rc<World> {
        self.world.clone()
    }

    // --- flags, items, codex -------------------------------------------------

    pub fn flag(&self, key: &str) -> bool {
        self.data.flags.get(key).is_some_and(FlagValue::truthy)
    }

    pub fn get(&self, key: &str) -> Option<&FlagValue> {
        self.data.flags.get(key)
    }

    pub fn set(&mut self, key: &str, value: impl Into<FlagValue>) {
        let v = value.into();
        if self.data.flags.get(key) == Some(&v) {
            return;
        }
        self.data.flags.insert(key.to_string(), v);
        self.events.push(StateEvent::Flag(key.to_string()));
    }

    /// Evaluate a condition such as `"a"`, `"!a"` or `"a,!b"` (all must hold).
    pub fn cond(&self, cond: Option<&str>) -> bool {
        let Some(cond) = cond else { return true };
        cond.split(',').all(|part| {
            let p = part.trim();
            match p.strip_prefix('!') {
                Some(k) => !self.flag(k),
                None => self.flag(p),
            }
        })
    }

    pub fn has(&self, item: &str) -> bool {
        self.data.items.iter().any(|i| i == item)
    }

    pub fn give(&mut self, item: &str) -> bool {
        if self.has(item) {
            return false;
        }
        self.data.items.push(item.to_string());
        self.events.push(StateEvent::Item { id: item.to_string(), added: true });
        true
    }

    pub fn take(&mut self, item: &str) {
        if let Some(i) = self.data.items.iter().position(|x| x == item) {
            self.data.items.remove(i);
            self.events.push(StateEvent::Item { id: item.to_string(), added: false });
        }
    }

    pub fn unlock_codex(&mut self, id: &str) -> bool {
        if self.data.codex.iter().any(|c| c == id) {
            return false;
        }
        self.data.codex.push(id.to_string());
        self.events.push(StateEvent::Codex(id.to_string()));
        true
    }

    pub fn set_objective(&mut self, text: &str) {
        self.data.objective = text.to_string();
        self.events.push(StateEvent::Objective(text.to_string()));
    }

    pub fn set_chapter(&mut self, n: u32) {
        if n <= self.data.chapter {
            return;
        }
        self.data.chapter = n;
        self.events.push(StateEvent::Chapter(n));
    }

    pub fn mark_solved(&mut self, id: &str, attempts: u32) {
        self.data.solved.entry(id.to_string()).or_insert(Solved { at: now_ms(), attempts });
    }

    /// Deterministic stream for puzzle content, unique to this save.
    pub fn rng(&self, label: &str) -> Drbg {
        Drbg::new(&from_hex(&self.data.seed).expect("valid seed"), label)
    }

    pub fn handle(&self) -> &str {
        if self.data.handle.is_empty() { "anon" } else { &self.data.handle }
    }

    /// Replace `{{handle}}` with the player's chosen name.
    pub fn fill(&self, text: &str) -> String {
        text.replace("{{handle}}", self.handle())
    }

    pub fn update_settings(&mut self, s: Settings) {
        self.settings = s;
        self.dirty_settings = true;
    }

    /// Chapter progress for the HUD pips: (done, current) for P, 1..6.
    pub fn chapter_done(&self, i: usize) -> bool {
        if i == 0 { self.flag("pro.done") } else { self.flag(&format!("ch{i}.done")) }
    }
}

pub const CHAPTER_LABELS: [&str; 7] = ["P", "1", "2", "3", "4", "5", "6"];
pub const CHAPTER_NAMES: [&str; 7] = [
    "Prologue: The Cabin",
    "Chapter 1: One Time Only",
    "Chapter 2: Strangers Agree",
    "Chapter 3: Threshold",
    "Chapter 4: Signatures",
    "Chapter 5: Knowledge",
    "Chapter 6: After the Quantum",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_roundtrip_and_conditions() {
        let mut s = GameState::new();
        s.new_game("tester", 1, Some("fixed"));
        s.set("a", true);
        s.set("n", 3.0);
        s.give("letter");
        s.unlock_codex("bip39");
        assert!(s.cond(Some("a,!b")));
        assert!(!s.cond(Some("a,b")));
        let json = s.save_json();
        let d = GameState::parse_save(&json).unwrap();
        let mut t = GameState::new();
        assert!(t.load(d));
        assert!(t.flag("a") && t.flag("n") && t.has("letter"));
        assert_eq!(t.data.seed, s.data.seed);
        let mut u = GameState::new();
        u.new_game("x", 0, Some("fixed"));
        assert_eq!(u.data.seed, s.data.seed, "seed phrases are deterministic");
    }
}
