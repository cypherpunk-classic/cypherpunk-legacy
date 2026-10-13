//! Chapter 3, part 2. Now over the real secp256k1 scalar field, so the secret
//! is an actual key. You have four shares: two from people, two from dead
//! drops. One dead drop was planted by the Directorate. Feldman commitments
//! C_j = a_j·G let you check each share, y·G == Σ xʲ·C_j, without knowing the
//! secret, and catch the forgery before it poisons the reconstruction.

use std::rc::Rc;

use cpl_crypto::bytes::{big_to_hex, to_hex};
use cpl_crypto::secp::{N, mul, x_only};
use cpl_crypto::shamir::{Share, feldman_check, reconstruct};
use egui::{Color32, RichText};

use crate::puzzle::{Host, Puzzle};
use crate::ui::theme::{self, DIM, GREEN, RED, TEXT, mono};
use crate::ui::widgets::{Kind, Log, Tone, button, button_enabled, card, kv, lede, math, note, shorten, tag};
use crate::world::World;

struct Candidate {
    label: &'static str,
    share: Share,
    genuine: bool,
    /// `Some(ok)` once verified
    checked: Option<bool>,
    chosen: bool,
}

pub struct FeldmanVerify {
    world: Rc<World>,
    shares: Vec<Candidate>,
    log: Log,
}

impl FeldmanVerify {
    pub fn new(world: Rc<World>) -> Self {
        let v = world.vault();
        let c = |label, share: &Share, genuine| Candidate { label, share: share.clone(), genuine, checked: None, chosen: false };
        // genuine shares 1, 2 (held) and 4 (good drop); forged 3 (planted drop)
        let shares = vec![
            c("Share 1 — Mara", &v.shares[0], true),
            c("Share 2 — Felix", &v.shares[1], true),
            c("Share 3 — dead drop (loose brick)", &v.forged3, false),
            c("Share 4 — dead drop (hollow oak)", &v.shares[3], true),
        ];
        Self { world, shares, log: Log::default() }
    }

    fn chosen(&self) -> usize {
        self.shares.iter().filter(|s| s.chosen).count()
    }

    fn verify(&mut self, i: usize, host: &mut Host) {
        let ok = feldman_check(&self.shares[i].share, &self.world.vault().commitments).ok;
        let s = &mut self.shares[i];
        s.checked = Some(ok);
        self.log.line(
            format!("{}: y·G {} Σ xʲ·Cⱼ  →  {}", s.label, if ok { "==" } else { "≠" }, if ok { "genuine" } else { "FORGED, discard" }),
            if ok { Tone::Ok } else { Tone::Bad },
        );
        host.sfx(if ok { "confirm" } else { "error" });
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
        let forged = picks.iter().any(|p| !p.genuine);
        let shares: Vec<Share> = picks.iter().map(|p| p.share.clone()).collect();
        let recon = reconstruct(&shares, &N);
        let vx = to_hex(&x_only(&mul(&recon)));
        if forged || recon != self.world.vault().d {
            self.log.line(format!("Reconstructed key {} — does NOT match the Vault public key. A bad share poisons everything.", shorten(&vx, 8, 6)), Tone::Bad);
            host.status("That set includes a forged share. The result is a wrong key.", Tone::Bad);
            host.sfx("error");
            return;
        }
        self.log.line(format!("Reconstructed secret · publicly: {} == C₀. It matches the Vault.", shorten(&vx, 10, 8)), Tone::Ok);
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
            note(ui, "Verify each share, then choose three that pass. C₀ is the Vault public key itself: y·G for a good share lands on the committed curve.");
            for (i, s) in self.shares.iter().enumerate() {
                let accent = s.checked.map(|ok| if ok { GREEN } else { RED });
                theme::card_frame(accent).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new(s.label).font(mono(13.5)).color(TEXT).strong());
                            ui.label(RichText::new(format!("x = {} · y = {}", s.share.x, shorten(&big_to_hex(&s.share.y, 32), 10, 8))).font(mono(11.5)).color(DIM));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| match s.checked {
                            Some(ok) => {
                                tag(ui, if ok { "VALID" } else { "FORGED" }, if ok { Tone::Ok } else { Tone::Bad });
                            }
                            None => {
                                if button(ui, "Verify", Kind::Normal).clicked() {
                                    verify = Some(i);
                                }
                            }
                        });
                    });
                    if let Some(ok) = s.checked {
                        let mut on = s.chosen;
                        let label = RichText::new(if ok { "use this share" } else { "do not use" }).font(mono(12.5)).color(DIM);
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
        for i in [0, 1, 3] {
            self.toggle(i, true);
        }
        self.reconstruct(host);
    }
}
