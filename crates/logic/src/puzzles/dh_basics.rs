//! Chapter 2, part 1. Diffie–Hellman with numbers small enough to watch:
//! p = 1019, g = 4 (order 509). Pick a secret, publish g^a, then build the
//! shared key yourself from what you hold (a and Felix's B) and land on the
//! same number he does. Then watch an eavesdropper break such a tiny group by
//! trying every exponent.

use std::rc::Rc;

use cpl_crypto::bytes::Big;
use cpl_crypto::dh::TOY_GROUP;
use egui::{Color32, RichText};

use crate::puzzle::{HandCell, HandSheet, Host, Puzzle, StepView, steps};
use crate::ui::theme::{self, DIM, GREEN, RED, mono};
use crate::ui::widgets::{Formula, Kind, Log, Part, Tone, button, button_enabled, card, columns2, formula, kv, lede, math, note};

const PARTS: &[Part] = &[Part::Text("K = "), Part::Slot, Part::Text(" ^ "), Part::Slot, Part::Text("  mod p")];
const PALETTE: &[&str] = &["A", "B", "a", "b", "g"];
use crate::world::World;

pub struct DhBasics {
    world: Rc<World>,
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
    /// A has gone to Felix (and B is known to be his answer)
    sent: bool,
    key: Formula,
    /// what the player's formula computed
    mine: Option<u64>,
}

impl DhBasics {
    pub fn new(world: Rc<World>) -> Self {
        let g = TOY_GROUP;
        let b = u64::try_from(world.rng("ch2/felix-toy").range(&Big::from(2), &Big::from(g.q - 1))).unwrap();
        Self { world, a_input: String::new(), a: None, big_a: None, b, big_b: g.public(b), k: None, kf: None, steps: Log::default(), eve: None, random_counter: 0, sent: false, key: Formula::new(PARTS), mine: None }
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
        self.sent = false;
        self.mine = None;
        host.sfx("confirm");
        host.status("Now send A to Felix. It is safe to say out loud.", Tone::Info);
    }

    fn send(&mut self, host: &mut Host) {
        let (Some(_), Some(big_a)) = (self.a, self.big_a) else {
            host.status("Compute A first.", Tone::Bad);
            return;
        };
        if host.solved() {
            return;
        }
        self.kf = Some(TOY_GROUP.shared(self.b, big_a));
        self.sent = true;
        host.sfx("blip");
        host.status("Felix has A, and he has computed his key from it. You have his B. Build yours.", Tone::Info);
    }

