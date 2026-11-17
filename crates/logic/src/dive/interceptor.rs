//! INTERCEPTOR: Shamir's secret sharing as a shooting gallery.
//!
//! A secret is the height of a polynomial at x = 0. The shares are points on
//! it. Bend your own curve until it threads the shares, fire, and the disc
//! rides the curve into the cloaked core at x = 0.
//!
//! The first six levels are over the reals, where you can feel the curve.
//! With too few shares the free handle sweeps the whole column: the secret
//! could be anywhere. With more shares than you need, they all agree, and a
//! forged one stands out as the only node that never lights with the rest.
//! The last two levels are the real thing, integers mod 31: the curve turns
//! into scattered dots, and the same interpolation still lands on the secret.

use cpl_crypto::drbg::Drbg;
use egui::{Align2, Pos2, Stroke, Vec2};

use super::{Dive, Fx, Input, NEON_CYAN, NEON_GOLD, NEON_GREEN, NEON_PINK, NEON_RED, NEON_VIOLET, Status, View, diamond, glow_dot, glow_line, glow_path, glow_ring, grid, sparks};
use crate::signals::wharf::DIVE_INTERCEPTOR;
use crate::ui::theme::{mono, retro};

const FIELD: (f32, f32) = (22.0, 13.0);
const HALF_W: f32 = 10.0;
const HALF_H: f32 = 5.8;
/// How close the curve must pass to light a share.
const TOL: f32 = 0.13;
/// How close the disc must pass to the core.
const HIT: f32 = 0.3;
const SNAP: f32 = 0.25;
const DISC_SPEED: f32 = 11.0;
const MISS_TRACE: f32 = 15.0;
/// The prime of the last two levels.
pub const P: i64 = 31;
const CELL: f32 = 0.36;

#[derive(Clone, Debug)]
struct Share {
    x: f32,
    y: f32,
    honest: bool,
    /// seconds into the level at which it logs on
    at: f32,
}

#[derive(Clone, Debug)]
struct Node {
    x: i64,
    y: i64,
    honest: bool,
}

#[derive(Clone, Debug)]
enum Kind {
    Real {
        /// x of each handle and the height that solves the level
        handles: Vec<(f32, f32)>,
        shares: Vec<Share>,
    },
    Field {
        k: usize,
        nodes: Vec<Node>,
        secret: i64,
    },
}

#[derive(Clone, Debug)]
struct LevelDef {
    name: &'static str,
    brief: &'static str,
    kind: Kind,
    /// trace per second while aiming
    rate: f32,
}

#[derive(Clone, Debug, PartialEq)]
enum Phase {
    /// the level's name card
    Intro(f32),
    Aim,
    /// disc x so far, whether it has reached the column, hit or miss
    Flying { x: f32, result: Option<bool> },
    Boom(f32),
    Traced(f32),
}

pub struct Interceptor {
    levels: Vec<LevelDef>,
    idx: usize,
    phase: Phase,
    /// handle heights (real levels)
    ys: Vec<f32>,
    /// chosen nodes (field levels)
    picked: Vec<usize>,
    /// which handle the keyboard moves, which one the pointer holds
    sel: usize,
    drag: Option<usize>,
    lit: Vec<bool>,
    trace: f32,
    /// seconds in this level
    time: f32,
    t: f32,
    misses: u32,
    trail: Vec<(f32, f32)>,
    /// where the last disc crossed the column
    last_shot: Option<(f32, f32)>,
    cleared: bool,
    bot_wait: f32,
    pub total_misses: u32,
}

// --- arithmetic --------------------------------------------------------------------

/// The polynomial through `pts`, evaluated at `x` (Lagrange).
pub fn lagrange(pts: &[(f32, f32)], x: f32) -> f32 {
    let mut sum = 0.0;
    for (i, (xi, yi)) in pts.iter().enumerate() {
        let mut term = *yi;
        for (j, (xj, _)) in pts.iter().enumerate() {
            if i != j {
                term *= (x - xj) / (xi - xj);
            }
        }
        sum += term;
    }
    sum
}

fn inv_mod(a: i64) -> i64 {
    // Fermat: a^(p-2)
    let (mut r, mut b, mut e) = (1i64, a.rem_euclid(P), P - 2);
    while e > 0 {
        if e & 1 == 1 {
            r = r * b % P;
        }
        b = b * b % P;
        e >>= 1;
    }
    r
}

/// The same interpolation over GF(31).
pub fn lagrange_mod(pts: &[(i64, i64)], x: i64) -> i64 {
    let mut sum = 0i64;
    for (i, (xi, yi)) in pts.iter().enumerate() {
        let (mut num, mut den) = (1i64, 1i64);
        for (j, (xj, _)) in pts.iter().enumerate() {
            if i != j {
                num = num * (x - xj).rem_euclid(P) % P;
                den = den * (xi - xj).rem_euclid(P) % P;
            }
        }
        sum = (sum + yi * num % P * inv_mod(den)) % P;
    }
    sum.rem_euclid(P)
}

// --- levels ------------------------------------------------------------------------

fn handle_xs(n: usize) -> Vec<f32> {
    match n {
        2 => vec![-8.0, 8.0],
        3 => vec![-8.0, 0.0, 8.0],
        _ => vec![-9.0, -3.0, 3.0, 9.0],
    }
}

