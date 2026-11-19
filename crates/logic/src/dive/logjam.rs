//! LOGJAM: discrete logarithms in a group small enough to take apart.
//!
//! The tower line ran Diffie-Hellman in an export-grade group: a 33-bit
//! prime, a subgroup of 31 bits. Its numbers are g^x for secrets nobody
//! sent. Level one counts: crank g round until it lands on the target, and
//! the number of turns is the secret. Level two is too big to count before
//! the trace catches you, so you build a table of the first m powers (baby
//! steps) and hop from the target by g^-m (giant steps) until a hop lands in
//! the table: x = i*m + j, in about 2*sqrt(q) work, and you choose m. Level
//! three is the tower itself: two numbers on one table. The second falls in
//! a fraction of the time, because the expensive part never depended on the
//! number. That is the Logjam paper's point (Adrian et al., 2015): when the
//! costly work depends only on the group, a group everybody shares falls
//! once, for all of them.
//!
//! Everything here is the real computation ([`cpl_crypto::dh::Bsgs`]), run a
//! slice per frame at a fixed rate so the clock means something.

use cpl_crypto::dh::{Bsgs, BsgsPhase, Group, TOY_GROUP, WEAK_GROUP};
use cpl_crypto::drbg::Drbg;
use egui::{Align2, Color32, Rect, Stroke, Vec2};

use super::{DEEP, Dive, Fx, Input, NEON_CYAN, NEON_GOLD, NEON_GREEN, NEON_PINK, NEON_RED, NEON_VIOLET, Status, View, glow_dot, glow_line, glow_ring, grid};
use crate::lines::exchange::DIVE_LOGJAM;
use crate::ui::theme::{mono, retro};

const FIELD: (f32, f32) = (24.0, 14.0);

/// A safe prime of 16 bits: big enough that counting is too slow.
pub const MID_GROUP: Group = Group { name: "mid-16", p: 40_127, q: 20_063, g: 4 };

#[derive(Clone, Debug)]
struct Level {
    name: &'static str,
    group: Group,
    targets: Vec<u64>,
    /// steps of work per second
    rate: f32,
    /// trace per second
    clock: f32,
    /// the biggest table the cellar's memory holds
    cap: u64,
    /// level one only turns the crank
    count: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    Intro(f32),
    Setup,
    Run,
    Found(f32),
    Done(f32),
    Traced(f32),
}

pub struct Logjam {
    levels: Vec<Level>,
    idx: usize,
    phase: Phase,
    /// table size, as a power of two
    log_m: u32,
    run: Option<Bsgs>,
    /// level one: turns so far and where the light is
    turns: u64,
    cursor: u64,
    /// which target of the level is being broken
    target: usize,
    found: Vec<u64>,
    /// recent positions (for the chords that show the jumps)
    trail: Vec<u64>,
    budget: f32,
    trace: f32,
    t: f32,
    cleared: bool,
    bot_wait: f32,
}

fn element(rng: &mut Drbg, g: &Group, lo: f64, hi: f64) -> u64 {
    let span = ((hi - lo) * g.q as f64) as u64;
    let x = (lo * g.q as f64) as u64 + u64::from_be_bytes(rng.array::<8>()) % span.max(1);
    g.public(x)
}

impl Logjam {
    /// `tower`: the two numbers off the tower line (the tape's and the next call's).
    pub fn new(seed: &[u8], tower: Vec<u64>) -> Self {
        let mut rng = Drbg::new(seed, "dive/logjam");
        let tower = if tower.is_empty() { vec![element(&mut rng, &WEAK_GROUP, 0.1, 0.9), element(&mut rng, &WEAK_GROUP, 0.1, 0.9)] } else { tower };
        let levels = vec![
            Level { name: "COUNT", group: TOY_GROUP, targets: vec![element(&mut rng, &TOY_GROUP, 0.3, 0.9)], rate: 24.0, clock: 2.2, cap: 1, count: true },
            Level { name: "TOO FAR TO COUNT", group: MID_GROUP, targets: vec![element(&mut rng, &MID_GROUP, 0.72, 0.95)], rate: 30.0, clock: 2.5, cap: 4096, count: false },
            Level { name: "THE TOWER", group: WEAK_GROUP, targets: tower, rate: 4000.0, clock: 1.45, cap: 1 << 16, count: false },
        ];
        let mut d = Self { levels, idx: 0, phase: Phase::Intro(0.0), log_m: 0, run: None, turns: 0, cursor: 1, target: 0, found: Vec::new(), trail: Vec::new(), budget: 0.0, trace: 0.0, t: 0.0, cleared: false, bot_wait: 0.0 };
        d.load();
        d
    }

