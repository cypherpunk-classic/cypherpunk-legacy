//! Chapter 5, part 2. Why does the proof leak nothing? Because a transcript
//! can be *faked* without the secret, with exactly the same distribution. This
//! is the real-or-simulated game: you are shown valid transcripts, some from
//! an honest prover, some forged by the simulator, and you cannot do better
//! than a coin flip at telling them apart. If a transcript proved anything,
//! you could. First the player forges one by hand: choose c and s, then solve
//! the verification equation for R.

use std::rc::Rc;

use cpl_crypto::bytes::{Big, to_hex};
use cpl_crypto::drbg::Drbg;
use cpl_crypto::bytes::inv_mod_i64;
use cpl_crypto::dh::TOY_GROUP;
use cpl_crypto::secp::{N, Pt, compressed, g, mul, mul_pt};
use cpl_crypto::sigma::{Transcript, commit, respond, simulate, verify_transcript};
use egui::{Color32, RichText};

use crate::puzzle::{HandCell, HandSheet, Host, Puzzle, StepView, steps};
use crate::ui::theme::{DIM, GREEN, RED, TEXT, VIOLET, mono};
use crate::ui::widgets::{Formula, Kind, Part, Tone, button, button_enabled, card, formula, lede, math, note, shorten, tag};

const PARTS: &[Part] = &[Part::Text("R = "), Part::Slot, Part::Text("·G  −  "), Part::Slot, Part::Text("·X")];
const PALETTE: &[&str] = &["s", "c", "r", "x"];
use crate::world::{World, hex32};

const TOTAL: u32 = 6;

