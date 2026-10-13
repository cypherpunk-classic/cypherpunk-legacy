//! Chapter 2, part 3. The Directorate forces links down to a weak,
//! "export-grade" group (echoing the real Logjam attack on 512-bit DH). With
//! a 31-bit subgroup, baby-step giant-step recovers the secret exponent in
//! about 2·√q steps instead of q, so the "downgraded for compliance" channel
//! is readable in seconds.

use std::rc::Rc;

use cpl_crypto::bytes::Big;
use cpl_crypto::dh::{Bsgs, BsgsPhase, WEAK_GROUP, sqrt_ceil};
use egui::{Color32, RichText};

use crate::puzzle::{Host, Puzzle};
use crate::ui::theme::{self, AMBER, CYAN, DIM, GREEN, RED, mono};
use crate::ui::widgets::{Kind, Tone, button, card, kv, lede, math, meter, note};
use crate::world::World;

pub struct Logjam {
    target: u64,
    m_input: String,
    run: Option<Bsgs>,
    started: f64,
    out: Option<(String, Color32)>,
}

impl Logjam {
    pub fn new(world: Rc<World>) -> Self {
        let g = WEAK_GROUP;
        let secret = u64::try_from(world.rng("ch2/logjam").range(&Big::from(1_000_000u64), &Big::from(g.q - 1))).unwrap();
        Self { target: g.public(secret), m_input: sqrt_ceil(g.q).to_string(), run: None, started: 0.0, out: None }
    }

    fn start(&mut self, host: &mut Host) {
        if self.run.is_some() || host.solved() {
            return;
        }
        let m = self.m_input.trim().parse::<u64>().unwrap_or_else(|_| sqrt_ceil(WEAK_GROUP.q)).max(1);
        self.run = Some(Bsgs::new(WEAK_GROUP, self.target, m));
        self.started = host.time;
        self.out = Some(("Running…".into(), DIM));
    }

    fn finish(&mut self, host: &mut Host) {
        let Some(run) = self.run.take() else { return };
        let g = WEAK_GROUP;
        let ms = ((host.time - self.started) * 1000.0).max(1.0);
        match run.result {
            Some(x) if g.public(x) == self.target => {
                self.out = Some((
                    format!(
                        "Recovered a = {x} in {ms:.0} ms — {} baby steps + {} giant steps. Brute force would have been up to {}.",
                        run.baby_steps(),
                        run.giant_steps(),
                        g.q
                    ),
                    GREEN,
                ));
                host.sfx("unlock");
                host.solve(format!(
                    "Secret exponent recovered: a = {x}. “Export-grade” means readable. Strong groups (2048-bit, or elliptic curves) put √q out of reach — until quantum computers, which is Chapter 6."
                ));
            }
            _ => {
                self.out = Some((format!("No match with m = {}. The step size must satisfy m · (giant steps) ≥ q; try √q.", run.m), RED));
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
        host.status("Recover the secret exponent from the public value.", Tone::Info);
    }

    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        // advance a running attack a slice per frame
        let mut finished = false;
        if let Some(run) = self.run.as_mut() {
            finished = run.step(250_000);
        }
        if finished {
            self.finish(host);
        }
        let g = WEAK_GROUP;
        lede(ui, "To comply with the Lawful Access Act, the Directorate downgrades every link it cannot read to an old \"export-grade\" group: a legacy of the 1990s, when strong cryptography was export-controlled. This one has a 31-bit subgroup. That was the real Logjam attack in 2015, and it is why weak groups are dangerous even today.");
        card(ui, Some("Intercepted exchange"), None, |ui| {
            kv(ui, "lj-kv", &[("group order q", format!("{}  (about 2^31)", g.q)), ("generator g", g.g.to_string()), ("public value A = g^a", self.target.to_string()), ("secret a", "this is what we want".into())]);
        });
        let mut run_clicked = false;
        card(ui, Some("Baby-step giant-step"), None, |ui| {
            note(ui, "Brute force would try up to q ≈ 2 billion exponents. Shanks's trick writes a = i·m + j with m ≈ √q: tabulate g^j for all j (baby steps), then step A·(g^−m)^i until a table entry matches (giant steps). About 2√q work, √q memory.");
            math(ui, "a = i·m + j,   store g^j for j < m,   seek A·(g^−m)^i ∈ table");
            ui.horizontal(|ui| {
                ui.label(RichText::new("step size m =").font(mono(13.0)).color(DIM));
                ui.add(egui::TextEdit::singleline(&mut self.m_input).desired_width(110.0).font(mono(14.0)));
                if button(ui, "optimal √q", Kind::Ghost).clicked() {
                    self.m_input = sqrt_ceil(g.q).to_string();
                }
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
            if button(ui, "Run the attack", Kind::Primary).clicked() {
                run_clicked = true;
            }
            if let Some((t, c)) = &self.out {
                ui.add(egui::Label::new(RichText::new(t).font(mono(13.0)).color(*c)).wrap());
            }
        });
        if run_clicked {
            self.start(host);
        }
        let _ = theme::TEXT;
    }

    fn autoplay(&mut self, host: &mut Host) {
        self.m_input = sqrt_ceil(WEAK_GROUP.q).to_string();
        self.start(host);
        if let Some(run) = self.run.as_mut() {
            while !run.step(1 << 20) {}
        }
        self.finish(host);
    }
}
