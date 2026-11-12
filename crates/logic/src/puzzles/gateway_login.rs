//! Prologue, part 2. The gateway knows Grandpa's public key. It sends a fresh
//! challenge; you answer it with the recovered private key (BIP-340 Schnorr):
//! pick a secret nonce k, publish R = k·G, and build the response s yourself
//! from k, the challenge hash e and your key d. The gateway checks
//! s·G = R + e·P. No password ever crosses the wire.

use std::rc::Rc;

use cpl_crypto::bip340::{Sig, challenge, even_key, explain_verify};
use cpl_crypto::bytes::{Big, to_hex};
use cpl_crypto::dh::TOY_GROUP;
use cpl_crypto::drbg::{Drbg, random_bytes};
use cpl_crypto::secp::{N, Pt, has_even_y, mod_n, mul, x_only, x_only_hex};
use cpl_crypto::bip39::BIP86_FIRST;
use egui::{Color32, RichText};

use crate::puzzle::{HandCell, HandSheet, Host, Puzzle, StepView, steps};
use crate::ui::theme::{AMBER, CYAN, GREEN, RED, mono};
use crate::ui::widgets::{Formula, Kind, Part, Tone, button_enabled, card, formula, hex_box, kv, lede, math, note, shorten};
use crate::world::{World, hex32};

const PARTS: &[Part] = &[Part::Text("s = "), Part::Slot, Part::Text(" + "), Part::Slot, Part::Text(" · "), Part::Slot, Part::Text("   mod n")];
const PALETTE: &[&str] = &["k", "e", "d", "R", "P"];

struct Verdict {
    e: String,
    lhs: String,
    rhs: String,
    ok: bool,
}

pub struct GatewayLogin {
    world: Rc<World>,
    challenge: String,
    /// the secret nonce (already normalised so R has even y) and R = k·G
    nonce: Option<(Big, Pt)>,
    e: Option<Big>,
    response: Formula,
    sig: Option<Sig>,
    built_ok: bool,
    tamper: bool,
    verdict: Option<Verdict>,
    rolls: u64,
}

impl GatewayLogin {
    pub fn new(world: Rc<World>) -> Self {
        let challenge = format!("CRYPTOPOLIS-LOGIN|user=nightjar|nonce={}", to_hex(&random_bytes::<16>()));
        Self { world, challenge, nonce: None, e: None, response: Formula::new(PARTS), sig: None, built_ok: false, tamper: false, verdict: None, rolls: 0 }
    }

    fn roll_nonce(&mut self, host: &mut Host) {
        self.rolls += 1;
        let mut rng = Drbg::fresh(&format!("pro/login-nonce-{}", self.rolls));
        let mut k = rng.scalar(&N);
        let mut r = mul(&k);
        // BIP-340 publishes only R.x, so it always uses the R with even y
        if !has_even_y(&r) {
            k = &*N - k;
            r = mul(&k);
        }
        let px = x_only(&self.world.grandpa().p);
        self.e = Some(challenge(&x_only(&r), &px, self.challenge.as_bytes()));
        self.nonce = Some((k, r));
        self.sig = None;
        self.verdict = None;
        self.built_ok = false;
        host.sfx("confirm");
        host.status("R = k·G is ready, and so is the challenge hash e. Now build your answer s.", Tone::Info);
    }

    /// Evaluate the formula with the real numbers. Points are not numbers.
    fn evaluate(&self) -> Result<Big, String> {
        let (Some((k, _)), Some(e)) = (&self.nonce, &self.e) else { return Err("Pick a nonce first.".into()) };
        let toks = self.response.tokens();
        let (Some(a), Some(b), Some(c)) = (toks[0], toks[1], toks[2]) else { return Err("Fill all three slots.".into()) };
        if [a, b, c].iter().any(|t| *t >= 3) {
            return Err("R and P are points on the curve. s is a number mod n: build it from the numbers k, e and d.".into());
        }
        let d = even_key(&self.world.grandpa().secret);
        let val = |t: usize| match t {
            0 => k.clone(),
            1 => e.clone(),
            _ => d.clone(),
        };
        Ok(mod_n(&(val(a) + val(b) * val(c))))
    }

