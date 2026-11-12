//! Chapter 4, part 1. Warden's signing device reused a nonce across two of
//! his orders. The player finds the two signatures that share R among three,
//! then cancels k by hand: s1 = k + e1·d, s2 = k + e2·d, so
//! d = (s1 − s2)/(e1 − e2). The terminal only evaluates the formula and checks
//! d·G against his public key. Then: sign a new order of your own.

use std::rc::Rc;

use cpl_crypto::bip340::{Sig, challenge, sign, sign_with_nonce, verify};
use cpl_crypto::bytes::{Big, to_hex};
use cpl_crypto::dh::TOY_GROUP;
use cpl_crypto::secp::{N, inv_n, mod_n, mul, x_only};
use egui::{Color32, RichText};

use crate::puzzle::{HandCell, HandSheet, Host, Puzzle, StepView, steps};
use crate::ui::theme::{AMBER, DIM, GREEN, RED, TEXT, mono};
use crate::ui::widgets::{Formula, Kind, Part, Tone, button, button_enabled, card, formula, hex_box, lede, math, note, select_card, shorten, tag};
use crate::world::{World, hex32};

const DEFAULT_ORDER: &str = "DLA ORDER 0419: UNSEAL THE ARCHIVE. RELEASE TO PUBLIC. AUTH: WARDEN";
const M0: &str = "DLA ORDER 0416: EXTEND CURFEW IN THE COMMONS BY ONE HOUR. AUTH: WARDEN";
const PARTS: &[Part] = &[Part::Text("d = ("), Part::Slot, Part::Text(" − "), Part::Slot, Part::Text(") / ("), Part::Slot, Part::Text(" − "), Part::Slot, Part::Text(")  mod n")];
const PALETTE: &[&str] = &["s₁", "s₂", "e₁", "e₂", "R"];

struct Order {
    name: &'static str,
    msg: String,
    sig: Sig,
    e: Big,
    ok: bool,
}

