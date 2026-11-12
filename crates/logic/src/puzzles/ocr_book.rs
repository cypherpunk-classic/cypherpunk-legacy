//! Chapter 2: the Book. Grandpa's one-time-pad program is "munitions": it may
//! not leave Cryptopolis on a disk. Printed in a book, it is speech. Felix
//! printed it with a checksum after every line; the volunteers across the
//! water scanned it, and the scanner misread some characters. You fix the scan
//! line by line until every checksum matches, then compile it.

use std::rc::Rc;

use cpl_crypto::ocr::{CONFUSABLE, alternatives, corrupt, crc16, repairs};
use egui::{Color32, RichText, Sense};

use crate::puzzle::{Host, Puzzle, StepView, steps};
use crate::ui::theme::{self, AMBER, CYAN, DIM, GREEN, RED, TEXT, mono};
use crate::ui::widgets::{Kind, Tone, button_enabled, card, lede, note};
use crate::world::World;

pub const SOURCE: &[&str] = &[
    "/* otp.c - one-time pad. nightjar, 1994. Public domain. */",
    "#include <stdio.h>",
    "int main(int argc, char **argv) {",
    "    FILE *pad = fopen(argv[1], \"rb\");",
    "    int c, k;",
    "    if (pad == NULL) return 2;",
    "    while ((c = getchar()) != EOF) {",
    "        if ((k = fgetc(pad)) == EOF) return 1;",
    "        putchar(c ^ k);",
    "    }",
    "    return 0; /* now burn the pad. never use it twice. */",
    "}",
];

pub struct OcrBook {
    printed: Vec<u16>,
    lines: Vec<Vec<char>>,
    compiled: bool,
    tries: u32,
}

fn next_reading(c: char) -> char {
    for g in CONFUSABLE {
        if let Some(i) = g.chars().position(|x| x == c) {
            let v: Vec<char> = g.chars().collect();
            return v[(i + 1) % v.len()];
        }
    }
    c
}

impl OcrBook {
    pub fn new(world: Rc<World>) -> Self {
        let mut rng = world.rng("ch2/ocr");
        let printed = SOURCE.iter().map(|l| crc16(l.as_bytes())).collect();
        let lines = SOURCE
            .iter()
            .enumerate()
            .map(|(i, l)| {
                // about half the lines come back from the scanner damaged
                let n = if i % 2 == 0 && l.chars().any(|c| !alternatives(c).is_empty()) { 1 + (i % 4 == 0) as usize } else { 0 };
                corrupt(l, n, &mut rng).chars().collect()
            })
            .collect();
        Self { printed, lines, compiled: false, tries: 0 }
    }

    fn line(&self, i: usize) -> String {
        self.lines[i].iter().collect()
    }

    fn ok(&self, i: usize) -> bool {
        crc16(self.line(i).as_bytes()) == self.printed[i]
    }

    fn bad_lines(&self) -> usize {
        (0..self.lines.len()).filter(|i| !self.ok(*i)).count()
    }

    fn compile(&mut self, host: &mut Host) {
        self.tries += 1;
        if self.bad_lines() > 0 {
            host.status(format!("{} line(s) still fail their checksum. The compiler would choke on them.", self.bad_lines()), Tone::Bad);
            host.sfx("error");
            return;
        }
        let matches = (0..self.lines.len()).all(|i| self.line(i) == SOURCE[i]);
        if !matches {
            host.status("Every checksum matches, but a line is still wrong: two readings collided on a 16-bit checksum. Rare, and why real books also printed page checksums. Look again.", Tone::Warn);
            return;
        }
        self.compiled = true;
        host.sfx("unlock");
        host.solve("It compiles. Grandpa's program is on a server across the water now, and nobody exported anything but a book.");
    }
}

