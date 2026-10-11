//! Chapter 1, part 1. Two radio messages were encrypted with the same pad
//! page. XOR the ciphertexts and the pad cancels: C1 ⊕ C2 = P1 ⊕ P2. Guess a
//! word in one message and the other message shows through.

use std::rc::Rc;

use cpl_crypto::bytes::to_hex;
use cpl_crypto::otp::{self, TwoTimePad as Ttp, crib_drag, partner, plausibility};
use egui::{Color32, Event, EventFilter, Key, RichText, Sense, Stroke};

use crate::puzzle::{Host, Puzzle};
use crate::ui::theme::{self, AMBER, CYAN, DIM, FAINT, GREEN, LINE, LINE2, RED, mono};
use crate::ui::widgets::{Kind, Tone, button, card, columns2, hex_box, lede, math, note, pill};
use crate::world::{CH1_P1, CH1_P2, World};

fn plausible(c: char) -> bool {
    c.is_ascii_uppercase() || c.is_ascii_digit() || " .,:;'!?-".contains(c)
}

pub struct TwoTimePad {
    t: Ttp,
    a: Vec<Option<char>>,
    b: Vec<Option<char>>,
    /// Which row the player typed at each position (0 = A, 1 = B); the other is derived.
    typed: Vec<Option<u8>>,
    crib: String,
    offset: usize,
    scan: Vec<(usize, String, f64)>,
    cursor: Option<(u8, usize)>,
    done: bool,
}

impl TwoTimePad {
    pub fn new(world: Rc<World>) -> Self {
        let t = otp::make_two_time_pad(CH1_P1, CH1_P2, &mut world.rng("ch1/pad"));
        let n = t.x.len();
        Self { t, a: vec![None; n], b: vec![None; n], typed: vec![None; n], crib: " THE ".into(), offset: 0, scan: Vec::new(), cursor: None, done: false }
    }

    fn set_char(&mut self, i: usize, row: u8, ch: Option<char>) {
        if i >= self.a.len() {
            return;
        }
        let derived = ch.map(|c| (self.t.x[i] ^ (c as u32 as u8)) as char);
        if row == 0 {
            self.a[i] = ch;
            self.b[i] = derived;
        } else {
            self.b[i] = ch;
            self.a[i] = derived;
        }
        self.typed[i] = ch.map(|_| row);
    }

    fn check(&mut self, host: &mut Host) {
        if self.done || host.solved() {
            return;
        }
        if self.a.iter().all(Option::is_some) && self.b.iter().all(Option::is_some) {
            let a: String = self.a.iter().map(|c| c.unwrap()).collect();
            let b: String = self.b.iter().map(|c| c.unwrap()).collect();
            if (a == CH1_P1 && b == CH1_P2) || (a == CH1_P2 && b == CH1_P1) {
                self.done = true;
                self.cursor = None;
                host.solve("Both messages recovered. The pad was never needed.");
            } else {
                host.status("Everything is filled in, but some of it is not quite right. Look for red letters or odd words.", Tone::Warn);
            }
        }
    }

    fn place_crib(&mut self, row: u8, host: &mut Host) {
        let crib: Vec<char> = self.crib.to_uppercase().chars().collect();
        if crib.is_empty() || self.offset + crib.len() > self.t.x.len() {
            return;
        }
        for (k, ch) in crib.iter().enumerate() {
            self.set_char(self.offset + k, row, Some(*ch));
        }
        host.sfx("select");
        self.check(host);
    }

