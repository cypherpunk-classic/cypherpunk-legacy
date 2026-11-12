//! Chapter 4, part 2. The Directorate's "joint custody" offers a 2-of-2
//! escrow: your key and theirs, so neither can act alone. To show Ibrahim the
//! flaw, the player plays the Directorate: with naive aggregation
//! (Q = P_you + P_DLA) they pick a rogue key that cancels yours and seize sole
//! control. Then the same trick against BIP-327 MuSig2 KeyAgg, which weights
//! each key by a hash of the whole list: every attempt to correct for the
//! weights changes the weights.

use std::rc::Rc;

use cpl_crypto::bip340::{sign, verify};
use cpl_crypto::bytes::{Big, inv_mod_i64, to_hex};
use cpl_crypto::dh::TOY_GROUP;
use cpl_crypto::musig::{key_agg, naive_aggregate};
use cpl_crypto::secp::{Pt, compressed, g, inv_n, mul, mul_pt, x_only};
use egui::{Color32, RichText};

use crate::puzzle::{HandCell, HandSheet, Host, Puzzle, StepView, steps};
use crate::ui::theme::{self, AMBER, GREEN, RED, retro};
use crate::ui::widgets::{Formula, Kind, Log, Part, Tone, button, button_enabled, card, formula, kv, lede, math, note, shorten};
use crate::world::World;

const PARTS: &[Part] = &[Part::Text("P_DLA = "), Part::Slot, Part::Text(" − "), Part::Slot];
const PALETTE: &[&str] = &["T", "P_you", "Q", "G"];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    Naive,
    Musig,
}

pub struct RogueKey {
    world: Rc<World>,
    dir_secret: Big,
    stage: Stage,
    rogue: Formula,
    /// the naive attack worked (the contract key equalled T)
    seized: bool,
    honest_tried: bool,
    /// MuSig2 attempts: the rogue key announced, and the coefficients that came back
    musig_tries: Vec<(Pt, Big, Big)>,
    log: Log,
}

impl RogueKey {
    pub fn new(world: Rc<World>) -> Self {
        let dir_secret = world.rng("ch4/dir").scalar(&(Big::from(1u8) << 250));
        Self { world, dir_secret, stage: Stage::Naive, rogue: Formula::new(PARTS), seized: false, honest_tried: false, musig_tries: Vec::new(), log: Log::default() }
    }

    fn you(&self) -> Pt {
        mul(&self.world.identity().x)
    }

    fn target(&self) -> Pt {
        mul(&self.dir_secret)
    }

    fn honest(&mut self, host: &mut Host) {
        let q = naive_aggregate(&[self.you(), self.target()]);
        self.honest_tried = true;
        self.log.line(format!("Directorate announces an honest P_DLA = d_DLA·G. Q = {}: it needs both keys.", shorten(&to_hex(&x_only(&q)), 8, 6)), Tone::Dim);
        host.status("Honestly, Q needs both keys. But nothing forces them to be honest. Now play them, and cheat.", Tone::Info);
    }

    fn naive(&mut self, host: &mut Host) {
        let toks = self.rogue.tokens();
        let (Some(a), Some(b)) = (toks[0], toks[1]) else { return };
        if a == 2 || b == 2 {
            host.status("Q is the contract key: it does not exist until both keys are published. Build P_DLA from what is already known.", Tone::Bad);
            host.sfx("error");
            return;
        }
        let val = |t: usize| match t {
            0 => self.target(),
            1 => self.you(),
            _ => g(),
        };
        let dla = val(a) - val(b);
        let q = naive_aggregate(&[self.you(), dla]);
        self.log.line(format!("You (as the Directorate) announce P_DLA = {} − {}. Q = P_you + P_DLA = {}", PALETTE[a], PALETTE[b], shorten(&to_hex(&x_only(&q)), 8, 6)), Tone::Info);
        if q != self.target() {
            self.log.line("Q is not T: you would still need the other party's key to sign.", Tone::Dim);
            host.status("Q ≠ T. Expand Q = P_you + P_DLA with your choice: what has to cancel?", Tone::Bad);
            host.sfx("error");
            return;
        }
        // the Directorate signs alone, and the contract accepts
        let m = b"MOVE EVERYTHING TO THE DIRECTORATE";
        let solo = sign(m, &self.dir_secret, None);
        let accepted = verify(&solo, m, &x_only(&q));
        self.log.line(format!("Q == T, a key the Directorate alone holds. A signature with t alone… contract accepts? {}", if accepted { "YES" } else { "no" }), if accepted { Tone::Bad } else { Tone::Ok });
        self.seized = accepted;
        host.sfx("error");
        host.status("Sole control, without ever knowing your key. MuSig2 is the fix.", Tone::Bad);
    }

