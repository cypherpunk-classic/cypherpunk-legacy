//! Chapter 2, part 3. The Directorate forces links down to a weak,
//! "export-grade" group (echoing the real Logjam attack on 512-bit DH). With
//! a 31-bit subgroup, baby-step giant-step recovers the secret exponent in
//! about 2·√q steps instead of q. The player sizes the attack: a table of m
//! entries, then up to q/m giant steps, inside the switchboard's memory and
//! before the link re-keys. Too small a table is too slow, too big does not
//! fit; √q is the sweet spot.

use std::rc::Rc;

use cpl_crypto::bytes::Big;
use cpl_crypto::dh::{Bsgs, BsgsPhase, WEAK_GROUP, sqrt_ceil};
use egui::{Color32, RichText};

use crate::puzzle::{HandCell, HandSheet, Host, Puzzle, StepView, steps};
use crate::ui::theme::{AMBER, CYAN, DIM, GREEN, RED, TEXT, mono};
use crate::ui::widgets::{Kind, Tone, button_enabled, card, kv, lede, math, meter, note};
use crate::world::World;

/// Table entries the switchboard has room for.
pub const MEMORY: u64 = 1 << 17;
/// Steps of work before the link re-keys and the intercept is worthless.
pub const BUDGET: u64 = 120_000;
const LOG_MIN: f32 = 8.0;
const LOG_MAX: f32 = 20.0;

pub struct Logjam {
    target: u64,
    log_m: f32,
    run: Option<Bsgs>,
    out: Option<(String, Color32)>,
    tried: bool,
    tried_fitting: bool,
}

fn worst_case(m: u64) -> u64 {
    m + WEAK_GROUP.q.div_ceil(m.max(1))
}

impl Logjam {
    pub fn new(world: Rc<World>) -> Self {
        let g = WEAK_GROUP;
        // a secret near the top of the range: the attack has to be sized right, not lucky
        let lo = g.q / 10 * 9;
        let secret = u64::try_from(world.rng("ch2/logjam").range(&Big::from(lo), &Big::from(g.q - 1))).unwrap();
        Self { target: g.public(secret), log_m: 12.0, run: None, out: None, tried: false, tried_fitting: false }
    }

    fn m(&self) -> u64 {
        (2f64.powf(self.log_m as f64)).round() as u64
    }

    fn start(&mut self, host: &mut Host) {
        if self.run.is_some() || host.solved() {
            return;
        }
        let m = self.m();
        self.tried = true;
        if m > MEMORY {
            self.out = Some((format!("OUT OF MEMORY: a table of {m} entries does not fit in the switchboard's {MEMORY} slots. A bigger table means fewer giant steps, but it has to live somewhere."), RED));
            host.status("The table does not fit. Make it smaller.", Tone::Bad);
            host.sfx("error");
            return;
        }
        self.tried_fitting = true;
        self.run = Some(Bsgs::new(WEAK_GROUP, self.target, m));
        self.out = Some(("Running…".into(), DIM));
    }

    fn tick(&mut self, host: &mut Host, slice: u64) {
        let Some(run) = self.run.as_mut() else { return };
        let finished = run.step(slice);
        let work = run.baby_steps() + run.giant_steps();
        if work > BUDGET {
            let (b, gs) = (run.baby_steps(), run.giant_steps());
            self.run = None;
            self.out = Some((format!("Too slow: the link re-keyed after {BUDGET} steps ({b} baby, {gs} giant), and the intercept is worthless now. Worst case at this m is {} steps.", worst_case(self.m())), RED));
            host.status("Out of time. Balance the table against the search.", Tone::Bad);
            host.sfx("error");
            return;
        }
        if finished {
            self.finish(host);
        }
    }

    fn finish(&mut self, host: &mut Host) {
        let Some(run) = self.run.take() else { return };
        let g = WEAK_GROUP;
        match run.result {
            Some(x) if g.public(x) == self.target => {
                self.out = Some((format!("Recovered a = {x} in {} baby steps + {} giant steps. Brute force would have needed up to {}.", run.baby_steps(), run.giant_steps(), g.q), GREEN));
                host.sfx("unlock");
                host.solve(format!("Secret exponent recovered: a = {x}. “Export-grade” means readable. Strong groups (2048-bit, or elliptic curves) put √q out of reach — until quantum computers, which is Chapter 6."));
            }
            _ => {
                self.out = Some((format!("No match with m = {}.", run.m), RED));
                host.status("Attack failed with that step size.", Tone::Bad);
                host.sfx("error");
            }
        }
    }
}

