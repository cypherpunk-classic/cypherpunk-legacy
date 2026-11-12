//! Building blocks shared by the puzzle terminals.

use egui::{Color32, CornerRadius, Margin, RichText, Stroke, Ui};

use super::markup::{self, Look};
use super::theme::{self, AMBER, CYAN, DIM, FAINT, GREEN, LINE, RED, TEXT, VIOLET, mono, retro};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Tone {
    #[default]
    None,
    Ok,
    Bad,
    Warn,
    Info,
    Dim,
}

impl Tone {
    pub fn color(self) -> Color32 {
        match self {
            Tone::None => TEXT,
            Tone::Ok => GREEN,
            Tone::Bad => RED,
            Tone::Warn => AMBER,
            Tone::Info => CYAN,
            Tone::Dim => DIM,
        }
    }
}

/// A titled box. `accent` tints the border.
pub fn card<R>(ui: &mut Ui, title: Option<&str>, accent: Option<Color32>, add: impl FnOnce(&mut Ui) -> R) -> R {
    theme::card_frame(accent)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if let Some(t) = title {
                ui.label(RichText::new(t.to_uppercase()).font(retro(19.0)).color(AMBER));
                ui.add_space(2.0);
            }
            add(ui)
        })
        .inner
}

/// Two cards side by side on wide screens, stacked on narrow ones.
pub fn columns2(ui: &mut Ui, left: impl FnOnce(&mut Ui), right: impl FnOnce(&mut Ui)) {
    if ui.available_width() > 640.0 {
        ui.columns(2, |cols| {
            left(&mut cols[0]);
            right(&mut cols[1]);
        });
    } else {
        left(ui);
        right(ui);
    }
}

/// Intro paragraph.
pub fn lede(ui: &mut Ui, text: &str) {
    markup::rich(ui, text, Look { size: 14.5, color: theme::rgb(0xc3cee2), family: None });
}

/// Smaller explanatory text.
pub fn note(ui: &mut Ui, text: &str) {
    markup::rich(ui, text, Look { size: 13.0, color: DIM, family: None });
}

/// A formula or identity, in a highlighted monospace line.
pub fn math(ui: &mut Ui, text: &str) {
    egui::Frame::NONE.fill(theme::rgb(0x0a0f1d)).stroke(Stroke::new(1.0, theme::rgb(0x1b2440))).corner_radius(CornerRadius::same(2)).inner_margin(Margin::symmetric(8, 4)).show(ui, |ui| {
        markup::rich(ui, text, Look { size: 13.5, color: theme::rgb(0xbff8ff), family: None });
    });
}

/// Inline coloured monospace text.
pub fn mono_text(ui: &mut Ui, text: &str, color: Color32, size: f32) -> egui::Response {
    ui.add(egui::Label::new(RichText::new(text).font(mono(size)).color(color)).wrap())
}

/// A hex value in a box with a copy button.
pub fn hex_box(ui: &mut Ui, value: &str, label: Option<&str>, tone: Option<Color32>, big: bool) {
    if let Some(l) = label {
        ui.label(RichText::new(l).font(mono(12.0)).color(DIM));
    }
    egui::Frame::NONE.fill(theme::rgb(0x070b16)).stroke(Stroke::new(1.0, LINE)).corner_radius(CornerRadius::same(4)).inner_margin(Margin::symmetric(8, 6)).show(ui, |ui| {
        ui.horizontal_top(|ui| {
            let w = (ui.available_width() - 56.0).max(60.0);
            ui.allocate_ui(egui::vec2(w, 0.0), |ui| {
                ui.set_width(w);
                let color = tone.unwrap_or(theme::rgb(0xbff8ff));
                ui.add(egui::Label::new(RichText::new(value).font(mono(if big { 15.0 } else { 12.5 })).color(color)).wrap());
            });
            if ui.small_button("copy").on_hover_text("Copy to clipboard").clicked() {
                ui.ctx().copy_text(value.to_string());
            }
        });
    });
}

/// Key/value rows.
pub fn kv(ui: &mut Ui, id: &str, rows: &[(&str, String)]) {
    egui::Grid::new(id).num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
        for (k, v) in rows {
            ui.label(RichText::new(*k).font(mono(12.5)).color(DIM));
            ui.add(egui::Label::new(RichText::new(v).font(mono(13.0)).color(TEXT)).wrap());
            ui.end_row();
        }
    });
}

