//! Chapter 4: the escrowed phone. The Directorate's "Lawful Access" phones
//! work like the Clipper chip: every call carries a LEAF the escrow agents can
//! open. Matt Blaze showed in 1994 that its 16-bit checksum can be beaten by
//! trying random LEAFs until the receiving chip accepts one.

use std::rc::Rc;

use cpl_crypto::clipper::{LeafSearch, checksum, expected_tries, leaf_hex};
use egui::{Color32, RichText};

use crate::puzzle::{HandCell, HandSheet, Host, Puzzle, StepView, steps};
use crate::ui::theme::{AMBER, CYAN, DIM, GREEN, RED, TEXT, mono};
use crate::ui::widgets::{Formula, Kind, Log, Part, Tone, button_enabled, card, formula, lede, meter, note};
use crate::world::World;

const EST_PARTS: &[Part] = &[Part::Text("expected tries ≈ 2^"), Part::Slot];
const EST_PALETTE: &[&str] = &["8", "16", "32", "80", "128"];

pub struct ClipperLeaf {
    world: Rc<World>,
    honest: Option<[u8; 16]>,
    tampered: bool,
    estimate: Formula,
    estimated: bool,
    search: Option<LeafSearch>,
    running: bool,
    wiretapped: Option<String>,
    log: Log,
}

impl ClipperLeaf {
    pub fn new(world: Rc<World>) -> Self {
        Self { world, honest: None, tampered: false, estimate: Formula::new(EST_PARTS), estimated: false, search: None, running: false, wiretapped: None, log: Log::default() }
    }

    fn call(&mut self, host: &mut Host) {
        let w = self.world.clipper();
        let chip = &w.escrow.chips[w.chip];
        let leaf = w.escrow.honest_leaf(chip, &w.session_key, &w.iv);
        self.honest = Some(leaf);
        let tap = w.escrow.wiretap(&leaf);
        self.log.line(format!("call placed · LEAF {}", leaf_hex(&leaf)), Tone::Info);
        match tap {
            Ok((unit, sk)) => {
                self.log.line(format!("escrow agents open it: unit {unit:08X}, session key {} → the call is readable", cpl_crypto::bytes::to_hex(&sk)), Tone::Warn);
                host.status("Every call hands the agents its own key. That is the design.", Tone::Warn);
            }
            Err(e) => self.log.line(e, Tone::Bad),
        }
    }

    fn tamper(&mut self, host: &mut Host) {
        let w = self.world.clipper();
        let Some(mut leaf) = self.honest else { return };
        leaf[5] ^= 0x40;
        let ok = w.escrow.chip_accepts(&leaf, &w.session_key, &w.iv);
        self.tampered = true;
        self.log.line(format!("edited one byte of the LEAF → receiving chip: {}", if ok { "accepts" } else { "REFUSES (checksum)" }), if ok { Tone::Ok } else { Tone::Bad });
        host.status("The whole LEAF is encrypted under a family key you don't have, and the receiving chip checks a 16-bit checksum. You can't edit it into anything sensible. But 16 bits is only 65,536 possibilities.", Tone::Info);
    }

    fn check_estimate(&mut self, host: &mut Host) {
        if self.estimate.is(&[1]) {
            self.estimated = true;
            host.status("Right: a random LEAF passes a 16-bit check with probability 1/65,536. Expect about that many tries.", Tone::Ok);
        } else {
            let bits = self.estimate.tokens()[0].map(|t| EST_PALETTE[t]).unwrap_or("?");
            host.status(format!("2^{bits} is the work for a {bits}-bit check. What does the chip actually check?"), Tone::Bad);
        }
    }

    fn start_search(&mut self) {
        if self.search.is_none() {
            self.search = Some(LeafSearch::new(&format!("{}/leaf", self.world.seed_hex())));
        }
        self.running = true;
    }

    fn tick(&mut self, budget: u64, host: &mut Host) {
        let w = self.world.clipper();
        let Some(s) = self.search.as_mut() else { return };
        if s.step(&w.escrow, &w.session_key, &w.iv, budget) && self.running {
            self.running = false;
            let leaf = s.found.unwrap();
            self.log.line(format!("after {} tries: LEAF {} → receiving chip ACCEPTS", s.tries, leaf_hex(&leaf)), Tone::Ok);
            host.sfx("unlock");
            host.status(format!("Found one after {} tries. The chip is satisfied. Now see what the agents get.", s.tries), Tone::Ok);
        }
    }

    fn wiretap(&mut self, host: &mut Host) {
        let w = self.world.clipper();
        let Some(leaf) = self.search.as_ref().and_then(|s| s.found) else { return };
        let fields = w.escrow.open_leaf(&leaf);
        let result = match w.escrow.wiretap(&leaf) {
            Err(e) => e,
            Ok((unit, sk)) if sk != w.session_key => format!("unit {unit:08X}, \"session key\" {} → decrypts the call into noise", cpl_crypto::bytes::to_hex(&sk)),
            Ok(_) => "the real key (astronomically unlikely)".into(),
        };
        self.log.line(format!("agents decrypt the forged LEAF: unit id {:08X}, checksum {:02X}{:02X} → {result}", fields.unit_id, fields.checksum[0], fields.checksum[1]), Tone::Ok);
        self.wiretapped = Some(result);
        host.sfx("success");
        host.solve("The phone talks; the escrow office hears noise. Key escrow failed on 16 bits, exactly as it did in 1994.");
    }
}

