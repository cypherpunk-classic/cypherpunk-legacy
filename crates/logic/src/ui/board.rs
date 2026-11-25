//! The close-up of a signal board: the lamps, the card readers, the lever,
//! the light table, your cards and the glyph book. No instructions; the
//! board reacts and the district outside keeps moving.

use cpl_crypto::lamps::{Bits, lamp};
use egui::{Align, Align2, Color32, CornerRadius, Key, Layout, Margin, Pos2, Rect, RichText, Sense, Stroke, Vec2};

use super::theme::{self, AMBER, CYAN, DIM, FAINT, GOLD, GREEN, LINE, LINE2, PANEL, RED, TEXT, VIOLET, mono, retro};
use super::widgets::{Kind, button, button_enabled};
use crate::signals::wharf::KIT;
use crate::signals::{CardKind, CardRef, Glyph, Outcome, Signals};
use crate::state::GameState;

const LAMP_ON: Color32 = theme::rgb(0xffcd75);
const LAMP_OFF: Color32 = theme::rgb(0x151a26);
const LAMP_CLEAR: Color32 = theme::rgb(0xa4ffe0);
const LAMP_SEAL: Color32 = theme::rgb(0x73eff7);

/// Where a click on a card sends it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Send {
    Reader,
    Table,
}

pub struct BoardView {
    pub board: &'static str,
    /// the script waiting for the close-up to end
    pub reply: u64,
    /// cards stacked on the light table
    pub table: Vec<CardRef>,
    pub send: Send,
    /// Grandpa's notes shown so far
    pub notes: usize,
    pub last: Option<(Outcome, f32)>,
    /// seconds since each lamp last changed (for the flicker)
    flash: [f32; 64],
    prev: Bits,
    /// close by itself after a lock obeys
    pub close_in: Option<f32>,
    drag: Option<CardRef>,
    pub age: f32,
    pub shake: f32,
}

impl BoardView {
    pub fn new(board: &'static str, reply: u64, shown: Bits) -> Self {
        Self { board, reply, table: Vec::new(), send: Send::Reader, notes: 0, last: None, flash: [9.0; 64], prev: shown, close_in: None, drag: None, age: 0.0, shake: 0.0 }
    }
}

fn kind_color(k: CardKind) -> Color32 {
    match k {
        CardKind::Pad => GOLD,
        CardKind::Copy => CYAN,
        CardKind::Cut => VIOLET,
    }
}

/// Paint an 8 x 8 grid of lamps (or holes) into `rect`.
pub fn paint_bits(p: &egui::Painter, rect: Rect, bits: Bits, on: Color32, off: Color32, seal_row: bool) {
    let cell = rect.width() / 8.0;
    let gap = (cell * 0.12).max(0.5);
    for y in 0..8 {
        for x in 0..8 {
            let r = Rect::from_min_size(rect.min + Vec2::new(x as f32 * cell + gap / 2.0, y as f32 * cell + gap / 2.0), Vec2::splat(cell - gap));
            let lit = lamp(bits, x, y);
            let c = if lit {
                if seal_row && y == 7 { LAMP_SEAL } else { on }
            } else {
                off
            };
            p.rect_filled(r, (cell * 0.18).min(3.0), c);
        }
    }
}

struct Thumb {
    r: CardRef,
    bits: Bits,
    label: String,
    tip: String,
    color: Color32,
}

fn thumbs(sig: &Signals, st: &GameState) -> (Vec<Thumb>, Vec<Thumb>) {
    let own = st
        .data
        .cards
        .iter()
        .map(|c| Thumb { r: CardRef::Own(c.id), bits: c.value(), label: c.name.clone(), tip: c.note.clone(), color: kind_color(c.kind) })
        .collect();
    let book = sig.known_glyphs(st).into_iter().map(|g| Thumb { r: CardRef::Glyph(g), bits: g.bits(), label: g.label().to_string(), tip: "a stencil from the glyph book".into(), color: DIM }).collect();
    (own, book)
}

