//! Chapter 5: Hashcash. The Directorate floods the remailers with junk so
//! that every batch is mostly its own messages (the "n − 1" attack on a mix).
//! Price each message in proof-of-work: cheap for a person, ruinous for a
//! flood. Set the price, mint a stamp at that price on this machine, spend it,
//! and see a replayed stamp bounce.

use cpl_crypto::drbg::Drbg;
use cpl_crypto::hashcash::{Minter, hash_hex, leading_zero_bits, stamp_hash, verify};
use egui::{Color32, RichText};

use crate::puzzle::{HandCell, HandSheet, Host, Puzzle, StepView, steps};
use crate::script::Ctx;
use crate::ui::theme::{AMBER, CYAN, DIM, GREEN, RED, TEXT, mono};
use crate::ui::widgets::{Kind, Log, Tone, button_enabled, card, lede, meter, note};

const RESOURCE: &str = "remailers@masquerade";
/// The Directorate's farm, in machines like yours, and the junk it needs per hour.
const FARM: f64 = 100.0;
const FLOOD_NEEDED: f64 = 200_000.0;
/// The most a person will wait for one stamp.
const PATIENCE: f64 = 4.0;
pub const STAMPS_NEEDED: usize = 3;

pub struct Stamps {
    ctx: Ctx,
    rng: Drbg,
    /// hashes per second on this machine, measured when the terminal opened
    rate: f64,
    bits: u32,
    price: Option<u32>,
    minter: Option<Minter>,
    minted: Vec<String>,
    spent: Vec<String>,
    replay_tried: bool,
    log: Log,
}

/// Time a few thousand hashes to estimate this machine's speed.
fn measure_rate() -> f64 {
    let start = web_time::Instant::now();
    let mut m = Minter { header: "1:64:000000:measure::x:".into(), bits: 64, counter: 0, best: 0, found: None };
    let mut n = 0u64;
    while start.elapsed().as_secs_f64() < 0.02 || n < 2_000 {
        m.step(500);
        n += 500;
        if n > 2_000_000 {
            break;
        }
    }
    (n as f64 / start.elapsed().as_secs_f64().max(1e-6)).max(10_000.0)
}

impl Stamps {
    pub fn new(ctx: Ctx) -> Self {
        let rng = ctx.world().rng("ch5/stamps");
        Self { ctx, rng, rate: measure_rate(), bits: 12, price: None, minter: None, minted: Vec::new(), spent: Vec::new(), replay_tried: false, log: Log::default() }
    }

    fn your_seconds(&self, bits: u32) -> f64 {
        2f64.powi(bits as i32) / self.rate
    }

    fn flood_per_hour(&self, bits: u32) -> f64 {
        FARM * self.rate * 3600.0 / 2f64.powi(bits as i32)
    }

    /// The cheapest price that stops the flood.
    pub fn best_bits(&self) -> u32 {
        (8..40).find(|b| self.flood_per_hour(*b) < FLOOD_NEEDED).unwrap_or(30)
    }

    fn set_price(&mut self, host: &mut Host) {
        let (you, flood) = (self.your_seconds(self.bits), self.flood_per_hour(self.bits));
        if flood >= FLOOD_NEEDED {
            host.status(format!("At {} bits their farm still makes {:.0} junk messages an hour; they need {FLOOD_NEEDED:.0}. Raise the price.", self.bits, flood), Tone::Bad);
            return;
        }
        if you > PATIENCE {
            host.status(format!("At {} bits your own message costs {:.1} s of work. Nobody will wait that long; the remailers would empty out. Lower it.", self.bits, you), Tone::Bad);
            return;
        }
        self.price = Some(self.bits);
        host.sfx("confirm");
        host.status(format!("Price set: {} zero bits. About {:.1} s for you, and their flood drops to {:.0} an hour.", self.bits, you, flood), Tone::Ok);
    }

    fn start_mint(&mut self) {
        if let Some(b) = self.price {
            self.minter = Some(Minter::new(b, "261113", RESOURCE, &mut self.rng));
        }
    }

