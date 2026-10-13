//! Chapter 6, part 1. Lattice reduction in two dimensions, by hand. A lattice
//! is all integer combinations of two basis vectors; a bad basis is long and
//! skewed, a good one short and near-orthogonal. Lagrange–Gauss reduction
//! repeatedly subtracts the nearest multiple of the shorter vector from the
//! longer and swaps. When it stops, the shorter vector is the shortest in the
//! whole lattice. This is LLL's engine, visible.

use std::rc::Rc;

use cpl_crypto::lattice::{Gauss2, V2, n2};
use egui::{Align2, Color32, Pos2, RichText, Sense, Shape, Stroke};

use crate::puzzle::{Host, Puzzle};
use crate::ui::theme::{self, AMBER, CYAN, DIM, MAGENTA, TEXT, mono};
use crate::ui::widgets::{Kind, Tone, button, card, lede, math, note};
use crate::world::World;

pub struct GaussReduce {
    s: Gauss2,
    shortest: i64,
    steps: u32,
    info: Option<(String, Color32)>,
}

impl GaussReduce {
    pub fn new(world: Rc<World>) -> Self {
        let mut r = world.rng("ch6/gauss");
        // start from a reduced-ish basis, then skew it with random unimodular moves
        let mut b1: V2 = [4 + r.int(2) as i64, 1];
        let mut b2: V2 = [1, 4 + r.int(3) as i64];
        for i in 0..4 {
            let k = (r.int(3) as i64 + 1) * if r.int(2) == 1 { 1 } else { -1 };
            if i % 2 == 0 {
                b2 = [b2[0] + k * b1[0], b2[1] + k * b1[1]];
            } else {
                b1 = [b1[0] + k * b2[0], b1[1] + k * b2[1]];
            }
        }
        let s = Gauss2 { b1, b2 };
        let shortest = n2(s.reduce().0.b1);
        Self { s, shortest, steps: 0, info: None }
    }

    fn reduce_step(&mut self, host: &mut Host) {
        let m = self.s.mu();
        if m.nearest == 0 {
            self.info = Some(("round(μ) = 0: subtracting would not help. Try a swap.".into(), AMBER));
            host.sfx("error");
            return;
        }
        self.s = self.s.step(m.nearest);
        self.steps += 1;
        host.sfx("blip");
        self.info = Some((format!("Subtracted {}·b₁ from b₂.", m.nearest), DIM));
    }

    fn swap(&mut self, host: &mut Host) {
        self.s = self.s.swap();
        self.steps += 1;
        host.sfx("blip");
        self.info = Some(("Swapped.".into(), DIM));
    }

    fn check(&mut self, host: &mut Host) {
        if host.solved() {
            return;
        }
        if self.s.is_reduced() {
            host.sfx("unlock");
            host.solve(format!("Reduced in {} moves. b₁ is now a shortest nonzero vector of the lattice. In high dimensions this exact loop, run pairwise, is LLL.", self.steps));
        } else {
            let m = self.s.mu();
            let text = if n2(self.s.b1) > n2(self.s.b2) {
                "Not reduced: b₂ is shorter than b₁. Swap them.".to_string()
            } else {
                format!("Not reduced: |μ| = {:.2} > 0.5. Subtract round(μ)·b₁ from b₂.", m.value.abs())
            };
            self.info = Some((text, AMBER));
            host.sfx("error");
        }
    }

    fn auto(&mut self, host: &mut Host) {
        let mut guard = 0;
        while !self.s.is_reduced() && guard < 50 {
            guard += 1;
            if n2(self.s.b1) > n2(self.s.b2) {
                self.swap(host);
            } else {
                if self.s.mu().nearest == 0 {
                    break;
                }
                self.reduce_step(host);
            }
        }
        self.check(host);
    }

