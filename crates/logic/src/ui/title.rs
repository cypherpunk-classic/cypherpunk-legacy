//! Title screen, new-game form, about page, and the credits roll.

use egui::{Align2, Color32, CornerRadius, Key, Margin, RichText, Stroke};

use super::markup::{self, Look};
use super::menu::codex_view;
use super::portraits::Portraits;
use super::theme::{self, CYAN, DIM, FAINT, LINE2, MAGENTA, PANEL, RED, mono, retro};
use super::widgets::{Kind, button, note};
use crate::state::{GameState, SaveData, Settings};

#[derive(Default)]
pub struct TitleState {
    pub page: TitlePage,
    pub handle: String,
    pub look: usize,
    pub error: String,
    pub codex_sel: Option<String>,
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TitlePage {
    #[default]
    Main,
    NewGame,
    Settings,
    About,
}

#[derive(Debug, PartialEq)]
pub enum TitleAction {
    None,
    Continue,
    NewGame { handle: String, look: usize },
    Settings(Settings),
}

pub const VERSION: &str = "v3.0";

fn logo(ui: &mut egui::Ui, time: f64) {
    let size = (ui.available_width() / 7.0).clamp(46.0, 112.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), size * 1.05), egui::Sense::hover());
    let p = ui.painter();
    let c = rect.center();
    let jitter = if (time * 0.7).fract() < 0.04 { 2.0 } else { 0.0 };
    p.text(c + egui::vec2(3.0 + jitter, 0.0), Align2::CENTER_CENTER, "CYPHERPUNK", retro(size), MAGENTA.gamma_multiply(0.55));
    p.text(c + egui::vec2(-3.0, 0.0), Align2::CENTER_CENTER, "CYPHERPUNK", retro(size), CYAN.gamma_multiply(0.55));
    p.text(c, Align2::CENTER_CENTER, "CYPHERPUNK", retro(size), Color32::WHITE);
    let small = size * 0.42;
    let (r2, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), small * 1.2), egui::Sense::hover());
    let text = "L  E  G  A  C  Y";
    ui.painter().text(r2.center(), Align2::CENTER_CENTER, text, retro(small), CYAN);
}

