//! Chapter 4, part 1. Warden's signing device reused a nonce across two
//! orders. Two signatures with the same R: s1 = k + e1·d, s2 = k + e2·d, so
//! d = (s1 − s2)/(e1 − e2). Recover the key, then sign a new order of your own.

use std::rc::Rc;

use cpl_crypto::bip340::{NonceReuseSolution, challenge, even_key, recover_from_nonce_reuse, sign, verify};
use cpl_crypto::bytes::{Big, to_hex};
use egui::{Color32, RichText};

use crate::puzzle::{Host, Puzzle};
use crate::ui::theme::{AMBER, DIM, GREEN, RED, TEXT, mono};
use crate::ui::widgets::{Kind, Tone, button, card, columns2, hex_box, lede, math, note, shorten, tag};
use crate::world::{World, hex32};

const DEFAULT_ORDER: &str = "DLA ORDER 0419: UNSEAL THE ARCHIVE. RELEASE TO PUBLIC. AUTH: WARDEN";

pub struct NonceReuse {
    world: Rc<World>,
    e1: Big,
    e2: Big,
    same_r: bool,
    ok1: bool,
    ok2: bool,
    solution: Option<(NonceReuseSolution, bool)>,
    forge_text: String,
    forged: Option<(String, bool)>,
}

/// One row of a label/value grid, with an optional coloured value.
fn row(ui: &mut egui::Ui, k: &str, v: &str, color: Color32) {
    ui.label(RichText::new(k).font(mono(12.5)).color(DIM));
    ui.label(RichText::new(v).font(mono(12.5)).color(color));
    ui.end_row();
}

impl NonceReuse {
    pub fn new(world: Rc<World>) -> Self {
        let w = world.warden();
        let e1 = challenge(&w.sig1.r, &w.p, w.m1.as_bytes());
        let e2 = challenge(&w.sig2.r, &w.p, w.m2.as_bytes());
        let same_r = w.sig1.r == w.sig2.r;
        let ok1 = verify(&w.sig1, w.m1.as_bytes(), &w.p);
        let ok2 = verify(&w.sig2, w.m2.as_bytes(), &w.p);
        Self { world, e1, e2, same_r, ok1, ok2, solution: None, forge_text: DEFAULT_ORDER.into(), forged: None }
    }

    fn recover(&mut self, host: &mut Host) {
        let w = self.world.warden();
        let Ok(sol) = recover_from_nonce_reuse(&w.p, w.m1.as_bytes(), &w.sig1, w.m2.as_bytes(), &w.sig2) else {
            host.status("These signatures do not share a nonce.", Tone::Bad);
            return;
        };
        let correct = sol.d == even_key(&w.d);
        self.solution = Some((sol, correct));
        host.sfx("unlock");
        host.status("Private key recovered. Now sign an order with it.", Tone::Ok);
    }

    fn forge(&mut self, host: &mut Host) {
        if host.solved() {
            return;
        }
        let Some((sol, _)) = &self.solution else {
            host.status("Recover the key first.", Tone::Bad);
            host.sfx("error");
            return;
        };
        let w = self.world.warden();
        let msg = self.forge_text.clone();
        let mut aux = [0u8; 32];
        aux[31] = 1;
        let sig = sign(msg.as_bytes(), &sol.d, Some(aux));
        let ok = verify(&sig, msg.as_bytes(), &w.p);
        self.forged = Some((sig.hex(), ok));
        if !ok {
            host.status("Signature did not verify.", Tone::Bad);
            return;
        }
        // it must actually be a different message than the two originals
        if msg == w.m1 || msg == w.m2 {
            host.status("That is one of his old orders. Write a new one.", Tone::Warn);
            return;
        }
        host.sfx("unlock");
        host.solve("Signed, as Warden, in Warden's own hand. The Archive will open for it.");
    }

    fn order_card(ui: &mut egui::Ui, id: &str, title: &str, msg: &str, rx: &[u8; 32], s: &Big, e: &Big, ok: bool, highlight: bool, ei: &str) {
        card(ui, Some(title), None, |ui| {
            note(ui, &format!("\"{msg}\""));
            egui::Grid::new(id).num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                row(ui, "R.x", &shorten(&to_hex(rx), 10, 8), if highlight { AMBER } else { TEXT });
                row(ui, &format!("s{ei}"), &shorten(&hex32(s), 10, 8), TEXT);
                row(ui, &format!("e{ei} = H(R‖P‖m{ei})"), &shorten(&hex32(e), 10, 8), TEXT);
                ui.label(RichText::new("verifies").font(mono(12.5)).color(DIM));
                tag(ui, if ok { "YES" } else { "NO" }, if ok { Tone::Ok } else { Tone::Bad });
                ui.end_row();
            });
        });
    }
}

