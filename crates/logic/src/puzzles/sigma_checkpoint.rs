//! Chapter 5, part 1. The Directorate checkpoint wants to see your key.
//! Instead you prove you *hold* it, revealing nothing, with the Schnorr
//! identification protocol: commit R = r·G, receive challenge c, answer
//! s = r + c·x; the verifier checks s·G = R + c·X. The player builds the
//! response formula; the terminal evaluates it with the real numbers. A
//! full-size challenge makes bluffing hopeless.

use std::rc::Rc;

use cpl_crypto::bytes::{Big, to_hex};
use cpl_crypto::drbg::Drbg;
use cpl_crypto::secp::{N, Pt, compressed, mod_n, mul};
use cpl_crypto::sigma::{Transcript, commit, verify_transcript};
use egui::{Color32, RichText};

use crate::puzzle::{HandCell, HandSheet, Host, Puzzle, StepView, steps};
use crate::ui::theme::{AMBER, DIM, RED, TEXT, VIOLET, mono};
use crate::ui::widgets::{Formula, Kind, Log, Part, Tone, button, button_enabled, card, formula, lede, math, note, shorten};

const PARTS: &[Part] = &[Part::Text("s = "), Part::Slot, Part::Text(" + "), Part::Slot, Part::Text(" · "), Part::Slot, Part::Text("   mod n")];
const PALETTE: &[&str] = &["r", "c", "x", "R", "X"];
use crate::world::{World, hex32};

const NEEDED: u32 = 3;

pub struct SigmaCheckpoint {
    x: Big,
    big_x: Pt,
    /// the round in progress: (r, R, c)
    pending: Option<(Big, Pt, Big)>,
    passed: u32,
    counter: u64,
    log: Log,
    response: Formula,
    built_ok: bool,
    committed_once: bool,
}

impl SigmaCheckpoint {
    pub fn new(world: Rc<World>) -> Self {
        let x = world.identity().x.clone();
        Self { big_x: mul(&x), x, pending: None, passed: 0, counter: 0, log: Log::default(), response: Formula::new(PARTS), built_ok: false, committed_once: false }
    }

    fn fresh(&mut self, what: &str) -> Drbg {
        self.counter += 1;
        Drbg::fresh(&format!("ch5/{what}-{}-{}", self.passed, self.counter))
    }

    fn do_commit(&mut self, host: &mut Host) {
        let (r, rp) = commit(&mut self.fresh("sigma"));
        let c = self.fresh("chal").scalar(&N);
        self.log.line(format!("round {}: R = {}", self.passed + 1, shorten(&to_hex(&compressed(&rp)), 8, 6)), Tone::Dim);
        self.log.line(format!("checkpoint challenges c = {}", shorten(&c.to_str_radix(16), 8, 6)), Tone::Info);
        self.pending = Some((r, rp, c));
        self.committed_once = true;
        host.sfx("blip");
    }

    /// The player's response formula, evaluated with this round's numbers.
    fn evaluate(&self, r: &Big, c: &Big) -> Result<Big, String> {
        let toks = self.response.tokens();
        let (Some(a), Some(b), Some(d)) = (toks[0], toks[1], toks[2]) else { return Err("Build the response first.".into()) };
        if [a, b, d].iter().any(|t| *t >= 3) {
            return Err("R and X are points. s is a number mod n: build it from r, c and x.".into());
        }
        let val = |t: usize| match t {
            0 => r.clone(),
            1 => c.clone(),
            _ => self.x.clone(),
        };
        Ok(mod_n(&(val(a) + val(b) * val(d))))
    }

    fn do_respond(&mut self, honest: bool, host: &mut Host) {
        if host.solved() {
            return;
        }
        let Some((r, rp, c)) = self.pending.clone() else { return };
        let s = if honest {
            match self.evaluate(&r, &c) {
                Ok(s) => s,
                Err(e) => {
                    host.status(e, Tone::Bad);
                    host.sfx("error");
                    return;
                }
            }
        } else {
            self.fresh("bluff").scalar(&N)
        };
        self.pending = None;
        let ok = verify_transcript(&self.big_x, &Transcript { r_point: rp, c, s: s.clone() });
        if honest {
            self.built_ok = ok;
        }
        self.log.line(format!("s = {} · verify s·G = R + c·X → {}", shorten(&s.to_str_radix(16), 8, 6), if ok { "PASS" } else { "FAIL" }), if ok { Tone::Ok } else { Tone::Bad });
        if !ok {
            host.sfx("error");
            self.passed = 0;
            let why = if honest { "Verification failed: s·G ≠ R + c·X. Expand R = r·G and X = x·G: which s makes both sides agree? The count starts over." } else { "Verification failed. A bluff against a full-size challenge has no chance. Start over." };
            host.status(why, Tone::Bad);
            return;
        }
        host.sfx("confirm");
        self.passed += 1;
        if self.passed >= NEEDED {
            host.solve("Checkpoint cleared. It is convinced you hold the key, and it has learned nothing about it.");
            return;
        }
        host.status(format!("Round passed ({}/{NEEDED}). The checkpoint still knows nothing about x.", self.passed), Tone::Ok);
    }
}