/// A random curve that stays on the screen, with its handles on the snap grid.
fn real_level(rng: &mut Drbg, degree: usize, share_xs: &[f32], liar: Option<usize>, late: Option<(usize, f32)>) -> Kind {
    let xs = handle_xs(degree + 1);
    loop {
        let handles: Vec<(f32, f32)> = xs.iter().map(|x| (*x, (rng.int(33) as f32 - 16.0) * SNAP)).collect();
        // on screen everywhere, not flat, and not already solved by leaving the handles at zero
        let on_screen = (0..=80).all(|i| lagrange(&handles, -HALF_W + i as f32 * 0.25).abs() <= HALF_H - 0.6);
        let lively = handles.iter().map(|h| h.1.abs()).sum::<f32>() >= 2.0 * (degree as f32 + 1.0) && handles.windows(2).any(|w| (w[0].1 - w[1].1).abs() >= 1.5);
        let secret = lagrange(&handles, 0.0);
        if !on_screen || !lively || secret.abs() < 0.75 {
            continue;
        }
        let mut shares: Vec<Share> = share_xs.iter().map(|x| Share { x: *x, y: lagrange(&handles, *x), honest: true, at: 0.0 }).collect();
        if let Some(i) = liar {
            let off = if shares[i].y > 0.0 { -1.0 } else { 1.0 } * (1.1 + rng.int(4) as f32 * 0.2);
            shares[i].y += off;
            shares[i].honest = false;
        }
        if let Some((i, at)) = late {
            shares[i].at = at;
        }
        return Kind::Real { handles, shares };
    }
}

fn field_level(rng: &mut Drbg, k: usize, n: usize, liar: bool) -> Kind {
    loop {
        let coeffs: Vec<i64> = (0..k).map(|_| rng.int(P as usize) as i64).collect();
        if coeffs[k - 1] == 0 {
            continue;
        }
        let f = |x: i64| coeffs.iter().rev().fold(0i64, |acc, c| (acc * x + c) % P);
        let mut xs: Vec<i64> = (1..P).collect();
        rng.shuffle(&mut xs);
        let mut nodes: Vec<Node> = xs.into_iter().take(n).map(|x| Node { x, y: f(x), honest: true }).collect();
        nodes.sort_by_key(|n| n.x);
        if liar {
            let i = rng.int(n);
            nodes[i].y = (nodes[i].y + 1 + rng.int((P - 1) as usize) as i64) % P;
            nodes[i].honest = false;
            // the forgery must not sit on the true curve, nor fool any honest pair into a neighbour
            let pts: Vec<(i64, i64)> = nodes.iter().filter(|n| n.honest).take(k).map(|n| (n.x, n.y)).collect();
            if lagrange_mod(&pts, nodes[i].x) == nodes[i].y {
                continue;
            }
        }
        return Kind::Field { k, nodes, secret: coeffs[0] };
    }
}

fn levels(seed: &[u8]) -> Vec<LevelDef> {
    let mut rng = Drbg::new(seed, "dive/interceptor");
    let liar = 1 + rng.int(3);
    vec![
        LevelDef { name: "TWO KEEPERS", brief: "Two shares, one line. Drag the handles until the line threads both, then fire.", kind: real_level(&mut rng, 1, &[-5.0, 6.0], None, None), rate: 0.7 },
        LevelDef { name: "A THIRD", brief: "Three shares bend it into one curve, and only one.", kind: real_level(&mut rng, 2, &[-6.0, 3.0, 7.0], None, None), rate: 0.9 },
        LevelDef { name: "ANY THREE", brief: "Five keepers, and a curve that needs three. Thread any three.", kind: real_level(&mut rng, 2, &[-9.0, -5.0, -2.0, 4.0, 7.0], None, None), rate: 1.1 },
        LevelDef { name: "ONE OF THEM LIES", brief: "Four agree. The one that never lights with the others is forged.", kind: real_level(&mut rng, 2, &[-9.0, -5.0, -2.0, 4.0, 7.0], Some(liar), None), rate: 1.1 },
        LevelDef { name: "NOT ENOUGH", brief: "Two keepers online. Move the free handle and watch where the core could be.", kind: real_level(&mut rng, 2, &[-6.0, 5.0, 2.0], None, Some((2, 14.0))), rate: 1.0 },
        LevelDef { name: "FOUR OF FOUR", brief: "A cubic. Every share you add pins it a little more.", kind: real_level(&mut rng, 3, &[-7.0, -5.0, 2.0, 6.0], None, None), rate: 0.9 },
        LevelDef { name: "THE REAL THING", brief: "Integers mod 31. Click three nodes: the curve is the scatter of pink dots. Fire.", kind: field_level(&mut rng, 3, 5, false), rate: 1.0 },
        LevelDef { name: "NOISE WITH A LIAR IN IT", brief: "Six nodes, one forged. The right three light the other honest ones too.", kind: field_level(&mut rng, 3, 6, true), rate: 1.0 },
    ]
}

fn node_pos(x: i64, y: i64) -> (f32, f32) {
    ((x as f32 - 15.0) * CELL * 1.5, (y as f32 - 15.0) * CELL)
}

