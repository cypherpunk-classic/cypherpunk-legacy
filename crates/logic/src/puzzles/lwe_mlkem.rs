//! Chapter 6, part 3. The other direction: building security on a lattice
//! problem instead of breaking one. Learning With Errors adds small noise to
//! linear equations; without noise, Gaussian elimination solves them
//! instantly, with noise, it is believed hard even for quantum computers. Then
//! the real thing: an ML-KEM-768 (FIPS 203) encapsulation, the standard the
//! world is actually migrating to.

use std::rc::Rc;

use cpl_crypto::bytes::to_hex;
use cpl_crypto::lwe::{LweParams, TOY_LWE, ciphertext_noise, gaussian_solve, lwe_decrypt, lwe_encrypt, lwe_keygen, lwe_phase};
use cpl_crypto::pq::{KEM_SIZES, KemKeys, kem_encapsulate};
use egui::{Align2, Color32, Pos2, Rect, RichText, Sense, Stroke};

use crate::puzzle::{Host, Puzzle};
use crate::ui::theme::{self, DIM, FAINT, GREEN, MAGENTA, RED, TEXT, mono};
use crate::ui::widgets::{Kind, Log, Tone, button, card, columns2, lede, math, note, shorten, tag};
use crate::world::World;

const BITS: [u8; 8] = [1, 0, 1, 1, 0, 0, 1, 0];

struct LweView {
    q: i64,
    secret: Vec<i64>,
    decrypted: Vec<u8>,
    max_noise: i64,
    gauss_recovers: bool,
    phases: Vec<i64>,
}

struct KemView {
    pk_len: usize,
    pk_hex: String,
    ct_len: usize,
    shared: String,
    recovered: String,
    agree: bool,
}

pub struct LweMlkem {
    world: Rc<World>,
    noise: i64,
    view: LweView,
    kem: Option<KemView>,
    log: Log,
}

impl LweMlkem {
    pub fn new(world: Rc<World>) -> Self {
        let view = Self::compute(&world, 1);
        Self { world, noise: 1, view, kem: None, log: Log::default() }
    }

    fn compute(world: &World, noise: i64) -> LweView {
        let params = LweParams { noise, ..TOY_LWE };
        let keys = lwe_keygen(params, &mut world.rng(&format!("ch6/lwe-{noise}")));
        let mut rng = world.rng(&format!("ch6/lwe-enc-{noise}"));
        let cts: Vec<_> = BITS.iter().map(|&b| lwe_encrypt(&keys, b, &mut rng)).collect();
        let decrypted = cts.iter().map(|c| lwe_decrypt(&keys, c)).collect();
        let max_noise = cts.iter().map(|c| ciphertext_noise(&keys, c).abs()).max().unwrap_or(0);
        let gauss_recovers = gaussian_solve(&keys.a, &keys.b, params.q).is_some_and(|g| g == keys.s);
        let phases = cts.iter().map(|c| lwe_phase(&keys, c)).collect();
        LweView { q: params.q, secret: keys.s.clone(), decrypted, max_noise, gauss_recovers, phases }
    }

    fn set_noise(&mut self, noise: i64) {
        self.noise = noise;
        self.view = Self::compute(&self.world, noise);
    }

    fn run_kem(&mut self, host: &mut Host) {
        let kp = KemKeys::generate();
        let pk = kp.public_key();
        let Ok((ct, ss)) = kem_encapsulate(&pk) else {
            host.status("Encapsulation failed.", Tone::Bad);
            return;
        };
        let Ok(rec) = kp.decapsulate(&ct) else {
            host.status("Decapsulation failed.", Tone::Bad);
            return;
        };
        let agree = rec == ss;
        self.kem = Some(KemView { pk_len: pk.len(), pk_hex: to_hex(&pk), ct_len: ct.len(), shared: to_hex(&ss), recovered: to_hex(&rec), agree });
        self.log.line("ML-KEM-768 encapsulation: both sides share the same 32-byte secret.", Tone::Ok);
        host.sfx("unlock");
        if agree {
            host.solve("A real post-quantum key agreement, computed on your machine. This is the ground the next generation stands on.");
        }
    }

    fn phase_plot(&self, ui: &mut egui::Ui) {
        let size = egui::vec2(ui.available_width().min(460.0), 150.0);
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let p = ui.painter_at(rect);
        p.rect_filled(rect, 3.0, theme::rgb(0x070b16));
        let q = self.view.q as f32;
        let span = rect.width() - 40.0;
        let cx = |v: f32| rect.left() + 20.0 + (v / q) * span;
        let (top, bottom) = (rect.top() + 10.0, rect.bottom() - 20.0);
        let quarter = span / 4.0;
        let ok_fill = Color32::from_rgba_unmultiplied(93, 255, 154, 20);
        let one_fill = Color32::from_rgba_unmultiplied(255, 62, 154, 20);
        p.rect_filled(Rect::from_min_max(Pos2::new(cx(0.0) - 10.0, top), Pos2::new(cx(0.0) + quarter, bottom)), 0.0, ok_fill);
        p.rect_filled(Rect::from_min_max(Pos2::new(cx(q) - quarter, top), Pos2::new(cx(q) + 10.0, bottom)), 0.0, ok_fill);
        p.rect_filled(Rect::from_min_max(Pos2::new(cx(q / 2.0) - quarter, top), Pos2::new(cx(q / 2.0) + quarter, bottom)), 0.0, one_fill);
        for v in [0.0, q / 2.0, q] {
            p.line_segment([Pos2::new(cx(v), top), Pos2::new(cx(v), bottom)], Stroke::new(1.0, theme::rgb(0x17203a)));
        }
        p.text(Pos2::new(cx(0.0), rect.bottom() - 6.0), Align2::LEFT_BOTTOM, "0 → bit 0", mono(10.0), FAINT);
        p.text(Pos2::new(cx(q / 2.0) - 20.0, rect.bottom() - 6.0), Align2::LEFT_BOTTOM, "q/2 → bit 1", mono(10.0), FAINT);
        for (i, ph) in self.view.phases.iter().enumerate() {
            let color = if BITS[i] == 1 { MAGENTA } else { GREEN };
            p.circle_filled(Pos2::new(cx(*ph as f32), rect.center().y - 6.0 + (i as f32 - 3.5) * 3.0), 4.0, color);
        }
    }
}

