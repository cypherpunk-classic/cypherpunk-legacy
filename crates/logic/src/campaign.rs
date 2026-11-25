//! The campaign: the city as a map of stages you pick from instead of
//! walking between them, stars for how cleanly you played a stage, and an
//! upgrade tree the stars buy.
//!
//! A district (the Wharf, the Exchange) is a stage. Playing it scores up to
//! three stars: one for freeing it, one for never being flagged on the way,
//! one for never reading past what the notebook gives you for free. Each
//! dive is a side stage worth one star. The best score for every stage is
//! kept in the save, and a finished district can be replayed from the map
//! with fresh pads, in a sandbox: the replay runs on a copy of the save, and
//! only the stars come back out of it.
//!
//! Upgrades make the street kinder, never the cryptography easier: no
//! upgrade reads a lamp, turns a knob or finds a key for you.

use cpl_crypto::bytes::{from_hex, to_hex};
use cpl_crypto::drbg::Drbg;
use serde::{Deserialize, Serialize};

use crate::state::{GameState, MapId, SaveData};

/// The best you have done on one stage.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StageRecord {
    /// best score, 0..=3
    pub stars: u8,
    /// times cleared (the story's run included)
    pub clears: u32,
    /// replays started (each gets its own pads)
    pub replays: u32,
}

/// What a run is scored on: three conditions, the first of them finishing it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Score {
    pub ok: [bool; 3],
    pub labels: [&'static str; 3],
}

pub const DISTRICT_LABELS: [&str; 3] = ["freed", "never flagged on the way", "no paid notebook pages"];
pub const BOSS_LABELS: [&str; 3] = ["the lamp goes dark", "never traced out", "never hit"];
pub const CITY_LABELS: [&str; 3] = ["every terminal solved", "steady hands: a wrong move per terminal at most", "step hints off (Minimal)"];

impl Score {
    pub fn stars(self) -> u8 {
        if !self.ok[0] {
            return 0;
        }
        1 + self.ok[1] as u8 + self.ok[2] as u8
    }
    pub fn cleared(self) -> bool {
        self.ok[0]
    }
}

/// A node on the campaign map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    /// a v4 district: scored, replayable
    District(MapId),
    /// a dive inside a district: one star
    Dive { district: MapId, spawn: &'static str, flag: &'static str },
    /// the cabin
    Cabin,
    /// a quarter of the v3 city, reached at its safe house
    City { spawn: &'static str },
    /// a boss fought inside a district: three stars
    Boss { district: MapId, spawn: &'static str, flag: &'static str },
}

#[derive(Clone, Copy, Debug)]
pub struct Node {
    pub id: &'static str,
    pub name: &'static str,
    /// the chapter label it belongs to ("P", "1".."6")
    pub chapter: &'static str,
    /// one line of history under the name
    pub era: &'static str,
    pub kind: NodeKind,
    /// position on the map, as a fraction of the city (0..1, 0..1)
    pub at: (f32, f32),
    /// the flag that puts it on the map
    pub opens: &'static str,
    /// the flag that says its part of the story is done
    pub done: &'static str,
    /// the node the road comes from
    pub from: Option<&'static str>,
}

/// Flags that make up a district's run in progress (kept in the sandbox and the story save alike).
pub fn run_flagged_key(d: MapId) -> String {
    format!("{}.run.flagged0", prefix(d))
}
pub fn run_hints_key(d: MapId) -> String {
    format!("{}.run.hints", prefix(d))
}
fn scored_key(d: MapId) -> String {
    format!("{}.run.scored", prefix(d))
}

fn prefix(d: MapId) -> &'static str {
    match d {
        MapId::Exchange => "x",
        _ => "w",
    }
}

/// The stage id of a district.
pub fn stage_id(d: MapId) -> &'static str {
    match d {
        MapId::Exchange => "exchange",
        _ => "wharf",
    }
}

/// The flag that frees a district, and the one that says the player has walked out of it.
pub fn goal(d: MapId) -> (&'static str, &'static str) {
    match d {
        MapId::Exchange => (crate::lines::exchange::FREE, crate::lines::exchange::END),
        _ => (crate::signals::wharf::MAST, crate::signals::wharf::END),
    }
}

/// The flag set when the story sends you into a district.
pub fn sent(d: MapId) -> &'static str {
    match d {
        MapId::Exchange => crate::story::ch2::STREET_SENT,
        _ => crate::story::ch1::HARBOUR_SENT,
    }
}

