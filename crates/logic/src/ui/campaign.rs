//! The campaign map: the whole city as stages joined by roads, the stars
//! each stage has given you, and the upgrade tree the stars buy. Picking a
//! stage takes you there; a finished district can be replayed on fresh pads.

use egui::{Align2, Color32, CornerRadius, Key, Margin, Pos2, Rect, RichText, Sense, Shape, Stroke, Vec2};

use super::theme::{self, AMBER, CYAN, DIM, FAINT, GOLD, GREEN, LINE, LINE2, PANEL, TEXT, mono, retro};
use super::widgets::{Kind, button, button_enabled};
use crate::campaign::{self, NODES, Node, NodeKind, UPGRADES};
use crate::maps::MapDef;
use crate::state::{GameState, MapId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Map,
    Upgrades,
}

pub struct CampaignView {
    /// index into [`NODES`]
    pub cursor: usize,
    pub tab: Tab,
    /// the upgrade line under the cursor
    pub line: usize,
    pub age: f32,
    tex: Option<egui::TextureHandle>,
    /// opened at a lantern: the only place stars are spent
    pub at_rest: bool,
}

impl CampaignView {
    pub fn new(st: &GameState) -> Self {
        // start on the stage the story is waiting at: the last open node not yet done
        let cursor = NODES.iter().rposition(|n| campaign::open(st, n) && !st.flag(n.done) && !matches!(n.kind, NodeKind::Dive { .. })).or_else(|| NODES.iter().rposition(|n| campaign::open(st, n))).unwrap_or(0);
        Self { cursor, tab: Tab::Map, line: 0, age: 0.0, tex: None, at_rest: false }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum CampaignAction {
    None,
    Close,
    /// go to the node (story travel, or into a district)
    Go(&'static str),
    /// replay a finished node: a district on fresh pads, a quarter as a gauntlet
    Replay(&'static str),
    /// out of the replay, back to the real save
    LeaveReplay,
    Buy(&'static str),
    Refund,
}

/// What pressing Go on a node does, in words (None: it cannot be entered now).
pub fn verb(st: &GameState, n: &Node, here: MapId) -> Option<&'static str> {
    if st.data.replay_of.is_some() || !campaign::open(st, n) {
        return None;
    }
    match n.kind {
        NodeKind::District(d) | NodeKind::Dive { district: d, .. } | NodeKind::Boss { district: d, .. } => {
            if campaign::replayable(st, d) {
                Some("Replay ▸")
            } else if here == d {
                Some("You are here")
            } else {
                Some("Go ▸")
            }
        }
        NodeKind::Cabin => (here != MapId::Cabin).then_some("Go ▸"),
        NodeKind::City { .. } => Some("Go ▸"),
    }
}

fn stars_text(n: u8, of: u8) -> String {
    (0..of).map(|i| if i < n { '★' } else { '☆' }).collect()
}

fn draw_star_row(p: &egui::Painter, at: Pos2, n: u8, of: u8, size: f32) {
    if of == 0 {
        return;
    }
    let w = size * 0.95;
    let x0 = at.x - w * (of as f32 - 1.0) / 2.0;
    for i in 0..of {
        let c = if i < n { GOLD } else { FAINT.gamma_multiply(0.7) };
        p.text(Pos2::new(x0 + i as f32 * w, at.y), Align2::CENTER_CENTER, if i < n { "★" } else { "☆" }, mono(size), c);
    }
}

/// Draw the campaign map. `city` is the city's map (its tiles are the backdrop); `here` is where the player is.
pub fn show(ctx: &egui::Context, v: &mut CampaignView, st: &GameState, city: &MapDef, here: MapId, dt: f32, sfx: &mut Vec<&'static str>) -> CampaignAction {
    v.age += dt;
    let mut action = CampaignAction::None;
    let tex = v.tex.get_or_insert_with(|| super::citymap::minimap(ctx, city)).clone();
    let replaying = st.data.replay_of.is_some();
    let (left, right, up, down, enter, tab, close) = ctx.input(|i| {
        (
            i.key_pressed(Key::ArrowLeft) || i.key_pressed(Key::A),
            i.key_pressed(Key::ArrowRight) || i.key_pressed(Key::D),
            i.key_pressed(Key::ArrowUp) || i.key_pressed(Key::W),
            i.key_pressed(Key::ArrowDown) || i.key_pressed(Key::S),
            i.key_pressed(Key::Enter) || i.key_pressed(Key::E),
            i.key_pressed(Key::Tab) || i.key_pressed(Key::U),
            i.key_pressed(Key::Escape) || (i.key_pressed(Key::M) && v.age > 0.2),
        )
    });
    if tab {
        v.tab = if v.tab == Tab::Map { Tab::Upgrades } else { Tab::Map };
        sfx.push("blip");
    }
    // the cursor walks the open nodes in story order
    let open: Vec<usize> = (0..NODES.len()).filter(|&i| campaign::open(st, &NODES[i])).collect();
    if v.tab == Tab::Map && !open.is_empty() && (left || right || up || down) {
        let k = open.iter().position(|&i| i == v.cursor).unwrap_or(0);
        let k = if right || down { (k + 1) % open.len() } else { (k + open.len() - 1) % open.len() };
        v.cursor = open[k];
        sfx.push("blip");
    }
    if v.tab == Tab::Upgrades && (left || right) {
        v.line = if right { (v.line + 1) % UPGRADES.len() } else { (v.line + UPGRADES.len() - 1) % UPGRADES.len() };
        sfx.push("blip");
    }

    let screen = ctx.content_rect();
    let frame = egui::Frame::NONE.fill(PANEL).stroke(Stroke::new(1.0, LINE2)).corner_radius(CornerRadius::same(6)).inner_margin(Margin::same(14));
    egui::Modal::new(egui::Id::new("campaign-map")).frame(frame).backdrop_color(Color32::from_black_alpha(170)).show(ctx, |ui| {
        let width = (screen.width() - 40.0).min(1180.0);
        ui.set_width(width);
        ui.set_max_height(screen.height() - 40.0);
        // --- header -----------------------------------------------------------------
        ui.horizontal(|ui| {
            ui.label(RichText::new("CRYPTOPOLIS").font(retro(30.0)).color(CYAN));
            ui.add_space(8.0);
            for (t, label) in [(Tab::Map, "MAP"), (Tab::Upgrades, "UPGRADES")] {
                let sel = v.tab == t;
                if ui.add(egui::Button::new(RichText::new(label).font(retro(18.0)).color(if sel { GOLD } else { DIM })).fill(if sel { theme::rgb(0x1a2440) } else { PANEL }).stroke(Stroke::new(1.0, if sel { GOLD } else { LINE }))).clicked() && !sel {
                    v.tab = t;
                    sfx.push("blip");
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if button(ui, "M ✕", Kind::Ghost).clicked() {
                    action = CampaignAction::Close;
                }
                let free = campaign::stars_free(st);
                ui.label(RichText::new(format!("{free} to spend")).font(mono(13.0)).color(if free > 0 { GOLD } else { DIM }));
                ui.label(RichText::new(format!("★ {} / {}", campaign::stars_earned(st), campaign::stars_possible())).font(retro(22.0)).color(GOLD));
            });
        });
        if replaying {
            egui::Frame::NONE.fill(theme::rgb(0x2a1d08)).stroke(Stroke::new(1.0, AMBER)).corner_radius(CornerRadius::same(3)).inner_margin(Margin::symmetric(10, 6)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let name = campaign::node(campaign::stage_id(here)).map(|n| n.name).unwrap_or("a district");
                    ui.label(RichText::new(format!("REPLAYING {}", name.to_uppercase())).font(retro(18.0)).color(AMBER));
                    ui.label(RichText::new("fresh pads; only the stars come home with you").font(mono(12.0)).color(DIM));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if button(ui, "Leave the replay", Kind::Primary).clicked() {
                            action = CampaignAction::LeaveReplay;
                        }
                    });
                });
            });
        }
        ui.add_space(6.0);
        match v.tab {
            Tab::Map => map_tab(ui, v, st, &tex, city, here, width, screen, &mut action, sfx),
            Tab::Upgrades => upgrades_tab(ui, v, st, width, &mut action, sfx),
        }
        ui.add_space(4.0);
        let help = match v.tab {
            Tab::Map => "← → choose   ·   Enter: go   ·   Tab: upgrades   ·   M: close",
            Tab::Upgrades => "← → choose a line   ·   Enter: buy the next level   ·   Tab: map   ·   M: close",
        };
        ui.label(RichText::new(help).font(mono(12.0)).color(FAINT));
    });
    if enter && action == CampaignAction::None {
        match v.tab {
            Tab::Map => {
                let n = &NODES[v.cursor];
                match verb(st, n, here) {
                    Some("You are here") => action = CampaignAction::Close,
                    Some(_) => action = go_action(st, n),
                    None => sfx.push("error"),
                }
            }
            Tab::Upgrades => action = CampaignAction::Buy(UPGRADES[v.line].id),
        }
    }
    if close && action == CampaignAction::None {
        action = CampaignAction::Close;
    }
    action
}

