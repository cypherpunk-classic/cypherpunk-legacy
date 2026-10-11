//! Chapter 5, part 1. The Directorate checkpoint wants to see your key.
//! Instead you prove you *hold* it, revealing nothing, with the Schnorr
//! identification protocol: commit R = r·G, receive challenge c, answer
//! s = r + c·x; the verifier checks s·G = R + c·X. A full-size challenge makes
//! bluffing hopeless.

use std::rc::Rc;

use cpl_crypto::bytes::{Big, to_hex};
use cpl_crypto::drbg::Drbg;
use cpl_crypto::secp::{N, Pt, compressed, mul};
use cpl_crypto::sigma::{Transcript, commit, respond, verify_transcript};
use egui::{Color32, RichText};

use crate::puzzle::{Host, Puzzle};
use crate::ui::theme::{AMBER, DIM, RED, TEXT, VIOLET, mono};
use crate::ui::widgets::{Kind, Log, Tone, button, card, lede, math, note, shorten};
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
}

impl SigmaCheckpoint {
    pub fn new(world: Rc<World>) -> Self {
        let x = world.identity().x.clone();
        Self { big_x: mul(&x), x, pending: None, passed: 0, counter: 0, log: Log::default() }
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
        host.sfx("blip");
    }

    fn do_respond(&mut self, honest: bool, host: &mut Host) {
        if host.solved() {
            return;
        }
        let Some((r, rp, c)) = self.pending.take() else { return };
        let s = if honest { respond(&r, &c, &self.x) } else { self.fresh("bluff").scalar(&N) };
        let ok = verify_transcript(&self.big_x, &Transcript { r_point: rp, c, s: s.clone() });
        self.log.line(format!("s = {} · verify s·G = R + c·X → {}", shorten(&s.to_str_radix(16), 8, 6), if ok { "PASS" } else { "FAIL" }), if ok { Tone::Ok } else { Tone::Bad });
        if !ok {
            host.sfx("error");
            self.passed = 0;
            host.status("Verification failed. A bluff against a full-size challenge has no chance. Start over.", Tone::Bad);
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
                    note(ui, "Answer with s = r + c·x. The checkpoint will verify s·G = R + c·X.");
                    ui.horizontal_wrapped(|ui| {
                        if button(ui, "Respond s = r + c·x", Kind::Primary).clicked() {
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
        for _ in 0..NEEDED {
            self.do_commit(host);
            self.do_respond(true, host);
        }
    }
}
