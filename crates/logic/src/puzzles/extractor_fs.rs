//! Chapter 5, part 3. The checkpoint's master prover is on your bench, as a
//! virtual machine you can snapshot and restore. Challenge it, rewind it to
//! just after its commitment, challenge it again: two accepting transcripts
//! (R, c1, s1), (R, c2, s2) give x = (s1 − s2)/(c1 − c2). That is the
//! extractor, which is also the soundness proof. Then Fiat–Shamir turns the
//! interactive proof into a signature, and an auditor shows what each field
//! left out of the hash lets someone else do.

use std::rc::Rc;

use cpl_crypto::bytes::{Big, big, bytes_to_big, inv_mod_i64, to_hex};
use cpl_crypto::dh::TOY_GROUP;
use cpl_crypto::drbg::Drbg;
use cpl_crypto::secp::{N, Pt, compressed, g, inv_n, mod_n, mul, mul_pt, tagged_hash, x_only};
use cpl_crypto::sigma::{Transcript, respond, verify_transcript};
use egui::{Color32, RichText};

use crate::puzzle::{HandCell, HandSheet, Host, Puzzle, StepView, steps};
use crate::ui::theme::{AMBER, DIM, GREEN, RED, TEXT, VIOLET, mono};
use crate::ui::widgets::{Formula, Kind, Log, Part, Tone, button, button_enabled, card, formula, lede, math, note, select_card, shorten, tag};
use crate::world::{World, hex32};

const PARTS: &[Part] = &[Part::Text("x = ("), Part::Slot, Part::Text(" − "), Part::Slot, Part::Text(") / ("), Part::Slot, Part::Text(" − "), Part::Slot, Part::Text(")  mod n")];
const PALETTE: &[&str] = &["s₁", "s₂", "c₁", "c₂", "R"];
const CONTEXT: &str = "checkpoint-7|clearance=all";
const OTHER_CONTEXT: &str = "checkpoint-9|clearance=all";

/// The Fiat–Shamir challenge over whichever fields are included.
fn fs_c(r: Option<&Pt>, x: Option<&Pt>, ctx: Option<&str>) -> Big {
    let rb = r.map(|p| compressed(p).to_vec()).unwrap_or_default();
    let xb = x.map(|p| compressed(p).to_vec()).unwrap_or_default();
    let cb = ctx.map(|c| c.as_bytes().to_vec()).unwrap_or_default();
    mod_n(&bytes_to_big(&tagged_hash("cypherpunk/fiat-shamir", &[&rb, &xb, &cb])))
}

/// What the hash covers.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Fields {
    r: bool,
    x: bool,
    ctx: bool,
}

impl Fields {
    fn challenge(&self, r: &Pt, x: &Pt, ctx: &str) -> Big {
        fs_c(self.r.then_some(r), self.x.then_some(x), self.ctx.then_some(ctx))
    }
}

/// The auditor's report on one proof.
struct Audit {
    fields: Fields,
    proof: (Pt, Big),
    valid: bool,
    replay: bool,
    forged_without_x: bool,
    related_key: bool,
}

impl Audit {
    fn clean(&self) -> bool {
        self.valid && !self.replay && !self.forged_without_x && !self.related_key
    }
}