pub const NODES: &[Node] = &[
    Node { id: "cabin", name: "The Cabin", chapter: "P", era: "BIP-39, 2013: twelve words for a key", kind: NodeKind::Cabin, at: (0.05, 0.95), opens: "pro.done", done: "pro.done", from: None },
    Node { id: "radio", name: "The Radio Shack", chapter: "1", era: "Vernam 1917, VENONA 1943", kind: NodeKind::City { spawn: "safe-wharf" }, at: (0.24, 0.78), opens: "city.arrived", done: "ch1.ctr", from: Some("cabin") },
    Node {
        id: "wharf",
        name: "The Wharf",
        chapter: "1",
        era: "Shannon 1949: the perfect cipher, misused",
        kind: NodeKind::District(MapId::Wharf),
        at: (0.50, 0.88),
        opens: crate::story::ch1::HARBOUR_SENT,
        done: crate::signals::wharf::END,
        from: Some("radio"),
    },
    Node {
        id: "interceptor",
        name: "INTERCEPTOR",
        chapter: "1",
        era: "Shamir 1979: secret sharing",
        kind: NodeKind::Dive { district: MapId::Wharf, spawn: "cabinet", flag: crate::signals::wharf::DIVE_INTERCEPTOR },
        at: (0.36, 0.91),
        opens: crate::story::ch1::HARBOUR_SENT,
        done: crate::signals::wharf::DIVE_INTERCEPTOR,
        from: Some("wharf"),
    },
    Node {
        id: "static",
        name: "STATIC",
        chapter: "1",
        era: "crib dragging, VENONA",
        kind: NodeKind::Dive { district: MapId::Wharf, spawn: "radio", flag: crate::signals::wharf::DIVE_STATIC },
        at: (0.64, 0.91),
        opens: crate::story::ch1::HARBOUR_SENT,
        done: crate::signals::wharf::DIVE_STATIC,
        from: Some("wharf"),
    },
    Node {
        id: "lamplighter",
        name: "The Lamplighter",
        chapter: "1",
        era: "boss · LFSRs, Berlekamp–Massey 1969, seed reuse",
        kind: NodeKind::Boss { district: MapId::Wharf, spawn: "mole", flag: crate::dive::lamplighter::DIVE_LAMPLIGHTER },
        at: (0.80, 0.80),
        opens: crate::signals::wharf::MAST,
        done: crate::dive::lamplighter::DIVE_LAMPLIGHTER,
        from: Some("wharf"),
    },
    Node { id: "switchboard", name: "The Switchboard", chapter: "2", era: "Diffie-Hellman 1976, PGP 1991", kind: NodeKind::City { spawn: "safe-exchange" }, at: (0.20, 0.57), opens: "ch1.done", done: "ch2.book", from: Some("wharf") },
    Node {
        id: "exchange",
        name: "The Exchange",
        chapter: "2",
        era: "blue boxes 1971, Clipper 1993, Logjam 2015",
        kind: NodeKind::District(MapId::Exchange),
        at: (0.08, 0.40),
        opens: crate::story::ch2::STREET_SENT,
        done: crate::lines::exchange::END,
        from: Some("switchboard"),
    },
    Node {
        id: "logjam",
        name: "LOGJAM",
        chapter: "2",
        era: "baby-step giant-step, export-grade DH",
        kind: NodeKind::Dive { district: MapId::Exchange, spawn: "cellar", flag: crate::lines::exchange::DIVE_LOGJAM },
        at: (0.26, 0.40),
        opens: crate::story::ch2::STREET_SENT,
        done: crate::lines::exchange::DIVE_LOGJAM,
        from: Some("exchange"),
    },
    Node { id: "commons", name: "The Commons", chapter: "3", era: "Shamir 1979, Chaum 1982", kind: NodeKind::City { spawn: "safe-commons" }, at: (0.82, 0.50), opens: "ch2.done", done: "ch3.done", from: Some("exchange") },
    Node { id: "mint", name: "The Mint", chapter: "4", era: "Schnorr 1989, Blaze 1994", kind: NodeKind::City { spawn: "safe-mint" }, at: (0.82, 0.18), opens: "party.done", done: "ch4.done", from: Some("commons") },
    Node { id: "masks", name: "The Masquerade", chapter: "5", era: "remailers 1993, Hashcash 1997", kind: NodeKind::City { spawn: "safe-masks" }, at: (0.18, 0.18), opens: "ch4.done", done: "ch5.done", from: Some("mint") },
    Node { id: "obs", name: "The Observatory", chapter: "6", era: "Shor 1994, ML-KEM 2024", kind: NodeKind::City { spawn: "safe-obs" }, at: (0.50, 0.09), opens: "ch5.done", done: "ch6.done", from: Some("masks") },
];

pub fn node(id: &str) -> Option<&'static Node> {
    NODES.iter().find(|n| n.id == id)
}

/// Can the player go there now?
pub fn open(st: &GameState, n: &Node) -> bool {
    if n.id == "cabin" {
        return st.flag("pro.done");
    }
    st.flag(n.opens)
}

/// The most stars this node can give.
pub fn max_stars(n: &Node) -> u8 {
    match n.kind {
        NodeKind::Dive { .. } => 1,
        _ => 3,
    }
}

/// The terminals a quarter of the v3 city is scored on, in story order (and
/// replayed as a gauntlet). Puzzles that lean on your allies, stamps or
/// wallet stay in the story only.
pub fn terminals(id: &str) -> &'static [&'static str] {
    match id {
        "cabin" => &["pro.seed", "pro.login"],
        "radio" => &["ch1.otp", "ch1.ctr"],
        "switchboard" => &["ch2.dh", "ch2.mitm", "ch2.logjam", "ch2.book"],
        "commons" => &["ch3.shamir", "ch3.feldman", "ch3.cash"],
        "mint" => &["party.keys", "ch4.nonce", "ch4.rogue", "ch4.taproot", "ch4.clipper"],
        "masks" => &["ch5.sigma", "ch5.sim", "ch5.extract"],
        "obs" => &["ch6.gauss", "ch6.knapsack", "ch6.pq"],
        _ => &[],
    }
}

fn data_flag(d: &SaveData, k: &str) -> bool {
    d.flags.get(k).is_some_and(crate::state::FlagValue::truthy)
}

/// Score a quarter from the terminals you have solved in this save.
pub fn city_score(d: &SaveData, id: &str) -> Score {
    let ids = terminals(id);
    let done: Vec<&crate::state::Solved> = ids.iter().filter_map(|t| d.solved.get(*t)).collect();
    let all = !ids.is_empty() && done.len() == ids.len();
    let forgive = d.upgrades.get(TERMINAL_KIT).copied().unwrap_or(0) >= 1;
    let mistakes: u32 = done.iter().map(|s| s.attempts.saturating_sub(if forgive { 2 } else { 1 })).sum();
    let steady = all && mistakes as usize <= ids.len();
    let minimal = all && done.iter().all(|s| !s.guided);
    Score { ok: [all, steady, minimal], labels: CITY_LABELS }
}

fn stage_scored_key(id: &str) -> String {
    format!("stage.{id}.scored")
}

/// A gauntlet in progress: the node it replays (set in the sandbox).
pub const GAUNTLET: &str = "gauntlet.node";
/// Set by the gauntlet script when its last terminal is solved.
pub const GAUNTLET_DONE: &str = "gauntlet.done";

pub fn gauntlet_node(st: &GameState) -> Option<&'static Node> {
    match st.get(GAUNTLET) {
        Some(crate::state::FlagValue::Str(s)) => node(s),
        _ => None,
    }
}

pub fn stars(st: &GameState, id: &str) -> u8 {
    st.data.stages.get(id).map(|r| r.stars).unwrap_or(0)
}