impl Puzzle for NonceReuse {
    fn id(&self) -> &'static str {
        "ch4.nonce"
    }

    fn title(&self) -> &str {
        "WARDEN'S SIGNING LOG"
    }

    fn subtitle(&self) -> Option<&str> {
        Some("BIP-340 · two orders, one nonce")
    }

    fn accent(&self) -> Color32 {
        RED
    }

    fn open(&mut self, host: &mut Host) {
        host.status("Two orders share a nonce. Recover the key.", Tone::Info);
    }

    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        let world = self.world.clone();
        let w = world.warden();
        lede(ui, "Warden signs every Directorate order with one key. His hardware signer has a broken random number generator: it produced the same nonce **R** for two different orders. That is fatal.");
        card(ui, Some("Warden's public key"), None, |ui| {
            hex_box(ui, &to_hex(&w.p), Some("x-only public key (verifies every order below)"), None, false);
        });
        let (e1, e2, same_r, ok1, ok2) = (&self.e1, &self.e2, self.same_r, self.ok1, self.ok2);
        columns2(
            ui,
            |ui| Self::order_card(ui, "nr-o1", "Order 0417", w.m1, &w.sig1.r, &w.sig1.s, e1, ok1, false, "₁"),
            |ui| Self::order_card(ui, "nr-o2", "Order 0418", w.m2, &w.sig2.r, &w.sig2.s, e2, ok2, same_r, "₂"),
        );
        let mut recover = false;
        card(ui, Some("The leak"), None, |ui| {
            note(ui, if same_r { "Both orders carry the **same R.x**. Same nonce k, two different challenges. Subtract the equations and k cancels:" } else { "These do not share a nonce." });
            math(ui, "s₁ = k + e₁·d,   s₂ = k + e₂·d   ⇒   d = (s₁ − s₂) / (e₁ − e₂)  mod n");
            if button(ui, "Recover Warden's key", Kind::Primary).clicked() {
                recover = true;
            }
            if let Some((sol, correct)) = &self.solution {
                ui.add_space(6.0);
                egui::Grid::new("nr-sol").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                    row(ui, "s₁ − s₂ mod n", &shorten(&hex32(&sol.s_diff), 10, 8), TEXT);
                    row(ui, "e₁ − e₂ mod n", &shorten(&hex32(&sol.e_diff), 10, 8), TEXT);
                    row(ui, "(e₁ − e₂)⁻¹ mod n", &shorten(&hex32(&sol.e_diff_inv), 10, 8), TEXT);
                    ui.label(RichText::new("recovered d").font(mono(12.5)).color(DIM));
                    ui.label(RichText::new(shorten(&hex32(&sol.d), 12, 10)).font(mono(15.0)).color(GREEN));
                    ui.end_row();
                    ui.label(RichText::new("d·G matches his key?").font(mono(12.5)).color(DIM));
                    tag(ui, if *correct { "YES" } else { "NO" }, if *correct { Tone::Ok } else { Tone::Bad });
                    ui.end_row();
                });
                note(ui, "This is not a theoretical attack. The PlayStation 3 signing key fell to a constant nonce in 2010, and Android wallets lost coins to repeated nonces in 2013.");
            }
        });
        if recover {
            self.recover(host);
        }
        let mut forge = false;
        card(ui, Some("Sign a new order"), None, |ui| {
            note(ui, "With Warden's private key you can sign anything the Directorate would, and every Directorate verifier will accept it. Write the order that unseals the Archive.");
            ui.add(egui::TextEdit::singleline(&mut self.forge_text).font(mono(13.5)).desired_width(f32::INFINITY));
            if button(ui, "Sign as Warden", Kind::Normal).clicked() {
                forge = true;
            }
            if let Some((sig, ok)) = &self.forged {
                hex_box(ui, sig, Some("your signature as Warden"), Some(if *ok { GREEN } else { RED }), false);
                note(ui, if *ok { "The Directorate's own verifiers accept this. The Archive door is listening for exactly this signature." } else { "Signature invalid." });
            }
        });
        if forge {
            self.forge(host);
        }
    }

    fn autoplay(&mut self, host: &mut Host) {
        self.recover(host);
        self.forge(host);
    }
}
