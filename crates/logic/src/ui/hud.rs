//! Heads-up display: objective, chapter pips, menu buttons, the district
//! title card, the key hint, and toasts.

use egui::{Align2, Color32, CornerRadius, Margin, RichText, Stroke};

use super::theme::{self, AMBER, CYAN, DIM, GREEN, LINE2, PANEL2, RED, mono, retro};
use crate::state::{CHAPTER_LABELS, CHAPTER_NAMES, GameState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Codex,
    Item,
    Warn,
}

impl ToastKind {
    fn color(self) -> Color32 {
        match self {
            ToastKind::Info => CYAN,
            ToastKind::Codex => AMBER,
            ToastKind::Item => GREEN,
            ToastKind::Warn => RED,
        }
    }
}

pub struct Toast {
    pub title: String,
    pub body: String,
    pub kind: ToastKind,
    pub age: f32,
}

#[derive(Default)]
pub struct HudState {
    pub toasts: Vec<Toast>,
    /// (name, sub, age)
    pub location: Option<(String, String, f32)>,
    /// Contextual key hint, e.g. ("E", "Talk to Mara").
    pub hint: Option<(String, String)>,
    pub objective_flash: f32,
}

#[derive(Default)]
pub struct HudClicks {
    pub menu: bool,
    pub codex: bool,
}

impl HudState {
    pub fn toast(&mut self, title: &str, body: &str, kind: ToastKind) {
        self.toasts.push(Toast { title: title.into(), body: body.into(), kind, age: 0.0 });
        while self.toasts.len() > 4 {
            self.toasts.remove(0);
        }
    }

    pub fn show_location(&mut self, name: &str, sub: &str) {
        self.location = Some((name.into(), sub.into(), 0.0));
    }

    pub fn tick(&mut self, dt: f32) {
        for t in &mut self.toasts {
            t.age += dt;
        }
        self.toasts.retain(|t| t.age < 3.9);
        if let Some(l) = &mut self.location {
            l.2 += dt;
            if l.2 > 3.0 {
                self.location = None;
            }
        }
        self.objective_flash = (self.objective_flash - dt).max(0.0);
    }

