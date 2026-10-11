//! Chapter 3, part 1. Shamir sharing in the small field GF(31), drawn on a
//! grid you can see. Two shares leave the secret f(0) completely
//! undetermined: every candidate secret has its own curve through the two
//! points. Collect the third share and the parabola, and the secret, snap
//! into place.

use std::rc::Rc;

use cpl_crypto::bytes::Big;
use cpl_crypto::shamir::{TOY_P, curve_for_guess, eval_toy, quadratic_through};
use egui::{Align2, Color32, Pos2, RichText, Sense, Shape, Stroke};

use crate::puzzle::{Host, Puzzle};
use crate::ui::theme::{self, AMBER, CYAN, FAINT, GREEN, mono};
use crate::ui::widgets::{Kind, Tone, button, button_enabled, card, kv, lede, math, note};
use crate::world::World;

pub struct ShamirReconstruct {
    coeffs: [i64; 3],
    shares: Vec<(i64, i64)>,
    guess: i64,
    have_third: bool,
    rebuilt: Option<i64>,
}

impl ShamirReconstruct {
    pub fn new(world: Rc<World>) -> Self {
        let mut r = world.rng("ch3/toy");
        let mut pick = |lo: i64, hi: i64| i64::try_from(r.range(&Big::from(lo), &Big::from(hi))).unwrap();
        let secret = pick(5, 25);
        let a = pick(1, 3);
        let b = pick(1, 10);
        let coeffs = [secret, b, a];
        let shares = (1..=4).map(|x| (x, eval_toy(&coeffs, x, TOY_P))).collect();
        Self { coeffs, shares, guess: 0, have_third: false, rebuilt: None }
    }

    fn known(&self) -> [(i64, i64); 2] {
        [self.shares[0], self.shares[1]]
    }

    fn add_third(&mut self, host: &mut Host) {
        if self.have_third {
            return;
        }
        self.have_third = true;
        let [p1, p2] = self.known();
        let rebuilt = quadratic_through(&[p1, p2, self.shares[2]], TOY_P);
        self.rebuilt = Some(rebuilt[0]);
        self.guess = rebuilt[0];
        host.sfx("confirm");
        host.status("Now lock it in.", Tone::Info);
    }

    fn check(&mut self, host: &mut Host) {
        if host.solved() {
            return;
        }
        if !self.have_third {
            host.status("With two shares you are only guessing. Collect the third share first.", Tone::Warn);
            host.sfx("error");
            return;
        }
        if self.guess != self.coeffs[0] {
            host.status("That is not the secret. Let the interpolation compute it.", Tone::Bad);
            host.sfx("error");
            return;
        }
        host.sfx("unlock");
        host.solve(format!("Secret recovered: {}. Three of five shares, and not one fewer.", self.coeffs[0]));
    }