pub fn stars_earned(st: &GameState) -> u32 {
    NODES.iter().map(|n| stars(st, n.id).min(max_stars(n)) as u32).sum()
}

pub fn stars_possible() -> u32 {
    NODES.iter().map(|n| max_stars(n) as u32).sum()
}

pub fn stars_spent(st: &GameState) -> u32 {
    UPGRADES.iter().map(|u| u.cost_to(level(st, u.id)) as u32).sum()
}

pub fn stars_free(st: &GameState) -> u32 {
    stars_earned(st).saturating_sub(stars_spent(st))
}

/// Keep the better score.
pub fn record(st: &mut GameState, id: &str, stars: u8) -> bool {
    let r = st.data.stages.entry(id.to_string()).or_default();
    r.clears += 1;
    let better = stars > r.stars;
    r.stars = r.stars.max(stars);
    better
}

// --- district runs --------------------------------------------------------------------

/// Called when a district is built: remember how many times you had been
/// flagged when this run began (once per run).
pub fn begin_run(st: &mut GameState, d: MapId) {
    let k = run_flagged_key(d);
    if st.get(&k).is_none() {
        let n = st.data.stats.flagged as f64;
        st.set(&k, n);
    }
}

/// A notebook page turned on a board or panel: `page` counts from 1. Pages
/// the Notebook upgrade gives you are free.
pub fn hint_read(st: &mut GameState, d: MapId, page: usize) {
    if page <= free_hint_pages(st) {
        return;
    }
    let k = run_hints_key(d);
    let n = match st.get(&k) {
        Some(crate::state::FlagValue::Num(n)) => *n,
        _ => 0.0,
    };
    st.set(&k, n + 1.0);
}

pub fn score(st: &GameState, d: MapId) -> Score {
    let num = |k: String| match st.get(&k) {
        Some(crate::state::FlagValue::Num(n)) => *n,
        _ => 0.0,
    };
    let flagged0 = num(run_flagged_key(d));
    Score { ok: [st.flag(goal(d).0), (st.data.stats.flagged as f64) <= flagged0, num(run_hints_key(d)) == 0.0], labels: DISTRICT_LABELS }
}

/// What finishing a stage looked like, for the result card.
#[derive(Clone, Debug, PartialEq)]
pub struct Result {
    pub stage: &'static str,
    pub name: &'static str,
    /// FREED or CLEARED
    pub headline: &'static str,
    pub score: Score,
    pub best: u8,
    pub improved: bool,
}

/// Score a district the moment it is freed (once per run).
pub fn settle(st: &mut GameState, d: MapId) -> Option<Result> {
    let s = score(st, d);
    if !s.cleared() || st.flag(&scored_key(d)) {
        return None;
    }
    st.set(&scored_key(d), true);
    let id = stage_id(d);
    let improved = record(st, id, s.stars());
    Some(Result { stage: id, name: node(id).map(|n| n.name).unwrap_or(id), headline: "FREED", score: s, best: stars(st, id), improved })
}

/// Score the quarters of the city: one the moment the story finishes its
/// chapter, or a gauntlet the moment its last terminal falls.
pub fn settle_city(st: &mut GameState) -> Vec<Result> {
    let mut out = Vec::new();
    if let Some(n) = gauntlet_node(st) {
        if st.flag(GAUNTLET_DONE) && !st.flag(&stage_scored_key(n.id)) {
            st.set(&stage_scored_key(n.id), true);
            out.push(settle_node(st, n));
        }
        return out;
    }
    for n in NODES.iter().filter(|n| matches!(n.kind, NodeKind::City { .. } | NodeKind::Cabin)) {
        if st.flag(n.done) && !st.flag(&stage_scored_key(n.id)) {
            st.set(&stage_scored_key(n.id), true);
            out.push(settle_node(st, n));
        }
    }
    out
}

fn settle_node(st: &mut GameState, n: &'static Node) -> Result {
    let s = city_score(&st.data, n.id);
    let improved = record(st, n.id, s.stars());
    Result { stage: n.id, name: n.name, headline: "CLEARED", score: s, best: stars(st, n.id), improved }
}

/// One star for a dive the first time it falls in a run.
pub fn settle_dives(st: &mut GameState) -> Vec<&'static str> {
    let mut out = Vec::new();
    for n in NODES {
        if let NodeKind::Dive { flag, .. } = n.kind {
            let k = format!("{flag}.scored");
            if st.flag(flag) && !st.flag(&k) {
                st.set(&k, true);
                record(st, n.id, 1);
                out.push(n.name);
            }
        }
    }
    out
}

/// A save from before the campaign: a star for everything already done.
pub fn migrate(d: &mut SaveData) {
    if d.version < 6 {
        // the quarters of the city, scored from the terminals already solved
        for n in NODES.iter().filter(|n| matches!(n.kind, NodeKind::City { .. } | NodeKind::Cabin)) {
            if data_flag(d, n.done) && !data_flag(d, &stage_scored_key(n.id)) {
                let stars = city_score(d, n.id).stars().max(1);
                d.flags.insert(stage_scored_key(n.id), true.into());
                let r = d.stages.entry(n.id.to_string()).or_default();
                r.stars = r.stars.max(stars);
                r.clears = r.clears.max(1);
            }
        }
    }
    if d.version >= 5 {
        return;
    }
    let done = |f: &str| d.flags.get(f).is_some_and(crate::state::FlagValue::truthy);
    let mut give = Vec::new();
    for dist in [MapId::Wharf, MapId::Exchange] {
        if done(goal(dist).0) {
            give.push((stage_id(dist), format!("{}.run.scored", prefix(dist))));
        }
    }
    for n in NODES {
        if let NodeKind::Dive { flag, .. } = n.kind
            && done(flag)
        {
            give.push((n.id, format!("{flag}.scored")));
        }
    }
    for (id, key) in give {
        d.flags.insert(key, true.into());
        d.stages.entry(id.to_string()).or_insert(StageRecord { stars: 1, clears: 1, replays: 0 });
    }
}

// --- replays --------------------------------------------------------------------------