    /// The in-world HUD (objective, pips, buttons, hint).
    pub fn show(&mut self, ctx: &egui::Context, state: &GameState) -> HudClicks {
        let mut clicks = HudClicks::default();
        // objective
        if !state.data.objective.is_empty() {
            egui::Area::new(egui::Id::new("hud-objective")).anchor(Align2::LEFT_TOP, egui::vec2(12.0, 12.0)).interactable(false).show(ctx, |ui| {
                let glow = self.objective_flash.min(1.0);
                let border = LINE2.lerp_to_gamma(CYAN, glow);
                egui::Frame::NONE.fill(theme::PANEL).stroke(Stroke::new(1.0, border)).corner_radius(CornerRadius::same(4)).inner_margin(Margin::symmetric(12, 8)).show(ui, |ui| {
                    ui.set_max_width(360.0);
                    ui.label(RichText::new("OBJECTIVE").font(retro(15.0)).color(CYAN));
                    ui.add(egui::Label::new(RichText::new(&state.data.objective).font(mono(13.5))).wrap());
                });
            });
        }
        // pips + buttons
        egui::Area::new(egui::Id::new("hud-right")).anchor(Align2::RIGHT_TOP, egui::vec2(-12.0, 12.0)).show(ctx, |ui| {
            ui.horizontal(|ui| {
                for (i, label) in CHAPTER_LABELS.iter().enumerate() {
                    let done = state.chapter_done(i);
                    let current = state.data.chapter as usize == i;
                    let (fill, fg, stroke) = if done {
                        (theme::rgb(0x0f3b2a), GREEN, GREEN.gamma_multiply(0.6))
                    } else if current {
                        (theme::rgb(0x0f3b44), CYAN, CYAN)
                    } else {
                        (theme::rgb(0x0d1220), DIM, LINE2)
                    };
                    egui::Frame::NONE
                        .fill(fill)
                        .stroke(Stroke::new(1.0, stroke))
                        .corner_radius(CornerRadius::same(2))
                        .inner_margin(Margin::symmetric(5, 0))
                        .show(ui, |ui| ui.label(RichText::new(*label).font(retro(17.0)).color(fg)))
                        .response
                        .on_hover_text(CHAPTER_NAMES[i]);
                }
                ui.add_space(6.0);
                if ui.add(egui::Button::new(RichText::new("CODEX").font(retro(18.0)).color(AMBER)).fill(theme::PANEL)).on_hover_text("Codex (C)").clicked() {
                    clicks.codex = true;
                }
                if ui.add(egui::Button::new(RichText::new("MENU").font(retro(18.0)).color(CYAN)).fill(theme::PANEL)).on_hover_text("Menu (Esc)").clicked() {
                    clicks.menu = true;
                }
            });
        });
        // key hint
        if let Some((key, label)) = &self.hint {
            egui::Area::new(egui::Id::new("hud-hint")).anchor(Align2::CENTER_BOTTOM, egui::vec2(0.0, -22.0)).interactable(false).show(ctx, |ui| {
                egui::Frame::NONE.fill(theme::PANEL).stroke(Stroke::new(1.0, LINE2)).corner_radius(CornerRadius::same(4)).inner_margin(Margin::symmetric(10, 5)).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        egui::Frame::NONE.fill(theme::rgb(0xe6f6ff)).corner_radius(CornerRadius::same(2)).inner_margin(Margin::symmetric(5, 0)).show(ui, |ui| {
                            ui.label(RichText::new(key).font(retro(17.0)).color(theme::rgb(0x0e0b16)));
                        });
                        ui.label(RichText::new(label).font(mono(13.5)));
                    });
                });
            });
        }
        clicks
    }

    /// The district title card and the toasts (also shown during cutscenes).
    pub fn show_overlays(&self, ctx: &egui::Context) {
        if let Some((name, sub, age)) = &self.location {
            let a = if *age < 0.3 { age / 0.3 } else if *age > 2.6 { (3.0 - age) / 0.4 } else { 1.0 }.clamp(0.0, 1.0);
            egui::Area::new(egui::Id::new("hud-location")).anchor(Align2::CENTER_TOP, egui::vec2(0.0, 70.0)).interactable(false).show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(name).font(retro(46.0)).color(CYAN.gamma_multiply(a)));
                    ui.label(RichText::new(sub).font(mono(14.0)).color(DIM.gamma_multiply(a)));
                });
            });
        }
        egui::Area::new(egui::Id::new("hud-toasts")).anchor(Align2::RIGHT_TOP, egui::vec2(-12.0, 52.0)).order(egui::Order::Tooltip).interactable(false).show(ctx, |ui| {
            for t in &self.toasts {
                let a = if t.age < 0.25 { t.age / 0.25 } else if t.age > 3.6 { (3.9 - t.age) / 0.3 } else { 1.0 }.clamp(0.0, 1.0);
                let c = t.kind.color();
                let frame = egui::Frame::NONE.fill(PANEL2.gamma_multiply(a)).stroke(Stroke::new(1.0, LINE2.gamma_multiply(a))).corner_radius(CornerRadius::same(4)).inner_margin(Margin { left: 14, right: 14, top: 7, bottom: 7 });
                let r = frame.show(ui, |ui| {
                    ui.set_max_width(400.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&t.title).font(retro(18.0)).color(c.gamma_multiply(a)));
                        ui.add(egui::Label::new(RichText::new(&t.body).font(mono(13.5)).color(theme::TEXT.gamma_multiply(a))).wrap());
                    });
                });
                let rect = r.response.rect;
                ui.painter().rect_filled(egui::Rect::from_min_size(rect.min, egui::vec2(3.0, rect.height())), 0.0, c.gamma_multiply(a));
                ui.add_space(4.0);
            }
        });
    }
}
