//! Pause menu: Codex, Keyring, Settings, Game.

use egui::{Align, CornerRadius, Key, Layout, Margin, RichText, Stroke};

use super::markup::{self, Look};
use super::theme::{self, AMBER, CYAN, DIM, FAINT, LINE, LINE2, PANEL, RED, mono, retro};
use super::widgets::{Kind, button, card, kv, note};
use crate::codex::{CODEX, CODEX_GROUPS, codex_by_id};
use crate::items::{item_def, item_details};
use crate::state::{GameState, Settings};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Codex,
    Keyring,
    Settings,
    Game,
}

pub struct MenuState {
    pub tab: Tab,
    pub codex_sel: Option<String>,
    pub save_box: String,
    pub storage_ok: bool,
    /// Frames shown (the Esc that opened the menu must not also close it).
    pub frames: u32,
}

impl MenuState {
    pub fn new(tab: Tab, focus: Option<&str>, state: &GameState) -> Self {
        let unlocked: Vec<&str> = state.data.codex.iter().map(|s| s.as_str()).collect();
        let sel = focus.filter(|f| unlocked.contains(f)).map(String::from).or_else(|| CODEX.iter().find(|e| unlocked.contains(&e.id)).map(|e| e.id.to_string()));
        Self { tab, codex_sel: sel, save_box: String::new(), storage_ok: true, frames: 0 }
    }
}

/// What the player asked for from the menu.
#[derive(Debug, PartialEq)]
pub enum MenuAction {
    None,
    Close,
    SaveNow,
    CopySave,
    Import(String),
    QuitToTitle,
    Settings(Settings),
}

fn format_time(ms: f64) -> String {
    let m = (ms / 60000.0).floor() as u64;
    if m < 60 { format!("{m} min") } else { format!("{} h {} min", m / 60, m % 60) }
}

/// Codex browser (also used on the About page).
pub fn codex_view(ui: &mut egui::Ui, state: &GameState, sel: &mut Option<String>, sfx: &mut Vec<&'static str>, height: f32) {
    let unlocked: Vec<&str> = state.data.codex.iter().map(|s| s.as_str()).collect();
    ui.horizontal_top(|ui| {
        ui.allocate_ui(egui::vec2(230.0, height), |ui| {
            egui::ScrollArea::vertical().id_salt("codex-list").max_height(height).auto_shrink([false, false]).show(ui, |ui| {
                ui.set_width(220.0);
                for g in CODEX_GROUPS {
                    ui.add_space(4.0);
                    ui.label(RichText::new(g.label().to_uppercase()).font(retro(16.0)).color(AMBER));
                    for e in CODEX.iter().filter(|e| e.group == g) {
                        let open = unlocked.contains(&e.id);
                        let selected = sel.as_deref() == Some(e.id);
                        let text = if open { e.title } else { "· · · · · ·" };
                        let color = if selected { CYAN } else if open { theme::TEXT } else { FAINT };
                        let r = ui.add_enabled(open, egui::Button::new(RichText::new(text).font(mono(12.5)).color(color)).fill(if selected { theme::rgb(0x13243a) } else { egui::Color32::TRANSPARENT }).stroke(Stroke::NONE).min_size(egui::vec2(212.0, 22.0)));
                        if r.clicked() {
                            *sel = Some(e.id.to_string());
                            sfx.push("blip");
                        }
                    }
                }
            });
        });
        ui.separator();
        ui.vertical(|ui| {
            egui::ScrollArea::vertical().id_salt("codex-entry").max_height(height).auto_shrink([false, false]).show(ui, |ui| {
                ui.set_width((ui.available_width() - 8.0).max(160.0));
                match sel.as_deref().and_then(codex_by_id) {
                    Some(e) => {
                        ui.label(RichText::new(e.title).font(retro(28.0)).color(CYAN));
                        ui.label(RichText::new(format!("{} · {}", e.group.label().to_uppercase(), e.era)).font(mono(12.0)).color(DIM));
                        ui.add_space(6.0);
                        for p in e.body.split("\n\n") {
                            markup::rich(ui, p.trim(), Look { size: 14.0, color: theme::TEXT, family: None });
                            ui.add_space(6.0);
                        }
                        if let Some(ig) = e.ingame {
                            egui::Frame::NONE.fill(theme::rgb(0x161226)).stroke(Stroke::new(1.0, theme::VIOLET.gamma_multiply(0.5))).corner_radius(CornerRadius::same(3)).inner_margin(Margin::symmetric(10, 6)).show(ui, |ui| {
                                markup::rich(ui, ig, Look { size: 13.0, color: theme::VIOLET, family: Some(theme::italic_family()) });
                            });
                        }
                    }
                    None => note(ui, "Nothing here yet. Explore, read, and solve: entries unlock as you go."),
                }
            });
        });
    });
    ui.label(RichText::new(format!("{} / {} entries", unlocked.len(), CODEX.len())).font(mono(12.0)).color(DIM));
}

