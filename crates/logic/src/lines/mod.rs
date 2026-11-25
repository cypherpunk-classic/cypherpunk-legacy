//! Lines: the puzzle language of the Exchange.
//!
//! A panel is a row of jacks, each the end of a line somewhere in the
//! district: a gate, a guard's call box, the Directorate's desk, the tower,
//! your own handset. A cord joins two jacks, and whatever one end says, the
//! other end hears. That is the whole rule. Nobody on a line can see who is
//! at the other end of it.
//!
//! Some lines are scrambled: both ends run Diffie-Hellman
//! ([`cpl_crypto::dh`]) and talk under the key they agree, so a cord that
//! only listens hears noise. Some scramblers then read out a word or two
//! derived from the key, a short authentication string; some commit to
//! their number before they see the other side's, a sealed handshake as in
//! ZRTP. Your relay box is two scramblers back to back: put it in the middle
//! and each end agrees a key with the box instead of with each other. The
//! tower line runs in an export-grade group, small enough to take apart
//! with baby-step giant-step, and every phone on the trunk carries a Clipper
//! LEAF with a 16-bit checksum ([`cpl_crypto::clipper`]).
//!
//! Everything a player can do is here, with no user interface: plug and
//! pull cords, answer a call on the handset, hold, turn and connect the
//! relay, change the orders it passes on, record a call, place a call,
//! forge a LEAF. Calls run on the district's clock.

pub mod exchange;

use cpl_crypto::aead::kdf;
use cpl_crypto::bytes::to_hex;
use cpl_crypto::clipper::{Escrow, LeafSearch};
use cpl_crypto::dh::Group;
use cpl_crypto::drbg::Drbg;

use crate::state::GameState;

pub type Word = &'static str;

/// Words a scrambler reads out once its key is agreed: five bits each.
pub const SAS_WORDS: [&str; 32] = [
    "ACORN", "ANVIL", "BASALT", "BEACON", "BRAMBLE", "CANVAS", "CEDAR", "CINDER", "COBALT", "DELTA", "EMBER", "FERN", "FLINT", "GARNET", "GLACIER", "HAZEL", "IVORY", "JASPER", "JUNIPER", "LANTERN", "MAPLE", "MARBLE", "MEADOW", "NECTAR", "ORCHID", "PEBBLE", "QUARTZ", "RIVER", "SAFFRON", "THISTLE", "VELVET", "WILLOW",
];

/// Orders a post understands, and the camera's.
pub const ORDERS: [Word; 5] = ["HOLD", "NORTH", "SOUTH", "EAST", "WEST"];
pub const CAMERA_ORDERS: [Word; 2] = ["WATCH", "SLEEP"];

/// Trace for a wrong countersign.
pub const WRONG: f32 = 25.0;
/// How long a relay may hold a call before both ends give up, and how long
/// a caller waits for your answer.
pub const HOLD_MAX: f32 = 20.0;
pub const ANSWER_MAX: f32 = 12.0;
/// LEAFs your phone tries against the far chip per second.
pub const FORGE_RATE: f32 = 6000.0;

// beats of a call, in seconds
const RING: f32 = 0.9;
const SHAKE: f32 = 1.1;
const WORDS: f32 = 1.5;
const SAY: f32 = 1.2;
const SHUT_AFTER: f32 = 3.0;
const HANG_UP: f32 = 1.2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    L,
    R,
}

impl Side {
    pub fn other(self) -> Side {
        match self {
            Side::L => Side::R,
            Side::R => Side::L,
        }
    }
}

/// Who is at the end of a jack.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum End {
    /// someone on a line in the district, by party id
    Line(&'static str),
    /// your own handset
    Handset,
    /// your relay box: two scramblers back to back
    Relay(Side),
    /// the Directorate's monitoring position on the trunk: their relay
    Monitor(Side),
}

/// How a line talks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sec {
    Clear,
    Scrambled {
        group: Group,
        /// words read out after the handshake (0: none)
        words: u8,
        /// both ends commit to their numbers before seeing the other's
        sealed: bool,
    },
}

impl Sec {
    pub fn scrambled(&self) -> bool {
        matches!(self, Sec::Scrambled { .. })
    }
}

/// What a word does to the one who hears it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    Flag(&'static str),
    Alarm,
    Nothing,
}