impl Interceptor {
    pub fn new(seed: &[u8]) -> Self {
        let mut d = Self {
            levels: levels(seed),
            idx: 0,
            phase: Phase::Intro(0.0),
            ys: Vec::new(),
            picked: Vec::new(),
            sel: 0,
            drag: None,
            lit: Vec::new(),
            trace: 0.0,
            time: 0.0,
            t: 0.0,
            misses: 0,
            trail: Vec::new(),
            last_shot: None,
            cleared: false,
            bot_wait: 0.0,
            total_misses: 0,
        };
        d.load();
        d
    }

    fn load(&mut self) {
        self.phase = Phase::Intro(0.0);
        self.trace = 0.0;
        self.time = 0.0;
        self.misses = 0;
        self.trail.clear();
        self.last_shot = None;
        self.drag = None;
        self.sel = 0;
        self.picked.clear();
        match &self.levels[self.idx].kind {
            Kind::Real { handles, shares } => {
                self.ys = vec![0.0; handles.len()];
                self.lit = vec![false; shares.len()];
            }
            Kind::Field { nodes, .. } => {
                self.ys.clear();
                self.lit = vec![false; nodes.len()];
            }
        }
    }

    fn handles(&self) -> Vec<(f32, f32)> {
        match &self.levels[self.idx].kind {
            Kind::Real { handles, .. } => handles.iter().zip(&self.ys).map(|(h, y)| (h.0, *y)).collect(),
            Kind::Field { .. } => Vec::new(),
        }
    }

    /// The curve you have drawn, at x.
    fn curve(&self, x: f32) -> f32 {
        lagrange(&self.handles(), x)
    }

    fn picked_pts(&self) -> Vec<(i64, i64)> {
        match &self.levels[self.idx].kind {
            Kind::Field { nodes, .. } => self.picked.iter().map(|i| (nodes[*i].x, nodes[*i].y)).collect(),
            _ => Vec::new(),
        }
    }

    /// Where the core really is.
    fn secret(&self) -> f32 {
        match &self.levels[self.idx].kind {
            Kind::Real { handles, .. } => lagrange(handles, 0.0),
            Kind::Field { secret, .. } => *secret as f32,
        }
    }

    fn relight(&mut self, fx: &mut Fx) {
        let now: Vec<bool> = match &self.levels[self.idx].kind {
            Kind::Real { shares, .. } => shares.iter().map(|s| self.time >= s.at && (self.curve(s.x) - s.y).abs() <= TOL).collect(),
            Kind::Field { k, nodes, .. } => {
                let pts = self.picked_pts();
                nodes.iter().map(|n| pts.len() == *k && lagrange_mod(&pts, n.x) == n.y).collect()
            }
        };
        if now.iter().zip(&self.lit).any(|(a, b)| *a && !*b) {
            fx.sfx.push("lock");
        }
        self.lit = now;
    }

    fn fire(&mut self, fx: &mut Fx) {
        if self.phase != Phase::Aim {
            return;
        }
        if let Kind::Field { k, .. } = &self.levels[self.idx].kind
            && self.picked.len() != *k
        {
            fx.sfx.push("error");
            return;
        }
        self.trail.clear();
        self.phase = Phase::Flying { x: -HALF_W, result: None };
        fx.sfx.push("launch");
    }

    /// Would a disc fired now hit the core?
    fn aim_true(&self) -> bool {
        match &self.levels[self.idx].kind {
            Kind::Real { .. } => (self.curve(0.0) - self.secret()).abs() <= HIT,
            Kind::Field { k, secret, .. } => self.picked.len() == *k && lagrange_mod(&self.picked_pts(), 0) == *secret,
        }
    }

    fn disc_pos(&self, x: f32) -> (f32, f32) {
        match &self.levels[self.idx].kind {
            Kind::Real { .. } => (x, self.curve(x)),
            Kind::Field { .. } => {
                // the disc hops from dot to dot, x = 30 down to 0
                let pts = self.picked_pts();
                let k = ((x + HALF_W) / (2.0 * HALF_W)).clamp(0.0, 1.0);
                let fx = 30.0 * (1.0 - k);
                let (a, b) = (fx.floor() as i64, (fx.ceil() as i64).min(30));
                let (pa, pb) = (node_pos(a, lagrange_mod(&pts, a)), node_pos(b, lagrange_mod(&pts, b)));
                let u = fx - fx.floor();
                (pa.0 + (pb.0 - pa.0) * u, pa.1 + (pb.1 - pa.1) * u)
            }
        }
    }
}

