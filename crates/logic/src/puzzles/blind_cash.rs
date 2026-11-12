//! Chapter 3: the Commons Mint. Withdraw one eCash token the way Chaum meant
//! it: choose a secret, blind it with a factor only you know, let the mint
//! sign what it cannot see, unblind, spend. Then try to spend it twice, and
//! play the Directorate's analyst with the mint's own records.

use std::rc::Rc;

use cpl_crypto::bytes::{Big, inv_mod_i64, pow_mod_u64, to_hex};
use cpl_crypto::dh::TOY_GROUP;
use cpl_crypto::drbg::Drbg;
use cpl_crypto::ecash::{Dleq, LedgerEntry, Mint, Token, blind, dleq_verify, hash_to_curve, prepare, finish};
use cpl_crypto::secp::{N, Pt, compressed, g, mul_pt};
use egui::{Color32, RichText};

use crate::puzzle::{HandCell, HandSheet, Host, Puzzle, StepView, steps};
use crate::ui::theme::{AMBER, CYAN, DIM, GREEN, RED, TEXT, mono};
use crate::ui::widgets::{Formula, Kind, Log, Part, Tone, button, button_enabled, card, formula, lede, math, note, select_card, shorten};
use crate::world::World;

const AMOUNT: u64 = 8;
const PARTS: &[Part] = &[Part::Text("C = C_ − "), Part::Slot, Part::Text(" · "), Part::Slot];
const PALETTE: &[&str] = &["r", "K", "k", "Y", "G"];

fn hexp(p: &Pt) -> String {
    to_hex(&compressed(p))
}

pub struct BlindCash {
    world: Rc<World>,
    rng: Drbg,
    mint: Mint,
    k_pub: Pt,
    secret: Option<Vec<u8>>,
    r: Option<Big>,
    /// what the player sent: 0 the secret, 1 Y, 2 B_
    sent: Option<u8>,
    signed: Option<(Pt, Dleq)>,
    proof_ok: Option<bool>,
    unblind: Formula,
    token: Option<Token>,
    spent: bool,
    double: Option<String>,
    links_tried: Vec<(usize, usize)>,
    /// the mint's ledger as a subpoena would see it: (withdrawals, redemptions)
    others: Vec<Token>,
    log: Log,
}

impl BlindCash {
    pub fn new(world: Rc<World>) -> Self {
        let mint = world.mint();
        let k_pub = mint.public_key(AMOUNT).expect("amount");
        let rng = world.rng("ch3/cash");
        Self { world, rng, mint, k_pub, secret: None, r: None, sent: None, signed: None, proof_ok: None, unblind: Formula::new(PARTS), token: None, spent: false, double: None, links_tried: Vec::new(), others: Vec::new(), log: Log::default() }
    }

    fn y(&self) -> Option<Pt> {
        self.secret.as_ref().map(|s| hash_to_curve(s))
    }

    fn b(&self) -> Option<Pt> {
        Some(blind(self.secret.as_ref()?, self.r.as_ref()?))
    }

    fn roll_secret(&mut self) {
        self.secret = Some(self.rng.bytes(32));
        self.sent = None;
        self.signed = None;
    }

    fn roll_r(&mut self) {
        self.r = Some(self.rng.scalar(&N));
        self.sent = None;
        self.signed = None;
    }

    fn send(&mut self, what: u8, host: &mut Host) {
        let Some(b) = self.b() else { return };
        self.sent = Some(what);
        match what {
            0 => {
                host.status("You handed the mint your secret. It will sign, and it will remember: the day you spend this coin, it knows it was yours.", Tone::Bad);
                host.sfx("error");
            }
            1 => {
                host.status("You sent Y = hash_to_curve(secret). The mint can compute that itself when you spend: linked. Blind it first.", Tone::Bad);
                host.sfx("error");
            }
            _ => {
                let (c_blind, proof) = self.mint.sign(AMOUNT, &b, "you", &mut self.rng).expect("amount");
                self.signed = Some((c_blind, proof));
                self.log.line(format!("mint signed B_ = {} → C_ = {}", shorten(&hexp(&b), 10, 6), shorten(&hexp(&c_blind), 10, 6)), Tone::Info);
                host.status("The mint signed a point it cannot connect to anything. Check its proof, then unblind.", Tone::Ok);
                host.sfx("confirm");
            }
        }
    }

