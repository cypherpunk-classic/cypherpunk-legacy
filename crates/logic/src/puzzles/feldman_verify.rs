//! Chapter 3, part 2. Now over the real secp256k1 scalar field, so the secret
//! is an actual key. You have four shares: two from people, two from dead
//! drops. One dead drop was planted by the Directorate. Feldman commitments
//! C_j = a_j·G let you check each share, y·G == Σ xʲ·C_j, without knowing the
//! secret, and catch the forgery before it poisons the reconstruction. The
//! terminal computes both sides; reading them and choosing is the player's.

use std::rc::Rc;

use cpl_crypto::bytes::{big_to_hex, to_hex};
use cpl_crypto::secp::{N, mul, x_only};
use cpl_crypto::dh::TOY_GROUP;
use cpl_crypto::shamir::{Share, feldman_check, reconstruct};
use egui::{Color32, RichText};

use crate::puzzle::{HandCell, HandSheet, Host, Puzzle, StepView, steps};
use crate::ui::theme::{self, CYAN, DIM, GREEN, TEXT, mono};
use crate::ui::widgets::{Kind, Log, Tone, button, button_enabled, card, hex_diff, kv, lede, math, note, shorten, tag};
use crate::world::World;

struct Candidate {
    label: &'static str,
    share: Share,
    genuine: bool,
    /// `Some(ok)` once verified
    checked: Option<bool>,
    /// both sides of the check, x-only hex: (y·G, C₀ + x·C₁ + x²·C₂)
    sides: Option<(String, String)>,
    chosen: bool,
}

pub struct FeldmanVerify {
    world: Rc<World>,
    shares: Vec<Candidate>,
    log: Log,
    rebuilt: bool,
}

impl FeldmanVerify {
    pub fn new(world: Rc<World>) -> Self {
        let v = world.vault();
        let c = |label, share: &Share, genuine| Candidate { label, share: share.clone(), genuine, checked: None, sides: None, chosen: false };
        // genuine shares 1, 2 (held) and 4 (good drop); forged 3 (planted drop)
        let shares = vec![
            c("Share 1 — Mara", &v.shares[0], true),
            c("Share 2 — Felix", &v.shares[1], true),
            c("Share 3 — dead drop (loose brick)", &v.forged3, false),
            c("Share 4 — dead drop (hollow oak)", &v.shares[3], true),
        ];
        Self { world, shares, log: Log::default(), rebuilt: false }
    }

    fn chosen(&self) -> usize {
        self.shares.iter().filter(|s| s.chosen).count()
    }

    fn verify(&mut self, i: usize, host: &mut Host) {
        let check = feldman_check(&self.shares[i].share, &self.world.vault().commitments);
        let s = &mut self.shares[i];
        s.checked = Some(check.ok);
        s.sides = Some((to_hex(&x_only(&check.lhs)), to_hex(&x_only(&check.rhs))));
        self.log.line(format!("{}: computed y·G and C₀ + {x}·C₁ + {x2}·C₂", s.label, x = s.share.x, x2 = &s.share.x * &s.share.x), Tone::Info);
        host.sfx("blip");
    }

    fn toggle(&mut self, i: usize, on: bool) {
        if self.shares[i].checked.is_none() {
            return;
        }
        if on && !self.shares[i].chosen && self.chosen() >= 3 {
            return;
        }
        self.shares[i].chosen = on;
    }

    fn reconstruct(&mut self, host: &mut Host) {
        if host.solved() || self.chosen() != 3 {
            return;
        }
        let picks: Vec<&Candidate> = self.shares.iter().filter(|s| s.chosen).collect();
        let forged: Vec<&str> = picks.iter().filter(|p| !p.genuine).map(|p| p.label).collect();
        let shares: Vec<Share> = picks.iter().map(|p| p.share.clone()).collect();
        let recon = reconstruct(&shares, &N);
        let vx = to_hex(&x_only(&mul(&recon)));
        if !forged.is_empty() || recon != self.world.vault().d {
            self.log.line(format!("Reconstructed key {} — does NOT match C₀, the Vault public key. A bad share poisons everything.", shorten(&vx, 8, 6)), Tone::Bad);
            host.status(format!("{} was in the set, and its two sides differ: y·G is not on the committed polynomial. The result is a wrong key.", forged.first().copied().unwrap_or("A forged share")), Tone::Bad);
            host.sfx("error");
            return;
        }
        self.log.line(format!("Reconstructed secret · publicly: {} == C₀. It matches the Vault.", shorten(&vx, 10, 8)), Tone::Ok);
        self.rebuilt = true;
        host.sfx("unlock");
        host.solve("The Vault key is whole, and genuine. Three good shares, one caught forgery.");
    }
}

