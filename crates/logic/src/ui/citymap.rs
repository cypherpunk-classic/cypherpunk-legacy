//! The district map: every place worth going, whether you can get there yet,
//! and one key to be standing in front of it. Also the dial of a locker.

use egui::{Align2, Color32, CornerRadius, Key, Margin, Pos2, Rect, RichText, Sense, Stroke, Vec2};

use super::theme::{self, AMBER, CYAN, DIM, FAINT, GOLD, GREEN, LINE, LINE2, PANEL, RED, TEXT, mono, retro};
use super::widgets::{Kind, button};
use crate::maps::MapDef;
use crate::signals::{Signals, SiteKind};
use crate::state::GameState;

pub struct MapView {
    pub cursor: usize,
    tex: Option<egui::TextureHandle>,
    pub age: f32,
}

impl MapView {
    pub fn new(sig: &Signals, st: &GameState) -> Self {
        let cursor = sig.next_site(st).and_then(|n| sig.sites.iter().position(|s| s.id == n.id)).unwrap_or(0);
        Self { cursor, tex: None, age: 0.0 }
    }
}

pub enum MapAction {
    None,
    Close,
    /// go to this site (spawn name)
    Travel(&'static str),
    /// open the campaign map (the whole city)
    Campaign,
}

fn tile_color(name: &str) -> Color32 {
    match name {
        "water" | "water_b" => theme::rgb(0x16304c),
        "wateredge" => theme::rgb(0x1f4468),
        "dock_h" | "dock_v" => theme::rgb(0x6b4526),
        "fence" | "railing" => theme::rgb(0x0b0d16),
        "hedge" => theme::rgb(0x12261c),
        "grass" | "grass2" | "flowers" => theme::rgb(0x234e33),
        "plaza" | "plaza2" | "plazaglow" => theme::rgb(0x3a3f55),
        "metal" => theme::rgb(0x374052),
        "asphalt" | "asphalt2" => theme::rgb(0x242833),
        "sand" => theme::rgb(0x6f5a3a),
        _ => theme::rgb(0x454e63),
    }
}

pub(crate) fn minimap(ctx: &egui::Context, map: &MapDef) -> egui::TextureHandle {
    let mut img = egui::ColorImage::filled([map.w, map.h], Color32::BLACK);
    for (y, row) in map.tiles.iter().enumerate() {
        for (x, t) in row.iter().enumerate() {
            img.pixels[y * map.w + x] = tile_color(t);
        }
    }
    ctx.load_texture("wharf-minimap", img, egui::TextureOptions::NEAREST)
}

/// Draw the map. `map` is the district being shown.
pub fn show(ctx: &egui::Context, v: &mut MapView, sig: &Signals, st: &GameState, map: &MapDef, dt: f32, sfx: &mut Vec<&'static str>) -> MapAction {
    v.age += dt;
    let mut action = MapAction::None;
    let tex = v.tex.get_or_insert_with(|| minimap(ctx, map)).clone();
    let n = sig.sites.len();
    let (up, down, enter, close) = ctx.input(|i| {
        (i.key_pressed(Key::ArrowUp) || i.key_pressed(Key::W), i.key_pressed(Key::ArrowDown) || i.key_pressed(Key::S), i.key_pressed(Key::Enter) || i.key_pressed(Key::E), i.key_pressed(Key::Escape) || (i.key_pressed(Key::M) && v.age > 0.2))
    });
    if ctx.input(|i| i.key_pressed(Key::Tab)) && v.age > 0.2 {
        return MapAction::Campaign;
    }
    if up {
        v.cursor = (v.cursor + n - 1) % n;
        sfx.push("blip");
    }
    if down {
        v.cursor = (v.cursor + 1) % n;
        sfx.push("blip");
    }
    let screen = ctx.content_rect();
    let scale = ((screen.width() - 420.0) / map.w as f32).clamp(4.0, 9.0).floor();
    let map_size = Vec2::new(map.w as f32 * scale, map.h as f32 * scale);
    let frame = egui::Frame::NONE.fill(PANEL).stroke(Stroke::new(1.0, LINE2)).corner_radius(CornerRadius::same(6)).inner_margin(Margin::same(14));
    egui::Modal::new(egui::Id::new("district-map")).frame(frame).backdrop_color(Color32::from_black_alpha(150)).show(ctx, |ui| {
        ui.set_max_height(screen.height() - 40.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(map.title.to_uppercase()).font(retro(30.0)).color(CYAN));
            let done = sig.sites.iter().filter(|s| st.flag(s.done)).count();
            ui.label(RichText::new(format!("{done} of {n}")).font(mono(13.0)).color(DIM));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if button(ui, "M ✕", Kind::Ghost).clicked() {
                    action = MapAction::Close;
                }
                if button(ui, "CITY ▸", Kind::Normal).on_hover_text("The campaign map: every stage, your stars, upgrades (Tab)").clicked() {
                    action = MapAction::Campaign;
                }
            });
        });
        ui.add_space(4.0);
        ui.horizontal_top(|ui| {
            // the picture
            let (rect, _) = ui.allocate_exact_size(map_size, Sense::hover());
            ui.painter().image(tex.id(), rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
            ui.painter().rect_stroke(rect, 2.0, Stroke::new(1.0, LINE2), egui::StrokeKind::Outside);
            // gates
            for o in &map.objects {
                if o.tex.starts_with("obj-barrier") {
                    let shut = st.cond(o.show_if);
                    let p = rect.min + Vec2::new(o.x / 16.0 * scale, (o.y - 8.0) / 16.0 * scale);
                    ui.painter().rect_filled(Rect::from_center_size(p, Vec2::splat(scale * 0.9)), 1.0, if shut { RED } else { GREEN.gamma_multiply(0.5) });
                }
            }
            for (i, s) in sig.sites.iter().enumerate() {
                let Some(sp) = map.spawns.get(s.spawn) else { continue };
                let p = rect.min + Vec2::new(sp.x / 16.0 * scale, sp.y / 16.0 * scale);
                let (open, done) = (sig.site_open(st, s.id), st.flag(s.done));
                let c = if done {
                    GREEN
                } else if open {
                    GOLD
                } else {
                    FAINT
                };
                let r = if i == v.cursor { scale * 1.5 + (v.age * 5.0).sin().abs() * 2.0 } else { scale * 1.1 };
                ui.painter().circle_filled(p, r, theme::rgb(0x0b0d16));
                ui.painter().circle_stroke(p, r, Stroke::new(if i == v.cursor { 2.0 } else { 1.0 }, c));
                let mark = if matches!(s.kind, SiteKind::Dive(_)) { "◆".to_string() } else { format!("{}", i + 1) };
                ui.painter().text(p, Align2::CENTER_CENTER, mark, mono((scale * 1.3).clamp(8.0, 12.0)), c);
            }
            ui.add_space(8.0);
            // the list
            ui.vertical(|ui| {
                ui.set_width(330.0);
                egui::ScrollArea::vertical().max_height(map_size.y).auto_shrink([false, true]).show(ui, |ui| {
                    for (i, s) in sig.sites.iter().enumerate() {
                        let (open, done) = (sig.site_open(st, s.id), st.flag(s.done));
                        let sel = i == v.cursor;
                        let c = if done {
                            GREEN
                        } else if open {
                            GOLD
                        } else {
                            FAINT
                        };
                        let frame = egui::Frame::NONE
                            .fill(if sel { theme::rgb(0x13243a) } else { Color32::TRANSPARENT })
                            .stroke(Stroke::new(1.0, if sel { CYAN } else { LINE }))
                            .corner_radius(CornerRadius::same(3))
                            .inner_margin(Margin::symmetric(8, 4));
                        let resp = frame
                            .show(ui, |ui| {
                                ui.set_width(306.0);
                                ui.horizontal(|ui| {
                                    let mark = if matches!(s.kind, SiteKind::Dive(_)) { "◆".to_string() } else { format!("{}", i + 1) };
                                    ui.label(RichText::new(mark).font(mono(12.0)).color(c));
                                    ui.label(RichText::new(s.name).font(retro(19.0)).color(if open { TEXT } else { FAINT }));
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        let status = if done {
                                            "done"
                                        } else if open {
                                            "open"
                                        } else {
                                            "shut"
                                        };
                                        ui.label(RichText::new(status).font(mono(11.5)).color(c));
                                    });
                                });
                                if sel {
                                    let line = if open { s.blurb.to_string() } else { format!("In the way: {}.", s.blocked_by) };
                                    ui.add(egui::Label::new(RichText::new(line).font(mono(12.0)).color(if open { DIM } else { AMBER })).wrap());
                                }
                            })
                            .response
                            .interact(Sense::click());
                        if resp.hovered() && ctx.input(|i| i.pointer.delta() != Vec2::ZERO) {
                            v.cursor = i;
                        }
                        if resp.clicked() {
                            v.cursor = i;
                            if open {
                                action = MapAction::Travel(s.spawn);
                            } else {
                                sfx.push("error");
                            }
                        }
                        if sel && (up || down) {
                            resp.scroll_to_me(None);
                        }
                    }
                });
            });
        });
        ui.add_space(4.0);
        ui.label(RichText::new("↑ ↓ choose   ·   Enter: go there   ·   Tab: the city   ·   M: close   ·   walking works too").font(mono(12.0)).color(FAINT));
    });
    if enter && matches!(action, MapAction::None) {
        let s = &sig.sites[v.cursor];
        if sig.site_open(st, s.id) {
            action = MapAction::Travel(s.spawn);
        } else {
            sfx.push("error");
        }
    }
    if close && matches!(action, MapAction::None) {
        action = MapAction::Close;
    }
    action
}

