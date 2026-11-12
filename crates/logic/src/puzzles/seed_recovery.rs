//! Prologue, part 1. One word of Grandpa's mnemonic is illegible. The BIP-39
//! checksum rules out ~15/16 of the wordlist; the master fingerprint on the
//! sticky note pins down the one true word. The sweep does the cheap
//! filtering; deriving and comparing fingerprints is left to the player.

use std::rc::Rc;

use cpl_crypto::bip32::master_fingerprint;
use cpl_crypto::bip39::{WORDLIST, checksum_candidates, checksum_report, is_word, seed_from_mnemonic};
use cpl_crypto::bytes::to_hex;
use cpl_crypto::secp::sha256;
use egui::{Color32, RichText};

use crate::puzzle::{HandCell, HandSheet, Host, Puzzle, StepView, steps};
use crate::ui::theme::{self, AMBER, CYAN, DIM, FAINT, GREEN, RED, mono};
use crate::ui::widgets::{Kind, Tone, bits_view, button_enabled, card, columns2, hex_diff, kv, lede, meter, note, pill};
use crate::world::World;

struct Sweep {
    stage: u8,
    timer: f32,
    valid: Vec<&'static str>,
    lettered: Vec<&'static str>,
}

pub struct SeedRecovery {
    world: Rc<World>,
    input: String,
    sweep: Option<Sweep>,
    locked: bool,
    /// fingerprints the player has derived, by word
    derived: Vec<(&'static str, String)>,
    /// the last word whose fingerprint was derived (shown in the comparison)
    comparing: Option<&'static str>,
    seen_letter: bool,
    seen_checksum: bool,
}

impl SeedRecovery {
    pub fn new(world: Rc<World>) -> Self {
        Self { world, input: String::new(), sweep: None, locked: false, derived: Vec::new(), comparing: None, seen_letter: false, seen_checksum: false }
    }

    /// One PBKDF2 run: the seed for this candidate, and its master fingerprint.
    fn derive(&mut self, word: &'static str) {
        if !self.derived.iter().any(|(w, _)| *w == word) {
            let fp = self.fingerprint_for(word);
            self.derived.push((word, fp));
        }
        self.comparing = Some(word);
        self.input = word.to_string();
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
        self.seen_letter = true;
        let rep = checksum_report(&self.words(&cand)).unwrap();
        if !rep.ok {
            host.status(format!("Checksum mismatch: “{cand}” would make the last 4 bits {}, but SHA-256 says {}.", rep.checksum_bits, rep.expected_bits), Tone::Bad);
            host.sfx("error");
            return;
        }
        self.seen_checksum = true;
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
        let (missing, first) = (g.missing_index, g.first_letter);
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
                self.seen_checksum = true;
                host.status("The checksum and the letter left a handful. Derive each one's seed and compare fingerprints.", Tone::Info);
            }
            _ => {}
        }
    }

    fn funnel(&mut self, ui: &mut egui::Ui) {
        let first = self.world.grandpa().mnemonic.first_letter;
        let target = self.world.grandpa().mnemonic.fingerprint.clone();
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
        let mut derive = None;
        if sw.stage >= 2 {
            row(ui, &format!("starts with “{first}”"), sw.lettered.len(), sw.lettered.len() as f32 / 2048.0);
            note(ui, "The cheap filters stop here. Click a survivor to derive its seed (one PBKDF2 run: 2048 rounds of HMAC-SHA512) and read off its fingerprint.");
            ui.horizontal_wrapped(|ui| {
                for w in &sw.lettered {
                    let done = self.derived.iter().find(|(d, _)| d == w);
                    let r = match done {
                        Some((_, fp)) => pill(ui, &format!("{w} · {fp}"), theme::TEXT, theme::rgb(0x111830)),
                        None => pill(ui, w, DIM, theme::rgb(0x0a0f1d)),
                    }
                    .interact(egui::Sense::click())
                    .on_hover_text("derive the seed for this word");
                    if r.clicked() {
                        derive = Some(*w);
                    }
                }
            });
        }
        if let Some(w) = derive {
            self.derive(w);
        }
        if let Some(w) = self.comparing
            && let Some((_, fp)) = self.derived.iter().find(|(d, _)| *d == w)
        {
            card(ui, Some(&format!("“{w}” against the sticky note")), None, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("derived   ").font(mono(12.5)).color(DIM));
                    hex_diff(ui, &target, fp);
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("sticky    ").font(mono(12.5)).color(DIM));
                    ui.label(RichText::new(&target).font(mono(13.0)).color(AMBER));
                });
                note(ui, "Same eight hex digits? Then that is the word: it is already in the gap. Press “Try this word”.");
            });
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
        host.status("Type a candidate word. Too many to try? The sweep narrows them down.", Tone::Info);
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
        if preview.is_some() && cand.starts_with(g.first_letter) {
            self.seen_letter = true;
            if preview.as_ref().is_some_and(|r| r.ok) {
                self.seen_checksum = true;
            }
        }
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
            note(ui, "Too many words start with that letter? Do what wallet-recovery tools do: test the whole wordlist against the checksum and keep the ones that fit the visible letter. Those filters are cheap. Deriving a seed is not, so that part is yours.");
            ui.add_space(4.0);
            let running = self.sweep.as_ref().is_some_and(|s| s.stage < 2);
            if button_enabled(ui, !running && self.sweep.is_none(), "Run the recovery sweep", Kind::Normal).clicked() {
                self.sweep = Some(Sweep { stage: 0, timer: 0.0, valid: Vec::new(), lettered: Vec::new() });
            }
            self.funnel(ui);
        });
        let _ = (WORDLIST.len(), FAINT, GREEN, RED);
    }

    fn autoplay(&mut self, host: &mut Host) {
        let g = &self.world.grandpa().mnemonic;
        self.input = g.words[g.missing_index].to_string();
        self.try_word(host);
    }

    fn steps(&self) -> Vec<StepView> {
        let l = self.world.grandpa().mnemonic.first_letter;
        let first = format!("Pick a word that starts with “{l}”");
        steps(&[
            (&first, "Type it into the gap. Its 11 bits and the 4-bit checksum light up.", self.seen_letter || self.locked),
            ("Make the 4-bit checksum fit", "Only about one word in sixteen fits. The recovery sweep lists them all.", self.seen_checksum || self.locked),
            ("Match the master fingerprint", "Derive a survivor's seed and compare its fingerprint with the sticky note, digit by digit.", self.locked),
        ])
    }

    fn deep_sheet(&self) -> Option<HandSheet> {
        Some(HandSheet::new(
            "what a lost word costs",
            "Each word carries 11 bits; a 12-word phrase ends in a 4-bit checksum (a 24-word phrase in 8). Work out how much searching a loss costs, the way recovery services quote it.",
            150.0,
            vec![
                HandCell::new("candidates for one lost word", "the wordlist", 2048),
                HandCell::new("…that also fit the 4-bit checksum, on average", "one in 2⁴", 128),
                HandCell::new("candidates for two lost words", "2048 × 2048", 4_194_304),
                HandCell::new("…that fit the checksum", "divide by 16", 262_144),
                HandCell::new("bits of entropy in a 24-word phrase", "24·11, minus the 8 checksum bits", 256),
            ],
        ))
    }
}
