//! The dialogue box: a typewriter for lines, a list for choices.
//! Space / Enter / E / click completes the line, then advances.

use egui::{Align2, Color32, CornerRadius, Event, Key, Margin, RichText, Sense, Stroke};

use super::markup::{self, Look};
use super::portraits::Portraits;
use super::theme::{self, CYAN, LINE2, PANEL, mono, retro};
use crate::script::Reply;
use crate::speakers::{Sp, Style};

pub enum DialogueKind {
    Lines { lines: Vec<(Sp, String)>, idx: usize, shown: f32 },
    Choice { sp: Sp, question: String, options: Vec<String>, sel: usize },
}

pub struct DialogueItem {
    pub reply: u64,
    pub kind: DialogueKind,
    typed: usize,
    /// Frames this item has been on screen (the key press that opened it must not also advance it).
    frames: u32,
}

impl DialogueItem {
    pub fn lines(reply: u64, lines: Vec<(Sp, String)>) -> Self {
        Self { reply, kind: DialogueKind::Lines { lines, idx: 0, shown: 0.0 }, typed: 0, frames: 0 }
    }

    pub fn choice(reply: u64, sp: Sp, question: String, options: Vec<String>) -> Self {
        Self { reply, kind: DialogueKind::Choice { sp, question, options, sel: 0 }, typed: 0, frames: 0 }
    }

    /// The text currently on screen (tests and debugging).
    pub fn current_text(&self) -> String {
        match &self.kind {
            DialogueKind::Lines { lines, idx, .. } => lines.get(*idx).map(|l| l.1.clone()).unwrap_or_default(),
            DialogueKind::Choice { question, .. } => question.clone(),
        }
    }
}

pub struct DialogueEnv<'a> {
    pub handle: &'a str,
    pub look: usize,
    /// characters per second, 0 = instant
    pub cps: f32,
    pub dt: f32,
    pub time: f64,
    pub portraits: &'a mut Portraits,
    pub sfx: &'a mut Vec<&'static str>,
}

/// Keys that advance dialogue, pressed this frame (ignoring key repeat).
pub fn advance_pressed(ctx: &egui::Context) -> bool {
    ctx.input(|i| {
        i.events.iter().any(|e| matches!(e, Event::Key { key: Key::Space | Key::Enter | Key::E, pressed: true, repeat: false, .. }))
    })
}

fn key_pressed(ctx: &egui::Context, key: Key) -> bool {
    ctx.input(|i| i.events.iter().any(|e| matches!(e, Event::Key { key: k, pressed: true, repeat: false, .. } if *k == key)))
}

