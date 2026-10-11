//! Chapter 2, part 2. Two Directorate nodes, a field patrol and its control
//! desk, set up a shared key with plain Diffie–Hellman through Felix's old
//! switchboard: neither side signs anything. Whoever sits on the wire can give
//! each side its own public value and end up sharing one key with each. Then
//! the traffic is readable, and rewritable. The fix comes in Chapter 4:
//! authenticate the key exchange and this stops working.

use std::rc::Rc;

use cpl_crypto::aead::{GcmBox, gcm_decrypt_text, gcm_encrypt_text};
use cpl_crypto::bytes::to_hex;
use cpl_crypto::dh::{DRONE_GROUP, Keypair};
use cpl_crypto::drbg::Drbg;
use egui::{Color32, RichText};

use crate::puzzle::{Host, Puzzle};
use crate::ui::theme::{self, AMBER, GREEN, RED, mono};
use crate::ui::widgets::{Kind, Log, Tone, button, card, kv, lede, note, shorten};
use crate::world::World;

const ORIGINAL: &str = "PATROL 7 LOG: UNESCROWED KEYS SEEN AT THE EXCHANGE. FLAG FOR AUDIT.";
const REWRITE: &str = "PATROL 7 LOG: NOTHING TO REPORT AT THE EXCHANGE. ALL CLEAR.";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    Keygen,
    PatrolHello,
    ControlHello,
    Message,
    Done,
}

pub struct MitmRelay {
    stage: Stage,
    patrol: Keypair,
    control: Keypair,
    relay: Option<Keypair>,
    to_control: u64,
    to_patrol: u64,
    message: GcmBox,
    k_patrol: Option<u64>,
    k_control: Option<u64>,
    log: Log,
    text: String,
    outcome: Option<(String, bool)>,
}

impl MitmRelay {
    pub fn new(world: Rc<World>) -> Self {
        let g = DRONE_GROUP;
        let mut r = world.rng("ch2/mitm");
        let patrol = g.keypair(&mut r);
        let control = g.keypair(&mut r);
        let nonce: [u8; 12] = r.array();
        let message = gcm_encrypt_text(&g.session_key(g.shared(patrol.secret, control.public)), &nonce, ORIGINAL);
        let mut log = Log::default();
        log.line("Line 7 open. Key exchange about to begin.", Tone::Dim);
        Self { stage: Stage::Keygen, patrol, control, relay: None, to_control: 0, to_patrol: 0, message, k_patrol: None, k_control: None, log, text: String::new(), outcome: None }
    }

    fn keygen(&mut self, host: &mut Host) {
        let relay = DRONE_GROUP.keypair(&mut Drbg::fresh("ch2/mitm-relay"));
        self.log.line(format!("You generated M = g^m = {}.", relay.public), Tone::Info);
        self.relay = Some(relay);
        self.stage = Stage::PatrolHello;
        host.sfx("confirm");
        host.status("The patrol's half of the exchange is passing through.", Tone::Info);
    }

    fn patrol_hello(&mut self, swap: bool, host: &mut Host) {
        let g = DRONE_GROUP;
        let relay = self.relay.unwrap();
        self.to_control = if swap { relay.public } else { self.patrol.public };
        if swap {
            self.k_patrol = Some(g.shared(relay.secret, self.patrol.public));
            self.log.line("You sent control your M instead of A. Control will share a key with YOU, not the patrol.", Tone::Warn);
        } else {
            self.log.line("You forwarded A unchanged.", Tone::Dim);
        }
        self.stage = Stage::ControlHello;
        host.sfx("blip");
        host.status("The control desk answers.", Tone::Info);
    }

    fn control_hello(&mut self, swap: bool, host: &mut Host) {
        let g = DRONE_GROUP;
        let relay = self.relay.unwrap();
        self.to_patrol = if swap { relay.public } else { self.control.public };
        if swap {
            self.k_control = Some(g.shared(relay.secret, self.control.public));
            self.log.line("You sent the patrol your M instead of B. The patrol will share a key with YOU.", Tone::Warn);
        } else {
            self.log.line("You forwarded B unchanged.", Tone::Dim);
        }
        self.stage = Stage::Message;
        host.sfx("blip");
        if self.read().is_some() {
            self.text = REWRITE.into();
            host.status("Read it, change it, pass it on.", Tone::Info);
        } else {
            host.status("Deliver the message to see whether you learned anything.", Tone::Warn);
        }
    }

