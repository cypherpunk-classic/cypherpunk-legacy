//! Chapter 1, part 2. The boathouse gate checks encrypted tickets. With
//! AES-CTR you can rewrite "role=guest" into "role=admin" without the key, by
//! flipping ciphertext bits. Then the gate is upgraded to AES-GCM and the same
//! trick is rejected outright.

use std::rc::Rc;

use cpl_crypto::aead::{CtrTicket, GcmBox, ctr_decrypt, ctr_encrypt, gcm_decrypt_text, gcm_encrypt_text};
use cpl_crypto::bytes::{ascii_to_bytes, to_hex};
use egui::{Color32, RichText, Sense, Stroke};

use crate::puzzle::{Host, Puzzle};
use crate::ui::theme::{self, AMBER, CYAN, DIM, GREEN, LINE, RED, VIOLET, mono};
use crate::ui::widgets::{Kind, Tone, button, card, columns2, lede, math, note};
use crate::world::World;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    Ctr,
    Gcm,
}

enum Pending {
    ToGcm,
    Solve,
}

pub struct BitFlip {
    world: Rc<World>,
    stage: Stage,
    plain: String,
    ticket: CtrTicket,
    gbox: GcmBox,
    work: Vec<u8>,
    selected: usize,
    want: String,
    result: Option<(String, Color32, Option<String>)>,
    gcm_rejected: bool,
    pending: Option<(f32, Pending)>,
}

impl BitFlip {
    pub fn new(world: Rc<World>) -> Self {
        let handle: String = world.handle().chars().take(16).collect();
        let plain = format!("role=guest;user={handle};pier=09");
        let (key, nonce) = *world.gate_key();
        let ticket = ctr_encrypt(&key, &nonce, &plain);
        let n12: [u8; 12] = nonce[..12].try_into().unwrap();
        let gbox = gcm_encrypt_text(&key, &n12, &plain);
        let work = ticket.ct.clone();
        Self { world, stage: Stage::Ctr, plain, ticket, gbox, work, selected: 5, want: String::new(), result: None, gcm_rejected: false, pending: None }
    }

    fn original(&self) -> &[u8] {
        match self.stage {
            Stage::Ctr => &self.ticket.ct,
            Stage::Gcm => &self.gbox.ct,
        }
    }

    fn predicted(&self, i: usize) -> u8 {
        self.plain.as_bytes()[i] ^ self.original()[i] ^ self.work[i]
    }

    fn to_gcm(&mut self, host: &mut Host) {
        self.stage = Stage::Gcm;
        self.work = self.gbox.ct.clone();
        self.selected = 5;
        self.result = None;
        host.status("The gate reboots with new firmware: AES-256-GCM.", Tone::Warn);
    }

    fn reset(&mut self) {
        self.work = self.original().to_vec();
        self.result = None;
    }

    fn present(&mut self, host: &mut Host) {
        if host.solved() || self.pending.is_some() {
            return;
        }
        let (key, _) = *self.world.gate_key();
        if self.stage == Stage::Ctr {
            let text = ctr_decrypt(&key, &CtrTicket { nonce: self.ticket.nonce, ct: self.work.clone() });
            let role = text.split(';').find_map(|kv| kv.strip_prefix("role=")).unwrap_or("?").to_string();
            let ok = role == "admin";
            self.result = Some((text, if ok { GREEN } else { AMBER }, Some("Gate decrypted your ticket to:".into())));
            if ok {
                host.sfx("unlock");
                host.status("GATE: welcome, admin. (It never knew the key was not yours.)", Tone::Ok);
                self.pending = Some((1.6, Pending::ToGcm));
            } else {
                host.status(if role == "guest" { "GATE: guests may not open lockboxes.".to_string() } else { format!("GATE: unknown role “{role}”.") }, Tone::Bad);
                host.sfx("error");
            }
            return;
        }
        match gcm_decrypt_text(&key, &GcmBox { nonce: self.gbox.nonce, ct: self.work.clone() }) {
            Err(_) => {
                self.gcm_rejected = true;
                self.result = Some(("REJECTED: authentication tag mismatch".into(), RED, None));
                host.sfx("error");
                host.status("Forgery detected. That is what authenticated encryption is for.", Tone::Ok);
                self.pending = Some((0.9, Pending::Solve));
            }
            Ok(text) => {
                let modified = self.work != self.gbox.ct;
                self.result = Some((text, AMBER, Some(if modified { "Hm, your edit survived? That should not happen.".into() } else { "Your unmodified ticket decrypts fine, and still says guest. Now try changing it.".into() })));
                host.status(if self.gcm_rejected { "Still a guest." } else { "Valid, but you are still a guest. Edit the ticket and present it again." }, Tone::Warn);
            }
        }
    }