    fn check_proof(&mut self, host: &mut Host) {
        let (Some((c_blind, proof)), Some(b)) = (self.signed.as_ref(), self.b()) else { return };
        let ok = dleq_verify(proof, &self.k_pub, &b, c_blind);
        self.proof_ok = Some(ok);
        self.log.line(format!("DLEQ: log_G(K) = log_B_(C_)? {}", if ok { "yes" } else { "NO" }), if ok { Tone::Ok } else { Tone::Bad });
        host.status(if ok { "The mint signed with the key it publishes for 8 cr, not a key made just for you. A per-customer key would tag your coin." } else { "Proof failed." }, if ok { Tone::Ok } else { Tone::Bad });
    }

    fn do_unblind(&mut self, host: &mut Host) {
        let (Some((c_blind, _)), Some(r), Some(y)) = (self.signed.clone(), self.r.clone(), self.y()) else { return };
        let toks = self.unblind.tokens();
        let (Some(a), Some(b)) = (toks[0], toks[1]) else {
            host.status("Fill both slots.", Tone::Warn);
            return;
        };
        let pick = |t: usize| -> Result<Pt, String> {
            match t {
                1 => Ok(self.k_pub),
                3 => Ok(y),
                4 => Ok(g()),
                2 => Err("k is the mint's secret. You will never see it; you only see K = k·G.".into()),
                _ => Err("r is a number; pair it with a point.".into()),
            }
        };
        let result = match (a, b) {
            (0, p) | (p, 0) if p != 0 => pick(p).map(|pt| c_blind - mul_pt(&r, &pt)),
            (2, _) | (_, 2) => Err("k is the mint's secret. You will never see it; you only see K = k·G.".into()),
            _ => Err("Each term needs your blinding factor r times a point.".into()),
        };
        match result {
            Err(e) => host.status(e, Tone::Bad),
            Ok(c) => {
                let t = Token { amount: AMOUNT, secret: to_hex(self.secret.as_ref().unwrap()), c: hexp(&c) };
                if self.mint.check(&t).is_ok() {
                    self.log.line(format!("unblinded C = {} (= k·Y)", shorten(&t.c, 10, 6)), Tone::Ok);
                    self.token = Some(t);
                    host.status("C = C_ − r·K = k·(Y + r·G) − r·k·G = k·Y. A valid coin, and the mint has never seen it.", Tone::Ok);
                    host.sfx("unlock");
                } else {
                    host.status("That point is not k·Y: the mint would reject it. Which public value carries k?", Tone::Bad);
                    host.sfx("error");
                }
            }
        }
    }

    fn spend(&mut self, host: &mut Host) {
        let Some(t) = self.token.clone() else { return };
        if !self.spent {
            match self.mint.redeem(&t) {
                Ok(()) => {
                    self.spent = true;
                    self.log.line(format!("café deposits {} cr: secret {}…, C ✓, not spent before ✓", t.amount, &t.secret[..10]), Tone::Ok);
                    host.status("Spent. The café got paid; the mint saw a secret and a signature for the first time.", Tone::Ok);
                    self.fill_ledger();
                }
                Err(e) => host.status(e.to_string(), Tone::Bad),
            }
        } else {
            let e = self.mint.redeem(&t).err().map(|e| e.to_string()).unwrap_or_default();
            self.log.line(format!("second deposit of the same coin → {e}"), Tone::Bad);
            self.double = Some(e);
            host.status("The mint keeps every spent secret. A copied coin is worth nothing the second time.", Tone::Ok);
        }
    }

    /// Four other customers withdraw and spend too, so the subpoena has something to chew on.
    fn fill_ledger(&mut self) {
        let mut rng = self.world.rng("ch3/cash-others");
        for _ in 0..4 {
            let p = prepare(AMOUNT, &mut rng);
            let (c_blind, _) = self.mint.sign(AMOUNT, &p.b_blind, "someone", &mut rng).expect("amount");
            self.others.push(finish(&p, &c_blind, &self.k_pub));
        }
        // they spend in a different order
        let mut spent = self.others.clone();
        rng.shuffle(&mut spent);
        for t in &spent {
            let _ = self.mint.redeem(t);
        }
    }

    fn ledger(&self) -> (Vec<String>, Vec<String>) {
        let mut issued = Vec::new();
        let mut redeemed = Vec::new();
        for e in &self.mint.ledger {
            match e {
                LedgerEntry::Issued { b_blind, .. } => issued.push(b_blind.clone()),
                LedgerEntry::Redeemed { secret, .. } => redeemed.push(secret.clone()),
            }
        }
        (issued, redeemed)
    }

