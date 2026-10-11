//! Terminal windows host the puzzles: title bar, scrolling body, status line.

use egui::{Align, CornerRadius, Key, Layout, Margin, RichText, Stroke};

use super::theme::{self, AMBER, DIM, GREEN, LINE, LINE2, PANEL, RED, mono, retro};
use super::widgets::{Kind, Tone, button};
use crate::puzzle::{Host, Puzzle};

pub struct ActivePuzzle {
    pub reply: u64,
    pub puzzle: Box<dyn Puzzle>,
    pub host: Host,
    opened: bool,
}

impl ActivePuzzle {
    pub fn new(reply: u64, puzzle: Box<dyn Puzzle>, handle: &str) -> Self {
        Self { reply, puzzle, host: Host::new(handle), opened: false }
    }

    pub fn ensure_open(&mut self) {
        if !self.opened {
            self.opened = true;
            self.puzzle.open(&mut self.host);
        }
    }
}

/// Draw the terminal. Returns `Some(solved)` when the window closes.
pub fn show(ctx: &egui::Context, ap: &mut ActivePuzzle, dt: f32) -> Option<bool> {
    ap.ensure_open();
    ap.host.dt = dt;
    ap.host.time += dt as f64;
    let screen = ctx.content_rect();
    let width = (screen.width() - 24.0).min(ap.puzzle.size().width());
    let max_h = screen.height() - 40.0;
    let accent = ap.puzzle.accent();
    let mut result: Option<bool> = None;

    let esc = ctx.input(|i| i.key_pressed(Key::Escape));
    let was_solved = ap.host.solved();
    let frame = egui::Frame::NONE.fill(PANEL).stroke(Stroke::new(1.0, LINE2)).corner_radius(CornerRadius::same(6)).inner_margin(Margin::ZERO);
    egui::Modal::new(egui::Id::new(("terminal", ap.puzzle.id()))).frame(frame).backdrop_color(egui::Color32::from_black_alpha(150)).show(ctx, |ui| {
        ui.set_width(width);
        ui.set_max_height(max_h);
        // title bar
        egui::Frame::NONE.fill(theme::rgb(0x0e1424)).inner_margin(Margin::symmetric(12, 8)).show(ui, |ui| {
            ui.horizontal(|ui| {
                for c in [RED, AMBER, GREEN] {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                    ui.painter().circle_filled(r.center(), 4.5, c.gamma_multiply(0.85));
                }
                ui.add_space(6.0);
                ui.label(RichText::new(ap.puzzle.title()).font(retro(24.0)).color(accent));
                if let Some(s) = ap.puzzle.subtitle() {
                    ui.label(RichText::new(s).font(mono(12.0)).color(DIM));
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let solved = ap.host.solved();
                    if solved || ap.puzzle.closable() {
                        let label = if solved { "DONE ✓" } else { "ESC ✕" };
                        if button(ui, label, Kind::Ghost).clicked() {
                            result = Some(solved);
                        }
                    }
                });
            });
        });
        ui.painter().hline(ui.min_rect().x_range(), ui.cursor().top(), Stroke::new(1.0, accent.gamma_multiply(0.5)));
        // body
        let body_h = (max_h - 110.0).max(200.0);
        egui::Frame::NONE.inner_margin(Margin::symmetric(16, 12)).show(ui, |ui| {
            egui::ScrollArea::vertical().id_salt("terminal-body").max_height(body_h).auto_shrink([false, true]).show(ui, |ui| {
                ui.set_width((ui.available_width() - 6.0).max(0.0));
                ui.spacing_mut().item_spacing.y = 8.0;
                ap.puzzle.ui(ui, &mut ap.host);
            });
        });
        // status bar
        egui::Frame::NONE.fill(theme::rgb(0x0b101d)).stroke(Stroke::new(1.0, LINE)).inner_margin(Margin::symmetric(14, 8)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let solved = ap.host.solved();
                let w = ui.available_width() - if solved { 140.0 } else { 0.0 };
                ui.allocate_ui(egui::vec2(w.max(100.0), 0.0), |ui| {
                    let c = if ap.host.tone == Tone::None { DIM } else { ap.host.tone.color() };
                    ui.add(egui::Label::new(RichText::new(&ap.host.status).font(mono(13.5)).color(c)).wrap());
                });
                if solved {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if button(ui, "Continue ▸", Kind::Primary).clicked() {
                            result = Some(true);
                        }
                    });
                }
            });
        });
    });
    if esc && result.is_none() {
        if ap.host.solved() {
            result = Some(true);
        } else if ap.puzzle.closable() {
            ap.host.sfx("blip");
            result = Some(false);
        }
    }
    if was_solved && result.is_none() && ctx.input(|i| i.key_pressed(Key::Enter)) && !ctx.egui_wants_keyboard_input() {
        result = Some(true);
    }
    result
}
