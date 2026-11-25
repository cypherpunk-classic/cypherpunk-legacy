//! The window a cyberspace dive plays in: the field, its level and brief, a
//! trace bar, and the player's mouse and keys turned into dive input.

use egui::{Align, Align2, Color32, CornerRadius, Key, Layout, Margin, Pos2, Rect, RichText, Sense, Stroke, Vec2};

use super::theme::{self, AMBER, CYAN, DIM, FAINT, GOLD, LINE2, RED, mono, retro};
use super::widgets::{Kind, button};
use crate::district::DiveView;
use crate::dive::{Dive, Input, View};

/// Draw the dive and collect this frame's input into `input`. Returns true
/// when the player steps out.
pub fn show(ctx: &egui::Context, v: &mut DiveView, d: &dyn Dive, input: &mut Input, auto: bool) -> bool {
    let mut close = false;
    // the dive owns the keyboard: take its keys before any button can (Tab
    // would move focus, Space would press the focused button)
    if !auto {
        ctx.input_mut(|i| {
            let shift = i.modifiers.shift;
            let m = if shift { egui::Modifiers::SHIFT } else { egui::Modifiers::NONE };
            input.fine = shift;
            input.left |= i.consume_key(m, Key::ArrowLeft) || i.consume_key(egui::Modifiers::NONE, Key::A);
            input.right |= i.consume_key(m, Key::ArrowRight) || i.consume_key(egui::Modifiers::NONE, Key::D);
            input.up |= i.consume_key(m, Key::ArrowUp) || i.consume_key(egui::Modifiers::NONE, Key::W);
            input.down_key |= i.consume_key(m, Key::ArrowDown) || i.consume_key(egui::Modifiers::NONE, Key::S);
            input.fire |= i.consume_key(egui::Modifiers::NONE, Key::Space) || i.consume_key(egui::Modifiers::NONE, Key::Enter);
            input.next |= i.consume_key(egui::Modifiers::NONE, Key::Tab) || i.consume_key(egui::Modifiers::SHIFT, Key::Tab);
            input.back |= i.consume_key(egui::Modifiers::NONE, Key::Backspace);
        });
    }
    let screen = ctx.content_rect();
    let w = (screen.width() - 24.0).min(1100.0);
    let field = d.field();
    let fh = ((w - 32.0) * field.1 / field.0).min(screen.height() - 190.0).max(220.0);
    let fw = fh * field.0 / field.1;
    let shake = if v.shake > 0.0 { (v.age * 60.0).sin() * 6.0 * (v.shake / 0.5).min(1.0) } else { 0.0 };
    let frame = egui::Frame::NONE.fill(theme::rgb(0x04050a)).stroke(Stroke::new(1.0, theme::MAGENTA.gamma_multiply(0.6))).corner_radius(CornerRadius::same(6)).inner_margin(Margin::same(12));
    egui::Modal::new(egui::Id::new(("dive", v.id))).frame(frame).backdrop_color(Color32::from_black_alpha(210)).show(ctx, |ui| {
        ui.set_width(fw.max(560.0));
        let (n, total, name) = d.level();
        ui.horizontal(|ui| {
            ui.label(RichText::new(d.title()).font(retro(28.0)).color(theme::MAGENTA));
            ui.label(RichText::new(format!("{n}/{total} · {name}")).font(mono(13.0)).color(GOLD));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if button(ui, "JACK OUT ✕", Kind::Ghost).on_hover_text("Leave the dive. You keep the levels you have broken.").clicked() {
                    close = true;
                }
                let tr = d.trace();
                let c = if tr > 75.0 { RED } else if tr > 40.0 { AMBER } else { CYAN };
                let (r, _) = ui.allocate_exact_size(Vec2::new(150.0, 8.0), Sense::hover());
                ui.painter().rect_filled(r, 2.0, theme::rgb(0x111830));
                let mut f = r;
                f.set_width(r.width() * (tr / 100.0).clamp(0.0, 1.0));
                ui.painter().rect_filled(f, 2.0, c);
                ui.label(RichText::new("TRACE").font(retro(15.0)).color(c));
            });
        });
        ui.add(egui::Label::new(RichText::new(d.brief()).font(mono(13.5)).color(theme::TEXT)).wrap());
        ui.add_space(4.0);
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(fw, fh), Sense::click_and_drag());
        let rect = rect.translate(Vec2::new(shake, 0.0));
        let view = View::new(rect, field, v.age);
        let painter = ui.painter_at(rect.expand(2.0));
        d.draw(&painter, &view);
        painter.rect_stroke(rect, 2.0, Stroke::new(1.0, LINE2), egui::StrokeKind::Outside);
        // input
        if !auto {
            let pointer = ctx.input(|i| i.pointer.interact_pos().or(i.pointer.hover_pos()));
            input.pointer = pointer.filter(|p| rect.contains(*p)).map(|p| view.back(p));
            ctx.input(|i| {
                input.pressed |= resp.is_pointer_button_down_on() && i.pointer.primary_pressed();
                input.down = i.pointer.primary_down();
                input.released |= i.pointer.primary_released();
                for e in &i.events {
                    if let egui::Event::Text(t) = e {
                        input.typed.extend(t.chars());
                    }
                }
            });
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
            }
        }
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            let help = match d.id() {
                "interceptor" => "drag a handle (Shift: fine)  ·  ← → pick a handle, ↑ ↓ move it  ·  click nodes on the mod-31 levels  ·  SPACE fire",
                "logjam" => "← → the size of the table (or click ◀ ▶)  ·  SPACE crank, or run the table",
                "lamplighter" => "← → change lane  ·  SPACE dodge (stamina)  ·  1–4 call the dark lane before it shows  ·  5–9 taps  ·  − + window  ·  [ ] offset  ·  or click",
                _ => "↑ ↓ choose a word  ·  ← → slide it  ·  TAB the other broadcast  ·  click a cell to drop it there  ·  SPACE commit",
            };
            ui.label(RichText::new(help).font(mono(11.5)).color(FAINT));
            if auto {
                ui.label(RichText::new("· autoplay").font(mono(11.5)).color(DIM));
            }
        });
    });
    // the title card floats over the field for a moment on entry
    if v.age < 1.2 {
        let a = (1.0 - (v.age - 0.8).max(0.0) / 0.4).clamp(0.0, 1.0);
        let layer = egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("dive-jack"));
        let p = ctx.layer_painter(layer);
        let c = screen.center();
        p.text(Pos2::new(c.x, c.y - 150.0), Align2::CENTER_CENTER, "JACKING IN", retro(30.0), theme::MAGENTA.gamma_multiply(a * 0.8));
        let _ = Rect::NOTHING;
    }
    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        close = true;
    }
    ctx.request_repaint();
    close
}