    fn try_link(&mut self, w: usize, r: usize, host: &mut Host) {
        self.links_tried.push((w, r));
        let (issued, redeemed) = self.ledger();
        host.status(
            format!(
                "Withdrawal {} ({}…) and deposit {} (secret {}…): consistent. For ANY pair there is some r with B_ − Y = r·G. The records cannot tell them apart.",
                w + 1,
                &issued[w][..8],
                r + 1,
                &redeemed[r][..8]
            ),
            Tone::Info,
        );
        if self.links_tried.len() >= 2 && !host.solved() {
            host.sfx("unlock");
            host.solve("Untraceable by design. Your wallet is open: everything you are paid from now on arrives as blind-signed eCash.");
        }
    }
}

impl Puzzle for BlindCash {
    fn id(&self) -> &'static str {
        "ch3.cash"
    }

    fn title(&self) -> &str {
        "THE COMMONS MINT"
    }

    fn subtitle(&self) -> Option<&str> {
        Some("Chaumian eCash · blind DH on secp256k1")
    }

    fn accent(&self) -> Color32 {
        GREEN
    }

    fn open(&mut self, host: &mut Host) {
        host.status("Withdraw one 8 cr coin without letting the mint see it.", Tone::Info);
    }

    fn steps(&self) -> Vec<StepView> {
        steps(&[
            ("Pick a secret and a blinding factor", "The secret becomes the coin; r is what hides it from the mint.", self.secret.is_some() && self.r.is_some()),
            ("Send the mint only what hides the secret", "The mint must never see the secret or Y = hash(secret). Send B_ = Y + r·G.", self.signed.is_some()),
            ("Check the mint's proof", "The DLEQ proof shows it used its public 8 cr key.", self.proof_ok == Some(true)),
            ("Unblind: C = C_ − r·K", "You know r and the public K = k·G. The mint's k stays secret.", self.token.is_some()),
            ("Spend it, then try to spend it again", "The café deposits it; the mint checks the signature and its spent list.", self.double.is_some()),
            ("Play the analyst: link a withdrawal to a deposit", "Pick any withdrawal and any deposit from the mint's records.", self.links_tried.len() >= 2),
        ])
    }

    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        lede(ui, "The Commons Mint is a cooperative: a key per denomination and a list of spent coins, nothing else. Its operator, Sol, wants it that way. The Directorate wants the opposite. Here is how a coin can be valid without anyone knowing whose it is.");
        let mut action: Option<u8> = None;
        card(ui, Some("1 · Your secret and your blinding factor"), None, |ui| {
            ui.horizontal(|ui| {
                if button(ui, "Roll a secret", Kind::Normal).clicked() {
                    action = Some(1);
                }
                if button(ui, "Roll r", Kind::Normal).clicked() {
                    action = Some(2);
                }
            });
            if let Some(s) = &self.secret {
                ui.label(RichText::new(format!("secret = {}", shorten(&to_hex(s), 16, 8))).font(mono(12.5)).color(TEXT));
                if let Some(y) = self.y() {
                    ui.label(RichText::new(format!("Y = hash_to_curve(secret) = {}", shorten(&hexp(&y), 12, 8))).font(mono(12.5)).color(TEXT));
                }
            }
            if let (Some(r), Some(b)) = (&self.r, self.b()) {
                ui.label(RichText::new(format!("r = {}", shorten(&r.to_str_radix(16), 12, 8))).font(mono(12.5)).color(AMBER));
                ui.label(RichText::new(format!("B_ = Y + r·G = {}", shorten(&hexp(&b), 12, 8))).font(mono(12.5)).color(CYAN));
            }
        });
        if self.b().is_some() {
            card(ui, Some("2 · What do you send the mint?"), None, |ui| {
                for (i, label) in ["the secret", "Y = hash_to_curve(secret)", "B_ = Y + r·G"].iter().enumerate() {
                    if select_card(ui, self.sent == Some(i as u8), CYAN, |ui| {
                        ui.label(RichText::new(*label).font(mono(13.0)).color(TEXT));
                    }) {
                        action = Some(10 + i as u8);
                    }
                }
            });
        }
        if let Some((c_blind, _)) = &self.signed {
            card(ui, Some("3 · The mint's answer"), None, |ui| {
                ui.label(RichText::new(format!("C_ = k·B_ = {}", shorten(&hexp(c_blind), 12, 8))).font(mono(12.5)).color(TEXT));
                ui.label(RichText::new(format!("K (published for 8 cr) = {}", shorten(&hexp(&self.k_pub), 12, 8))).font(mono(12.5)).color(DIM));
                if button_enabled(ui, self.proof_ok.is_none(), "Check the DLEQ proof", Kind::Normal).clicked() {
                    action = Some(3);
                }
            });
            card(ui, Some("4 · Unblind"), None, |ui| {
                formula(ui, PARTS, PALETTE, &mut self.unblind);
                if button_enabled(ui, self.token.is_none() && self.unblind.complete(), "Unblind", Kind::Primary).clicked() {
                    action = Some(4);
                }
            });
        }
        if self.token.is_some() {
            card(ui, Some("5 · Spend it"), None, |ui| {
                let label = if self.spent { "Spend the same coin again" } else { "Pay the café 8 cr" };
                if button_enabled(ui, self.double.is_none(), label, Kind::Primary).clicked() {
                    action = Some(5);
                }
            });
        }
        if self.spent && self.double.is_some() {
            let (issued, redeemed) = self.ledger();
            card(ui, Some("6 · The subpoena"), Some(RED), |ui| {
                note(ui, "The Directorate seizes the mint's records: five withdrawals (the blinded points it signed) and five deposits (the coins it accepted). One of each is yours. Link them.");
                ui.columns(2, |cols| {
                    cols[0].label(RichText::new("withdrawals: B_").font(mono(12.0)).color(DIM));
                    for (i, b) in issued.iter().enumerate() {
                        cols[0].label(RichText::new(format!("{} · {}", i + 1, shorten(b, 10, 6))).font(mono(12.0)).color(TEXT));
                    }
                    cols[1].label(RichText::new("deposits: secret").font(mono(12.0)).color(DIM));
                    for (j, s) in redeemed.iter().enumerate() {
                        cols[1].label(RichText::new(format!("{} · {}", j + 1, shorten(s, 10, 6))).font(mono(12.0)).color(TEXT));
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("try a pair:").font(mono(12.0)).color(DIM));
                    for w in 0..issued.len() {
                        for r in 0..redeemed.len() {
                            if (w + r) % 2 == 0 && ui.small_button(format!("{}↔{}", w + 1, r + 1)).clicked() {
                                action = Some(100 + (w * 10 + r) as u8);
                            }
                        }
                    }
                });
            });
        }
        match action {
            Some(1) => self.roll_secret(),
            Some(2) => self.roll_r(),
            Some(3) => self.check_proof(host),
            Some(4) => self.do_unblind(host),
            Some(5) => self.spend(host),
            Some(a) if (10..13).contains(&a) => self.send(a - 10, host),
            Some(a) if a >= 100 => {
                let v = (a - 100) as usize;
                self.try_link(v / 10, v % 10, host);
            }
            _ => {}
        }
        math(ui, "B_ = Y + r·G,   C_ = k·B_,   C = C_ − r·K = k·Y");
        self.log.show(ui, "cash-log", 120.0);
    }

    fn autoplay(&mut self, host: &mut Host) {
        self.roll_secret();
        self.roll_r();
        self.send(2, host);
        self.check_proof(host);
        self.unblind.set(&[0, 1]);
        self.do_unblind(host);
        self.spend(host);
        self.spend(host);
        self.try_link(0, 1, host);
        self.try_link(2, 3, host);
    }

    fn deep_sheet(&self) -> Option<HandSheet> {
        let grp = TOY_GROUP;
        let mut rng = self.world.rng("ch3/cash-deep");
        let y = 2 + rng.int(500) as u64;
        let r = 2 + rng.int(500) as u64;
        let k = 2 + rng.int(500) as u64;
        let yy = grp.pow(grp.g, y);
        let b = yy * grp.pow(grp.g, r) % grp.p;
        let cb = grp.pow(b, k);
        let kp = grp.pow(grp.g, k);
        let kr = grp.pow(kp, r);
        let inv = inv_mod_i64(kr as i64, grp.p as i64).unwrap();
        let c = (cb as i64 * inv).rem_euclid(grp.p as i64);
        debug_assert_eq!(c as u64, pow_mod_u64(yy, k, grp.p));
        Some(HandSheet::new(
            "blind signing in a toy group",
            &format!("The same protocol in the group of integers mod p = {} generated by g = {}, written multiplicatively. Your coin is Y = {yy}. Your blinding factor is r = {r}. Pretend to be the mint too, with key k = {k}.", grp.p, grp.g),
            240.0,
            vec![
                HandCell::new(format!("B = Y · g^r mod {}", grp.p), "g^r by square-and-multiply, then multiply by Y", b as i64),
                HandCell::new(format!("C_ = B^k mod {}", grp.p), "the mint's blind signature", cb as i64),
                HandCell::new(format!("K = g^k mod {}", grp.p), "the mint's public key", kp as i64),
                HandCell::new(format!("K^r mod {}", grp.p), "what blinding added", kr as i64),
                HandCell::new(format!("C = C_ · (K^r)⁻¹ mod {}", grp.p), "inverse by the extended Euclidean algorithm", c),
            ],
        ))
    }
}
