//! Chapter 6, part 2. The Directorate's "quantum-safe QSAFE" channel is a
//! relabelled Merkle–Hellman knapsack. We build the CJLOSS lattice from the
//! public key and a ciphertext, run the real integral LLL, and watch the
//! Gram–Schmidt profile flatten as reduction finds the short vector that holds
//! the plaintext bits. "Hiding structure in a lattice" cuts both ways.

use std::rc::Rc;

use cpl_crypto::knapsack::{KnapsackPuzzle, attack_block, bits_to_text, density, make_knapsack_puzzle};
use egui::{Align2, Color32, Pos2, RichText, Sense, Shape, Stroke};

use crate::puzzle::{Host, Puzzle};
use crate::ui::theme::{self, FAINT, GREEN, mono};
use crate::ui::widgets::{Kind, Log, Tone, button_enabled, card, columns2, kv, lede, math, note};
use crate::world::{KNAPSACK_MESSAGE, World};

pub struct KnapsackBreak {
    world: Rc<World>,
    puzzle: Option<KnapsackPuzzle>,
    frames: u32,
    block: usize,
    recovered: String,
    profile: Option<(Vec<f64>, Vec<f64>)>,
    log: Log,
    /// "Break all": time of the last block broken
    auto_all: Option<f64>,
}

impl KnapsackBreak {
    pub fn new(world: Rc<World>) -> Self {
        Self { world, puzzle: None, frames: 0, block: 0, recovered: String::new(), profile: None, log: Log::default(), auto_all: None }
    }

    fn ensure(&mut self, host: &mut Host) {
        if self.puzzle.is_none() {
            self.puzzle = Some(make_knapsack_puzzle(&mut self.world.rng("ch6/knapsack"), KNAPSACK_MESSAGE, 32));
            host.status("Run the lattice attack on the QSAFE ciphertext.", Tone::Info);
        }
    }

    fn break_block(&mut self, host: &mut Host) {
        if host.solved() {
            return;
        }
        self.ensure(host);
        let Some(p) = &self.puzzle else { return };
        if self.block >= p.blocks.len() {
            return;
        }
        let t0 = web_time::Instant::now();
        let res = attack_block(&p.keys.public, &p.blocks[self.block], 4);
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        self.profile = Some((res.lll.profile_before.clone(), res.lll.profile_after.clone()));
        match &res.bits {
            Some(bits) => {
                let text = bits_to_text(bits);
                self.recovered.push_str(&text);
                self.log.line(format!("block {}: LLL {} swaps, {ms:.0}ms → \"{text}\"", self.block + 1, res.lll.swaps), Tone::Ok);
            }
            None => self.log.line(format!("block {}: no short vector found", self.block + 1), Tone::Bad),
        }
        self.block += 1;
        host.sfx("confirm");
        if self.block >= p.blocks.len() {
            self.finish(host);
        }
    }

    fn finish(&mut self, host: &mut Host) {
        self.auto_all = None;
        let msg = self.recovered.trim_end().to_string();
        host.sfx("unlock");
        host.solve(format!("QSAFE broken: \"{msg}\". A knapsack is not quantum-safe; it is barely classically safe. Real post-quantum crypto needs the hard lattice problems, not the easy ones in disguise."));
    }

    fn plot(&self, ui: &mut egui::Ui) {
        let size = egui::vec2(ui.available_width().min(460.0), 220.0);
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let p = ui.painter_at(rect);
        p.rect_filled(rect, 3.0, theme::rgb(0x070b16));
        let Some((before, after)) = &self.profile else {
            p.text(rect.left_center() + egui::vec2(20.0, 0.0), Align2::LEFT_CENTER, "run the attack to see the profile", mono(12.0), FAINT);
            return;
        };
        let all: Vec<f64> = before.iter().chain(after.iter()).copied().collect();
        let lo = all.iter().copied().fold(f64::INFINITY, f64::min) - 0.5;
        let hi = all.iter().copied().fold(f64::NEG_INFINITY, f64::max) + 0.5;
        let n = before.len().max(after.len()).max(2);
        let (w, h) = (rect.width(), rect.height());
        let px = |i: usize| rect.left() + 30.0 + (i as f32 / (n - 1) as f32) * (w - 50.0);
        let py = |v: f64| rect.top() + h - 25.0 - (((v - lo) / (hi - lo)) as f32) * (h - 45.0);
        let series = |prof: &[f64], color: Color32, label: &str, dy: f32| {
            let pts: Vec<Pos2> = prof.iter().enumerate().map(|(i, v)| Pos2::new(px(i), py(*v))).collect();
            p.add(Shape::line(pts.clone(), Stroke::new(2.0, color)));
            for q in pts {
                p.circle_filled(q, 2.0, color);
            }
            p.text(Pos2::new(rect.right() - 130.0, rect.top() + 14.0 + dy), Align2::LEFT_CENTER, label, mono(11.0), color);
        };
        series(before, theme::rgb(0xff6b81), "before (skewed)", 0.0);
        series(after, GREEN, "after LLL (flat)", 14.0);
    }
}