    fn tick(&mut self, budget: u64, host: &mut Host) {
        let Some(m) = self.minter.as_mut() else { return };
        if m.step(budget) {
            let stamp = m.found.clone().unwrap();
            self.log.line(format!("minted after {} hashes: {stamp}", m.tries()), Tone::Ok);
            self.minted.push(stamp);
            self.minter = None;
            host.sfx("confirm");
            if self.minted.len() == 1 {
                host.status("A stamp. Checking it takes the board one hash.", Tone::Ok);
            }
            if self.minted.len() >= STAMPS_NEEDED && self.spent.is_empty() {
                host.status("Three stamps in hand. Post one.", Tone::Info);
            }
        }
    }

    fn post(&mut self, replay: bool, host: &mut Host) {
        let Some(price) = self.price else { return };
        let stamp = if replay { self.spent.last().cloned() } else { self.minted.first().cloned() };
        let Some(stamp) = stamp else { return };
        match verify(&stamp, RESOURCE, price, &self.spent) {
            Ok(bits) => {
                self.log.line(format!("board: SHA-256 = {}… has {bits} zero bits → accepted", &hash_hex(&stamp)[..16]), Tone::Ok);
                self.spent.push(stamp.clone());
                self.minted.retain(|s| *s != stamp);
                host.status("Posted. One hash to check, a few million to make.", Tone::Ok);
            }
            Err(e) => {
                self.log.line(format!("board: {e}"), Tone::Bad);
                if replay {
                    self.replay_tried = true;
                    host.status("Replays bounce: the board remembers every stamp it has accepted. Work cannot be spent twice.", Tone::Ok);
                }
            }
        }
        self.check_done(host);
    }

    fn check_done(&mut self, host: &mut Host) {
        if !host.solved() && self.replay_tried && !self.spent.is_empty() && self.minted.len() + 1 >= STAMPS_NEEDED {
            let keep: Vec<String> = self.minted.clone();
            {
                let mut s = self.ctx.state();
                for st in keep {
                    s.add_stamp(st);
                }
            }
            self.minted.clear();
            host.sfx("unlock");
            host.solve("Stamps in your wallet for the broadcast. Adam Back posted Hashcash to the Cypherpunks list in 1997 for exactly this; eleven years later Bitcoin used the same puzzle to keep time.");
        }
    }
}