impl Puzzle for ClipperLeaf {
    fn id(&self) -> &'static str {
        "ch4.clipper"
    }

    fn title(&self) -> &str {
        "ESCROWED PHONE"
    }

    fn subtitle(&self) -> Option<&str> {
        Some("the Clipper chip's LEAF · Blaze, 1994")
    }

    fn accent(&self) -> Color32 {
        RED
    }

    fn open(&mut self, host: &mut Host) {
        host.status("Place a call and see what the escrow agents get.", Tone::Info);
    }

    fn steps(&self) -> Vec<StepView> {
        steps(&[
            ("Place an honest call", "Watch the LEAF go out and the agents open it.", self.honest.is_some()),
            ("Try editing the LEAF", "Change a byte and see what the receiving chip says.", self.tampered),
            ("Estimate the work", "A random 128-bit LEAF passes the chip's check if its checksum happens to match.", self.estimated),
            ("Run the search", "Generate random LEAFs until the chip accepts one.", self.search.as_ref().is_some_and(|s| s.found.is_some())),
            ("Hand the agents your LEAF", "What do they recover from it?", self.wiretapped.is_some()),
        ])
    }

    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        if self.running {
            self.tick(4096, host);
            ui.ctx().request_repaint();
        }
        lede(ui, "Ibrahim's desk phone is a Directorate \"Lawful Access\" model. Every call it makes carries a **LEAF**: the phone's unit ID and the call's session key, encrypted for the escrow office, plus a 16-bit checksum. The phone at the other end refuses any call whose LEAF fails the checksum.");
        let mut act = 0;
        card(ui, Some("The phone"), None, |ui| {
            ui.horizontal(|ui| {
                if button_enabled(ui, self.honest.is_none(), "Place an honest call", Kind::Normal).clicked() {
                    act = 1;
                }
                if button_enabled(ui, self.honest.is_some() && !self.tampered, "Edit one byte of the LEAF", Kind::Normal).clicked() {
                    act = 2;
                }
            });
            let cs = {
                let w = self.world.clipper();
                checksum(&w.session_key, &w.iv)
            };
            note(ui, &format!("LEAF = E_family( unit ID (32 bits) ‖ E_unit(session key) (80 bits) ‖ checksum (16 bits) ). This call's checksum: {:02X}{:02X}.", cs[0], cs[1]));
        });
        if self.tampered {
            card(ui, Some("How much work?"), None, |ui| {
                formula(ui, EST_PARTS, EST_PALETTE, &mut self.estimate);
                if button_enabled(ui, !self.estimated && self.estimate.complete(), "Check estimate", Kind::Normal).clicked() {
                    act = 3;
                }
            });
        }
        if self.estimated {
            card(ui, Some("Blaze's search"), None, |ui| {
                let tries = self.search.as_ref().map(|s| s.tries).unwrap_or(0);
                let found = self.search.as_ref().is_some_and(|s| s.found.is_some());
                ui.horizontal(|ui| {
                    if button_enabled(ui, !self.running && !found, "Try random LEAFs", Kind::Primary).clicked() {
                        act = 4;
                    }
                    ui.label(RichText::new(format!("{tries} tries")).font(mono(14.0)).color(if found { GREEN } else { CYAN }));
                });
                meter(ui, (tries as f64 / (expected_tries(16) * 2.0)) as f32, if found { GREEN } else { AMBER });
                note(ui, "In 1994 each try took the chip about 38 ms, so an average search took 42 minutes. Here it takes a moment.");
                if found && button_enabled(ui, self.wiretapped.is_none(), "Make the call; give the agents this LEAF", Kind::Primary).clicked() {
                    act = 5;
                }
            });
        }
        match act {
            1 => self.call(host),
            2 => self.tamper(host),
            3 => self.check_estimate(host),
            4 => self.start_search(),
            5 => self.wiretap(host),
            _ => {}
        }
        self.log.show(ui, "clipper-log", 150.0);
        if let Some(r) = &self.wiretapped {
            ui.label(RichText::new(r).font(mono(12.5)).color(TEXT));
            note(ui, "Blaze's paper, *Protocol Failure in the Escrowed Encryption Standard*, ran on the front page of the New York Times. Clipper was dead within two years. The lesson outlived it: a system built so that someone else can always read your messages has to be perfect, and none is.");
        }
        let _ = DIM;
    }

    fn autoplay(&mut self, host: &mut Host) {
        self.call(host);
        self.tamper(host);
        self.estimate.set(&[1]);
        self.check_estimate(host);
        self.start_search();
        while self.running {
            self.tick(1 << 16, host);
        }
        self.wiretap(host);
    }

    fn deep_sheet(&self) -> Option<HandSheet> {
        Some(HandSheet::new(
            "the arithmetic of 16 bits",
            "Blaze's numbers. Round down where it says so.",
            150.0,
            vec![
                HandCell::new("tries expected for a 16-bit checksum (2^16)", "", 65_536),
                HandCell::new("seconds at 25 tries per second, rounded down", "65,536 ÷ 25", 2_621),
                HandCell::new("tries expected for a 32-bit checksum (2^32)", "", 4_294_967_296),
                HandCell::new("days at 25 tries per second, rounded down", "2^32 ÷ 25 ÷ 86,400", 1_988),
            ],
        ))
    }
}