    fn read(&self) -> Option<String> {
        let g = DRONE_GROUP;
        self.k_patrol.and_then(|k| gcm_decrypt_text(&g.session_key(k), &self.message).ok())
    }

    fn deliver(&mut self, host: &mut Host) {
        if host.solved() {
            return;
        }
        let g = DRONE_GROUP;
        let (Some(_), Some(kc_relay)) = (self.k_patrol, self.k_control) else {
            self.outcome = Some(("You forwarded the exchange honestly, so you never held a key. The control desk read the patrol's real message and you read nothing. That is Diffie–Hellman working as intended.".into(), false));
            self.log.line("Control received the original message intact. You saw nothing.", Tone::Dim);
            host.status("Relayed honestly. To read or change the traffic, substitute your key on BOTH exchanges.", Tone::Warn);
            self.stage = Stage::Done;
            return;
        };
        let forwarded = gcm_encrypt_text(&g.session_key(kc_relay), &self.message.nonce, &self.text);
        let kc = g.shared(self.control.secret, self.to_control);
        let seen = gcm_decrypt_text(&g.session_key(kc), &forwarded).unwrap_or_else(|_| "(garbled)".into());
        self.outcome = Some((seen, true));
        self.log.line("Control accepted your rewritten message as genuine.", Tone::Bad);
        host.sfx("unlock");
        host.solve("You sat in the middle of the whole conversation. The cure is authentication: make each side sign its public value, and your substituted key no longer matches a signature. That is Chapter 4.");
        self.stage = Stage::Done;
    }

    fn reset(&mut self) {
        self.stage = Stage::Keygen;
        self.relay = None;
        self.to_control = 0;
        self.to_patrol = 0;
        self.k_patrol = None;
        self.k_control = None;
        self.outcome = None;
        self.log.line("Line 7 reset. The nodes start a fresh exchange.", Tone::Dim);
    }
}

fn v(x: u64) -> String {
    if x == 0 { "—".into() } else { shorten(&x.to_string(), 8, 6) }
}

