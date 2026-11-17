//! Cyberspace: short real-time games played inside a terminal, where the
//! mathematics is the playing field. Each dive is a set of levels with a
//! trace clock; the simulation is plain data stepped with a time delta, so
//! tests and the demo drive it with the same inputs a player would.

pub mod interceptor;
pub mod static_;

use egui::{Color32, Pos2, Rect, Stroke, Vec2};

use crate::ui::theme;

/// One frame of player input, in the dive's own field coordinates.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Input {
    /// pointer position in field units, if it is over the field
    pub pointer: Option<(f32, f32)>,
    pub pressed: bool,
    pub down: bool,
    pub released: bool,
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down_key: bool,
    /// fine adjustment (Shift)
    pub fine: bool,
    /// Space or Enter, this frame
    pub fire: bool,
    /// Tab, this frame
    pub next: bool,
    /// Backspace, this frame
    pub back: bool,
    /// characters typed this frame
    pub typed: Vec<char>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Playing,
    /// every level done
    Cleared,
}

/// Sounds and screen effects a dive asks for.
#[derive(Default)]
pub struct Fx {
    pub sfx: Vec<&'static str>,
    pub shake: f32,
}

pub trait Dive {
    fn id(&self) -> &'static str;
    fn title(&self) -> &'static str;
    /// The flag set when the last level falls.
    fn done_flag(&self) -> &'static str;
    /// (current level from 1, levels, its name)
    fn level(&self) -> (usize, usize, String);
    /// One line under the title: what this level wants.
    fn brief(&self) -> String;
    /// 0..100; the dive throws you back to the start of the level at 100.
    fn trace(&self) -> f32;
    /// The field's size in its own units (width, height); (0, 0) is the centre.
    fn field(&self) -> (f32, f32);
    fn update(&mut self, dt: f32, input: &Input, fx: &mut Fx);
    fn draw(&self, p: &egui::Painter, view: &View);
    fn status(&self) -> Status;
    /// What a player who knows the answer would press this frame.
    fn bot(&mut self, dt: f32) -> Input;
}

/// Maps field units to the screen.
#[derive(Clone, Copy, Debug)]
pub struct View {
    pub rect: Rect,
    pub scale: f32,
    /// seconds, for animation
    pub t: f32,
}

impl View {
    pub fn new(rect: Rect, field: (f32, f32), t: f32) -> Self {
        let scale = (rect.width() / field.0).min(rect.height() / field.1);
        Self { rect, scale, t }
    }

    /// Field (x right, y up) to screen.
    pub fn at(&self, x: f32, y: f32) -> Pos2 {
        let c = self.rect.center();
        Pos2::new(c.x + x * self.scale, c.y - y * self.scale)
    }

    pub fn back(&self, p: Pos2) -> (f32, f32) {
        let c = self.rect.center();
        ((p.x - c.x) / self.scale, (c.y - p.y) / self.scale)
    }

    pub fn len(&self, units: f32) -> f32 {
        units * self.scale
    }
}

// --- neon drawing helpers ------------------------------------------------------------

pub const NEON_CYAN: Color32 = theme::rgb(0x3ef0ff);
pub const NEON_PINK: Color32 = theme::rgb(0xff3e9a);
pub const NEON_GOLD: Color32 = theme::rgb(0xffcd75);
pub const NEON_GREEN: Color32 = theme::rgb(0x5dff9a);
pub const NEON_RED: Color32 = theme::rgb(0xff5566);
pub const NEON_VIOLET: Color32 = theme::rgb(0xc792ff);
pub const DEEP: Color32 = theme::rgb(0x05060c);

/// A line with a soft halo.
pub fn glow_line(p: &egui::Painter, a: Pos2, b: Pos2, c: Color32, w: f32) {
    p.line_segment([a, b], Stroke::new(w * 4.0, c.gamma_multiply(0.10)));
    p.line_segment([a, b], Stroke::new(w * 2.0, c.gamma_multiply(0.28)));
    p.line_segment([a, b], Stroke::new(w, c));
}