    /// Evaluate K = base ^ exponent with the real numbers, or say why not.
    fn combine(&mut self, host: &mut Host) {
        if host.solved() {
            return;
        }
        let (Some(a), Some(big_a), Some(kf)) = (self.a, self.big_a, self.kf) else {
            host.status("Send A to Felix first.", Tone::Warn);
            return;
        };
        let g = TOY_GROUP;
        let toks = self.key.tokens();
        let (Some(base), Some(exp)) = (toks[0], toks[1]) else { return };
        if base == 3 || exp == 3 {
            host.status("b is Felix's secret. It never left his machine, and it never will.", Tone::Bad);
            host.sfx("error");
            return;
        }
        if base == 2 {
            host.status("a is a secret exponent, not a group element. The base must be one of the public values.", Tone::Bad);
            host.sfx("error");
            return;
        }
        if exp != 2 {
            host.status(format!("{} is a group element, not an exponent. The only secret number you hold is a.", PALETTE[exp]), Tone::Bad);
            host.sfx("error");
            return;
        }
        let base_v = match base {
            0 => big_a,
            1 => self.big_b,
            _ => g.g,
        };
        let k = g.pow(base_v, a);
        self.mine = Some(k);
        if k == kf {
            self.k = Some(k);
            host.sfx("unlock");
            host.solve(format!("Same key on both sides: K = {k}. B^a = (g^b)^a = g^(ab) = (g^a)^b = A^b, and nobody ever sent it."));
            return;
        }
        let why = match base {
            0 => "A^a = g^(a·a): a number only you can compute, but Felix cannot. Use what he sent you.",
            _ => "g^a is just A, your public value. Eve heard it on the line, so it is no secret.",
        };
        host.status(format!("You get {k}, Felix gets something else. {why}"), Tone::Bad);
        host.sfx("error");
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
                        if button_enabled(ui, self.big_a.is_some() && !self.sent, "Send A to Felix", Kind::Normal).clicked() {
                            send = true;
                        }
                    });
                    if !self.steps.lines.is_empty() {
                        self.steps.show(ui, "dh-steps", 150.0);
                    }
                    if let Some(a) = self.big_a {
                        kv(ui, "dh-mine", &[("public A", a.to_string())]);
                    }
                    if let Some(k) = self.mine {
                        kv(ui, "dh-mine-k", &[("your key", k.to_string())]);
                    }
                });
            },
            |ui| {
                card(ui, Some("Felix"), None, |ui| {
                    kv(ui, "dh-felix", &[("secret b", "(only Felix knows)".into()), ("public B", big_b.to_string())]);
                    if let Some(kf) = self.kf {
                        let shown = if self.k.is_some() { kf.to_string() } else { "computed, kept to himself".into() };
                        kv(ui, "dh-felix-k", &[("Felix computes", "A^b mod p".into()), ("his key", shown)]);
                        if let Some(m) = self.mine {
                            ui.label(RichText::new(if m == kf { "keys match" } else { "keys differ" }).font(mono(13.0)).color(if m == kf { GREEN } else { RED }));
                        }
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
        let mut combine = false;
        card(ui, Some("Your key"), None, |ui| {
            if !self.sent {
                note(ui, "Send A first. Then you hold your secret a and Felix's public B; he holds b and your A.");
                return;
            }
            note(ui, "Build your key from what you hold. It has to come out equal to Felix's, which he built from b and A.");
            formula(ui, PARTS, PALETTE, &mut self.key);
            if button_enabled(ui, self.key.complete() && !host.solved(), "Compute and compare with Felix", Kind::Primary).clicked() {
                combine = true;
            }
        });
        if combine {
            self.combine(host);
        }
        let mut brute = false;
        card(ui, Some("Eve, listening on the line"), None, |ui| {
            note(ui, "Eve sees p, g, A and B. To get K she needs a or b: a {discrete logarithm}.");
            if self.big_a.is_some() {
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
        self.key.set(&[1, 2]);
        self.combine(host);
    }

    fn steps(&self) -> Vec<StepView> {
        steps(&[
            ("Choose a secret a and compute A = g^a mod p", "Any number from 2 to 508. Square-and-multiply does the rest.", self.big_a.is_some()),
            ("Send A to Felix", "Public values are safe to say out loud. His B is already on the board.", self.sent),
            ("Build the same key Felix has", "You hold a and B; he holds b and A. Both must land on g^(ab).", self.k.is_some()),
            ("Let Eve try every exponent (optional)", "", self.eve.is_some()),
        ])
    }

    fn deep_sheet(&self) -> Option<HandSheet> {
        let g = TOY_GROUP;
        let mut rng = self.world.rng("ch2/dh-deep");
        let a = 11 + rng.int(20) as u64;
        let b = 11 + rng.int(20) as u64;
        let (ya, yb) = (g.public(a), g.public(b));
        let k = g.shared(a, yb);
        Some(HandSheet::new(
            "Diffie–Hellman with pencil and paper",
            &format!("p = {}, g = {}. Your secret is a = {a}; Felix's public value is B = {yb} (his secret, which you would never see, is b = {b}). Square-and-multiply: write the exponent in binary, square at every bit, multiply by the base at every 1.", g.p, g.g),
            240.0,
            vec![
                HandCell::new(format!("A = {}^{a} mod {}", g.g, g.p), format!("{a} = 0b{a:b}"), ya as i64),
                HandCell::new(format!("K = B^a = {yb}^{a} mod {}", g.p), "same method, bigger base", k as i64),
                HandCell::new(format!("Felix's check: A^b = A^{b} mod {}", g.p), "it must equal your K", k as i64),
                HandCell::new("exponents Eve must try, at most", "the group order, minus the trivial one", (g.q - 1) as i64),
            ],
        ))
    }
}