/// A small coloured badge.
pub fn tag(ui: &mut Ui, text: &str, tone: Tone) -> egui::Response {
    let c = if tone == Tone::None { VIOLET } else { tone.color() };
    egui::Frame::NONE
        .fill(c.gamma_multiply(0.14))
        .stroke(Stroke::new(1.0, c.gamma_multiply(0.7)))
        .corner_radius(CornerRadius::same(2))
        .inner_margin(Margin::symmetric(6, 1))
        .show(ui, |ui| ui.label(RichText::new(text).font(mono(11.5)).color(c)))
        .response
}

/// Bits in groups, with optional per-bit highlight colours.
pub fn bits_view(ui: &mut Ui, bits: &str, group: usize, mark: impl Fn(usize) -> Option<Color32>) {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = ui.available_width();
    for (i, b) in bits.chars().enumerate() {
        let base = if b == '1' { theme::rgb(0xbff8ff) } else { FAINT };
        let color = mark(i).unwrap_or(base);
        job.append(&b.to_string(), 0.0, egui::TextFormat { font_id: mono(12.5), color, ..Default::default() });
        if (i + 1) % group == 0 {
            job.append(" ", 0.0, egui::TextFormat { font_id: mono(12.5), ..Default::default() });
        }
    }
    ui.label(job);
}

/// A scrolling log of tagged lines.
#[derive(Default, Clone)]
pub struct Log {
    pub lines: Vec<(String, Tone)>,
}

impl Log {
    pub fn line(&mut self, text: impl Into<String>, tone: Tone) {
        self.lines.push((text.into(), tone));
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }

    pub fn show(&self, ui: &mut Ui, id: &str, max_height: f32) {
        egui::Frame::NONE.fill(theme::rgb(0x070b16)).stroke(Stroke::new(1.0, LINE)).corner_radius(CornerRadius::same(3)).inner_margin(Margin::same(6)).show(ui, |ui| {
            egui::ScrollArea::vertical().id_salt(id).max_height(max_height).stick_to_bottom(true).auto_shrink([false, true]).show(ui, |ui| {
                ui.set_width(ui.available_width());
                for (t, tone) in &self.lines {
                    let c = if *tone == Tone::None { theme::rgb(0xb8c4da) } else { tone.color() };
                    ui.add(egui::Label::new(RichText::new(t).font(mono(12.5)).color(c)).wrap());
                }
            });
        });
    }
}

/// A thin progress bar.
pub fn meter(ui: &mut Ui, frac: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 8.0), egui::Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 2.0, theme::rgb(0x111830));
    let mut fill = rect;
    fill.set_width(rect.width() * frac.clamp(0.0, 1.0));
    p.rect_filled(fill, 2.0, color);
}

pub fn shorten(hex: &str, head: usize, tail: usize) -> String {
    if hex.len() <= head + tail + 1 { hex.to_string() } else { format!("{}…{}", &hex[..head], &hex[hex.len() - tail..]) }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Normal,
    Primary,
    Danger,
    Ghost,
}

/// A themed button.
pub fn button(ui: &mut Ui, label: &str, kind: Kind) -> egui::Response {
    let (fill, text, stroke) = match kind {
        Kind::Primary => (theme::rgb(0x0f3b44), CYAN, CYAN),
        Kind::Danger => (theme::rgb(0x3a121f), RED, RED),
        Kind::Ghost => (Color32::TRANSPARENT, DIM, LINE),
        Kind::Normal => (theme::rgb(0x151c30), TEXT, theme::LINE2),
    };
    ui.add(egui::Button::new(RichText::new(label).font(mono(14.0)).color(text)).fill(fill).stroke(Stroke::new(1.0, stroke)))
}

pub fn button_enabled(ui: &mut Ui, enabled: bool, label: &str, kind: Kind) -> egui::Response {
    ui.add_enabled_ui(enabled, |ui| button(ui, label, kind)).inner
}

/// A single-line text field.
pub fn text_field(ui: &mut Ui, value: &mut String, hint: &str, width: f32) -> egui::Response {
    ui.add(egui::TextEdit::singleline(value).hint_text(hint).font(mono(14.0)).desired_width(width).text_color(TEXT))
}

/// A heading inside a card.
pub fn sub(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).font(retro(18.0)).color(AMBER));
}

/// A pill (word chip).
pub fn pill(ui: &mut Ui, text: &str, fg: Color32, bg: Color32) -> egui::Response {
    egui::Frame::NONE
        .fill(bg)
        .stroke(Stroke::new(1.0, fg.gamma_multiply(0.5)))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(8, 2))
        .show(ui, |ui| ui.label(RichText::new(text).font(mono(13.0)).color(fg)))
        .response
}