pub fn glow_path(p: &egui::Painter, pts: &[Pos2], c: Color32, w: f32) {
    if pts.len() < 2 {
        return;
    }
    p.add(egui::Shape::line(pts.to_vec(), Stroke::new(w * 4.0, c.gamma_multiply(0.10))));
    p.add(egui::Shape::line(pts.to_vec(), Stroke::new(w * 2.0, c.gamma_multiply(0.28))));
    p.add(egui::Shape::line(pts.to_vec(), Stroke::new(w, c)));
}

pub fn glow_ring(p: &egui::Painter, at: Pos2, r: f32, c: Color32, w: f32) {
    p.circle_stroke(at, r, Stroke::new(w * 3.0, c.gamma_multiply(0.15)));
    p.circle_stroke(at, r, Stroke::new(w, c));
}

pub fn glow_dot(p: &egui::Painter, at: Pos2, r: f32, c: Color32) {
    p.circle_filled(at, r * 2.2, c.gamma_multiply(0.12));
    p.circle_filled(at, r * 1.5, c.gamma_multiply(0.25));
    p.circle_filled(at, r, c);
}

/// A diamond outline (shares, nodes).
pub fn diamond(p: &egui::Painter, at: Pos2, r: f32, c: Color32, w: f32, fill: bool) {
    let pts = vec![at + Vec2::new(0.0, -r), at + Vec2::new(r, 0.0), at + Vec2::new(0.0, r), at + Vec2::new(-r, 0.0)];
    if fill {
        p.add(egui::Shape::convex_polygon(pts.clone(), c.gamma_multiply(0.35), Stroke::NONE));
    }
    let mut closed = pts;
    closed.push(closed[0]);
    glow_path(p, &closed, c, w);
}

/// The grid every dive floats over.
pub fn grid(p: &egui::Painter, v: &View, field: (f32, f32), step: f32) {
    p.rect_filled(v.rect, 0.0, DEEP);
    let (hw, hh) = (field.0 / 2.0, field.1 / 2.0);
    let line = NEON_CYAN.gamma_multiply(0.07);
    let mut x = -hw.floor();
    while x <= hw {
        p.line_segment([v.at(x, -hh), v.at(x, hh)], Stroke::new(1.0, line));
        x += step;
    }
    let mut y = -hh.floor();
    while y <= hh {
        p.line_segment([v.at(-hw, y), v.at(hw, y)], Stroke::new(1.0, line));
        y += step;
    }
    // a slow pulse rolling across the floor
    let k = (v.t * 0.25).fract();
    let px = -hw + field.0 * k;
    p.line_segment([v.at(px, -hh), v.at(px, hh)], Stroke::new(2.0, NEON_CYAN.gamma_multiply(0.10)));
}

/// A burst of sparks: lines flying out of a point, `age` seconds old.
pub fn sparks(p: &egui::Painter, at: Pos2, age: f32, c: Color32, n: usize, reach: f32, seed: u32) {
    if age > 0.9 {
        return;
    }
    let k = age / 0.9;
    for i in 0..n {
        let h = (i as u32).wrapping_mul(2654435761).wrapping_add(seed.wrapping_mul(40503));
        let ang = (h % 6283) as f32 / 1000.0;
        let speed = 0.5 + ((h >> 13) % 100) as f32 / 100.0;
        let d = Vec2::new(ang.cos(), ang.sin());
        let r0 = reach * speed * k;
        let r1 = reach * speed * (k * 0.6);
        p.line_segment([at + d * r1, at + d * r0], Stroke::new(2.0, c.gamma_multiply(1.0 - k)));
    }
}

/// Make a dive by id.
pub fn make(id: &str, seed: &[u8]) -> Option<Box<dyn Dive>> {
    match id {
        "interceptor" => Some(Box::new(interceptor::Interceptor::new(seed))),
        "static" => Some(Box::new(static_::Static::new(seed))),
        _ => None,
    }
}

/// Play a dive to the end with its own bot (tests and the demo use this).
/// Returns the simulated seconds it took, or `None` if it did not finish.
pub fn autoplay(d: &mut dyn Dive, limit: f32) -> Option<f32> {
    let dt = 1.0 / 60.0;
    let mut t = 0.0;
    let mut fx = Fx::default();
    while t < limit {
        if d.status() == Status::Cleared {
            return Some(t);
        }
        let input = d.bot(dt);
        d.update(dt, &input, &mut fx);
        fx.sfx.clear();
        t += dt;
    }
    None
}