pub struct SimulatorGame {
    world: Rc<World>,
    /// the player's own forgery: (c, s) chosen first, then R from the formula
    mine: Option<(Big, Big)>,
    forge: Formula,
    forged: Option<(Transcript, bool)>,
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
        let mut s = Self { world, mine: None, forge: Formula::new(PARTS), forged: None, big_x: mul(&x), x, round: 0, correct: 0, counter: 0, current: None, reveal: None, done: false };
        s.deal();
        s
    }

    fn roll_mine(&mut self) {
        self.counter += 1;
        let mut rng = Drbg::fresh(&format!("ch5/sim-mine-{}", self.counter));
        self.mine = Some((rng.scalar(&N), rng.scalar(&N)));
        self.forged = None;
    }

    fn forge_one(&mut self, host: &mut Host) {
        let Some((c, s)) = self.mine.clone() else { return };
        let toks = self.forge.tokens();
        let (Some(a), Some(b)) = (toks[0], toks[1]) else { return };
        if a >= 2 || b >= 2 {
            let why = if a == 3 || b == 3 { "x is exactly what a simulator does not have. That is the point." } else { "There is no r in a simulation: you never committed to anything first." };
            host.status(why, Tone::Bad);
            host.sfx("error");
            return;
        }
        let val = |t: usize| if t == 0 { s.clone() } else { c.clone() };
        let r_point = mul_pt(&val(a), &g()) - mul_pt(&val(b), &self.big_x);
        let t = Transcript { r_point, c: c.clone(), s: s.clone() };
        let ok = verify_transcript(&self.big_x, &t);
        self.forged = Some((t, ok));
        if ok {
            host.sfx("unlock");
            host.status("It verifies, and you never touched x. Now: can anyone tell yours from an honest one?", Tone::Ok);
        } else {
            host.sfx("error");
            host.status("That R does not satisfy s·G = R + c·X. Solve that equation for R.", Tone::Bad);
        }
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
        host.status("First forge a transcript yourself. Then guess real or simulated, six rounds.", Tone::Info);
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
        let mut roll = false;
        let mut forge = false;
        card(ui, Some("Forge one yourself"), None, |ui| {
            note(ui, "Be the simulator. You have the public key X and nothing else. Choose the challenge and the answer first, then make R fit.");
            if button_enabled(ui, !self.forged.as_ref().is_some_and(|f| f.1), "Pick c and s at random", Kind::Normal).clicked() {
                roll = true;
            }
            if let Some((c, s)) = &self.mine {
                egui::Grid::new("sim-mine").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                    for (k, v) in [("c", shorten(&hex32(c), 12, 10)), ("s", shorten(&hex32(s), 12, 10))] {
                        ui.label(RichText::new(k).font(mono(12.5)).color(DIM));
                        ui.label(RichText::new(v).font(mono(12.5)).color(TEXT));
                        ui.end_row();
                    }
                });
                math(ui, "the checkpoint accepts iff  s·G = R + c·X");
                formula(ui, PARTS, PALETTE, &mut self.forge);
                if button_enabled(ui, self.forge.complete() && !self.forged.as_ref().is_some_and(|f| f.1), "Build R and verify", Kind::Primary).clicked() {
                    forge = true;
                }
                if let Some((t, ok)) = &self.forged {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("R = {}", shorten(&to_hex(&compressed(&t.r_point)), 12, 10))).font(mono(12.5)).color(TEXT));
                        tag(ui, if *ok { "VALID" } else { "INVALID" }, if *ok { Tone::Ok } else { Tone::Bad });
                    });
                }
            }
        });
        if roll {
            self.roll_mine();
        }
        if forge {
            self.forge_one(host);
        }
        if !self.forged.as_ref().is_some_and(|f| f.1) {
            return;
        }
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
        if !self.forged.as_ref().is_some_and(|f| f.1) {
            self.roll_mine();
            self.forge.set(&[0, 1]);
            self.forge_one(host);
        }
        // answer each round (we happen to know, since we built it) and move on
        while !self.done {
            let truth = self.current.as_ref().map(|c| c.1).unwrap_or(true);
            self.guess(truth, host);
            self.advance(host);
        }
    }

    fn steps(&self) -> Vec<StepView> {
        let forged = self.forged.as_ref().is_some_and(|f| f.1);
        let rounds = format!("Real or simulated? ({} of {TOTAL})", self.round.min(TOTAL));
        steps(&[
            ("Pick c and s first, with no secret", "A simulator works backwards.", self.mine.is_some()),
            ("Solve the check for R", "s·G = R + c·X. Move c·X to the other side.", forged),
            (&rounds, "If you could tell them apart, the proof would leak something.", self.done),
        ])
    }

    fn deep_sheet(&self) -> Option<HandSheet> {
        let grp = TOY_GROUP;
        let mut rng = self.world.rng("ch5/sim-deep");
        let x = 2 + rng.int(500) as u64;
        let c = 2 + rng.int(500) as u64;
        let s = 2 + rng.int(500) as u64;
        let big_x = grp.public(x);
        let gs = grp.public(s);
        let xc = grp.pow(big_x, c);
        let inv = inv_mod_i64(xc as i64, grp.p as i64).unwrap_or(1) as u64;
        let r = gs * inv % grp.p;
        debug_assert_eq!(r * xc % grp.p, gs);
        Some(HandSheet::new(
            "a forgery in a toy group",
            &format!("Mod p = {} with g = {}, the check reads g^s = R · X^c. The public key is X = {big_x}. You pick c = {c} and s = {s}, and never learn x.", grp.p, grp.g),
            240.0,
            vec![
                HandCell::new(format!("g^s mod {}", grp.p), "", gs as i64),
                HandCell::new(format!("X^c mod {}", grp.p), "", xc as i64),
                HandCell::new(format!("(X^c)⁻¹ mod {}", grp.p), "extended Euclid", inv as i64),
                HandCell::new(format!("R = g^s · (X^c)⁻¹ mod {}", grp.p), "your forged commitment", r as i64),
                HandCell::new(format!("check: R · X^c mod {}", grp.p), "equals g^s, so it verifies", gs as i64),
            ],
        ))
    }
}