/// One card thumbnail. Returns (clicked, secondary-clicked, drag started).
fn thumb(ui: &mut egui::Ui, t: &Thumb, size: f32, marked: Option<Color32>) -> (bool, bool, bool, Rect) {
    // wide enough for its name, so names never run into each other
    let w = (size + 8.0).max(t.label.chars().count() as f32 * 6.4 + 4.0);
    let (outer, resp) = ui.allocate_exact_size(Vec2::new(w, size + 22.0), Sense::click_and_drag());
    let rect = Rect::from_min_size(Pos2::new(outer.center().x - (size + 8.0) / 2.0, outer.min.y), Vec2::new(size + 8.0, size + 22.0));
    let p = ui.painter();
    let hovered = resp.hovered();
    let frame = Rect::from_min_size(rect.min, Vec2::new(size + 8.0, size + 8.0));
    p.rect_filled(frame, 3.0, if hovered { theme::rgb(0x1b2440) } else { theme::rgb(0x0f1424) });
    p.rect_stroke(frame, 3.0, Stroke::new(if marked.is_some() { 2.0 } else { 1.0 }, marked.unwrap_or(if hovered { t.color } else { t.color.gamma_multiply(0.55) })), egui::StrokeKind::Inside);
    paint_bits(p, Rect::from_min_size(rect.min + Vec2::splat(4.0), Vec2::splat(size)), t.bits, theme::rgb(0xe6e2c3), theme::rgb(0x1a2033), false);
    p.text(Pos2::new(rect.center().x, rect.max.y - 7.0), Align2::CENTER_CENTER, &t.label, mono(10.5), if hovered { TEXT } else { t.color });
    let resp = if t.tip.is_empty() { resp } else { resp.on_hover_text(&t.tip) };
    (resp.clicked(), resp.secondary_clicked(), resp.drag_started(), outer)
}

fn outcome_text(o: Outcome) -> (String, Color32) {
    match o {
        Outcome::Done(g, _) => (format!("THE LOCK OBEYS: {}", g.label()), GREEN),
        Outcome::Idle(g) => (format!("THE LOCK READS {}. NOTHING CHANGES.", g.label()), DIM),
        Outcome::Alarm(g) => (format!("THE LOCK READS {}.", g.label()), RED),
        Outcome::NotASignal => ("NOT AN ORDER. THE LOCK SAW NOISE.".into(), AMBER),
        Outcome::SealBroken => ("SEAL BROKEN.".into(), RED),
        Outcome::NoLock => (String::new(), DIM),
    }
}

/// Show what the lock did, with the sound and shake that go with it.
pub fn after_pull(v: &mut BoardView, o: Outcome, sfx: &mut Vec<&'static str>) {
    v.last = Some((o, 0.0));
    match o {
        Outcome::Done(..) => {
            sfx.push("unlock");
            v.close_in = Some(1.1);
        }
        Outcome::Idle(_) => sfx.push("blip"),
        Outcome::Alarm(_) | Outcome::SealBroken => {
            sfx.push("alarm");
            v.shake = 0.4;
        }
        Outcome::NotASignal => {
            sfx.push("error");
            v.shake = 0.25;
        }
        Outcome::NoLock => {}
    }
}

/// What the close-up wants the game to know.
#[derive(Default)]
pub struct BoardResult {
    pub close: bool,
}