/// The main title menu. `save` is the stored game, if any.
pub fn show(ctx: &egui::Context, t: &mut TitleState, save: Option<&SaveData>, state: &GameState, portraits: &mut Portraits, time: f64, sfx: &mut Vec<&'static str>) -> TitleAction {
    let mut action = TitleAction::None;
    let screen = ctx.content_rect();
    egui::Area::new(egui::Id::new("title")).anchor(Align2::CENTER_CENTER, egui::vec2(0.0, -10.0)).show(ctx, |ui| {
        let w = (screen.width() - 80.0).min(720.0);
        // a dark veil over the skyline so the text reads
        egui::Frame::NONE.fill(Color32::from_rgba_unmultiplied(6, 8, 18, 228)).corner_radius(CornerRadius::same(12)).inner_margin(Margin::symmetric(24, 18)).show(ui, |ui| {
        ui.set_width(w);
        ui.vertical_centered(|ui| {
            logo(ui, time);
            ui.add_space(8.0);
            ui.allocate_ui(egui::vec2(w.min(620.0), 0.0), |ui| {
                markup::rich(
                    ui,
                    "Your grandfather left you a cabin, a cat, and twelve words. Behind them waits a city he helped build, and an argument about privacy that never ended. Every key, signature and attack in this game is real cryptography, computed on your machine.",
                    Look { size: 14.5, color: theme::rgb(0xb9c6dc), family: None },
                );
            });
            ui.add_space(16.0);
            let bw = 240.0;
            let big = |ui: &mut egui::Ui, label: &str, primary: bool| -> bool {
                let b = egui::Button::new(RichText::new(label).font(retro(26.0)).color(if primary { theme::rgb(0x02171a) } else { theme::TEXT }))
                    .fill(if primary { CYAN } else { theme::rgb(0x121a2e) })
                    .stroke(Stroke::new(1.0, if primary { CYAN } else { LINE2 }))
                    .min_size(egui::vec2(bw, 38.0));
                ui.add(b).clicked()
            };
            if save.is_some() && big(ui, "CONTINUE", true) {
                sfx.push("confirm");
                action = TitleAction::Continue;
            }
            if big(ui, "NEW GAME", save.is_none()) {
                sfx.push("select");
                t.page = TitlePage::NewGame;
                t.error.clear();
            }
            if big(ui, "SETTINGS", false) {
                sfx.push("select");
                t.page = TitlePage::Settings;
            }
            if big(ui, "ABOUT", false) {
                sfx.push("select");
                t.page = TitlePage::About;
            }
            if let Some(s) = save {
                ui.add_space(6.0);
                ui.label(RichText::new(format!("Saved game: {} · chapter {}", s.handle, if s.chapter == 0 { "P".to_string() } else { s.chapter.min(6).to_string() })).font(mono(12.5)).color(DIM));
            }
            ui.add_space(14.0);
            ui.label(RichText::new(format!("A game about the crypto wars · public domain (CC0) · {VERSION} · built with Bevy")).font(mono(12.0)).color(FAINT));
        });
        });
    });

    match t.page {
        TitlePage::Main => {}
        TitlePage::NewGame => {
            let r = window(ctx, "NEW GAME", if save.is_some() { "This replaces your current save." } else { "Who are you?" }, 520.0, |ui| {
                markup::rich(ui, "Pick a handle. In 1993 everyone on the list had one; your grandfather was \"nightjar\".", Look::default());
                let resp = ui.add(egui::TextEdit::singleline(&mut t.handle).hint_text("anon").char_limit(16).font(mono(18.0)).desired_width(f32::INFINITY));
                if !resp.has_focus() && t.handle.is_empty() {
                    resp.request_focus();
                }
                let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                ui.label(RichText::new(&t.error).color(RED).font(mono(12.5)));
                note(ui, "Look");
                ui.horizontal(|ui| {
                    for (i, (id, name)) in [("player-a", "Hoodie"), ("player-b", "Bob"), ("player-c", "Spikes")].iter().enumerate() {
                        let on = t.look == i;
                        let frame = egui::Frame::NONE.fill(if on { theme::rgb(0x13243a) } else { theme::rgb(0x0d1220) }).stroke(Stroke::new(1.0, if on { CYAN } else { LINE2 })).corner_radius(CornerRadius::same(4)).inner_margin(Margin::same(6));
                        let r = frame
                            .show(ui, |ui| {
                                ui.set_width(84.0);
                                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                                    if let Some(tex) = portraits.full_body(ui.ctx(), id) {
                                        ui.add(egui::Image::new((tex.id(), egui::vec2(64.0, 96.0))));
                                    }
                                    ui.label(RichText::new(*name).font(mono(12.5)).color(if on { CYAN } else { DIM }));
                                });
                            })
                            .response
                            .interact(egui::Sense::click());
                        if r.clicked() {
                            t.look = i;
                            sfx.push("blip");
                        }
                    }
                });
                ui.add_space(6.0);
                let mut begin = enter;
                ui.horizontal(|ui| {
                    if button(ui, "Back", Kind::Ghost).clicked() {
                        t.page = TitlePage::Main;
                    }
                    if button(ui, "Begin ▸", Kind::Primary).clicked() {
                        begin = true;
                    }
                });
                if begin {
                    let handle = if t.handle.trim().is_empty() { "anon".to_string() } else { t.handle.trim().to_lowercase() };
                    let ok = !handle.is_empty() && handle.len() <= 16 && handle.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "_.-".contains(c));
                    if ok {
                        sfx.push("confirm");
                        action = TitleAction::NewGame { handle, look: t.look };
                        t.page = TitlePage::Main;
                    } else {
                        t.error = "Letters, digits, dot, dash and underscore only (max 16).".into();
                        sfx.push("error");
                    }
                }
            });
            if r {
                t.page = TitlePage::Main;
            }
        }
        TitlePage::Settings => {
            let mut s = state.settings.clone();
            let before = s.clone();
            let r = window(ctx, "SETTINGS", "", 520.0, |ui| {
                egui::Grid::new("title-settings").num_columns(2).spacing([24.0, 10.0]).show(ui, |ui| {
                    ui.label("Music volume");
                    ui.add(egui::Slider::new(&mut s.music, 0.0..=1.0).show_value(false));
                    ui.end_row();
                    ui.label("Effects volume");
                    ui.add(egui::Slider::new(&mut s.sfx, 0.0..=1.0).show_value(false));
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
            });
            if s != before {
                action = TitleAction::Settings(s);
            }
            if r {
                t.page = TitlePage::Main;
            }
        }
        TitlePage::About => {
            let r = window(ctx, "ABOUT", "", 840.0, |ui| {
                markup::rich(ui, "Cypherpunk Legacy is a short adventure about cryptography and the people who fought to make it public. It runs entirely on your machine: no server, no tracking, no accounts.", Look::default());
                ui.add_space(4.0);
                markup::rich(ui, "All cryptography is real. Seeds follow BIP-39 and BIP-32, signatures follow BIP-340, key aggregation BIP-327, addresses BIP-341. Secret sharing, zero-knowledge proofs, lattice reduction (integral LLL), LWE and the NIST post-quantum standards are implemented in Rust or called from RustCrypto's open libraries (k256, aes-gcm, ml-kem, ml-dsa), and verified against official test vectors.", Look::default());
                ui.add_space(4.0);
                note(ui, "Characters and organisations in Cryptopolis are fictional. The history in the Codex is not. Entries you have unlocked:");
                codex_view(ui, state, &mut t.codex_sel, sfx, 300.0);
            });
            if r {
                t.page = TitlePage::Main;
            }
        }
    }
    action
}

