//! Chapter 5, part 3. The checkpoint's own prover is careless: it answers two
//! different challenges using the same commitment R. Two accepting
//! transcripts (R, c1, s1), (R, c2, s2) yield x = (s1 − s2)/(c1 − c2): the
//! extractor, which is also the soundness proof. Then Fiat–Shamir turns the
//! interactive proof into a signature, and the "weak" version shows why the
//! hash must bind the whole statement.

use std::rc::Rc;

use cpl_crypto::bytes::{Big, big, to_hex};
use cpl_crypto::drbg::Drbg;
use cpl_crypto::secp::{N, Pt, compressed, mul, x_only};
use cpl_crypto::sigma::{FsOptions, Transcript, extract, fs_challenge, fs_verify, respond, verify_transcript};
use egui::{Color32, RichText};

use crate::puzzle::{Host, Puzzle};
use crate::ui::theme::{self, AMBER, DIM, GREEN, RED, TEXT, VIOLET, mono};
use crate::ui::widgets::{Kind, Log, Tone, button, card, lede, math, note, shorten, tag};
use crate::world::{World, hex32};

struct Forged {
    c: Big,
    proof: String,
    ok: bool,
    weak: bool,
    replay_ok: bool,
}

pub struct ExtractorFs {
    x: Big,
    big_x: Pt,
    t1: Transcript,
    t2: Transcript,
    both_verify: bool,
    extracted: Option<Big>,
    context: String,
    weak: bool,
    counter: u64,
    forged: Option<Forged>,
    log: Log,
}

fn grid_row(ui: &mut egui::Ui, k: &str, v: &str, color: Color32) {
    ui.label(RichText::new(k).font(mono(12.5)).color(DIM));
    ui.add(egui::Label::new(RichText::new(v).font(mono(12.5)).color(color)).wrap());
    ui.end_row();
}

impl ExtractorFs {
    pub fn new(world: Rc<World>) -> Self {
        let ck = world.checkpoint_key();
        let x = ck.x.clone();
        let big_x = mul(&x);
        let rp = mul(&ck.r);
        let c1 = big(7);
        let c2 = big(13) + (&x % big(9973));
        let t1 = Transcript { r_point: rp, s: respond(&ck.r, &c1, &x), c: c1 };
        let t2 = Transcript { r_point: rp, s: respond(&ck.r, &c2, &x), c: c2 };
        let both_verify = verify_transcript(&big_x, &t1) && verify_transcript(&big_x, &t2);
        let mut log = Log::default();
        log.line("Prover reused commitment R across challenges c₁ and c₂.", Tone::Warn);
        Self { x, big_x, t1, t2, both_verify, extracted: None, context: "checkpoint-7|clearance=all".into(), weak: false, counter: 0, forged: None, log }
    }

    fn do_extract(&mut self, host: &mut Host) {
        let Ok(x) = extract(&self.t1, &self.t2) else { return };
        let ok = x == self.x;
        self.log.line(format!("extracted x = (s₁ − s₂)/(c₁ − c₂) = {} · x·G == X ? {}", shorten(&hex32(&x), 8, 6), if ok { "YES" } else { "NO" }), if ok { Tone::Ok } else { Tone::Bad });
        self.extracted = Some(x);
        host.sfx("unlock");
        host.status("Key extracted. Now forge a non-interactive proof.", Tone::Ok);
    }

    fn forge(&mut self, host: &mut Host) {
        if host.solved() {
            return;
        }
        let Some(x) = self.extracted.clone() else { return };
        self.counter += 1;
        let r = Drbg::fresh(&format!("ch5/fs-{}", self.counter)).scalar(&N);
        let rp = mul(&r);
        let opts = FsOptions { include_key: true, include_context: !self.weak };
        let c = fs_challenge(&rp, &self.big_x, &self.context, opts);
        let s = respond(&r, &c, &x);
        let t = Transcript { r_point: rp, c: c.clone(), s: s.clone() };
        let ok = fs_verify(&self.big_x, &self.context, &t, opts);
        // the replay bug: a proof made for one context verifies for another
        let replay_ok = self.weak && fs_verify(&self.big_x, &format!("{}|DIFFERENT", self.context), &t, opts);
        let proof = format!("{} , {}", shorten(&to_hex(&x_only(&rp)), 8, 6), shorten(&s.to_str_radix(16), 8, 6));
        self.log.line(
            format!("{} proof for \"{}\": verifies {}{}", if self.weak { "weak" } else { "strong" }, self.context, if ok { "YES" } else { "NO" }, if self.weak { format!(", replays elsewhere {}", if replay_ok { "YES" } else { "no" }) } else { String::new() }),
            if self.weak { Tone::Warn } else if ok { Tone::Ok } else { Tone::Bad },
        );
        self.forged = Some(Forged { c, proof, ok, weak: self.weak, replay_ok });
        host.sfx(if self.weak { "error" } else { "confirm" });
        if !self.weak && ok {
            host.solve("A sound, non-interactive proof of clearance, bound to its context. You can pass every checkpoint now, and tell them nothing.");
        } else if self.weak {
            host.status("See the danger, then switch off weak mode to finish.", Tone::Warn);
        }
    }
}

