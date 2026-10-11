//! Chapter 4, part 2. The Directorate's "joint custody" offers a 2-of-2
//! escrow: your key and theirs, so neither can act alone. With naive key
//! aggregation (Q = P1 + P2) they announce a rogue key and seize sole control.
//! BIP-327 MuSig2 KeyAgg, which weights each key by a hash of the whole list,
//! stops it.

use std::rc::Rc;

use cpl_crypto::bip340::{sign, verify};
use cpl_crypto::bytes::{Big, to_hex};
use cpl_crypto::musig::{key_agg, key_agg_coeff, naive_aggregate, rogue_key};
use cpl_crypto::secp::{compressed, mul, x_only};
use egui::{Color32, RichText};

use crate::puzzle::{Host, Puzzle};
use crate::ui::theme::{self, AMBER, GREEN, RED, retro};
use crate::ui::widgets::{Kind, Log, Tone, button, card, kv, lede, math, note, shorten};
use crate::world::World;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    Naive,
    Musig,
}

pub struct RogueKey {
    world: Rc<World>,
    dir_secret: Big,
    stage: Stage,
    /// `Some(attack)` after a naive round
    naive_round: Option<bool>,
    musig_done: bool,
    log: Log,
}

impl RogueKey {
    pub fn new(world: Rc<World>) -> Self {
        let dir_secret = world.rng("ch4/dir").scalar(&(Big::from(1u8) << 250));
        Self { world, dir_secret, stage: Stage::Naive, naive_round: None, musig_done: false, log: Log::default() }
    }

    fn naive(&mut self, attack: bool, host: &mut Host) {
        let you = mul(&self.world.identity().x);
        let dla = if attack {
            self.log.line("Directorate announces P_DLA = (their target key) − P_you.", Tone::Warn);
            rogue_key(&mul(&self.dir_secret), &[you]) // P_dla = T − P_you
        } else {
            self.log.line("Directorate announces an honest P_DLA = d_DLA·G.", Tone::Dim);
            mul(&self.dir_secret)
        };
        let q = naive_aggregate(&[you, dla]);
        self.log.line(format!("Contract key Q = P_you + P_DLA = {}", shorten(&to_hex(&x_only(&q)), 8, 6)), Tone::Info);
        self.naive_round = Some(attack);
        if attack {
            // the attacker can now sign for Q alone: Q = target = dir_secret·G
            let alone = q == mul(&self.dir_secret);
            let m = b"MOVE EVERYTHING TO THE DIRECTORATE";
            let solo = sign(m, &self.dir_secret, None);
            let accepted = verify(&solo, m, &x_only(&q));
            self.log.line(format!("Q == (their own key alone)? {}", if alone { "YES" } else { "no" }), if alone { Tone::Bad } else { Tone::Ok });
            self.log.line(
                format!("Directorate signs alone, no key of yours involved… contract accepts? {}", if accepted { "YES" } else { "no" }),
                if accepted { Tone::Bad } else { Tone::Ok },
            );
            host.sfx("error");
            host.status("They took sole control. MuSig2 is the fix.", Tone::Bad);
        } else {
            self.log.line("Honestly, Q needs both keys. But nothing forces them to be honest.", Tone::Ok);
        }
    }

    fn to_musig(&mut self, host: &mut Host) {
        self.stage = Stage::Musig;
        host.status("Now aggregate the same keys with MuSig2.", Tone::Info);
    }