pub struct ExtractorFs {
    world: Rc<World>,
    x: Big,
    big_x: Pt,
    /// the prover's current nonce, its commitment, and whether it has answered from it
    r: Big,
    big_r: Pt,
    answered: bool,
    restarts: u64,
    rewinds: u32,
    challenge_input: String,
    transcripts: Vec<Transcript>,
    picked: Vec<usize>,
    solve_for: Formula,
    extracted: Option<(Big, bool)>,
    fields: Fields,
    audit: Option<Audit>,
    counter: u64,
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
        let r = ck.r.clone();
        let mut log = Log::default();
        log.line("Prover VM booted. It commits R = r·G and snapshots itself.", Tone::Dim);
        Self {
            world,
            x,
            big_x,
            big_r: mul(&r),
            r,
            answered: false,
            restarts: 0,
            rewinds: 0,
            challenge_input: "7".into(),
            transcripts: Vec::new(),
            picked: Vec::new(),
            solve_for: Formula::new(PARTS),
            extracted: None,
            fields: Fields { r: true, x: false, ctx: false },
            audit: None,
            counter: 0,
            log,
        }
    }

    fn restart(&mut self) {
        self.restarts += 1;
        self.r = self.world.rng(&format!("ch5/prover-restart-{}", self.restarts)).scalar(&N);
        self.big_r = mul(&self.r);
        self.answered = false;
        self.log.line(format!("Prover starts over: fresh r, new R = {}", shorten(&to_hex(&compressed(&self.big_r)), 8, 6)), Tone::Dim);
    }

    fn challenge(&mut self, host: &mut Host) {
        let Ok(c) = self.challenge_input.trim().parse::<u64>() else {
            host.status("A challenge is a number.", Tone::Warn);
            return;
        };
        if self.answered {
            // an honest prover never answers twice from one commitment
            self.log.line("It already answered from this R, so it moves on to a fresh commitment.", Tone::Warn);
            self.restart();
        }
        let c = big(c as i64);
        let s = respond(&self.r, &c, &self.x);
        let t = Transcript { r_point: self.big_r, c: c.clone(), s: s.clone() };
        debug_assert!(verify_transcript(&self.big_x, &t));
        self.log.line(format!("challenge c = {c} → s = {}", shorten(&s.to_str_radix(16), 8, 6)), Tone::Info);
        self.transcripts.push(t);
        self.answered = true;
        host.sfx("blip");
        // suggest a different challenge next time
        self.challenge_input = (c.to_string().parse::<u64>().unwrap_or(7) + 6).to_string();
    }

    fn rewind(&mut self, host: &mut Host) {
        self.answered = false;
        self.rewinds += 1;
        self.log.line("Restored the snapshot: the prover is back just after committing R, with the same r in its memory.", Tone::Warn);
        host.sfx("glitch");
    }

    fn pick(&mut self, i: usize) {
        if let Some(at) = self.picked.iter().position(|p| *p == i) {
            self.picked.remove(at);
        } else {
            if self.picked.len() == 2 {
                self.picked.remove(0);
            }
            self.picked.push(i);
        }
    }

    fn pair_ok(&self) -> bool {
        self.picked.len() == 2 && {
            let (a, b) = (&self.transcripts[self.picked[0]], &self.transcripts[self.picked[1]]);
            a.r_point == b.r_point && a.c != b.c
        }
    }

    fn extract(&mut self, host: &mut Host) {
        if self.picked.len() != 2 {
            host.status("Pick two transcripts first: they become ₁ and ₂.", Tone::Warn);
            return;
        }
        let toks: Vec<usize> = self.solve_for.tokens().into_iter().flatten().collect();
        if toks.len() != 4 {
            return;
        }
        if toks.contains(&4) {
            host.status("R is a point, not a number. x comes out of the s and c values.", Tone::Bad);
            host.sfx("error");
            return;
        }
        let (t1, t2) = (&self.transcripts[self.picked[0]], &self.transcripts[self.picked[1]]);
        let val = |t: usize| match t {
            0 => t1.s.clone(),
            1 => t2.s.clone(),
            2 => t1.c.clone(),
            _ => t2.c.clone(),
        };
        let num = mod_n(&(val(toks[0]) + &*N - val(toks[1])));
        let den = mod_n(&(val(toks[2]) + &*N - val(toks[3])));
        if den == Big::from(0u8) {
            host.status("The denominator is zero: the two challenges must differ.", Tone::Bad);
            host.sfx("error");
            return;
        }
        let x = mod_n(&(num * inv_n(&den)));
        let ok = mul(&x) == self.big_x;
        self.log.line(format!("x = {} · x·G == X ? {}", shorten(&hex32(&x), 8, 6), if ok { "YES" } else { "NO" }), if ok { Tone::Ok } else { Tone::Bad });
        self.extracted = Some((x, ok));
        if ok {
            host.sfx("unlock");
            host.status("The witness, extracted. Anyone who can answer two challenges for one commitment must know x: that is soundness.", Tone::Ok);
        } else if t1.r_point != t2.r_point {
            host.sfx("error");
            host.status("Those two transcripts have different R, so different r, and r does not cancel. Rewind to get two answers for one R.", Tone::Bad);
        } else {
            host.sfx("error");
            host.status("x·G ≠ X. Subtract s₂ = r + c₂·x from s₁ = r + c₁·x, then divide.", Tone::Bad);
        }
    }

    /// Make a proof with the extracted x and let the auditor try everything.
    fn prove(&mut self, host: &mut Host) {
        if host.solved() {
            return;
        }
        let Some((x, true)) = self.extracted.clone() else { return };
        self.counter += 1;
        let f = self.fields;
        let r = Drbg::fresh(&format!("ch5/fs-{}", self.counter)).scalar(&N);
        let big_r = mul(&r);
        let c = f.challenge(&big_r, &self.big_x, CONTEXT);
        let s = mod_n(&(r + &c * &x));
        let check = |rp: &Pt, s: &Big, xp: &Pt, ctx: &str| mul(s) == *rp + mul_pt(&f.challenge(rp, xp, ctx), xp);
        let valid = check(&big_r, &s, &self.big_x, CONTEXT);
        // the same proof shown at another checkpoint
        let replay = check(&big_r, &s, &self.big_x, OTHER_CONTEXT);
        // a forger with no x: if R is not hashed, c is known before R is chosen
        let forged_without_x = !f.r && {
            let fs = Drbg::fresh(&format!("ch5/fs-forger-{}", self.counter)).scalar(&N);
            let fc = f.challenge(&g(), &self.big_x, CONTEXT); // R is ignored by this hash
            let fr = mul(&fs) - mul_pt(&fc, &self.big_x);
            check(&fr, &fs, &self.big_x, CONTEXT)
        };
        // a related key X' = X + t·G: if X is not hashed, s' = s + c·t works for X'
        let related_key = !f.x && {
            let t = big(1234567);
            let x2 = self.big_x + mul(&t);
            let s2 = mod_n(&(&s + &c * &t));
            check(&big_r, &s2, &x2, CONTEXT)
        };
        let audit = Audit { fields: f, proof: (big_r, s), valid, replay, forged_without_x, related_key };
        let clean = audit.clean();
        self.log.line(
            format!(
                "proof with c = H({}): valid {}, replays at checkpoint 9 {}, forgeable without x {}, works for a related key {}",
                [(f.r, "R"), (f.x, "X"), (f.ctx, "ctx")].iter().filter(|(on, _)| *on).map(|(_, n)| *n).collect::<Vec<_>>().join("‖"),
                yes(valid),
                yes(audit.replay),
                yes(audit.forged_without_x),
                yes(audit.related_key)
            ),
            if clean { Tone::Ok } else { Tone::Warn },
        );
        self.audit = Some(audit);
        if clean {
            host.sfx("unlock");
            host.solve("A sound, non-interactive proof of clearance, bound to this checkpoint and this key. You can pass every gate now, and tell them nothing. (It is a Schnorr signature, and this is why BIP-340 hashes R, the key, and the message.)");
        } else {
            host.sfx("error");
            host.status("The auditor got through. Cover what it exploited and prove again.", Tone::Warn);
        }
    }
}