impl Dive for Interceptor {
    fn id(&self) -> &'static str {
        "interceptor"
    }

    fn title(&self) -> &'static str {
        "INTERCEPTOR"
    }

    fn done_flag(&self) -> &'static str {
        DIVE_INTERCEPTOR
    }

    fn level(&self) -> (usize, usize, String) {
        (self.idx + 1, self.levels.len(), self.levels[self.idx].name.to_string())
    }

    fn brief(&self) -> String {
        let l = &self.levels[self.idx];
        if let Kind::Real { shares, .. } = &l.kind {
            let online = shares.iter().filter(|s| self.time >= s.at).count();
            if online < shares.len() {
                return format!("{} ({} of {} keepers online)", l.brief, online, shares.len());
            } else if shares.iter().any(|s| s.at > 0.0) {
                return "The third keeper is online. Now there is only one curve.".to_string();
            }
        }
        l.brief.to_string()
    }

    fn trace(&self) -> f32 {
        self.trace
    }

    fn field(&self) -> (f32, f32) {
        FIELD
    }

    fn status(&self) -> Status {
        if self.cleared { Status::Cleared } else { Status::Playing }
    }

    fn update(&mut self, dt: f32, input: &Input, fx: &mut Fx) {
        self.t += dt;
        if self.cleared {
            return;
        }
        match self.phase.clone() {
            Phase::Intro(age) => {
                self.phase = if age > 1.3 || input.fire || input.pressed { Phase::Aim } else { Phase::Intro(age + dt) };
                return;
            }
            Phase::Boom(age) => {
                if age > 1.9 {
                    if self.idx + 1 == self.levels.len() {
                        self.cleared = true;
                        fx.sfx.push("success");
                    } else {
                        self.idx += 1;
                        self.load();
                    }
                } else {
                    self.phase = Phase::Boom(age + dt);
                }
                return;
            }
            Phase::Traced(age) => {
                if age > 1.4 {
                    self.load();
                } else {
                    self.phase = Phase::Traced(age + dt);
                }
                return;
            }
            Phase::Flying { x, result } => {
                let nx = x + DISC_SPEED * dt * if matches!(self.levels[self.idx].kind, Kind::Field { .. }) { 0.75 } else { 1.0 };
                let (px, py) = self.disc_pos(nx);
                self.trail.push((px, py));
                if self.trail.len() > 46 {
                    self.trail.remove(0);
                }
                let column = if matches!(self.levels[self.idx].kind, Kind::Field { .. }) { HALF_W - 0.01 } else { 0.0 };
                let mut result = result;
                if result.is_none() && nx >= column {
                    let hit = self.aim_true();
                    result = Some(hit);
                    self.last_shot = Some(self.disc_pos(column));
                    if hit {
                        fx.sfx.push("boom");
                        fx.shake = 0.5;
                        self.phase = Phase::Boom(0.0);
                        return;
                    }
                    fx.sfx.push("miss");
                    self.misses += 1;
                    self.total_misses += 1;
                    self.trace += MISS_TRACE;
                }
                if nx >= HALF_W {
                    self.phase = Phase::Aim;
                    self.trail.clear();
                } else {
                    self.phase = Phase::Flying { x: nx, result };
                }
                return;
            }
            Phase::Aim => {}
        }
        self.time += dt;
        self.trace += self.levels[self.idx].rate * dt;
        if self.trace >= 100.0 {
            self.trace = 100.0;
            self.phase = Phase::Traced(0.0);
            fx.sfx.push("alarm");
            fx.shake = 0.4;
            return;
        }
        let was_online: usize = match &self.levels[self.idx].kind {
            Kind::Real { shares, .. } => shares.iter().filter(|s| self.time - dt >= s.at).count(),
            _ => 0,
        };
        match self.levels[self.idx].kind.clone() {
            Kind::Real { handles, shares } => {
                if shares.iter().filter(|s| self.time >= s.at).count() > was_online {
                    fx.sfx.push("pickup");
                }
                // pointer: grab the nearest knob and slide it
                if input.pressed
                    && let Some((px, py)) = input.pointer
                {
                    let near = handles.iter().enumerate().map(|(i, h)| (i, (h.0 - px).abs(), (self.ys[i] - py).abs())).filter(|(_, dx, dy)| *dx < 0.9 && *dy < 1.4).min_by(|a, b| (a.1 + a.2).partial_cmp(&(b.1 + b.2)).unwrap());
                    // or anywhere on a rail
                    let rail = handles.iter().enumerate().map(|(i, h)| (i, (h.0 - px).abs())).filter(|(_, dx)| *dx < 0.5).min_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
                    if let Some(i) = near.map(|n| n.0).or(rail.map(|r| r.0)) {
                        self.drag = Some(i);
                        self.sel = i;
                    }
                }
                if let (Some(i), Some((_, py)), true) = (self.drag, input.pointer, input.down) {
                    let step = if input.fine { SNAP / 5.0 } else { SNAP };
                    self.ys[i] = ((py / step).round() * step).clamp(-HALF_H + 0.3, HALF_H - 0.3);
                }
                if input.released || !input.down {
                    self.drag = None;
                }
                // keyboard
                let n = handles.len();
                if input.next || input.right {
                    self.sel = (self.sel + 1) % n;
                }
                if input.left {
                    self.sel = (self.sel + n - 1) % n;
                }
                let step = if input.fine { SNAP / 5.0 } else { SNAP };
                if input.up {
                    self.ys[self.sel] = (self.ys[self.sel] + step).min(HALF_H - 0.3);
                }
                if input.down_key {
                    self.ys[self.sel] = (self.ys[self.sel] - step).max(-HALF_H + 0.3);
                }
            }
            Kind::Field { k, nodes, .. } => {
                if input.pressed
                    && let Some((px, py)) = input.pointer
                {
                    let near = nodes.iter().enumerate().map(|(i, n)| (i, node_pos(n.x, n.y))).map(|(i, (x, y))| (i, ((x - px).powi(2) + (y - py).powi(2)).sqrt())).filter(|(_, d)| *d < 0.5).min_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
                    if let Some((i, _)) = near {
                        if let Some(at) = self.picked.iter().position(|p| *p == i) {
                            self.picked.remove(at);
                        } else {
                            if self.picked.len() == k {
                                self.picked.remove(0);
                            }
                            self.picked.push(i);
                        }
                        fx.sfx.push("blip");
                    }
                }
                let n = nodes.len();
                if input.next || input.right {
                    self.sel = (self.sel + 1) % n;
                }
                if input.left {
                    self.sel = (self.sel + n - 1) % n;
                }
                if input.up || input.down_key {
                    let i = self.sel;
                    if let Some(at) = self.picked.iter().position(|p| *p == i) {
                        self.picked.remove(at);
                    } else {
                        if self.picked.len() == k {
                            self.picked.remove(0);
                        }
                        self.picked.push(i);
                    }
                    fx.sfx.push("blip");
                }
            }
        }
        self.relight(fx);
        // the launcher: Space, or a click on the pad
        let pad = input.pressed && input.pointer.is_some_and(|(x, y)| x < -HALF_W + 1.6 && (y - self.disc_pos(-HALF_W).1).abs() < 1.2 && self.drag.is_none() && matches!(self.levels[self.idx].kind, Kind::Real { .. }));
        let pad_field = input.pressed && input.pointer.is_some_and(|(x, y)| x > HALF_W - 2.6 && y < -HALF_H + 1.4) && matches!(self.levels[self.idx].kind, Kind::Field { .. });
        if input.fire || pad || pad_field {
            self.fire(fx);
        }
    }

    fn draw(&self, p: &egui::Painter, v: &View) {
        grid(p, v, FIELD, 1.0);
        let def = &self.levels[self.idx];
        let flying = matches!(self.phase, Phase::Flying { .. });
        let boom = if let Phase::Boom(a) = self.phase { Some(a) } else { None };
        match &def.kind {
            Kind::Real { handles, shares } => {
                // axes
                p.line_segment([v.at(-HALF_W, 0.0), v.at(HALF_W, 0.0)], Stroke::new(1.0, NEON_CYAN.gamma_multiply(0.25)));
                // the vault column: where the core could be, given what is online
                let online: Vec<&Share> = shares.iter().filter(|s| self.time >= s.at).collect();
                let pinned = online.len() > handles.len() - 1;
                let shimmer = 0.12 + 0.05 * (v.t * 3.0).sin();
                if boom.is_none() {
                    let band = if pinned { 0.5 } else { HALF_H };
                    let mid = if pinned && self.lit.iter().zip(shares).filter(|(l, s)| **l && self.time >= s.at).count() >= handles.len() { self.curve(0.0) } else { 0.0 };
                    let (lo, hi) = if pinned && mid != 0.0 { (mid - band, mid + band) } else { (-HALF_H, HALF_H) };
                    p.rect_filled(egui::Rect::from_two_pos(v.at(-0.35, hi), v.at(0.35, lo)), 3.0, NEON_PINK.gamma_multiply(shimmer));
                    glow_line(p, v.at(0.0, -HALF_H), v.at(0.0, HALF_H), NEON_PINK.gamma_multiply(0.5), 1.0);
                    p.text(v.at(0.0, HALF_H + 0.25), Align2::CENTER_BOTTOM, "x = 0 · the core is on this line", mono(11.0), NEON_PINK.gamma_multiply(0.8));
                }
                // your curve
                let mut seg: Vec<Pos2> = Vec::new();
                let c = if self.aim_true() && self.lit.iter().filter(|l| **l).count() >= handles.len() { NEON_GREEN } else { NEON_GOLD };
                for i in 0..=160 {
                    let x = -HALF_W + i as f32 * 0.125;
                    let y = self.curve(x);
                    if y.abs() <= HALF_H + 0.4 {
                        seg.push(v.at(x, y));
                    } else {
                        glow_path(p, &seg, c, 2.0);
                        seg.clear();
                    }
                }
                glow_path(p, &seg, c, 2.0);
                // rails and knobs
                for (i, (hx, _)) in handles.iter().enumerate() {
                    let on = self.sel == i || self.drag == Some(i);
                    p.line_segment([v.at(*hx, -HALF_H), v.at(*hx, HALF_H)], Stroke::new(1.0, NEON_GOLD.gamma_multiply(if on { 0.35 } else { 0.14 })));
                    let at = v.at(*hx, self.ys[i]);
                    glow_ring(p, at, v.len(0.34), NEON_GOLD, if on { 3.0 } else { 2.0 });
                    p.circle_filled(at, v.len(0.16), NEON_GOLD);
                    p.text(at + Vec2::new(0.0, -v.len(0.62)), Align2::CENTER_BOTTOM, format!("{:+.2}", self.ys[i]), mono(10.5), NEON_GOLD.gamma_multiply(0.8));
                }
                // shares
                for (i, s) in shares.iter().enumerate() {
                    if self.time < s.at {
                        continue;
                    }
                    let at = v.at(s.x, s.y);
                    let lit = self.lit[i];
                    let fresh = s.at > 0.0 && self.time - s.at < 1.0;
                    // how far the curve is from this share
                    let cy = self.curve(s.x);
                    if !lit && cy.abs() <= HALF_H + 0.4 {
                        p.line_segment([at, v.at(s.x, cy)], Stroke::new(1.0, NEON_CYAN.gamma_multiply(0.35)));
                    }
                    let c = if lit { NEON_GREEN } else { NEON_CYAN };
                    diamond(p, at, v.len(if lit { 0.34 } else { 0.28 }), c, 2.0, lit);
                    if lit {
                        glow_ring(p, at, v.len(0.5 + 0.08 * (v.t * 6.0).sin()), NEON_GREEN, 1.0);
                    }
                    if fresh {
                        glow_ring(p, at, v.len(0.4 + (self.time - s.at) * 1.6), NEON_CYAN.gamma_multiply(1.0 - (self.time - s.at)), 2.0);
                    }
                    p.text(at + Vec2::new(v.len(0.45), -v.len(0.3)), Align2::LEFT_BOTTOM, format!("S{}", i + 1), mono(11.0), c);
                }
                // the launcher
                let (lx, ly) = self.disc_pos(-HALF_W);
                if !flying && boom.is_none() && ly.abs() <= HALF_H {
                    let at = v.at(lx + 0.5, ly);
                    glow_ring(p, at, v.len(0.42), NEON_PINK, 2.0);
                    p.text(at, Align2::CENTER_CENTER, "▶", mono(13.0), NEON_PINK);
                    p.text(at + Vec2::new(0.0, v.len(0.75)), Align2::CENTER_TOP, "SPACE", mono(10.0), NEON_PINK.gamma_multiply(0.8));
                }
            }
            Kind::Field { k, nodes, .. } => {
                // the lattice
                for x in 0..P {
                    for y in 0..P {
                        let (fx, fy) = node_pos(x, y);
                        p.circle_filled(v.at(fx, fy), 1.0, NEON_CYAN.gamma_multiply(if x == 0 { 0.5 } else { 0.16 }));
                    }
                }
                let (cx, _) = node_pos(0, 0);
                if boom.is_none() {
                    p.rect_filled(egui::Rect::from_two_pos(v.at(cx - 0.22, node_pos(0, 30).1 + 0.2), v.at(cx + 0.22, node_pos(0, 0).1 - 0.2)), 3.0, NEON_PINK.gamma_multiply(0.14 + 0.05 * (v.t * 3.0).sin()));
                    p.text(v.at(cx, node_pos(0, 30).1 + 0.45), Align2::CENTER_BOTTOM, "x = 0", mono(11.0), NEON_PINK.gamma_multiply(0.8));
                }
                p.text(v.at(node_pos(30, 0).0, node_pos(0, 0).1 - 0.55), Align2::RIGHT_TOP, "all arithmetic mod 31", mono(11.0), NEON_CYAN.gamma_multiply(0.6));
                // your curve: 31 scattered points
                let pts = self.picked_pts();
                if pts.len() == *k {
                    let good = self.lit.iter().filter(|l| **l).count() > *k;
                    let c = if good { NEON_GREEN } else { NEON_GOLD };
                    for x in 0..P {
                        let (fx, fy) = node_pos(x, lagrange_mod(&pts, x));
                        glow_dot(p, v.at(fx, fy), if x == 0 { 4.0 } else { 2.5 }, if x == 0 { NEON_PINK } else { c });
                    }
                }
                for (i, n) in nodes.iter().enumerate() {
                    let (fx, fy) = node_pos(n.x, n.y);
                    let at = v.at(fx, fy);
                    let (picked, lit) = (self.picked.contains(&i), self.lit[i]);
                    let c = if lit { NEON_GREEN } else { NEON_CYAN };
                    diamond(p, at, v.len(0.3), c, 2.0, lit);
                    if picked {
                        glow_ring(p, at, v.len(0.5), NEON_GOLD, 2.0);
                    }
                    if self.sel == i {
                        p.circle_stroke(at, v.len(0.66), Stroke::new(1.0, NEON_VIOLET.gamma_multiply(0.6)));
                    }
                    p.text(at + Vec2::new(v.len(0.42), -v.len(0.28)), Align2::LEFT_BOTTOM, format!("({}, {})", n.x, n.y), mono(10.5), c);
                }
                // the fire pad
                if !flying && boom.is_none() {
                    let at = v.at(HALF_W - 1.3, -HALF_H + 0.6);
                    let ready = self.picked.len() == *k;
                    let c = if ready { NEON_PINK } else { NEON_PINK.gamma_multiply(0.35) };
                    glow_ring(p, at, v.len(0.5), c, 2.0);
                    p.text(at, Align2::CENTER_CENTER, "▶", mono(14.0), c);
                    p.text(at + Vec2::new(-v.len(0.7), 0.0), Align2::RIGHT_CENTER, if ready { "SPACE: fire" } else { "pick three nodes" }, mono(11.0), c);
                }
            }
        }
        // the disc and its trail
        if let Phase::Flying { x, .. } = self.phase {
            let pts: Vec<Pos2> = self.trail.iter().map(|(x, y)| v.at(*x, *y)).collect();
            glow_path(p, &pts, NEON_PINK, 2.5);
            let (dx, dy) = self.disc_pos(x);
            let at = v.at(dx, dy);
            glow_ring(p, at, v.len(0.3), NEON_PINK, 2.5);
            for i in 0..4 {
                let a = v.t * 14.0 + i as f32 * std::f32::consts::FRAC_PI_2;
                p.line_segment([at, at + Vec2::new(a.cos(), a.sin()) * v.len(0.28)], Stroke::new(1.5, NEON_PINK));
            }
            p.circle_filled(at, v.len(0.09), egui::Color32::WHITE);
        }
        // where the last disc crossed and found nothing
        if let (Some((x, y)), None) = (self.last_shot, boom)
            && !flying
            && self.misses > 0
        {
            let at = v.at(x, y);
            p.line_segment([at + Vec2::new(-5.0, -5.0), at + Vec2::new(5.0, 5.0)], Stroke::new(1.5, NEON_RED.gamma_multiply(0.7)));
            p.line_segment([at + Vec2::new(-5.0, 5.0), at + Vec2::new(5.0, -5.0)], Stroke::new(1.5, NEON_RED.gamma_multiply(0.7)));
        }
        // the core, uncloaked as it dies
        if let Some(age) = boom {
            let s = self.secret();
            let (cx, cy) = match &def.kind {
                Kind::Real { .. } => (0.0, s),
                Kind::Field { secret, .. } => node_pos(0, *secret),
            };
            let at = v.at(cx, cy);
            sparks(p, at, age, NEON_PINK, 26, v.len(3.2), self.idx as u32 + 1);
            sparks(p, at, age * 0.8, NEON_GOLD, 18, v.len(2.2), self.idx as u32 + 77);
            glow_ring(p, at, v.len(0.3 + age * 2.6), NEON_PINK.gamma_multiply((1.0 - age / 1.9).max(0.0)), 3.0);
            let text = match &def.kind {
                Kind::Real { .. } => format!("SECRET  f(0) = {s:+.2}"),
                Kind::Field { secret, .. } => format!("SECRET  f(0) = {secret}  (mod 31)"),
            };
            p.text(v.at(0.0, HALF_H - 0.8), Align2::CENTER_CENTER, text, retro(34.0), NEON_GREEN);
        }
        match self.phase {
            Phase::Intro(age) => {
                let a = (1.0 - (age - 0.9).max(0.0) / 0.4).clamp(0.0, 1.0);
                p.text(v.at(0.0, 0.6), Align2::CENTER_CENTER, format!("{} / {}", self.idx + 1, self.levels.len()), mono(14.0), NEON_CYAN.gamma_multiply(a));
                p.text(v.at(0.0, -0.3), Align2::CENTER_CENTER, def.name, retro(54.0), NEON_GOLD.gamma_multiply(a));
            }
            Phase::Traced(_) => {
                p.rect_filled(v.rect, 0.0, NEON_RED.gamma_multiply(0.12));
                p.text(v.at(0.0, 0.0), Align2::CENTER_CENTER, "TRACED · THE ICE THROWS YOU BACK", retro(40.0), NEON_RED);
            }
            _ => {}
        }
        if self.cleared {
            p.text(v.at(0.0, 0.4), Align2::CENTER_CENTER, "VAULT OPEN", retro(64.0), NEON_GREEN);
            p.text(v.at(0.0, -0.8), Align2::CENTER_CENTER, format!("eight cores, {} wasted discs", self.total_misses), mono(14.0), NEON_CYAN);
        }
    }

    fn bot(&mut self, dt: f32) -> Input {
        let mut i = Input::default();
        match self.phase {
            Phase::Intro(_) => {
                i.fire = true;
                return i;
            }
            Phase::Aim => {}
            _ => return i,
        }
        self.bot_wait -= dt;
        if self.bot_wait > 0.0 {
            return i;
        }
        match self.levels[self.idx].kind.clone() {
            Kind::Real { handles, shares } => {
                // wait for every keeper, then slide each knob home
                if shares.iter().any(|s| self.time < s.at) {
                    return i;
                }
                if let Some((n, h)) = handles.iter().enumerate().find(|(n, h)| (self.ys[*n] - h.1).abs() > 0.01) {
                    let y = self.ys[n];
                    let step = (h.1 - y).clamp(-9.0 * dt, 9.0 * dt);
                    let ny = if (h.1 - y).abs() <= SNAP / 2.0 { h.1 } else { y + step };
                    i.pointer = Some((h.0, ny));
                    i.pressed = self.drag != Some(n);
                    i.down = true;
                    // the snap grid would hold a slow hand in place: move in whole steps
                    if (ny - y).abs() < SNAP && (h.1 - y).abs() > SNAP / 2.0 {
                        i.pointer = Some((h.0, y + SNAP * (h.1 - y).signum()));
                        self.bot_wait = 0.03;
                    }
                    return i;
                }
                if self.drag.is_some() {
                    i.released = true;
                    self.bot_wait = 0.25;
                    return i;
                }
                i.fire = true;
            }
            Kind::Field { k, nodes, .. } => {
                let want: Vec<usize> = nodes.iter().enumerate().filter(|(_, n)| n.honest).map(|(i, _)| i).take(k).collect();
                if let Some(extra) = self.picked.iter().find(|p| !want.contains(p)) {
                    let (x, y) = node_pos(nodes[*extra].x, nodes[*extra].y);
                    i.pointer = Some((x, y));
                    i.pressed = true;
                    self.bot_wait = 0.35;
                    return i;
                }
                if let Some(next) = want.iter().find(|w| !self.picked.contains(w)) {
                    let (x, y) = node_pos(nodes[*next].x, nodes[*next].y);
                    i.pointer = Some((x, y));
                    i.pressed = true;
                    self.bot_wait = 0.35;
                    return i;
                }
                i.fire = true;
            }
        }
        self.bot_wait = 0.4;
        i
    }
}

