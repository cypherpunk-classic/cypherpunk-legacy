//! Side quest: Yellow Dots. Theo printed a Directorate memo on the office
//! colour laser and wants to leak it. Under a blue light the page shows a grid
//! of tiny yellow dots. Read them: they say which printer, and when.

use std::rc::Rc;

use cpl_crypto::dots::{COLS, FIELDS, Grid, PrintInfo, ROWS, column_value, encode, parity_ok};
use egui::{Color32, RichText, Stroke};

use crate::puzzle::{Host, Puzzle, StepView, steps};
use crate::ui::theme::{self, AMBER, CYAN, DIM, GREEN, RED, TEXT, mono};
use crate::ui::widgets::{Kind, Tone, button, card, lede, note, select_card, text_field};
use crate::world::World;

pub struct TrackingDots {
    info: PrintInfo,
    grid: Grid,
    hour: String,
    minute: String,
    serial: String,
    decoded: bool,
    plan: Option<usize>,
}

impl TrackingDots {
    pub fn new(world: Rc<World>) -> Self {
        let mut r = world.rng("q/dots");
        let info = PrintInfo {
            minute: r.int(60) as u8,
            hour: 8 + r.int(10) as u8,
            day: 1 + r.int(28) as u8,
            month: 1 + r.int(12) as u8,
            year: 26,
            serial: 20_000_000 + r.int(70_000_000) as u32,
        };
        Self { grid: encode(&info), info, hour: String::new(), minute: String::new(), serial: String::new(), decoded: false, plan: None }
    }

    fn check_reading(&mut self, host: &mut Host) {
        let h = self.hour.trim().parse::<u8>().ok();
        let m = self.minute.trim().parse::<u8>().ok();
        let s = self.serial.trim().replace(' ', "").parse::<u32>().ok();
        let mut wrong = Vec::new();
        if h != Some(self.info.hour) {
            wrong.push("the hour");
        }
        if m != Some(self.info.minute) {
            wrong.push("the minute");
        }
        if s != Some(self.info.serial) {
            wrong.push("the serial number");
        }
        if wrong.is_empty() {
            self.decoded = true;
            host.sfx("confirm");
            host.status(format!("Printer {} at {:02}:{:02} on {:02}/{:02}. The office keeps a log of who printed what, on which machine, and when.", self.info.serial, self.info.hour, self.info.minute, self.info.day, self.info.month), Tone::Ok);
        } else {
            host.status(format!("Not quite: check {}. Each column is one number: seven dots, top dot = 64, then 32, 16, 8, 4, 2, 1. The serial is four columns of two digits each.", wrong.join(", ")), Tone::Bad);
        }
    }

    fn choose(&mut self, i: usize, host: &mut Host) {
        self.plan = Some(i);
        match i {
            0 => host.status("A colour scan keeps the dots. That is how a leaked NSA report led straight to its printer in 2017.", Tone::Bad),
            1 => host.status("A different colour laser prints its own dots. Now it points at that printer instead.", Tone::Bad),
            _ => {
                host.sfx("unlock");
                host.solve("Theo retypes the memo as plain text and sends it through the remailers. No paper, no dots, no printer log.");
            }
        }
    }
}

