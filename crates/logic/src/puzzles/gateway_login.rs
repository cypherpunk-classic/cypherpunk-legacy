//! Prologue, part 2. The gateway knows Grandpa's public key. It sends a fresh
//! challenge; you sign it with the recovered private key (BIP-340 Schnorr);
//! the gateway checks s·G = R + e·P. No password ever crosses the wire.

use std::rc::Rc;

use cpl_crypto::bip340::{Sig, explain_verify, sign};
use cpl_crypto::bip39::BIP86_FIRST;
use cpl_crypto::bytes::{Big, to_hex};
use cpl_crypto::drbg::random_bytes;
use cpl_crypto::secp::{x_only, x_only_hex};
use egui::{Color32, RichText};

use crate::puzzle::{Host, Puzzle};
use crate::ui::theme::{AMBER, CYAN, GREEN, RED, mono};
use crate::ui::widgets::{Kind, Tone, button, button_enabled, card, columns2, hex_box, kv, lede, math, note, shorten, tag};
use crate::world::{World, hex32};

struct Verdict {
    e: String,
    lhs: String,
    rhs: String,
    ok: bool,
}

pub struct GatewayLogin {
    world: Rc<World>,
    challenge: String,
    sig: Option<Sig>,
    tamper: bool,
    verdict: Option<Verdict>,
}

impl GatewayLogin {
    pub fn new(world: Rc<World>) -> Self {
        let challenge = format!("CRYPTOPOLIS-LOGIN|user=nightjar|nonce={}", to_hex(&random_bytes::<16>()));
        Self { world, challenge, sig: None, tamper: false, verdict: None }
    }

    fn do_sign(&mut self, host: &mut Host) {
        let g = self.world.grandpa();
        self.sig = Some(sign(self.challenge.as_bytes(), &g.secret, Some(random_bytes::<32>())));
        host.sfx("confirm");
        host.status("Signed. Send it to the gateway.", Tone::Info);
    }

    fn send(&mut self, host: &mut Host) {
        let Some(sig) = &self.sig else { return };
        if host.solved() {
            return;
        }
        let g = self.world.grandpa();
        let sent = if self.tamper { Sig { r: sig.r, s: &sig.s ^ Big::from(1) } } else { sig.clone() };
        let ex = explain_verify(&sent, self.challenge.as_bytes(), &x_only(&g.p)).expect("valid key");
        self.verdict = Some(Verdict { e: hex32(&ex.e), lhs: x_only_hex(&ex.lhs), rhs: x_only_hex(&ex.rhs), ok: ex.ok });
        if !ex.ok {
            host.status("ACCESS DENIED: s·G ≠ R + e·P. One flipped bit and the equation falls apart.", Tone::Bad);
            host.sfx("error");
            return;
        }
        host.solve("ACCESS GRANTED. Welcome back, nightjar.");
    }
}

impl Puzzle for GatewayLogin {
    fn id(&self) -> &'static str {
        "pro.login"
    }

    fn title(&self) -> &str {
        "CRYPTOPOLIS GATEWAY"
    }

    fn subtitle(&self) -> Option<&str> {
        Some("challenge–response · BIP-340")
    }

    fn accent(&self) -> Color32 {
        CYAN
    }

    fn open(&mut self, host: &mut Host) {
        host.status("Sign the challenge with the recovered key.", Tone::Info);
    }

    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        let pubkey = x_only_hex(&self.world.grandpa().p);
        lede(ui, "The terminal does not want the seed, and it does not want a password. It wants proof that you hold the private key behind the public key it has on file for **nightjar**. It sends a random challenge; you sign it; it checks the signature.");
        columns2(
            ui,
            |ui| {
                card(ui, Some("Your side"), None, |ui| {
                    kv(ui, "login-you", &[("derivation path", BIP86_FIRST.into()), ("private key", "•••• never leaves this machine ••••".into()), ("public key (x-only)", pubkey.clone())]);
                });
            },
            |ui| {
                card(ui, Some("Gateway records"), None, |ui| {
                    kv(ui, "login-gw", &[("user", "nightjar".into()), ("registered key", pubkey.clone())]);
                    tag(ui, "SAME KEY", Tone::Ok);
                });
            },
        );
        let mut sign_clicked = false;
        card(ui, Some("Challenge"), None, |ui| {
            note(ui, "A fresh nonce every time, so an old signature can never be replayed.");
            hex_box(ui, &self.challenge, None, Some(AMBER), false);
            if button(ui, "Sign the challenge", Kind::Primary).clicked() {
                sign_clicked = true;
            }
            match &self.sig {
                None => note(ui, "Not signed yet."),
                Some(sig) => {
                    note(ui, "A BIP-340 signature is 64 bytes: R.x (the nonce point) followed by s = k + e·d.");
                    let shown = if self.tamper && self.verdict.as_ref().is_some_and(|v| !v.ok) { Sig { r: sig.r, s: &sig.s ^ Big::from(1) } } else { sig.clone() };
                    hex_box(ui, &shown.hex(), Some("signature  =  R.x ‖ s"), None, false);
                }
            }
        });
        if sign_clicked {
            self.do_sign(host);
        }
        let mut send_clicked = false;
        card(ui, Some("Send"), None, |ui| {
            ui.checkbox(&mut self.tamper, RichText::new("Flip one bit of the signature on the way (see what happens)").font(mono(13.0)));
            if button_enabled(ui, self.sig.is_some() && !host.solved(), "Send signature to gateway", Kind::Primary).clicked() {
                send_clicked = true;
            }
            if let Some(v) = &self.verdict {
                math(ui, "e = H_challenge(R.x ‖ P.x ‖ m)  mod n");
                ui.label(RichText::new(format!("e = {}", shorten(&v.e, 20, 12))).font(mono(12.5)));
                let c = if v.ok { GREEN } else { RED };
                note(ui, "left side:  s·G  (x coordinate)");
                ui.add(egui::Label::new(RichText::new(&v.lhs).font(mono(12.5)).color(c)).wrap());
                note(ui, "right side: R + e·P  (x coordinate)");
                ui.add(egui::Label::new(RichText::new(&v.rhs).font(mono(12.5)).color(c)).wrap());
            }
        });
        if send_clicked {
            self.send(host);
        }
    }

    fn autoplay(&mut self, host: &mut Host) {
        self.tamper = false;
        self.do_sign(host);
        self.send(host);
    }
}