    fn musig(&mut self, host: &mut Host) {
        if host.solved() {
            return;
        }
        let you = mul(&self.world.identity().x);
        let target = mul(&self.dir_secret);
        // they try the same trick: announce T − P_you
        let rogue = rogue_key(&target, &[you]);
        let pks = [compressed(&you), compressed(&rogue)];
        let Ok(agg) = key_agg(&pks) else { return };
        let a1 = key_agg_coeff(&pks, &pks[0]);
        let a2 = key_agg_coeff(&pks, &pks[1]);
        let hit = agg.q == target;
        self.log.line(format!("MuSig2 coefficients: a_you = {}, a_DLA = {}", shorten(&a1.to_str_radix(16), 6, 4), shorten(&a2.to_str_radix(16), 6, 4)), Tone::Info);
        self.log.line(format!("Q = a_you·P_you + a_DLA·P_DLA = {}", shorten(&to_hex(&x_only(&agg.q)), 8, 6)), Tone::Info);
        self.log.line(format!("Q equals their target key? {}", if hit { "YES" } else { "NO" }), if hit { Tone::Bad } else { Tone::Ok });
        if hit {
            return;
        }
        self.musig_done = true;
        host.sfx("unlock");
        host.solve("Real joint custody: MuSig2 key aggregation. The same signature on-chain as a single key, but no one can seize it.");
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
        host.status("See how naive aggregation fails.", Tone::Info);
    }

    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        lede(ui, "The Directorate offers \"joint custody\": a 2-of-2 address built from your key and theirs, so, they say, neither side can move anything alone. You sign a contract by publishing your key; they publish theirs. The contract key is the two combined.");
        let your_x = to_hex(&self.world.identity().xonly);
        let mut action: Option<u8> = None;
        card(ui, Some("The deal"), None, |ui| match self.stage {
            Stage::Naive => {
                note(ui, "Naive aggregation just adds the keys:");
                math(ui, "Q = P_you + P_DLA");
                kv(ui, "rk-deal", &[("your key P_you", shorten(&your_x, 10, 8)), ("you expect", "a key only the two of you can spend from".into())]);
                note(ui, "You publish first (you always have to). Now the Directorate announces its key. Watch what they can choose.");
                ui.horizontal_wrapped(|ui| {
                    if button(ui, "Let the Directorate announce honestly", Kind::Normal).clicked() {
                        action = Some(0);
                    }
                    if button(ui, "…or let them play a rogue key", Kind::Primary).clicked() {
                        action = Some(1);
                    }
                });
                match self.naive_round {
                    Some(true) => {
                        Self::banner(ui, RED, "Rogue-key attack succeeded.", "They chose P_DLA = T − P_you, so Q = T, a key they alone hold. \"Joint custody\" is sole Directorate custody. Now do it properly.");
                        if button(ui, "Switch to MuSig2 →", Kind::Primary).clicked() {
                            action = Some(2);
                        }
                    }
                    Some(false) => {
                        note(ui, "That only worked because they chose to play fair. A real attacker will not. Make them.");
                        if button(ui, "Replay, with the rogue key", Kind::Primary).clicked() {
                            action = Some(1);
                        }
                    }
                    None => {}
                }
            }
            Stage::Musig => {
                note(ui, "MuSig2 (BIP-327) weights each key by a coefficient that depends on the whole key list, so no one can cancel the others:");
                math(ui, "aᵢ = H(L ‖ Pᵢ),   L = H(P₁ ‖ … ‖ Pᵤ),   Q = a₁P₁ + a₂P₂");
                if !self.musig_done && button(ui, "Let the Directorate try the rogue key again", Kind::Primary).clicked() {
                    action = Some(3);
                }
                if self.musig_done {
                    Self::banner(ui, GREEN, "Rogue key defeated.", "Their rogue P_DLA was chosen to cancel P_you under addition. But MuSig2 multiplies each key by aᵢ first, and aᵢ depends on the full key list, so they would have to know the coefficients before choosing a key that depends on them. They cannot. The aggregate genuinely needs both.");
                }
            }
        });
        card(ui, Some("Record"), None, |ui| self.log.show(ui, "rk-log", 180.0));
        match action {
            Some(0) => self.naive(false, host),
            Some(1) => self.naive(true, host),
            Some(2) => self.to_musig(host),
            Some(3) => self.musig(host),
            _ => {}
        }
    }

    fn autoplay(&mut self, host: &mut Host) {
        self.naive(true, host);
        self.to_musig(host);
        self.musig(host);
    }
}
