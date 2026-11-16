//! Signal boards: the puzzle language of the Wharf.
//!
//! A board is 64 lamps. A card is 64 holes; put a card in a board's reader
//! and every lamp behind a hole flips. That is XOR, and with a random card it
//! is the one-time pad (see `cpl_crypto::lamps`). Locks read a board through
//! a pad of their own that you never see, and act on the picture they find.
//! Everything the player can do is here, with no user interface: slot and
//! pull cards, flip lamps by hand, take a rubbing of a board, stack cards on
//! the light table and cut the result, pull a lever.
//!
//! The district's boards and their order live in [`wharf`].

pub mod glyph;
pub mod wharf;

use cpl_crypto::lamps::{Bits, PICTURE, seal_ok, sealed};
use serde::{Deserialize, Serialize};

pub use glyph::Glyph;

use crate::state::GameState;

/// Trace added by a lever pulled on nonsense, a broken seal, and an alarm order.
pub const BUZZ: f32 = 12.0;
pub const SEAL_BROKEN: f32 = 25.0;
pub const ALARM: f32 = 45.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CardKind {
    /// a pad someone punched and left for you
    Pad,
    /// a rubbing of a board
    Copy,
    /// cut on the light table
    Cut,
}

/// A card you carry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Card {
    pub id: u32,
    pub name: String,
    /// 16 hex digits (JSON numbers cannot hold 64 bits everywhere)
    pub bits: String,
    pub kind: CardKind,
    /// where it came from, for the tooltip
    #[serde(default)]
    pub note: String,
}

impl Card {
    pub fn value(&self) -> Bits {
        u64::from_str_radix(&self.bits, 16).unwrap_or(0)
    }
}

/// Anything that fits a reader or the light table: a card of yours, or a
/// stencil from the glyph book.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CardRef {
    Own(u32),
    Glyph(Glyph),
}

/// What a console keeps sending to its board.
#[derive(Clone, Debug, PartialEq)]
pub enum Feed {
    /// nothing: every lamp dark
    Blank,
    Fixed(Glyph),
    /// one fixed pattern that is no picture at all (so nobody can guess it)
    Pattern(Bits),
    /// a new pattern every `period` seconds, forever
    Rotating { seq: Vec<Bits>, period: f32 },
    /// `rest` most of the time; `open` for `len` seconds every `period`, starting at `at`
    Window { rest: Glyph, open: Glyph, period: f32, at: f32, len: f32 },
}

impl Feed {
    /// The picture being sent at time `t` (top seven rows).
    pub fn picture(&self, t: f32) -> Bits {
        match self {
            Feed::Blank => 0,
            Feed::Fixed(g) => g.bits(),
            Feed::Pattern(b) => *b & PICTURE,
            Feed::Rotating { seq, period } => seq[(t / period).floor() as usize % seq.len()] & PICTURE,
            Feed::Window { rest, open, period, at, len } => {
                let phase = t.rem_euclid(*period);
                if phase >= *at && phase < at + len { open.bits() } else { rest.bits() }
            }
        }
    }