fn yes(b: bool) -> &'static str {
    if b { "YES" } else { "no" }
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
        host.status("Get two answers to one commitment. The snapshot is your friend.", Tone::Info);
    }

    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        lede(ui, "Lin lifted the checkpoint's master prover onto your bench as a virtual machine. It proves it knows the clearance key x, honestly: commit R = r·G, take a challenge, answer s = r + c·x. An honest prover never answers twice from one R. But a virtual machine can be **rewound**.");
        let mut act: Option<u8> = None;
        card(ui, Some("The prover"), None, |ui| {
            egui::Grid::new("xf-p").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                grid_row(ui, "public key X", &shorten(&to_hex(&compressed(&self.big_x)), 10, 8), TEXT);
                grid_row(ui, "current commitment R", &shorten(&to_hex(&compressed(&self.big_r)), 10, 8), AMBER);
                grid_row(ui, "state", if self.answered { "answered (snapshot available)" } else { "committed, waiting for a challenge" }, if self.answered { DIM } else { GREEN });
            });
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("challenge c =").font(mono(13.0)).color(DIM));
                ui.add(egui::TextEdit::singleline(&mut self.challenge_input).desired_width(80.0).font(mono(14.0)));
                if button(ui, "Send challenge", Kind::Primary).clicked() {
                    act = Some(0);
                }
                if button_enabled(ui, self.answered, "Restore snapshot (rewind)", Kind::Normal).clicked() {
                    act = Some(1);
                }
                if button(ui, "Reboot (new R)", Kind::Ghost).clicked() {
                    act = Some(2);
                }
            });
        });
        match act {
            Some(0) => self.challenge(host),
            Some(1) => self.rewind(host),
            Some(2) => self.restart(),
            _ => {}
        }
        let mut picked = None;
        let mut extract = false;
        card(ui, Some("Transcripts · pick two"), None, |ui| {
            if self.transcripts.is_empty() {
                note(ui, "No answers yet.");
            }
            for (i, t) in self.transcripts.iter().enumerate() {
                let slot = self.picked.iter().position(|p| *p == i).map(|k| if k == 0 { "₁" } else { "₂" });
                if select_card(ui, slot.is_some(), VIOLET, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("R {}", shorten(&to_hex(&compressed(&t.r_point)), 6, 4))).font(mono(12.5)).color(AMBER));
                        ui.label(RichText::new(format!("c = {}", t.c)).font(mono(12.5)).color(TEXT));
                        ui.label(RichText::new(format!("s = {}", shorten(&t.s.to_str_radix(16), 6, 4))).font(mono(12.5)).color(TEXT));
                        if let Some(k) = slot {
                            ui.label(RichText::new(k).font(mono(13.0)).color(VIOLET));
                        }
                    });
                }) {
                    picked = Some(i);
                }
            }
            if self.picked.len() == 2 {
                let ok = self.pair_ok();
                tag(ui, if ok { "same R, different c" } else { "not one commitment" }, if ok { Tone::Ok } else { Tone::Warn });
            }
            math(ui, "s₁ = r + c₁·x,   s₂ = r + c₂·x");
            formula(ui, PARTS, PALETTE, &mut self.solve_for);
            if button_enabled(ui, self.solve_for.complete() && self.picked.len() == 2, "Extract x and check x·G", Kind::Primary).clicked() {
                extract = true;
            }
            if let Some((x, ok)) = &self.extracted {
                egui::Grid::new("xf-x").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                    ui.label(RichText::new("extracted x").font(mono(12.5)).color(DIM));
                    ui.label(RichText::new(shorten(&hex32(x), 12, 10)).font(mono(15.0)).color(if *ok { GREEN } else { RED }));
                    ui.end_row();
                    ui.label(RichText::new("x·G == X ?").font(mono(12.5)).color(DIM));
                    tag(ui, if *ok { "YES" } else { "NO" }, if *ok { Tone::Ok } else { Tone::Bad });
                    ui.end_row();
                });
                if *ok {
                    note(ui, "This *is* the soundness proof: anyone who can answer two challenges for one commitment must know x. It is also the nonce-reuse attack from Chapter 4, wearing different notation.");
                }
            }
        });
        if let Some(i) = picked {
            self.pick(i);
        }
        if extract {
            self.extract(host);
        }
        let mut prove = false;
        card(ui, Some("Make it non-interactive (Fiat–Shamir)"), None, |ui| {
            note(ui, "With x, you can produce proofs nobody needs to challenge: let a hash be the challenge. Choose what the hash covers. A Directorate auditor will try to abuse every field you leave out.");
            if !self.extracted.as_ref().is_some_and(|e| e.1) {
                return;
            }
            ui.horizontal_wrapped(|ui| {
                ui.checkbox(&mut self.fields.r, RichText::new("the commitment R").font(mono(12.5)));
                ui.checkbox(&mut self.fields.x, RichText::new("the public key X").font(mono(12.5)));
                ui.checkbox(&mut self.fields.ctx, RichText::new(format!("the context “{CONTEXT}”")).font(mono(12.5)));
            });
            if button_enabled(ui, !host.solved(), "Make a proof and audit it", Kind::Primary).clicked() {
                prove = true;
            }
            if let Some(a) = &self.audit {
                let (rp, s) = &a.proof;
                egui::Grid::new("xf-fs").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                    grid_row(ui, "proof (R.x, s)", &format!("{} , {}", shorten(&to_hex(&x_only(rp)), 8, 6), shorten(&s.to_str_radix(16), 8, 6)), TEXT);
                    let rows = [
                        ("verifies at checkpoint 7", a.valid, true),
                        ("replays at checkpoint 9", a.replay, false),
                        ("someone forges one without x", a.forged_without_x, false),
                        ("works for a related key X + t·G", a.related_key, false),
                    ];
                    for (label, v, want) in rows {
                        ui.label(RichText::new(label).font(mono(12.5)).color(DIM));
                        tag(ui, if v { "YES" } else { "NO" }, if v == want { Tone::Ok } else { Tone::Bad });
                        ui.end_row();
                    }
                });
                if !a.fields.ctx {
                    note(ui, "Without the context in the hash, a clearance for one gate works at every gate.");
                }
                if !a.fields.r {
                    note(ui, "Without R in the hash, c is known before R is chosen: pick s, set R = s·G − c·X, and you have a proof with no x at all. (The 2022 \"Frozen Heart\" bugs were this.)");
                }
                if !a.fields.x {
                    note(ui, "Without the key in the hash, the same R and c serve any key X + t·G, with s + c·t. BIP-340 prefixes the key for exactly this reason.");
                }
            }
        });
        if prove {
            self.prove(host);
        }
        card(ui, Some("Bench log"), None, |ui| self.log.show(ui, "xf-log", 170.0));
    }

    fn autoplay(&mut self, host: &mut Host) {
        if self.extracted.as_ref().is_none_or(|e| !e.1) {
            self.challenge_input = "7".into();
            self.challenge(host);
            self.rewind(host);
            self.challenge_input = "13".into();
            self.challenge(host);
            let n = self.transcripts.len();
            self.picked = vec![n - 2, n - 1];
            self.solve_for.set(&[0, 1, 2, 3]);
            self.extract(host);
        }
        self.fields = Fields { r: true, x: true, ctx: true };
        self.prove(host);
    }

    fn steps(&self) -> Vec<StepView> {
        let same_r = self.transcripts.iter().enumerate().any(|(i, a)| self.transcripts.iter().skip(i + 1).any(|b| a.r_point == b.r_point && a.c != b.c));
        let extracted = self.extracted.as_ref().is_some_and(|e| e.1);
        steps(&[
            ("Get two answers for one commitment", "Challenge, restore the snapshot, challenge again with a different c.", same_r || extracted),
            ("Extract x from that pair", "Pick the two transcripts with the same R. Subtract, then divide.", extracted),
            ("Choose what the Fiat–Shamir hash covers", "Each field you leave out, the auditor will use.", self.audit.is_some()),
            ("Make a proof only you could make, for this gate only", "", self.audit.as_ref().is_some_and(Audit::clean)),
        ])
    }

    fn deep_sheet(&self) -> Option<HandSheet> {
        let q = TOY_GROUP.q as i64;
        let mut rng = self.world.rng("ch5/extract-deep");
        let x = 2 + rng.int(500) as i64;
        let r = 2 + rng.int(500) as i64;
        let c1 = 2 + rng.int(200) as i64;
        let c2 = c1 + 1 + rng.int(200) as i64;
        let s1 = (r + c1 * x) % q;
        let s2 = (r + c2 * x) % q;
        let ds = (s1 - s2).rem_euclid(q);
        let dc = (c1 - c2).rem_euclid(q);
        let inv = inv_mod_i64(dc, q).unwrap_or(1);
        let got = ds * inv % q;
        debug_assert_eq!(got, x);
        Some(HandSheet::new(
            "the extractor by hand",
            format!("Exponents mod q = {q}. One commitment, two answers: (c₁ = {c1}, s₁ = {s1}) and (c₂ = {c2}, s₂ = {s2})."),
            240.0,
            vec![
                HandCell::new(format!("s₁ − s₂ mod {q}"), "", ds),
                HandCell::new(format!("c₁ − c₂ mod {q}"), "", dc),
                HandCell::new(format!("(c₁ − c₂)⁻¹ mod {q}"), "extended Euclid", inv),
                HandCell::new(format!("x mod {q}"), "", got),
                HandCell::new(format!("r = s₁ − c₁·x mod {q}"), "the prover's nonce", r),
            ],
        ))
    }
}