impl Puzzle for SigmaCheckpoint {
    fn id(&self) -> &'static str {
        "ch5.sigma"
    }

    fn title(&self) -> &str {
        "CHECKPOINT · IDENTIFY YOURSELF"
    }

    fn subtitle(&self) -> Option<&str> {
        Some("Schnorr identification (honest prover)")
    }

    fn accent(&self) -> Color32 {
        VIOLET
    }

    fn open(&mut self, host: &mut Host) {
        host.status("Pass three rounds to clear the checkpoint.", Tone::Info);
    }

    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        lede(ui, "The checkpoint has your public key X on file. The old way: hand over your private key so it can check. You will not. Instead you prove you know x such that X = x·G, and the checkpoint learns nothing else.");
        math(ui, "commit R = r·G  →  challenge c  →  respond s = r + c·x.   Accept iff s·G = R + c·X");
        card(ui, Some("Public"), None, |ui| {
            egui::Grid::new("sc-pub").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                ui.label(RichText::new("your public key X").font(mono(12.5)).color(DIM));
                ui.label(RichText::new(shorten(&to_hex(&compressed(&self.big_x)), 10, 8)).font(mono(12.5)).color(TEXT));
                ui.end_row();
                ui.label(RichText::new("your secret x").font(mono(12.5)).color(DIM));
                ui.label(RichText::new("never transmitted").font(mono(12.5)).color(RED));
                ui.end_row();
            });
        });
        let mut action: Option<u8> = None;
        let title = format!("Round {} of {NEEDED}", (self.passed + 1).min(NEEDED));
        card(ui, Some(&title), None, |ui| {
            if host.solved() {
                note(ui, "All three rounds passed.");
                return;
            }
            match &self.pending {
                None => {
                    note(ui, "Pick a fresh random nonce r and send its commitment R = r·G. (Never reuse r: that is the whole of Chapter 4.)");
                    if button(ui, "Generate nonce and commit R", Kind::Primary).clicked() {
                        action = Some(0);
                    }
                }
                Some((_, rp, c)) => {
                    egui::Grid::new("sc-round").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                        ui.label(RichText::new("your R").font(mono(12.5)).color(DIM));
                        ui.label(RichText::new(shorten(&to_hex(&compressed(rp)), 10, 8)).font(mono(12.5)).color(TEXT));
                        ui.end_row();
                        ui.label(RichText::new("challenge c").font(mono(12.5)).color(DIM));
                        ui.label(RichText::new(shorten(&hex32(c), 10, 8)).font(mono(12.5)).color(AMBER));
                        ui.end_row();
                    });
                    note(ui, "Answer with a number s. The checkpoint will verify s·G = R + c·X, knowing only R, c and X.");
                    formula(ui, PARTS, PALETTE, &mut self.response);
                    ui.horizontal_wrapped(|ui| {
                        if button_enabled(ui, self.response.complete(), "Send s", Kind::Primary).clicked() {
                            action = Some(1);
                        }
                        if button(ui, "Try to bluff (random s)", Kind::Ghost).clicked() {
                            action = Some(2);
                        }
                    });
                }
            }
        });
        card(ui, Some("Transcript"), None, |ui| self.log.show(ui, "sc-log", 190.0));
        match action {
            Some(0) => self.do_commit(host),
            Some(1) => self.do_respond(true, host),
            Some(2) => self.do_respond(false, host),
            _ => {}
        }
    }

    fn autoplay(&mut self, host: &mut Host) {
        self.response.set(&[0, 1, 2]);
        for _ in 0..2 * NEEDED {
            if host.solved() {
                break;
            }
            if self.pending.is_none() {
                self.do_commit(host);
            }
            self.do_respond(true, host);
        }
    }

    fn steps(&self) -> Vec<StepView> {
        let rounds = format!("Pass three rounds in a row ({}/{NEEDED})", self.passed.min(NEEDED));
        steps(&[
            ("Commit: a fresh r, and R = r·G", "Never reuse r. That was all of Chapter 4.", self.committed_once),
            ("Answer: build s from r, c and x", "The check is s·G = R + c·X. Write R as r·G and X as x·G.", self.built_ok),
            (&rounds, "Each round needs a new commitment.", self.passed >= NEEDED),
        ])
    }

    fn deep_sheet(&self) -> Option<HandSheet> {
        Some(HandSheet::new(
            "how lucky does a bluffer get",
            "A prover without x can still pass one round if it guesses the challenge before committing (pick s, then R = s·G − c·X). With a challenge of b bits, a guess is right with odds 1 in 2^b. Answer each as N, for odds of 1 in N.",
            150.0,
            vec![
                HandCell::new("1-bit challenges, one round", "", 2),
                HandCell::new("1-bit challenges, three rounds in a row", "", 8),
                HandCell::new("2-bit challenges, three rounds", "", 64),
                HandCell::new("1-bit rounds needed for odds under one in a million", "2^20 = 1,048,576", 20),
                HandCell::new("this checkpoint, one round: 2^?", "c is a 256-bit number", 256),
            ],
        ))
    }
}