pub struct NonceReuse {
    world: Rc<World>,
    orders: Vec<Order>,
    /// the two orders picked, in the order they were picked (they become ₁ and ₂)
    picked: Vec<usize>,
    solve_for: Formula,
    /// the d the player's formula produced, and whether d·G is Warden's key
    recovered: Option<(Big, bool)>,
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
        // an older order, signed properly with its own nonce
        let k0 = world.rng("ch4/order-0416").scalar(&N);
        let sig0 = sign_with_nonce(M0.as_bytes(), &w.d, &k0);
        let order = |name, msg: &str, sig: &Sig| Order { name, msg: msg.to_string(), e: challenge(&sig.r, &w.p, msg.as_bytes()), ok: verify(sig, msg.as_bytes(), &w.p), sig: sig.clone() };
        let orders = vec![order("Order 0416", M0, &sig0), order("Order 0417", w.m1, &w.sig1), order("Order 0418", w.m2, &w.sig2)];
        Self { world, orders, picked: Vec::new(), solve_for: Formula::new(PARTS), recovered: None, forge_text: DEFAULT_ORDER.into(), forged: None }
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
        self.recovered = None;
    }

    fn same_r(&self) -> bool {
        self.picked.len() == 2 && self.orders[self.picked[0]].sig.r == self.orders[self.picked[1]].sig.r
    }

    fn solve(&mut self, host: &mut Host) {
        if self.picked.len() != 2 {
            host.status("Pick two orders first: they become order ₁ and order ₂.", Tone::Warn);
            return;
        }
        let toks = self.solve_for.tokens();
        if toks.iter().any(Option::is_none) {
            return;
        }
        let toks: Vec<usize> = toks.into_iter().flatten().collect();
        if toks.contains(&4) {
            host.status("R is a point on the curve, not a number. d comes out of the numbers s and e.", Tone::Bad);
            host.sfx("error");
            return;
        }
        let (o1, o2) = (&self.orders[self.picked[0]], &self.orders[self.picked[1]]);
        let val = |t: usize| match t {
            0 => o1.sig.s.clone(),
            1 => o2.sig.s.clone(),
            2 => o1.e.clone(),
            _ => o2.e.clone(),
        };
        let num = mod_n(&(val(toks[0]) + &*N - val(toks[1])));
        let den = mod_n(&(val(toks[2]) + &*N - val(toks[3])));
        if den == Big::from(0u8) {
            host.status("That denominator is zero mod n: you cannot divide by it.", Tone::Bad);
            host.sfx("error");
            return;
        }
        let d = mod_n(&(num * inv_n(&den)));
        let w = self.world.warden();
        let ok = x_only(&mul(&d)) == w.p;
        self.recovered = Some((d, ok));
        if ok {
            host.sfx("unlock");
            host.status("d·G is Warden's public key. The private key is yours. Now sign an order with it.", Tone::Ok);
            return;
        }
        host.sfx("error");
        let why = if !self.same_r() {
            "d·G is not Warden's key. These two orders carry different R, so their nonces differ and k does not cancel. Look for the pair with the same R.x."
        } else {
            "d·G is not Warden's key. Write both equations out: s₁ = k + e₁·d and s₂ = k + e₂·d. Subtract them; what is left on each side?"
        };
        host.status(why, Tone::Bad);
    }

    fn forge(&mut self, host: &mut Host) {
        if host.solved() {
            return;
        }
        let Some((d, true)) = self.recovered.clone() else {
            host.status("Recover the key first.", Tone::Bad);
            host.sfx("error");
            return;
        };
        let w = self.world.warden();
        let msg = self.forge_text.clone();
        let mut aux = [0u8; 32];
        aux[31] = 1;
        let sig = sign(msg.as_bytes(), &d, Some(aux));
        let ok = verify(&sig, msg.as_bytes(), &w.p);
        self.forged = Some((sig.hex(), ok));
        if !ok {
            host.status("Signature did not verify.", Tone::Bad);
            return;
        }
        // it must actually be a different message than his old orders
        if self.orders.iter().any(|o| o.msg == msg) {
            host.status("That is one of his old orders. Write a new one.", Tone::Warn);
            return;
        }
        host.sfx("unlock");
        host.solve("Signed, as Warden, in Warden's own hand. The Archive will open for it.");
    }

    fn order_card(&self, ui: &mut egui::Ui, i: usize) -> bool {
        let o = &self.orders[i];
        let slot = self.picked.iter().position(|p| *p == i).map(|k| if k == 0 { "₁" } else { "₂" });
        let shared = self.orders.iter().enumerate().any(|(j, p)| j != i && p.sig.r == o.sig.r);
        select_card(ui, slot.is_some(), AMBER, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(o.name).font(mono(14.0)).color(TEXT).strong());
                if let Some(k) = slot {
                    ui.label(RichText::new(format!("order {k}")).font(mono(12.5)).color(AMBER));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    tag(ui, if o.ok { "VERIFIES" } else { "BAD" }, if o.ok { Tone::Ok } else { Tone::Bad });
                });
            });
            note(ui, &format!("\"{}\"", o.msg));
            egui::Grid::new(("nr-o", i)).num_columns(2).spacing([12.0, 2.0]).show(ui, |ui| {
                row(ui, "R.x", &shorten(&to_hex(&o.sig.r), 10, 8), if shared && slot.is_some() && self.same_r() { AMBER } else { TEXT });
                row(ui, "s", &shorten(&hex32(&o.sig.s), 10, 8), TEXT);
                row(ui, "e = H(R‖P‖m)", &shorten(&hex32(&o.e), 10, 8), TEXT);
            });
        })
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
        Some("BIP-340 · three orders, one reused nonce")
    }

    fn accent(&self) -> Color32 {
        RED
    }

    fn open(&mut self, host: &mut Host) {
        host.status("Two of these orders share a nonce. Find them, then solve for the key.", Tone::Info);
    }

    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        let world = self.world.clone();
        let w = world.warden();
        lede(ui, "Warden signs every Directorate order with one key. His hardware signer has a broken random number generator: twice it produced the same nonce, so two different orders carry the same **R**. That is fatal.");
        card(ui, Some("Warden's public key"), None, |ui| {
            hex_box(ui, &to_hex(&w.p), Some("x-only public key P (verifies every order below)"), None, false);
        });
        let mut picked = None;
        card(ui, Some("The signing log · pick two"), None, |ui| {
            for i in 0..self.orders.len() {
                if self.order_card(ui, i) {
                    picked = Some(i);
                }
            }
        });
        if let Some(i) = picked {
            self.pick(i);
        }
        let mut solve = false;
        card(ui, Some("The leak"), None, |ui| {
            math(ui, "s = k + e·d  mod n     (every BIP-340 signature)");
            note(ui, "Write the equation for order ₁ and for order ₂. If both used the same k, one subtraction makes it vanish. Solve what is left for d.");
            if formula(ui, PARTS, PALETTE, &mut self.solve_for) {
                self.recovered = None;
            }
            if button_enabled(ui, self.solve_for.complete() && self.picked.len() == 2, "Solve for d and check d·G", Kind::Primary).clicked() {
                solve = true;
            }
            if let Some((d, ok)) = &self.recovered {
                egui::Grid::new("nr-sol").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                    ui.label(RichText::new("your d").font(mono(12.5)).color(DIM));
                    ui.label(RichText::new(shorten(&hex32(d), 12, 10)).font(mono(15.0)).color(if *ok { GREEN } else { RED }));
                    ui.end_row();
                    ui.label(RichText::new("d·G is his key?").font(mono(12.5)).color(DIM));
                    tag(ui, if *ok { "YES" } else { "NO" }, if *ok { Tone::Ok } else { Tone::Bad });
                    ui.end_row();
                });
                if *ok {
                    note(ui, "This is not a theoretical attack. The PlayStation 3 signing key fell to a constant nonce in 2010, and Android wallets lost coins to repeated nonces in 2013.");
                }
            }
        });
        if solve {
            self.solve(host);
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
        // the two orders whose R matches
        self.picked.clear();
        'outer: for i in 0..self.orders.len() {
            for j in i + 1..self.orders.len() {
                if self.orders[i].sig.r == self.orders[j].sig.r {
                    self.picked = vec![i, j];
                    break 'outer;
                }
            }
        }
        self.solve_for.set(&[0, 1, 2, 3]);
        self.solve(host);
        self.forge(host);
    }

    fn steps(&self) -> Vec<StepView> {
        let found = self.recovered.as_ref().is_some_and(|(_, ok)| *ok);
        steps(&[
            ("Find the two orders that share a nonce", "Compare R.x across the three orders.", self.same_r() || found),
            ("Cancel k and solve for d", "Subtract s₂ = k + e₂·d from s₁ = k + e₁·d, then divide.", found),
            ("Sign a new order as Warden", "", self.forged.as_ref().is_some_and(|(_, ok)| *ok)),
        ])
    }

    fn deep_sheet(&self) -> Option<HandSheet> {
        let q = TOY_GROUP.q as i64;
        let mut rng = self.world.rng("ch4/nonce-deep");
        let d = 2 + rng.int(500) as i64;
        let k = 2 + rng.int(500) as i64;
        let e1 = 2 + rng.int(500) as i64;
        let mut e2 = 2 + rng.int(500) as i64;
        if e2 == e1 {
            e2 = (e1 + 7) % q;
        }
        let s1 = (k + e1 * d) % q;
        let s2 = (k + e2 * d) % q;
        let ds = (s1 - s2).rem_euclid(q);
        let de = (e1 - e2).rem_euclid(q);
        let inv = cpl_crypto::bytes::inv_mod_i64(de, q).unwrap_or(1);
        let got = ds * inv % q;
        debug_assert_eq!(got, d);
        Some(HandSheet::new(
            "the PS3 attack, mod 509",
            format!("Exponents mod q = {q}. Two signatures with one nonce: (e₁ = {e1}, s₁ = {s1}) and (e₂ = {e2}, s₂ = {s2}). Recover the key, then the nonce."),
            240.0,
            vec![
                HandCell::new(format!("s₁ − s₂ mod {q}"), "", ds),
                HandCell::new(format!("e₁ − e₂ mod {q}"), "", de),
                HandCell::new(format!("(e₁ − e₂)⁻¹ mod {q}"), "extended Euclid", inv),
                HandCell::new(format!("d mod {q}"), "multiply the two", got),
                HandCell::new(format!("k = s₁ − e₁·d mod {q}"), "the nonce falls out too", k),
            ],
        ))
    }
}