#[cfg(test)]
mod tests {
    use super::super::autoplay;
    use super::*;

    #[test]
    fn interpolation_recovers_the_secret_over_the_reals_and_mod_31() {
        let f = |x: f32| 0.5 * x * x - 2.0 * x + 3.0;
        let pts = [(-4.0, f(-4.0)), (1.0, f(1.0)), (6.0, f(6.0))];
        assert!((lagrange(&pts, 0.0) - 3.0).abs() < 1e-4);
        let g = |x: i64| (7 * x * x + 3 * x + 19).rem_euclid(P);
        let pts = [(4, g(4)), (11, g(11)), (29, g(29))];
        assert_eq!(lagrange_mod(&pts, 0), 19);
        for x in 0..P {
            assert_eq!(lagrange_mod(&pts, x), g(x));
        }
    }

    #[test]
    fn two_shares_of_three_leave_every_secret_possible() {
        // mod 31: for each candidate secret there is exactly one parabola through two shares and it
        let g = |x: i64| (5 * x * x + 2 * x + 8).rem_euclid(P);
        let two = [(3, g(3)), (17, g(17))];
        let mut seen = std::collections::HashSet::new();
        for s in 0..P {
            let pts = [(0, s), two[0], two[1]];
            // a third point pinned at x = 0 gives a valid polynomial through both shares
            assert_eq!(lagrange_mod(&pts, 3), g(3));
            assert_eq!(lagrange_mod(&pts, 17), g(17));
            seen.insert(lagrange_mod(&pts, 9));
        }
        assert_eq!(seen.len(), P as usize, "all 31 secrets are consistent, and they all differ elsewhere");
    }