// --- lockers ---------------------------------------------------------------------------

pub struct LockerView {
    pub id: &'static str,
    pub reply: u64,
    pub digits: Vec<u8>,
    shake: f32,
    age: f32,
    /// seconds since it opened
    pub opened: Option<f32>,
}

impl LockerView {
    pub fn new(id: &'static str, reply: u64, len: usize, open: bool) -> Self {
        Self { id, reply, digits: vec![0; len], shake: 0.0, age: 0.0, opened: open.then_some(9.0) }
    }
}

/// What is inside, once it opens.
fn contents(id: &str) -> &'static str {
    match id {
        "kit" => "A field kit, packed by hand: wax for taking rubbings of a board, and a folding light table for stacking cards and cutting new ones.",
        "ice" => "One card, in Grandpa's punch: a bird with its wings out. His sign. It is in your glyph book now.",
        _ => "Empty.",
    }
}

/// Draw a locker's dial. Returns `Some(open)` when the player steps away.
pub fn locker(ctx: &egui::Context, v: &mut LockerView, sig: &mut Signals, st: &mut GameState, dt: f32, sfx: &mut Vec<&'static str>) -> Option<bool> {
    v.age += dt;
    v.shake = (v.shake - dt).max(0.0);
    if let Some(t) = &mut v.opened {
        *t += dt;
    }
    let title = sig.locker(v.id).map(|l| l.title).unwrap_or("Locker");
    let mut result = None;
    let mut try_it = false;
    let shake = if v.shake > 0.0 { (v.age * 70.0).sin() * 5.0 * (v.shake / 0.3) } else { 0.0 };
    let frame = egui::Frame::NONE.fill(PANEL).stroke(Stroke::new(1.0, LINE2)).corner_radius(CornerRadius::same(6)).inner_margin(Margin::same(16)).outer_margin(Margin { left: shake.max(0.0) as i8, right: (-shake).max(0.0) as i8, top: 0, bottom: 0 });
    egui::Modal::new(egui::Id::new(("locker", v.id))).frame(frame).backdrop_color(Color32::from_black_alpha(110)).show(ctx, |ui| {
        ui.set_width(300.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(title.to_uppercase()).font(retro(26.0)).color(GOLD));
            ui.add_space(6.0);
            if v.opened.is_some() {
                ui.label(RichText::new("OPEN").font(retro(34.0)).color(GREEN));
                ui.add(egui::Label::new(RichText::new(contents(v.id)).font(mono(13.0)).color(TEXT)).wrap());
                ui.add_space(8.0);
                if button(ui, "Take it  ▸", Kind::Primary).clicked() {
                    result = Some(true);
                }
            } else {
                ui.horizontal(|ui| {
                    let w = v.digits.len() as f32 * 64.0;
                    ui.add_space(((ui.available_width() - w) / 2.0).max(0.0));
                    for d in v.digits.iter_mut() {
                        ui.vertical(|ui| {
                            if ui.add(egui::Button::new(RichText::new("▲").font(mono(15.0)).color(DIM)).min_size(Vec2::new(52.0, 22.0))).clicked() {
                                *d = (*d + 1) % 10;
                                sfx.push("tick");
                            }
                            let (r, _) = ui.allocate_exact_size(Vec2::new(52.0, 60.0), Sense::hover());
                            ui.painter().rect_filled(r, 4.0, theme::rgb(0x07080d));
                            ui.painter().rect_stroke(r, 4.0, Stroke::new(1.0, LINE2), egui::StrokeKind::Inside);
                            ui.painter().text(r.center(), Align2::CENTER_CENTER, d.to_string(), retro(52.0), GOLD);
                            if ui.add(egui::Button::new(RichText::new("▼").font(mono(15.0)).color(DIM)).min_size(Vec2::new(52.0, 22.0))).clicked() {
                                *d = (*d + 9) % 10;
                                sfx.push("tick");
                            }
                        });
                    }
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_space((ui.available_width() - 170.0).max(0.0) / 2.0);
                    if button(ui, "Try the handle", Kind::Primary).clicked() {
                        try_it = true;
                    }
                    if button(ui, "Leave", Kind::Ghost).clicked() {
                        result = Some(false);
                    }
                });
            }
        });
    });
    // digits can be typed
    if v.opened.is_none() {
        let typed: Vec<u8> = ctx.input(|i| {
            i.events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::Text(t) => t.chars().next().and_then(|c| c.to_digit(10)).map(|d| d as u8),
                    _ => None,
                })
                .collect()
        });
        for d in typed {
            v.digits.rotate_left(1);
            *v.digits.last_mut().unwrap() = d;
            sfx.push("tick");
        }
        if ctx.input(|i| i.key_pressed(Key::Enter)) {
            try_it = true;
        }
    } else if ctx.input(|i| i.key_pressed(Key::Enter) || i.key_pressed(Key::E)) && v.opened.is_some_and(|t| t > 0.3) {
        result = Some(true);
    }
    if try_it {
        if sig.dial(st, v.id, &v.digits) {
            v.opened = Some(0.0);
            sfx.push("unlock");
        } else {
            v.shake = 0.3;
            sfx.push("error");
        }
    }
    if ctx.input(|i| i.key_pressed(Key::Escape)) && result.is_none() {
        result = Some(v.opened.is_some());
    }
    result
}