impl Puzzle for ExtractorFs {
    fn id(&self) -> &'static str {
        "ch5.extract"
    }

    fn title(&self) -> &str {
        "REWIND THE WARDEN"
    }

    fn subtitle(&self) -> Option<&str> {
        Some("special soundness → extractor → Fiat–Shamir")
    }

    fn accent(&self) -> Color32 {
        VIOLET
    }

    fn open(&mut self, host: &mut Host) {
        host.status("Rewind the prover: extract its key.", Tone::Info);
    }

    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        lede(ui, "The checkpoint's master prover has a flaw: when you challenge it twice, it answers from the **same** commitment R. That is exactly the situation the soundness proof exploits. Two answers to two challenges reveal the witness.");
        let mut run = false;
        card(ui, Some("Two transcripts, one commitment"), None, |ui| {
            egui::Grid::new("xf-t").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                grid_row(ui, "R (shared)", &shorten(&to_hex(&compressed(&self.t1.r_point)), 10, 8), AMBER);
                grid_row(ui, "c₁, s₁", &format!("{} , {}", self.t1.c, shorten(&self.t1.s.to_str_radix(16), 8, 6)), TEXT);
                grid_row(ui, "c₂, s₂", &format!("{} , {}", self.t2.c, shorten(&self.t2.s.to_str_radix(16), 8, 6)), TEXT);
                ui.label(RichText::new("both verify").font(mono(12.5)).color(DIM));
                tag(ui, if self.both_verify { "YES" } else { "NO" }, if self.both_verify { Tone::Ok } else { Tone::Bad });
                ui.end_row();
            });
            math(ui, "s₁ = r + c₁x,  s₂ = r + c₂x   ⇒   x = (s₁ − s₂)/(c₁ − c₂)");
            if button(ui, "Extract the witness x", Kind::Primary).clicked() {
                run = true;
            }
            if let Some(x) = &self.extracted {
                let ok = *x == self.x;
                egui::Grid::new("xf-x").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                    ui.label(RichText::new("extracted x").font(mono(12.5)).color(DIM));
                    ui.label(RichText::new(shorten(&hex32(x), 12, 10)).font(mono(15.0)).color(GREEN));
                    ui.end_row();
                    ui.label(RichText::new("x·G == X ?").font(mono(12.5)).color(DIM));
                    tag(ui, if ok { "YES" } else { "NO" }, if ok { Tone::Ok } else { Tone::Bad });
                    ui.end_row();
                });
                note(ui, "This *is* the soundness proof: anyone who can answer two challenges for one commitment must know x. It is also the nonce-reuse attack from Chapter 4, wearing different notation.");
            }
        });
        if run {
            self.do_extract(host);
        }
        let mut forge = false;
        card(ui, Some("Make it non-interactive (Fiat–Shamir)"), None, |ui| {
            note(ui, "With x, you can produce proofs nobody needs to challenge: let the hash be the challenge, c = H(R ‖ X ‖ context). That is a Schnorr signature. But the hash must include everything.");
            if self.extracted.is_none() {
                return;
            }
            ui.horizontal(|ui| {
                ui.label(RichText::new("context:").font(mono(12.5)).color(DIM));
                ui.add(egui::TextEdit::singleline(&mut self.context).font(mono(13.5)).desired_width(f32::INFINITY));
            });
            ui.checkbox(&mut self.weak, RichText::new("use weak Fiat–Shamir (hash omits the context)").font(mono(12.5)).color(DIM));
            if button(ui, "Forge non-interactive proof", Kind::Primary).clicked() {
                forge = true;
            }
            if let Some(f) = &self.forged {
                egui::Grid::new("xf-fs").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                    grid_row(ui, if f.weak { "c = H(R‖X)" } else { "c = H(R‖X‖ctx)" }, &shorten(&hex32(&f.c), 10, 8), TEXT);
                    grid_row(ui, "proof (R.x, s)", &f.proof, GREEN);
                    ui.label(RichText::new("verifies for this context").font(mono(12.5)).color(DIM));
                    tag(ui, if f.ok { "YES" } else { "NO" }, if f.ok { Tone::Ok } else { Tone::Bad });
                    ui.end_row();
                });
                if f.weak {
                    theme::card_frame(Some(RED)).show(ui, |ui| {
                        note(ui, &format!("Weak Fiat–Shamir: the same proof also verifies for a *different* context ({}). Leaving the context out of the hash means a clearance for one gate works at every gate. This is the 2022 \"Frozen Heart\" class of bugs.", if f.replay_ok { "YES — replayable!" } else { "no" }));
                    });
                } else {
                    note(ui, "Strong Fiat–Shamir: the hash binds R, the key, and the context, so this proof is good for this context and no other.");
                }
            }
        });
        if forge {
            self.forge(host);
        }
        card(ui, Some("Transcript"), None, |ui| self.log.show(ui, "xf-log", 170.0));
    }

    fn autoplay(&mut self, host: &mut Host) {
        self.do_extract(host);
        self.weak = false;
        self.forge(host);
    }
}
