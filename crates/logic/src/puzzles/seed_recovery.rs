//! Prologue, part 1. One word of Grandpa's mnemonic is illegible. The BIP-39
//! checksum rules out ~15/16 of the wordlist; the master fingerprint on the
//! sticky note pins down the one true word.

use std::rc::Rc;

use cpl_crypto::bip32::master_fingerprint;
use cpl_crypto::bip39::{WORDLIST, checksum_candidates, checksum_report, is_word, seed_from_mnemonic};
use cpl_crypto::bytes::to_hex;
use cpl_crypto::secp::sha256;
use egui::{Color32, RichText};

use crate::puzzle::{Host, Puzzle};
use crate::ui::theme::{self, AMBER, CYAN, FAINT, GREEN, RED, mono};
use crate::ui::widgets::{Kind, Tone, bits_view, button, button_enabled, card, columns2, kv, lede, meter, note, pill};
use crate::world::World;

struct Sweep {
    stage: u8,
    timer: f32,
    valid: Vec<&'static str>,
    lettered: Vec<&'static str>,
    results: Vec<(String, bool)>,
    matched: Option<&'static str>,
}

pub struct SeedRecovery {
    world: Rc<World>,
    input: String,
    sweep: Option<Sweep>,
    locked: bool,
}

impl SeedRecovery {
    pub fn new(world: Rc<World>) -> Self {
        Self { world, input: String::new(), sweep: None, locked: false }
    }

    fn words(&self, candidate: &str) -> Vec<String> {
        let g = &self.world.grandpa().mnemonic;
        let mut w: Vec<String> = g.words.iter().map(|s| s.to_string()).collect();
        w[g.missing_index] = candidate.to_string();
        w
    }

    fn fingerprint_for(&self, candidate: &str) -> String {
        master_fingerprint(&seed_from_mnemonic(&self.words(candidate).join(" "), ""))
    }

    fn try_word(&mut self, host: &mut Host) {
        if host.solved() {
            return;
        }
        let g = &self.world.grandpa().mnemonic;
        let cand = self.input.trim().to_lowercase();
        if !is_word(&cand) {
            host.status("That is not a BIP-39 word.", Tone::Bad);
            host.sfx("error");
            return;
        }
        if !cand.starts_with(g.first_letter) {
            host.status(format!("The surviving letter is “{}”; “{cand}” cannot be it.", g.first_letter), Tone::Bad);
            host.sfx("error");
            return;
        }
        let rep = checksum_report(&self.words(&cand)).unwrap();
        if !rep.ok {
            host.status(format!("Checksum mismatch: “{cand}” would make the last 4 bits {}, but SHA-256 says {}.", rep.checksum_bits, rep.expected_bits), Tone::Bad);
            host.sfx("error");
            return;
        }
        let fp = self.fingerprint_for(&cand);
        if fp != g.fingerprint {
            host.status(format!("Checksum passes, but the fingerprint is {fp}, not {}. About one wrong word in sixteen slips past a 4-bit checksum.", g.fingerprint), Tone::Warn);
            host.sfx("error");
            return;
        }
        host.sfx("unlock");
        self.locked = true;
        host.solve(format!("Fingerprint {fp} matches. The seed is whole again."));
    }

    fn step_sweep(&mut self, host: &mut Host) {
        let g = &self.world.grandpa().mnemonic;
        let (missing, first, target) = (g.missing_index, g.first_letter, g.fingerprint.clone());
        let words: Vec<Option<&str>> = g.words.iter().enumerate().map(|(i, w)| (i != missing).then_some(*w)).collect();
        let Some(sw) = self.sweep.as_mut() else { return };
        sw.timer += host.dt;
        match sw.stage {
            0 if sw.timer > 0.35 => {
                sw.valid = checksum_candidates(&words, missing);
                sw.stage = 1;
                sw.timer = 0.0;
            }
            1 if sw.timer > 0.35 => {
                sw.lettered = sw.valid.iter().copied().filter(|w| w.starts_with(first)).collect();
                sw.stage = 2;
                sw.timer = 0.0;
            }
            2 if sw.timer > 0.06 => {
                sw.timer = 0.0;
                let i = sw.results.len();
                if i < sw.lettered.len() {
                    let w = sw.lettered[i];
                    let mut trial: Vec<String> = g.words.iter().map(|s| s.to_string()).collect();
                    trial[missing] = w.to_string();
                    let fp = master_fingerprint(&seed_from_mnemonic(&trial.join(" "), ""));
                    let ok = fp == target;
                    if ok {
                        sw.matched = Some(w);
                    }
                    sw.results.push((fp, ok));
                } else {
                    sw.stage = 3;
                }
            }
            _ => {}
        }
    }