    fn workbench(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        let n = self.t.x.len();
        let id = ui.id().with("ttp-workbench");
        let has_focus = ui.memory(|m| m.has_focus(id));
        if has_focus {
            ui.memory_mut(|m| m.set_focus_lock_filter(id, EventFilter { tab: true, horizontal_arrows: true, vertical_arrows: true, escape: false }));
        }
        // keyboard editing
        if has_focus && let Some((row, i)) = self.cursor {
            let events = ui.input(|inp| inp.events.clone());
            let mut cur = (row, i);
            for e in events {
                match e {
                    Event::Text(t) => {
                        for ch in t.chars() {
                            let ch = ch.to_ascii_uppercase();
                            if (' '..='~').contains(&ch) {
                                self.set_char(cur.1, cur.0, Some(ch));
                                cur.1 = (cur.1 + 1).min(n - 1);
                            }
                        }
                    }
                    Event::Key { key: Key::Backspace, pressed: true, .. } => {
                        let at = if self.cell(cur.0, cur.1).is_none() && cur.1 > 0 { cur.1 - 1 } else { cur.1 };
                        self.set_char(at, cur.0, None);
                        cur.1 = at;
                    }
                    Event::Key { key: Key::ArrowLeft, pressed: true, .. } => cur.1 = cur.1.saturating_sub(1),
                    Event::Key { key: Key::ArrowRight, pressed: true, .. } => cur.1 = (cur.1 + 1).min(n - 1),
                    Event::Key { key: Key::ArrowUp | Key::ArrowDown, pressed: true, .. } => cur.0 = 1 - cur.0,
                    _ => {}
                }
            }
            if cur != (row, i) {
                self.cursor = Some(cur);
                self.offset = cur.1.min(n.saturating_sub(self.crib.chars().count()));
            }
            self.check(host);
        }
        let per = 17;
        let cell = egui::vec2(22.0, 24.0);
        let mut clicked: Option<(u8, usize)> = None;
        egui::ScrollArea::horizontal().id_salt("ttp-cells").show(ui, |ui| {
            for start in (0..n).step_by(per) {
                let end = (start + per).min(n);
                ui.horizontal(|ui| {
                    ui.add_space(24.0);
                    for i in start..end {
                        let (r, _) = ui.allocate_exact_size(egui::vec2(cell.x, 14.0), Sense::hover());
                        ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, i.to_string(), mono(9.5), FAINT);
                    }
                });
                for row in 0..2u8 {
                    ui.horizontal(|ui| {
                        ui.add_sized([20.0, cell.y], egui::Label::new(RichText::new(if row == 0 { "A" } else { "B" }).font(mono(13.0)).color(DIM)));
                        for i in start..end {
                            let (r, resp) = ui.allocate_exact_size(cell, Sense::click());
                            let v = self.cell(row, i);
                            let selected = self.cursor == Some((row, i)) && has_focus;
                            let fill = if selected { theme::rgb(0x13243a) } else { theme::rgb(0x0a0f1d) };
                            let stroke = if selected { CYAN } else if v.is_some_and(|c| !plausible(c)) { RED } else { LINE };
                            ui.painter().rect_filled(r.shrink(1.0), 2.0, fill);
                            ui.painter().rect_stroke(r.shrink(1.0), 2.0, Stroke::new(1.0, stroke), egui::StrokeKind::Inside);
                            if let Some(c) = v {
                                let derived = self.typed[i].is_some_and(|t| t != row);
                                let color = if !plausible(c) {
                                    RED
                                } else if self.done {
                                    GREEN
                                } else if derived {
                                    AMBER
                                } else {
                                    theme::TEXT
                                };
                                let shown = if (' '..='~').contains(&c) { c.to_string() } else { "·".into() };
                                ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, shown, mono(14.0), color);
                            }
                            if resp.clicked() && !self.done {
                                clicked = Some((row, i));
                            }
                        }
                    });
                }
                ui.add_space(6.0);
            }
        });
        if let Some(c) = clicked {
            self.cursor = Some(c);
            self.offset = c.1.min(n.saturating_sub(self.crib.chars().count()));
            ui.memory_mut(|m| m.request_focus(id));
        }
        // keep our focus id alive
        let r = ui.interact(egui::Rect::from_min_size(ui.cursor().min, egui::vec2(1.0, 1.0)), id, Sense::focusable_noninteractive());
        let _ = r;
    }

    fn cell(&self, row: u8, i: usize) -> Option<char> {
        if row == 0 { self.a[i] } else { self.b[i] }
    }
}