impl Puzzle for OcrBook {
    fn id(&self) -> &'static str {
        "ch2.book"
    }

    fn title(&self) -> &str {
        "THE BOOK"
    }

    fn subtitle(&self) -> Option<&str> {
        Some("source code as speech · CRC-16 per line")
    }

    fn accent(&self) -> Color32 {
        AMBER
    }

    fn open(&mut self, host: &mut Host) {
        host.status(format!("The scan is back. {} line(s) fail their checksum.", self.bad_lines()), Tone::Info);
    }

    fn steps(&self) -> Vec<StepView> {
        let bad = self.bad_lines();
        steps(&[
            ("Find the lines whose checksum fails", "Each line's printed checksum is on the right; red means the scan differs.", bad < SOURCE.len()),
            ("Fix each red line", "Click a character to cycle through what the scanner might have confused it with: 0 O o, 1 l I |, 5 S, 8 B, e c, ( {, , .", bad == 0),
            ("Compile the scan", "When every line is green, compile it.", self.compiled),
        ])
    }

    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        lede(ui, "Under the Directorate's export rules this program is a munition: it may not leave on a disk. A book is speech. Felix printed it in a scanner-friendly font with a CRC after every line; the volunteers across the water scanned it, and the scanner misread a few characters.");
        let mut click: Option<(usize, usize)> = None;
        card(ui, Some("The scan"), None, |ui| {
            egui::Grid::new("ocr").num_columns(3).spacing([10.0, 2.0]).show(ui, |ui| {
                for (i, line) in self.lines.iter().enumerate() {
                    let ok = crc16(line.iter().collect::<String>().as_bytes()) == self.printed[i];
                    ui.label(RichText::new(format!("{:>2}", i + 1)).font(mono(12.0)).color(DIM));
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        for (j, c) in line.iter().enumerate() {
                            let confusable = !alternatives(*c).is_empty();
                            let color = if ok {
                                TEXT
                            } else if confusable && host.hints {
                                AMBER
                            } else {
                                theme::rgb(0xc3cee2)
                            };
                            let text = if *c == ' ' { "\u{00a0}".to_string() } else { c.to_string() };
                            let r = ui.add(egui::Label::new(RichText::new(text).font(mono(13.0)).color(color)).sense(if confusable { Sense::click() } else { Sense::hover() }));
                            if confusable && r.clicked() {
                                click = Some((i, j));
                            }
                        }
                    });
                    let now = crc16(line.iter().collect::<String>().as_bytes());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("{:04X}", self.printed[i])).font(mono(12.5)).color(AMBER));
                        ui.label(RichText::new(if ok { "✓".to_string() } else { format!("≠ {now:04X}") }).font(mono(12.5)).color(if ok { GREEN } else { RED }));
                    });
                    ui.end_row();
                }
            });
        });
        if let Some((i, j)) = click {
            self.lines[i][j] = next_reading(self.lines[i][j]);
            host.sfx("blip");
            if self.ok(i) {
                host.status(format!("Line {} checks out.", i + 1), Tone::Ok);
            }
        }
        note(ui, "A 16-bit CRC catches any single misread character and almost every pair. It is not a signature: anyone can compute it. It protects against accidents, not against a forger. (For that, the PGP releases were signed.)");
        if button_enabled(ui, !self.compiled, "Compile the scanned source", Kind::Primary).clicked() {
            self.compile(host);
        }
        if self.compiled {
            ui.label(RichText::new("$ cc otp.c -o otp && echo ok\nok").font(mono(13.0)).color(CYAN));
        }
    }

    fn autoplay(&mut self, host: &mut Host) {
        for i in 0..self.lines.len() {
            if !self.ok(i) {
                let fixes = repairs(&self.line(i), self.printed[i], 2);
                let pick = fixes.iter().find(|f| f.as_str() == SOURCE[i]).or(fixes.first()).cloned();
                if let Some(f) = pick {
                    self.lines[i] = f.chars().collect();
                }
            }
        }
        self.compile(host);
    }
}