/// Draw the close-up and carry out what the player does. `seen`: a sensor
/// or the guard has the player right now; `footsteps`: the guard is coming.
pub fn show(ctx: &egui::Context, v: &mut BoardView, sig: &mut Signals, st: &mut GameState, dt: f32, (seen, footsteps): (bool, bool), sfx: &mut Vec<&'static str>) -> BoardResult {
    let mut out = BoardResult::default();
    v.age += dt;
    v.shake = (v.shake - dt).max(0.0);
    if let Some((_, age)) = &mut v.last {
        *age += dt;
    }
    if let Some(left) = &mut v.close_in {
        *left -= dt;
        if *left <= 0.0 {
            out.close = true;
        }
    }
    // lamp flicker bookkeeping
    let shown = sig.shown(st, v.board);
    let changed = shown ^ v.prev;
    for (i, f) in v.flash.iter_mut().enumerate() {
        if changed >> i & 1 == 1 {
            *f = 0.0;
        } else {
            *f += dt;
        }
    }
    v.prev = shown;

    let Some(b) = sig.board(v.board) else {
        out.close = true;
        return out;
    };
    let (id, title, console, slots, keys, monitor, has_lock, sealed, encrypted) = (b.id, b.title, b.console, b.slots, b.keys, b.monitor, b.has_lock(), b.seal.is_some(), b.encrypted());
    let legend = b.legend.clone();
    let in_slots = b.cards.clone();
    let in_clear = b.message().is_some_and(|g| g.bits() == shown) && b.reveal.is_some();
    let countdown = match &b.feed {
        crate::signals::Feed::Rotating { period, .. } => b.feed.until_change(sig.t).map(|l| (l, *period)),
        _ => None,
    };
    let site = b.site;
    let notes: Vec<&'static str> = sig.site(site).map(|s| s.notes.to_vec()).unwrap_or_default();
    let kit = st.flag(KIT);
    let lock_view = sig.lock_view(st, id);
    let (own, book) = thumbs(sig, st);

    // actions collected while drawing, applied afterwards
    let mut toggles: Vec<(usize, usize)> = Vec::new();
    let mut to_reader: Option<CardRef> = None;
    let mut to_table: Option<CardRef> = None;
    let mut unslot: Option<usize> = None;
    let mut pull = false;
    let mut rub = false;
    let mut cut = false;
    let mut clear_board = false;
    let mut discard: Option<u32> = None;
    let mut readers_rect = Rect::NOTHING;
    let mut table_rect = Rect::NOTHING;

    let screen = ctx.content_rect();
    let wide = screen.width() >= 900.0;
    let cell = if wide { ((screen.height() - 330.0) / 8.0).clamp(22.0, 36.0) } else { 24.0 };
    let grid = cell * 8.0;
    let width = (screen.width() - 24.0).min(if kit { grid + 580.0 } else { grid + 320.0 });
    let shake = if v.shake > 0.0 { (v.age * 70.0).sin() * 5.0 * (v.shake / 0.4) } else { 0.0 };
    let frame = egui::Frame::NONE.fill(PANEL).stroke(Stroke::new(1.0, LINE2)).corner_radius(CornerRadius::same(6)).inner_margin(Margin::ZERO).outer_margin(Margin { left: shake.max(0.0) as i8, right: (-shake).max(0.0) as i8, top: 0, bottom: 0 });
    egui::Modal::new(egui::Id::new(("board", id))).frame(frame).backdrop_color(Color32::from_black_alpha(96)).show(ctx, |ui| {
        ui.set_width(width);
        ui.set_max_height(screen.height() - 30.0);
        // --- header -------------------------------------------------------------------
        egui::Frame::NONE.fill(theme::rgb(0x0e1424)).inner_margin(Margin::symmetric(12, 7)).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(title.to_uppercase()).font(retro(24.0)).color(GOLD));
                if !console.is_empty() {
                    ui.label(RichText::new(format!("console: {console}")).font(mono(12.0)).color(DIM)).on_hover_text("Boards fed by one console share its pad.");
                } else {
                    ui.label(RichText::new("no console: plain lamps").font(mono(12.0)).color(DIM));
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if button(ui, "ESC ✕", Kind::Ghost).clicked() {
                        out.close = true;
                    }
                    if !notes.is_empty() {
                        let label = if v.notes == 0 { "?".to_string() } else { format!("? {}/{}", v.notes, notes.len()) };
                        let costs = v.notes < notes.len() && v.notes + 1 > crate::campaign::free_hint_pages(st);
                        let tip = if costs { "Grandpa's notebook: a nudge, then a push, then the answer.\nThe next page costs this run its hint star." } else { "Grandpa's notebook: a nudge, then a push, then the answer." };
                        if button(ui, &label, Kind::Ghost).on_hover_text(tip).clicked() && v.notes < notes.len() {
                            v.notes += 1;
                            crate::campaign::hint_read(st, crate::state::MapId::Wharf, v.notes);
                            sfx.push("blip");
                        }
                    }
                    // the street outside does not wait
                    let c = if st.trace > 75.0 { RED } else if st.trace > 40.0 { AMBER } else { CYAN };
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(90.0, 7.0), Sense::hover());
                    ui.painter().rect_filled(rect, 2.0, theme::rgb(0x111830));
                    let mut fill = rect;
                    fill.set_width(rect.width() * (st.trace / 100.0).clamp(0.0, 1.0));
                    ui.painter().rect_filled(fill, 2.0, c);
                    let (word, wc) = if seen {
                        ("SEEN", RED)
                    } else if footsteps {
                        ("FOOTSTEPS", AMBER)
                    } else {
                        ("TRACE", c)
                    };
                    let blink = !(seen || footsteps) || (v.age * 4.0).fract() < 0.7;
                    ui.label(RichText::new(word).font(retro(14.0)).color(if blink { wc } else { wc.gamma_multiply(0.3) }));
                });
            });
        });
        ui.painter().hline(ui.min_rect().x_range(), ui.cursor().top(), Stroke::new(1.0, GOLD.gamma_multiply(0.4)));

        egui::Frame::NONE.inner_margin(Margin::symmetric(14, 10)).show(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 6.0);
            ui.horizontal_top(|ui| {
                // --- the board --------------------------------------------------------------
                ui.vertical(|ui| {
                    ui.set_width(grid + 170.0);
                    if !legend.is_empty() {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("POSTED").font(retro(14.0)).color(FAINT));
                            for g in &legend {
                                let (r, resp) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::hover());
                                paint_bits(ui.painter(), r, g.bits(), theme::rgb(0xe6e2c3), theme::rgb(0x1a2033), false);
                                resp.on_hover_text(g.label());
                            }
                        });
                    }
                    ui.horizontal_top(|ui| {
                        // lamps
                        let (rect, resp) = ui.allocate_exact_size(Vec2::splat(grid + 12.0), if keys { Sense::click() } else { Sense::hover() });
                        let p = ui.painter();
                        p.rect_filled(rect, 6.0, theme::rgb(0x07080d));
                        p.rect_stroke(rect, 6.0, Stroke::new(2.0, if in_clear { LAMP_CLEAR.gamma_multiply(0.7) } else { theme::rgb(0x2a3040) }), egui::StrokeKind::Inside);
                        let inner = Rect::from_min_size(rect.min + Vec2::splat(6.0), Vec2::splat(grid));
                        let gap = cell * 0.14;
                        for y in 0..8 {
                            for x in 0..8 {
                                let i = y * 8 + x;
                                let r = Rect::from_min_size(inner.min + Vec2::new(x as f32 * cell + gap / 2.0, y as f32 * cell + gap / 2.0), Vec2::splat(cell - gap));
                                let lit = lamp(shown, x, y);
                                let base = if in_clear {
                                    LAMP_CLEAR
                                } else if sealed && y == 7 {
                                    LAMP_SEAL
                                } else {
                                    LAMP_ON
                                };
                                let mut c = if lit { base } else { LAMP_OFF };
                                // a lamp that just changed flashes white and settles
                                let f = v.flash[i];
                                if f < 0.25 {
                                    c = c.lerp_to_gamma(Color32::WHITE, 1.0 - f / 0.25);
                                }
                                if lit {
                                    p.rect_filled(r.expand(1.5), cell * 0.25, base.gamma_multiply(0.12));
                                }
                                p.rect_filled(r, cell * 0.2, c);
                            }
                        }
                        if sealed {
                            let y = inner.min.y + 7.0 * cell - 1.0;
                            p.hline(inner.x_range(), y, Stroke::new(1.0, LAMP_SEAL.gamma_multiply(0.5)));
                            p.text(Pos2::new(inner.max.x - 2.0, y - 2.0), Align2::RIGHT_BOTTOM, "seal", mono(10.0), LAMP_SEAL.gamma_multiply(0.8));
                        }
                        if !keys && encrypted && has_lock {
                            // glass over the lamps
                            p.line_segment([inner.min + Vec2::new(8.0, 20.0), inner.min + Vec2::new(26.0, 6.0)], Stroke::new(2.0, Color32::from_white_alpha(26)));
                            p.line_segment([inner.min + Vec2::new(8.0, 32.0), inner.min + Vec2::new(40.0, 8.0)], Stroke::new(1.0, Color32::from_white_alpha(18)));
                        }
                        if keys
                            && resp.clicked()
                            && let Some(pos) = resp.interact_pointer_pos()
                        {
                            let (x, y) = (((pos.x - inner.min.x) / cell).floor() as i32, ((pos.y - inner.min.y) / cell).floor() as i32);
                            if (0..8).contains(&x) && (0..8).contains(&y) {
                                toggles.push((x as usize, y as usize));
                            }
                        }
                        if keys {
                            resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                        }

                        // readers, lever, monitor
                        ui.vertical(|ui| {
                            ui.set_width(146.0);
                            ui.label(RichText::new(if slots == 1 { "READER" } else { "READERS" }).font(retro(15.0)).color(FAINT));
                            let start = ui.cursor().min;
                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::splat(4.0);
                                for i in 0..slots {
                                    match in_slots.get(i) {
                                        Some(r) => {
                                            let t = own.iter().chain(book.iter()).find(|t| t.r == *r);
                                            if let Some(t) = t {
                                                let (c, s, _, _) = thumb(ui, t, 30.0, Some(GOLD));
                                                if c || s {
                                                    unslot = Some(i);
                                                }
                                            }
                                        }
                                        None => {
                                            let (r, _) = ui.allocate_exact_size(Vec2::new(38.0, 52.0), Sense::hover());
                                            let slot = Rect::from_min_size(r.min, Vec2::splat(38.0));
                                            ui.painter().rect_stroke(slot, 3.0, Stroke::new(1.0, LINE2), egui::StrokeKind::Inside);
                                            ui.painter().text(slot.center(), Align2::CENTER_CENTER, "+", mono(16.0), FAINT);
                                        }
                                    }
                                }
                            });
                            readers_rect = Rect::from_min_max(start, Pos2::new(start.x + 146.0, ui.cursor().min.y)).expand(6.0);
                            if (!in_slots.is_empty() || keys) && button(ui, "Clear", Kind::Ghost).on_hover_text("Pull every card and undo every flip.").clicked() {
                                clear_board = true;
                            }
                            if kit && button(ui, "Take rubbing", Kind::Normal).on_hover_text("Copy the lamps, as they are now, onto a card.").clicked() {
                                rub = true;
                            }
                            if has_lock {
                                ui.add_space(4.0);
                                let lever = egui::Button::new(RichText::new("PULL LEVER").font(retro(20.0)).color(theme::rgb(0x0e0b16))).fill(RED).min_size(Vec2::new(118.0, 32.0));
                                if ui.add(lever).on_hover_text("The lock reads the lamps through its own pad and does what it finds.").clicked() {
                                    pull = true;
                                }
                            }
                            if monitor {
                                ui.add_space(4.0);
                                ui.label(RichText::new("THE LOCK SEES").font(retro(14.0)).color(CYAN));
                                let (r, _) = ui.allocate_exact_size(Vec2::splat(100.0), Sense::hover());
                                ui.painter().rect_filled(r, 3.0, theme::rgb(0x0b2a31));
                                paint_bits(ui.painter(), r.shrink(4.0), lock_view, CYAN, theme::rgb(0x0f3b44), false);
                                let name = Glyph::from_bits(lock_view).map(|g| g.label()).unwrap_or("nothing it knows");
                                ui.label(RichText::new(name).font(mono(11.5)).color(CYAN));
                            }
                        });
                    });
                    // feedback under the board
                    if let Some((left, period)) = countdown {
                        let (r, _) = ui.allocate_exact_size(Vec2::new(grid + 12.0, 5.0), Sense::hover());
                        ui.painter().rect_filled(r, 2.0, theme::rgb(0x111830));
                        let mut f = r;
                        f.set_width(r.width() * (left / period).clamp(0.0, 1.0));
                        ui.painter().rect_filled(f, 2.0, if left < 6.0 { RED } else { AMBER });
                        ui.label(RichText::new("the console repaints this board when the bar runs out").font(mono(11.0)).color(FAINT));
                    }
                    if in_clear {
                        let g = Glyph::from_bits(shown).map(|g| g.label()).unwrap_or("");
                        ui.label(RichText::new(format!("IN CLEAR: {g}")).font(retro(20.0)).color(LAMP_CLEAR));
                    }
                    if let Some((o, age)) = v.last
                        && age < 6.0
                    {
                        let (text, c) = outcome_text(o);
                        ui.label(RichText::new(text).font(retro(20.0)).color(c));
                    }
                });

                // --- the light table ----------------------------------------------------------
                if kit {
                    ui.add_space(4.0);
                    ui.vertical(|ui| {
                        let r = egui::Frame::NONE.fill(theme::rgb(0x0a0f1c)).stroke(Stroke::new(1.0, LINE)).corner_radius(CornerRadius::same(4)).inner_margin(Margin::same(8)).show(ui, |ui| {
                            ui.set_width(ui.available_width().min(250.0));
                            ui.label(RichText::new("LIGHT TABLE").font(retro(16.0)).color(VIOLET));
                            let prev = 132.0;
                            let x = sig.xor(st, &v.table);
                            ui.horizontal_top(|ui| {
                                let (r, _) = ui.allocate_exact_size(Vec2::splat(prev), Sense::hover());
                                ui.painter().rect_filled(r, 4.0, theme::rgb(0xd9d2b0));
                                paint_bits(ui.painter(), r.shrink(6.0), x, theme::rgb(0x12141c), theme::rgb(0xefe9cc), false);
                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing = Vec2::splat(3.0);
                                    for (i, r) in v.table.clone().iter().enumerate() {
                                        if let Some(t) = own.iter().chain(book.iter()).find(|t| t.r == *r) {
                                            let (c, s, _, _) = thumb(ui, t, 22.0, Some(VIOLET));
                                            if c || s {
                                                v.table.remove(i);
                                                sfx.push("blip");
                                                break;
                                            }
                                        }
                                    }
                                    if v.table.is_empty() {
                                        ui.label(RichText::new("stack\ncards\nhere").font(mono(11.0)).color(FAINT));
                                    }
                                });
                            });
                            let known = Glyph::from_bits(x).filter(|g| sig.known_glyphs(st).contains(g));
                            let text = match (v.table.len(), known) {
                                (0, _) => "what shows through a stack".to_string(),
                                (_, Some(g)) => format!("that is {}", g.label()),
                                (_, None) if x == 0 => "nothing shows through".to_string(),
                                _ => format!("{} holes", x.count_ones()),
                            };
                            ui.label(RichText::new(text).font(mono(12.0)).color(if known.is_some() { GREEN } else { DIM }));
                            ui.horizontal(|ui| {
                                if button_enabled(ui, v.table.len() >= 2 || (v.table.len() == 1 && matches!(v.table[0], CardRef::Glyph(_))), "Cut a card", Kind::Primary).on_hover_text("Cut what shows through into a card of its own.").clicked() {
                                    cut = true;
                                }
                                if button_enabled(ui, !v.table.is_empty(), "Sweep", Kind::Ghost).clicked() {
                                    v.table.clear();
                                }
                            });
                        });
                        table_rect = r.response.rect;
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("a click sends a card to").font(mono(11.0)).color(FAINT));
                            for (mode, label) in [(Send::Reader, "the reader"), (Send::Table, "the table")] {
                                let on = v.send == mode;
                                if ui.add(egui::Button::new(RichText::new(label).font(mono(11.5)).color(if on { theme::rgb(0x0e0b16) } else { DIM })).fill(if on { CYAN } else { theme::rgb(0x0f1424) })).clicked() {
                                    v.send = mode;
                                }
                            }
                        });
                        ui.label(RichText::new("(right-click: the other one; or drag)").font(mono(10.5)).color(FAINT));
                    });
                }
            });

            // --- your cards and the glyph book --------------------------------------------------
            ui.add_space(2.0);
            ui.painter().hline(ui.min_rect().x_range(), ui.cursor().top(), Stroke::new(1.0, LINE));
            ui.add_space(4.0);
            let max_h = (screen.height() - grid - 250.0).clamp(84.0, 190.0);
            egui::ScrollArea::vertical().id_salt("cards").max_height(max_h).auto_shrink([false, true]).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(5.0, 4.0);
                    ui.label(RichText::new("CARDS").font(retro(15.0)).color(GOLD));
                    for t in &own {
                        let mark = if in_slots.contains(&t.r) {
                            Some(GOLD)
                        } else if v.table.contains(&t.r) {
                            Some(VIOLET)
                        } else {
                            None
                        };
                        let (c, s, d, rect) = thumb(ui, t, 34.0, mark);
                        let primary = if v.send == Send::Reader || !kit { &mut to_reader } else { &mut to_table };
                        if c {
                            *primary = Some(t.r);
                        }
                        if s && kit {
                            if v.send == Send::Reader { to_table = Some(t.r) } else { to_reader = Some(t.r) }
                        }
                        if d {
                            v.drag = Some(t.r);
                        }
                        // rubbings and cut cards can be thrown away
                        if let CardRef::Own(cid) = t.r
                            && t.color != GOLD
                            && ui.rect_contains_pointer(rect)
                        {
                            let x = Rect::from_min_size(Pos2::new(rect.max.x - 13.0, rect.min.y + 1.0), Vec2::splat(12.0));
                            let resp = ui.interact(x, ui.id().with(("discard", cid)), Sense::click());
                            ui.painter().text(x.center(), Align2::CENTER_CENTER, "×", mono(12.0), if resp.hovered() { RED } else { DIM });
                            if resp.clicked() {
                                discard = Some(cid);
                            }
                        }
                    }
                    if !book.is_empty() {
                        ui.add_space(6.0);
                        ui.label(RichText::new("GLYPH BOOK").font(retro(15.0)).color(DIM));
                        for t in &book {
                            let mark = if in_slots.contains(&t.r) {
                                Some(GOLD)
                            } else if v.table.contains(&t.r) {
                                Some(VIOLET)
                            } else {
                                None
                            };
                            let (c, s, d, _) = thumb(ui, t, 34.0, mark);
                            let primary = if v.send == Send::Reader || !kit { &mut to_reader } else { &mut to_table };
                            if c {
                                *primary = Some(t.r);
                            }
                            if s && kit {
                                if v.send == Send::Reader { to_table = Some(t.r) } else { to_reader = Some(t.r) }
                            }
                            if d {
                                v.drag = Some(t.r);
                            }
                        }
                    }
                });
            });
            if v.notes > 0 {
                ui.add_space(2.0);
                egui::Frame::NONE.fill(theme::rgb(0x1a1608)).stroke(Stroke::new(1.0, GOLD.gamma_multiply(0.4))).corner_radius(CornerRadius::same(3)).inner_margin(Margin::symmetric(10, 6)).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new("GRANDPA'S NOTEBOOK").font(retro(14.0)).color(GOLD));
                    for n in notes.iter().take(v.notes) {
                        ui.add(egui::Label::new(RichText::new(*n).font(mono(12.5)).color(theme::rgb(0xe6d9a8))).wrap());
                    }
                });
            }
        });
    });

    // a card in the air
    if let Some(r) = v.drag {
        let (pos, released) = ctx.input(|i| (i.pointer.interact_pos(), i.pointer.any_released() || !i.pointer.any_down()));
        if let Some(pos) = pos {
            let bits = sig.bits_of(st, r);
            let layer = egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("card-drag"));
            let p = ctx.layer_painter(layer);
            let rect = Rect::from_center_size(pos, Vec2::splat(44.0));
            p.rect_filled(rect.expand(3.0), 3.0, theme::rgb(0x1b2440));
            paint_bits(&p, rect, bits, theme::rgb(0xe6e2c3), theme::rgb(0x1a2033), false);
            if released {
                if readers_rect.contains(pos) {
                    to_reader = Some(r);
                } else if table_rect.contains(pos) && kit {
                    to_table = Some(r);
                }
            }
        }
        if released {
            v.drag = None;
        }
        ctx.request_repaint();
    }

    // --- apply ------------------------------------------------------------------------
    for (x, y) in toggles {
        if sig.toggle(st, id, x, y) {
            sfx.push("tick");
        }
    }
    if let Some(r) = to_reader {
        if sig.slot(st, id, r) {
            sfx.push("confirm");
        } else {
            sfx.push("error");
        }
    }
    if let Some(r) = to_table {
        if let Some(i) = v.table.iter().position(|c| *c == r) {
            v.table.remove(i);
        } else if v.table.len() < 5 {
            v.table.push(r);
        }
        sfx.push("blip");
    }
    if let Some(i) = unslot {
        sig.unslot(st, id, i);
        sfx.push("blip");
    }
    if clear_board {
        sig.clear(st, id);
        sfx.push("blip");
    }
    if rub && sig.copy(st, id).is_some() {
        sfx.push("pickup");
    }
    if cut && sig.cut(st, &v.table).is_some() {
        v.table.clear();
        sfx.push("pickup");
    }
    if let Some(cid) = discard {
        v.table.retain(|c| *c != CardRef::Own(cid));
        sig.discard(st, cid);
        sfx.push("blip");
    }
    if pull {
        let o = sig.pull(st, id);
        sfx.push("lever");
        after_pull(v, o, sfx);
    }
    sig.note_pads(st);

    if ctx.input(|i| i.key_pressed(Key::Escape)) && v.drag.is_none() {
        out.close = true;
    }
    ctx.request_repaint();
    out
}