    fn controls(&mut self, ui: &mut egui::Ui, act: &mut Option<u8>) {
        lede(ui, "Both vectors below generate the same infinite grid of points. But one basis is a tangled mess and the equivalent short basis reveals the lattice's shortest vector. Reduce it.");
        math(ui, "μ = ⟨b₁,b₂⟩ / ⟨b₁,b₁⟩,   replace b₂ ← b₂ − round(μ)·b₁,   swap if |b₂| < |b₁|");
        let m = self.s.mu();
        let (b1, b2) = (self.s.b1, self.s.b2);
        card(ui, Some("Current basis"), None, |ui| {
            egui::Grid::new("gauss-basis").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                let rows = [
                    ("b₁", format!("({}, {})  |b₁| = {:.2}", b1[0], b1[1], (n2(b1) as f64).sqrt()), CYAN),
                    ("b₂", format!("({}, {})  |b₂| = {:.2}", b2[0], b2[1], (n2(b2) as f64).sqrt()), MAGENTA),
                    ("μ", format!("{:.3}  → round(μ) = {}", m.value, m.nearest), TEXT),
                    ("det (area)", format!("{}  (never changes)", self.s.det()), TEXT),
                ];
                for (k, v, c) in rows {
                    ui.label(RichText::new(k).font(mono(12.5)).color(DIM));
                    ui.label(RichText::new(v).font(mono(13.0)).color(c));
                    ui.end_row();
                }
            });
        });
        card(ui, Some("Moves"), None, |ui| {
            note(ui, "Subtract the nearest multiple of the shorter vector from the longer, then swap so the shorter is b₁. Repeat until no subtraction shrinks anything.");
            ui.horizontal_wrapped(|ui| {
                if button(ui, "b₂ ← b₂ − round(μ)·b₁", Kind::Primary).clicked() {
                    *act = Some(0);
                }
                if button(ui, "swap b₁ ↔ b₂", Kind::Normal).clicked() {
                    *act = Some(1);
                }
            });
            ui.horizontal_wrapped(|ui| {
                if button(ui, "Check if reduced", Kind::Normal).clicked() {
                    *act = Some(2);
                }
                if button(ui, "Auto-reduce", Kind::Ghost).clicked() {
                    *act = Some(3);
                }
            });
            if let Some((t, c)) = &self.info {
                ui.label(RichText::new(t).font(mono(13.0)).color(*c));
            }
        });
    }

    fn plot(&self, ui: &mut egui::Ui) {
        let size = egui::vec2(ui.available_width().min(440.0), 400.0);
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let p = ui.painter_at(rect);
        let (b1, b2) = (self.s.b1, self.s.b2);
        let maxc = [b1[0], b1[1], b2[0], b2[1]].iter().map(|v| v.abs()).max().unwrap_or(4).max(4) as f32;
        let scale = (26.0f32).min((rect.width().min(rect.height()) / 2.0 - 20.0) / (maxc + 1.0));
        let c = rect.center();
        let at = |x: f32, y: f32| Pos2::new(c.x + x * scale, c.y - y * scale);
        p.rect_filled(rect, 3.0, theme::rgb(0x070b16));
        // lattice points
        let range = maxc.ceil() as i64 + 3;
        for i in -range..=range {
            for j in -range..=range {
                let x = (i * b1[0] + j * b2[0]) as f32;
                let y = (i * b1[1] + j * b2[1]) as f32;
                if x.abs() <= maxc + 2.0 && y.abs() <= maxc + 2.0 {
                    p.circle_filled(at(x, y), 2.0, theme::rgb(0x2a3a66));
                }
            }
        }
        // axes
        let axis = Stroke::new(1.0, theme::rgb(0x17203a));
        p.line_segment([Pos2::new(rect.left(), c.y), Pos2::new(rect.right(), c.y)], axis);
        p.line_segment([Pos2::new(c.x, rect.top()), Pos2::new(c.x, rect.bottom())], axis);
        // the shortest vector's circle
        let r = (self.shortest as f32).sqrt() * scale;
        let pts: Vec<Pos2> = (0..=64).map(|k| {
            let a = k as f32 / 64.0 * std::f32::consts::TAU;
            Pos2::new(c.x + r * a.cos(), c.y + r * a.sin())
        }).collect();
        p.extend(Shape::dashed_line(&pts, Stroke::new(1.0, Color32::from_rgba_unmultiplied(93, 255, 154, 64)), 3.0, 3.0));
        // the basis vectors
        for (v, color, label) in [(b1, CYAN, "b₁"), (b2, MAGENTA, "b₂")] {
            let tip = at(v[0] as f32, v[1] as f32);
            p.line_segment([c, tip], Stroke::new(2.0, color));
            p.circle_filled(tip, 3.5, color);
            p.text(tip + egui::vec2(5.0, -4.0), Align2::LEFT_BOTTOM, label, mono(13.0), color);
        }
    }
}

impl Puzzle for GaussReduce {
    fn id(&self) -> &'static str {
        "ch6.gauss"
    }

    fn title(&self) -> &str {
        "LATTICE REDUCTION"
    }

    fn subtitle(&self) -> Option<&str> {
        Some("Lagrange–Gauss, by hand")
    }

    fn accent(&self) -> Color32 {
        theme::rgb(0x73b7ff)
    }

    fn open(&mut self, host: &mut Host) {
        host.status("Reduce the basis until b₁ is the shortest vector.", Tone::Info);
    }

    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        let mut act = None;
        if ui.available_width() > 700.0 {
            ui.columns(2, |cols| {
                self.controls(&mut cols[0], &mut act);
                self.plot(&mut cols[1]);
            });
        } else {
            self.controls(ui, &mut act);
            self.plot(ui);
        }
        match act {
            Some(0) => self.reduce_step(host),
            Some(1) => self.swap(host),
            Some(2) => self.check(host),
            Some(3) => self.auto(host),
            _ => {}
        }
    }

    fn autoplay(&mut self, host: &mut Host) {
        self.auto(host);
    }
}