/// A simple modal window; returns true when it should close.
fn window(ctx: &egui::Context, title: &str, subtitle: &str, width: f32, add: impl FnOnce(&mut egui::Ui)) -> bool {
    let mut close = false;
    let screen = ctx.content_rect();
    let frame = egui::Frame::NONE.fill(PANEL).stroke(Stroke::new(1.0, LINE2)).corner_radius(CornerRadius::same(6)).inner_margin(Margin::same(0));
    let m = egui::Modal::new(egui::Id::new(("title-window", title))).frame(frame).backdrop_color(Color32::from_black_alpha(140)).show(ctx, |ui| {
        ui.set_width((screen.width() - 24.0).min(width).max(200.0));
        egui::Frame::NONE.fill(theme::rgb(0x0e1424)).inner_margin(Margin::symmetric(12, 8)).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(title).font(retro(24.0)).color(CYAN));
                if !subtitle.is_empty() {
                    ui.label(RichText::new(subtitle).font(mono(12.0)).color(DIM));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if button(ui, "ESC ✕", Kind::Ghost).clicked() {
                        close = true;
                    }
                });
            });
        });
        egui::Frame::NONE.inner_margin(Margin::same(16)).show(ui, |ui| {
            egui::ScrollArea::vertical().max_height(screen.height() - 140.0).auto_shrink([false, true]).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                add(ui);
            });
        });
    });
    close || m.should_close()
}

// ---------------------------------------------------------------------------
// Credits
// ---------------------------------------------------------------------------