    fn level_ref(&self) -> &Level {
        &self.levels[self.idx]
    }

    fn load(&mut self) {
        self.phase = Phase::Intro(0.0);
        self.run = None;
        self.turns = 0;
        self.cursor = 1;
        self.target = 0;
        self.found.clear();
        self.trail.clear();
        self.trace = 0.0;
        self.budget = 0.0;
        if self.log_m > self.max_log_m() {
            self.log_m = self.max_log_m();
        }
    }

    fn max_log_m(&self) -> u32 {
        63 - self.level_ref().cap.leading_zeros()
    }

    fn m(&self) -> u64 {
        1u64 << self.log_m
    }

    /// The table size a player who knows the trick would pick.
    fn best_log_m(&self) -> u32 {
        let l = self.level_ref();
        let want = ((l.targets.len() as f64) * l.group.q as f64).sqrt();
        let mut best = 0;
        for k in 0..=self.max_log_m() {
            if ((1u64 << k) as f64 - want).abs() < ((1u64 << best) as f64 - want).abs() {
                best = k;
            }
        }
        best
    }

    /// Worst-case steps for this table: fill it, then hop through every target.
    fn work(&self, m: u64) -> u64 {
        let l = self.level_ref();
        m + l.group.q.div_ceil(m) * l.targets.len() as u64
    }

    fn start(&mut self, fx: &mut Fx) {
        let l = self.level_ref().clone();
        if !l.count {
            self.run = Some(Bsgs::new(l.group, l.targets[self.target], self.m()));
        }
        self.phase = Phase::Run;
        fx.sfx.push("launch");
    }

    fn advance(&mut self, dt: f32, fx: &mut Fx) {
        let l = self.level_ref().clone();
        self.budget += l.rate * dt;
        let steps = self.budget.floor() as u64;
        self.budget -= steps as f32;
        if steps == 0 {
            return;
        }
        let goal = l.targets[self.target];
        if l.count {
            for _ in 0..steps {
                self.cursor = cpl_crypto::bytes::mul_mod_u64(self.cursor, l.group.g, l.group.p);
                self.turns += 1;
                self.push_trail(self.cursor);
                if self.cursor == goal {
                    self.found.push(self.turns);
                    self.phase = Phase::Found(0.0);
                    fx.sfx.push("unlock");
                    return;
                }
            }
            if steps > 0 {
                fx.sfx.push("tick");
            }
            return;
        }
        let (done, at, result) = {
            let Some(run) = self.run.as_mut() else { return };
            let done = run.step(steps);
            (done, run_position(run), run.result)
        };
        self.push_trail(at);
        if done {
            match result {
                Some(x) => {
                    self.found.push(x);
                    self.phase = Phase::Found(0.0);
                    fx.sfx.push("unlock");
                }
                None => {
                    // a table too small for this group: it can only fail
                    self.phase = Phase::Traced(0.0);
                    fx.sfx.push("alarm");
                }
            }
        }
    }

    fn push_trail(&mut self, v: u64) {
        self.trail.push(v);
        if self.trail.len() > 14 {
            self.trail.remove(0);
        }
    }

    /// Angle of an element on the ring (by value: exponentiation scrambles them).
    fn angle(&self, v: u64) -> f32 {
        let p = self.level_ref().group.p as f64;
        (std::f64::consts::TAU * (v as f64 / p)) as f32 + std::f32::consts::FRAC_PI_2
    }

    fn ring_at(&self, v: u64, r: f32) -> (f32, f32) {
        let a = self.angle(v);
        (RING.0 + r * a.cos(), RING.1 + r * a.sin())
    }
}