    fn send(&mut self, host: &mut Host) {
        if host.solved() {
            return;
        }
        let Some((_, r)) = self.nonce.clone() else {
            host.status("Pick a nonce first: the gateway needs R.", Tone::Warn);
            return;
        };
        let s = match self.evaluate() {
            Ok(s) => s,
            Err(e) => {
                host.status(e, Tone::Bad);
                host.sfx("error");
                return;
            }
        };
        let honest = Sig { r: x_only(&r), s };
        self.built_ok = explain_verify(&honest, self.challenge.as_bytes(), &x_only(&self.world.grandpa().p)).is_some_and(|x| x.ok);
        let sent = if self.tamper { Sig { r: honest.r, s: &honest.s ^ Big::from(1) } } else { honest.clone() };
        let px = x_only(&self.world.grandpa().p);
        let ex = explain_verify(&sent, self.challenge.as_bytes(), &px).expect("valid key");
        self.sig = Some(sent);
        self.verdict = Some(Verdict { e: hex32(&ex.e), lhs: x_only_hex(&ex.lhs), rhs: x_only_hex(&ex.rhs), ok: ex.ok });
        if !ex.ok {
            let toks = self.response.tokens();
            let uses_d = toks.contains(&Some(2));
            let why = if self.tamper && self.built_ok {
                "ACCESS DENIED: one flipped bit of s, and s·G lands somewhere else entirely. Untick the box and send it clean."
            } else if !uses_d {
                "ACCESS DENIED: s·G ≠ R + e·P. Without d in it, s is something anyone could compute from public values, so it proves nothing about the key."
            } else {
                "ACCESS DENIED: s·G ≠ R + e·P. Expand both sides: R = k·G and P = d·G. Which s makes s·G equal k·G + e·d·G?"
            };
            host.status(why, Tone::Bad);
            host.sfx("error");
            return;
        }
        host.sfx("unlock");
        host.solve("ACCESS GRANTED. s·G = k·G + e·d·G = R + e·P. Welcome back, nightjar; your key never left this machine.");
    }
}