fn go_action(st: &GameState, n: &Node) -> CampaignAction {
    match n.kind {
        NodeKind::District(d) | NodeKind::Dive { district: d, .. } | NodeKind::Boss { district: d, .. } if campaign::replayable(st, d) => CampaignAction::Replay(n.id),
        _ => CampaignAction::Go(n.id),
    }
}

#[allow(clippy::too_many_arguments)]
fn map_tab(ui: &mut egui::Ui, v: &mut CampaignView, st: &GameState, tex: &egui::TextureHandle, city: &MapDef, here: MapId, width: f32, screen: Rect, action: &mut CampaignAction, sfx: &mut Vec<&'static str>) {
    let side = 300.0;
    let avail_h = (screen.height() - 190.0).max(260.0);
    let map_w = (width - side - 16.0).max(300.0);
    let aspect = city.h as f32 / city.w as f32;
    // a band of sea under the city for the harbour
    let (w, h) = if map_w * aspect * 1.12 > avail_h { (avail_h / aspect / 1.12, avail_h) } else { (map_w, map_w * aspect * 1.12) };
    ui.horizontal_top(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(w, h), Sense::hover());
        let p = ui.painter_at(rect);
        p.rect_filled(rect, 3.0, theme::rgb(0x0c1a2c));
        let city_rect = Rect::from_min_size(rect.min, Vec2::new(w, w * aspect));
        p.image(tex.id(), city_rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::from_gray(120));
        // sea lines
        for k in 0..4 {
            let y = city_rect.max.y + 6.0 + k as f32 * 7.0;
            let x = rect.min.x + ((v.age * 6.0 + k as f32 * 37.0) % 60.0);
            let mut xx = x;
            while xx < rect.max.x {
                p.line_segment([Pos2::new(xx, y), Pos2::new((xx + 14.0).min(rect.max.x), y)], Stroke::new(1.0, theme::rgb(0x1f4468)));
                xx += 60.0;
            }
        }
        p.rect_stroke(rect, 3.0, Stroke::new(1.0, LINE2), egui::StrokeKind::Outside);
        let at = |n: &Node| rect.min + Vec2::new(n.at.0 * w, n.at.1 * h);
        // roads
        for n in NODES {
            let Some(from) = n.from.and_then(campaign::node) else { continue };
            let (a, b) = (at(from), at(n));
            let lit = campaign::open(st, n);
            let c = if lit { GOLD.gamma_multiply(0.75) } else { FAINT.gamma_multiply(0.5) };
            let dash = if matches!(n.kind, NodeKind::Dive { .. }) { 3.0 } else { 7.0 };
            p.extend(Shape::dashed_line(&[a, b], Stroke::new(if lit { 2.0 } else { 1.0 }, c), dash, 4.0));
        }
        // nodes
        let pointer = ui.ctx().pointer_hover_pos();
        let clicked = ui.ctx().input(|i| i.pointer.primary_clicked());
        for (i, n) in NODES.iter().enumerate() {
            let c0 = at(n);
            let open = campaign::open(st, n);
            let done = st.flag(n.done);
            let sel = i == v.cursor;
            let base = match n.kind {
                NodeKind::District(_) => 17.0,
                NodeKind::Dive { .. } => 9.0,
                NodeKind::Boss { .. } => 13.0,
                _ => 12.0,
            };
            let pulse = if open && !done && !st.settings.reduced_motion { (v.age * 3.0).sin().abs() * 2.5 } else { 0.0 };
            let r = base + pulse + if sel { 2.0 } else { 0.0 };
            let col = if !open {
                FAINT
            } else if done {
                GREEN
            } else {
                GOLD
            };
            match n.kind {
                NodeKind::Boss { .. } => {
                    // a lamp: a diamond with a ring of light
                    let pts = vec![c0 + Vec2::new(0.0, -r), c0 + Vec2::new(r, 0.0), c0 + Vec2::new(0.0, r), c0 + Vec2::new(-r, 0.0)];
                    p.add(Shape::convex_polygon(pts, theme::rgb(0x1a0812), Stroke::new(if sel { 2.5 } else { 1.8 }, if open { theme::MAGENTA } else { col })));
                    p.circle_stroke(c0, r + 3.0, Stroke::new(1.0, col.gamma_multiply(0.5)));
                }
                NodeKind::Dive { .. } => {
                    let pts = vec![c0 + Vec2::new(0.0, -r), c0 + Vec2::new(r, 0.0), c0 + Vec2::new(0.0, r), c0 + Vec2::new(-r, 0.0)];
                    p.add(Shape::convex_polygon(pts, theme::rgb(0x0b0d16), Stroke::new(if sel { 2.5 } else { 1.5 }, col)));
                }
                _ => {
                    p.circle_filled(c0, r, theme::rgb(0x0b0d16));
                    p.circle_stroke(c0, r, Stroke::new(if sel { 2.5 } else { 1.5 }, col));
                    if matches!(n.kind, NodeKind::District(_)) {
                        p.circle_stroke(c0, r - 4.0, Stroke::new(1.0, col.gamma_multiply(0.5)));
                    }
                }
            }
            let mark = match n.kind {
                NodeKind::Dive { .. } => "◆".to_string(),
                NodeKind::Boss { .. } => "☼".to_string(),
                _ => n.chapter.to_string(),
            };
            p.text(c0, Align2::CENTER_CENTER, mark, if matches!(n.kind, NodeKind::Dive { .. }) { mono(9.0) } else { retro(r * 1.1) }, col);
            if sel {
                p.circle_stroke(c0, r + 5.0, Stroke::new(1.0, CYAN.gamma_multiply(0.7)));
            }
            let max = campaign::max_stars(n);
            if open && max > 0 {
                draw_star_row(&p, c0 + Vec2::new(0.0, r + 9.0), campaign::stars(st, n.id), max, if max == 1 { 10.0 } else { 12.0 });
            }
            if !matches!(n.kind, NodeKind::Dive { .. }) || sel {
                let label_y = r + if open && max > 0 { 22.0 } else { 10.0 };
                p.text(c0 + Vec2::new(0.0, label_y), Align2::CENTER_CENTER, n.name, mono(11.0), if open { TEXT } else { FAINT });
            }
            if let Some(pp) = pointer
                && pp.distance(c0) <= r + 6.0
                && rect.contains(pp)
                && clicked
            {
                // first click selects, second goes
                if v.cursor == i {
                    match verb(st, n, here) {
                        Some("You are here") => *action = CampaignAction::Close,
                        Some(_) => *action = go_action(st, n),
                        None => sfx.push("error"),
                    }
                } else {
                    v.cursor = i;
                    sfx.push("blip");
                }
            }
        }
        // a cat on the stage you are on
        if let Some(n) = NODES.iter().find(|n| match n.kind {
            NodeKind::District(d) => d == here,
            NodeKind::Cabin => here == MapId::Cabin,
            _ => false,
        }) {
            p.text(at(n) + Vec2::new(-22.0, -20.0), Align2::CENTER_CENTER, "YOU", mono(10.0), CYAN);
        }
        ui.add_space(12.0);
        // --- the selected node ------------------------------------------------------------
        ui.vertical(|ui| {
            ui.set_width(side);
            let n = &NODES[v.cursor];
            let open = campaign::open(st, n);
            let chapter = if n.chapter == "P" { "Prologue".to_string() } else { format!("Chapter {}", n.chapter) };
            ui.label(
                RichText::new(match n.kind {
                    NodeKind::District(_) => format!("{chapter} · district"),
                    NodeKind::Dive { .. } => format!("{chapter} · dive"),
                    NodeKind::Boss { .. } => format!("{chapter} · boss"),
                    _ => format!("{chapter} · the city"),
                })
                .font(mono(12.0))
                .color(DIM),
            );
            ui.label(RichText::new(n.name).font(retro(30.0)).color(if open { GOLD } else { FAINT }));
            ui.add(egui::Label::new(RichText::new(n.era).font(mono(12.0)).color(CYAN)).wrap());
            ui.add_space(8.0);
            if !open {
                ui.add(egui::Label::new(RichText::new("Not yet. The story opens this road.").font(mono(13.0)).color(AMBER)).wrap());
            } else {
                let best = campaign::stars(st, n.id);
                match n.kind {
                    NodeKind::District(d) => {
                        ui.label(RichText::new(stars_text(best, 3)).font(retro(30.0)).color(GOLD));
                        let s = campaign::score(st, d);
                        let live = !st.flag(campaign::goal(d).0) && st.data.scene == d;
                        for (k, (what, ok)) in s.labels.iter().copied().zip(s.ok).enumerate() {
                            let mark = if best as usize > k { "★" } else { "☆" };
                            let tail = match (live, k, ok) {
                                (false, ..) => "",
                                (true, 0, _) => "  (in progress)",
                                (true, _, true) => "  (so far)",
                                (true, _, false) => "  (lost this run)",
                            };
                            ui.label(RichText::new(format!("{mark} {what}{tail}")).font(mono(12.5)).color(if best as usize > k { GOLD } else { DIM }));
                        }
                        let rec = st.data.stages.get(n.id);
                        if let Some(r) = rec.filter(|r| r.clears > 0) {
                            ui.label(RichText::new(format!("cleared {}×{}", r.clears, if r.replays > 0 { format!(", {} replays", r.replays) } else { String::new() })).font(mono(11.5)).color(FAINT));
                        }
                        if campaign::replayable(st, d) {
                            ui.add_space(4.0);
                            ui.add(egui::Label::new(RichText::new("Finished. A replay runs on fresh pads in a sandbox: the story is untouched, and only better stars come back.").font(mono(12.0)).color(DIM)).wrap());
                        }
                    }
                    NodeKind::Boss { .. } => {
                        ui.label(RichText::new(stars_text(best, 3)).font(retro(30.0)).color(GOLD));
                        for (k, what) in campaign::BOSS_LABELS.iter().enumerate() {
                            let got = best as usize > k;
                            ui.label(RichText::new(format!("{} {what}", if got { "★" } else { "☆" })).font(mono(12.5)).color(if got { GOLD } else { DIM }));
                        }
                        ui.add_space(4.0);
                        ui.add(egui::Label::new(RichText::new("The Directorate's sweeper drone, back for the mast. Its floodlight leaves one lane dark each beat, chosen by a shift register. Dodge to live; predict to win.").font(mono(12.0)).color(DIM)).wrap());
                    }
                    NodeKind::Dive { district, .. } => {
                        ui.label(RichText::new(stars_text(best, 1)).font(retro(26.0)).color(GOLD));
                        let inside = campaign::node(campaign::stage_id(district)).map(|x| x.name).unwrap_or("");
                        ui.add(egui::Label::new(RichText::new(format!("A cyberspace dive inside {inside}. Break it once in a run for its star.")).font(mono(12.5)).color(DIM)).wrap());
                    }
                    NodeKind::City { .. } | NodeKind::Cabin => {
                        ui.label(RichText::new(stars_text(best, 3)).font(retro(30.0)).color(GOLD));
                        let done = st.flag(n.done);
                        for (k, what) in campaign::CITY_LABELS.iter().enumerate() {
                            let got = best as usize > k;
                            ui.add(egui::Label::new(RichText::new(format!("{} {what}", if got { "★" } else { "☆" })).font(mono(12.5)).color(if got { GOLD } else { DIM })).wrap());
                        }
                        let ts = campaign::terminals(n.id);
                        ui.add(egui::Label::new(RichText::new(format!("{} terminals: {}", ts.len(), ts.join(" · "))).font(mono(11.5)).color(FAINT)).wrap());
                        ui.add_space(4.0);
                        let line = if done {
                            "Its chapter is behind you. A gauntlet runs these terminals back to back on fresh numbers, in a sandbox: leave one unsolved and the run ends; only better stars come back."
                        } else if n.id == "cabin" {
                            "Grandpa's cabin, and the gateway that brought you here."
                        } else {
                            "The story continues here. Its stars are scored when the chapter closes."
                        };
                        ui.add(egui::Label::new(RichText::new(line).font(mono(12.0)).color(DIM)).wrap());
                    }
                }
            }
            ui.add_space(10.0);
            let verb_now = verb(st, n, here);
            if replaying_banner(st) {
                ui.add(egui::Label::new(RichText::new("Leave the replay to travel.").font(mono(12.0)).color(AMBER)).wrap());
            } else if let Some(word) = verb_now {
                if matches!(n.kind, NodeKind::City { .. } | NodeKind::Cabin) && campaign::can_replay(st, n) && button(ui, "Gauntlet ▸", Kind::Primary).on_hover_text("The quarter's terminals back to back, fresh numbers").clicked() {
                    *action = CampaignAction::Replay(n.id);
                }
                let enabled = word != "You are here";
                if button_enabled(ui, enabled, word, Kind::Primary).clicked() {
                    *action = go_action(st, n);
                }
            }
        });
    });
}