    fn funnel(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        let first = self.world.grandpa().mnemonic.first_letter;
        let Some(sw) = self.sweep.as_ref() else { return };
        let row = |ui: &mut egui::Ui, label: &str, count: usize, frac: f32| {
            ui.horizontal(|ui| {
                ui.add_sized([180.0, 18.0], egui::Label::new(RichText::new(label).font(mono(13.0))));
                ui.add_sized([50.0, 18.0], egui::Label::new(RichText::new(count.to_string()).font(mono(14.0)).color(CYAN)));
                meter(ui, frac.max(0.01), theme::CYAN_DIM);
            });
        };
        row(ui, "whole wordlist", 2048, 1.0);
        if sw.stage >= 1 {
            row(ui, "checksum fits", sw.valid.len(), sw.valid.len() as f32 / 2048.0);
        }
        if sw.stage >= 2 {
            row(ui, &format!("starts with “{first}”"), sw.lettered.len(), sw.lettered.len() as f32 / 2048.0);
            ui.horizontal_wrapped(|ui| {
                for (i, w) in sw.lettered.iter().enumerate() {
                    let r = match sw.results.get(i) {
                        Some((fp, true)) => pill(ui, w, Color32::BLACK, GREEN).on_hover_text(format!("fingerprint {fp}")),
                        Some((fp, false)) => pill(ui, w, FAINT, theme::rgb(0x0a0f1d)).on_hover_text(format!("fingerprint {fp}")),
                        None => pill(ui, w, theme::DIM, theme::rgb(0x111830)),
                    };
                    let _ = r;
                }
            });
        }
        if sw.stage >= 3 {
            row(ui, "fingerprint matches", usize::from(sw.matched.is_some()), 1.0 / 2048.0);
            note(ui, "Each survivor cost one PBKDF2 run (2048 × HMAC-SHA512). The checksum did the cheap filtering; the fingerprint did the rest.");
            if let Some(m) = sw.matched
                && button(ui, &format!("Use “{m}”"), Kind::Primary).clicked()
            {
                self.input = m.to_string();
                self.try_word(host);
            }
        }
    }
}