/// The flags, items and chapter of a game that has just been sent into a
/// district by the story (what the skip-ahead switches and replays start from).
pub fn entry_state(d: MapId) -> (Vec<&'static str>, Vec<&'static str>, u32) {
    let mut flags = vec!["pro.intro", "pro.letter", "pro.note", "pro.booted", "pro.seed", "pro.login", "pro.done", "pro.toCity", "pro.metCat", "city.arrived", "warden.on", "ch1.met", "ch1.otp", "ch1.ctr", crate::story::ch1::HARBOUR_SENT];
    let mut items = vec!["letter", "descriptor", "gateway-key", "share1"];
    let mut chapter = 1;
    if d == MapId::Exchange {
        flags.extend([crate::signals::wharf::MAST, crate::signals::wharf::END, "w.intro", "w.started", "ch1.done", "ch2.met", "ch2.dh", "ch2.mitm", "ch2.logjam", "ch2.bookIntro", "ch2.book", "book.out", crate::story::ch2::STREET_SENT]);
        items.push("sealed-messenger");
        chapter = 2;
    }
    (flags, items, chapter)
}

/// Can this district be replayed from the map?
pub fn replayable(st: &GameState, d: MapId) -> bool {
    st.data.replay_of.is_none() && st.flag(goal(d).1)
}

/// Can this node be replayed from the map (a district, or a quarter's terminals as a gauntlet)?
pub fn can_replay(st: &GameState, n: &Node) -> bool {
    match n.kind {
        NodeKind::District(d) | NodeKind::Dive { district: d, .. } => replayable(st, d),
        NodeKind::City { .. } | NodeKind::Cabin => st.data.replay_of.is_none() && st.flag(n.done) && !terminals(n.id).is_empty(),
        // a boss is fought again in a sandbox of its district, once the story has left it
        NodeKind::Boss { district, .. } => replayable(st, district),
    }
}

/// Score a boss when it falls: ★ beaten, ★ never traced out, ★ never hit.
pub fn settle_boss(st: &mut GameState, id: &str, deaths: u32, hits: u32) -> Option<Result> {
    let n = node(id).filter(|n| matches!(n.kind, NodeKind::Boss { .. }))?;
    let s = Score { ok: [true, deaths == 0, hits == 0], labels: BOSS_LABELS };
    let improved = record(st, n.id, s.stars());
    Some(Result { stage: n.id, name: n.name, headline: "BROKEN", score: s, best: stars(st, n.id), improved })
}

/// The sandbox a boss rematch runs in: its district with every lock already
/// yours, standing in front of it.
pub fn boss_save(real: &SaveData, id: &str) -> Option<SaveData> {
    let n = node(id)?;
    let NodeKind::Boss { district, spawn, .. } = n.kind else { return None };
    let mut s = replay_save(real, district);
    // replay_save counted a district replay; this is the boss's
    let back = real.stages.get(stage_id(district)).map(|r| r.replays).unwrap_or(0);
    let k = real.stages.get(id).map(|r| r.replays).unwrap_or(0) + 1;
    for stages in [&mut s.stages, &mut s.replay_of.as_mut().expect("a sandbox").stages] {
        stages.entry(stage_id(district).to_string()).or_default().replays = back;
        stages.entry(id.to_string()).or_default().replays = k;
    }
    for site in crate::signals::wharf::sites() {
        if !matches!(site.kind, crate::signals::SiteKind::Dive(_)) {
            s.flags.insert(site.done.to_string(), true.into());
        }
    }
    s.flags.insert(scored_key(district), true.into());
    s.flags.insert("w.lamplighter.met".into(), true.into());
    let sp = crate::maps::map(district).spawn(spawn);
    s.pos = Some(crate::state::Pos { x: sp.x, y: sp.y, facing: sp.facing });
    Some(s)
}

fn fresh_seed(real: &SaveData, label: &str, n: u32) -> String {
    let seed_bytes = from_hex(&real.seed).unwrap_or_default();
    to_hex(&Drbg::new(&seed_bytes, &format!("replay/{label}/{n}")).bytes(32))
}

/// The sandbox a gauntlet runs in: your save as it is, on a fresh seed (so
/// every terminal has new numbers), with no terminals solved yet, standing
/// at the quarter's safe house, and the real save inside it.
pub fn gauntlet_save(real: &SaveData, id: &str) -> Option<SaveData> {
    let n = node(id)?;
    let k = real.stages.get(id).map(|r| r.replays).unwrap_or(0) + 1;
    let mut s = real.clone();
    s.seed = fresh_seed(real, id, k);
    s.solved.clear();
    s.flags.insert(GAUNTLET.to_string(), n.id.into());
    s.flags.remove(GAUNTLET_DONE);
    s.flags.remove(&stage_scored_key(id));
    let (map, spawn) = match n.kind {
        NodeKind::City { spawn } => (MapId::City, spawn),
        _ => (MapId::Cabin, "computer"),
    };
    let sp = crate::maps::map(map).spawn(spawn);
    s.scene = map;
    s.pos = Some(crate::state::Pos { x: sp.x, y: sp.y, facing: sp.facing });
    let mut real = real.clone();
    real.stages.entry(id.to_string()).or_default().replays = k;
    s.stages = real.stages.clone();
    s.replay_of = Some(Box::new(real));
    Some(s)
}

/// The sandbox a replay runs in: the district's entry state on fresh pads,
/// with your upgrades and stars, and the real save tucked inside it.
pub fn replay_save(real: &SaveData, d: MapId) -> SaveData {
    let n = real.stages.get(stage_id(d)).map(|r| r.replays).unwrap_or(0) + 1;
    let mut s = GameState::blank(&real.handle, real.look, Some(fresh_seed(real, stage_id(d), n)));
    let (flags, items, chapter) = entry_state(d);
    for f in flags {
        s.flags.insert(f.to_string(), true.into());
    }
    // no welcome speeches the second time round
    s.flags.insert(if d == MapId::Exchange { "x.intro" } else { "w.intro" }.to_string(), true.into());
    s.items = items.into_iter().map(String::from).collect();
    s.chapter = chapter;
    s.scene = d;
    s.codex = real.codex.clone();
    s.upgrades = real.upgrades.clone();
    let mut real = real.clone();
    real.stages.entry(stage_id(d).to_string()).or_default().replays = n;
    s.stages = real.stages.clone();
    s.replay_of = Some(Box::new(real));
    s
}