fn row(ui: &mut egui::Ui, k: &str, v: &str, color: Color32) {
    ui.label(RichText::new(k).font(mono(12.5)).color(DIM));
    ui.add(egui::Label::new(RichText::new(v).font(mono(12.5)).color(color)).wrap());
    ui.end_row();
}

impl Puzzle for LweMlkem {
    fn id(&self) -> &'static str {
        "ch6.pq"
    }

    fn title(&self) -> &str {
        "AFTER THE QUANTUM"
    }

    fn subtitle(&self) -> Option<&str> {
        Some("LWE → ML-KEM-768 (FIPS 203)")
    }

    fn accent(&self) -> Color32 {
        theme::rgb(0x73b7ff)
    }

    fn open(&mut self, host: &mut Host) {
        host.status("Push the noise up and down. See where decryption breaks.", Tone::Info);
    }

    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        lede(ui, "Everything so far, Diffie–Hellman, Schnorr, your own keys, breaks the day a large quantum computer runs Shor's algorithm. Traffic recorded today can be decrypted then: harvest now, decrypt later. So the ground is shifting to problems quantum computers do not help with. Lattices.");
        math(ui, "LWE: given A, b = A·s + e (small e), find s.   No noise → Gaussian elimination. Small noise → believed hard.");
        let mut noise = self.noise;
        card(ui, Some("Learning With Errors (toy: n=4, q=97)"), None, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("error size:").font(mono(12.5)).color(DIM));
                ui.add(egui::Slider::new(&mut noise, 0..=6).show_value(false));
                ui.label(RichText::new(if noise == 0 { "none (insecure)".to_string() } else { format!("±{noise}") }).font(mono(12.5)).color(TEXT));
            });
            let v = &self.view;
            let all_ok = v.decrypted.as_slice() == BITS;
            columns2(
                ui,
                |ui| {
                    egui::Grid::new("lwe-kv").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                        row(ui, "secret s", &format!("{:?}", v.secret), TEXT);
                        row(ui, "sent bits", &BITS.iter().map(|b| b.to_string()).collect::<String>(), TEXT);
                        row(ui, "decrypted", &v.decrypted.iter().map(|b| b.to_string()).collect::<String>(), if all_ok { GREEN } else { RED });
                        row(ui, "ciphertext noise", &format!("up to ±{}  (fails past q/4 ≈ 24)", v.max_noise), TEXT);
                        ui.label(RichText::new("Gaussian elimination").font(mono(12.5)).color(DIM));
                        if v.gauss_recovers {
                            tag(ui, "RECOVERS s", Tone::Bad);
                        } else {
                            tag(ui, "returns garbage", Tone::Ok);
                        }
                        ui.end_row();
                    });
                    note(ui, if self.noise == 0 { "With no noise, the equations are exact and s falls out instantly. That is not encryption; that is linear algebra." } else { "The same small noise that is a nuisance for decryption is what makes recovering s hard." });
                },
                |ui| {
                    note(ui, "phase of each ciphertext bit (distance from 0 or q/2 must stay clear):");
                    self.phase_plot(ui);
                },
            );
        });
        if noise != self.noise {
            self.set_noise(noise);
        }
        let mut run = false;
        card(ui, Some("The standard: ML-KEM-768"), None, |ui| {
            note(ui, "Built on module-LWE, standardised as FIPS 203 in August 2024 (from CRYSTALS-Kyber). Browsers and Signal already use hybrid key exchange with it. Run a real encapsulation.");
            if button(ui, "Generate keypair + encapsulate", Kind::Primary).clicked() {
                run = true;
            }
            if let Some(k) = &self.kem {
                egui::Grid::new("kem-kv").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                    row(ui, "public key", &format!("{} bytes  ({})", k.pk_len, shorten(&k.pk_hex, 8, 6)), TEXT);
                    row(ui, "ciphertext", &format!("{} bytes", k.ct_len), TEXT);
                    row(ui, "shared secret", &k.shared, GREEN);
                    row(ui, "decapsulated", &shorten(&k.recovered, 10, 8), if k.agree { GREEN } else { RED });
                    ui.label(RichText::new("agree").font(mono(12.5)).color(DIM));
                    tag(ui, if k.agree { "YES" } else { "NO" }, if k.agree { Tone::Ok } else { Tone::Bad });
                    ui.end_row();
                });
                note(ui, &format!("ML-KEM-768 sizes: pk {} B, ct {} B, secret {} B. Bigger than a 32-byte curve key, but it survives Shor.", KEM_SIZES.public_key, KEM_SIZES.cipher_text, KEM_SIZES.shared_secret));
            }
        });
        if run {
            self.run_kem(host);
        }
        if !self.log.lines.is_empty() {
            self.log.show(ui, "pq-log", 150.0);
        }
    }

    fn autoplay(&mut self, host: &mut Host) {
        self.set_noise(0);
        self.set_noise(1);
        self.run_kem(host);
    }
}
