//! Side quest: the Ledger. Wen keeps the Commons co-op's books as a hash
//! chain. Someone edited it. First a clumsy edit that left the old hashes in
//! place; then a careful one that recomputed every hash after it. Only the
//! hashes Wen read out on the radio each evening can catch the second.

use std::rc::Rc;

use cpl_crypto::hashchain::{Hash, chain, first_break, short};
use egui::{Color32, RichText};

use crate::puzzle::{Host, Puzzle, StepView, steps};
use crate::ui::theme::{AMBER, CYAN, DIM, GREEN, RED, TEXT, mono};
use crate::ui::widgets::{Kind, Tone, button, card, lede, note};
use crate::world::World;

const GOODS: [&str; 10] = ["12 crates of oranges", "40 kg of rice", "a crate of radio valves", "8 sacks of flour", "solar panel, used", "30 litres of olive oil", "medicine for the clinic", "two bicycles", "seeds, 14 varieties", "a printing press, second hand"];
/// Wen read the head hash on the radio after these days.
const ANCHORS: [usize; 3] = [2, 5, 8];

pub struct LedgerChain {
    honest: Vec<Hash>,
    /// stage 1: edited text, old hashes
    clumsy: (Vec<String>, usize),
    /// stage 2: edited text, recomputed hashes
    careful: (Vec<String>, Vec<Hash>, usize),
    stage: u8,
    recomputed: bool,
    picks: Vec<String>,
}

impl LedgerChain {
    pub fn new(world: Rc<World>) -> Self {
        let mut r = world.rng("q/ledger");
        let texts: Vec<String> = GOODS.iter().enumerate().map(|(i, g)| format!("day {}: {} from the north co-op, paid in eCash", i + 1, g)).collect();
        let honest = chain(&texts);
        let e1 = 2 + r.int(6);
        let mut t1 = texts.clone();
        t1[e1] = t1[e1].replace("from the north co-op", "from the Directorate depot");
        let e2 = 3 + r.int(5);
        let mut t2 = texts.clone();
        t2[e2] = t2[e2].replace("paid in eCash", "paid by card no. 4417");
        let forged = chain(&t2);
        Self { honest, clumsy: (t1, e1), careful: (t2, forged, e2), stage: 1, recomputed: false, picks: Vec::new() }
    }

    fn pick_row(&mut self, i: usize, host: &mut Host) {
        let (texts, edited) = &self.clumsy;
        let brk = first_break(texts, &self.honest);
        if Some(i) == brk {
            debug_assert_eq!(i, *edited);
            self.picks.push(format!("day {} was edited", i + 1));
            self.stage = 2;
            self.recomputed = false;
            host.sfx("confirm");
            host.status("Caught: its recorded hash no longer matches its text. Whoever did it next time will recompute the hashes too. Here is the ledger a week later.", Tone::Ok);
        } else if brk.is_some_and(|b| i > b) {
            host.status("That row breaks, but only because an earlier one did: every hash covers the one before it. Find the first.", Tone::Warn);
        } else {
            host.status("That row's hash still matches its text and its predecessor.", Tone::Bad);
        }
    }

    fn pick_segment(&mut self, seg: usize, host: &mut Host) {
        let (_, forged, edited) = &self.careful;
        let bad_anchor = ANCHORS.iter().position(|&a| forged[a] != self.honest[a]);
        let Some(b) = bad_anchor else { return };
        let lo = if b == 0 { 0 } else { ANCHORS[b - 1] + 1 };
        let hi = ANCHORS[b];
        if seg == b {
            debug_assert!(*edited >= lo && *edited <= hi);
            self.picks.push(format!("the edit is in days {}–{}", lo + 1, hi + 1));
            host.sfx("unlock");
            host.solve(format!("Days {}–{}: the chain agrees with the radio up to day {lo} and not at day {}. Wen goes to the co-op's paper receipts for those days and finds the forged line.", lo + 1, hi + 1, hi + 1));
        } else {
            host.status("Compare the hashes Wen broadcast with the ledger's hashes on those days. The edit sits after the last match and at or before the first mismatch.", Tone::Bad);
        }
    }
}

