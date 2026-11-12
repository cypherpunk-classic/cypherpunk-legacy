//! Chapter 6, part 1. Lattice reduction in two dimensions, by hand. A lattice
//! is all integer combinations of two basis vectors; a bad basis is long and
//! skewed, a good one short and near-orthogonal. Lagrange–Gauss reduction
//! repeatedly subtracts the nearest multiple of the shorter vector from the
//! longer and swaps. When it stops, the shorter vector is the shortest in the
//! whole lattice. This is LLL's engine, visible. The player picks every
//! multiple; the terminal only applies it and says whether the basis is
//! reduced.

use std::rc::Rc;

use cpl_crypto::lattice::{Gauss2, V2, n2};
use egui::{Align2, Color32, Pos2, RichText, Sense, Shape, Stroke};

use crate::puzzle::{HandCell, HandSheet, Host, Puzzle, StepView, steps};
use crate::ui::theme::{self, AMBER, CYAN, DIM, GREEN, MAGENTA, TEXT, mono};
use crate::ui::widgets::{Kind, Tone, button, card, lede, math, note};
use crate::world::World;

pub struct GaussReduce {
    world: Rc<World>,
    s: Gauss2,
    shortest: i64,
    steps: u32,
    info: Option<(String, Color32)>,
    /// the multiple of b₁ the player will subtract next
    k: i64,
    reductions: u32,
    swaps: u32,
    done: bool,
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
        Self { world, s, shortest, steps: 0, info: None, k: 1, reductions: 0, swaps: 0, done: false }
    }

    fn reduce_step(&mut self, host: &mut Host) {
        if self.k == 0 {
            self.info = Some(("Subtracting 0·b₁ changes nothing.".into(), AMBER));
            return;
        }
        let before = n2(self.s.b2);
        self.s = self.s.step(self.k);
        self.steps += 1;
        self.reductions += 1;
        let after = n2(self.s.b2);
        host.sfx("blip");
        let (text, c) = if after < before {
            (format!("b₂ ← b₂ − {}·b₁: |b₂| went from {:.2} to {:.2}.", self.k, (before as f64).sqrt(), (after as f64).sqrt()), DIM)
        } else {
            (format!("b₂ ← b₂ − {}·b₁ made b₂ longer ({:.2} → {:.2}). Still the same lattice; try another multiple.", self.k, (before as f64).sqrt(), (after as f64).sqrt()), AMBER)
        };
        self.info = Some((text, c));
    }

    fn swap(&mut self, host: &mut Host) {
        self.s = self.s.swap();
        self.steps += 1;
        self.swaps += 1;
        host.sfx("blip");
        self.info = Some(("Swapped.".into(), DIM));
    }

    fn check(&mut self, host: &mut Host) {
        if host.solved() {
            return;
        }
        if self.s.is_reduced() {
            self.done = true;
            host.sfx("unlock");
            host.solve(format!("Reduced in {} moves. b₁ is now a shortest nonzero vector of the lattice. In high dimensions this exact loop, run pairwise, is LLL.", self.steps));
        } else {
            let m = self.s.mu();
            let text = if n2(self.s.b1) > n2(self.s.b2) {
                "Not reduced: b₂ is shorter than b₁. Swap them.".to_string()
            } else {
                format!("Not reduced: |μ| = {:.2} > 0.5, so some multiple of b₁ still shortens b₂.", m.value.abs())
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
                self.k = self.s.mu().nearest;
                self.reduce_step(host);
            }
        }
        self.check(host);
    }

    fn controls(&mut self, ui: &mut egui::Ui, act: &mut Option<u8>, hints: bool) {
        lede(ui, "Both vectors below generate the same infinite grid of points. But one basis is a tangled mess and the equivalent short basis reveals the lattice's shortest vector. Reduce it.");
        math(ui, "μ = ⟨b₁,b₂⟩ / ⟨b₁,b₁⟩,   replace b₂ ← b₂ − round(μ)·b₁,   swap if |b₂| < |b₁|");
        let m = self.s.mu();
        let (b1, b2) = (self.s.b1, self.s.b2);
        card(ui, Some("Current basis"), None, |ui| {
            egui::Grid::new("gauss-basis").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                let mu = if hints { format!("{:.3}", m.value) } else { "work it out".into() };
                let rows = [
                    ("b₁", format!("({}, {})  |b₁| = {:.2}", b1[0], b1[1], (n2(b1) as f64).sqrt()), CYAN),
                    ("b₂", format!("({}, {})  |b₂| = {:.2}", b2[0], b2[1], (n2(b2) as f64).sqrt()), MAGENTA),
                    ("⟨b₁,b₂⟩ / ⟨b₁,b₁⟩", format!("{} / {}", m.num, m.den), TEXT),
                    ("μ", mu, TEXT),
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
            let preview = self.s.step(self.k).b2;
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("k =").font(mono(13.0)).color(DIM));
                ui.add(egui::DragValue::new(&mut self.k).range(-12..=12).speed(0.1));
                ui.label(RichText::new(format!("→ b₂ would be ({}, {}), |b₂| = {:.2}", preview[0], preview[1], (n2(preview) as f64).sqrt())).font(mono(12.5)).color(if n2(preview) < n2(b2) { GREEN } else { DIM }));
            });
            ui.horizontal_wrapped(|ui| {
                if button(ui, "b₂ ← b₂ − k·b₁", Kind::Primary).clicked() {
                    *act = Some(0);
                }
                if button(ui, "swap b₁ ↔ b₂", Kind::Normal).clicked() {
                    *act = Some(1);
                }
                if button(ui, "Check if reduced", Kind::Normal).clicked() {
                    *act = Some(2);
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
        let hints = host.hints;
        if ui.available_width() > 700.0 {
            ui.columns(2, |cols| {
                self.controls(&mut cols[0], &mut act, hints);
                self.plot(&mut cols[1]);
            });
        } else {
            self.controls(ui, &mut act, hints);
            self.plot(ui);
        }
        match act {
            Some(0) => self.reduce_step(host),
            Some(1) => self.swap(host),
            Some(2) => self.check(host),
            _ => {}
        }
    }

    fn autoplay(&mut self, host: &mut Host) {
        self.auto(host);
    }

    fn steps(&self) -> Vec<StepView> {
        steps(&[
            ("Shorten b₂ by subtracting a multiple of b₁", "The best k is the nearest whole number to ⟨b₁,b₂⟩ / ⟨b₁,b₁⟩.", self.reductions > 0),
            ("Swap whenever b₂ becomes the shorter one", "", self.swaps > 0 || self.done),
            ("Stop when nothing shrinks, and check", "Reduced means |μ| ≤ ½ and |b₁| ≤ |b₂|.", self.done),
        ])
    }

    fn deep_sheet(&self) -> Option<HandSheet> {
        let mut r = self.world.rng("ch6/gauss-deep");
        let b1: V2 = [3 + r.int(3) as i64, 1 + r.int(2) as i64];
        let k = 2 + r.int(3) as i64;
        let b2: V2 = [k * b1[0] + 1, k * b1[1] - 2 + r.int(2) as i64];
        let s = Gauss2 { b1, b2 };
        let m = s.mu();
        let next = s.step(m.nearest).b2;
        Some(HandSheet::new(
            "one Gauss step, by hand",
            &format!("b₁ = ({}, {}), b₂ = ({}, {}). One reduction step: k = round(⟨b₁,b₂⟩ / ⟨b₁,b₁⟩), then b₂ ← b₂ − k·b₁.", b1[0], b1[1], b2[0], b2[1]),
            150.0,
            vec![
                HandCell::new("⟨b₁,b₂⟩", "x₁x₂ + y₁y₂", m.num),
                HandCell::new("⟨b₁,b₁⟩", "|b₁|²", m.den),
                HandCell::new("k = round(μ)", "nearest whole number", m.nearest),
                HandCell::new("new b₂, x", "", next[0]),
                HandCell::new("new b₂, y", "", next[1]),
            ],
        ))
    }
}