/// Back out of a replay: the real save, with whatever stars the replay won.
pub fn restore(sandbox: &SaveData) -> Option<SaveData> {
    let mut real = (**sandbox.replay_of.as_ref()?).clone();
    for (id, r) in &sandbox.stages {
        let e = real.stages.entry(id.clone()).or_default();
        e.stars = e.stars.max(r.stars);
        e.clears = e.clears.max(r.clears);
        e.replays = e.replays.max(r.replays);
    }
    for c in &sandbox.codex {
        if !real.codex.contains(c) {
            real.codex.push(c.clone());
        }
    }
    Some(real)
}

// --- upgrades -------------------------------------------------------------------------

pub struct Upgrade {
    pub id: &'static str,
    pub name: &'static str,
    /// what each level does
    pub levels: &'static [(&'static str, &'static str)],
    /// stars for each level
    pub costs: &'static [u8],
}

impl Upgrade {
    /// Stars to buy level `n` (from 1).
    pub fn cost(&self, n: u8) -> u8 {
        self.costs.get(n as usize - 1).copied().unwrap_or(n)
    }
    /// Stars spent to reach level `n`.
    pub fn cost_to(&self, n: u8) -> u8 {
        (1..=n.min(self.levels.len() as u8)).map(|k| self.cost(k)).sum()
    }
}

pub const OPSEC: &str = "opsec";
pub const WHARF_KIT: &str = "wharf";
pub const EXCHANGE_KIT: &str = "exchange";
pub const NOTEBOOK: &str = "notebook";
pub const CHAFF: &str = "chaff";
pub const TERMINAL_KIT: &str = "terminal";

pub const UPGRADES: &[Upgrade] = &[
    Upgrade {
        id: OPSEC,
        name: "Opsec",
        levels: &[
            ("Quiet phone", "Sensors log you 15% slower, everywhere."),
            ("Cold trail", "Your trace falls half again as fast when nothing sees you."),
            ("Plausible", "A lock that buzzes, a broken seal or a line that catches you lying raises your trace half as much."),
        ],
        costs: &[1, 2, 3],
    },
    Upgrade {
        id: CHAFF,
        name: "Chaff",
        levels: &[
            ("Chaff", "Q: six seconds of noise the sensors cannot tell from you, then 50 seconds to recharge. Rivest called it chaffing, 1998."),
            ("Double chaff", "Nine seconds of noise, recharging in 35."),
        ],
        costs: &[2, 3],
    },
    Upgrade { id: WHARF_KIT, name: "Wharf kit", levels: &[("Soft soles", "On the Wharf the guard has to be a fifth closer to notice you."), ("Tide table", "The mast holds each pattern 34 seconds instead of 26.")], costs: &[1, 2] },
    Upgrade { id: EXCHANGE_KIT, name: "Lineman's set", levels: &[("Hi-vis vest", "In the Exchange the agent has to be a fifth closer to notice you."), ("Work order", "The guard posts in the yard and on the tower road watch a smaller circle.")], costs: &[1, 2] },
    Upgrade { id: NOTEBOOK, name: "Grandpa's notebook", levels: &[("Margins", "The first page of a board's notes (the nudge) no longer costs a star."), ("Dog-eared", "The second page (the push) is free too. The answer never is.")], costs: &[1, 2] },
    Upgrade {
        id: TERMINAL_KIT,
        name: "Terminal kit",
        levels: &[("Forgiving", "The first wrong move on each terminal does not count against steady hands."), ("Slow clock", "Deep-dive clocks run a third longer.")],
        costs: &[1, 2],
    },
];

pub fn upgrade(id: &str) -> Option<&'static Upgrade> {
    UPGRADES.iter().find(|u| u.id == id)
}

pub fn level(st: &GameState, id: &str) -> u8 {
    st.data.upgrades.get(id).copied().unwrap_or(0)
}

/// Buy the next level of an upgrade, if there is one and you can afford it.
pub fn buy(st: &mut GameState, id: &str) -> bool {
    let Some(u) = upgrade(id) else { return false };
    let now = level(st, id);
    if now as usize >= u.levels.len() || stars_free(st) < u.cost(now + 1) as u32 {
        return false;
    }
    st.data.upgrades.insert(id.to_string(), now + 1);
    true
}

/// Take every star back out of the tree.
pub fn refund(st: &mut GameState) {
    st.data.upgrades.clear();
}

/// How much longer the deep-dive clocks run.
pub fn deep_scale(st: &GameState) -> f32 {
    if level(st, TERMINAL_KIT) >= 2 { 4.0 / 3.0 } else { 1.0 }
}

/// Chaff, once bought: (seconds of noise, seconds to recharge).
pub fn chaff_spec(st: &GameState) -> Option<(f32, f32)> {
    match level(st, CHAFF) {
        0 => None,
        1 => Some((6.0, 50.0)),
        _ => Some((9.0, 35.0)),
    }
}

/// Throw chaff, if you have it and it has recharged.
pub fn throw_chaff(st: &mut GameState) -> bool {
    let Some((on, cd)) = chaff_spec(st) else { return false };
    if st.chaff_cd > 0.0 {
        return false;
    }
    st.chaff = on;
    st.chaff_cd = cd;
    true
}

pub fn tick_chaff(st: &mut GameState, dt: f32) {
    st.chaff = (st.chaff - dt).max(0.0);
    st.chaff_cd = (st.chaff_cd - dt).max(0.0);
    st.dodge = (st.dodge - dt).max(0.0);
    st.dodge_cd = (st.dodge_cd - dt).max(0.0);
}

// --- souls: the dodge, the lanterns, what you drop ---------------------------------------

/// Seconds a dodge lasts, its cooldown, and the trace a sudden move costs.
pub const DODGE: f32 = 0.3;
pub const DODGE_CD: f32 = 0.9;
pub const DODGE_COST: f32 = 4.0;

