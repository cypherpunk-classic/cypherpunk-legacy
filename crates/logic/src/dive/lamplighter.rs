//! THE LAMPLIGHTER: the Directorate's sweeper drone comes back for the mast.
//!
//! A boss fight you win by cryptanalysis. Every beat its floodlight takes
//! three of the four lanes and leaves one dark; which one is two bits of a
//! keystream. Dodge on reflex and you survive; to hurt it you must *call*
//! the dark lane before the light shows, and a call is a prediction. The
//! keystream is a linear-feedback shift register, so it can be predicted:
//!
//! * **I · The walk.** One register of length 5. Record ten bits and find
//!   its taps: the taps that fit every bit you saw are the only ones there are.
//! * **II · Two lamps.** Two registers XORed. Still linear, only longer:
//!   Berlekamp–Massey on enough of the log finds a register of length 9,
//!   and *enough* is 2L = 18 bits; feed it fewer and its guess breaks.
//! * **III · The reboot.** Knocked down, it reboots with the seed it used
//!   for phase II, at some point along the same stream. A reused seed is a
//!   reused keystream: line the new beats up against your phase-II log and
//!   you know every beat that follows.
//!
//! Trace out and the phase starts over with a fresh register, but what you
//! worked out (the taps, the window, the log) stays with you. Knowledge is
//! the one thing you keep.

use cpl_crypto::drbg::Drbg;
use cpl_crypto::lfsr::{Lfsr, berlekamp_massey, mismatches, predict, primitive_taps};
use egui::{Align2, Pos2, Rect, Stroke, Vec2};

use super::{DEEP, Dive, Fx, Input, NEON_CYAN, NEON_GOLD, NEON_GREEN, NEON_PINK, NEON_RED, NEON_VIOLET, Status, View, glow_dot, glow_line, glow_ring, grid, sparks};
use crate::ui::theme::{mono, retro};

pub const DIVE_LAMPLIGHTER: &str = "w.dive.lamplighter";

const FIELD: (f32, f32) = (26.0, 15.0);
pub const LANES: usize = 4;
const LANE_W: f32 = 3.0;
const PLAYER_Y: f32 = -5.0;
const BOSS: (f32, f32) = (-5.0, 5.4);
const SPEED: f32 = 18.0;
const HIT: f32 = 34.0;
const DECAY: f32 = 3.0;
const DODGE_COST: f32 = 40.0;
const IFRAMES: f32 = 0.42;
const REGEN: f32 = 28.0;
/// bits of the log the window may take
const MAX_WINDOW: usize = 40;

struct PhaseCfg {
    name: &'static str,
    hp: u32,
    gap: f32,
    tell: f32,
    strike: f32,
}

const PHASES: [PhaseCfg; 3] = [
    PhaseCfg { name: "THE WALK", hp: 4, gap: 0.75, tell: 0.95, strike: 0.3 },
    PhaseCfg { name: "TWO LAMPS", hp: 5, gap: 0.65, tell: 0.75, strike: 0.3 },
    PhaseCfg { name: "THE REBOOT", hp: 6, gap: 0.55, tell: 0.5, strike: 0.28 },
];