    #[test]
    fn levels_are_fair() {
        for seed in [&b"a"[..], b"b", b"c", b"d"] {
            for (n, l) in levels(seed).iter().enumerate() {
                match &l.kind {
                    Kind::Real { handles, shares } => {
                        let honest: Vec<&Share> = shares.iter().filter(|s| s.honest).collect();
                        assert!(honest.len() >= handles.len(), "level {n}");
                        for s in &honest {
                            assert!((lagrange(handles, s.x) - s.y).abs() < 1e-3);
                            assert!(s.y.abs() < HALF_H, "level {n}: share off screen");
                            assert!(handles.iter().all(|h| (h.0 - s.x).abs() > 0.5), "level {n}: a share sits on a rail");
                        }
                        for s in shares.iter().filter(|s| !s.honest) {
                            assert!((lagrange(handles, s.x) - s.y).abs() > 4.0 * TOL, "level {n}: the forgery is too close to the truth");
                            assert!(s.y.abs() < HALF_H);
                        }
                        assert!(lagrange(handles, 0.0).abs() < HALF_H - 0.5);
                    }
                    Kind::Field { k, nodes, secret } => {
                        let pts: Vec<(i64, i64)> = nodes.iter().filter(|n| n.honest).take(*k).map(|n| (n.x, n.y)).collect();
                        assert_eq!(lagrange_mod(&pts, 0), *secret);
                        assert!(nodes.iter().filter(|n| n.honest).count() > *k, "level {n}: spare honest nodes show which three are right");
                    }
                }
            }
        }
    }