/// Draw the dialogue item; returns the reply once it is finished.
pub fn show(ctx: &egui::Context, item: &mut DialogueItem, env: DialogueEnv) -> Option<Reply> {
    let screen = ctx.content_rect();
    let width = (screen.width() - 24.0).min(900.0);
    item.frames += 1;
    let fresh = item.frames <= 1;
    let advance = !fresh && advance_pressed(ctx);
    let (sp, text_owned, is_choice) = match &item.kind {
        DialogueKind::Lines { lines, idx, .. } => (lines[*idx].0, lines[*idx].1.clone(), false),
        DialogueKind::Choice { sp, question, .. } => (*sp, question.clone(), true),
    };
    let info = sp.info(env.handle, env.look);
    let (bg, border) = match info.style {
        Style::Letter => (theme::rgb(0x241d12), theme::rgb(0x6b5530)),
        Style::Narration => (Color32::from_rgba_premultiplied(8, 10, 18, 235), LINE2),
        _ => (PANEL, LINE2),
    };
    let text_color = match info.style {
        Style::Terminal => theme::rgb(0xbff8ff),
        Style::Narration => theme::rgb(0xb8c6dc),
        Style::Letter => theme::rgb(0xf3e6c8),
        Style::Normal => theme::TEXT,
    };
    let family = match info.style {
        Style::Narration => Some(theme::italic_family()),
        _ => None,
    };
    let total = markup::visible_len(&text_owned);

    // typewriter
    let mut finished = true;
    if let DialogueKind::Lines { shown, .. } = &mut item.kind {
        if env.cps <= 0.0 {
            *shown = total as f32;
        } else {
            *shown = (*shown + env.dt * env.cps).min(total as f32);
        }
        let now = *shown as usize;
        let plain: Vec<char> = markup::plain(&text_owned).chars().collect();
        while item.typed < now {
            if item.typed % 3 == 0 && plain.get(item.typed).is_some_and(|c| *c != ' ') {
                env.sfx.push("type");
            }
            item.typed += 1;
        }
        finished = now >= total;
    }

    let mut clicked_box = false;
    let mut picked: Option<usize> = None;
    let area = egui::Area::new(egui::Id::new("dialogue")).order(egui::Order::Foreground).anchor(Align2::CENTER_BOTTOM, egui::vec2(0.0, -18.0));
    area.show(ctx, |ui| {
        let frame = egui::Frame::NONE.fill(bg).stroke(Stroke::new(1.0, border)).corner_radius(CornerRadius::same(6)).inner_margin(Margin { left: 16, right: 16, top: 14, bottom: 16 });
        let resp = frame.show(ui, |ui| {
            ui.set_width((width - 32.0).max(120.0));
            ui.horizontal_top(|ui| {
                if let Some(id) = &info.portrait
                    && let Some(tex) = env.portraits.portrait(ui.ctx(), id)
                {
                    let size = egui::vec2(80.0, 75.0);
                    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
                    ui.painter().rect_filled(rect, 4.0, theme::rgb(0x182240));
                    ui.painter().rect_stroke(rect, 4.0, Stroke::new(1.0, LINE2), egui::StrokeKind::Inside);
                    let img_rect = egui::Rect::from_center_size(rect.center(), egui::vec2(80.0, 75.0) * 0.94);
                    ui.painter().image(tex.id(), img_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
                    ui.add_space(6.0);
                }
                ui.vertical(|ui| {
                    if !info.name.is_empty() {
                        ui.label(RichText::new(&info.name).font(retro(22.0)).color(theme::color(info.color)));
                        ui.add_space(2.0);
                    }
                    let limit = match &item.kind {
                        DialogueKind::Lines { shown, .. } => Some(*shown as usize),
                        _ => None,
                    };
                    let size = if info.style == Style::Letter { 16.0 } else { 15.5 };
                    let job = markup::job(&text_owned, Look { size, color: text_color, family: family.clone() }, ui.available_width(), limit);
                    // reserve height for the full text so the box does not grow while typing
                    let full = markup::job(&text_owned, Look { size, color: text_color, family: family.clone() }, ui.available_width(), None);
                    let full_h = ui.fonts_mut(|f| f.layout_job(full).size().y).max(size * 3.1);
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), full_h), Sense::hover());
                    let galley = ui.fonts_mut(|f| f.layout_job(job));
                    ui.painter().galley(rect.min, galley, text_color);
                    if let DialogueKind::Choice { options, sel, .. } = &mut item.kind {
                        ui.add_space(8.0);
                        for (i, o) in options.iter().enumerate() {
                            let selected = i == *sel;
                            let label = RichText::new(format!("{}  {}", i + 1, o)).font(mono(14.5)).color(if selected { CYAN } else { theme::TEXT });
                            let b = egui::Button::new(label)
                                .fill(if selected { theme::rgb(0x1b2a4d) } else { theme::rgb(0x121a2e) })
                                .stroke(Stroke::new(1.0, if selected { CYAN } else { LINE2 }))
                                .min_size(egui::vec2(ui.available_width(), 28.0));
                            let r = ui.add(b);
                            if r.hovered() {
                                *sel = i;
                            }
                            if r.clicked() {
                                picked = Some(i);
                            }
                        }
                    }
                });
            });
        });
        let r = resp.response.interact(Sense::click());
        if r.clicked() && !is_choice {
            clicked_box = true;
        }
        if finished && !is_choice {
            let blink = (env.time * 2.0).fract() < 0.5;
            if blink {
                let p = resp.response.rect.right_bottom() + egui::vec2(-20.0, -14.0);
                ui.painter().text(p, Align2::CENTER_CENTER, "▼", mono(12.0), CYAN);
            }
        }
    });

    match &mut item.kind {
        DialogueKind::Lines { lines, idx, shown } => {
            if advance || clicked_box {
                if !finished {
                    *shown = total as f32;
                    item.typed = total;
                } else if *idx + 1 < lines.len() {
                    *idx += 1;
                    *shown = 0.0;
                    item.typed = 0;
                    env.sfx.push("blip");
                } else {
                    env.sfx.push("blip");
                    return Some(Reply::Done);
                }
            }
            None
        }
        DialogueKind::Choice { options, sel, .. } => {
            let n = options.len();
            for k in 0..n.min(9) {
                let key = [Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5, Key::Num6, Key::Num7, Key::Num8, Key::Num9][k];
                if !fresh && key_pressed(ctx, key) {
                    picked = Some(k);
                }
            }
            if key_pressed(ctx, Key::ArrowDown) || key_pressed(ctx, Key::S) {
                *sel = (*sel + 1) % n;
                env.sfx.push("blip");
            }
            if key_pressed(ctx, Key::ArrowUp) || key_pressed(ctx, Key::W) {
                *sel = (*sel + n - 1) % n;
                env.sfx.push("blip");
            }
            if advance {
                picked = Some(*sel);
            }
            picked.map(|i| {
                env.sfx.push("select");
                Reply::Choice(i)
            })
        }
    }
}