impl Puzzle for FeldmanVerify {
    fn id(&self) -> &'static str {
        "ch3.feldman"
    }

    fn title(&self) -> &str {
        "VAULT KEY · VERIFY SHARES"
    }

    fn subtitle(&self) -> Option<&str> {
        Some("Feldman VSS over secp256k1")
    }

    fn accent(&self) -> Color32 {
        GREEN
    }

    fn open(&mut self, host: &mut Host) {
        host.status("Verify the shares before you trust them.", Tone::Info);
    }

    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        lede(ui, "The Vault key was split five ways over the real curve. You hold four shares; you need three good ones. But the Directorate leaves forged shares in dead drops, hoping you will rebuild the wrong key. Grandpa published **commitments** so you can tell.");
        math(ui, "dealer publishes Cⱼ = aⱼ·G.   Share (x, y) is genuine ⇔ y·G = C₀ + x·C₁ + x²·C₂");
        let rows: Vec<(String, String)> = self.world.vault().commitments.iter().enumerate().map(|(j, c)| (format!("C{j}"), shorten(&to_hex(&x_only(c)), 10, 8))).collect();
        card(ui, Some("Public commitments (from Grandpa)"), None, |ui| {
            let rows: Vec<(&str, String)> = rows.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
            kv(ui, "feldman-commit", &rows);
        });
        let mut verify = None;
        let mut toggles = Vec::new();
        card(ui, Some("Shares"), None, |ui| {
            note(ui, "Compute both sides for each share, compare them, then choose three that pass. C₀ is the Vault public key itself: y·G for a good share lands exactly on the committed polynomial. Red digits mark where the sides differ.");
            for (i, s) in self.shares.iter().enumerate() {
                let accent = s.chosen.then_some(CYAN);
                theme::card_frame(accent).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new(s.label).font(mono(13.5)).color(TEXT).strong());
                            ui.label(RichText::new(format!("x = {} · y = {}", s.share.x, shorten(&big_to_hex(&s.share.y, 32), 10, 8))).font(mono(11.5)).color(DIM));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| match s.checked {
                            Some(_) => {
                                tag(ui, "COMPUTED", Tone::Dim);
                            }
                            None => {
                                if button(ui, "Compute both sides", Kind::Normal).clicked() {
                                    verify = Some(i);
                                }
                            }
                        });
                    });
                    if let Some((lhs, rhs)) = &s.sides {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("y·G           ").font(mono(12.0)).color(DIM));
                            ui.label(RichText::new(lhs).font(mono(13.0)).color(CYAN));
                        });
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("C₀+x·C₁+x²·C₂ ").font(mono(12.0)).color(DIM));
                            hex_diff(ui, lhs, rhs);
                        });
                        let mut on = s.chosen;
                        let label = RichText::new("use this share").font(mono(12.5)).color(DIM);
                        if ui.checkbox(&mut on, label).changed() {
                            toggles.push((i, on));
                        }
                    }
                });
            }
        });
        if let Some(i) = verify {
            self.verify(i, host);
        }
        for (i, on) in toggles {
            self.toggle(i, on);
        }
        let mut recon = false;
        let ready = self.chosen() == 3;
        card(ui, Some("Reconstruct"), None, |ui| {
            if button_enabled(ui, ready, "Reconstruct from the 3 chosen shares", Kind::Primary).clicked() {
                recon = true;
            }
            if !ready {
                note(ui, &format!("{} of 3 shares chosen.", self.chosen()));
            }
            ui.add_space(6.0);
            self.log.show(ui, "feldman-log", 170.0);
        });
        if recon {
            self.reconstruct(host);
        }
    }

    fn autoplay(&mut self, host: &mut Host) {
        for i in 0..self.shares.len() {
            self.verify(i, host);
        }
        // keep the shares whose sides match
        let good: Vec<usize> = (0..self.shares.len()).filter(|&i| self.shares[i].sides.as_ref().is_some_and(|(l, r)| l == r)).collect();
        for i in good.into_iter().take(3) {
            self.toggle(i, true);
        }
        self.reconstruct(host);
    }

    fn steps(&self) -> Vec<StepView> {
        let all_checked = self.shares.iter().all(|s| s.checked.is_some());
        let good_three = self.chosen() == 3 && self.shares.iter().filter(|s| s.chosen).all(|s| s.genuine);
        steps(&[
            ("Compute both sides for every share", "y·G on one side, C₀ + x·C₁ + x²·C₂ on the other. Nobody needs the secret to do this.", all_checked),
            ("Choose three shares whose sides match", "One dead drop was planted. Its two sides disagree.", good_three),
            ("Rebuild the Vault key", "", self.rebuilt),
        ])
    }

    fn deep_sheet(&self) -> Option<HandSheet> {
        let g = TOY_GROUP;
        let mut rng = self.world.rng("ch3/feldman-deep");
        let (s0, a1, a2) = (2 + rng.int(500) as u64, 2 + rng.int(500) as u64, 2 + rng.int(500) as u64);
        let f = |x: u64| (s0 + a1 * x + a2 * x * x) % g.q;
        let (c0, c1, c2) = (g.public(s0), g.public(a1), g.public(a2));
        let y2 = f(2);
        let y3 = (f(3) + 1 + rng.int(20) as u64) % g.q; // forged
        let rhs = |x: u64| c0 * g.pow(c1, x) % g.p * g.pow(c2, x * x) % g.p;
        let forged = if g.public(y3) == rhs(3) { 0 } else { 3 };
        Some(HandSheet::new(
            "Feldman in a toy group",
            format!(
                "Exponents mod q = {}, group elements mod p = {}, g = {}. The dealer published C₀ = {c0}, C₁ = {c1}, C₂ = {c2}. Two shares arrive: (2, {y2}) and (3, {y3}). In this group the check reads g^y = C₀ · C₁^x · C₂^(x²).",
                g.q, g.p, g.g
            ),
            300.0,
            vec![
                HandCell::new(format!("g^{y2} mod {}", g.p), "share 2, left side", g.public(y2) as i64),
                HandCell::new(format!("C₀ · C₁² · C₂⁴ mod {}", g.p), "share 2, right side", rhs(2) as i64),
                HandCell::new(format!("g^{y3} mod {}", g.p), "share 3, left side", g.public(y3) as i64),
                HandCell::new(format!("C₀ · C₁³ · C₂⁹ mod {}", g.p), "share 3, right side", rhs(3) as i64),
                HandCell::new("which share is forged? (2 or 3)", "the one whose sides differ", forged),
            ],
        ))
    }
}