impl Puzzle for LedgerChain {
    fn id(&self) -> &'static str {
        "q.ledger"
    }

    fn title(&self) -> &str {
        "WEN'S LEDGER"
    }

    fn subtitle(&self) -> Option<&str> {
        Some("hash chains · Haber–Stornetta timestamps")
    }

    fn accent(&self) -> Color32 {
        GREEN
    }

    fn open(&mut self, host: &mut Host) {
        host.status("Each line's hash covers the line and the hash before it. Find where the ledger was changed.", Tone::Info);
    }

    fn steps(&self) -> Vec<StepView> {
        steps(&[
            ("Recompute the chain", "Hash every line with the hash before it and compare with what is written down.", self.stage > 1 || self.recomputed),
            ("Click the first line that breaks", "Every line after it breaks too.", self.stage > 1),
            ("Second edit: compare with the radio", "The forger fixed every hash. Only the hashes Wen read out on air can't be changed.", self.picks.len() >= 2),
        ])
    }

    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        lede(ui, "Wen runs the Commons co-op. Every delivery goes into the ledger as one line, and every line's hash covers the hash before it, so nobody can quietly rewrite a past day. The Directorate wants the co-op's supplier list.");
        let mut act: Option<(u8, usize)> = None;
        if self.stage == 1 {
            let (texts, _) = &self.clumsy;
            let now = chain(texts);
            card(ui, Some("The ledger"), None, |ui| {
                egui::Grid::new("ledger1").num_columns(3).spacing([10.0, 3.0]).show(ui, |ui| {
                    for (i, t) in texts.iter().enumerate() {
                        let recorded = short(&self.honest[i]);
                        let fresh = if self.recomputed { first_break(&texts[..=i], &self.honest[..=i]).map_or_else(|| "✓".to_string(), |_| format!("≠ {}", short(&now[i]))) } else { String::new() };
                        if ui.add(egui::Button::new(RichText::new(t).font(mono(12.0)).color(TEXT)).frame(false)).clicked() {
                            act = Some((1, i));
                        }
                        ui.label(RichText::new(recorded).font(mono(12.0)).color(AMBER));
                        ui.label(RichText::new(fresh.clone()).font(mono(12.0)).color(if fresh == "✓" { GREEN } else { RED }));
                        ui.end_row();
                    }
                });
                if !self.recomputed && button(ui, "Recompute every hash", Kind::Primary).clicked() {
                    act = Some((0, 0));
                }
            });
        } else {
            let (texts, forged, _) = &self.careful;
            card(ui, Some("The ledger, a week later"), None, |ui| {
                egui::Grid::new("ledger2").num_columns(3).spacing([10.0, 3.0]).show(ui, |ui| {
                    for (i, t) in texts.iter().enumerate() {
                        ui.label(RichText::new(t).font(mono(12.0)).color(TEXT));
                        ui.label(RichText::new(short(&forged[i])).font(mono(12.0)).color(AMBER));
                        let anchor = ANCHORS.iter().position(|a| *a == i);
                        ui.label(RichText::new(anchor.map(|_| format!("radio: {}", short(&self.honest[i]))).unwrap_or_default()).font(mono(12.0)).color(CYAN));
                        ui.end_row();
                    }
                });
                note(ui, "Recomputing finds nothing: the forger recomputed every hash after the edit. But Wen read the latest hash on the community radio on days 3, 6 and 9, and a thousand people heard it.");
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("the edit is in:").font(mono(12.5)).color(DIM));
                    for (b, &a) in ANCHORS.iter().enumerate() {
                        let lo = if b == 0 { 0 } else { ANCHORS[b - 1] + 1 };
                        if button(ui, &format!("days {}–{}", lo + 1, a + 1), Kind::Normal).clicked() {
                            act = Some((2, b));
                        }
                    }
                });
            });
        }
        match act {
            Some((0, _)) => self.recomputed = true,
            Some((1, i)) => {
                if self.recomputed {
                    self.pick_row(i, host);
                } else {
                    host.status("Recompute the chain first, then compare.", Tone::Warn);
                }
            }
            Some((2, b)) => self.pick_segment(b, host),
            _ => {}
        }
        for p in &self.picks {
            ui.label(RichText::new(format!("✓ {p}")).font(mono(12.5)).color(GREEN));
        }
        if host.solved() {
            note(ui, "Haber and Stornetta proposed exactly this in 1991; from 1995 their company printed its chain's latest hash every week in the classified ads of the New York Times. Bitcoin's blocks are chained the same way, and its \"newspaper\" is the work piled on top.");
        }
    }

    fn autoplay(&mut self, host: &mut Host) {
        self.recomputed = true;
        let b = first_break(&self.clumsy.0, &self.honest).unwrap();
        self.pick_row(b, host);
        let bad = ANCHORS.iter().position(|&a| self.careful.1[a] != self.honest[a]).unwrap();
        self.pick_segment(bad, host);
    }
}
