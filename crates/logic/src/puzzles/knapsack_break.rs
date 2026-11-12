//! Chapter 6, part 2. The Directorate's "quantum-safe QSAFE" channel is a
//! relabelled Merkle–Hellman knapsack. We build the CJLOSS lattice from the
//! public key and a ciphertext and run the real integral LLL; the
//! Gram–Schmidt profile flattens as reduction works. Reading the result is the
//! player's job: find the short ±1 row among the reduced vectors, turn it into
//! bits, and check the subset sum. "Hiding structure in a lattice" cuts both
//! ways.

use std::rc::Rc;

use cpl_crypto::bytes::Big;
use cpl_crypto::knapsack::{AttackResult, KnapsackPuzzle, attack_block, bits_to_text, density, make_knapsack_puzzle, mh_encrypt};
use egui::{Align2, Color32, Pos2, RichText, Sense, Shape, Stroke};

use crate::puzzle::{HandCell, HandSheet, Host, Puzzle, StepView, steps};
use crate::ui::theme::{self, AMBER, DIM, FAINT, GREEN, RED, TEXT, mono};
use crate::ui::widgets::{Kind, Log, Tone, button_enabled, card, columns2, kv, lede, math, note, select_card};
use crate::world::{KNAPSACK_MESSAGE, World};

/// Rows of the reduced basis shown to the player.
const SHOWN: usize = 6;

/// One block being worked on by hand.
struct Bench {
    block: usize,
    res: AttackResult,
    /// indices into the reduced basis, in display order
    rows: Vec<usize>,
    picked: Option<usize>,
    /// true: +1 means bit 1; false: −1 means bit 1
    plus_is_one: bool,
    /// (bits, decoded text, subset sum matches)
    tried: Option<(Vec<u8>, String, bool)>,
}

pub struct KnapsackBreak {
    world: Rc<World>,
    puzzle: Option<KnapsackPuzzle>,
    frames: u32,
    /// blocks decoded so far
    block: usize,
    recovered: String,
    profile: Option<(Vec<f64>, Vec<f64>)>,
    bench: Option<Bench>,
    /// the first block was decoded by hand
    by_hand: bool,
    log: Log,
}

impl KnapsackBreak {
    pub fn new(world: Rc<World>) -> Self {
        Self { world, puzzle: None, frames: 0, block: 0, recovered: String::new(), profile: None, bench: None, by_hand: false, log: Log::default() }
    }

    fn ensure(&mut self, host: &mut Host) {
        if self.puzzle.is_none() {
            self.puzzle = Some(make_knapsack_puzzle(&mut self.world.rng("ch6/knapsack"), KNAPSACK_MESSAGE, 32));
            host.status("Run LLL on the first block's lattice.", Tone::Info);
        }
    }

    /// Reduce the lattice of the next block and put it on the bench.
    fn reduce(&mut self, host: &mut Host) {
        self.ensure(host);
        let Some(p) = &self.puzzle else { return };
        if self.block >= p.blocks.len() || self.bench.is_some() {
            return;
        }
        let t0 = web_time::Instant::now();
        let res = attack_block(&p.keys.public, &p.blocks[self.block], 4);
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        self.profile = Some((res.lll.profile_before.clone(), res.lll.profile_after.clone()));
        let mut rows: Vec<usize> = (0..res.lll.basis.len().min(SHOWN)).collect();
        if let Some(r) = res.row
            && !rows.contains(&r)
        {
            rows.pop();
            rows.push(r);
            rows.sort_unstable();
        }
        self.log.line(format!("block {}: LLL done, {} swaps, {ms:.0} ms. Read the reduced rows.", self.block + 1, res.lll.swaps), Tone::Info);
        self.bench = Some(Bench { block: self.block, res, rows, picked: None, plus_is_one: true, tried: None });
        host.sfx("confirm");
    }

    /// Decode the picked row with the chosen sign and check the subset sum.
    fn try_decode(&mut self, host: &mut Host) {
        let Some(p) = &self.puzzle else { return };
        let Some(b) = self.bench.as_mut() else { return };
        let Some(r) = b.picked else {
            host.status("Pick a row first.", Tone::Warn);
            return;
        };
        let n = p.keys.public.len();
        let row = &b.res.lll.basis[r];
        let one = Big::from(1);
        let minus = -Big::from(1);
        if row[n] != Big::from(0u8) {
            host.status("That row's last coordinate is not 0. The last column carries N·(Σ aᵢxᵢ − c): only a row with 0 there satisfies the sum.", Tone::Bad);
            host.sfx("error");
            return;
        }
        if !row[..n].iter().all(|x| x == &one || x == &minus) {
            host.status("That row has entries other than ±1. The solution row is the vector 2x − 1: every entry is +1 or −1.", Tone::Bad);
            host.sfx("error");
            return;
        }
        let want = if b.plus_is_one { one } else { minus };
        let bits: Vec<u8> = row[..n].iter().map(|x| u8::from(x == &want)).collect();
        let c = &p.blocks[b.block];
        let ok = &mh_encrypt(&p.keys.public, &bits) == c;
        let text = bits_to_text(&bits);
        b.tried = Some((bits, text.clone(), ok));
        if !ok {
            host.status(format!("Σ aᵢ·bitᵢ ≠ c: that is the complement subset (it decodes to “{}”). Flip the sign.", text.escape_default()), Tone::Bad);
            host.sfx("error");
            return;
        }
        self.recovered.push_str(&text);
        self.log.line(format!("block {}: subset sum matches → \"{text}\"", b.block + 1), Tone::Ok);
        self.block += 1;
        self.bench = None;
        self.by_hand = true;
        host.sfx("unlock");
        host.status("The sum checks out: that is the plaintext. The other blocks fall the same way.", Tone::Ok);
        self.check_done(host);
    }