    fn tick(&mut self, host: &mut Host) {
        if let Some((t, _)) = &mut self.pending {
            *t -= host.dt;
            if *t <= 0.0 {
                let (_, what) = self.pending.take().unwrap();
                match what {
                    Pending::ToGcm => self.to_gcm(host),
                    Pending::Solve => host.solve("Lesson learned: encryption hides, authentication protects. Use AEAD."),
                }
            }
        }
    }
}

impl Puzzle for BitFlip {
    fn id(&self) -> &'static str {
        "ch1.ctr"
    }

    fn title(&self) -> &str {
        "BOATHOUSE TICKET GATE"
    }

    fn subtitle(&self) -> Option<&str> {
        Some(match self.stage {
            Stage::Ctr => "AES-256-CTR (no tag)",
            Stage::Gcm => "AES-256-GCM",
        })
    }

    fn accent(&self) -> Color32 {
        theme::rgb(0x73eff7)
    }

    fn open(&mut self, host: &mut Host) {
        host.status("Goal: make the gate read role=admin.", Tone::Info);
    }

    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        self.tick(host);
        match self.stage {
            Stage::Ctr => {
                lede(ui, "Mara's lockbox is behind a Directorate ticket gate. Your ticket says `role=guest`. The gate decrypts tickets with AES in counter mode, which XORs a keystream into the bytes. You do not know the key. You do know your own ticket's plaintext.");
                math(ui, "C = P ⊕ keystream   ⇒   (C ⊕ Δ) decrypts to (P ⊕ Δ)");
            }
            Stage::Gcm => {
                lede(ui, "Mara has swapped the gate's firmware for AES-GCM: the same counter-mode encryption, plus a 16-byte authentication tag computed over the ciphertext. Try your trick again.");
                math(ui, "C = P ⊕ keystream,   tag = GHASH_H(C) ⊕ E_K(nonce ‖ 1)");
            }
        }
        let n = self.plain.len();
        card(ui, Some("Your ticket"), None, |ui| {
            note(ui, "Click a byte to see and flip its bits. The top line is what you know the gate will decrypt it to.");
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(3.0, 3.0);
                for i in 0..n {
                    let p = self.predicted(i);
                    let changed = p != self.plain.as_bytes()[i];
                    let ch = if (32..127).contains(&p) { (p as char).to_string() } else { "·".into() };
                    let (r, resp) = ui.allocate_exact_size(egui::vec2(32.0, 38.0), Sense::click());
                    let sel = i == self.selected;
                    ui.painter().rect_filled(r, 3.0, theme::rgb(0x0d1220));
                    ui.painter().rect_stroke(r, 3.0, Stroke::new(1.0, if sel { CYAN } else { LINE }), egui::StrokeKind::Inside);
                    ui.painter().text(r.center_top() + egui::vec2(0.0, 11.0), egui::Align2::CENTER_CENTER, ch, mono(15.0), if changed { AMBER } else { theme::TEXT });
                    ui.painter().text(r.center_bottom() - egui::vec2(0.0, 9.0), egui::Align2::CENTER_CENTER, format!("{:02x}", self.work[i]), mono(10.5), DIM);
                    if resp.clicked() {
                        self.selected = i;
                    }
                }
                if self.stage == Stage::Gcm {
                    let tag = to_hex(&self.work[n..]);
                    let (r, _) = ui.allocate_exact_size(egui::vec2(110.0, 38.0), Sense::hover());
                    ui.painter().rect_stroke(r, 3.0, Stroke::new(1.0, VIOLET), egui::StrokeKind::Inside);
                    ui.painter().text(r.center_top() + egui::vec2(0.0, 11.0), egui::Align2::CENTER_CENTER, "TAG", mono(12.0), VIOLET);
                    ui.painter().text(r.center_bottom() - egui::vec2(0.0, 9.0), egui::Align2::CENTER_CENTER, format!("{}…", &tag[..12]), mono(10.5), DIM);
                }
            });
            ui.add_space(6.0);
            let i = self.selected;
            let v = self.work[i];
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(format!("byte {i} · ciphertext bits (click to flip):")).font(mono(12.5)).color(DIM));
                for b in (0..8).rev() {
                    let on = (v >> b) & 1 == 1;
                    let btn = egui::Button::new(RichText::new(if on { "1" } else { "0" }).font(mono(13.0)).color(if on { theme::rgb(0xbff8ff) } else { DIM }))
                        .fill(if on { theme::rgb(0x10283a) } else { theme::rgb(0x121a2e) })
                        .min_size(egui::vec2(26.0, 22.0));
                    if ui.add(btn).clicked() {
                        self.work[i] ^= 1 << b;
                        host.sfx("blip");
                    }
                }
                let p = self.predicted(i);
                ui.label(RichText::new(format!("→ decrypts to “{}” (0x{:02x})", if (32..127).contains(&p) { p as char } else { '·' }, p)).font(mono(12.5)).color(DIM));
            });
        });
        let mut present = false;
        let mut reset = false;
        let mut apply: Option<u8> = None;
        let i = self.selected;
        let cur = self.predicted(i);
        columns2(
            ui,
            |ui| {
                card(ui, Some("XOR helper"), None, |ui| {
                    note(ui, &format!("Byte {i} currently decrypts to “{}”.", if (32..127).contains(&cur) { cur as char } else { '·' }));
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut self.want).char_limit(1).desired_width(44.0).font(mono(15.0)).hint_text("?"));
                        if button(ui, "Apply Δ to this byte", Kind::Normal).clicked()
                            && let Some(w) = self.want.chars().next()
                        {
                            apply = Some(cur ^ (w as u32 as u8));
                        }
                    });
                    match self.want.chars().next() {
                        None => note(ui, "Type the character you want at this position."),
                        Some(w) => {
                            let wb = w as u32 as u8;
                            ui.label(RichText::new(format!("“{}” = {:08b}", cur as char, cur)).font(mono(12.5)));
                            ui.label(RichText::new(format!("“{w}” = {wb:08b}")).font(mono(12.5)));
                            ui.label(RichText::new(format!("flip mask Δ = {:08b}", cur ^ wb)).font(mono(12.5)).color(AMBER));
                        }
                    }
                });
            },
            |ui| {
                card(ui, Some("Present the ticket"), None, |ui| {
                    ui.horizontal(|ui| {
                        if button(ui, "Present to gate", Kind::Primary).clicked() {
                            present = true;
                        }
                        if button(ui, "Reset ticket", Kind::Normal).clicked() {
                            reset = true;
                        }
                    });
                    if let Some((text, color, label)) = &self.result {
                        if let Some(l) = label {
                            note(ui, l);
                        }
                        ui.add(egui::Label::new(RichText::new(text).font(mono(15.0)).color(*color)).wrap());
                        if *color == RED {
                            note(ui, "The gate never decrypted your edit. It recomputed the tag over the ciphertext it received, saw it differ, and refused to say anything else.");
                        }
                    }
                });
            },
        );
        if let Some(delta) = apply {
            self.work[i] ^= delta;
            host.sfx("select");
            if i + 1 < n {
                self.selected = i + 1;
            }
            self.want.clear();
        }
        if reset {
            self.reset();
        }
        if present {
            self.present(host);
        }
    }

    fn autoplay(&mut self, host: &mut Host) {
        if self.stage == Stage::Ctr {
            let target = self.plain.replace("role=guest", "role=admin");
            let (a, b) = (ascii_to_bytes(&self.plain), ascii_to_bytes(&target));
            for i in 0..a.len() {
                self.work[i] ^= a[i] ^ b[i];
            }
            self.present(host);
            self.pending = None;
            self.to_gcm(host);
        }
        self.work[5] ^= 0x01;
        self.present(host);
        self.pending = None;
        host.solve("Lesson learned: encryption hides, authentication protects. Use AEAD.");
    }
}
