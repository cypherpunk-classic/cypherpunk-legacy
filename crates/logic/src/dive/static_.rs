//! STATIC: two harbour broadcasts sent on one key.
//!
//! `C1 xor C2 = P1 xor P2`: the key is gone and two messages lie on top of
//! each other. Guess a word of one (a crib), slide it along, and wherever it
//! is right the same span of the other message falls out in clear. That is
//! crib dragging, the way two-time pads have always been broken, on real
//! bytes: nothing here knows the key.

use cpl_crypto::drbg::Drbg;
use cpl_crypto::otp::make_two_time_pad;
use egui::{Align2, Color32, Rect, Stroke, Vec2};

use super::{DEEP, Dive, Fx, Input, NEON_CYAN, NEON_GOLD, NEON_GREEN, NEON_PINK, NEON_RED, NEON_VIOLET, Status, View, glow_ring, grid};
use crate::signals::wharf::DIVE_STATIC;
use crate::ui::theme::{mono, retro};

const FIELD: (f32, f32) = (22.0, 13.0);
const WRONG: f32 = 6.0;

const WORDS: &[&str] = &[
    "HARBOUR", "PATROL", "MIDNIGHT", "CARGO", "GATE", "PIER", "NORTH", "SOUTH", "MOVE", "HOLD", "OPEN", "CLOSE", "THE", "DAWN", "SEVEN", "NINE", "CRATES", "RADIO", "SILENT", "WATCH", "MOLE", "MAST", "TIDE", "HIGH", "SHIP", "DOCK", "GUARD", "CHANGE",
    "ORDERS", "SEARCH", "ALL", "BOATS", "AT", "TO", "NO", "BY",
];

#[derive(Clone, Debug)]
struct Level {
    name: &'static str,
    /// the two plaintexts (never shown; only used to check a crib)
    p: [Vec<u8>; 2],
    /// c1 xor c2
    x: Vec<u8>,
    c: [Vec<u8>; 2],
    bank: Vec<&'static str>,
    rate: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    Intro(f32),
    Play,
    Done(f32),
    Traced(f32),
}

pub struct Static {
    levels: Vec<Level>,
    idx: usize,
    phase: Phase,
    /// which message the crib is laid on
    row: usize,
    crib: usize,
    off: usize,
    locked: Vec<bool>,
    trace: f32,
    t: f32,
    /// seconds since the last commit, and whether it took
    last: Option<(f32, bool)>,
    cleared: bool,
    bot_wait: f32,
    pub wrong: u32,
}