    /// Seconds until the picture changes (for the countdown on a rotating board).
    pub fn until_change(&self, t: f32) -> Option<f32> {
        match self {
            Feed::Rotating { period, .. } => Some(period - t.rem_euclid(*period)),
            Feed::Window { period, at, len, .. } => {
                let phase = t.rem_euclid(*period);
                Some(if phase < *at {
                    at - phase
                } else if phase < at + len {
                    at + len - phase
                } else {
                    period - phase + at
                })
            }
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    /// sets this flag (a gate opens, a guard is reassigned)
    Flag(&'static str),
    Alarm,
    /// a valid order that changes nothing (the gate is told to stay shut)
    Nothing,
}

/// What pulling a lever did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    NoLock,
    /// the lock obeyed; the flag is now set
    Done(Glyph, &'static str),
    /// a valid order with no effect
    Idle(Glyph),
    Alarm(Glyph),
    /// the lock decrypted the board and found no order it knows
    NotASignal,
    SealBroken,
}

pub struct Board {
    pub id: &'static str,
    pub site: &'static str,
    pub title: &'static str,
    /// name of the console that feeds it (boards on one console share its pad)
    pub console: &'static str,
    /// the lock's (and console's) pad; zero on a plaintext board
    pub pad: Bits,
    pub seal: Option<[u8; 16]>,
    pub slots: usize,
    /// lamps can be flipped by hand
    pub keys: bool,
    /// a training monitor shows what the lock sees
    pub monitor: bool,
    pub feed: Feed,
    pub accepts: Vec<(Glyph, Effect)>,
    /// set when the lamps show the console's picture in clear
    pub reveal: Option<&'static str>,
    /// pictures posted beside the board
    pub legend: Vec<Glyph>,
    // --- what is on it right now ---
    /// what the console sends: (sealed) picture xor pad
    pub base: Bits,
    /// the picture itself
    pub picture: Bits,
    /// lamps the player flipped by hand since the last broadcast
    pub hand: Bits,
    pub cards: Vec<CardRef>,
    pub pulls: u32,
}

impl Board {
    pub fn has_lock(&self) -> bool {
        !self.accepts.is_empty()
    }

    pub fn encrypted(&self) -> bool {
        self.pad != 0
    }

    /// What the console puts on the lamps for a picture.
    pub fn encode(&self, picture: Bits) -> Bits {
        let m = match &self.seal {
            Some(k) => sealed(k, picture),
            None => picture & PICTURE,
        };
        m ^ self.pad
    }

    /// The picture the console is sending, if it is one of the glyphs.
    pub fn message(&self) -> Option<Glyph> {
        Glyph::from_bits(self.picture)
    }

    /// The flag this board sets when it is solved (by its lock or by being read).
    pub fn goal(&self) -> Option<&'static str> {
        self.reveal.or_else(|| self.accepts.iter().find_map(|(_, e)| if let Effect::Flag(f) = e { Some(*f) } else { None }))
    }
}

/// A locker with a dial.
pub struct Locker {
    pub id: &'static str,
    pub site: &'static str,
    pub title: &'static str,
    pub code: Vec<u8>,
    /// the digits may be entered in any order
    pub any_order: bool,
    pub flag: &'static str,
    pub tries: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SiteKind {
    Boards,
    Dive(&'static str),
}

/// A place on the district map.
pub struct Site {
    pub id: &'static str,
    pub name: &'static str,
    /// one line for the map
    pub blurb: &'static str,
    pub spawn: &'static str,
    /// flags that must be set before you can get there
    pub needs: &'static [&'static str],
    /// what the map says is in the way while a need is unmet
    pub blocked_by: &'static str,
    pub done: &'static str,
    pub kind: SiteKind,
    /// Grandpa's notes: optional hints, vaguest first
    pub notes: &'static [&'static str],
}

/// Things the rest of the game reacts to (sounds, toasts, camera).
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// a lock obeyed or a board was read: this flag was just set
    Solved { board: &'static str, flag: &'static str },
    Revealed { board: &'static str, glyph: Glyph },
    Alarm { board: &'static str },
    Buzz { board: &'static str },
    SealBroken { board: &'static str },
    Card { id: u32 },
    Learned { glyph: Glyph },
    Locker { id: &'static str, open: bool },
    /// a console changed what it sends
    Broadcast { board: &'static str },
}

/// Someone the district moves around on its own.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub id: &'static str,
    pub x: f32,
    pub y: f32,
    /// direction of travel (unit vector, or zero when standing)
    pub dx: f32,
    pub dy: f32,
    /// seen from the air rather than on foot (drawn above everything)
    pub flying: bool,
    pub visible: bool,
}

/// A moving thing that raises your trace when you are near it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Watcher {
    pub id: &'static str,
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub gain: f32,
}

/// A closed loop of waypoints walked at constant speed, with a pause at each.
#[derive(Clone, Debug)]
pub struct Walk {
    /// x, y, seconds to stand there
    pub pts: Vec<(f32, f32, f32)>,
    pub speed: f32,
}

impl Walk {
    pub fn period(&self) -> f32 {
        let n = self.pts.len();
        (0..n).map(|i| self.leg(i) / self.speed + self.pts[i].2).sum()
    }

    fn leg(&self, i: usize) -> f32 {
        let (a, b) = (self.pts[i], self.pts[(i + 1) % self.pts.len()]);
        ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt()
    }

    /// Position and heading at time `t`.
    pub fn at(&self, t: f32) -> (f32, f32, f32, f32) {
        let n = self.pts.len();
        if n == 1 {
            return (self.pts[0].0, self.pts[0].1, 0.0, 0.0);
        }
        let mut left = t.rem_euclid(self.period().max(0.001));
        for i in 0..n {
            let a = self.pts[i];
            if left < a.2 {
                return (a.0, a.1, 0.0, 0.0);
            }
            left -= a.2;
            let len = self.leg(i);
            let dur = len / self.speed;
            if left < dur {
                let b = self.pts[(i + 1) % n];
                let k = left / dur;
                let (dx, dy) = ((b.0 - a.0) / len.max(0.001), (b.1 - a.1) / len.max(0.001));
                return (a.0 + (b.0 - a.0) * k, a.1 + (b.1 - a.1) * k, dx, dy);
            }
            left -= dur;
        }
        (self.pts[0].0, self.pts[0].1, 0.0, 0.0)
    }
}

/// The district's signals, live.
pub struct Signals {
    pub boards: Vec<Board>,
    pub lockers: Vec<Locker>,
    pub sites: Vec<Site>,
    /// seconds since the district was loaded (drives rotating boards and patrols)
    pub t: f32,
    pub events: Vec<Event>,
    /// when the guard was reassigned (he walks to his new post from where he stood)
    pub(crate) guard_moved_at: Option<(f32, f32, f32)>,
}

impl Signals {
    pub fn board(&self, id: &str) -> Option<&Board> {
        self.boards.iter().find(|b| b.id == id)
    }

    fn board_mut(&mut self, id: &str) -> Option<&mut Board> {
        self.boards.iter_mut().find(|b| b.id == id)
    }

    pub fn locker(&self, id: &str) -> Option<&Locker> {
        self.lockers.iter().find(|l| l.id == id)
    }

    pub fn site(&self, id: &str) -> Option<&Site> {
        self.sites.iter().find(|s| s.id == id)
    }

    pub fn site_open(&self, st: &GameState, id: &str) -> bool {
        self.site(id).is_some_and(|s| s.needs.iter().all(|f| st.flag(f)))
    }

    pub fn site_done(&self, st: &GameState, id: &str) -> bool {
        self.site(id).is_some_and(|s| st.flag(s.done))
    }

    /// The first site that is open and not done (where the map puts its cursor).
    pub fn next_site(&self, st: &GameState) -> Option<&Site> {
        self.sites.iter().find(|s| self.site_open(st, s.id) && !st.flag(s.done))
    }

    pub fn bits_of(&self, st: &GameState, r: CardRef) -> Bits {
        match r {
            CardRef::Glyph(g) => g.bits(),
            CardRef::Own(id) => st.data.cards.iter().find(|c| c.id == id).map(|c| c.value()).unwrap_or(0),
        }
    }

    /// The lamps as anyone can see them.
    pub fn shown(&self, st: &GameState, board: &str) -> Bits {
        let Some(b) = self.board(board) else { return 0 };
        b.cards.iter().fold(b.base ^ b.hand, |acc, c| acc ^ self.bits_of(st, *c))
    }

    /// What the lock behind the board reads (the training monitor shows this).
    pub fn lock_view(&self, st: &GameState, board: &str) -> Bits {
        self.board(board).map(|b| self.shown(st, board) ^ b.pad).unwrap_or(0)
    }

    pub fn xor(&self, st: &GameState, refs: &[CardRef]) -> Bits {
        refs.iter().fold(0, |acc, r| acc ^ self.bits_of(st, *r))
    }

    // --- what the player can do --------------------------------------------------

    /// Put a card in a free reader. A second copy of a card already in pulls that one out instead.
    pub fn slot(&mut self, st: &mut GameState, board: &str, card: CardRef) -> bool {
        let Some(b) = self.board_mut(board) else { return false };
        if let Some(i) = b.cards.iter().position(|c| *c == card) {
            b.cards.remove(i);
        } else if b.cards.len() < b.slots {
            b.cards.push(card);
        } else {
            return false;
        }
        self.after_change(st, board);
        true
    }

    pub fn unslot(&mut self, st: &mut GameState, board: &str, index: usize) -> Option<CardRef> {
        let b = self.board_mut(board)?;
        if index >= b.cards.len() {
            return None;
        }
        let c = b.cards.remove(index);
        self.after_change(st, board);
        Some(c)
    }

    /// Flip one lamp by hand (only where the casing is open).
    pub fn toggle(&mut self, st: &mut GameState, board: &str, x: usize, y: usize) -> bool {
        let Some(b) = self.board_mut(board) else { return false };
        if !b.keys || x > 7 || y > 7 {
            return false;
        }
        b.hand ^= 1 << (y * 8 + x);
        self.after_change(st, board);
        true
    }

    /// Undo every hand flip and pull every card.
    pub fn clear(&mut self, st: &mut GameState, board: &str) {
        if let Some(b) = self.board_mut(board) {
            b.hand = 0;
            b.cards.clear();
        }
        self.after_change(st, board);
    }

    /// Take a rubbing of the lamps as they are now.
    pub fn copy(&mut self, st: &mut GameState, board: &str) -> Option<u32> {
        if !st.flag(wharf::KIT) {
            return None;
        }
        let bits = self.shown(st, board);
        let title = self.board(board)?.title;
        let n = st.data.cards.iter().filter(|c| c.kind == CardKind::Copy).count() + 1;
        Some(self.add_card(st, format!("R{n}"), bits, CardKind::Copy, format!("rubbing of {title}")))
    }

    /// Stack cards on the light table and cut what shows through.
    pub fn cut(&mut self, st: &mut GameState, refs: &[CardRef]) -> Option<u32> {
        if !st.flag(wharf::KIT) || refs.is_empty() {
            return None;
        }
        let bits = self.xor(st, refs);
        let n = st.data.cards.iter().filter(|c| c.kind == CardKind::Cut).count() + 1;
        let names: Vec<String> = refs.iter().map(|r| self.name_of(st, *r)).collect();
        Some(self.add_card(st, format!("X{n}"), bits, CardKind::Cut, names.join(" + ")))
    }

    pub fn name_of(&self, st: &GameState, r: CardRef) -> String {
        match r {
            CardRef::Glyph(g) => g.label().to_string(),
            CardRef::Own(id) => st.data.cards.iter().find(|c| c.id == id).map(|c| c.name.clone()).unwrap_or_default(),
        }
    }

    pub fn add_card(&mut self, st: &mut GameState, name: String, bits: Bits, kind: CardKind, note: String) -> u32 {
        let id = st.data.cards.iter().map(|c| c.id).max().unwrap_or(0) + 1;
        st.data.cards.push(Card { id, name, bits: format!("{bits:016x}"), kind, note });
        self.events.push(Event::Card { id });
        id
    }

    /// Throw a card away (pads included: they are only paper).
    pub fn discard(&mut self, st: &mut GameState, id: u32) {
        st.data.cards.retain(|c| c.id != id);
        let ids: Vec<&'static str> = self.boards.iter().map(|b| b.id).collect();
        for b in &mut self.boards {
            b.cards.retain(|c| *c != CardRef::Own(id));
        }
        for b in ids {
            self.after_change(st, b);
        }
    }

    pub fn learn(&mut self, st: &mut GameState, glyphs: &[Glyph]) {
        for g in glyphs {
            if !st.data.glyphs.iter().any(|k| k == g.id()) {
                st.data.glyphs.push(g.id().to_string());
                self.events.push(Event::Learned { glyph: *g });
            }
        }
    }

    pub fn known_glyphs(&self, st: &GameState) -> Vec<Glyph> {
        Glyph::ALL.iter().copied().filter(|g| st.data.glyphs.iter().any(|k| k == g.id())).collect()
    }

    /// Looking at a board teaches you the pictures posted beside it.
    pub fn visit(&mut self, st: &mut GameState, board: &str) {
        if let Some(b) = self.board(board) {
            let legend = b.legend.clone();
            self.learn(st, &legend);
        }
    }

    /// Pull the lever: the lock decrypts what it sees and acts on it.
    pub fn pull(&mut self, st: &mut GameState, board: &str) -> Outcome {
        let view = self.lock_view(st, board);
        let Some(b) = self.board_mut(board) else { return Outcome::NoLock };
        if !b.has_lock() {
            return Outcome::NoLock;
        }
        b.pulls += 1;
        let id = b.id;
        if let Some(k) = &b.seal
            && !seal_ok(k, view)
        {
            // a broken seal trips the lock and the console repaints the board
            b.hand = 0;
            st.trace = (st.trace + SEAL_BROKEN).min(100.0);
            self.events.push(Event::SealBroken { board: id });
            self.after_change(st, id);
            return Outcome::SealBroken;
        }
        let picture = if b.seal.is_some() { view & PICTURE } else { view };
        let hit = Glyph::from_bits(picture).and_then(|g| b.accepts.iter().find(|(a, _)| *a == g).copied());
        match hit {
            None => {
                st.trace = (st.trace + BUZZ).min(100.0);
                self.events.push(Event::Buzz { board: id });
                Outcome::NotASignal
            }
            Some((g, Effect::Nothing)) => Outcome::Idle(g),
            Some((g, Effect::Alarm)) => {
                st.trace = (st.trace + ALARM).min(100.0);
                self.events.push(Event::Alarm { board: id });
                Outcome::Alarm(g)
            }
            Some((g, Effect::Flag(f))) => {
                if !st.flag(f) {
                    st.set(f, true);
                    self.events.push(Event::Solved { board: id, flag: f });
                    self.reward(st, f);
                }
                Outcome::Done(g, f)
            }
        }
    }

    /// Try a combination on a locker's dial.
    pub fn dial(&mut self, st: &mut GameState, locker: &str, digits: &[u8]) -> bool {
        let Some(l) = self.lockers.iter_mut().find(|l| l.id == locker) else { return false };
        let (id, flag) = (l.id, l.flag);
        if st.flag(flag) {
            return true;
        }
        l.tries += 1;
        let mut ok = digits == l.code.as_slice();
        if !ok && l.any_order && digits.len() == l.code.len() {
            let (mut a, mut b) = (digits.to_vec(), l.code.clone());
            a.sort_unstable();
            b.sort_unstable();
            ok = a == b;
        }
        if ok {
            st.set(flag, true);
            self.reward(st, flag);
        }
        self.events.push(Event::Locker { id, open: ok });
        ok
    }

    // --- time ------------------------------------------------------------------------

    /// Advance consoles, patrols and drones.
    pub fn tick(&mut self, st: &mut GameState, dt: f32) {
        self.t += dt;
        let t = self.t;
        let mut changed: Vec<&'static str> = Vec::new();
        for b in &mut self.boards {
            let m = b.feed.picture(t);
            if m != b.picture {
                b.picture = m;
                b.base = b.encode(m);
                // a fresh broadcast repaints the lamps; cards in the reader stay
                b.hand = 0;
                changed.push(b.id);
            }
        }
        for id in changed {
            self.events.push(Event::Broadcast { board: id });
            self.after_change(st, id);
        }
    }

    fn after_change(&mut self, st: &mut GameState, board: &str) {
        let shown = self.shown(st, board);
        let Some(b) = self.board(board) else { return };
        let (id, reveal, msg) = (b.id, b.reveal, b.message());
        if let (Some(flag), Some(g)) = (reveal, msg)
            && shown == g.bits()
            && !st.flag(flag)
        {
            st.set(flag, true);
            self.events.push(Event::Revealed { board: id, glyph: g });
            self.events.push(Event::Solved { board: id, flag });
            self.reward(st, flag);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_walk_loops_and_pauses() {
        let w = Walk { pts: vec![(0.0, 0.0, 1.0), (10.0, 0.0, 0.0), (10.0, 10.0, 2.0)], speed: 5.0 };
        // 1 s wait, 2 s east, 2 s south, 2 s wait, then the diagonal home
        let diag = (200.0f32).sqrt() / 5.0;
        assert!((w.period() - (7.0 + diag)).abs() < 1e-4);
        assert_eq!(w.at(0.5), (0.0, 0.0, 0.0, 0.0));
        let (x, y, dx, dy) = w.at(2.0);
        assert!((x - 5.0).abs() < 1e-4 && y == 0.0 && dx == 1.0 && dy == 0.0);
        let (x, y, ..) = w.at(6.0);
        assert_eq!((x, y), (10.0, 10.0));
        let again = w.at(2.0 + w.period());
        assert!((again.0 - 5.0).abs() < 1e-3);
    }

    #[test]
    fn feeds_follow_the_clock() {
        let f = Feed::Window { rest: Glyph::Closed, open: Glyph::Open, period: 30.0, at: 10.0, len: 5.0 };
        assert_eq!(f.picture(0.0), Glyph::Closed.bits());
        assert_eq!(f.picture(12.0), Glyph::Open.bits());
        assert_eq!(f.picture(15.5), Glyph::Closed.bits());
        assert_eq!(f.picture(41.0), Glyph::Open.bits());
        assert!((f.until_change(3.0).unwrap() - 7.0).abs() < 1e-4);
        assert!((f.until_change(12.0).unwrap() - 3.0).abs() < 1e-4);
        assert!((f.until_change(20.0).unwrap() - 20.0).abs() < 1e-4);
        let r = Feed::Rotating { seq: vec![Glyph::Moon.bits(), Glyph::Star.bits()], period: 20.0 };
        assert_eq!(r.picture(19.0), Glyph::Moon.bits());
        assert_eq!(r.picture(21.0), Glyph::Star.bits());
        assert_eq!(r.picture(41.0), Glyph::Moon.bits());
    }
}