impl Puzzle for GatewayLogin {
    fn id(&self) -> &'static str {
        "pro.login"
    }

    fn title(&self) -> &str {
        "CRYPTOPOLIS GATEWAY"
    }

    fn subtitle(&self) -> Option<&str> {
        Some("challenge–response · BIP-340")
    }

    fn accent(&self) -> Color32 {
        CYAN
    }

    fn open(&mut self, host: &mut Host) {
        host.status("Answer the challenge with the recovered key, without revealing it.", Tone::Info);
    }

    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        let pubkey = x_only_hex(&self.world.grandpa().p);
        lede(ui, "The terminal does not want the seed, and it does not want a password. It wants proof that you hold the private key **d** behind the public key **P = d·G** it has on file for **nightjar**. It sends a random challenge; you answer with a Schnorr signature you build yourself.");
        card(ui, Some("Keys"), None, |ui| {
            kv(ui, "login-you", &[("your private key d", format!("{BIP86_FIRST} · never leaves this machine")), ("on file for nightjar: P", pubkey.clone())]);
        });
        let mut roll = false;
        card(ui, Some("1 · Challenge and nonce"), None, |ui| {
            note(ui, "The gateway's challenge carries a fresh nonce, so an old answer can never be replayed. Your answer needs a fresh secret nonce of its own, k. Only R = k·G is sent.");
            hex_box(ui, &self.challenge, Some("message m"), Some(AMBER), false);
            ui.horizontal(|ui| {
                if button_enabled(ui, !host.solved(), if self.nonce.is_none() { "Pick a random nonce k" } else { "Pick a new nonce" }, Kind::Primary).clicked() {
                    roll = true;
                }
            });
            if let (Some((_, r)), Some(e)) = (&self.nonce, &self.e) {
                kv(ui, "login-nonce", &[("k", "secret, on this machine only".into()), ("R = k·G (x)", shorten(&x_only_hex(r), 12, 10)), ("e = H(R.x ‖ P.x ‖ m)", shorten(&hex32(e), 12, 10))]);
                note(ui, "Anyone can compute e from public values; the gateway will compute the same number.");
            }
        });
        if roll {
            self.roll_nonce(host);
        }
        let mut send = false;
        card(ui, Some("2 · Build the response"), None, |ui| {
            math(ui, "the gateway accepts (R, s) iff  s·G = R + e·P");
            if self.nonce.is_none() {
                note(ui, "Pick a nonce first.");
                return;
            }
            if formula(ui, PARTS, PALETTE, &mut self.response) {
                self.verdict = None;
            }
            ui.checkbox(&mut self.tamper, RichText::new("Flip one bit of s on the way (see what happens)").font(mono(13.0)));
            if button_enabled(ui, self.response.complete() && !host.solved(), "Send (R, s) to the gateway", Kind::Primary).clicked() {
                send = true;
            }
            if let (Some(v), Some(sig)) = (&self.verdict, &self.sig) {
                hex_box(ui, &sig.hex(), Some("signature sent  =  R.x ‖ s"), None, false);
                math(ui, "e = H_challenge(R.x ‖ P.x ‖ m)  mod n");
                ui.label(RichText::new(format!("e = {}", shorten(&v.e, 20, 12))).font(mono(12.5)));
                let c = if v.ok { GREEN } else { RED };
                note(ui, "left side:  s·G  (x coordinate)");
                ui.add(egui::Label::new(RichText::new(&v.lhs).font(mono(12.5)).color(c)).wrap());
                note(ui, "right side: R + e·P  (x coordinate)");
                ui.add(egui::Label::new(RichText::new(&v.rhs).font(mono(12.5)).color(c)).wrap());
            }
        });
        if send {
            self.send(host);
        }
    }

    fn autoplay(&mut self, host: &mut Host) {
        self.tamper = false;
        if self.nonce.is_none() {
            self.roll_nonce(host);
        }
        self.response.set(&[0, 1, 2]);
        self.send(host);
    }

    fn steps(&self) -> Vec<StepView> {
        steps(&[
            ("Pick a fresh secret nonce k", "R = k·G goes on the wire; k never does.", self.nonce.is_some()),
            ("Build s from k, e and your key d", "s·G must equal R + e·P. Write R as k·G and P as d·G, then read off s.", self.built_ok),
            ("Send (R, s) and get in", "", self.verdict.as_ref().is_some_and(|v| v.ok)),
        ])
    }

    fn deep_sheet(&self) -> Option<HandSheet> {
        let grp = TOY_GROUP;
        let mut rng = self.world.rng("pro/login-deep");
        let d = 2 + rng.int(500) as u64;
        let k = 2 + rng.int(500) as u64;
        let e = 2 + rng.int(500) as u64;
        let p = grp.pow(grp.g, d);
        let r = grp.pow(grp.g, k);
        let s = (k + e * d) % grp.q;
        let lhs = grp.pow(grp.g, s);
        let rhs = r * grp.pow(p, e) % grp.p;
        debug_assert_eq!(lhs, rhs);
        Some(HandSheet::new(
            "a Schnorr login in a toy group",
            format!(
                "The same protocol with numbers you can hold: integers mod p = {} under multiplication, generator g = {} of prime order q = {}. Your key is d = {d}, your nonce k = {k}, and the hash gave e = {e}. Exponents live mod q.",
                grp.p, grp.g, grp.q
            ),
            240.0,
            vec![
                HandCell::new(format!("P = g^d mod {}", grp.p), "square-and-multiply", p as i64),
                HandCell::new(format!("R = g^k mod {}", grp.p), "the commitment", r as i64),
                HandCell::new(format!("s = k + e·d mod {}", grp.q), "exponents add mod q", s as i64),
                HandCell::new(format!("g^s mod {}", grp.p), "the gateway's left side", lhs as i64),
                HandCell::new(format!("R · P^e mod {}", grp.p), "the right side; it should match", rhs as i64),
            ],
        ))
    }
}