/// Draw the menu window; returns the action requested this frame.
pub fn show(ctx: &egui::Context, m: &mut MenuState, state: &GameState, sfx: &mut Vec<&'static str>) -> MenuAction {
    let mut action = MenuAction::None;
    let screen = ctx.content_rect();
    let width = (screen.width() - 24.0).min(860.0);
    let height = (screen.height() - 220.0).clamp(220.0, 560.0);
    let frame = egui::Frame::NONE.fill(PANEL).stroke(Stroke::new(1.0, LINE2)).corner_radius(CornerRadius::same(6)).inner_margin(Margin::same(0));
    let modal = egui::Modal::new(egui::Id::new("menu")).frame(frame).backdrop_color(egui::Color32::from_black_alpha(150)).show(ctx, |ui| {
        ui.set_width(width);
        egui::Frame::NONE.fill(theme::rgb(0x0e1424)).inner_margin(Margin::symmetric(12, 8)).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("MENU").font(retro(24.0)).color(CYAN));
                ui.label(RichText::new(format!("{} · {}", state.handle(), format_time(state.data.play_ms))).font(mono(12.0)).color(DIM));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if button(ui, "ESC ✕", Kind::Ghost).clicked() {
                        action = MenuAction::Close;
                    }
                });
            });
        });
        egui::Frame::NONE.inner_margin(Margin::symmetric(14, 6)).show(ui, |ui| {
            ui.horizontal(|ui| {
                for (t, label) in [(Tab::Codex, "CODEX"), (Tab::Keyring, "KEYRING"), (Tab::Settings, "SETTINGS"), (Tab::Game, "GAME")] {
                    let on = m.tab == t;
                    let b = egui::Button::new(RichText::new(label).font(retro(18.0)).color(if on { CYAN } else { DIM })).fill(if on { theme::rgb(0x13243a) } else { egui::Color32::TRANSPARENT }).stroke(Stroke::new(1.0, if on { CYAN } else { LINE }));
                    if ui.add(b).clicked() && !on {
                        m.tab = t;
                        sfx.push("blip");
                    }
                }
            });
        });
        egui::Frame::NONE.inner_margin(Margin { left: 16, right: 16, top: 4, bottom: 16 }).show(ui, |ui| match m.tab {
            Tab::Codex => codex_view(ui, state, &mut m.codex_sel, sfx, height),
            Tab::Keyring => {
                egui::ScrollArea::vertical().id_salt("keyring").max_height(height).auto_shrink([false, true]).show(ui, |ui| {
                    if state.data.items.is_empty() {
                        note(ui, "Your keyring is empty.");
                    }
                    for id in &state.data.items {
                        let Some(def) = item_def(id) else { continue };
                        card(ui, Some(def.name), None, |ui| {
                            note(ui, def.desc);
                            let details = item_details(id, state);
                            if !details.is_empty() {
                                let rows: Vec<(&str, String)> = details.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
                                kv(ui, &format!("kv-{id}"), &rows);
                            }
                        });
                        ui.add_space(4.0);
                    }
                });
            }
            Tab::Settings => {
                let mut s = state.settings.clone();
                let before = s.clone();
                egui::Grid::new("settings").num_columns(2).spacing([24.0, 10.0]).show(ui, |ui| {
                    ui.label("Music volume");
                    ui.add(egui::Slider::new(&mut s.music, 0.0..=1.0).show_value(false));
                    ui.end_row();
                    ui.label("Effects volume");
                    if ui.add(egui::Slider::new(&mut s.sfx, 0.0..=1.0).show_value(false)).drag_stopped() {
                        sfx.push("blip");
                    }
                    ui.end_row();
                    ui.label("Text speed");
                    egui::ComboBox::from_id_salt("speed")
                        .selected_text(match s.text_speed as i32 {
                            0 => "Instant",
                            1..=40 => "Slow",
                            41..=90 => "Normal",
                            _ => "Fast",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut s.text_speed, 30.0, "Slow");
                            ui.selectable_value(&mut s.text_speed, 60.0, "Normal");
                            ui.selectable_value(&mut s.text_speed, 120.0, "Fast");
                            ui.selectable_value(&mut s.text_speed, 0.0, "Instant");
                        });
                    ui.end_row();
                    ui.label("Text size");
                    ui.add(egui::Slider::new(&mut s.ui_scale, 0.8..=1.4).show_value(false));
                    ui.end_row();
                    ui.label("CRT scanlines");
                    ui.checkbox(&mut s.crt, "");
                    ui.end_row();
                    ui.label("Reduce motion");
                    ui.checkbox(&mut s.reduced_motion, "");
                    ui.end_row();
                });
                ui.add_space(12.0);
                note(ui, "Controls: WASD or arrow keys to move, Shift to walk faster, E / Space / Enter to interact, C for the codex, Esc for this menu.");
                if s != before {
                    action = MenuAction::Settings(s);
                }
            }
            Tab::Game => {
                ui.horizontal_wrapped(|ui| {
                    if button(ui, "Save now", Kind::Primary).clicked() {
                        action = MenuAction::SaveNow;
                    }
                    if button(ui, "Copy save to clipboard", Kind::Normal).clicked() {
                        action = MenuAction::CopySave;
                    }
                    if button(ui, "Show save", Kind::Normal).clicked() {
                        m.save_box = state.save_json();
                    }
                    if button(ui, "Import from box", Kind::Normal).clicked() {
                        action = MenuAction::Import(m.save_box.clone());
                    }
                });
                ui.add(egui::TextEdit::multiline(&mut m.save_box).hint_text("Paste a save here to import it.").font(mono(11.0)).desired_rows(6).desired_width(f32::INFINITY));
                if !m.storage_ok {
                    ui.label(RichText::new("Storage is not available here, so progress lasts only until the game closes. Copy your save to keep it.").color(AMBER).font(mono(12.5)));
                }
                ui.add_space(6.0);
                if button(ui, "Save and quit to title", Kind::Danger).clicked() {
                    action = MenuAction::QuitToTitle;
                }
                note(ui, "Your save contains a random 32-byte seed. Every key, pad and share in your playthrough is derived from it, so no two players see the same puzzles.");
            }
        });
    });
    m.frames += 1;
    if m.frames > 1 && (ctx.input(|i| i.key_pressed(Key::Escape)) || modal.should_close()) {
        if action == MenuAction::None {
            action = MenuAction::Close;
        }
    }
    let _ = RED;
    action
}