impl Puzzle for Stamps {
    fn id(&self) -> &'static str {
        "ch5.stamps"
    }

    fn title(&self) -> &str {
        "HASHCASH"
    }

    fn subtitle(&self) -> Option<&str> {
        Some("proof-of-work postage · Back, 1997")
    }

    fn accent(&self) -> Color32 {
        CYAN
    }

    fn open(&mut self, host: &mut Host) {
        host.status(format!("This machine does about {:.1} million hashes a second. Price the remailers' postage.", self.rate / 1e6), Tone::Info);
    }

    fn steps(&self) -> Vec<StepView> {
        steps(&[
            ("Set the price", "High enough that the flood stops, low enough that you'd still pay it.", self.price.is_some()),
            ("Mint three stamps", "Search counters until SHA-256 of the stamp starts with that many zero bits.", self.minted.len() + self.spent.len() >= STAMPS_NEEDED),
            ("Post one, then replay it", "The board checks with one hash, and keeps the stamps it has seen.", !self.spent.is_empty() && self.replay_tried),
        ])
    }

    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        if self.minter.is_some() {
            self.tick((self.rate / 30.0) as u64, host);
            ui.ctx().request_repaint();
        }
        lede(ui, &format!("The Directorate floods the remailers so that every batch is mostly its own messages; then the one real message in a batch is easy to follow (the **n − 1 attack**). To keep it up they need **{FLOOD_NEEDED:.0}** messages an hour, and they have a farm of about **{FARM:.0}** machines like this one. Price every message in work."));
        let mut act = 0;
        card(ui, Some("1 · The price"), None, |ui| {
            ui.add(egui::Slider::new(&mut self.bits, 8..=30).text("zero bits required"));
            let (you, flood) = (self.your_seconds(self.bits), self.flood_per_hour(self.bits));
            ui.label(RichText::new(format!("your cost per message: 2^{} / {:.2e} ≈ {:.2} s", self.bits, self.rate, you)).font(mono(13.0)).color(if you <= PATIENCE { GREEN } else { RED }));
            ui.label(RichText::new(format!("their flood: {FARM:.0} × {:.2e} × 3600 / 2^{} ≈ {:.0} per hour", self.rate, self.bits, flood)).font(mono(13.0)).color(if flood < FLOOD_NEEDED { GREEN } else { RED }));
            if button_enabled(ui, self.price.is_none(), "Set this price", Kind::Primary).clicked() {
                act = 1;
            }
        });
        if self.price.is_some() {
            card(ui, Some("2 · Mint"), None, |ui| {
                let have = self.minted.len() + self.spent.len();
                ui.horizontal(|ui| {
                    if button_enabled(ui, self.minter.is_none() && have < STAMPS_NEEDED, "Mint a stamp", Kind::Primary).clicked() {
                        act = 2;
                    }
                    ui.label(RichText::new(format!("{have} / {STAMPS_NEEDED}")).font(mono(14.0)).color(AMBER));
                });
                if let Some(m) = &self.minter {
                    ui.label(RichText::new(format!("{} tries · best so far {} zero bits", m.tries(), m.best)).font(mono(12.5)).color(CYAN));
                    meter(ui, m.tries() as f32 / 2f32.powi(m.bits as i32), CYAN);
                }
                for s in &self.minted {
                    ui.label(RichText::new(s).font(mono(11.5)).color(TEXT));
                    ui.label(RichText::new(format!("SHA-256 → {} ({} zero bits)", &hash_hex(s)[..24], leading_zero_bits(&stamp_hash(s)))).font(mono(11.0)).color(DIM));
                }
            });
            card(ui, Some("3 · Post"), None, |ui| {
                ui.horizontal(|ui| {
                    if button_enabled(ui, !self.minted.is_empty() && self.spent.is_empty(), "Post with a stamp", Kind::Primary).clicked() {
                        act = 3;
                    }
                    if button_enabled(ui, !self.spent.is_empty() && !self.replay_tried, "Post again with the same stamp", Kind::Normal).clicked() {
                        act = 4;
                    }
                });
            });
        }
        match act {
            1 => self.set_price(host),
            2 => self.start_mint(),
            3 => self.post(false, host),
            4 => self.post(true, host),
            _ => {}
        }
        self.log.show(ui, "stamp-log", 110.0);
        note(ui, "Hashcash did not end spam: a botnet has a lot of machines. What it does is put a price on volume, and volume is what floods and n − 1 attacks need.");
    }

    fn autoplay(&mut self, host: &mut Host) {
        self.bits = self.best_bits();
        self.set_price(host);
        if self.price.is_none() {
            // a very slow machine: accept a longer wait rather than fail
            self.price = Some(self.bits);
        }
        while self.minted.len() < STAMPS_NEEDED {
            self.start_mint();
            while self.minter.is_some() {
                self.tick(1 << 20, host);
            }
        }
        self.post(false, host);
        self.post(true, host);
    }

    fn deep_sheet(&self) -> Option<HandSheet> {
        Some(HandSheet::new(
            "the price of a stamp",
            "Work it out for a 20-bit stamp.",
            150.0,
            vec![
                HandCell::new("expected hashes for 20 zero bits (2^20)", "", 1_048_576),
                HandCell::new("hashes per second to mint one in 2 s", "2^20 ÷ 2", 524_288),
                HandCell::new("a farm of 100 machines at 1,000,000 hashes/s: stamps per hour, rounded down", "100 × 10^6 × 3600 ÷ 2^20", 343_322),
            ],
        ))
    }
}
