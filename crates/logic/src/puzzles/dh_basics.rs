//! Chapter 2, part 1. Diffie–Hellman with numbers small enough to watch:
//! p = 1019, g = 4 (order 509). Pick a secret, publish g^a, combine with
//! Felix's public value, and end up with the same key he has. Then watch an
//! eavesdropper break such a tiny group by trying every exponent.

use std::rc::Rc;

use cpl_crypto::bytes::Big;
use cpl_crypto::dh::TOY_GROUP;
use egui::{Color32, RichText};

use crate::puzzle::{Host, Puzzle};
use crate::ui::theme::{self, DIM, GREEN, RED, mono};
use crate::ui::widgets::{Kind, Log, Tone, button, card, columns2, kv, lede, math, note};
use crate::world::World;

pub struct DhBasics {
    a_input: String,
    a: Option<u64>,
    big_a: Option<u64>,
    b: u64,
    big_b: u64,
    k: Option<u64>,
    kf: Option<u64>,
    steps: Log,
    eve: Option<String>,
    random_counter: u64,
}

impl DhBasics {
    pub fn new(world: Rc<World>) -> Self {
        let g = TOY_GROUP;
        let b = u64::try_from(world.rng("ch2/felix-toy").range(&Big::from(2), &Big::from(g.q - 1))).unwrap();
        Self { a_input: String::new(), a: None, big_a: None, b, big_b: g.public(b), k: None, kf: None, steps: Log::default(), eve: None, random_counter: 0 }
    }

    fn compute_a(&mut self, host: &mut Host) {
        let g = TOY_GROUP;
        let a = match self.a_input.trim().parse::<u64>() {
            Ok(a) if (2..g.q).contains(&a) => a,
            _ => {
                host.status("Pick a between 2 and 508.", Tone::Bad);
                return;
            }
        };
        self.steps.clear();
        let bits = format!("{a:b}");
        self.steps.line(format!("a = {a} = 0b{bits}  (square-and-multiply, mod {})", g.p), Tone::Dim);
        let mut acc: u64 = 1;
        for bit in bits.chars() {
            acc = acc * acc % g.p;
            if bit == '1' {
                acc = acc * g.g % g.p;
                self.steps.line(format!("square, multiply by g → {acc}"), Tone::None);
            } else {
                self.steps.line(format!("square → {acc}"), Tone::None);
            }
        }
        self.steps.line(format!("A = {acc}"), Tone::Ok);
        self.a = Some(a);
        self.big_a = Some(acc);
        self.k = None;
        self.kf = None;
        host.sfx("confirm");
        host.status("Now send A to Felix. It is safe to say out loud.", Tone::Info);
    }

    fn send(&mut self, host: &mut Host) {
        let (Some(a), Some(big_a)) = (self.a, self.big_a) else {
            host.status("Compute A first.", Tone::Bad);
            return;
        };
        if host.solved() {
            return;
        }
        let g = TOY_GROUP;
        let k = g.shared(a, self.big_b);
        let kf = g.shared(self.b, big_a);
        self.k = Some(k);
        self.kf = Some(kf);
        if k != kf {
            host.status("Keys differ? That should be impossible.", Tone::Bad);
            return;
        }
        host.sfx("unlock");
        host.solve(format!("Same key on both sides: K = {k}. Nobody ever sent it."));
    }

    fn brute(&mut self) {
        let g = TOY_GROUP;
        let Some(target) = self.big_a else { return };
        let t0 = web_time::Instant::now();
        let found = (1..g.q).find(|&x| g.public(x) == target).unwrap_or(0);
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        self.eve = Some(format!(
            "Eve found a = {found} in {ms:.2} ms by brute force, and with it K. With a 2048-bit prime, or a 256-bit elliptic curve, the best known classical attacks need about 2^112 to 2^128 steps instead."
        ));
    }
}

impl Puzzle for DhBasics {
    fn id(&self) -> &'static str {
        "ch2.dh"
    }

    fn title(&self) -> &str {
        "SWITCHBOARD · KEY EXCHANGE"
    }

    fn subtitle(&self) -> Option<&str> {
        Some("p = 1019, g = 4")
    }

    fn accent(&self) -> Color32 {
        theme::rgb(0xef7d57)
    }

    fn open(&mut self, host: &mut Host) {
        host.status("Choose a secret exponent a between 2 and 508.", Tone::Info);
    }

    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        lede(ui, "In 1976 Diffie and Hellman showed two strangers how to agree on a secret while everyone listens. Everything below is public except the two secret exponents. Here the numbers are tiny so you can see them.");
        math(ui, "A = g^a mod p,   B = g^b mod p,   K = B^a = A^b = g^(ab)");
        let mut compute = false;
        let mut send = false;
        let big_b = self.big_b;
        columns2(
            ui,
            |ui| {
                card(ui, Some("You"), None, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("secret a =").font(mono(13.0)).color(DIM));
                        ui.add(egui::TextEdit::singleline(&mut self.a_input).hint_text("2 … 508").desired_width(110.0).font(mono(14.0)));
                        if button(ui, "Random", Kind::Ghost).clicked() {
                            self.random_counter += 1;
                            let r = (crate::state::now_ms() as u64).wrapping_mul(2654435761).wrapping_add(self.random_counter * 7919);
                            self.a_input = (2 + r % 500).to_string();
                        }
                    });
                    ui.horizontal_wrapped(|ui| {
                        if button(ui, "Compute A = g^a mod p", Kind::Primary).clicked() {
                            compute = true;
                        }
                        if button(ui, "Send A to Felix", Kind::Normal).clicked() {
                            send = true;
                        }
                    });
                    if !self.steps.lines.is_empty() {
                        self.steps.show(ui, "dh-steps", 150.0);
                    }
                    if let Some(a) = self.big_a {
                        kv(ui, "dh-mine", &[("public A", a.to_string())]);
                    }
                    if let Some(k) = self.k {
                        kv(ui, "dh-mine-k", &[("you compute", "B^a mod p".into()), ("your key", k.to_string())]);
                    }
                });
            },
            |ui| {
                card(ui, Some("Felix"), None, |ui| {
                    kv(ui, "dh-felix", &[("secret b", "(only Felix knows)".into()), ("public B", big_b.to_string())]);
                    if let Some(kf) = self.kf {
                        kv(ui, "dh-felix-k", &[("Felix computes", "A^b mod p".into()), ("his key", kf.to_string())]);
                        ui.label(RichText::new(if Some(kf) == self.k { "keys match" } else { "keys differ" }).font(mono(13.0)).color(if Some(kf) == self.k { GREEN } else { RED }));
                    }
                });
            },
        );
        if compute {
            self.compute_a(host);
        }
        if send {
            self.send(host);
        }
        let mut brute = false;
        card(ui, Some("Eve, listening on the line"), None, |ui| {
            note(ui, "Eve sees p, g, A and B. To get K she needs a or b: a {discrete logarithm}.");
            if self.k.is_some() {
                ui.horizontal(|ui| {
                    if button(ui, "Let Eve try every exponent", Kind::Normal).clicked() {
                        brute = true;
                    }
                    note(ui, &format!("only {} possibilities in this toy group", TOY_GROUP.q - 1));
                });
            }
            if let Some(e) = &self.eve {
                note(ui, e);
            }
        });
        if brute {
            self.brute();
        }
    }

    fn autoplay(&mut self, host: &mut Host) {
        self.a_input = "277".into();
        self.compute_a(host);
        self.send(host);
    }
}