    #[test]
    fn the_bot_clears_every_level_and_a_blind_shot_misses() {
        let mut d = Interceptor::new(b"test");
        let mut fx = Fx::default();
        // skip the name card, fire with the handles flat
        for _ in 0..120 {
            d.update(1.0 / 60.0, &Input::default(), &mut fx);
        }
        let before = d.trace();
        d.update(1.0 / 60.0, &Input { fire: true, ..Default::default() }, &mut fx);
        for _ in 0..200 {
            d.update(1.0 / 60.0, &Input::default(), &mut fx);
        }
        assert_eq!(d.total_misses, 1);
        assert!(d.trace() > before + MISS_TRACE - 1.0);
        assert_eq!(d.level().0, 1);
        let secs = autoplay(&mut d, 400.0).expect("the bot finishes");
        assert!(secs < 240.0, "took {secs} s");
        assert_eq!(d.status(), Status::Cleared);
        assert_eq!(d.total_misses, 1, "the bot wastes nothing");
    }

    #[test]
    fn lingering_gets_you_traced_and_the_level_restarts() {
        let mut d = Interceptor::new(b"slow");
        let mut fx = Fx::default();
        let mut traced = false;
        for _ in 0..(160 * 60) {
            d.update(1.0 / 60.0, &Input::default(), &mut fx);
            if matches!(d.phase, Phase::Traced(_)) {
                traced = true;
            }
        }
        assert!(traced);
        assert!(fx.sfx.contains(&"alarm"));
        assert_eq!(d.level().0, 1);
        assert!(d.trace() < 60.0, "trace was reset when the level restarted");
    }
}