impl Puzzle for SeedRecovery {
    fn id(&self) -> &'static str {
        "pro.seed"
    }

    fn title(&self) -> &str {
        "SEED RESTORE"
    }

    fn subtitle(&self) -> Option<&str> {
        Some("BIP-39 · 12 words · 128-bit entropy")
    }

    fn accent(&self) -> Color32 {
        GREEN
    }

    fn open(&mut self, host: &mut Host) {
        host.status("Type a candidate word, or just press “Run the recovery sweep” to let the machine find it.", Tone::Info);
    }

    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        self.step_sweep(host);
        let world = self.world.clone();
        let g = &world.grandpa().mnemonic;
        lede(
            ui,
            &format!(
                "Grandpa's letter lists twelve words. Word **{}** sits under a coffee ring: only its first letter, **“{}”**, survives. The sticky note on the monitor gives the wallet's {{master fingerprint}}. Find the word whose seed produces that fingerprint.",
                g.missing_index + 1,
                g.first_letter
            ),
        );
        ui.add_space(4.0);
        card(ui, Some("The letter"), None, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (i, w) in g.words.iter().enumerate() {
                    ui.label(RichText::new(format!("{:02}", i + 1)).font(mono(12.0)).color(if i == g.missing_index { AMBER } else { FAINT }));
                    if i == g.missing_index {
                        let r = ui.add_enabled(
                            !self.locked,
                            egui::TextEdit::singleline(&mut self.input).hint_text(format!("{}▒▒▒▒", g.first_letter)).font(mono(14.0)).desired_width(110.0).text_color(if self.locked { GREEN } else { AMBER }),
                        );
                        if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            self.try_word(host);
                        }
                    } else {
                        ui.label(RichText::new(*w).font(mono(14.0)));
                    }
                    ui.add_space(10.0);
                }
            });
        });
        ui.add_space(4.0);
        let cand = self.input.trim().to_lowercase();
        let missing = g.missing_index;
        let preview = if is_word(&cand) { Some(checksum_report(&self.words(&cand)).unwrap()) } else { None };
        let placeholder = checksum_report(&self.words("abandon")).unwrap();
        let mut try_clicked = false;
        columns2(
            ui,
            |ui| {
                card(ui, Some("What the words encode"), None, |ui| {
                    note(ui, "Each word is an 11-bit index into the 2048-word list. 12 × 11 = 132 bits: 128 bits of entropy, then a 4-bit checksum, the first 4 bits of SHA-256(entropy).");
                    ui.add_space(4.0);
                    match &preview {
                        None => {
                            bits_view(ui, &placeholder.bits, 11, |i| {
                                if i >= 128 {
                                    Some(AMBER)
                                } else if i >= missing * 11 && i < missing * 11 + 11 {
                                    Some(RED)
                                } else {
                                    None
                                }
                            });
                            if cand.is_empty() {
                                note(ui, "The red bits are unknown until you choose a word.");
                            } else {
                                ui.label(RichText::new(format!("“{cand}” is not in the BIP-39 wordlist.")).color(RED).font(mono(13.0)));
                            }
                        }
                        Some(rep) => {
                            let ok = rep.ok;
                            bits_view(ui, &rep.bits, 11, |i| (i >= 128).then_some(if ok { GREEN } else { RED }));
                            let digest = to_hex(&sha256(&rep.entropy));
                            note(ui, &format!("SHA-256(entropy) = `{}…` → first 4 bits **{}**; the words say **{}**.", &digest[..8], rep.expected_bits, rep.checksum_bits));
                            ui.label(RichText::new(if ok { "Checksum OK." } else { "Checksum fails." }).color(if ok { GREEN } else { RED }).font(mono(13.0)));
                        }
                    }
                });
            },
            |ui| {
                card(ui, Some("Sticky note"), Some(AMBER.gamma_multiply(0.6)), |ui| {
                    ui.add(egui::Label::new(RichText::new(&world.grandpa().descriptor).font(mono(12.0)).color(AMBER)).wrap());
                    ui.add_space(6.0);
                    kv(ui, "seed-kv", &[("master fingerprint", g.fingerprint.clone()), ("meaning", "first 4 bytes of HASH160(master public key)".into())]);
                    ui.add_space(6.0);
                    if button_enabled(ui, !host.solved(), "Try this word", Kind::Primary).clicked() {
                        try_clicked = true;
                    }
                });
            },
        );
        if try_clicked {
            self.try_word(host);
        }
        ui.add_space(4.0);
        card(ui, Some("Recovery sweep"), None, |ui| {
            note(ui, "Too many words start with that letter? Let the machine do what wallet-recovery tools do: test the whole wordlist against the checksum, keep the ones that fit the visible letter, then derive a seed for each survivor and compare fingerprints.");
            ui.add_space(4.0);
            let running = self.sweep.as_ref().is_some_and(|s| s.stage < 3);
            if button_enabled(ui, !running, "Run the recovery sweep", Kind::Normal).clicked() {
                self.sweep = Some(Sweep { stage: 0, timer: 0.0, valid: Vec::new(), lettered: Vec::new(), results: Vec::new(), matched: None });
            }
            self.funnel(ui, host);
        });
        let _ = WORDLIST.len();
    }

    fn autoplay(&mut self, host: &mut Host) {
        let g = &self.world.grandpa().mnemonic;
        self.input = g.words[g.missing_index].to_string();
        self.try_word(host);
    }
}