impl Puzzle for KnapsackBreak {
    fn id(&self) -> &'static str {
        "ch6.knapsack"
    }

    fn title(&self) -> &str {
        "QSAFE CHANNEL · BREAK IN"
    }

    fn subtitle(&self) -> Option<&str> {
        Some("Merkle–Hellman + LLL (CJLOSS)")
    }

    fn accent(&self) -> Color32 {
        theme::rgb(0x73b7ff)
    }

    fn open(&mut self, host: &mut Host) {
        host.status("Generating an attackable QSAFE instance…", Tone::Info);
    }

    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        // let the window appear once before the (brief) key generation
        self.frames += 1;
        if self.puzzle.is_none() {
            if self.frames > 1 {
                self.ensure(host);
            }
            ui.ctx().request_repaint();
        }
        if let Some(last) = self.auto_all {
            if host.time - last >= 0.12 {
                self.break_block(host);
                if self.auto_all.is_some() {
                    self.auto_all = Some(host.time);
                }
            }
            ui.ctx().request_repaint();
        }
        lede(ui, "The Directorate rebranded an old public-key scheme as \"QSAFE\" and promised it is quantum-safe. It is a Merkle–Hellman knapsack from 1978, broken since 1982. Encryption hides an easy subset-sum inside a hard-looking one; lattice reduction digs it back out.");
        math(ui, "CJLOSS basis: bᵢ = (0,…,2,…,0, N·aᵢ),   bₙ₊₁ = (1,…,1, N·c).   Target: (±1,…,±1, 0)");
        let Some(p) = &self.puzzle else {
            note(ui, "Generating keys…");
            return;
        };
        let (weights, dens, blocks) = (p.keys.public.len(), density(&p.keys.public), p.blocks.len());
        card(ui, Some("Intercepted"), None, |ui| {
            kv(ui, "kn-int", &[("public key", format!("{weights} weights, density {dens:.2} (low = breakable)")), ("ciphertext blocks", blocks.to_string())]);
        });
        let mut one = false;
        let mut all = false;
        let busy = self.auto_all.is_some();
        let left_done = self.block >= blocks;
        columns2(
            ui,
            |ui| {
                card(ui, Some("Gram–Schmidt profile (log₂ |bᵢ*|)"), None, |ui| {
                    note(ui, "A skewed basis has a steep profile. LLL flattens it; that is reduction, made visible.");
                    self.plot(ui);
                });
            },
            |ui| {
                card(ui, Some("Attack"), None, |ui| {
                    note(ui, "Run LLL on each block's lattice. The ±1 short vector it finds decodes to the plaintext bits.");
                    ui.horizontal_wrapped(|ui| {
                        if button_enabled(ui, !busy && !left_done, "Break next block", Kind::Primary).clicked() {
                            one = true;
                        }
                        if button_enabled(ui, !busy && !left_done, "Break all", Kind::Normal).clicked() {
                            all = true;
                        }
                    });
                    let shown = if self.recovered.is_empty() { "…".to_string() } else { self.recovered.trim_end().to_string() };
                    ui.add(egui::Label::new(RichText::new(shown).font(mono(15.0)).color(GREEN)).wrap());
                    note(ui, &format!("{} of {blocks} blocks", self.block));
                    self.log.show(ui, "kn-log", 150.0);
                });
            },
        );
        if one {
            self.break_block(host);
        }
        if all {
            self.auto_all = Some(host.time - 1.0);
        }
    }

    fn autoplay(&mut self, host: &mut Host) {
        self.ensure(host);
        while !host.solved() && self.puzzle.as_ref().is_some_and(|p| self.block < p.blocks.len()) {
            self.break_block(host);
        }
    }
}