    /// The rule you just applied, for every remaining block.
    fn decode_rest(&mut self, host: &mut Host) {
        if !self.by_hand || host.solved() {
            return;
        }
        self.bench = None;
        let Some(p) = &self.puzzle else { return };
        while self.block < p.blocks.len() {
            let res = attack_block(&p.keys.public, &p.blocks[self.block], 4);
            match &res.bits {
                Some(bits) => {
                    let text = bits_to_text(bits);
                    self.recovered.push_str(&text);
                    self.log.line(format!("block {}: ±1 row {} → \"{text}\"", self.block + 1, res.row.unwrap_or(0)), Tone::Ok);
                }
                None => self.log.line(format!("block {}: no short vector found", self.block + 1), Tone::Bad),
            }
            self.profile = Some((res.lll.profile_before.clone(), res.lll.profile_after.clone()));
            self.block += 1;
        }
        self.check_done(host);
    }

    fn check_done(&mut self, host: &mut Host) {
        let Some(p) = &self.puzzle else { return };
        if self.block < p.blocks.len() || host.solved() {
            return;
        }
        let msg = self.recovered.trim_end().to_string();
        host.sfx("unlock");
        host.solve(format!("QSAFE broken: \"{msg}\". A knapsack is not quantum-safe; it is barely classically safe. Real post-quantum crypto needs the hard lattice problems, not the easy ones in disguise."));
    }

