//! Chapter 5, part 2. Why does the proof leak nothing? Because a transcript
//! can be *faked* without the secret, with exactly the same distribution. This
//! is the real-or-simulated game: you are shown valid transcripts, some from
//! an honest prover, some forged by the simulator, and you cannot do better
//! than a coin flip at telling them apart. If a transcript proved anything,
//! you could.

use std::rc::Rc;

use cpl_crypto::bytes::{Big, to_hex};
use cpl_crypto::drbg::Drbg;
use cpl_crypto::secp::{N, Pt, compressed, mul};
use cpl_crypto::sigma::{Transcript, commit, respond, simulate, verify_transcript};
use egui::{Color32, RichText};

use crate::puzzle::{Host, Puzzle};
use crate::ui::theme::{DIM, GREEN, RED, TEXT, VIOLET, mono};
use crate::ui::widgets::{Kind, Tone, button, card, lede, math, note, shorten, tag};
use crate::world::{World, hex32};

const TOTAL: u32 = 8;

pub struct SimulatorGame {
    x: Big,
    big_x: Pt,
    round: u32,
    correct: u32,
    counter: u64,
    current: Option<(Transcript, bool, bool)>,
    /// (guessed right, when) while the answer is on screen
    reveal: Option<(bool, f64)>,
    done: bool,
}

impl SimulatorGame {
    pub fn new(world: Rc<World>) -> Self {
        let x = world.identity().x.clone();
        let mut s = Self { big_x: mul(&x), x, round: 0, correct: 0, counter: 0, current: None, reveal: None, done: false };
        s.deal();
        s
    }

    fn deal(&mut self) {
        self.counter += 1;
        let mut rng = Drbg::fresh(&format!("ch5/sim-{}-{}", self.round, self.counter));
        let real = rng.int(2) == 0;
        let t = if real {
            let (r, rp) = commit(&mut rng);
            let c = rng.scalar(&N);
            Transcript { r_point: rp, s: respond(&r, &c, &self.x), c }
        } else {
            let (c, s) = (rng.scalar(&N), rng.scalar(&N));
            simulate(&self.big_x, &c, &s)
        };
        let valid = verify_transcript(&self.big_x, &t);
        self.current = Some((t, real, valid));
        self.reveal = None;
    }

    fn guess(&mut self, real: bool, host: &mut Host) {
        if self.reveal.is_some() || self.done {
            return;
        }
        let Some((_, truth, _)) = &self.current else { return };
        let right = real == *truth;
        if right {
            self.correct += 1;
        }
        host.sfx(if right { "confirm" } else { "error" });
        self.reveal = Some((right, host.time));
    }

    fn advance(&mut self, host: &mut Host) {
        self.round += 1;
        if self.round >= TOTAL {
            self.finish(host);
        } else {
            self.deal();
        }
    }

    fn finish(&mut self, host: &mut Host) {
        self.done = true;
        self.current = None;
        self.reveal = None;
        host.sfx("unlock");
        host.solve("You could not beat a coin flip. Neither can the Directorate. The proofs leak nothing.");
    }
}

impl Puzzle for SimulatorGame {
    fn id(&self) -> &'static str {
        "ch5.sim"
    }

    fn title(&self) -> &str {
        "REAL OR SIMULATED?"
    }

    fn subtitle(&self) -> Option<&str> {
        Some("honest-verifier zero knowledge")
    }

    fn accent(&self) -> Color32 {
        VIOLET
    }

    fn open(&mut self, host: &mut Host) {
        host.status("Guess real or simulated. Eight rounds.", Tone::Info);
    }

    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        if let Some((_, at)) = self.reveal {
            if host.time - at >= 0.65 {
                self.advance(host);
            } else {
                ui.ctx().request_repaint();
            }
        }
        lede(ui, "Here is why the checkpoint learned nothing. A **real** transcript comes from the honest protocol: pick r, get c, answer s = r + c·x. A **simulated** one is forged with no secret at all: pick c and s first, then set R = s·G − c·X. Both verify. Both have the same distribution. Can you tell which is which?");
        math(ui, "real: R = r·G, s = r + c·x     simulated: choose c, s, set R = s·G − c·X");
        let mut pick: Option<bool> = None;
        if self.done {
            let pct = (self.correct as f32 / TOTAL as f32 * 100.0).round();
            card(ui, Some(&format!("You scored {} / {TOTAL} ({pct}%)", self.correct)), Some(VIOLET), |ui| {
                note(ui, "Random guessing is 50%. That is the point: there is **no** strategy that does better, because a simulator with no secret produces transcripts indistinguishable from the real ones. A transcript therefore proves, to anyone who was not the verifier, exactly nothing. That is what \"zero knowledge\" means.");
            });
            return;
        }
        if let Some((t, real, valid)) = &self.current {
            card(ui, Some("Transcript"), None, |ui| {
                egui::Grid::new("sim-t").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                    for (k, v) in [("R", shorten(&to_hex(&compressed(&t.r_point)), 12, 10)), ("c", shorten(&hex32(&t.c), 12, 10)), ("s", shorten(&hex32(&t.s), 12, 10))] {
                        ui.label(RichText::new(k).font(mono(12.5)).color(DIM));
                        ui.label(RichText::new(v).font(mono(12.5)).color(TEXT));
                        ui.end_row();
                    }
                    ui.label(RichText::new("s·G = R + c·X ?").font(mono(12.5)).color(DIM));
                    tag(ui, if *valid { "VALID" } else { "INVALID" }, if *valid { Tone::Ok } else { Tone::Bad });
                    ui.end_row();
                });
                note(ui, "Both kinds verify, so \"valid\" tells you nothing.");
                ui.add_enabled_ui(self.reveal.is_none(), |ui| {
                    ui.horizontal_wrapped(|ui| {
                        if button(ui, "Real (honest prover)", Kind::Primary).clicked() {
                            pick = Some(true);
                        }
                        if button(ui, "Simulated (no secret)", Kind::Normal).clicked() {
                            pick = Some(false);
                        }
                    });
                });
                if let Some((right, _)) = self.reveal {
                    let text = format!("It was {}. You guessed {}.", if *real { "real" } else { "simulated" }, if right { "right" } else { "wrong" });
                    ui.label(RichText::new(text).font(mono(13.0)).color(if right { GREEN } else { RED }));
                }
            });
        }
        note(ui, &format!("Round {} of {TOTAL} · correct so far: {}", (self.round + 1).min(TOTAL), self.correct));
        if let Some(real) = pick {
            self.guess(real, host);
        }
    }

    fn autoplay(&mut self, host: &mut Host) {
        // answer each round (we happen to know, since we built it) and move on
        while !self.done {
            let truth = self.current.as_ref().map(|c| c.1).unwrap_or(true);
            self.guess(truth, host);
            self.advance(host);
        }
    }
}