#[derive(Clone, Debug)]
pub enum Role {
    /// says `open` when it calls and `shut` a little later (the tram depot)
    Depot { open: Word, shut: Word },
    /// does what it is told: `open` sets `flag` and the latch holds
    Gate { open: Word, flag: Option<&'static str> },
    /// asks for orders with a challenge, and acts on the countersign and the order
    Post { table: Vec<(Word, Word)>, obey: Vec<(Word, Effect)> },
    /// asks for orders with no challenge
    Camera { obey: Vec<(Word, Effect)> },
    /// the Directorate's desk: answers posts and the camera, calls the tower with a code
    Desk { post: Word, camera: Word, code: Vec<u8> },
    /// takes maintenance calls
    Tower,
    /// Mara, at the far end of the trunk; dead until `live` is set
    Trunk { live: &'static str },
}

#[derive(Clone, Debug)]
pub struct Party {
    pub id: &'static str,
    pub name: &'static str,
    pub role: Role,
    pub sec: Sec,
    /// trace when this line catches tampering
    pub tamper: f32,
}

#[derive(Clone, Debug)]
pub struct Jack {
    pub end: End,
    pub label: &'static str,
    /// a flag you need before this jack is any use to you (your relay's)
    pub needs: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    /// speech in the clear
    Plain,
    /// a scrambled line, as a cord that only listens hears it
    Noise,
    /// what your relay decrypts
    Relay,
    /// the panel's own notes (rings, hang-ups)
    System,
    Good,
    Bad,
}

#[derive(Clone, Debug)]
pub struct LogLine {
    pub t: f32,
    pub who: String,
    pub text: String,
    pub tone: Tone,
}

/// Your relay box's settings on one panel.
#[derive(Clone, Debug, Default)]
pub struct RelaySet {
    /// times the knob has been turned (each turn draws a new number for the far side)
    pub turns: u32,
    /// the order passed on instead of the one that was given
    pub rewrite: Option<Word>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Via {
    Direct,
    Relay(Side),
    Monitor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Ring,
    Shake,
    /// your relay holds the call until you connect it
    Hold,
    Words,
    Ask,
    /// waiting for the far end (or you) to answer
    Answer,
    Back,
    /// the depot's second word
    Shut,
    Escrow,
    /// your phone is trying LEAFs against the far chip
    Forge,
    Done,
}

/// A handshake: who sent which number and who ended up with which key.
#[derive(Clone, Debug)]
pub struct Keys {
    pub group: Group,
    pub words: u8,
    pub sealed: bool,
    /// the caller's public number, and the far end's (if it is a scrambler)
    pub a_pub: u64,
    pub b_pub: Option<u64>,
    /// the middle box's numbers: the one the caller saw, and the one the far end saw
    pub mid: Option<(u64, Option<u64>)>,
    /// the key the caller holds, and the far end's
    pub k_near: u64,
    pub k_far: Option<u64>,
}

impl Keys {
    pub fn word_near(&self) -> String {
        sas(&self.group, self.k_near, self.words)
    }

    pub fn word_far(&self) -> Option<String> {
        self.k_far.map(|k| sas(&self.group, k, self.words))
    }

    pub fn matched(&self) -> bool {
        self.word_far().is_some_and(|w| w == self.word_near())
    }
}

/// One call on a panel.
#[derive(Clone, Debug)]
pub struct Call {
    pub n: u32,
    pub caller: usize,
    /// the jack that answers (beyond the box in the middle, if any)
    pub far: Option<usize>,
    pub via: Via,
    pub step: Step,
    pub t: f32,
    pub keys: Option<Keys>,
    pub challenge: Option<Word>,
    /// what came back: (countersign, order)
    pub reply: Option<(Option<Word>, Word)>,
    pub connected: bool,
    pub search: Option<LeafSearch>,
    /// the cords the call rides on (pull one and the line goes dead)
    pub cords: Vec<(usize, usize)>,
    /// LEAF tries already reported in the log
    pub told: u64,
}

pub struct Panel {
    pub id: &'static str,
    pub site: &'static str,
    pub title: &'static str,
    pub jacks: Vec<Jack>,
    pub cords: Vec<(usize, usize)>,
    pub initial: Vec<(usize, usize)>,
    pub max_cords: usize,
    /// the line that calls on its own: (jack, period, first call)
    pub schedule: Option<(usize, f32, f32)>,
    /// once this flag is set the caller has somewhere else to be, and stops calling
    pub until: Option<&'static str>,
    pub log: Vec<LogLine>,
    pub call: Option<Call>,
    pub calls: u32,
    /// challenge and countersign pairs heard in the clear at this panel
    pub heard: Vec<(Word, Word)>,
    pub recorder: bool,
    pub recording: bool,
    /// your handset here is a scrambler phone that can call out
    pub phone: Option<Sec>,
    pub forged: bool,
    pub relay: RelaySet,
    pub t: f32,
}

impl Panel {
    pub fn partner(&self, jack: usize) -> Option<usize> {
        self.cords.iter().find_map(|&(a, b)| {
            if a == jack {
                Some(b)
            } else if b == jack {
                Some(a)
            } else {
                None
            }
        })
    }

    pub fn jack_of(&self, end: End) -> Option<usize> {
        self.jacks.iter().position(|j| j.end == end)
    }

    pub fn usable(&self, st: &GameState, jack: usize) -> bool {
        self.jacks.get(jack).is_some_and(|j| j.needs.is_none_or(|f| st.flag(f)))
    }

    /// Follow the cords from a caller: the far jack and what sits between.
    pub fn route(&self, caller: usize) -> (Option<usize>, Via, Vec<(usize, usize)>) {
        let Some(p) = self.partner(caller) else { return (None, Via::Direct, Vec::new()) };
        let mut cords = vec![(caller, p)];
        let through = match self.jacks[p].end {
            End::Relay(s) => Some((End::Relay(s.other()), Via::Relay(s))),
            End::Monitor(s) => Some((End::Monitor(s.other()), Via::Monitor)),
            _ => None,
        };
        match through {
            None => (Some(p), Via::Direct, cords),
            Some((exit, via)) => {
                let Some(x) = self.jack_of(exit) else { return (None, via, cords) };
                match self.partner(x) {
                    Some(f) if !matches!(self.jacks[f].end, End::Relay(_) | End::Monitor(_)) => {
                        cords.push((x, f));
                        (Some(f), via, cords)
                    }
                    _ => (None, via, cords),
                }
            }
        }
    }

    fn line(&mut self, who: &str, text: impl Into<String>, tone: Tone) {
        self.log.push(LogLine { t: self.t, who: who.to_string(), text: text.into(), tone });
        if self.log.len() > 80 {
            self.log.remove(0);
        }
    }

    fn cord_alive(&self, c: (usize, usize)) -> bool {
        self.cords.iter().any(|&(a, b)| (a, b) == c || (b, a) == c)
    }
}

/// The words a scrambler reads for key `k`.
pub fn sas(group: &Group, k: u64, words: u8) -> String {
    let h = group.session_key(k);
    (0..words.max(1) as usize).map(|i| SAS_WORDS[(h[i] & 31) as usize]).collect::<Vec<_>>().join(" ")
}

/// A line's text under key `k`, as the bytes on the wire (a keystream from the key).
pub fn cipher(group: &Group, k: u64, n: u32, text: &str) -> Vec<u8> {
    let ks = kdf(&group.session_key(k), &format!("line {n}"));
    text.bytes().take(32).zip(ks).map(|(b, s)| b ^ s).collect()
}

pub fn decipher(group: &Group, k: u64, n: u32, bytes: &[u8]) -> String {
    let ks = kdf(&group.session_key(k), &format!("line {n}"));
    bytes.iter().zip(ks).map(|(b, s)| (b ^ s) as char).collect()
}

fn hex_groups(b: &[u8]) -> String {
    let h = to_hex(b);
    h.as_bytes().chunks(4).map(|c| std::str::from_utf8(c).unwrap_or("")).collect::<Vec<_>>().join(" ")
}

pub fn short(n: u64) -> String {
    let h = format!("{n:x}");
    if h.len() > 8 { format!("{}…", &h[..8]) } else { h }
}

/// A secret in `group` from a labelled stream.
fn secret(st: &GameState, group: &Group, label: &str) -> u64 {
    let mut r: Drbg = st.world().rng(label);
    let x = u64::from_be_bytes(r.array::<8>());
    2 + x % (group.q - 3)
}

/// Things the rest of the game reacts to.
#[derive(Clone, Debug, PartialEq)]
pub enum LineEvent {
    Ring { panel: &'static str, party: &'static str },
    Plug { panel: &'static str },
    Solved { panel: &'static str, flag: &'static str },
    Alarm { panel: &'static str },
    Tamper { panel: &'static str },
    NoCarrier { panel: &'static str },
    Heard { panel: &'static str },
    Taped { panel: &'static str },
    TapeRead,
    Escrow { panel: &'static str },
    Forged { panel: &'static str },
}

/// What a recorder took off the tower line.
#[derive(Clone, Debug, PartialEq)]
pub struct Tape {
    pub n: u32,
    pub group: Group,
    pub a_pub: u64,
    pub b_pub: u64,
    pub cipher: Vec<u8>,
}

/// The district's lines, live.
pub struct Lines {
    pub parties: Vec<Party>,
    pub panels: Vec<Panel>,
    pub escrow: Escrow,
    pub events: Vec<LineEvent>,
    /// flags that, once set, make the district say something (handled in `tick`)
    pub tape_read: bool,
}

impl Lines {
    pub fn panel(&self, id: &str) -> Option<&Panel> {
        self.panels.iter().find(|p| p.id == id)
    }

    pub fn panel_mut(&mut self, id: &str) -> Option<&mut Panel> {
        self.panels.iter_mut().find(|p| p.id == id)
    }

    pub fn party(&self, id: &str) -> Option<&Party> {
        self.parties.iter().find(|p| p.id == id)
    }

    /// The name on a jack's end.
    pub fn end_name(&self, end: End) -> String {
        match end {
            End::Line(id) => self.party(id).map(|p| p.name).unwrap_or(id).to_string(),
            End::Handset => "YOU".into(),
            End::Relay(Side::L) => "RELAY L".into(),
            End::Relay(Side::R) => "RELAY R".into(),
            End::Monitor(Side::L) => "MONITOR L".into(),
            End::Monitor(Side::R) => "MONITOR R".into(),
        }
    }

    fn sec_of(&self, p: &Panel, end: End) -> Option<Sec> {
        match end {
            End::Line(id) => self.party(id).map(|x| x.sec),
            End::Handset => Some(p.phone.unwrap_or(Sec::Clear)),
            End::Relay(_) | End::Monitor(_) => None,
        }
    }

    // --- what the player does --------------------------------------------------------

    /// Plug a cord between two free jacks.
    pub fn plug(&mut self, st: &GameState, panel: &str, a: usize, b: usize) -> bool {
        let Some(p) = self.panel_mut(panel) else { return false };
        if a == b || a >= p.jacks.len() || b >= p.jacks.len() || p.partner(a).is_some() || p.partner(b).is_some() || p.cords.len() >= p.max_cords || !p.usable(st, a) || !p.usable(st, b) {
            return false;
        }
        p.cords.push((a, b));
        let id = p.id;
        self.events.push(LineEvent::Plug { panel: id });
        true
    }

    /// Pull the cord in this jack (both of its plugs come out).
    pub fn pull(&mut self, panel: &str, jack: usize) -> bool {
        let Some(p) = self.panel_mut(panel) else { return false };
        let before = p.cords.len();
        p.cords.retain(|&(a, b)| a != jack && b != jack);
        p.cords.len() != before
    }

    /// Let a held call through your relay.
    pub fn connect(&mut self, panel: &str) -> bool {
        let Some(p) = self.panel_mut(panel) else { return false };
        match p.call.as_mut() {
            Some(c) if c.step == Step::Hold && !c.connected => {
                c.connected = true;
                true
            }
            _ => false,
        }
    }

    /// Turn the relay's knob: a new number for the far side, and a new far word.
    pub fn turn(&mut self, st: &GameState, panel: &str) -> bool {
        let Some(pi) = self.panels.iter().position(|p| p.id == panel) else { return false };
        let sealed = self.panels[pi].call.as_ref().and_then(|c| c.keys.as_ref()).is_some_and(|k| k.sealed);
        let holding = self.panels[pi].call.as_ref().is_some_and(|c| c.step == Step::Hold && matches!(c.via, Via::Relay(_)));
        if !holding || sealed {
            return false;
        }
        self.panels[pi].relay.turns += 1;
        let keys = self.handshake(st, pi);
        if let Some(c) = self.panels[pi].call.as_mut() {
            c.keys = keys;
        }
        true
    }

    pub fn set_rewrite(&mut self, panel: &str, w: Option<Word>) {
        if let Some(p) = self.panel_mut(panel) {
            p.relay.rewrite = w;
        }
    }

    /// Answer the call waiting on your handset.
    pub fn answer(&mut self, panel: &str, countersign: Option<Word>, order: Word) -> bool {
        let Some(p) = self.panel_mut(panel) else { return false };
        let ok = p.call.as_ref().is_some_and(|c| c.step == Step::Answer && c.far.is_some_and(|f| p.jacks[f].end == End::Handset));
        if !ok {
            return false;
        }
        let who = "YOU";
        let text = match countersign {
            Some(cs) => format!("{cs}. {order}."),
            None => format!("{order}."),
        };
        p.line(who, text, Tone::Plain);
        if let Some(c) = p.call.as_mut() {
            c.reply = Some((countersign, order));
            c.step = Step::Back;
            c.t = 0.0;
        }
        true
    }

    /// Arm the recorder for the next call.
    pub fn record(&mut self, panel: &str) -> bool {
        let Some(p) = self.panel_mut(panel) else { return false };
        if !p.recorder || p.recording {
            return false;
        }
        p.recording = true;
        p.line("", "Recorder armed: it takes the next call on this panel.", Tone::System);
        true
    }

    pub fn set_forged(&mut self, panel: &str, forged: bool) {
        if let Some(p) = self.panel_mut(panel) {
            p.forged = forged;
        }
    }

    /// Pick up your phone and call whatever your handset is plugged into.
    pub fn dial(&mut self, st: &mut GameState, panel: &str) -> bool {
        let Some(pi) = self.panels.iter().position(|p| p.id == panel) else { return false };
        let p = &self.panels[pi];
        if p.phone.is_none() || p.call.is_some() {
            return false;
        }
        let Some(h) = p.jack_of(End::Handset) else { return false };
        self.start(st, pi, h);
        true
    }

    /// Being caught: they pull your cords and put theirs back.
    pub fn reset(&mut self) {
        for p in &mut self.panels {
            p.cords = p.initial.clone();
            p.call = None;
            p.recording = false;
            p.relay = RelaySet::default();
            p.forged = false;
        }
    }

    // --- the clock -----------------------------------------------------------------------

    pub fn tick(&mut self, st: &mut GameState, dt: f32) {
        for pi in 0..self.panels.len() {
            let before = self.panels[pi].t;
            self.panels[pi].t += dt;
            let now = self.panels[pi].t;
            let off_duty = self.panels[pi].until.is_some_and(|f| st.flag(f));
            if let Some((jack, period, at)) = self.panels[pi].schedule
                && !off_duty
                && now >= at
                && (before < at || ((now - at) / period).floor() > ((before - at) / period).floor())
                && self.panels[pi].call.is_none()
            {
                self.start(st, pi, jack);
            }
            if self.panels[pi].call.is_some() {
                self.advance(st, pi, dt);
            }
        }
    }

    fn start(&mut self, st: &mut GameState, pi: usize, caller: usize) {
        let p = &mut self.panels[pi];
        let n = p.calls;
        p.calls += 1;
        let (far, via, cords) = p.route(caller);
        let name = match p.jacks[caller].end {
            End::Line(id) => self.parties.iter().find(|x| x.id == id).map(|x| x.name).unwrap_or(id),
            _ => "YOU",
        };
        let party = match p.jacks[caller].end {
            End::Line(id) => id,
            _ => "you",
        };
        p.line("", format!("{name} rings."), Tone::System);
        p.call = Some(Call { n, caller, far, via, step: Step::Ring, t: 0.0, keys: None, challenge: None, reply: None, connected: false, search: None, cords, told: 0 });
        let id = p.id;
        self.events.push(LineEvent::Ring { panel: id, party });
        let _ = st;
    }

    /// The handshake for the call on panel `pi`, as it stands (relay turns included).
    fn handshake(&self, st: &GameState, pi: usize) -> Option<Keys> {
        let p = &self.panels[pi];
        let c = p.call.as_ref()?;
        let Sec::Scrambled { group, words, sealed } = self.sec_of(p, p.jacks[c.caller].end)? else { return None };
        let base = format!("lines/{}/{}", p.id, c.n);
        let a = secret(st, &group, &format!("{base}/caller"));
        let a_pub = group.public(a);
        let far_sec = c.far.and_then(|f| self.sec_of(p, p.jacks[f].end));
        let b = far_sec.filter(Sec::scrambled).map(|_| secret(st, &group, &format!("{base}/far")));
        let b_pub = b.map(|b| group.public(b));
        match c.via {
            Via::Direct => {
                let b = b?;
                let k = group.shared(a, group.public(b));
                Some(Keys { group, words, sealed, a_pub, b_pub, mid: None, k_near: k, k_far: Some(k) })
            }
            Via::Relay(_) | Via::Monitor => {
                let who = if c.via == Via::Monitor { "monitor" } else { "relay" };
                let m1 = secret(st, &group, &format!("{base}/{who}/near"));
                let k_near = group.shared(m1, a_pub);
                let near_word = sas(&group, k_near, words);
                let turns = if c.via == Via::Monitor { 0 } else { p.relay.turns };
                // a sealed box committed before it saw anything: a matching word would be luck,
                // and the game does not hand out that luck
                let mut extra = 0u32;
                loop {
                    let m2 = secret(st, &group, &format!("{base}/{who}/far/{}", turns + extra));
                    let k_far = b_pub.map(|bp| group.shared(m2, bp));
                    if sealed && words > 0 && k_far.is_some_and(|k| sas(&group, k, words) == near_word) {
                        extra += 1000;
                        continue;
                    }
                    let mid = Some((group.public(m1), b_pub.map(|_| group.public(m2))));
                    return Some(Keys { group, words, sealed, a_pub, b_pub, mid, k_near, k_far });
                }
            }
        }
    }

    fn advance(&mut self, st: &mut GameState, pi: usize, dt: f32) {
        let Some(mut c) = self.panels[pi].call.take() else { return };
        c.t += dt;
        let panel_id = self.panels[pi].id;
        // a pulled cord kills the line
        if c.step != Step::Done && !c.cords.iter().all(|&k| self.panels[pi].cord_alive(k)) {
            self.panels[pi].line("", "The line goes dead.", Tone::System);
            c.step = Step::Done;
            c.t = 0.0;
        }
        let caller_end = self.panels[pi].jacks[c.caller].end;
        let caller_name = self.end_name(caller_end);
        let caller_party = match caller_end {
            End::Line(id) => self.party(id).cloned(),
            _ => None,
        };
        let far_end = c.far.map(|f| self.panels[pi].jacks[f].end);
        let far_name = far_end.map(|e| self.end_name(e)).unwrap_or_default();
        let far_party = match far_end {
            Some(End::Line(id)) => self.party(id).cloned(),
            _ => None,
        };
        let caller_sec = self.sec_of(&self.panels[pi], caller_end).unwrap_or(Sec::Clear);
        let far_sec = far_end.and_then(|e| self.sec_of(&self.panels[pi], e));
        let scrambled = caller_sec.scrambled();
        // how the panel hears speech on this call
        let tone = if !scrambled {
            Tone::Plain
        } else if matches!(c.via, Via::Relay(_)) {
            Tone::Relay
        } else {
            Tone::Noise
        };
        match c.step {
            Step::Ring if c.t >= RING => {
                let dead_trunk = matches!(far_party.as_ref().map(|x| &x.role), Some(Role::Trunk { live }) if !st.flag(live));
                if c.far.is_none() {
                    let what = match c.via {
                        Via::Relay(_) => "The relay's other side is not plugged into anything.",
                        Via::Monitor => "The monitor's other side is not plugged into anything.",
                        Via::Direct => "Nothing on the other end of the cord.",
                    };
                    self.panels[pi].line("", what, Tone::System);
                    c.step = Step::Done;
                } else if dead_trunk {
                    self.panels[pi].line("", "The trunk is dead. It runs out over the tower, and the tower is theirs.", Tone::System);
                    c.step = Step::Done;
                } else if c.via == Via::Direct && scrambled != far_sec.is_some_and(|s| s.scrambled()) {
                    let who = if scrambled { caller_name.clone() } else { far_name.clone() };
                    self.panels[pi].line(&who, "NO CARRIER. (one end scrambles, the other does not)", Tone::Bad);
                    self.events.push(LineEvent::NoCarrier { panel: panel_id });
                    c.step = Step::Done;
                } else if scrambled {
                    c.step = Step::Shake;
                } else {
                    c.step = Step::Ask;
                }
                c.t = 0.0;
            }
            Step::Shake if c.t >= SHAKE => {
                self.panels[pi].call = Some(c.clone());
                c.keys = self.handshake(st, pi);
                let Some(k) = c.keys.clone() else {
                    c.step = Step::Done;
                    self.panels[pi].call = Some(c);
                    return;
                };
                let grade = if k.group.bits() < 40 { format!("EXPORT GRADE, {} bits", k.group.bits() + 1) } else { "STRONG".to_string() };
                let seal = if k.sealed { ", sealed" } else { "" };
                let p = &mut self.panels[pi];
                match k.mid {
                    None => p.line("", format!("Key exchange ({grade}{seal}): {} sends {}, {} sends {}.", caller_name, short(k.a_pub), far_name, short(k.b_pub.unwrap_or(0))), Tone::Noise),
                    Some((m_near, m_far)) => {
                        let box_name = if c.via == Via::Monitor { "the monitor" } else { "your relay" };
                        p.line("", format!("Key exchange ({grade}{seal}): {} sends {}, {} answers {}.", caller_name, short(k.a_pub), box_name, short(m_near)), Tone::Noise);
                        if let (Some(b), Some(m)) = (k.b_pub, m_far) {
                            p.line("", format!("Key exchange ({grade}{seal}): {} sends {}, {} answers {}.", box_name, short(m), far_name, short(b)), Tone::Noise);
                        }
                    }
                }
                c.step = match c.via {
                    Via::Relay(_) => {
                        if k.sealed {
                            p.line("", "Sealed handshake: each end committed to its number before it saw the other's, and so did your relay. The knob is locked.", Tone::System);
                        } else {
                            p.line("", "Your relay is holding the call.", Tone::System);
                        }
                        Step::Hold
                    }
                    _ if k.words > 0 => Step::Words,
                    _ => Step::Ask,
                };
                c.t = 0.0;
            }
            Step::Hold => {
                if c.connected {
                    let words = c.keys.as_ref().is_some_and(|k| k.words > 0);
                    self.panels[pi].line("", "You connect the relay.", Tone::System);
                    c.step = if words { Step::Words } else { Step::Ask };
                    c.t = 0.0;
                } else if c.t >= HOLD_MAX {
                    self.panels[pi].line("", "Both ends give up on the silence and hang up.", Tone::System);
                    c.step = Step::Done;
                    c.t = 0.0;
                }
            }
            Step::Words if c.t >= WORDS => {
                let k = c.keys.clone().expect("words come after a handshake");
                let near = k.word_near();
                let far = k.word_far();
                let p = &mut self.panels[pi];
                if c.via == Via::Direct {
                    // read out over the scrambled line: a cord that only listens hears that they agree, not what
                    let agree = if k.matched() { "they agree" } else { "they differ" };
                    p.line("", format!("{caller_name} and {far_name} read each other the words on their scramblers: {agree}."), Tone::System);
                } else {
                    p.line(&caller_name, format!("reads: {near}"), Tone::Relay);
                    match &far {
                        Some(w) => p.line(&far_name, format!("reads: {w}"), Tone::Relay),
                        None => p.line(&far_name, "reads nothing back.", Tone::Relay),
                    }
                }
                let trunk = matches!(far_party.as_ref().map(|x| &x.role), Some(Role::Trunk { .. }));
                if k.matched() {
                    c.step = if trunk { Step::Escrow } else { Step::Ask };
                } else if trunk {
                    p.line("MARA", "Those aren't my words. Somebody is sitting on this line. I'm hanging up.", Tone::Bad);
                    c.step = Step::Done;
                } else {
                    let cost = caller_party.as_ref().map(|x| x.tamper).unwrap_or(WRONG);
                    p.line(&caller_name, "The words differ. Line dropped; tampering reported.", Tone::Bad);
                    if st.settings.dragnet {
                        st.trace = (st.trace + crate::campaign::penalty(st, cost)).min(100.0);
                    }
                    self.events.push(LineEvent::Tamper { panel: panel_id });
                    c.step = Step::Done;
                }
                c.t = 0.0;
            }
            Step::Ask if c.t >= SAY * 0.5 => {
                let role = caller_party.as_ref().map(|x| x.role.clone());
                let text = match &role {
                    Some(Role::Depot { open, .. }) => open.to_string(),
                    Some(Role::Post { table, .. }) => {
                        let ch = table[c.n as usize % table.len()].0;
                        c.challenge = Some(ch);
                        format!("Orders? Challenge: {ch}.")
                    }
                    Some(Role::Camera { .. }) => "Orders?".to_string(),
                    Some(Role::Desk { code, .. }) => format!("TOWER, MAINTENANCE {}", code.iter().map(|d| d.to_string()).collect::<String>()),
                    _ => String::new(),
                };
                self.say(pi, &c, &caller_name, &text, tone);
                // the recorder takes whatever the line carried
                if self.panels[pi].recording
                    && let Some(k) = &c.keys
                    && let Some(b) = k.b_pub
                    && c.via == Via::Direct
                {
                    let p = &mut self.panels[pi];
                    p.recording = false;
                    p.line("", format!("Recorded: call {}, both numbers and {} bytes of noise.", c.n + 1, text.len().min(32)), Tone::Good);
                    let _ = b;
                    st.set(exchange::TAPE, (c.n + 1) as f64);
                    self.events.push(LineEvent::Taped { panel: panel_id });
                }
                // what the far end does with it
                if let (Some(Role::Depot { .. }), Some(Role::Gate { flag, .. })) = (&role, far_party.as_ref().map(|x| &x.role)) {
                    self.panels[pi].line(&far_name, "opens.", Tone::Good);
                    if let Some(f) = flag
                        && !st.flag(f)
                    {
                        st.set(f, true);
                        self.events.push(LineEvent::Solved { panel: panel_id, flag: f });
                    }
                }
                c.step = if matches!(role, Some(Role::Depot { .. })) { Step::Shut } else { Step::Answer };
                c.t = 0.0;
            }
            Step::Shut if c.t >= SHUT_AFTER => {
                if let Some(Role::Depot { shut, .. }) = caller_party.as_ref().map(|x| &x.role) {
                    self.say(pi, &c, &caller_name, shut, tone);
                    if let Some(Role::Gate { flag, .. }) = far_party.as_ref().map(|x| &x.role) {
                        let text = if flag.is_some_and(|f| st.flag(f)) { "stays open: its latch has caught." } else { "shuts." };
                        self.panels[pi].line(&far_name, text, Tone::Plain);
                    }
                }
                c.step = Step::Done;
                c.t = 0.0;
            }
            Step::Answer => {
                let to_you = far_end == Some(End::Handset);
                if to_you {
                    if c.t >= ANSWER_MAX {
                        self.panels[pi].line(&caller_name, "No answer. Hangs up.", Tone::System);
                        c.step = Step::Done;
                        c.t = 0.0;
                    }
                } else if c.t >= SAY {
                    let ask = caller_party.as_ref().map(|x| x.role.clone());
                    let answer: Option<(Option<Word>, Word)> = match (far_party.as_ref().map(|x| &x.role), &ask) {
                        (Some(Role::Desk { post, .. }), Some(Role::Post { table, .. })) => {
                            let cs = c.challenge.and_then(|ch| table.iter().find(|(a, _)| *a == ch).map(|(_, b)| *b));
                            Some((cs, *post))
                        }
                        (Some(Role::Desk { camera, .. }), Some(Role::Camera { .. })) => Some((None, *camera)),
                        (Some(Role::Tower), Some(Role::Desk { .. })) => {
                            self.say(pi, &c, &far_name, "ACK.", tone);
                            None
                        }
                        _ => {
                            self.panels[pi].line(&far_name, "(clicks; it does not answer calls)", Tone::System);
                            None
                        }
                    };
                    match answer {
                        Some((cs, order)) => {
                            let said = match cs {
                                Some(cs) => format!("{cs}. {order}."),
                                None => format!("{order}."),
                            };
                            self.say(pi, &c, &far_name, &said, tone);
                            // a countersign heard in the clear can be used again
                            if tone != Tone::Noise
                                && let (Some(ch), Some(cs)) = (c.challenge, cs)
                            {
                                let p = &mut self.panels[pi];
                                if !p.heard.contains(&(ch, cs)) {
                                    p.heard.push((ch, cs));
                                    self.events.push(LineEvent::Heard { panel: panel_id });
                                }
                            }
                            // your relay passes on what you tell it to
                            let order = match (c.via, self.panels[pi].relay.rewrite) {
                                (Via::Relay(_), Some(w)) if w != order => {
                                    self.panels[pi].line("", format!("Your relay passes on {w} instead of {order}."), Tone::Relay);
                                    w
                                }
                                _ => order,
                            };
                            c.reply = Some((cs, order));
                            c.step = Step::Back;
                        }
                        None => c.step = Step::Done,
                    }
                    c.t = 0.0;
                }
            }
            Step::Back if c.t >= SAY * 0.6 => {
                let (cs, order) = c.reply.unwrap_or((None, "HOLD"));
                match caller_party.as_ref().map(|x| &x.role) {
                    Some(Role::Post { table, obey }) => {
                        let want = c.challenge.and_then(|ch| table.iter().find(|(a, _)| *a == ch).map(|(_, b)| *b));
                        if cs.is_none() || cs != want {
                            self.panels[pi].line(&caller_name, "Wrong countersign. Reporting this line.", Tone::Bad);
                            if st.settings.dragnet {
                                st.trace = (st.trace + crate::campaign::penalty(st, WRONG)).min(100.0);
                            }
                            self.events.push(LineEvent::Alarm { panel: panel_id });
                        } else {
                            let effect = obey.iter().find(|(w, _)| *w == order).map(|(_, e)| *e).unwrap_or(Effect::Nothing);
                            self.obey(st, pi, &c, &caller_name, order, effect, tone);
                        }
                    }
                    Some(Role::Camera { obey }) => {
                        let effect = obey.iter().find(|(w, _)| *w == order).map(|(_, e)| *e).unwrap_or(Effect::Nothing);
                        self.obey(st, pi, &c, &caller_name, order, effect, tone);
                    }
                    _ => {}
                }
                c.step = Step::Done;
                c.t = 0.0;
            }
            Step::Escrow if c.t >= SAY * 0.5 => {
                let k = c.keys.clone().expect("a handshake");
                let sk = k.group.session_key(k.k_near);
                let (key10, iv): ([u8; 10], [u8; 8]) = (sk[..10].try_into().unwrap(), sk[10..18].try_into().unwrap());
                if self.panels[pi].forged {
                    self.panels[pi].line("", "Your phone starts feeding Mara's chip random LEAFs.", Tone::System);
                    c.search = Some(LeafSearch::new(&format!("{}/{}/{}", st.world().seed_hex(), panel_id, c.n)));
                    c.step = Step::Forge;
                } else {
                    let leaf = self.escrow.honest_leaf(&self.escrow.chips[0], &key10, &iv);
                    let p = &mut self.panels[pi];
                    match self.escrow.wiretap(&leaf) {
                        Ok((unit, key)) => {
                            p.line("ESCROW", format!("LEAF read: unit {unit:08X}, session key {}…. This call is open to the Directorate.", &to_hex(&key)[..8]), Tone::Bad);
                        }
                        Err(e) => p.line("ESCROW", e, Tone::Bad),
                    }
                    p.line("MARA", "They have the key. Every one of these phones hands its key to the escrow desk. Not a word on this line.", Tone::Plain);
                    if st.settings.dragnet {
                        st.trace = (st.trace + crate::campaign::penalty(st, 15.0)).min(100.0);
                    }
                    if !st.flag(exchange::ESCROW_SEEN) {
                        st.set(exchange::ESCROW_SEEN, true);
                    }
                    self.events.push(LineEvent::Escrow { panel: panel_id });
                    c.step = Step::Done;
                }
                c.t = 0.0;
            }
            Step::Forge => {
                let k = c.keys.clone().expect("a handshake");
                let sk = k.group.session_key(k.k_near);
                let (key10, iv): ([u8; 10], [u8; 8]) = (sk[..10].try_into().unwrap(), sk[10..18].try_into().unwrap());
                let budget = (FORGE_RATE * dt).ceil().max(1.0) as u64;
                let done = c.search.as_mut().is_some_and(|s| s.step(&self.escrow, &key10, &iv, budget));
                let tries = c.search.as_ref().map(|s| s.tries).unwrap_or(0);
                if done {
                    let leaf = c.search.as_ref().and_then(|s| s.found).expect("found");
                    let p = &mut self.panels[pi];
                    p.line("", format!("Mara's chip accepts LEAF number {tries}: its 16-bit checksum happens to fit."), Tone::Good);
                    match self.escrow.wiretap(&leaf) {
                        Err(e) => p.line("ESCROW", format!("LEAF read: {e}. No key."), Tone::Good),
                        Ok((unit, _)) => p.line("ESCROW", format!("LEAF read: unit {unit:08X}, a key that opens nothing."), Tone::Good),
                    }
                    p.line("MARA", "...I hear you. And for once, nobody else does.", Tone::Good);
                    if !st.flag(exchange::FREE) {
                        st.set(exchange::FREE, true);
                        self.events.push(LineEvent::Solved { panel: panel_id, flag: exchange::FREE });
                    }
                    self.events.push(LineEvent::Forged { panel: panel_id });
                    c.step = Step::Done;
                    c.t = 0.0;
                } else if tries >= c.told + 8192 {
                    c.told = tries - tries % 8192;
                    self.panels[pi].line("", format!("Mara's chip has turned down {} LEAFs.", c.told), Tone::System);
                }
            }
            Step::Done if c.t >= HANG_UP => {
                return;
            }
            _ => {}
        }
        self.panels[pi].call = Some(c);
    }

    /// Log speech as the panel hears it: words, noise, or what your relay decrypts.
    fn say(&mut self, pi: usize, c: &Call, who: &str, text: &str, tone: Tone) {
        let p = &mut self.panels[pi];
        match (tone, &c.keys) {
            (Tone::Noise, Some(k)) => {
                let bytes = cipher(&k.group, k.k_near, c.n, text);
                p.line(who, hex_groups(&bytes), Tone::Noise);
            }
            _ => p.line(who, text, tone),
        }
    }

    /// The caller acts on the order it got, and says so down the line.
    #[allow(clippy::too_many_arguments)]
    fn obey(&mut self, st: &mut GameState, pi: usize, c: &Call, who: &str, order: Word, effect: Effect, tone: Tone) {
        let id = self.panels[pi].id;
        // what the line carries is noise to a cord that only listens; the effect shows in the street
        let heard = |ok: Tone| if tone == Tone::Noise { Tone::Noise } else { ok };
        match effect {
            Effect::Flag(f) => {
                let t = heard(Tone::Good);
                self.say(pi, c, who, &format!("{order}. Understood."), t);
                if !st.flag(f) {
                    st.set(f, true);
                    self.events.push(LineEvent::Solved { panel: id, flag: f });
                }
            }
            Effect::Alarm => {
                let t = heard(Tone::Bad);
                self.say(pi, c, who, &format!("{order}? That makes no sense. Reporting this line."), t);
                if st.settings.dragnet {
                    st.trace = (st.trace + crate::campaign::penalty(st, WRONG)).min(100.0);
                }
                self.events.push(LineEvent::Alarm { panel: id });
            }
            Effect::Nothing => {
                let t = heard(tone);
                self.say(pi, c, who, &format!("{order}. Understood."), t);
            }
        }
    }
}