const CREDITS: &[(&str, &str)] = &[
    ("h1", "CYPHERPUNK LEGACY"),
    ("quote", "\"Cypherpunks write code.\""),
    ("note", "— Eric Hughes, A Cypherpunk's Manifesto, 1993"),
    ("h3", "THE PEOPLE"),
    ("line", "Grandpa (nightjar) — your legacy"),
    ("line", "Mara — one-time pads, and patience"),
    ("line", "Felix — key exchange, and the switchboard"),
    ("line", "Juno — secret sharing, and trust"),
    ("line", "Ibrahim — signatures, and the Archive"),
    ("line", "Lin — zero knowledge, and the mask"),
    ("line", "Ada — the quantum future"),
    ("line", "Nonce — a daemon wearing a cat"),
    ("h3", "THE CRYPTOGRAPHY WAS REAL"),
    ("line", "BIP-39 / BIP-32 seed recovery & fingerprints"),
    ("line", "BIP-340 Schnorr signatures"),
    ("line", "BIP-327 MuSig2 key aggregation"),
    ("line", "BIP-341 Taproot"),
    ("line", "One-time pads & the two-time pad attack"),
    ("line", "AES-CTR malleability vs AES-GCM"),
    ("line", "Diffie–Hellman, MITM & baby-step giant-step"),
    ("line", "Shamir secret sharing & Feldman VSS"),
    ("line", "Schnorr Σ-protocols, simulators & extractors"),
    ("line", "The Fiat–Shamir transform (and its weak form)"),
    ("line", "LLL lattice reduction (exact integer arithmetic)"),
    ("line", "Merkle–Hellman knapsack & the CJLOSS attack"),
    ("line", "Learning With Errors"),
    ("line", "ML-KEM-768 (FIPS 203) & ML-DSA-65 (FIPS 204)"),
    ("h3", "BUILT WITH"),
    ("line", "Rust · Bevy 0.20 · egui"),
    ("line", "RustCrypto: k256 · sha2 · hmac · pbkdf2 · aes-gcm"),
    ("line", "RustCrypto: ml-kem · ml-dsa · num-bigint"),
    ("line", "Verified against official BIP & NIST test vectors"),
    ("h3", "FOR"),
    ("line", "everyone who wrote the code instead of asking permission,"),
    ("line", "and everyone who read it and found the bugs."),
    ("quote", "Privacy is necessary for an open society in the electronic age."),
];

#[derive(Default)]
pub struct CreditsState {
    pub scroll: f32,
}

/// Rolls the credits; returns true when the player returns to the title.
pub fn credits(ctx: &egui::Context, c: &mut CreditsState, dt: f32) -> bool {
    let screen = ctx.content_rect();
    let mut done = false;
    let speed = if ctx.input(|i| i.key_down(Key::Space) || i.key_down(Key::ArrowDown)) { 160.0 } else { 34.0 };
    c.scroll += dt * speed;
    egui::Area::new(egui::Id::new("credits-bg")).fixed_pos(screen.min).order(egui::Order::Background).show(ctx, |ui| {
        ui.painter().rect_filled(screen, 0.0, theme::rgb(0x05060b));
    });
    let top = screen.bottom() - c.scroll;
    egui::Area::new(egui::Id::new("credits")).fixed_pos(egui::pos2(screen.center().x - 300.0, top)).show(ctx, |ui| {
        ui.set_width(600.0);
        ui.vertical_centered(|ui| {
            for (kind, text) in CREDITS {
                match *kind {
                    "h1" => {
                        ui.label(RichText::new(*text).font(retro(56.0)).color(Color32::WHITE));
                    }
                    "h3" => {
                        ui.add_space(26.0);
                        ui.label(RichText::new(*text).font(retro(24.0)).color(CYAN));
                    }
                    "quote" => {
                        ui.add_space(18.0);
                        ui.label(RichText::new(*text).font(egui::FontId::new(18.0, theme::italic_family())).color(theme::GOLD));
                    }
                    "note" => {
                        ui.label(RichText::new(*text).font(mono(12.5)).color(DIM));
                    }
                    _ => {
                        ui.label(RichText::new(*text).font(mono(14.0)).color(theme::rgb(0xc9d4e8)));
                    }
                }
            }
            ui.add_space(50.0);
            if button(ui, "Return to title", Kind::Primary).clicked() {
                done = true;
            }
            ui.add_space(screen.height() * 0.4);
        });
    });
    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        done = true;
    }
    done
}