// ---------------------------------------------------------------------------
// Hands-on building blocks (v3.1): step lists, formulas built from tokens,
// selectable cards, character diffs, bit toggles, deep-dive sheets.
// ---------------------------------------------------------------------------

/// The step list beside a puzzle: done steps ticked, the current one lit, its
/// hint shown in Guided mode.
pub fn step_list(ui: &mut Ui, steps: &[crate::puzzle::StepView], hints: bool) {
    let current = steps.iter().position(|s| !s.done);
    for (i, s) in steps.iter().enumerate() {
        let (mark, color) = if s.done {
            ("✓", GREEN)
        } else if Some(i) == current {
            ("▸", CYAN)
        } else {
            ("·", FAINT)
        };
        ui.horizontal_top(|ui| {
            ui.label(RichText::new(mark).font(mono(13.0)).color(color));
            ui.add(egui::Label::new(RichText::new(&s.label).font(mono(12.5)).color(if s.done { DIM } else if Some(i) == current { TEXT } else { FAINT })).wrap());
        });
        if hints && Some(i) == current && !s.hint.is_empty() {
            ui.horizontal_top(|ui| {
                ui.add_space(14.0);
                ui.add(egui::Label::new(RichText::new(&s.hint).font(mono(11.5)).color(AMBER.gamma_multiply(0.85))).wrap());
            });
        }
        ui.add_space(2.0);
    }
}

/// One piece of a formula: fixed text or a slot for a token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Text(&'static str),
    Slot,
}

/// The player's progress on a formula: which token sits in each slot, and
/// which slot the next token goes into.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Formula {
    pub slots: Vec<Option<usize>>,
    pub focus: usize,
}

impl Formula {
    pub fn new(parts: &[Part]) -> Self {
        Self { slots: vec![None; parts.iter().filter(|p| **p == Part::Slot).count()], focus: 0 }
    }

    pub fn complete(&self) -> bool {
        self.slots.iter().all(Option::is_some)
    }

    /// Put tokens into the slots in order (autoplay).
    pub fn set(&mut self, tokens: &[usize]) {
        for (s, t) in self.slots.iter_mut().zip(tokens) {
            *s = Some(*t);
        }
        self.focus = self.slots.len();
    }

    pub fn clear(&mut self) {
        self.slots.iter_mut().for_each(|s| *s = None);
        self.focus = 0;
    }

    pub fn tokens(&self) -> Vec<Option<usize>> {
        self.slots.clone()
    }

    /// Does the formula hold exactly these tokens, in order?
    pub fn is(&self, tokens: &[usize]) -> bool {
        self.slots.len() == tokens.len() && self.slots.iter().zip(tokens).all(|(s, t)| *s == Some(*t))
    }
}

/// Draw a formula with clickable slots and a palette of tokens beneath it.
/// Click a token to drop it into the focused slot; click a slot to focus it
/// (or to empty it, if it is already focused). Returns true if it changed.
pub fn formula(ui: &mut Ui, parts: &[Part], palette: &[&str], f: &mut Formula) -> bool {
    let mut changed = false;
    if f.slots.len() != parts.iter().filter(|p| **p == Part::Slot).count() {
        *f = Formula::new(parts);
    }
    egui::Frame::NONE.fill(theme::rgb(0x0a0f1d)).stroke(Stroke::new(1.0, theme::rgb(0x1b2440))).corner_radius(CornerRadius::same(3)).inner_margin(Margin::symmetric(8, 6)).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            let mut slot = 0;
            for p in parts {
                match p {
                    Part::Text(t) => {
                        ui.label(RichText::new(*t).font(mono(15.0)).color(theme::rgb(0xbff8ff)));
                    }
                    Part::Slot => {
                        let i = slot;
                        slot += 1;
                        let focused = f.focus == i;
                        let (label, color) = match f.slots[i] {
                            Some(t) => (palette.get(t).copied().unwrap_or("?").to_string(), GOLD_TOKEN),
                            None => ("  □  ".to_string(), FAINT),
                        };
                        let b = egui::Button::new(RichText::new(label).font(mono(15.0)).color(color))
                            .fill(if focused { theme::rgb(0x13243a) } else { theme::rgb(0x0d1424) })
                            .stroke(Stroke::new(1.0, if focused { CYAN } else { LINE }));
                        if ui.add(b).clicked() {
                            if focused && f.slots[i].is_some() {
                                f.slots[i] = None;
                                changed = true;
                            }
                            f.focus = i;
                        }
                    }
                }
            }
        });
    });
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("tokens:").font(mono(12.0)).color(DIM));
        for (t, label) in palette.iter().enumerate() {
            let b = egui::Button::new(RichText::new(*label).font(mono(14.0)).color(GOLD_TOKEN)).fill(theme::rgb(0x1a1a2a)).stroke(Stroke::new(1.0, GOLD_TOKEN.gamma_multiply(0.5)));
            if ui.add(b).clicked() && f.focus < f.slots.len() {
                f.slots[f.focus] = Some(t);
                changed = true;
                // move to the next empty slot
                f.focus = (0..f.slots.len()).find(|j| f.slots[*j].is_none()).unwrap_or(f.slots.len());
            }
        }
        if ui.small_button("clear").clicked() {
            f.clear();
            changed = true;
        }
    });
    changed
}