/// Start a dodge if it has recovered. A sudden move draws the eye a little.
pub fn try_dodge(st: &mut GameState) -> bool {
    if st.dodge_cd > 0.0 {
        return false;
    }
    st.dodge = DODGE;
    st.dodge_cd = DODGE_CD;
    if st.settings.dragnet {
        st.trace = (st.trace + DODGE_COST).min(99.0);
    }
    true
}

/// Where you rest (a lantern at a safe house, or at a district's start).
pub const REST_AT: &str = "rest.last";

/// Rest at a lantern: the trace drops, chaff recharges, the cards in your
/// pocket are safe from your next fall, and the street will send you back
/// here when it catches you.
pub fn rest(st: &mut GameState, at: &str) {
    st.trace = 0.0;
    st.chaff_cd = 0.0;
    st.data.banked = st.data.cards.iter().map(|c| c.id).collect();
    st.set(REST_AT, at);
}

/// What you dropped when they caught you.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Dropped {
    pub map: MapId,
    pub x: f32,
    pub y: f32,
    pub cards: Vec<crate::signals::Card>,
}

/// Cards a fall would cost you: rubbings and cuts made since you last rested
/// (pads are never dropped; every rubbing and cut can be made again).
pub fn loose(st: &GameState) -> Vec<u32> {
    st.data.cards.iter().filter(|c| c.kind != crate::signals::CardKind::Pad && !st.data.banked.contains(&c.id)).map(|c| c.id).collect()
}

/// They caught you at (x, y): drop the loose cards there. Anything still
/// lying where they caught you last time is gone for good. Returns (dropped, lost).
pub fn drop_loose(st: &mut GameState, map: MapId, x: f32, y: f32) -> (usize, usize) {
    let ids = loose(st);
    let lost = st.data.dropped.take().map(|d| d.cards.len()).unwrap_or(0);
    if ids.is_empty() {
        return (0, lost);
    }
    let cards: Vec<crate::signals::Card> = st.data.cards.iter().filter(|c| ids.contains(&c.id)).cloned().collect();
    st.data.cards.retain(|c| !ids.contains(&c.id));
    let n = cards.len();
    st.data.dropped = Some(Dropped { map, x, y, cards });
    (n, lost)
}

/// Walk over what you dropped and it is yours again.
pub fn pick_up(st: &mut GameState, map: MapId, x: f32, y: f32) -> Option<usize> {
    let d = st.data.dropped.as_ref()?;
    if d.map != map || (d.x - x).powi(2) + (d.y - y).powi(2) > 18.0 * 18.0 {
        return None;
    }
    let d = st.data.dropped.take()?;
    let n = d.cards.len();
    for c in d.cards {
        if !st.data.cards.iter().any(|x| x.id == c.id) {
            st.data.cards.push(c);
        }
    }
    Some(n)
}

/// Notebook pages that cost nothing.
pub fn free_hint_pages(st: &GameState) -> usize {
    level(st, NOTEBOOK) as usize
}

/// Multiply a trace penalty (a buzz, an alarm, a lie caught on a line).
pub fn penalty(st: &GameState, x: f32) -> f32 {
    if level(st, OPSEC) >= 3 { x * 0.5 } else { x }
}

/// The trace's fall per second.
pub fn decay(st: &GameState, base: f32) -> f32 {
    if level(st, OPSEC) >= 2 { base * 1.5 } else { base }
}

/// The opsec upgrade's share of the trace gain.
pub fn gain_factor(st: &GameState) -> f32 {
    if level(st, OPSEC) >= 1 { 0.85 } else { 1.0 }
}

/// How close a district's walking watcher has to be.
pub fn guard_radius(st: &GameState, d: MapId, base: f32) -> f32 {
    let kit = if d == MapId::Exchange { EXCHANGE_KIT } else { WHARF_KIT };
    if level(st, kit) >= 1 { base * 0.8 } else { base }
}

pub fn post_radius(st: &GameState, base: f32) -> f32 {
    if level(st, EXCHANGE_KIT) >= 2 { base * 0.75 } else { base }
}