impl Puzzle for Logjam {
    fn id(&self) -> &'static str {
        "ch2.logjam"
    }

    fn title(&self) -> &str {
        "DOWNGRADE · EXPORT-GRADE DH"
    }

    fn subtitle(&self) -> Option<&str> {
        Some("q ≈ 2³¹, baby-step giant-step")
    }

    fn accent(&self) -> Color32 {
        AMBER
    }

    fn open(&mut self, host: &mut Host) {
        host.status("Size the attack: recover the secret exponent before the link re-keys.", Tone::Info);
    }

    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        // advance a running attack a slice per frame
        if self.run.is_some() {
            self.tick(host, 4_000);
            ui.ctx().request_repaint();
        }
        let g = WEAK_GROUP;
        lede(ui, "To comply with the Lawful Access Act, the Directorate downgrades every link it cannot read to an old \"export-grade\" group: a legacy of the 1990s, when strong cryptography was export-controlled. This one has a 31-bit subgroup. That was the real Logjam attack in 2015, and it is why weak groups are dangerous even today.");
        card(ui, Some("Intercepted exchange"), None, |ui| {
            kv(ui, "lj-kv", &[("group order q", format!("{}  (about 2^31)", g.q)), ("generator g", g.g.to_string()), ("public value A = g^a", self.target.to_string()), ("secret a", "this is what we want".into())]);
        });
        let mut run_clicked = false;
        card(ui, Some("Baby-step giant-step"), None, |ui| {
            note(ui, "Brute force would try up to q ≈ 2 billion exponents. Shanks's trick writes a = i·m + j: tabulate g^j for every j < m (baby steps), then step A·(g^−m)^i until a table entry matches (giant steps). You choose m.");
            math(ui, "a = i·m + j,   store g^j for j < m,   seek A·(g^−m)^i ∈ table");
            kv(ui, "lj-limits", &[("switchboard memory", format!("{MEMORY} table entries")), ("link re-keys after", format!("{BUDGET} steps of work"))]);
            let busy = self.run.is_some();
            ui.add_enabled_ui(!busy && !host.solved(), |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("table size m =").font(mono(13.0)).color(DIM));
                    ui.add(egui::Slider::new(&mut self.log_m, LOG_MIN..=LOG_MAX).show_value(false).step_by(0.05));
                    ui.label(RichText::new(self.m().to_string()).font(mono(14.0)).color(TEXT));
                });
            });
            let m = self.m();
            let fits = m <= MEMORY;
            let worst = worst_case(m);
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("table: {m} entries")).font(mono(12.5)).color(if fits { GREEN } else { RED }));
                ui.label(RichText::new(format!("· giant steps: up to {}", g.q.div_ceil(m))).font(mono(12.5)).color(DIM));
                ui.label(RichText::new(format!("· worst case: {worst} steps")).font(mono(12.5)).color(if worst <= BUDGET { GREEN } else { AMBER }));
            });
            let (baby, giant) = match &self.run {
                Some(r) => (r.baby_fraction(), if r.phase == BsgsPhase::Baby { 0.0 } else { r.giant_fraction() }),
                None => match &self.out {
                    Some((_, c)) if *c == GREEN => (1.0, 1.0),
                    _ => (0.0, 0.0),
                },
            };
            note(ui, "baby steps (building table)");
            meter(ui, baby, CYAN);
            note(ui, "giant steps (searching)");
            meter(ui, giant, AMBER);
            if button_enabled(ui, !busy && !host.solved(), "Run the attack", Kind::Primary).clicked() {
                run_clicked = true;
            }
            if let Some((t, c)) = &self.out {
                ui.add(egui::Label::new(RichText::new(t).font(mono(13.0)).color(*c)).wrap());
            }
        });
        if run_clicked {
            self.start(host);
        }
    }

    fn autoplay(&mut self, host: &mut Host) {
        self.log_m = (sqrt_ceil(WEAK_GROUP.q) as f32).log2();
        self.start(host);
        while self.run.is_some() {
            self.tick(host, 1 << 20);
        }
    }

    fn steps(&self) -> Vec<StepView> {
        steps(&[
            ("Choose a table size m and run", "The table holds m entries; the search then takes up to q/m giant steps.", self.tried),
            ("Fit the table in the switchboard's memory", "At most 131,072 entries.", self.tried_fitting),
            ("Finish before the link re-keys", "Total work ≈ m + q/m. For which m is that smallest?", matches!(&self.out, Some((_, c)) if *c == GREEN)),
        ])
    }

    fn deep_sheet(&self) -> Option<HandSheet> {
        let q = WEAK_GROUP.q;
        let m = sqrt_ceil(q);
        Some(HandSheet::new(
            "how big is big enough",
            &format!("The same arithmetic the Logjam authors did, without a computer. q = {q}."),
            180.0,
            vec![
                HandCell::new("⌈√q⌉", "about 46 thousand", m as i64),
                HandCell::new("worst-case work at that m: m + ⌈q/m⌉", "baby steps plus giant steps", worst_case(m) as i64),
                HandCell::new("brute force instead: q − 1 tries", "", (q - 1) as i64),
                HandCell::new("a 256-bit curve group: about 2^? steps", "the square root of 2^256", 128),
                HandCell::new("a 2048-bit prime with a 224-bit subgroup: 2^?", "√ again", 112),
            ],
        ))
    }
}