fn replaying_banner(st: &GameState) -> bool {
    st.data.replay_of.is_some()
}

fn upgrades_tab(ui: &mut egui::Ui, v: &mut CampaignView, st: &GameState, width: f32, action: &mut CampaignAction, sfx: &mut Vec<&'static str>) {
    let replaying = st.data.replay_of.is_some();
    // stars are spent at a lantern, like a bonfire, and nowhere else
    let shop = v.at_rest && !replaying;
    let free = campaign::stars_free(st);
    ui.add(egui::Label::new(RichText::new("Stars buy kit. Kit makes the street kinder; it never reads a lamp, turns a knob or finds a key for you. Bought and refunded at a lantern.").font(mono(12.5)).color(DIM)).wrap());
    ui.add_space(8.0);
    let col_w = ((width - 3.0 * 12.0) / UPGRADES.len() as f32).floor();
    ui.horizontal_top(|ui| {
        for (li, u) in UPGRADES.iter().enumerate() {
            let have = campaign::level(st, u.id);
            let sel = li == v.line;
            let frame = egui::Frame::NONE.fill(if sel { theme::rgb(0x111c32) } else { theme::rgb(0x0d1220) }).stroke(Stroke::new(1.0, if sel { CYAN } else { LINE })).corner_radius(CornerRadius::same(4)).inner_margin(Margin::same(10));
            let resp = frame.show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.set_width(col_w - 20.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new(u.name.to_uppercase()).font(retro(20.0)).color(if have > 0 { GOLD } else { TEXT }));
                        ui.label(RichText::new(format!("{have} / {}", u.levels.len())).font(mono(11.5)).color(DIM));
                    });
                    ui.add_space(6.0);
                    // the highest level on top, like a tower
                    for k in (0..u.levels.len()).rev() {
                        let lvl = k as u8 + 1;
                        let (name, what) = u.levels[k];
                        let owned = lvl <= have;
                        let next = lvl == have + 1;
                        let afford = free >= u.cost(lvl) as u32;
                        let (fill, stroke, fg) = if owned {
                            (theme::rgb(0x2a2410), GOLD, GOLD)
                        } else if next && afford && shop {
                            (theme::rgb(0x10243a), CYAN, TEXT)
                        } else {
                            (theme::rgb(0x0b0d16), LINE, FAINT)
                        };
                        let box_resp = egui::Frame::NONE.fill(fill).stroke(Stroke::new(if next && sel { 2.0 } else { 1.0 }, stroke)).corner_radius(CornerRadius::same(3)).inner_margin(Margin::symmetric(8, 6)).show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(name).font(retro(17.0)).color(fg));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let tag = if owned { "✓".to_string() } else { format!("★{}", u.cost(lvl)) };
                                    ui.label(RichText::new(tag).font(mono(12.0)).color(if owned {
                                        GREEN
                                    } else if afford {
                                        GOLD
                                    } else {
                                        FAINT
                                    }));
                                });
                            });
                            ui.add(egui::Label::new(RichText::new(what).font(mono(11.5)).color(if owned { TEXT } else { DIM })).wrap());
                        });
                        let r = box_resp.response.interact(Sense::click());
                        if r.clicked() {
                            v.line = li;
                            if next && shop {
                                *action = CampaignAction::Buy(u.id);
                            } else if !owned {
                                sfx.push("error");
                            }
                        }
                        ui.add_space(4.0);
                    }
                });
            });
            if resp.response.hovered() && ui.ctx().input(|i| i.pointer.delta() != Vec2::ZERO) {
                v.line = li;
            }
            if li + 1 < UPGRADES.len() {
                ui.add_space(12.0 - 8.0);
            }
        }
    });
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if replaying {
            ui.label(RichText::new("Kit is bought on the map, not in the middle of a replay.").font(mono(12.0)).color(AMBER));
        } else if !v.at_rest {
            ui.label(RichText::new("Rest at a lantern (a safe house, or the start of a district) to spend stars.").font(mono(12.0)).color(AMBER));
        } else if button_enabled(ui, campaign::stars_spent(st) > 0, "Refund all stars", Kind::Ghost).clicked() {
            *action = CampaignAction::Refund;
        }
    });
}