pub fn lane_x(i: usize) -> f32 {
    -11.0 + LANE_W / 2.0 + LANE_W * i as f32
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    Intro(f32),
    Fight,
    /// a phase just fell
    Down(f32),
    Traced(f32),
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Beat {
    Gap,
    Tell,
    Strike,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Note {
    Reflected,
    Hit,
    Miscall,
    Late,
    Dodged,
}

/// The keystream for the phase being fought.
#[derive(Clone)]
enum Gen {
    One(Lfsr),
    Two(Lfsr, Lfsr),
}

impl Gen {
    fn bit(&mut self) -> u8 {
        match self {
            Gen::One(a) => a.next_bit(),
            Gen::Two(a, b) => a.next_bit() ^ b.next_bit(),
        }
    }
}

pub struct Lamplighter {
    rng: Drbg,
    phase: usize,
    mode: Mode,
    beat: Beat,
    bt: f32,
    t: f32,
    stream: Gen,
    /// the secret taps of phase I and the two registers of phase II
    taps1: Vec<u8>,
    taps2: (Vec<u8>, Vec<u8>),
    /// where phase II's winning run started (phase III reuses it)
    p2_start: Option<Gen>,
    /// phase III: beats it skipped along the reused stream
    k3: usize,
    safe: usize,
    /// bits seen this phase, this life (two per beat)
    observed: Vec<u8>,
    /// everything seen in phase II's winning run
    p2_log: Vec<u8>,
    /// the register the player found in phase II (taps)
    p2_poly: Vec<u8>,
    hp: u32,
    lane: usize,
    x: f32,
    stamina: f32,
    iframes: f32,
    call: Option<usize>,
    trace: f32,
    // the player's tools
    taps: [u8; 5],
    window: usize,
    offset: usize,
    // the record
    pub hits: u32,
    pub deaths: u32,
    merciful: bool,
    note: Option<(Note, f32)>,
    spark: Option<(Pos2Field, f32)>,
    bot_cool: f32,
}

#[derive(Clone, Copy, Debug)]
struct Pos2Field(f32, f32);

fn random_state(rng: &mut Drbg, l: usize) -> Vec<u8> {
    loop {
        let s: Vec<u8> = (0..l).map(|_| (rng.int(2)) as u8).collect();
        if s.contains(&1) {
            return s;
        }
    }
}

impl Lamplighter {
    /// `round` makes a fresh fight for each rematch.
    pub fn new(seed: &[u8], round: u32) -> Self {
        let mut rng = Drbg::new(seed, &format!("dive/lamplighter/{round}"));
        let taps1 = rng.pick(&primitive_taps(5)).clone();
        let taps2 = (rng.pick(&primitive_taps(4)).clone(), rng.pick(&primitive_taps(5)).clone());
        let stream = Gen::One(Lfsr::new(taps1.clone(), random_state(&mut rng, 5)));
        let mut d = Self {
            rng,
            phase: 0,
            mode: Mode::Intro(0.0),
            beat: Beat::Gap,
            bt: 0.0,
            t: 0.0,
            stream,
            taps1,
            taps2,
            p2_start: None,
            k3: 0,
            safe: 0,
            observed: Vec::new(),
            p2_log: Vec::new(),
            p2_poly: Vec::new(),
            hp: PHASES[0].hp,
            lane: 1,
            x: lane_x(1),
            stamina: 100.0,
            iframes: 0.0,
            call: None,
            trace: 0.0,
            taps: [0; 5],
            window: 8,
            offset: 0,
            hits: 0,
            deaths: 0,
            merciful: false,
            note: None,
            spark: None,
            bot_cool: 0.0,
        };
        d.start_beat();
        d
    }

    fn cfg(&self) -> &'static PhaseCfg {
        &PHASES[self.phase]
    }

    /// A fresh life for the current phase: a new register state, the same taps.
    fn restart_phase(&mut self) {
        self.stream = match self.phase {
            0 => Gen::One(Lfsr::new(self.taps1.clone(), random_state(&mut self.rng, 5))),
            1 => Gen::Two(Lfsr::new(self.taps2.0.clone(), random_state(&mut self.rng, 4)), Lfsr::new(self.taps2.1.clone(), random_state(&mut self.rng, 5))),
            _ => {
                // the reused seed, at a new point along its stream
                let mut g = self.p2_start.clone().expect("phase II was won");
                let beats = self.p2_log.len() / 2;
                self.k3 = if beats >= 8 { 2 + self.rng.int(beats - 6) } else { 1 };
                for _ in 0..2 * self.k3 {
                    g.bit();
                }
                g
            }
        };
        if self.phase == 1 {
            self.p2_start = Some(self.stream.clone());
        }
        self.observed.clear();
        self.hp = self.cfg().hp;
        self.trace = 0.0;
        self.call = None;
        self.beat = Beat::Gap;
        self.bt = 0.0;
        self.start_beat();
    }

    fn start_beat(&mut self) {
        let b0 = self.stream.bit();
        let b1 = self.stream.bit();
        self.safe = (b0 * 2 + b1) as usize;
    }

    fn player_lane(&self) -> usize {
        (((self.x - lane_x(0)) / LANE_W).round().clamp(0.0, (LANES - 1) as f32)) as usize
    }

    // --- the player's knowledge ------------------------------------------------------------

    /// Phase II's register as the player has it: Berlekamp–Massey on the window.
    fn bm(&self) -> (usize, Vec<u8>) {
        let n = self.window.min(self.observed.len());
        berlekamp_massey(&self.observed[self.observed.len() - n..])
    }

    /// The phase-II stream as the player knows it: the log, extended with their register.
    fn p2_extended(&self, len: usize) -> Vec<u8> {
        let mut s = self.p2_log.clone();
        while s.len() < len {
            match predict(&self.p2_poly, &s) {
                Some(b) if !self.p2_poly.is_empty() => s.push(b),
                _ => break,
            }
        }
        s
    }

    /// Phase III: do the new beats line up with the phase-II log at the chosen offset?
    fn aligned(&self) -> bool {
        let n = self.observed.len();
        if n < 6 {
            return false;
        }
        let ext = self.p2_extended(2 * self.offset + n);
        ext.len() >= 2 * self.offset + n && ext[2 * self.offset..2 * self.offset + n] == self.observed[..]
    }

    /// The next two bits as the player can predict them, if they have a lock.
    fn predicted(&self) -> Option<usize> {
        let two = |taps: &[u8], seq: &[u8]| -> Option<usize> {
            let a = predict(taps, seq)?;
            let mut s = seq.to_vec();
            s.push(a);
            let b = predict(taps, &s)?;
            Some((a * 2 + b) as usize)
        };
        match self.phase {
            0 => (self.observed.len() >= 10 && mismatches(&self.taps, &self.observed) == 0).then(|| two(&self.taps, &self.observed)).flatten(),
            1 => {
                let (l, c) = self.bm();
                (l > 0 && self.observed.len() >= 2 * l && mismatches(&c, &self.observed) == 0).then(|| two(&c, &self.observed)).flatten()
            }
            _ => {
                if self.aligned() {
                    let at = 2 * self.offset + self.observed.len();
                    let ext = self.p2_extended(at + 2);
                    (ext.len() >= at + 2).then(|| (ext[at] * 2 + ext[at + 1]) as usize)
                } else if !self.p2_poly.is_empty() && self.observed.len() >= 2 * self.p2_poly.len() && mismatches(&self.p2_poly, &self.observed) == 0 {
                    two(&self.p2_poly, &self.observed)
                } else {
                    None
                }
            }
        }
    }

    fn locked(&self) -> bool {
        self.predicted().is_some()
    }

    // --- tools -------------------------------------------------------------------------------

    fn toggle_tap(&mut self, i: usize) {
        if self.phase == 0 && i < 5 {
            self.taps[i] ^= 1;
        }
    }

    fn window_by(&mut self, d: i32) {
        if self.phase == 1 {
            self.window = (self.window as i32 + d).clamp(2, MAX_WINDOW as i32) as usize;
        }
    }

    fn offset_by(&mut self, d: i32) {
        if self.phase == 2 {
            self.offset = (self.offset as i32 + d).clamp(0, 30) as usize;
        }
    }

    /// Panel cells, in field units: (what, rect).
    fn cells(&self) -> Vec<(Cell, Rect)> {
        let r = |x: f32, y: f32, w: f32, h: f32| Rect::from_center_size(Pos2::new(x, y), Vec2::new(w, h));
        let mut v = Vec::new();
        match self.phase {
            0 => {
                for i in 0..5 {
                    v.push((Cell::Tap(i), r(3.6 + i as f32 * 1.8, 3.4, 1.4, 1.4)));
                }
            }
            1 => {
                v.push((Cell::Less, r(3.6, 3.4, 1.4, 1.4)));
                v.push((Cell::More, r(10.8, 3.4, 1.4, 1.4)));
            }
            _ => {
                v.push((Cell::Back, r(3.6, 3.4, 1.4, 1.4)));
                v.push((Cell::Fwd, r(10.8, 3.4, 1.4, 1.4)));
            }
        }
        v
    }

    fn feel(&mut self, n: Note) {
        self.note = Some((n, 0.0));
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cell {
    Tap(usize),
    Less,
    More,
    Back,
    Fwd,
}

impl Dive for Lamplighter {
    fn id(&self) -> &'static str {
        "lamplighter"
    }
    fn title(&self) -> &'static str {
        "THE LAMPLIGHTER"
    }
    fn done_flag(&self) -> &'static str {
        DIVE_LAMPLIGHTER
    }
    fn level(&self) -> (usize, usize, String) {
        (self.phase + 1, 3, self.cfg().name.to_string())
    }
    fn brief(&self) -> String {
        let lock = if self.locked() { "LOCKED: call the dark lane (1–4) before the light shows." } else { "No lock yet: dodge, and watch." };
        let what = match self.phase {
            0 => "Its lanes come from one shift register, five bits long. Collect ten bits, then set the taps (5–9 or click) until they fit every bit you saw.",
            1 => "Two registers XORed: still linear, just longer. Widen the Berlekamp–Massey window (− / +) until its register fits the whole log. It needs twice its length in bits.",
            _ => "It rebooted with phase II's seed. The same keystream, from somewhere along it: slide your phase-II log ([ ]) until the new beats line up.",
        };
        format!("{what} {lock}")
    }
    fn trace(&self) -> f32 {
        self.trace
    }
    fn field(&self) -> (f32, f32) {
        FIELD
    }
    fn set_merciful(&mut self, on: bool) {
        self.merciful = on;
    }
    fn tally(&self) -> Option<(u32, u32)> {
        Some((self.deaths, self.hits))
    }

    fn update(&mut self, dt: f32, input: &Input, fx: &mut Fx) {
        self.t += dt;
        if let Some((_, a)) = self.note.as_mut() {
            *a += dt;
            if *a > 1.1 {
                self.note = None;
            }
        }
        if let Some((_, a)) = self.spark.as_mut() {
            *a += dt;
        }
        match self.mode {
            Mode::Intro(a) => {
                self.mode = if a > 1.6 { Mode::Fight } else { Mode::Intro(a + dt) };
                return;
            }
            Mode::Down(a) => {
                if a > 1.8 {
                    self.phase += 1;
                    if self.phase >= PHASES.len() {
                        self.phase = PHASES.len() - 1;
                        self.mode = Mode::Done;
                        return;
                    }
                    self.restart_phase();
                    self.mode = Mode::Fight;
                } else {
                    self.mode = Mode::Down(a + dt);
                }
                return;
            }
            Mode::Traced(a) => {
                if a > 1.6 {
                    self.restart_phase();
                    self.mode = Mode::Fight;
                } else {
                    self.mode = Mode::Traced(a + dt);
                }
                return;
            }
            Mode::Done => return,
            Mode::Fight => {}
        }
        // --- the player ---
        if input.left && self.lane > 0 {
            self.lane -= 1;
        }
        if input.right && self.lane + 1 < LANES {
            self.lane += 1;
        }
        let target = lane_x(self.lane);
        let step = SPEED * dt;
        self.x = if (target - self.x).abs() <= step { target } else { self.x + step * (target - self.x).signum() };
        if input.fire && self.iframes <= 0.0 && self.stamina >= DODGE_COST {
            self.iframes = IFRAMES;
            self.stamina -= DODGE_COST;
            fx.sfx.push("blip");
        }
        if self.iframes > 0.0 {
            self.iframes -= dt;
        } else {
            self.stamina = (self.stamina + REGEN * dt).min(100.0);
        }
        // tools and calls, by key
        for ch in &input.typed {
            match ch {
                '1'..='4' => {
                    let lane = (*ch as u8 - b'1') as usize;
                    if self.beat == Beat::Gap {
                        self.call = Some(lane);
                        fx.sfx.push("tick");
                    } else {
                        self.feel(Note::Late);
                    }
                }
                '5'..='9' => self.toggle_tap((*ch as u8 - b'5') as usize),
                '-' => self.window_by(-2),
                '=' | '+' => self.window_by(2),
                '[' => self.offset_by(-1),
                ']' => self.offset_by(1),
                _ => {}
            }
        }
        // and by pointer
        if input.pressed
            && let Some((px, py)) = input.pointer
        {
            let hit = self.cells().into_iter().find(|(_, r)| r.expand(0.2).contains(Pos2::new(px, py))).map(|(c, _)| c);
            match hit {
                Some(Cell::Tap(i)) => self.toggle_tap(i),
                Some(Cell::Less) => self.window_by(-2),
                Some(Cell::More) => self.window_by(2),
                Some(Cell::Back) => self.offset_by(-1),
                Some(Cell::Fwd) => self.offset_by(1),
                None => {
                    // a click on a lane is a call
                    if px < 1.0 && self.beat == Beat::Gap {
                        let lane = (((px - lane_x(0)) / LANE_W).round().clamp(0.0, 3.0)) as usize;
                        self.call = Some(lane);
                        fx.sfx.push("tick");
                    }
                }
            }
        }
        // --- the lamp ---
        self.trace = (self.trace - DECAY * dt).max(0.0);
        self.bt += dt;
        let cfg = self.cfg();
        match self.beat {
            Beat::Gap if self.bt >= cfg.gap => {
                self.beat = Beat::Tell;
                self.bt = 0.0;
                self.observed.push((self.safe >> 1) as u8);
                self.observed.push((self.safe & 1) as u8);
                if self.phase == 1 {
                    self.p2_log = self.observed.clone();
                }
                fx.sfx.push("ping");
            }
            Beat::Tell if self.bt >= cfg.tell => {
                self.beat = Beat::Strike;
                self.bt = 0.0;
                let me = self.player_lane();
                match self.call.take() {
                    Some(c) if c == self.safe => {
                        self.hp = self.hp.saturating_sub(1);
                        self.feel(Note::Reflected);
                        self.spark = Some((Pos2Field(BOSS.0, BOSS.1), 0.0));
                        fx.sfx.push("success");
                        fx.shake = 0.25;
                    }
                    other => {
                        if other.is_some() {
                            self.stamina = 0.0;
                            self.feel(Note::Miscall);
                            fx.sfx.push("error");
                        }
                        if me != self.safe {
                            if self.iframes > 0.0 {
                                self.feel(Note::Dodged);
                            } else {
                                self.hits += 1;
                                if !self.merciful {
                                    self.trace = (self.trace + HIT).min(100.0);
                                }
                                self.feel(Note::Hit);
                                fx.sfx.push("warden");
                                fx.shake = 0.5;
                            }
                        }
                    }
                }
            }
            Beat::Strike if self.bt >= cfg.strike => {
                self.beat = Beat::Gap;
                self.bt = 0.0;
                if self.hp == 0 {
                    if self.phase == 1 {
                        // what phase III will be checked against
                        self.p2_poly = self.bm().1;
                        if mismatches(&self.p2_poly, &self.p2_log) > 0 {
                            self.p2_poly = berlekamp_massey(&self.p2_log).1;
                        }
                    }
                    self.mode = if self.phase + 1 >= PHASES.len() { Mode::Done } else { Mode::Down(0.0) };
                    fx.sfx.push("unlock");
                    return;
                }
                self.start_beat();
            }
            _ => {}
        }
        if self.trace >= 100.0 {
            self.deaths += 1;
            self.mode = Mode::Traced(0.0);
            fx.sfx.push("error");
            fx.shake = 0.8;
        }
    }

    fn status(&self) -> Status {
        if self.mode == Mode::Done { Status::Cleared } else { Status::Playing }
    }

    fn bot(&mut self, dt: f32) -> Input {
        let mut i = Input::default();
        if self.mode != Mode::Fight {
            return i;
        }
        self.bot_cool -= dt;
        // the tools: what a player who has done the work would set
        if self.bot_cool <= 0.0 {
            self.bot_cool = 0.12;
            match self.phase {
                0 if self.observed.len() >= 10 => {
                    if let Some(k) = (0..5).find(|&k| self.taps[k] != self.taps1[k]) {
                        i.typed.push((b'5' + k as u8) as char);
                    }
                }
                1 => {
                    let want = (self.observed.len() & !1).min(MAX_WINDOW);
                    if self.window + 1 < want && !self.locked() {
                        i.typed.push('=');
                    }
                }
                2 if self.observed.len() >= 6 && !self.locked() => {
                    i.typed.push(if self.offset < self.k3 { ']' } else { '[' });
                }
                _ => {}
            }
        }
        // a call, if it can predict
        if self.beat == Beat::Gap
            && self.call.is_none()
            && self.bt > 0.05
            && let Some(p) = self.predicted()
        {
            i.typed.push((b'1' + p as u8) as char);
        }
        // where to stand: the predicted lane in the gap, the dark lane once it shows
        let want = match (self.beat, self.predicted()) {
            (Beat::Gap, Some(p)) => p,
            (Beat::Gap, None) => self.lane,
            _ => self.safe,
        };
        if want < self.lane {
            i.left = true;
        } else if want > self.lane {
            i.right = true;
        }
        // too far to make it: roll
        if self.beat == Beat::Tell && self.player_lane() != self.safe && self.cfg().tell - self.bt < 0.08 && self.call != Some(self.safe) {
            i.fire = true;
        }
        i
    }

    fn draw(&self, p: &egui::Painter, v: &View) {
        grid(p, v, FIELD, 1.0);
        let cfg = self.cfg();
        // lanes
        for k in 0..LANES {
            let x0 = lane_x(k) - LANE_W / 2.0;
            let r = Rect::from_two_pos(v.at(x0, 4.2), v.at(x0 + LANE_W, -6.8));
            p.rect_stroke(r, 0.0, Stroke::new(1.0, NEON_CYAN.gamma_multiply(0.12)), egui::StrokeKind::Inside);
            let called = self.call == Some(k);
            p.text(v.at(lane_x(k), -6.3), Align2::CENTER_CENTER, format!("{}", k + 1), retro(v.len(1.1)), if called { NEON_GOLD } else { NEON_CYAN.gamma_multiply(0.4) });
        }
        // the floodlight: three lanes lit, one dark
        let lit = match self.beat {
            Beat::Tell => Some((NEON_GOLD, 0.10 + 0.12 * (self.bt / cfg.tell))),
            Beat::Strike => Some((NEON_RED, 0.42)),
            Beat::Gap => None,
        };
        if let (Some((c, a)), Mode::Fight) = (lit, self.mode) {
            for k in (0..LANES).filter(|k| *k != self.safe) {
                let x0 = lane_x(k) - LANE_W / 2.0;
                let r = Rect::from_two_pos(v.at(x0 + 0.1, 4.2), v.at(x0 + LANE_W - 0.1, -6.8));
                p.rect_filled(r, 0.0, c.gamma_multiply(a));
                if self.beat == Beat::Strike {
                    glow_line(p, v.at(BOSS.0, BOSS.1), v.at(lane_x(k), -6.8), NEON_RED, 2.0);
                }
            }
        }
        // the lamp
        let hurt = matches!(self.mode, Mode::Down(_) | Mode::Done);
        let bc = if hurt { NEON_GREEN } else { NEON_PINK };
        glow_ring(p, v.at(BOSS.0, BOSS.1), v.len(1.1), bc, 2.0);
        glow_dot(p, v.at(BOSS.0, BOSS.1), v.len(0.45), if self.beat == Beat::Gap { bc.gamma_multiply(0.5) } else { NEON_GOLD });
        for k in 0..cfg.hp {
            let on = k < self.hp;
            let at = v.at(BOSS.0 - (cfg.hp as f32 - 1.0) * 0.45 + k as f32 * 0.9, BOSS.1 - 1.6);
            p.rect_filled(Rect::from_center_size(at, Vec2::splat(v.len(0.55))), 1.0, if on { NEON_PINK } else { DEEP });
            p.rect_stroke(Rect::from_center_size(at, Vec2::splat(v.len(0.55))), 1.0, Stroke::new(1.0, NEON_PINK.gamma_multiply(0.6)), egui::StrokeKind::Inside);
        }
        if let Some((at, age)) = self.spark {
            sparks(p, v.at(at.0, at.1), age, NEON_GOLD, 14, v.len(2.5), (self.t * 7.0) as u32);
        }
        // you
        let me = v.at(self.x, PLAYER_Y);
        let ic = if self.iframes > 0.0 { NEON_VIOLET } else { NEON_CYAN };
        glow_dot(p, me, v.len(0.38), ic);
        if self.iframes > 0.0 {
            glow_ring(p, me, v.len(0.9), NEON_VIOLET, 1.5);
        }
        let bar = Rect::from_min_size(v.at(self.x - 1.0, PLAYER_Y - 0.8), Vec2::new(v.len(2.0), v.len(0.18)));
        p.rect_filled(bar, 1.0, DEEP);
        let mut f = bar;
        f.set_width(bar.width() * self.stamina / 100.0);
        p.rect_filled(f, 1.0, if self.stamina >= DODGE_COST { NEON_GREEN } else { NEON_RED.gamma_multiply(0.7) });
        if let Some((n, a)) = self.note {
            let (s, c) = match n {
                Note::Reflected => ("REFLECTED", NEON_GOLD),
                Note::Hit => ("HIT", NEON_RED),
                Note::Miscall => ("MISCALL", NEON_RED),
                Note::Late => ("TOO LATE: A CALL IS A PREDICTION", NEON_PINK),
                Note::Dodged => ("DODGED", NEON_VIOLET),
            };
            p.text(v.at(-5.0, -2.0 + a), Align2::CENTER_CENTER, s, retro(v.len(0.9)), c.gamma_multiply(1.0 - a / 1.1));
        }
        // --- the bench: what you have worked out -------------------------------------------
        let px = 2.2;
        p.rect_filled(Rect::from_two_pos(v.at(px - 0.4, 6.8), v.at(12.8, -7.2)), 3.0, egui::Color32::from_rgba_unmultiplied(8, 12, 24, 220));
        let text = |x: f32, y: f32, s: String, size: f32, c: egui::Color32| {
            p.text(v.at(x, y), Align2::LEFT_CENTER, s, mono(v.len(size)), c);
        };
        let title = match self.phase {
            0 => "SHIFT REGISTER · 5 TAPS",
            1 => "BERLEKAMP–MASSEY",
            _ => "SEED REUSE · ALIGN",
        };
        text(px, 6.1, title.to_string(), 0.62, NEON_CYAN);
        for (c, r) in self.cells() {
            let rr = Rect::from_two_pos(v.at(r.min.x, r.max.y), v.at(r.max.x, r.min.y));
            let (label, on) = match c {
                Cell::Tap(i) => (format!("c{}", i + 1), self.taps[i] == 1),
                Cell::Less => ("−".into(), false),
                Cell::More => ("+".into(), false),
                Cell::Back => ("◀".into(), false),
                Cell::Fwd => ("▶".into(), false),
            };
            p.rect_filled(rr, 2.0, if on { NEON_GOLD.gamma_multiply(0.35) } else { DEEP });
            p.rect_stroke(rr, 2.0, Stroke::new(1.0, if on { NEON_GOLD } else { NEON_CYAN.gamma_multiply(0.5) }), egui::StrokeKind::Inside);
            p.text(rr.center(), Align2::CENTER_CENTER, label, mono(v.len(0.55)), if on { NEON_GOLD } else { NEON_CYAN });
        }
        let n = self.observed.len();
        match self.phase {
            0 => {
                text(px, 4.9, "s[t] = c1·s[t−1] ⊕ … ⊕ c5·s[t−5]".into(), 0.45, NEON_CYAN.gamma_multiply(0.7));
                let m = mismatches(&self.taps, &self.observed);
                let checks = n.saturating_sub(5);
                let (s, c) = if n < 10 { (format!("{n}/10 bits: keep watching"), NEON_CYAN.gamma_multiply(0.7)) } else if m == 0 { (format!("fits all {checks} checks"), NEON_GREEN) } else { (format!("{m} of {checks} checks fail"), NEON_RED) };
                text(px, 2.0, s, 0.5, c);
            }
            1 => {
                p.text(v.at(7.2, 3.4), Align2::CENTER_CENTER, format!("window {} bits", self.window.min(n)), mono(v.len(0.5)), NEON_CYAN);
                let (l, c) = self.bm();
                let poly: String = c.iter().map(|b| if *b == 1 { '1' } else { '·' }).collect();
                text(px, 2.0, format!("L = {l}   taps {poly}"), 0.48, NEON_CYAN);
                let m = mismatches(&c, &self.observed);
                let (s, col) = if l == 0 {
                    ("no register yet".to_string(), NEON_CYAN.gamma_multiply(0.6))
                } else if m > 0 {
                    (format!("breaks on {m} of the log's bits"), NEON_RED)
                } else if n < 2 * l {
                    (format!("fits, but {n} < 2L = {} bits", 2 * l), NEON_GOLD)
                } else {
                    ("fits the whole log".to_string(), NEON_GREEN)
                };
                text(px, 1.0, s, 0.48, col);
            }
            _ => {
                p.text(v.at(7.2, 3.4), Align2::CENTER_CENTER, format!("offset {} beats", self.offset), mono(v.len(0.5)), NEON_CYAN);
                // the phase-II log, and the new beats laid under it at the offset
                let log = self.p2_extended(2 * (self.offset + n / 2) + 2);
                let beats = (log.len() / 2).min(14);
                let cw = 0.72;
                for b in 0..beats {
                    let lane = log[2 * b] * 2 + log[2 * b + 1] + 1;
                    p.text(v.at(px + 0.3 + b as f32 * cw, 2.0), Align2::CENTER_CENTER, lane.to_string(), mono(v.len(0.45)), NEON_VIOLET.gamma_multiply(0.8));
                }
                for b in 0..n / 2 {
                    let at = self.offset + b;
                    if at >= 14 {
                        break;
                    }
                    let lane = self.observed[2 * b] * 2 + self.observed[2 * b + 1];
                    let ok = log.get(2 * at..2 * at + 2).is_some_and(|w| w == &self.observed[2 * b..2 * b + 2]);
                    p.text(v.at(px + 0.3 + at as f32 * cw, 1.0), Align2::CENTER_CENTER, (lane + 1).to_string(), mono(v.len(0.45)), if ok { NEON_GREEN } else { NEON_RED });
                }
                text(px, 0.0, if n < 6 { format!("{}/3 new beats", n / 2) } else if self.aligned() { "lined up: same keystream".into() } else { "no match at this offset".into() }, 0.45, if self.aligned() { NEON_GREEN } else { NEON_CYAN.gamma_multiply(0.7) });
            }
        }
        // the prediction
        match self.predicted() {
            Some(l) => {
                text(px, -1.8, "NEXT DARK LANE".into(), 0.5, NEON_GOLD.gamma_multiply(0.8));
                p.text(v.at(10.6, -1.8), Align2::CENTER_CENTER, format!("{}", l + 1), retro(v.len(1.4)), NEON_GOLD);
            }
            None => text(px, -1.8, "NO LOCK".into(), 0.5, NEON_PINK.gamma_multiply(0.8)),
        }
        // the log, newest last
        text(px, -3.6, "SEEN".into(), 0.42, NEON_CYAN.gamma_multiply(0.6));
        let beats: Vec<String> = self.observed.chunks(2).map(|w| ((w[0] * 2 + w[1]) + 1).to_string()).collect();
        let tail = beats.iter().rev().take(12).rev().cloned().collect::<Vec<_>>().join(" ");
        text(px, -4.3, tail, 0.45, NEON_CYAN);
        let bits: String = self.observed.iter().rev().take(24).rev().map(|b| if *b == 1 { '1' } else { '0' }).collect();
        text(px, -5.0, bits, 0.42, NEON_CYAN.gamma_multiply(0.55));
        text(px, -6.4, format!("hits {}  ·  traced {}", self.hits, self.deaths), 0.42, NEON_CYAN.gamma_multiply(0.5));
        // phase cards
        let card = match self.mode {
            Mode::Intro(_) => Some(("THE LAMPLIGHTER", NEON_PINK)),
            Mode::Down(_) => Some(("PHASE BROKEN", NEON_GREEN)),
            Mode::Traced(_) => Some(("TRACED · THE PHASE STARTS OVER · YOU KEEP WHAT YOU WORKED OUT", NEON_RED)),
            Mode::Done => Some(("THE LAMP GOES DARK", NEON_GOLD)),
            Mode::Fight => None,
        };
        if let Some((s, c)) = card {
            p.text(v.at(-5.0, 0.5), Align2::CENTER_CENTER, s, retro(v.len(if s.len() > 20 { 0.6 } else { 1.3 })), c);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dive::autoplay;

    #[test]
    fn the_bot_breaks_all_three_phases_by_prediction() {
        let mut d = Lamplighter::new(b"lamplighter-test-seed-0123456789", 0);
        let t = autoplay(&mut d, 900.0).expect("the lamp goes dark");
        eprintln!("lamplighter: {t:.0} s, hits {}, traced {}", d.hits, d.deaths);
        assert!(t < 400.0, "took {t:.0} s");
        assert_eq!(d.status(), Status::Cleared);
    }

    #[test]
    fn dodging_alone_never_hurts_it() {
        let mut d = Lamplighter::new(b"lamplighter-test-seed-0123456789", 1);
        let mut fx = Fx::default();
        let dt = 1.0 / 60.0;
        for _ in 0..(60 * 120) {
            // a player who only ever runs to the dark lane once it shows
            let mut i = Input::default();
            if d.mode == Mode::Fight && d.beat != Beat::Gap {
                if d.safe < d.lane {
                    i.left = true;
                } else if d.safe > d.lane {
                    i.right = true;
                }
            }
            d.update(dt, &i, &mut fx);
        }
        assert_eq!(d.phase, 0);
        assert_eq!(d.hp, PHASES[0].hp, "only a correct call hurts it");
    }

    #[test]
    fn taps_that_fit_ten_bits_are_the_real_taps() {
        let mut d = Lamplighter::new(b"lamplighter-test-seed-0123456789", 2);
        // ten bits of the register
        let mut g = d.stream.clone();
        d.observed = (0..10).map(|_| g.bit()).collect();
        let fits: Vec<[u8; 5]> = (0..32u8).map(|m| [m & 1, (m >> 1) & 1, (m >> 2) & 1, (m >> 3) & 1, (m >> 4) & 1]).filter(|t| mismatches(t, &d.observed) == 0 && t.iter().any(|b| *b == 1)).collect();
        assert_eq!(fits.len(), 1);
        assert_eq!(fits[0].to_vec(), d.taps1);
    }

    #[test]
    fn a_short_window_breaks_and_the_reboot_lines_up() {
        let mut d = Lamplighter::new(b"lamplighter-test-seed-0123456789", 3);
        d.phase = 1;
        d.restart_phase();
        let mut g = d.stream.clone();
        // the beat already drawn, then 24 more bits
        d.observed = vec![(d.safe >> 1) as u8, (d.safe & 1) as u8];
        d.observed.extend((0..24).map(|_| g.bit()));
        d.window = 8;
        assert!(!d.locked(), "eight bits cannot pin down a length-9 register");
        d.window = 18;
        assert!(d.locked());
        d.p2_log = d.observed.clone();
        d.p2_poly = d.bm().1;
        d.phase = 2;
        d.restart_phase();
        let mut g3 = d.stream.clone();
        d.observed = vec![(d.safe >> 1) as u8, (d.safe & 1) as u8];
        d.observed.extend((0..4).map(|_| g3.bit()));
        d.offset = d.k3;
        assert!(d.aligned(), "the reused seed lines up at offset {}", d.k3);
        d.offset = d.k3 + 1;
        assert!(!d.aligned());
    }
}