pub fn mast_period(st: &GameState) -> f32 {
    if level(st, WHARF_KIT) >= 2 { 34.0 } else { crate::signals::wharf::MAST_PERIOD }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game() -> GameState {
        let mut s = GameState::new();
        s.new_game("t", 0, Some("campaign"));
        s
    }

    #[test]
    fn stars_add_up_and_buy_upgrades() {
        let mut s = game();
        assert_eq!(stars_possible(), 33, "seven quarters, two districts and a boss at three, three dives at one");
        assert_eq!(stars_free(&s), 0);
        assert!(!buy(&mut s, OPSEC));
        record(&mut s, "wharf", 2);
        record(&mut s, "wharf", 1);
        assert_eq!(stars(&s, "wharf"), 2, "the best score stays");
        record(&mut s, "static", 1);
        assert_eq!(stars_free(&s), 3);
        assert!(buy(&mut s, OPSEC));
        assert!(buy(&mut s, OPSEC));
        assert_eq!(stars_spent(&s), 3);
        assert!(!buy(&mut s, OPSEC), "level 3 costs 3 more");
        assert!((gain_factor(&s) - 0.85).abs() < 1e-6 && (decay(&s, 10.0) - 15.0).abs() < 1e-6);
        assert_eq!(penalty(&s, 10.0), 10.0);
        refund(&mut s);
        assert_eq!(stars_free(&s), 3);
    }

    #[test]
    fn a_run_scores_clear_unflagged_and_hintless() {
        let mut s = game();
        begin_run(&mut s, MapId::Wharf);
        assert_eq!(score(&s, MapId::Wharf).stars(), 0);
        s.set(crate::signals::wharf::MAST, true);
        assert_eq!(score(&s, MapId::Wharf).stars(), 3);
        s.data.stats.flagged += 1;
        begin_run(&mut s, MapId::Wharf); // a second build of the district does not reset the run
        assert_eq!(score(&s, MapId::Wharf).stars(), 2);
        hint_read(&mut s, MapId::Wharf, 1);
        assert_eq!(score(&s, MapId::Wharf).stars(), 1);
        let r = settle(&mut s, MapId::Wharf).unwrap();
        assert_eq!((r.best, r.improved), (1, true));
        assert!(settle(&mut s, MapId::Wharf).is_none(), "scored once per run");
    }

    #[test]
    fn the_notebook_upgrade_makes_early_pages_free() {
        let mut s = game();
        record(&mut s, "wharf", 3);
        assert!(buy(&mut s, NOTEBOOK));
        begin_run(&mut s, MapId::Exchange);
        s.set(crate::lines::exchange::FREE, true);
        hint_read(&mut s, MapId::Exchange, 1);
        assert!(score(&s, MapId::Exchange).ok[2]);
        hint_read(&mut s, MapId::Exchange, 2);
        assert!(!score(&s, MapId::Exchange).ok[2]);
    }

    #[test]
    fn a_replay_is_a_sandbox_on_fresh_pads() {
        let mut s = game();
        let (flags, _, _) = entry_state(MapId::Exchange);
        for f in flags {
            s.set(f, true);
        }
        s.set(crate::lines::exchange::END, true);
        record(&mut s, "exchange", 1);
        s.data.upgrades.insert(OPSEC.into(), 1);
        assert!(replayable(&s, MapId::Exchange));
        let sb = replay_save(&s.data, MapId::Exchange);
        assert_ne!(sb.seed, s.data.seed);
        assert_eq!(sb.scene, MapId::Exchange);
        assert!(!sb.flags.contains_key(crate::lines::exchange::FREE));
        assert_eq!(sb.upgrades.get(OPSEC), Some(&1));
        let again = replay_save(&s.data, MapId::Exchange);
        assert_eq!(again.seed, sb.seed, "the same replay number gives the same pads");
        // win three stars inside, then come back out
        let mut inside = GameState::new();
        assert!(inside.load(sb.clone()));
        assert!(!replayable(&inside, MapId::Exchange), "no replay inside a replay");
        record(&mut inside, "exchange", 3);
        let back = restore(&inside.data).unwrap();
        assert_eq!(back.seed, s.data.seed);
        assert_eq!(back.stages["exchange"].stars, 3);
        assert_eq!(back.stages["exchange"].replays, 1);
        assert!(back.replay_of.is_none());
        let next = replay_save(&back, MapId::Exchange);
        assert_ne!(next.seed, sb.seed, "the next replay gets new pads");
    }

    #[test]
    fn old_saves_get_a_star_for_what_they_finished() {
        let mut s = game();
        s.data.version = 4;
        s.data.flags.insert(crate::signals::wharf::MAST.into(), true.into());
        s.data.flags.insert(crate::signals::wharf::DIVE_STATIC.into(), true.into());
        migrate(&mut s.data);
        assert_eq!(s.data.stages["wharf"].stars, 1);
        assert_eq!(s.data.stages["static"].stars, 1);
        assert!(!s.data.stages.contains_key("exchange"));
        let mut t = GameState::new();
        assert!(t.load(s.data.clone()));
        assert!(settle(&mut t, MapId::Wharf).is_none(), "a finished district is not scored again");
    }

    #[test]
    fn a_quarter_scores_from_its_terminals() {
        let mut s = game();
        let solve = |s: &mut GameState, id: &str, attempts: u32, guided: bool| {
            s.data.solved.insert(id.into(), crate::state::Solved { at: 0.0, attempts, guided });
        };
        assert_eq!(city_score(&s.data, "commons").stars(), 0);
        solve(&mut s, "ch3.shamir", 1, false);
        solve(&mut s, "ch3.feldman", 3, false);
        assert_eq!(city_score(&s.data, "commons").stars(), 0, "one terminal still open");
        solve(&mut s, "ch3.cash", 2, false);
        // mistakes: 0 + 2 + 1 = 3 <= 3 terminals
        assert_eq!(city_score(&s.data, "commons").stars(), 3);
        s.data.solved.get_mut("ch3.cash").unwrap().guided = true;
        assert_eq!(city_score(&s.data, "commons").stars(), 2, "one terminal had its hints on");
        s.data.solved.get_mut("ch3.feldman").unwrap().attempts = 5;
        assert_eq!(city_score(&s.data, "commons").ok, [true, false, false]);
        s.data.upgrades.insert(TERMINAL_KIT.into(), 1);
        assert!(city_score(&s.data, "commons").ok[1], "forgiving: 0 + 3 + 0 <= 3");
        // the story closes the chapter: scored once, with a card
        s.set("ch3.done", true);
        let r = settle_city(&mut s);
        assert_eq!(r.len(), 1);
        assert_eq!((r[0].stage, r[0].headline, r[0].best), ("commons", "CLEARED", 2));
        assert!(settle_city(&mut s).is_empty());
    }

    #[test]
    fn a_gauntlet_runs_on_fresh_numbers_and_scores_in_the_sandbox() {
        let mut s = game();
        s.set("city.arrived", true);
        s.set("ch1.ctr", true);
        s.data.solved.insert("ch1.otp".into(), crate::state::Solved { at: 0.0, attempts: 1, guided: true });
        s.data.solved.insert("ch1.ctr".into(), crate::state::Solved { at: 0.0, attempts: 1, guided: true });
        settle_city(&mut s);
        assert_eq!(stars(&s, "radio"), 2);
        assert!(can_replay(&s, node("radio").unwrap()));
        assert!(!can_replay(&s, node("commons").unwrap()), "its chapter is not done");
        let sb = gauntlet_save(&s.data, "radio").unwrap();
        assert_ne!(sb.seed, s.data.seed);
        assert!(sb.solved.is_empty() && sb.scene == MapId::City);
        let mut inside = GameState::new();
        assert!(inside.load(sb));
        assert_eq!(gauntlet_node(&inside).map(|n| n.id), Some("radio"));
        assert!(settle_city(&mut inside).is_empty(), "nothing scored until the last terminal");
        for id in terminals("radio") {
            inside.mark_solved(id, 1, false);
        }
        inside.set(GAUNTLET_DONE, true);
        let r = settle_city(&mut inside);
        assert_eq!(r[0].best, 3);
        let back = restore(&inside.data).unwrap();
        assert_eq!(back.stages["radio"].stars, 3);
        assert_eq!(back.stages["radio"].replays, 1);
        assert!(!back.flags.contains_key(GAUNTLET), "the real save never sees the gauntlet");
        assert_eq!(back.solved.len(), 2);
    }

    #[test]
    fn chaff_hides_you_then_recharges() {
        let mut s = game();
        assert!(!throw_chaff(&mut s), "not bought");
        s.data.upgrades.insert(CHAFF.into(), 1);
        assert!(throw_chaff(&mut s));
        assert!(!throw_chaff(&mut s), "recharging");
        s.set("city.arrived", true);
        let m = crate::maps::map(MapId::City);
        let mast = m.sensors.iter().find(|x| x.id == "mast-wharf").unwrap().clone();
        s.trace = 50.0;
        s.dragnet(&m, mast.x, mast.y, 0.0, 1.0);
        assert!(s.trace < 50.0, "a sensor sees you through the chaff, but the trace still falls");
        tick_chaff(&mut s, 6.0);
        s.dragnet(&m, mast.x, mast.y, 0.0, 1.0);
        assert!(s.trace > 40.0);
        tick_chaff(&mut s, 50.0);
        assert!(throw_chaff(&mut s));
    }

    #[test]
    fn upgrades_cost_what_their_table_says() {
        let u = upgrade(CHAFF).unwrap();
        assert_eq!((u.cost(1), u.cost(2), u.cost_to(2)), (2, 3, 5));
        let total: u32 = UPGRADES.iter().map(|u| u.cost_to(u.levels.len() as u8) as u32).sum();
        assert!(total < stars_possible(), "the tree can be filled, eventually");
        assert!(total > stars_possible() / 2, "but not halfway through the game");
    }

    #[test]
    fn a_dodge_hides_you_for_a_moment_at_a_small_cost() {
        let mut s = game();
        s.set("city.arrived", true);
        let m = crate::maps::map(MapId::City);
        let mast = m.sensors.iter().find(|x| x.id == "mast-wharf").unwrap().clone();
        assert!(try_dodge(&mut s));
        assert_eq!(s.trace, DODGE_COST);
        assert!(!try_dodge(&mut s), "it has to recover");
        s.dragnet(&m, mast.x, mast.y, 0.0, 0.1);
        assert!(s.trace <= DODGE_COST, "nothing to fix on mid-dodge");
        tick_chaff(&mut s, DODGE_CD);
        s.dragnet(&m, mast.x, mast.y, 0.0, 0.5);
        assert!(s.trace > DODGE_COST + 5.0);
        assert!(try_dodge(&mut s));
    }

    #[test]
    fn a_fall_drops_what_you_have_not_banked_and_a_second_fall_loses_it() {
        let mut s = game();
        let card = |id: u32, kind: crate::signals::CardKind| crate::signals::Card { id, name: format!("C{id}"), bits: "00ff".into(), kind, note: String::new() };
        s.data.cards = vec![card(1, crate::signals::CardKind::Pad), card(2, crate::signals::CardKind::Copy)];
        rest(&mut s, "wharf");
        s.data.cards.push(card(3, crate::signals::CardKind::Cut));
        s.data.cards.push(card(4, crate::signals::CardKind::Pad));
        assert_eq!(loose(&s), vec![3], "pads never fall, banked cards are safe");
        assert_eq!(drop_loose(&mut s, MapId::Wharf, 10.0, 10.0), (1, 0));
        assert_eq!(s.data.cards.len(), 3);
        s.data.cards.push(card(5, crate::signals::CardKind::Copy));
        assert_eq!(drop_loose(&mut s, MapId::Wharf, 50.0, 50.0), (1, 1), "the first stain is gone");
        assert!(pick_up(&mut s, MapId::Wharf, 10.0, 10.0).is_none());
        assert!(pick_up(&mut s, MapId::Exchange, 50.0, 50.0).is_none(), "on the map you fell on");
        assert_eq!(pick_up(&mut s, MapId::Wharf, 55.0, 45.0), Some(1));
        assert!(s.data.cards.iter().any(|c| c.id == 5) && !s.data.cards.iter().any(|c| c.id == 3));
        assert_eq!(s.get(REST_AT), Some(&crate::state::FlagValue::Str("wharf".into())));
    }

    #[test]
    fn a_boss_scores_on_its_record_and_rematches_in_a_sandbox() {
        let mut s = game();
        let r = settle_boss(&mut s, "lamplighter", 0, 2).unwrap();
        assert_eq!((r.best, r.score.ok), (2, [true, true, false]));
        assert!(settle_boss(&mut s, "wharf", 0, 0).is_none(), "not a boss");
        assert!(!can_replay(&s, node("lamplighter").unwrap()));
        s.set(crate::signals::wharf::END, true);
        assert!(can_replay(&s, node("lamplighter").unwrap()));
        let sb = boss_save(&s.data, "lamplighter").unwrap();
        assert_eq!(sb.scene, MapId::Wharf);
        assert!(data_flag(&sb, crate::signals::wharf::MAST) && data_flag(&sb, crate::signals::wharf::GATE_W));
        assert!(!data_flag(&sb, crate::signals::wharf::DIVE_STATIC), "no free dive stars");
        assert_eq!(sb.stages["lamplighter"].replays, 1);
    }

    #[test]
    fn every_node_road_and_spawn_exists() {
        for n in NODES {
            if let Some(f) = n.from {
                assert!(node(f).is_some(), "{} comes from unknown {f}", n.id);
            }
            match n.kind {
                NodeKind::City { spawn } => assert!(crate::maps::map(MapId::City).spawns.contains_key(spawn), "{spawn}"),
                NodeKind::Dive { district, spawn, .. } | NodeKind::Boss { district, spawn, .. } => assert!(crate::maps::map(district).spawns.contains_key(spawn), "{spawn} on {}", district.name()),
                _ => {}
            }
        }
    }
}