impl Puzzle for MitmRelay {
    fn id(&self) -> &'static str {
        "ch2.mitm"
    }

    fn title(&self) -> &str {
        "SWITCHBOARD · LINE 7"
    }

    fn subtitle(&self) -> Option<&str> {
        Some("patrol ⇄ control, unauthenticated DH (64-bit group)")
    }

    fn accent(&self) -> Color32 {
        theme::rgb(0xef7d57)
    }

    fn open(&mut self, host: &mut Host) {
        host.status("Prepare a relay key.", Tone::Info);
    }

    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        let g = DRONE_GROUP;
        lede(ui, "Two Directorate nodes agree on a key with ordinary Diffie–Hellman and talk over Felix's switchboard. Nobody signs their public value, and the exchange never says *who* sent it. You are the switchboard. Give each side a public value of your own and you will share a different key with each, and sit in the middle of the conversation.");
        let key = |k: Option<u64>| k.map(|k| shorten(&to_hex(&g.session_key(k)), 6, 4)).unwrap_or_else(|| "—".into());
        let kp = (self.to_patrol != 0).then(|| g.shared(self.patrol.secret, self.to_patrol));
        let kc = (self.to_control != 0).then(|| g.shared(self.control.secret, self.to_control));
        ui.columns(3, |cols| {
            card(&mut cols[0], Some("Field patrol"), None, |ui| {
                kv(ui, "mitm-p", &[("secret a", "hidden".into()), ("sends A", v(self.patrol.public)), ("thinks control is", v(self.to_patrol)), ("its key", key(kp))]);
            });
            card(&mut cols[1], Some("You (switchboard)"), Some(AMBER.gamma_multiply(0.5)), |ui| {
                kv(ui, "mitm-y", &[("secret m", if self.relay.is_some() { "yours".into() } else { "—".into() }), ("public M", self.relay.map(|r| v(r.public)).unwrap_or_else(|| "—".into())), ("key with patrol", key(self.k_patrol)), ("key with control", key(self.k_control))]);
            });
            card(&mut cols[2], Some("Control desk"), None, |ui| {
                kv(ui, "mitm-c", &[("secret b", "hidden".into()), ("sends B", v(self.control.public)), ("thinks patrol is", v(self.to_control)), ("its key", key(kc))]);
            });
        });
        #[derive(PartialEq)]
        enum Act {
            None,
            Keygen,
            Patrol(bool),
            Control(bool),
            Deliver,
            Reset,
        }
        let mut act = Act::None;
        card(ui, Some("Switchboard"), None, |ui| match self.stage {
            Stage::Keygen => {
                note(ui, "First make yourself a key pair in the same group, so you have a public value to substitute for each side.");
                if button(ui, "Generate switchboard key m, M = g^m", Kind::Primary).clicked() {
                    act = Act::Keygen;
                }
            }
            Stage::PatrolHello => {
                ui.label(RichText::new(format!("▶ PATROL → CONTROL   KEYX  A = {}", self.patrol.public)).font(mono(13.0)));
                ui.horizontal(|ui| {
                    if button(ui, "Forward A unchanged", Kind::Normal).clicked() {
                        act = Act::Patrol(false);
                    }
                    if button(ui, "Substitute your M", Kind::Primary).clicked() {
                        act = Act::Patrol(true);
                    }
                });
            }
            Stage::ControlHello => {
                ui.label(RichText::new(format!("◀ CONTROL → PATROL   KEYX  B = {}", self.control.public)).font(mono(13.0)));
                ui.horizontal(|ui| {
                    if button(ui, "Forward B unchanged", Kind::Normal).clicked() {
                        act = Act::Control(false);
                    }
                    if button(ui, "Substitute your M", Kind::Primary).clicked() {
                        act = Act::Control(true);
                    }
                });
            }
            Stage::Message => {
                ui.label(RichText::new(format!("▶ PATROL → CONTROL   MSG  {} bytes, GCM", self.message.ct.len())).font(mono(13.0)));
                match self.read() {
                    Some(text) => {
                        note(ui, "Decrypted with your shared-with-patrol key:");
                        ui.add(egui::Label::new(RichText::new(text).font(mono(15.0)).color(GREEN)).wrap());
                        note(ui, "Re-encrypt whatever you like for the control desk:");
                        ui.add(egui::TextEdit::multiline(&mut self.text).desired_rows(2).desired_width(f32::INFINITY).font(mono(13.5)));
                    }
                    None => note(ui, "You do not share a key with the patrol, so this is just ciphertext to you. (Forward both exchanges unchanged and you learn nothing.)"),
                }
                if button(ui, "Relay to control desk", Kind::Primary).clicked() {
                    act = Act::Deliver;
                }
            }
            Stage::Done => {
                match &self.outcome {
                    Some((text, true)) => {
                        note(ui, "The control desk decrypts with the key it thinks it shares with the patrol, and reads:");
                        ui.add(egui::Label::new(RichText::new(text).font(mono(15.0)).color(AMBER)).wrap());
                        note(ui, "It has no way to tell this did not come straight from the patrol. Nothing in the exchange carried an identity.");
                    }
                    Some((text, false)) => {
                        note(ui, text);
                        if !host.solved() && button(ui, "Try the line again", Kind::Normal).clicked() {
                            act = Act::Reset;
                        }
                    }
                    None => {}
                }
                if host.solved() {
                    note(ui, "Line 7 closed. Felix saw the whole thing from the switchboard.");
                }
            }
        });
        match act {
            Act::None => {}
            Act::Keygen => self.keygen(host),
            Act::Patrol(s) => self.patrol_hello(s, host),
            Act::Control(s) => self.control_hello(s, host),
            Act::Deliver => self.deliver(host),
            Act::Reset => self.reset(),
        }
        card(ui, Some("Line log"), None, |ui| self.log.show(ui, "mitm-log", 180.0));
        let _ = RED;
    }

    fn autoplay(&mut self, host: &mut Host) {
        if self.stage == Stage::Done {
            self.reset();
        }
        self.keygen(host);
        self.patrol_hello(true, host);
        self.control_hello(true, host);
        self.text = REWRITE.into();
        self.deliver(host);
    }
}