    fn plot(&self, ui: &mut egui::Ui) {
        let size = egui::vec2(ui.available_width().min(460.0), 200.0);
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let p = ui.painter_at(rect);
        p.rect_filled(rect, 3.0, theme::rgb(0x070b16));
        let Some((before, after)) = &self.profile else {
            p.text(rect.left_center() + egui::vec2(20.0, 0.0), Align2::LEFT_CENTER, "run LLL to see the profile", mono(12.0), FAINT);
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

    /// A reduced row in one line: + and − for ±1, 0, · for anything bigger, then the last coordinate.
    fn row_text(row: &[Big]) -> (String, String) {
        let n = row.len() - 1;
        let one = Big::from(1);
        let minus = -Big::from(1);
        let body: String = row[..n]
            .iter()
            .map(|x| {
                if x == &one {
                    '+'
                } else if x == &minus {
                    '−'
                } else if *x == Big::from(0u8) {
                    '0'
                } else {
                    '·'
                }
            })
            .collect();
        let last = row[n].to_string();
        let last = if last.len() > 10 { format!("{}…", &last[..10]) } else { last };
        (body, last)
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
        lede(ui, "The Directorate rebranded an old public-key scheme as \"QSAFE\" and promised it is quantum-safe. It is a Merkle–Hellman knapsack from 1978, broken since 1982. Encryption hides an easy subset-sum inside a hard-looking one; lattice reduction digs it back out.");
        math(ui, "CJLOSS basis: bᵢ = (0,…,2,…,0, N·aᵢ),   bₙ₊₁ = (1,…,1, N·c).   Solution: (2x₁−1, …, 2xₙ−1, 0)");
        let Some(p) = &self.puzzle else {
            note(ui, "Generating keys…");
            return;
        };
        let (weights, dens, blocks) = (p.keys.public.len(), density(&p.keys.public), p.blocks.len());
        card(ui, Some("Intercepted"), None, |ui| {
            kv(ui, "kn-int", &[("public key", format!("{weights} weights, density {dens:.2} (low = breakable)")), ("ciphertext blocks", blocks.to_string())]);
        });
        let mut act: Option<u8> = None;
        let mut pick: Option<usize> = None;
        columns2(
            ui,
            |ui| {
                card(ui, Some("Gram–Schmidt profile (log₂ |bᵢ*|)"), None, |ui| {
                    note(ui, "A skewed basis has a steep profile. LLL flattens it; that is reduction, made visible.");
                    self.plot(ui);
                });
                card(ui, Some("Recovered"), None, |ui| {
                    let shown = if self.recovered.is_empty() { "…".to_string() } else { self.recovered.trim_end().to_string() };
                    ui.add(egui::Label::new(RichText::new(shown).font(mono(15.0)).color(GREEN)).wrap());
                    note(ui, &format!("{} of {blocks} blocks", self.block));
                    self.log.show(ui, "kn-log", 120.0);
                });
            },
            |ui| {
                card(ui, Some("Workbench"), None, |ui| {
                    match &self.bench {
                        None => {
                            note(ui, if self.block == 0 { "Reduce the first block's lattice. LLL is the microscope; reading the slide is up to you." } else { "Next block." });
                            ui.horizontal_wrapped(|ui| {
                                if button_enabled(ui, self.block < blocks && !self.by_hand, "Run LLL on the next block", Kind::Primary).clicked() {
                                    act = Some(0);
                                }
                                if button_enabled(ui, self.by_hand && self.block < blocks, "Decode the remaining blocks the same way", Kind::Primary).clicked() {
                                    act = Some(3);
                                }
                            });
                        }
                        Some(b) => {
                            note(ui, &format!("Block {}: the first rows of the reduced basis. + and − are ±1, · is anything bigger; the last number is the final coordinate.", b.block + 1));
                            for &r in &b.rows {
                                let (body, last) = Self::row_text(&b.res.lll.basis[r]);
                                if select_card(ui, b.picked == Some(r), AMBER, |ui| {
                                    ui.label(RichText::new(format!("row {r:>2}  {body}")).font(mono(11.5)).color(TEXT));
                                    ui.label(RichText::new(format!("        last: {last}")).font(mono(11.0)).color(DIM));
                                }) {
                                    pick = Some(r);
                                }
                            }
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new("bit 1 is").font(mono(12.5)).color(DIM));
                                if ui.selectable_label(b.plus_is_one, RichText::new("+1").font(mono(13.0))).clicked() {
                                    act = Some(1);
                                }
                                if ui.selectable_label(!b.plus_is_one, RichText::new("−1").font(mono(13.0))).clicked() {
                                    act = Some(2);
                                }
                                if button_enabled(ui, b.picked.is_some(), "Decode and check Σ aᵢ·bitᵢ = c", Kind::Primary).clicked() {
                                    act = Some(4);
                                }
                            });
                            if let Some((_, text, ok)) = &b.tried {
                                ui.label(RichText::new(format!("decodes to “{}”", text.escape_default())).font(mono(13.0)).color(if *ok { GREEN } else { RED }));
                            }
                        }
                    }
                });
            },
        );
        if let Some(r) = pick
            && let Some(b) = self.bench.as_mut()
        {
            b.picked = Some(r);
            b.tried = None;
        }
        match act {
            Some(0) => self.reduce(host),
            Some(1) => {
                if let Some(b) = self.bench.as_mut() {
                    b.plus_is_one = true;
                }
            }
            Some(2) => {
                if let Some(b) = self.bench.as_mut() {
                    b.plus_is_one = false;
                }
            }
            Some(3) => self.decode_rest(host),
            Some(4) => self.try_decode(host),
            _ => {}
        }
    }

    fn autoplay(&mut self, host: &mut Host) {
        self.ensure(host);
        if !self.by_hand {
            self.reduce(host);
            let row = self.bench.as_ref().and_then(|b| b.res.row);
            if let Some(b) = self.bench.as_mut() {
                b.picked = row;
                b.plus_is_one = true;
            }
            self.try_decode(host);
            if !self.by_hand {
                if let Some(b) = self.bench.as_mut() {
                    b.plus_is_one = false;
                }
                self.try_decode(host);
            }
        }
        self.decode_rest(host);
    }

    fn steps(&self) -> Vec<StepView> {
        let reduced = self.bench.is_some() || self.by_hand;
        let picked_right = self.by_hand || self.bench.as_ref().is_some_and(|b| b.picked.is_some() && b.picked == b.res.row);
        steps(&[
            ("Run LLL on the first block's lattice", "", reduced),
            ("Find the short row: all ±1, last entry 0", "That row is 2x − 1 for the plaintext bits x.", picked_right),
            ("Turn ±1 into bits and check the subset sum", "One sign gives the plaintext, the other its complement.", self.by_hand),
            ("Decode the remaining blocks the same way", "", self.puzzle.as_ref().is_some_and(|p| self.block >= p.blocks.len())),
        ])
    }

    fn deep_sheet(&self) -> Option<HandSheet> {
        Some(HandSheet::new(
            "Merkle–Hellman by hand",
            "Private key: the superincreasing list w = (2, 3, 7, 14, 30), modulus q = 61, multiplier r = 17. Public key: aᵢ = r·wᵢ mod q. Encrypt the bits 1 0 1 1 0, then decrypt.",
            240.0,
            vec![
                HandCell::new("a₁ = 17·2 mod 61", "", 34),
                HandCell::new("a₃ = 17·7 mod 61", "", 58),
                HandCell::new("c = a₁ + a₃ + a₄  (a₄ = 55)", "the ciphertext is a plain sum", 147),
                HandCell::new("17⁻¹ mod 61", "extended Euclid", 18),
                HandCell::new("c · 17⁻¹ mod 61", "now greedy: 30? 14? 7? 3? 2?", 23),
            ],
        ))
    }
}