/// Where a running search is: the last baby step, or the current hop.
fn run_position(run: &Bsgs) -> u64 {
    run.position()
}

const RING: (f32, f32) = (-5.6, -0.2);
const R: f32 = 5.4;

impl Dive for Logjam {
    fn id(&self) -> &'static str {
        "logjam"
    }

    fn title(&self) -> &'static str {
        "LOGJAM"
    }

    fn done_flag(&self) -> &'static str {
        DIVE_LOGJAM
    }

    fn level(&self) -> (usize, usize, String) {
        (self.idx + 1, self.levels.len(), self.levels[self.idx].name.to_string())
    }

    fn brief(&self) -> String {
        match self.idx {
            0 => "Each turn of the crank multiplies by g, and the light jumps somewhere you cannot predict. Crank until it lands on the target: the number of turns is the secret. SPACE starts the crank.".into(),
            1 => "Counting this ring would take minutes. Build a table of the first m powers, then hop back from the target m at a time until a hop lands in the table. ← → set m, SPACE runs it.".into(),
            _ => "The tower's numbers, in the export group. The cellar holds 65,536 drawers. Break the desk's number, then the next call's on the same table.".into(),
        }
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
        match self.phase {
            Phase::Intro(age) => {
                self.phase = if age > 1.3 || input.fire || input.pressed { Phase::Setup } else { Phase::Intro(age + dt) };
                return;
            }
            Phase::Found(age) => {
                if age > 1.6 {
                    let n = self.level_ref().targets.len();
                    if self.target + 1 < n {
                        // the same table, a new number: only the hops are left to do
                        self.target += 1;
                        let y = self.level_ref().targets[self.target];
                        if let Some(run) = self.run.as_mut() {
                            run.retarget(y);
                        }
                        self.trail.clear();
                        self.phase = Phase::Run;
                    } else {
                        self.phase = Phase::Done(0.0);
                        fx.sfx.push("success");
                    }
                } else {
                    self.phase = Phase::Found(age + dt);
                }
                return;
            }
            Phase::Done(age) => {
                if age > 2.4 {
                    if self.idx + 1 == self.levels.len() {
                        self.cleared = true;
                    } else {
                        self.idx += 1;
                        self.load();
                    }
                } else {
                    self.phase = Phase::Done(age + dt);
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
            Phase::Setup | Phase::Run => {}
        }
        if self.cleared {
            return;
        }
        self.trace += self.level_ref().clock * dt;
        if self.trace >= 100.0 {
            self.phase = Phase::Traced(0.0);
            fx.sfx.push("alarm");
            fx.shake = 0.4;
            return;
        }
        if self.phase == Phase::Setup {
            let count = self.level_ref().count;
            if !count {
                let mut clicked_minus = false;
                let mut clicked_plus = false;
                let mut clicked_run = false;
                if input.pressed
                    && let Some((px, py)) = input.pointer
                {
                    clicked_minus = (px - 3.2).abs() < 0.8 && (py + 4.6).abs() < 0.6;
                    clicked_plus = (px - 9.6).abs() < 0.8 && (py + 4.6).abs() < 0.6;
                    clicked_run = (px - 6.4).abs() < 2.4 && (py + 5.8).abs() < 0.55;
                }
                if (input.left || clicked_minus) && self.log_m > 0 {
                    self.log_m -= 1;
                    fx.sfx.push("tick");
                }
                if (input.right || clicked_plus) && self.log_m < self.max_log_m() {
                    self.log_m += 1;
                    fx.sfx.push("tick");
                }
                if input.fire || clicked_run {
                    self.start(fx);
                }
            } else if input.fire || input.pressed {
                self.start(fx);
            }
            return;
        }
        self.advance(dt, fx);
    }

    fn draw(&self, p: &egui::Painter, v: &View) {
        grid(p, v, FIELD, 1.0);
        let l = self.level_ref();
        let g = l.group;
        let center = v.at(RING.0, RING.1);
        glow_ring(p, center, v.len(R), NEON_CYAN.gamma_multiply(0.45), 1.5);
        // level one: every element of the little group, faintly
        if l.count {
            let mut e = 1u64;
            for _ in 0..g.q {
                let (x, y) = self.ring_at(e, R);
                p.circle_filled(v.at(x, y), 1.6, NEON_CYAN.gamma_multiply(0.22));
                e = cpl_crypto::bytes::mul_mod_u64(e, g.g, g.p);
            }
        }
        // the table: baby steps g^0 .. g^(m-1), sampled so the ring stays readable
        if let Some(run) = &self.run {
            let filled = run.baby_steps().min(run.m);
            let stride = (filled / 700).max(1);
            let mut j = 0;
            while j < filled {
                let (x, y) = self.ring_at(g.pow(g.g, j), R - 0.35);
                p.circle_filled(v.at(x, y), 2.0, NEON_GREEN.gamma_multiply(0.55));
                j += stride;
            }
        }
        // the target(s)
        for (i, &t) in l.targets.iter().enumerate() {
            let (x, y) = self.ring_at(t, R + 0.55);
            let at = v.at(x, y);
            let done = i < self.found.len();
            let c = if done { NEON_GREEN } else if i == self.target { NEON_GOLD } else { NEON_GOLD.gamma_multiply(0.4) };
            let pulse = 1.0 + 0.25 * (self.t * 5.0).sin();
            glow_ring(p, at, v.len(0.32) * if i == self.target && !done { pulse } else { 1.0 }, c, 2.0);
            p.text(at + Vec2::new(0.0, -v.len(0.62)), Align2::CENTER_BOTTOM, if done { "✓" } else { "?" }, retro(18.0), c);
        }
        // the jumps
        let col = if l.count { NEON_PINK } else if self.run.as_ref().is_some_and(|r| r.phase == BsgsPhase::Baby) { NEON_GREEN } else { NEON_VIOLET };
        for w in self.trail.windows(2) {
            let (a, b) = (self.ring_at(w[0], R - 0.2), self.ring_at(w[1], R - 0.2));
            p.line_segment([v.at(a.0, a.1), v.at(b.0, b.1)], Stroke::new(1.2, col.gamma_multiply(0.35)));
        }
        if let Some(&last) = self.trail.last() {
            let (x, y) = self.ring_at(last, R - 0.2);
            glow_dot(p, v.at(x, y), 4.0, col);
        }
        p.text(center, Align2::CENTER_CENTER, format!("p = {}", g.p), mono(12.0), NEON_CYAN.gamma_multiply(0.6));
        p.text(center + Vec2::new(0.0, v.len(0.7)), Align2::CENTER_CENTER, format!("g = {}", g.g), mono(12.0), NEON_CYAN.gamma_multiply(0.6));

        // --- the machine room --------------------------------------------------------------
        let x0 = 1.6;
        let text = |y: f32, s: String, c: Color32, big: bool| {
            p.text(v.at(x0, y), Align2::LEFT_CENTER, s, if big { retro(22.0) } else { mono(13.0) }, c);
        };
        text(5.6, format!("GROUP OF {} ELEMENTS", fmt(g.q)), NEON_CYAN, true);
        if l.count {
            text(4.4, format!("TURNS  {}", self.turns), NEON_GOLD, true);
            text(3.5, "each turn: light × g (mod p)".into(), NEON_CYAN.gamma_multiply(0.7), false);
            text(2.8, format!("{} turns a second", l.rate as u32), NEON_CYAN.gamma_multiply(0.7), false);
            if self.phase == Phase::Setup {
                let at = v.at(6.4, -5.8);
                p.rect_stroke(Rect::from_center_size(at, Vec2::new(v.len(4.8), v.len(0.9))), 4.0, Stroke::new(1.5, NEON_GREEN), egui::StrokeKind::Inside);
                p.text(at, Align2::CENTER_CENTER, "SPACE · CRANK", retro(20.0), NEON_GREEN);
            }
        } else {
            let m = self.run.as_ref().map(|r| r.m).unwrap_or(self.m());
            let work = self.work(m);
            let secs = work as f32 / l.rate;
            let left = (100.0 - self.trace) / l.clock;
            text(4.4, format!("TABLE  m = {}", fmt(m)), NEON_GREEN, true);
            text(3.6, format!("memory: {} drawers", fmt(l.cap)), NEON_CYAN.gamma_multiply(0.7), false);
            let fits = secs <= left;
            text(2.9, format!("worst case {} steps, {} a second", fmt(work), fmt(l.rate as u64)), NEON_CYAN.gamma_multiply(0.7), false);
            text(2.2, format!("≈ {:.0} s of work; the trace leaves {:.0} s", secs, left.max(0.0)), if fits { NEON_CYAN.gamma_multiply(0.9) } else { NEON_RED }, false);
            if let Some(run) = &self.run {
                let baby = run.baby_steps().min(run.m);
                bar(p, v, 1.2, "BABY STEPS", baby as f32 / run.m as f32, NEON_GREEN);
                bar(p, v, 0.2, "GIANT STEPS", run.giant_fraction(), NEON_VIOLET);
                text(-0.8, format!("hops {} of at most {}", fmt(if run.phase == BsgsPhase::Baby { 0 } else { run.giant_steps() }), fmt(run.giants_total)), NEON_VIOLET.gamma_multiply(0.85), false);
            }
            if self.phase == Phase::Setup {
                for (x, s) in [(3.2, "◀"), (9.6, "▶")] {
                    let at = v.at(x, -4.6);
                    p.rect_stroke(Rect::from_center_size(at, Vec2::splat(v.len(1.1))), 4.0, Stroke::new(1.2, NEON_CYAN.gamma_multiply(0.7)), egui::StrokeKind::Inside);
                    p.text(at, Align2::CENTER_CENTER, s, mono(18.0), NEON_CYAN);
                }
                p.text(v.at(6.4, -4.6), Align2::CENTER_CENTER, format!("m = {}", fmt(self.m())), retro(22.0), NEON_GREEN);
                let at = v.at(6.4, -5.8);
                p.rect_stroke(Rect::from_center_size(at, Vec2::new(v.len(4.8), v.len(0.9))), 4.0, Stroke::new(1.5, if fits { NEON_GREEN } else { NEON_RED }), egui::StrokeKind::Inside);
                p.text(at, Align2::CENTER_CENTER, "SPACE · RUN", retro(20.0), if fits { NEON_GREEN } else { NEON_RED });
            }
        }
        for (i, x) in self.found.iter().enumerate() {
            let what = if l.count { format!("x = {x}: g^{x} is the target") } else { format!("number {}: x = {}", i + 1, fmt(*x)) };
            text(-2.0 - i as f32 * 0.8, what, NEON_GREEN, false);
        }
        if self.found.len() == 1 && l.targets.len() > 1 && self.phase == Phase::Run {
            text(-3.6, "same table: only hops left to do".into(), NEON_GOLD, false);
        }

        match self.phase {
            Phase::Intro(age) => {
                let a = (1.0 - (age - 0.9).max(0.0) / 0.4).clamp(0.0, 1.0);
                p.rect_filled(v.rect, 0.0, DEEP.gamma_multiply(0.8 * a));
                p.text(v.at(0.0, 0.6), Align2::CENTER_CENTER, format!("{} / {}", self.idx + 1, self.levels.len()), mono(14.0), NEON_CYAN.gamma_multiply(a));
                p.text(v.at(0.0, -0.3), Align2::CENTER_CENTER, l.name, retro(54.0), NEON_GOLD.gamma_multiply(a));
            }
            Phase::Found(_) => {
                let what = if l.count { "THE LIGHT LANDED ON THE TARGET" } else { "A HOP LANDED IN THE TABLE · x = i·m + j" };
                glow_line(p, v.at(-11.0, 5.5), v.at(11.0, 5.5), NEON_GREEN.gamma_multiply(0.4), 1.0);
                p.text(v.at(0.0, 6.1), Align2::CENTER_CENTER, what, retro(22.0), NEON_GREEN);
            }
            Phase::Done(_) => {
                let what = match self.idx {
                    0 => "THE SECRET WAS THE COUNT",
                    1 => "FOUND WITHOUT COUNTING",
                    _ => "THE TOWER'S KEY IS YOURS",
                };
                p.text(v.at(0.0, 6.1), Align2::CENTER_CENTER, what, retro(26.0), NEON_GREEN);
            }
            Phase::Traced(_) => {
                p.rect_filled(v.rect, 0.0, NEON_RED.gamma_multiply(0.12));
                p.text(v.at(0.0, 6.1), Align2::CENTER_CENTER, "TRACED · THE CELLAR GOES DARK", retro(30.0), NEON_RED);
            }
            _ => {}
        }
        if self.cleared {
            p.rect_filled(v.rect, 0.0, DEEP.gamma_multiply(0.75));
            p.text(v.at(0.0, 0.4), Align2::CENTER_CENTER, "TOWER READ", retro(64.0), NEON_GREEN);
            p.text(v.at(0.0, -0.8), Align2::CENTER_CENTER, "one table, every number on the group", mono(14.0), NEON_CYAN);
        }
    }

    fn bot(&mut self, dt: f32) -> Input {
        let mut i = Input::default();
        match self.phase {
            Phase::Intro(_) => {
                i.fire = true;
                return i;
            }
            Phase::Setup => {}
            _ => return i,
        }
        self.bot_wait -= dt;
        if self.bot_wait > 0.0 {
            return i;
        }
        self.bot_wait = 0.12;
        if self.level_ref().count {
            i.fire = true;
            return i;
        }
        let best = self.best_log_m();
        if self.log_m < best {
            i.right = true;
        } else if self.log_m > best {
            i.left = true;
        } else {
            i.fire = true;
        }
        i
    }
}

fn bar(p: &egui::Painter, v: &View, y: f32, label: &str, k: f32, c: Color32) {
    let (x0, w) = (1.6, 9.6);
    p.text(v.at(x0, y + 0.35), Align2::LEFT_BOTTOM, label, mono(11.0), c.gamma_multiply(0.8));
    let r = Rect::from_min_max(v.at(x0, y + 0.25), v.at(x0 + w, y - 0.15));
    p.rect_filled(r, 2.0, DEEP);
    p.rect_stroke(r, 2.0, Stroke::new(1.0, c.gamma_multiply(0.4)), egui::StrokeKind::Inside);
    let mut f = r;
    f.set_width(r.width() * k.clamp(0.0, 1.0));
    p.rect_filled(f, 2.0, c.gamma_multiply(0.7));
}

/// 1234567 → "1,234,567".
fn fmt(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::autoplay;
    use super::*;

    #[test]
    fn counting_finds_the_secret_and_the_table_finds_it_faster() {
        let mut d = Logjam::new(b"count", Vec::new());
        let secs = autoplay(&mut d, 400.0).expect("the bot clears LOGJAM");
        assert_eq!(d.status(), Status::Cleared);
        assert!(secs < 200.0, "took {secs}");
    }

    #[test]
    fn the_found_logs_are_real() {
        let g = WEAK_GROUP;
        let xs = [123_456_789u64, 1_987_654_321];
        let targets: Vec<u64> = xs.iter().map(|x| g.public(*x)).collect();
        let mut run = Bsgs::new(g, targets[0], 1 << 16);
        while !run.step(1 << 18) {}
        assert_eq!(run.result, Some(xs[0]));
        let built = run.baby_steps();
        run.retarget(targets[1]);
        while !run.step(1 << 18) {}
        assert_eq!(run.result, Some(xs[1]));
        assert_eq!(run.baby_steps(), built, "the table was not built again");
    }

    #[test]
    fn a_tiny_table_cannot_beat_the_clock_but_a_balanced_one_can() {
        let d = Logjam::new(b"work", Vec::new());
        let mut d = d;
        d.idx = 1;
        d.load();
        let l = d.level_ref().clone();
        let budget = 100.0 / l.clock * l.rate;
        assert!(d.work(1) as f32 > budget, "counting is too slow on level two");
        assert!(d.work(1 << d.best_log_m()) as f32 <= budget, "the balanced table fits");
        d.idx = 2;
        d.load();
        let l = d.level_ref().clone();
        let budget = 100.0 / l.clock * l.rate;
        assert!(d.work(1 << 10) as f32 > budget);
        assert!(d.work(1 << d.best_log_m()) as f32 <= budget);
        assert_eq!(fmt(65536), "65,536");
    }
}