impl Puzzle for TwoTimePad {
    fn id(&self) -> &'static str {
        "ch1.otp"
    }

    fn title(&self) -> &str {
        "RADIO LOG · PAD PAGE 17"
    }

    fn subtitle(&self) -> Option<&str> {
        Some("two messages, one pad")
    }

    fn accent(&self) -> Color32 {
        theme::GOLD
    }

    fn open(&mut self, host: &mut Host) {
        host.status("Mara's hint: who writes to whom? Names make good cribs.", Tone::Info);
    }

    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        let n = self.t.x.len();
        lede(ui, "A one-time pad is unbreakable, once. These two messages from Grandpa's radio log were encrypted with the **same** pad page. XOR the two ciphertexts and the pad disappears:");
        math(ui, "C₁ ⊕ C₂ = (P₁ ⊕ K) ⊕ (P₂ ⊕ K) = P₁ ⊕ P₂");
        let (c1, c2) = (to_hex(&self.t.c1), to_hex(&self.t.c2));
        columns2(ui, |ui| hex_box(ui, &c1, Some("C₁ (message A, intercepted)"), None, false), |ui| hex_box(ui, &c2, Some("C₂ (message B, intercepted)"), None, false));
        let mut place: Option<u8> = None;
        card(ui, Some("Crib dragging"), None, |ui| {
            note(ui, "Guess a word that probably appears in one message (a **crib**), slide it along, and read what it implies for the other message at the same spot. Real words mean you guessed right.");
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.crib).font(mono(15.0)).desired_width(180.0).char_limit(n));
                let len = self.crib.chars().count();
                let max = n.saturating_sub(len.max(1));
                self.offset = self.offset.min(max);
                ui.add(egui::Slider::new(&mut self.offset, 0..=max).show_value(false));
            });
            let crib = self.crib.to_uppercase();
            if !crib.is_empty() && self.offset + crib.chars().count() <= n {
                let p = partner(&self.t.x, &crib, self.offset);
                let score = plausibility(&p);
                let color = if score >= 1.0 {
                    GREEN
                } else if score > 0.7 {
                    AMBER
                } else {
                    RED
                };
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(format!("offset {}: crib", self.offset)).font(mono(13.0)).color(DIM));
                    ui.label(RichText::new(format!("“{crib}”")).font(mono(14.0)).color(AMBER));
                    ui.label(RichText::new("in one message ⇒").font(mono(13.0)).color(DIM));
                    ui.label(RichText::new(format!("“{p}”")).font(mono(14.0)).color(color));
                    ui.label(RichText::new("in the other").font(mono(13.0)).color(DIM));
                });
            }
            ui.horizontal_wrapped(|ui| {
                if button(ui, "Place crib in A here", Kind::Primary).clicked() {
                    place = Some(0);
                }
                if button(ui, "Place crib in B here", Kind::Normal).clicked() {
                    place = Some(1);
                }
                if button(ui, "Scan all positions", Kind::Normal).clicked() && !crib.is_empty() {
                    let mut hits = crib_drag(&self.t.x, &crib);
                    hits.sort_by(|p, q| q.score.partial_cmp(&p.score).unwrap());
                    self.scan = hits.into_iter().take(6).map(|h| (h.offset, h.partner_text, h.score)).collect();
                }
            });
            if !self.scan.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    for (off, text, score) in self.scan.clone() {
                        let hit = score >= 1.0;
                        let r = pill(ui, &format!("@{off}: “{text}”"), if hit { Color32::BLACK } else { theme::TEXT }, if hit { GREEN } else { theme::rgb(0x111830) }).interact(Sense::click()).on_hover_text("Jump to this offset");
                        if r.clicked() {
                            self.offset = off;
                        }
                    }
                });
            }
        });
        if let Some(row) = place {
            self.place_crib(row, host);
        }
        card(ui, Some("Workbench"), None, |ui| {
            note(ui, "Click a cell and type. Whatever you put in A forces the letter in B at the same position, and vice versa. Amber letters are derived; red ones are not plausible text, so a guess nearby is wrong.");
            ui.add_space(4.0);
            self.workbench(ui, host);
            let good = (0..n).filter(|&i| matches!((self.a[i], self.b[i]), (Some(x), Some(y)) if plausible(x) && plausible(y))).count();
            ui.label(RichText::new(format!("{good} / {n} positions read as text in both messages.")).font(mono(12.5)).color(DIM));
            if button(ui, "Clear workbench", Kind::Ghost).clicked() && !self.done {
                self.a.fill(None);
                self.b.fill(None);
                self.typed.fill(None);
            }
        });
        let _ = LINE2;
    }

    fn autoplay(&mut self, host: &mut Host) {
        self.crib = "NIGHTJAR".into();
        for (i, ch) in CH1_P1.chars().enumerate() {
            self.set_char(i, 0, Some(ch));
        }
        self.check(host);
    }
}