    fn controls(&mut self, ui: &mut egui::Ui, third: &mut bool, lock: &mut bool) {
        lede(ui, "A secret is hidden as **f(0)** of a degree-2 polynomial over the field GF(31). You were handed two shares, Mara's and Felix's. The ringed dot is your guess for the secret.");
        math(ui, "f(x) = a₂x² + a₁x + s   (mod 31),   secret = f(0)");
        let [m, f] = self.known();
        card(ui, Some("Known shares"), None, |ui| {
            kv(ui, "shamir-known", &[("Mara (x=1)", format!("y = {}", m.1)), ("Felix (x=2)", format!("y = {}", f.1))]);
        });
        card(ui, Some("Guess the secret"), None, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("secret f(0) =").font(mono(13.0)));
                ui.add_enabled(!self.have_third, egui::Slider::new(&mut self.guess, 0..=30));
            });
            note(ui, "With only two shares, **every** value of f(0) from 0 to 30 gives a valid parabola through both points. Two shares reveal nothing: that is the whole idea.");
            if button_enabled(ui, !self.have_third, "Add Juno's share (x = 3)", Kind::Normal).clicked() {
                *third = true;
            }
            if let Some(s) = self.rebuilt {
                note(ui, &format!("Three points fix one parabola. Interpolating at x = 0 gives the secret: **f(0) = {s}**."));
            }
            if button(ui, "Lock in the secret", Kind::Primary).clicked() {
                *lock = true;
            }
        });
    }

    fn plot(&self, ui: &mut egui::Ui) {
        let size = egui::vec2(ui.available_width().min(460.0), 360.0);
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let p = ui.painter_at(rect);
        let (w, h) = (rect.width(), rect.height());
        let px = |x: f32| rect.left() + 40.0 + (x / 5.0) * (w - 60.0);
        let py = |y: f32| rect.top() + h - 30.0 - (y / 31.0) * (h - 50.0);
        p.rect_filled(rect, 3.0, theme::rgb(0x070b16));
        let grid = Stroke::new(1.0, theme::rgb(0x17203a));
        for x in 0..=5 {
            p.line_segment([Pos2::new(px(x as f32), py(0.0)), Pos2::new(px(x as f32), py(31.0))], grid);
        }
        for y in (0..=30).step_by(5) {
            p.line_segment([Pos2::new(px(0.0), py(y as f32)), Pos2::new(px(5.0), py(y as f32))], grid);
        }
        p.text(Pos2::new(px(0.0) - 6.0, py(31.0) - 8.0), Align2::LEFT_BOTTOM, "f(0) = secret", mono(11.0), FAINT);
        for x in 0..=4 {
            p.text(Pos2::new(px(x as f32), rect.bottom() - 10.0), Align2::CENTER_CENTER, format!("x={x}"), mono(11.0), FAINT);
        }
        p.add(Shape::dashed_line(&[Pos2::new(px(0.0), py(0.0)), Pos2::new(px(0.0), py(31.0))], Stroke::new(1.0, AMBER), 3.0, 3.0));
        let curve = |c: &[i64; 3], color: Color32, width: f32| {
            let pts: Vec<Pos2> = (0..=5).map(|x| Pos2::new(px(x as f32), py(eval_toy(c, x, TOY_P) as f32))).collect();
            p.add(Shape::line(pts, Stroke::new(width, color)));
        };
        if !self.have_third {
            for s in (0..=30).step_by(3) {
                curve(&curve_for_guess(s, &self.known(), TOY_P), Color32::from_rgba_unmultiplied(90, 110, 150, 64), 1.0);
            }
            curve(&curve_for_guess(self.guess, &self.known(), TOY_P), CYAN, 2.0);
        } else {
            curve(&self.coeffs, GREEN, 2.0);
        }
        let dot = |x: f32, y: f32, color: Color32, r: f32, ring: bool| {
            p.circle_filled(Pos2::new(px(x), py(y)), r, color);
            if ring {
                p.circle_stroke(Pos2::new(px(x), py(y)), r + 4.0, Stroke::new(1.5, color));
            }
        };
        for (x, y) in self.known() {
            dot(x as f32, y as f32, GREEN, 5.0, false);
        }
        if self.have_third {
            dot(3.0, self.shares[2].1 as f32, GREEN, 5.0, false);
        }
        let secret = if self.have_third { self.coeffs[0] } else { self.guess };
        let c = if self.have_third { GREEN } else { CYAN };
        dot(0.0, secret as f32, c, 6.0, !self.have_third);
        p.text(Pos2::new(px(0.0) - 14.0, py(secret as f32)), Align2::RIGHT_CENTER, secret.to_string(), mono(13.0), c);
    }
}

impl Puzzle for ShamirReconstruct {
    fn id(&self) -> &'static str {
        "ch3.shamir"
    }

    fn title(&self) -> &str {
        "JUNO'S WORKSHOP · GF(31)"
    }

    fn subtitle(&self) -> Option<&str> {
        Some("threshold 3 of 5")
    }

    fn accent(&self) -> Color32 {
        GREEN
    }

    fn open(&mut self, host: &mut Host) {
        host.status("Two shares: the secret could be anything. Slide the guess and watch.", Tone::Info);
    }

    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        let mut third = false;
        let mut lock = false;
        if ui.available_width() > 700.0 {
            ui.columns(2, |cols| {
                self.controls(&mut cols[0], &mut third, &mut lock);
                self.plot(&mut cols[1]);
            });
        } else {
            self.controls(ui, &mut third, &mut lock);
            self.plot(ui);
        }
        if third {
            self.add_third(host);
        }
        if lock {
            self.check(host);
        }
    }

    fn autoplay(&mut self, host: &mut Host) {
        self.add_third(host);
        self.check(host);
    }
}