impl Puzzle for TrackingDots {
    fn id(&self) -> &'static str {
        "q.dots"
    }

    fn title(&self) -> &str {
        "YELLOW DOTS"
    }

    fn subtitle(&self) -> Option<&str> {
        Some("machine identification code · metadata on paper")
    }

    fn accent(&self) -> Color32 {
        AMBER
    }

    fn open(&mut self, host: &mut Host) {
        host.status("Read the dots: when was it printed, and on which printer?", Tone::Info);
    }

    fn steps(&self) -> Vec<StepView> {
        steps(&[
            ("Read the hour and the minute", "Columns 2 and 1. Add up the dots: 64, 32, 16, 8, 4, 2, 1 from the top (the very top row is parity).", self.decoded),
            ("Read the serial number", "Columns 7 to 10, two digits each.", self.decoded),
            ("Choose how Theo should leak it", "Anything that goes through a colour printer gets dots.", self.plan == Some(2)),
        ])
    }

    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        lede(ui, "Theo printed the memo that proves the Harvest program on the office colour laser. Under a blue lamp you can see what the printer added: a faint grid of yellow dots, repeated all over the page.");
        // the dot grid
        let cell = 22.0;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(cell * (COLS as f32 + 1.5), cell * (ROWS as f32 + 1.5)), egui::Sense::hover());
        let p = ui.painter_at(rect);
        p.rect_filled(rect, 4.0, theme::rgb(0x1b2a4a));
        let origin = rect.min + egui::vec2(cell * 1.2, cell * 1.0);
        for col in 0..COLS {
            p.text(origin + egui::vec2(col as f32 * cell, -cell * 0.55), egui::Align2::CENTER_CENTER, col.to_string(), mono(10.5), DIM);
            let ok = parity_ok(&self.grid, col);
            for row in 0..ROWS {
                let c = origin + egui::vec2(col as f32 * cell, row as f32 * cell);
                if row == 0 {
                    p.line_segment([c + egui::vec2(-cell / 2.0, cell / 2.0), c + egui::vec2(cell / 2.0, cell / 2.0)], Stroke::new(1.0, theme::rgb(0x2a3a5a)));
                }
                if self.grid[row][col] {
                    p.circle_filled(c, 4.5, if row == 0 { AMBER.gamma_multiply(0.6) } else { theme::rgb(0xffe35a) });
                } else {
                    p.circle_stroke(c, 2.0, Stroke::new(1.0, theme::rgb(0x2a3a5a)));
                }
            }
            if !ok {
                p.text(origin + egui::vec2(col as f32 * cell, ROWS as f32 * cell), egui::Align2::CENTER_CENTER, "!", mono(11.0), RED);
            }
        }
        for row in 1..ROWS {
            p.text(rect.min + egui::vec2(cell * 0.45, cell * 1.0 + row as f32 * cell), egui::Align2::CENTER_CENTER, (1 << (7 - row)).to_string(), mono(9.5), DIM);
        }
        if host.hints {
            note(ui, &format!("Columns: {}", FIELDS.iter().enumerate().filter(|(_, f)| **f != "unused").map(|(i, f)| format!("{i} {f}")).collect::<Vec<_>>().join(" · ")));
        }
        card(ui, Some("Your reading"), None, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("printed at").font(mono(12.5)).color(DIM));
                text_field(ui, &mut self.hour, "hh", 40.0);
                ui.label(":");
                text_field(ui, &mut self.minute, "mm", 40.0);
                ui.label(RichText::new("on printer").font(mono(12.5)).color(DIM));
                text_field(ui, &mut self.serial, "8 digits", 110.0);
            });
            if !self.decoded && button(ui, "Check my reading", Kind::Primary).clicked() {
                self.check_reading(host);
            }
            if self.decoded {
                ui.label(RichText::new(format!("column values: {}", (0..COLS).map(|c| column_value(&self.grid, c).to_string()).collect::<Vec<_>>().join(" "))).font(mono(12.0)).color(GREEN));
            }
        });
        if self.decoded {
            let mut pick = None;
            card(ui, Some("How should Theo leak it?"), None, |ui| {
                for (i, o) in ["Scan the printout in colour and send the scan", "Print it again on a different colour printer", "Retype it as plain text and send it through the remailers"].iter().enumerate() {
                    if select_card(ui, self.plan == Some(i), CYAN, |ui| {
                        ui.label(RichText::new(*o).font(mono(13.0)).color(TEXT));
                    }) {
                        pick = Some(i);
                    }
                }
            });
            if let Some(i) = pick {
                self.choose(i, host);
            }
        }
    }

    fn autoplay(&mut self, host: &mut Host) {
        self.hour = column_value(&self.grid, 2).to_string();
        self.minute = column_value(&self.grid, 1).to_string();
        self.serial = format!("{:02}{:02}{:02}{:02}", column_value(&self.grid, 7), column_value(&self.grid, 8), column_value(&self.grid, 9), column_value(&self.grid, 10));
        self.check_reading(host);
        self.choose(2, host);
    }
}
