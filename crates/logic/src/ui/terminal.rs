//! Terminal windows host the puzzles: title bar, the step list, a scrolling
//! body, the status line, and (once solved) the optional deep dive.

use egui::{Align, CornerRadius, Key, Layout, Margin, RichText, Stroke};

use super::theme::{self, AMBER, DIM, GREEN, LINE, LINE2, PANEL, RED, VIOLET, mono, retro};
use super::widgets::{Kind, Tone, button, countdown, hand_sheet, step_list};
use crate::puzzle::{HandSheet, Host, Puzzle};

pub struct ActivePuzzle {
    pub reply: u64,
    pub puzzle: Box<dyn Puzzle>,
    pub host: Host,
    opened: bool,
    /// The deep dive in progress: the sheet and the seconds left.
    pub deep: Option<(HandSheet, f32)>,
    /// A deep dive may be offered (not yet earned in this save).
    pub deep_available: bool,
    /// Whether the puzzle has a deep dive (asked once, after solving).
    has_deep: Option<bool>,
}

impl ActivePuzzle {
    pub fn new(reply: u64, puzzle: Box<dyn Puzzle>, handle: &str, hints: bool, deep_available: bool) -> Self {
        let mut host = Host::new(handle);
        host.hints = hints;
        Self { reply, puzzle, host, opened: false, deep: None, deep_available, has_deep: None }
    }

    pub fn ensure_open(&mut self) {
        if !self.opened {
            self.opened = true;
            self.puzzle.open(&mut self.host);
        }
    }

    /// Start the deep dive, if the puzzle has one.
    pub fn start_deep(&mut self) -> bool {
        match self.puzzle.deep_sheet() {
            Some(s) => {
                let t = s.seconds;
                self.deep = Some((s, t));
                self.host.status("Deep dive: work it by hand before the clock runs out.", Tone::Info);
                true
            }
            None => false,
        }
    }
}

/// Draw the terminal. Returns `Some(solved)` when the window closes.
pub fn show(ctx: &egui::Context, ap: &mut ActivePuzzle, dt: f32) -> Option<bool> {
    ap.ensure_open();
    ap.host.dt = dt;
    ap.host.time += dt as f64;
    let screen = ctx.content_rect();
    let steps = ap.puzzle.steps();
    let side = !steps.is_empty() && screen.width() > 900.0;
    let extra = if side { 210.0 } else { 0.0 };
    let width = (screen.width() - 24.0).min(ap.puzzle.size().width() + extra);
    let max_h = screen.height() - 40.0;
    let accent = ap.puzzle.accent();
    let mut result: Option<bool> = None;
    let mut deep_clicked = false;

    // the deep dive's clock
    if let Some((sheet, left)) = ap.deep.as_mut() {
        *left -= dt;
        if sheet.done() {
            ap.deep = None;
            ap.deep_available = false;
            ap.host.deep_done = true;
            ap.host.sfx("unlock");
            ap.host.status("Deep dive complete. Badge earned, and the Commons pays 4 credits for it.", Tone::Ok);
        } else if *left <= 0.0 {
            for c in &mut sheet.cells {
                c.input.clear();
            }
            *left = sheet.seconds;
            ap.host.sfx("error");
            ap.host.status("Out of time. The numbers are the same; try again.", Tone::Warn);
        }
    }

    if ap.host.solved() && ap.has_deep.is_none() {
        ap.has_deep = Some(ap.deep_available && ap.puzzle.deep_sheet().is_some());
    }
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
            let hints = ap.host.hints;
            let draw_body = |ui: &mut egui::Ui, ap: &mut ActivePuzzle| {
                egui::ScrollArea::vertical().id_salt("terminal-body").max_height(body_h).auto_shrink([false, true]).show(ui, |ui| {
                    ui.set_width((ui.available_width() - 6.0).max(0.0));
                    ui.spacing_mut().item_spacing.y = 8.0;
                    if let Some((sheet, left)) = ap.deep.as_mut() {
                        ui.label(RichText::new(format!("DEEP DIVE · {}", sheet.title)).font(retro(20.0)).color(VIOLET));
                        countdown(ui, *left, sheet.seconds);
                        hand_sheet(ui, sheet, hints);
                    } else {
                        ap.puzzle.ui(ui, &mut ap.host);
                    }
                });
            };
            if side && ap.deep.is_none() {
                ui.horizontal_top(|ui| {
                    ui.allocate_ui_with_layout(egui::vec2(196.0, body_h), Layout::top_down(Align::LEFT), |ui| {
                        ui.set_width(196.0);
                        egui::Frame::NONE.fill(theme::rgb(0x0a0f1c)).stroke(Stroke::new(1.0, LINE)).corner_radius(CornerRadius::same(4)).inner_margin(Margin::same(8)).show(ui, |ui| {
                            ui.set_width(180.0);
                            ui.label(RichText::new("STEPS").font(retro(16.0)).color(AMBER));
                            step_list(ui, &steps, hints);
                        });
                    });
                    ui.add_space(6.0);
                    ui.vertical(|ui| draw_body(ui, ap));
                });
            } else {
                if !steps.is_empty() && ap.deep.is_none() {
                    egui::CollapsingHeader::new(RichText::new("STEPS").font(retro(16.0)).color(AMBER)).id_salt("steps").default_open(true).show(ui, |ui| step_list(ui, &steps, hints));
                }
                draw_body(ui, ap);
            }
        });
        // status bar
        egui::Frame::NONE.fill(theme::rgb(0x0b101d)).stroke(Stroke::new(1.0, LINE)).inner_margin(Margin::symmetric(14, 8)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let solved = ap.host.solved();
                let offer_deep = solved && ap.deep.is_none() && ap.deep_available && ap.has_deep == Some(true);
                let w = ui.available_width() - if solved { 140.0 } else { 0.0 } - if offer_deep { 140.0 } else { 0.0 };
                ui.allocate_ui(egui::vec2(w.max(100.0), 0.0), |ui| {
                    let c = if ap.host.tone == Tone::None { DIM } else { ap.host.tone.color() };
                    ui.add(egui::Label::new(RichText::new(&ap.host.status).font(mono(13.5)).color(c)).wrap());
                });
                if solved {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if button(ui, "Continue ▸", Kind::Primary).clicked() {
                            result = Some(true);
                        }
                        if offer_deep && button(ui, "Deep dive ◆", Kind::Normal).on_hover_text("An optional harder version, worked by hand. Pays credits and a badge.").clicked() {
                            deep_clicked = true;
                        }
                    });
                }
            });
        });
    });
    if deep_clicked {
        ap.start_deep();
    }
    if esc && result.is_none() {
        if ap.deep.is_some() {
            ap.deep = None;
            ap.host.status("Deep dive abandoned. It stays open for later.", Tone::Dim);
        } else if ap.host.solved() {
            result = Some(true);
        } else if ap.puzzle.closable() {
            ap.host.sfx("blip");
            result = Some(false);
        }
    }
    if was_solved && ap.deep.is_none() && result.is_none() && ctx.input(|i| i.key_pressed(Key::Enter)) && !ctx.egui_wants_keyboard_input() {
        result = Some(true);
    }
    result
}