    fn to_musig(&mut self, host: &mut Host) {
        self.stage = Stage::Musig;
        host.status("Now the same keys, aggregated with MuSig2. Try the trick again.", Tone::Info);
    }

    /// Announce a rogue key against MuSig2: first the naive one, then one
    /// corrected for the coefficients the last attempt produced.
    fn musig(&mut self, correct: bool, host: &mut Host) {
        if host.solved() {
            return;
        }
        let (you, target) = (self.you(), self.target());
        let dla = match (correct, self.musig_tries.last()) {
            (true, Some((_, a_you, a_dla))) => {
                // P_DLA = (T − a_you·P_you) / a_DLA, using the last coefficients
                mul_pt(&inv_n(a_dla), &(target - mul_pt(a_you, &you)))
            }
            _ => target - you,
        };
        let pks = [compressed(&you), compressed(&dla)];
        let Ok(agg) = key_agg(&pks) else { return };
        let (a1, a2) = (agg.coefficients[0].clone(), agg.coefficients[1].clone());
        let hit = agg.q == target;
        if let Some((_, prev, _)) = self.musig_tries.last()
            && correct
        {
            self.log.line(format!("a_you was {} last time; with your new P_DLA in the list it is now {}.", shorten(&prev.to_str_radix(16), 6, 4), shorten(&a1.to_str_radix(16), 6, 4)), Tone::Warn);
        }
        self.log.line(format!("MuSig2: a_you = {}, a_DLA = {}. Q = a_you·P_you + a_DLA·P_DLA = {}", shorten(&a1.to_str_radix(16), 6, 4), a2.to_str_radix(16), shorten(&to_hex(&x_only(&agg.q)), 8, 6)), Tone::Info);
        self.log.line(format!("Q equals T? {}", if hit { "YES" } else { "NO" }), if hit { Tone::Bad } else { Tone::Ok });
        self.musig_tries.push((dla, a1, a2));
        if hit {
            return;
        }
        if correct {
            host.sfx("unlock");
            host.solve("Real joint custody: MuSig2 key aggregation. Every key you could announce changes the coefficient on the other key, so you are always one step behind. The same 64-byte signature on-chain as a single key, but no one can seize it.");
        } else {
            host.sfx("confirm");
            host.status("Q ≠ T: P_you is multiplied by a_you before anything is added. Try correcting for the coefficient you just saw.", Tone::Ok);
        }
    }

    fn banner(ui: &mut egui::Ui, color: Color32, title: &str, body: &str) {
        theme::card_frame(Some(color)).fill(color.gamma_multiply(0.06)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(title).font(retro(20.0)).color(color));
            note(ui, body);
        });
    }
}