const GOLD_TOKEN: Color32 = theme::rgb(0xffcd75);

/// A card the player can select. Returns true when clicked.
pub fn select_card(ui: &mut Ui, selected: bool, accent: Color32, add: impl FnOnce(&mut Ui)) -> bool {
    let stroke = if selected { Stroke::new(2.0, accent) } else { Stroke::new(1.0, LINE) };
    let r = egui::Frame::NONE
        .fill(if selected { accent.gamma_multiply(0.10) } else { theme::rgb(0x0d1322) })
        .stroke(stroke)
        .corner_radius(CornerRadius::same(4))
        .inner_margin(Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui);
        });
    r.response.interact(egui::Sense::click()).clicked()
}

/// Show `b` with every character that differs from `a` in red (and the rest in green).
pub fn hex_diff(ui: &mut Ui, a: &str, b: &str) {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = ui.available_width();
    let ac: Vec<char> = a.chars().collect();
    for (i, c) in b.chars().enumerate() {
        let color = if ac.get(i) == Some(&c) { GREEN } else { RED };
        job.append(&c.to_string(), 0.0, egui::TextFormat { font_id: mono(13.0), color, ..Default::default() });
    }
    ui.label(job);
}

/// Eight clickable bits (most significant first). Returns true if one was toggled.
pub fn bit_toggles(ui: &mut Ui, value: &mut u8) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        for bit in (0..8).rev() {
            let on = (*value >> bit) & 1 == 1;
            let b = egui::Button::new(RichText::new(if on { "1" } else { "0" }).font(mono(14.0)).color(if on { theme::rgb(0x0b0d16) } else { DIM }))
                .fill(if on { CYAN } else { theme::rgb(0x0d1424) })
                .stroke(Stroke::new(1.0, if on { CYAN } else { LINE }))
                .min_size(egui::vec2(22.0, 22.0));
            if ui.add(b).clicked() {
                *value ^= 1 << bit;
                changed = true;
            }
        }
    });
    changed
}

/// A countdown bar with the seconds left.
pub fn countdown(ui: &mut Ui, left: f32, total: f32) {
    ui.horizontal(|ui| {
        let c = if left < total * 0.2 { RED } else if left < total * 0.5 { AMBER } else { CYAN };
        ui.label(RichText::new(format!("{:>3.0} s", left.max(0.0))).font(mono(13.0)).color(c));
        meter(ui, left / total.max(1.0), c);
    });
}

/// Draw a deep-dive sheet: one row per cell, a number to type, a tick when right.
pub fn hand_sheet(ui: &mut Ui, sheet: &mut crate::puzzle::HandSheet, hints: bool) {
    lede(ui, &sheet.intro);
    ui.add_space(6.0);
    for (i, cell) in sheet.cells.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            let label_w = (ui.available_width() - 160.0).max(220.0);
            ui.allocate_ui_with_layout(egui::vec2(label_w, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                ui.set_width(label_w);
                ui.add(egui::Label::new(RichText::new(&cell.label).font(mono(13.5)).color(TEXT)).wrap());
                if hints && !cell.hint.is_empty() {
                    ui.add(egui::Label::new(RichText::new(&cell.hint).font(mono(11.5)).color(DIM)).wrap());
                }
            });
            ui.add(egui::TextEdit::singleline(&mut cell.input).id_salt(("hand", i)).font(mono(14.0)).desired_width(110.0));
            let (mark, c) = if cell.input.trim().is_empty() {
                ("", DIM)
            } else if cell.ok() {
                ("✓", GREEN)
            } else {
                ("✕", RED)
            };
            ui.label(RichText::new(mark).font(mono(15.0)).color(c));
        });
        ui.add_space(6.0);
    }
}