/// A sentence of exactly `n` characters from the vocabulary.
fn sentence(rng: &mut Drbg, n: usize, avoid: &[&'static str]) -> Vec<&'static str> {
    loop {
        let mut words: Vec<&'static str> = Vec::new();
        let mut len = 0;
        while len < n {
            let w = *rng.pick(WORDS);
            if words.contains(&w) || avoid.contains(&w) {
                continue;
            }
            let add = w.len() + usize::from(!words.is_empty());
            if len + add > n {
                break;
            }
            words.push(w);
            len += add;
        }
        if len == n && words.len() >= 3 && words.iter().filter(|w| w.len() >= 5).count() >= 2 {
            return words;
        }
    }
}

fn level(rng: &mut Drbg, name: &'static str, n: usize, bank_size: usize, rate: f32) -> Level {
    let a = sentence(rng, n, &[]);
    let b = sentence(rng, n, &a);
    let (pa, pb) = (a.join(" "), b.join(" "));
    // a real two-time pad: one fresh key, two messages
    let tp = make_two_time_pad(&pa, &pb, rng);
    let mut bank: Vec<&'static str> = a.iter().chain(b.iter()).copied().collect();
    while bank.len() < bank_size {
        let w = *rng.pick(WORDS);
        if !bank.contains(&w) {
            bank.push(w);
        }
    }
    bank.sort_unstable();
    Level { name, p: [pa.into_bytes(), pb.into_bytes()], x: tp.x.clone(), c: [tp.c1, tp.c2], bank, rate }
}

fn plausible(b: u8) -> bool {
    b == b' ' || b.is_ascii_uppercase()
}

impl Static {
    pub fn new(seed: &[u8]) -> Self {
        let mut rng = Drbg::new(seed, "dive/static");
        let levels = vec![level(&mut rng, "FIRST WATCH", 19, 8, 0.6), level(&mut rng, "SECOND WATCH", 25, 11, 0.7), level(&mut rng, "DOG WATCH", 31, 14, 0.8)];
        let mut d = Self { levels, idx: 0, phase: Phase::Intro(0.0), row: 0, crib: 0, off: 0, locked: Vec::new(), trace: 0.0, t: 0.0, last: None, cleared: false, bot_wait: 0.0, wrong: 0 };
        d.load();
        d
    }

    fn load(&mut self) {
        let n = self.levels[self.idx].x.len();
        self.locked = vec![false; n];
        self.phase = Phase::Intro(0.0);
        self.trace = 0.0;
        self.row = 0;
        self.crib = 0;
        self.off = 0;
        self.last = None;
    }

    fn word(&self) -> &'static str {
        self.levels[self.idx].bank[self.crib]
    }

    fn max_off(&self) -> usize {
        self.levels[self.idx].x.len().saturating_sub(self.word().len())
    }

    /// What the crib, laid here, says the other message is.
    fn implied(&self) -> Vec<u8> {
        let l = &self.levels[self.idx];
        self.word().bytes().enumerate().map(|(i, b)| l.x[self.off + i] ^ b).collect()
    }

    fn reads_as_text(&self) -> bool {
        self.implied().iter().all(|b| plausible(*b))
    }

    /// Does the crib contradict something already known?
    fn clashes(&self) -> bool {
        let l = &self.levels[self.idx];
        self.word().bytes().enumerate().any(|(i, b)| self.locked[self.off + i] && l.p[self.row][self.off + i] != b)
    }

    fn commit(&mut self, fx: &mut Fx) {
        let l = &self.levels[self.idx];
        let (w, o, r) = (self.word().as_bytes(), self.off, self.row);
        let right = l.p[r][o..o + w.len()] == *w;
        if right && (o..o + w.len()).all(|c| self.locked[c]) {
            return;
        }
        self.last = Some((0.0, right));
        if !right {
            self.trace += WRONG;
            self.wrong += 1;
            fx.sfx.push("error");
            fx.shake = 0.2;
            return;
        }
        for c in o..o + w.len() {
            self.locked[c] = true;
        }
        // the gaps beside a word come with it
        let n = l.x.len();
        if o > 0 && l.p[r][o - 1] == b' ' {
            self.locked[o - 1] = true;
        }
        if o + w.len() < n && l.p[r][o + w.len()] == b' ' {
            self.locked[o + w.len()] = true;
        }
        fx.sfx.push("lock");
        if self.locked.iter().all(|l| *l) {
            self.phase = Phase::Done(0.0);
            fx.sfx.push("unlock");
        }
    }

    fn cell_w(&self) -> f32 {
        (20.0 / self.levels[self.idx].x.len() as f32).min(0.78)
    }

    fn cell_x(&self, c: usize) -> f32 {
        let n = self.levels[self.idx].x.len() as f32;
        (c as f32 - (n - 1.0) / 2.0) * self.cell_w()
    }

    fn row_y(row: usize) -> f32 {
        if row == 0 { 2.6 } else { -0.4 }
    }
}

impl Dive for Static {
    fn id(&self) -> &'static str {
        "static"
    }

    fn title(&self) -> &'static str {
        "STATIC"
    }

    fn done_flag(&self) -> &'static str {
        DIVE_STATIC
    }

    fn level(&self) -> (usize, usize, String) {
        (self.idx + 1, self.levels.len(), self.levels[self.idx].name.to_string())
    }

    fn brief(&self) -> String {
        "Two broadcasts, one key. Slide a word along either message: where it is right, the other one reads as words too. Commit it.".to_string()
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
        if let Some((age, _)) = &mut self.last {
            *age += dt;
        }
        match self.phase {
            Phase::Intro(age) => {
                self.phase = if age > 1.3 || input.fire || input.pressed { Phase::Play } else { Phase::Intro(age + dt) };
                return;
            }
            Phase::Done(age) => {
                if age > 2.6 {
                    if self.idx + 1 == self.levels.len() {
                        self.cleared = true;
                        fx.sfx.push("success");
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
            Phase::Play => {}
        }
        if self.cleared {
            return;
        }
        self.trace += self.levels[self.idx].rate * dt;
        if self.trace >= 100.0 {
            self.phase = Phase::Traced(0.0);
            fx.sfx.push("alarm");
            fx.shake = 0.4;
            return;
        }
        let n_bank = self.levels[self.idx].bank.len();
        if input.up {
            self.crib = (self.crib + n_bank - 1) % n_bank;
            fx.sfx.push("tick");
        }
        if input.down_key {
            self.crib = (self.crib + 1) % n_bank;
            fx.sfx.push("tick");
        }
        if input.next {
            self.row = 1 - self.row;
            fx.sfx.push("blip");
        }
        self.off = self.off.min(self.max_off());
        if input.left && self.off > 0 {
            self.off -= 1;
            fx.sfx.push("tick");
        }
        if input.right && self.off < self.max_off() {
            self.off += 1;
            fx.sfx.push("tick");
        }
        // pointer: pick a word from the bank, or drop the crib on a cell
        if input.pressed
            && let Some((px, py)) = input.pointer
        {
            let cw = self.cell_w();
            for row in 0..2 {
                if (py - Self::row_y(row)).abs() < 0.55 {
                    let n = self.levels[self.idx].x.len() as f32;
                    let c = (px / cw + (n - 1.0) / 2.0).round();
                    if c >= 0.0 && c < n {
                        self.row = row;
                        self.off = (c as usize).min(self.max_off());
                        fx.sfx.push("tick");
                    }
                }
            }
            for i in 0..n_bank {
                let (bx, by) = bank_pos(i, n_bank);
                if (px - bx).abs() < 1.55 && (py - by).abs() < 0.32 {
                    self.crib = i;
                    self.off = self.off.min(self.max_off());
                    fx.sfx.push("tick");
                }
            }
            if (px - 0.0).abs() < 2.0 && (py + 2.2).abs() < 0.45 {
                self.commit(fx);
            }
        }
        if input.fire {
            self.commit(fx);
        }
    }

    fn draw(&self, p: &egui::Painter, v: &View) {
        grid(p, v, FIELD, 1.0);
        let l = &self.levels[self.idx];
        let n = l.x.len();
        let cw = self.cell_w();
        let word = self.word().as_bytes();
        let implied = self.implied();
        let playing = self.phase == Phase::Play;
        let reads = self.reads_as_text() && !self.clashes();
        // a waterfall of noise behind the rows: the raw difference of the two broadcasts
        for c in 0..n {
            let x = self.cell_x(c);
            let h = l.x[c] as f32 / 255.0;
            let col = if self.locked[c] { NEON_GREEN } else { NEON_CYAN };
            p.line_segment([v.at(x, 1.1 - 0.5 * h), v.at(x, 1.1 + 0.5 * h)], Stroke::new(v.len(cw * 0.5), col.gamma_multiply(0.16)));
            p.text(v.at(x, 1.1), Align2::CENTER_CENTER, format!("{:02x}", l.x[c]), mono((v.len(cw) * 0.42).clamp(7.0, 11.0)), col.gamma_multiply(0.55));
        }
        p.text(v.at(-10.4, 1.1), Align2::LEFT_CENTER, "⊕", mono(13.0), NEON_CYAN.gamma_multiply(0.6));
        for row in 0..2 {
            let y = Self::row_y(row);
            p.text(v.at(self.cell_x(0) - cw * 0.9, y + 0.75), Align2::LEFT_BOTTOM, if row == 0 { "BROADCAST A" } else { "BROADCAST B" }, mono(11.0), NEON_CYAN.gamma_multiply(0.7));
            for c in 0..n {
                let at = v.at(self.cell_x(c), y);
                let r = Rect::from_center_size(at, Vec2::new(v.len(cw) - 3.0, v.len(0.95)));
                let in_crib = playing && c >= self.off && c < self.off + word.len();
                let (ch, col, bg): (Option<u8>, Color32, Color32) = if self.locked[c] {
                    (Some(l.p[row][c]), NEON_GOLD, NEON_GOLD.gamma_multiply(0.10))
                } else if in_crib && row == self.row {
                    (Some(word[c - self.off]), Color32::WHITE, NEON_VIOLET.gamma_multiply(0.45))
                } else if in_crib {
                    let b = implied[c - self.off];
                    if plausible(b) { (Some(b), if reads { NEON_GREEN } else { NEON_CYAN }, NEON_GREEN.gamma_multiply(if reads { 0.22 } else { 0.08 })) } else { (None, NEON_RED, NEON_RED.gamma_multiply(0.22)) }
                } else {
                    (None, NEON_CYAN.gamma_multiply(0.3), DEEP)
                };
                p.rect_filled(r, 2.0, bg);
                p.rect_stroke(r, 2.0, Stroke::new(1.0, col.gamma_multiply(0.45)), egui::StrokeKind::Inside);
                let glyph = match ch {
                    Some(b' ') => "·".to_string(),
                    Some(b) => (b as char).to_string(),
                    None if in_crib => "▒".to_string(),
                    None => "·".to_string(),
                };
                p.text(at, Align2::CENTER_CENTER, glyph, retro((v.len(cw) * 1.05).clamp(12.0, 30.0)), col);
            }
        }
        if playing {
            // which message carries the crib
            let y = Self::row_y(self.row);
            let (a, b) = (self.cell_x(self.off) - cw / 2.0, self.cell_x(self.off + word.len() - 1) + cw / 2.0);
            p.line_segment([v.at(a, y - 0.62), v.at(b, y - 0.62)], Stroke::new(2.0, NEON_VIOLET));
            let verdict = if self.clashes() {
                ("contradicts what you already know", NEON_RED)
            } else if reads {
                ("the other one reads as text here", NEON_GREEN)
            } else {
                ("the other one is noise here", NEON_CYAN.gamma_multiply(0.6))
            };
            p.text(v.at(0.0, -1.45), Align2::CENTER_CENTER, verdict.0, mono(13.0), verdict.1);
            let c = if reads { NEON_GREEN } else { NEON_PINK.gamma_multiply(0.6) };
            let at = v.at(0.0, -2.2);
            p.rect_stroke(Rect::from_center_size(at, Vec2::new(v.len(4.0), v.len(0.8))), 4.0, Stroke::new(1.5, c), egui::StrokeKind::Inside);
            p.text(at, Align2::CENTER_CENTER, "SPACE · COMMIT", retro(20.0), c);
        }
        if let Some((age, ok)) = self.last
            && age < 0.5
        {
            let y = Self::row_y(self.row);
            glow_ring(p, v.at(self.cell_x(self.off + word.len() / 2), y), v.len(0.6 + age * 4.0), if ok { NEON_GREEN } else { NEON_RED }.gamma_multiply(1.0 - age / 0.5), 2.0);
        }
        // the word bank
        let nb = l.bank.len();
        p.text(v.at(-10.4, -3.0), Align2::LEFT_CENTER, "WORDS THE HARBOUR USES   ↑ ↓ choose · ← → slide · TAB other message", mono(11.5), NEON_CYAN.gamma_multiply(0.7));
        for (i, w) in l.bank.iter().enumerate() {
            let (bx, by) = bank_pos(i, nb);
            let at = v.at(bx, by);
            let sel = i == self.crib;
            let r = Rect::from_center_size(at, Vec2::new(v.len(3.0), v.len(0.58)));
            p.rect_filled(r, 3.0, if sel { NEON_VIOLET.gamma_multiply(0.35) } else { DEEP });
            p.rect_stroke(r, 3.0, Stroke::new(1.0, if sel { NEON_VIOLET } else { NEON_CYAN.gamma_multiply(0.25) }), egui::StrokeKind::Inside);
            p.text(at, Align2::CENTER_CENTER, *w, retro(19.0), if sel { Color32::WHITE } else { NEON_CYAN.gamma_multiply(0.8) });
        }
        match self.phase {
            Phase::Intro(age) => {
                let a = (1.0 - (age - 0.9).max(0.0) / 0.4).clamp(0.0, 1.0);
                p.rect_filled(v.rect, 0.0, DEEP.gamma_multiply(0.8 * a));
                p.text(v.at(0.0, 0.6), Align2::CENTER_CENTER, format!("{} / {}", self.idx + 1, self.levels.len()), mono(14.0), NEON_CYAN.gamma_multiply(a));
                p.text(v.at(0.0, -0.3), Align2::CENTER_CENTER, l.name, retro(54.0), NEON_GOLD.gamma_multiply(a));
            }
            Phase::Done(_) => {
                p.text(v.at(0.0, 4.6), Align2::CENTER_CENTER, "BOTH MESSAGES IN CLEAR · THE KEY NEVER MATTERED", retro(26.0), NEON_GREEN);
                // and now the key itself falls out: K = C xor P
                let key: String = l.c[0].iter().zip(&l.p[0]).take(12).map(|(c, m)| format!("{:02x}", c ^ m)).collect();
                p.text(v.at(0.0, 3.9), Align2::CENTER_CENTER, format!("key = C ⊕ P = {key}…"), mono(12.0), NEON_CYAN.gamma_multiply(0.8));
            }
            Phase::Traced(_) => {
                p.rect_filled(v.rect, 0.0, NEON_RED.gamma_multiply(0.12));
                p.text(v.at(0.0, 4.6), Align2::CENTER_CENTER, "TRACED · THE STATION CHANGES FREQUENCY", retro(34.0), NEON_RED);
            }
            Phase::Play => {}
        }
        if self.cleared {
            p.rect_filled(v.rect, 0.0, DEEP.gamma_multiply(0.75));
            p.text(v.at(0.0, 0.4), Align2::CENTER_CENTER, "STATION READ", retro(64.0), NEON_GREEN);
            p.text(v.at(0.0, -0.8), Align2::CENTER_CENTER, format!("three watches, {} wrong guesses", self.wrong), mono(14.0), NEON_CYAN);
        }
    }

    fn bot(&mut self, dt: f32) -> Input {
        let mut i = Input::default();
        match self.phase {
            Phase::Intro(_) => {
                i.fire = true;
                return i;
            }
            Phase::Play => {}
            _ => return i,
        }
        self.bot_wait -= dt;
        if self.bot_wait > 0.0 {
            return i;
        }
        self.bot_wait = 0.06;
        let l = &self.levels[self.idx];
        // the next unlocked word of message A, then of message B
        let mut target: Option<(usize, usize, usize)> = None; // row, offset, bank index
        'find: for row in 0..2 {
            let text = String::from_utf8_lossy(&l.p[row]).to_string();
            let mut o = 0;
            for w in text.split(' ') {
                if (o..o + w.len()).any(|c| !self.locked[c])
                    && let Some(b) = l.bank.iter().position(|k| *k == w)
                {
                    target = Some((row, o, b));
                    break 'find;
                }
                o += w.len() + 1;
            }
        }
        let Some((row, off, bank)) = target else { return i };
        if self.row != row {
            i.next = true;
        } else if self.crib != bank {
            let n = l.bank.len();
            if (bank + n - self.crib) % n <= n / 2 { i.down_key = true } else { i.up = true }
        } else if self.off < off.min(self.max_off()) {
            i.right = true;
        } else if self.off > off {
            i.left = true;
        } else {
            i.fire = true;
            self.bot_wait = 0.5;
        }
        i
    }
}

fn bank_pos(i: usize, n: usize) -> (f32, f32) {
    let cols = 6usize.min(n);
    let (col, row) = (i % cols, i / cols);
    ((col as f32 - (cols as f32 - 1.0) / 2.0) * 3.3, -3.8 - row as f32 * 0.75)
}

#[cfg(test)]
mod tests {
    use super::super::autoplay;
    use super::*;

    #[test]
    fn levels_are_real_two_time_pads_built_from_the_bank() {
        for seed in [&b"a"[..], b"b", b"c"] {
            let d = Static::new(seed);
            for l in &d.levels {
                assert_eq!(l.p[0].len(), l.p[1].len());
                assert_ne!(l.p[0], l.p[1]);
                for i in 0..l.x.len() {
                    assert_eq!(l.x[i], l.p[0][i] ^ l.p[1][i]);
                    assert_eq!(l.c[0][i] ^ l.c[1][i], l.x[i], "the key cancels");
                }
                for row in 0..2 {
                    for w in String::from_utf8_lossy(&l.p[row]).split(' ') {
                        assert!(l.bank.contains(&w), "{w} is not in the bank");
                    }
                }
            }
        }
    }

    #[test]
    fn a_right_crib_opens_both_messages_and_a_wrong_one_costs() {
        let mut d = Static::new(b"crib");
        let mut fx = Fx::default();
        d.phase = Phase::Play;
        let first = String::from_utf8_lossy(&d.levels[0].p[0]).split(' ').next().unwrap().to_string();
        d.crib = d.levels[0].bank.iter().position(|w| *w == first).unwrap();
        d.off = 0;
        assert!(d.reads_as_text(), "a right crib leaves text in the other message");
        d.commit(&mut fx);
        assert!(d.locked[..first.len()].iter().all(|l| *l));
        assert!(d.locked[first.len()], "the gap after the word comes with it");
        // the same word one cell along is wrong
        let before = d.trace;
        d.off = 1;
        d.commit(&mut fx);
        assert_eq!(d.wrong, 1);
        assert!(d.trace >= before + WRONG);
    }

    #[test]
    fn the_bot_reads_the_station() {
        let mut d = Static::new(b"bot");
        let secs = autoplay(&mut d, 300.0).expect("finishes");
        assert!(secs < 200.0, "took {secs}");
        assert_eq!(d.wrong, 0);
        assert_eq!(d.status(), Status::Cleared);
    }
}