impl Puzzle for RogueKey {
    fn id(&self) -> &'static str {
        "ch4.rogue"
    }

    fn title(&self) -> &str {
        "JOINT CUSTODY · KEY AGGREGATION"
    }

    fn subtitle(&self) -> Option<&str> {
        Some("rogue key vs BIP-327 MuSig2")
    }

    fn accent(&self) -> Color32 {
        AMBER
    }

    fn open(&mut self, host: &mut Host) {
        host.status("Play the Directorate: seize the contract without your victim's key.", Tone::Info);
    }

    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        lede(ui, "The Directorate offers \"joint custody\": a 2-of-2 address built from your key and theirs, so, they say, neither side can move anything alone. You publish your key first (you always have to); they publish theirs; the contract key is the two combined. Ibrahim wants to see what they could do with that. Play them.");
        let your_x = to_hex(&self.world.identity().xonly);
        let target_x = to_hex(&x_only(&self.target()));
        let mut action: Option<u8> = None;
        card(ui, Some("The deal"), None, |ui| match self.stage {
            Stage::Naive => {
                note(ui, "Naive aggregation just adds the keys:");
                math(ui, "Q = P_you + P_DLA");
                kv(ui, "rk-deal", &[("your key P_you (published)", shorten(&your_x, 10, 8)), ("T = t·G, a key the Directorate alone holds", shorten(&target_x, 10, 8))]);
                note(ui, "As the Directorate, choose the key to announce. You want the contract key Q to come out as T.");
                formula(ui, PARTS, PALETTE, &mut self.rogue);
                ui.horizontal_wrapped(|ui| {
                    if button_enabled(ui, self.rogue.complete() && !self.seized, "Announce P_DLA", Kind::Primary).clicked() {
                        action = Some(1);
                    }
                    if button(ui, "Compare: announce honestly", Kind::Ghost).clicked() {
                        action = Some(0);
                    }
                });
                if self.seized {
                    Self::banner(ui, RED, "Rogue-key attack succeeded.", "P_DLA = T − P_you, so Q = T, a key the Directorate alone holds. They never needed yours. \"Joint custody\" is sole Directorate custody. Now do it properly.");
                    if button(ui, "Switch to MuSig2 →", Kind::Primary).clicked() {
                        action = Some(2);
                    }
                }
            }
            Stage::Musig => {
                note(ui, "MuSig2 (BIP-327) weights each key by a coefficient that depends on the whole key list, so no one can cancel the others. (The second key in the list gets coefficient 1; it makes no difference.)");
                math(ui, "aᵢ = H(L ‖ Pᵢ),   L = H(P₁ ‖ … ‖ Pᵤ),   Q = a₁P₁ + a₂P₂");
                if !host.solved() {
                    ui.horizontal_wrapped(|ui| {
                        if button(ui, "Announce T − P_you again", Kind::Primary).clicked() {
                            action = Some(3);
                        }
                        if button_enabled(ui, !self.musig_tries.is_empty(), "Correct for a_you: announce (T − a_you·P_you)/a_DLA", Kind::Normal).clicked() {
                            action = Some(4);
                        }
                    });
                } else {
                    Self::banner(ui, GREEN, "Rogue key defeated.", "To cancel P_you you would need to know a_you before choosing P_DLA. But a_you is a hash of a list that includes P_DLA. Every correction changes the thing it corrects for. The aggregate genuinely needs both keys.");
                }
            }
        });
        card(ui, Some("Record"), None, |ui| self.log.show(ui, "rk-log", 180.0));
        match action {
            Some(0) => self.honest(host),
            Some(1) => self.naive(host),
            Some(2) => self.to_musig(host),
            Some(3) => self.musig(false, host),
            Some(4) => self.musig(true, host),
            _ => {}
        }
    }

    fn autoplay(&mut self, host: &mut Host) {
        self.rogue.set(&[0, 1]);
        self.naive(host);
        self.to_musig(host);
        self.musig(false, host);
        self.musig(true, host);
    }

    fn steps(&self) -> Vec<StepView> {
        steps(&[
            ("As the Directorate, pick P_DLA so that Q = T", "Q = P_you + P_DLA. What would make P_you cancel?", self.seized),
            ("Switch the contract to MuSig2", "", self.stage == Stage::Musig),
            ("Try the same rogue key against MuSig2", "", !self.musig_tries.is_empty()),
            ("Correct for the coefficient, and watch it move", "Use the a_you you just saw. Then look at a_you again.", self.musig_tries.len() >= 2),
        ])
    }

    fn deep_sheet(&self) -> Option<HandSheet> {
        let grp = TOY_GROUP;
        let mut rng = self.world.rng("ch4/rogue-deep");
        let d = 2 + rng.int(500) as u64;
        let t = 2 + rng.int(500) as u64;
        let (p_you, tt) = (grp.public(d), grp.public(t));
        let inv = inv_mod_i64(p_you as i64, grp.p as i64).unwrap_or(1) as u64;
        let dla = tt * inv % grp.p;
        let q = p_you * dla % grp.p;
        debug_assert_eq!(q, tt);
        Some(HandSheet::new(
            "the rogue key in a toy group",
            &format!("In the integers mod p = {} (g = {}), \"adding keys\" is multiplying them. Your key is P_you = g^{d}; the Directorate's secret is t = {t}.", grp.p, grp.g),
            240.0,
            vec![
                HandCell::new(format!("P_you = g^{d} mod {}", grp.p), "", p_you as i64),
                HandCell::new(format!("T = g^{t} mod {}", grp.p), "", tt as i64),
                HandCell::new(format!("P_you⁻¹ mod {}", grp.p), "extended Euclid", inv as i64),
                HandCell::new(format!("rogue P_DLA = T · P_you⁻¹ mod {}", grp.p), "", dla as i64),
                HandCell::new(format!("Q = P_you · P_DLA mod {}", grp.p), "it should be T", q as i64),
            ],
        ))
    }
}
