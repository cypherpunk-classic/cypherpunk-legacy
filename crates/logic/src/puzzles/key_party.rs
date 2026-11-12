//! Interlude: a key-signing party at the Exchange. Each guest hands you a slip
//! with their fingerprint and shows you an ID. You look their short ID up on
//! the keyserver, compare the whole fingerprint, and sign only what you have
//! checked. The Directorate has uploaded a key whose short ID collides with
//! Juno's (the Evil32 trick), and one guest has no ID with them.

use std::rc::Rc;

use cpl_crypto::bytes::to_hex;
use cpl_crypto::wot::{Fingerprint, certify, collide_short_id, fp_display, short_id};
use egui::{Color32, RichText};

use crate::puzzle::{Host, Puzzle, StepView, steps};
use crate::ui::theme::{AMBER, CYAN, DIM, GREEN, RED, TEXT, mono, retro};
use crate::ui::widgets::{Kind, Tone, button, card, hex_diff, lede, note, select_card, shorten, tag};
use crate::world::World;

struct Guest {
    name: &'static str,
    /// what the slip says
    slip: Fingerprint,
    has_id: bool,
    looked_up: bool,
    /// which keyserver entry the player picked
    picked: Option<usize>,
    signed: bool,
    refused: bool,
}

struct ServerKey {
    uid: &'static str,
    fp: Fingerprint,
    forged: bool,
}

pub struct KeyParty {
    world: Rc<World>,
    guests: Vec<Guest>,
    server: Vec<ServerKey>,
    sel: usize,
    forge_tries: u64,
    sigs: Vec<(String, String)>,
    mistakes: Vec<String>,
}

impl KeyParty {
    pub fn new(world: Rc<World>) -> Self {
        let g = |id: &'static str, name: &'static str, has_id: bool| Guest { name, slip: world.key_of(id).fp, has_id, looked_up: false, picked: None, signed: false, refused: false };
        let guests = vec![g("felix", "Felix", true), g("mara", "Mara", true), g("juno", "Juno", true), g("c2", "Ines (fisher)", true), g("c5", "Ravi (courier)", false)];
        let mut server: Vec<ServerKey> = vec![
            ServerKey { uid: "Felix <felix@exchange>", fp: world.key_of("felix").fp, forged: false },
            ServerKey { uid: "Mara <mara@wharf.radio>", fp: world.key_of("mara").fp, forged: false },
            ServerKey { uid: "Juno <juno@commons.park>", fp: world.key_of("juno").fp, forged: false },
            ServerKey { uid: "Ines <ines@wharf>", fp: world.key_of("c2").fp, forged: false },
            ServerKey { uid: "Ravi <ravi@avenue>", fp: world.key_of("c5").fp, forged: false },
        ];
        // the Directorate's colliding key for Juno
        let dla = world.key_of("dla-evil32");
        let (_, fake, tries) = collide_short_id(&world.key_of("juno").fp, &dla.xonly, dla.created);
        server.push(ServerKey { uid: "Juno <juno@commons.park>", fp: fake, forged: true });
        // shuffle so the list's order gives nothing away
        world.rng("party/order").shuffle(&mut server);
        Self { world, guests, server, sel: 0, forge_tries: tries, sigs: Vec::new(), mistakes: Vec::new() }
    }

    fn matches(&self, g: usize) -> Vec<usize> {
        let sid = short_id(&self.guests[g].slip);
        (0..self.server.len()).filter(|i| short_id(&self.server[*i].fp) == sid).collect()
    }

    fn look_up(&mut self, g: usize) {
        self.guests[g].looked_up = true;
        let m = self.matches(g);
        if m.len() == 1 {
            self.guests[g].picked = Some(m[0]);
        }
    }

    fn sign(&mut self, g: usize, host: &mut Host) {
        let guest = &self.guests[g];
        let Some(k) = guest.picked else {
            host.status("Pick the keyserver entry you are signing first.", Tone::Warn);
            return;
        };
        let key = &self.server[k];
        if key.fp != guest.slip {
            let same = to_hex(&key.fp).chars().zip(to_hex(&guest.slip).chars()).filter(|(a, b)| a == b).count();
            self.mistakes.push(format!("{}: signed a key that matches the slip in only {same} of 40 digits", guest.name));
            host.status(format!("Stop. That key matches {}'s slip in the last four digits (the short ID) and almost nothing else. Someone generated it to collide. Compare the whole fingerprint.", guest.name), Tone::Bad);
            host.sfx("error");
            self.guests[g].picked = None;
            return;
        }
        if !guest.has_id {
            self.mistakes.push(format!("{}: signed without checking an ID", guest.name));
            host.status(format!("The slip matches the key, but you have nothing showing the slip is {}'s. A signature says \"I checked\". You didn't.", guest.name), Tone::Bad);
            host.sfx("error");
            return;
        }
        let me = self.world.key_of("you");
        let sig = certify(&me.secret, &me.fp, &key.fp, key.uid);
        let name = guest.name;
        self.sigs.push((name.to_string(), sig.hex()));
        self.guests[g].signed = true;
        host.sfx("confirm");
        host.status(format!("Signed {name}'s key: a BIP-340 signature over your fingerprint, theirs, and their name."), Tone::Ok);
        self.check_done(host);
    }

    fn refuse(&mut self, g: usize, host: &mut Host) {
        let guest = &self.guests[g];
        if guest.has_id {
            host.status(format!("{} showed you an ID and the slip matches. Refusing is safe, but you could have signed.", guest.name), Tone::Warn);
        } else {
            host.status(format!("Right. {} can get your signature next time, with an ID. Felix knows them and has signed already; that may be enough for you later.", guest.name), Tone::Ok);
        }
        self.guests[g].refused = true;
        self.check_done(host);
    }

    fn check_done(&mut self, host: &mut Host) {
        let ok = self.guests.iter().all(|g| if g.has_id { g.signed } else { g.refused && !g.signed });
        if ok {
            host.sfx("unlock");
            host.solve(format!("Four keys signed, one politely declined, one forgery caught (it took the Directorate about {} tries to make).", self.forge_tries));
        }
    }

    fn step_flags(&self) -> (bool, bool, bool, bool) {
        let looked = self.guests.iter().all(|g| g.looked_up);
        let picked = self.guests.iter().filter(|g| g.has_id).all(|g| g.picked.is_some());
        let decided = self.guests.iter().all(|g| g.signed || g.refused);
        (looked, picked, decided, self.guests.iter().any(|g| g.signed))
    }
}

impl Puzzle for KeyParty {
    fn id(&self) -> &'static str {
        "party.keys"
    }

    fn title(&self) -> &str {
        "KEY-SIGNING PARTY"
    }

    fn subtitle(&self) -> Option<&str> {
        Some("PGP web of trust · check, then sign")
    }

    fn accent(&self) -> Color32 {
        AMBER
    }

    fn open(&mut self, host: &mut Host) {
        host.status("Five guests, five slips. Sign only what you have checked yourself.", Tone::Info);
    }

    fn steps(&self) -> Vec<StepView> {
        let (looked, picked, decided, _) = self.step_flags();
        steps(&[
            ("Look up each slip's short ID on the keyserver", "Click a guest, then \"Look up\". The keyserver searches by the last four hex digits.", looked),
            ("Compare the whole fingerprint", "Two keys can share a short ID. Only the full 40 digits identify a key.", picked),
            ("Check the ID in person, then sign or decline", "Sign only keys whose owner you have identified. No ID, no signature.", decided),
        ])
    }

    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        lede(ui, "Felix has pushed the tables together. Everyone brought a paper slip with their key's fingerprint, and something with their name on it. The keyserver on Felix's laptop lists who uploaded what. The Directorate knows about this party too.");
        let mut act: Option<(usize, u8)> = None;
        ui.horizontal_wrapped(|ui| {
            for (i, g) in self.guests.iter().enumerate() {
                let state = if g.signed { "✓" } else if g.refused { "✕" } else { "" };
                let on = self.sel == i;
                let b = egui::Button::new(RichText::new(format!("{} {}", g.name, state)).font(mono(13.5)).color(if g.signed { GREEN } else if on { CYAN } else { TEXT })).fill(if on { crate::ui::theme::rgb(0x13243a) } else { crate::ui::theme::rgb(0x111726) });
                if ui.add(b).clicked() {
                    self.sel = i;
                }
            }
        });
        let gi = self.sel;
        let matches = self.matches(gi);
        let g = &self.guests[gi];
        card(ui, Some(format!("{}'s slip", g.name).as_str()), None, |ui| {
            ui.label(RichText::new(fp_display(&g.slip)).font(mono(15.0)).color(AMBER));
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("short ID {}", short_id(&g.slip))).font(mono(12.5)).color(DIM));
                if g.has_id {
                    tag(ui, "shows a photo ID", Tone::Ok);
                } else {
                    tag(ui, "\"left my ID at home, sorry\"", Tone::Warn);
                }
            });
            if !g.looked_up && button(ui, &format!("Look up {} on the keyserver", short_id(&g.slip)), Kind::Primary).clicked() {
                act = Some((gi, 0));
            }
        });
        if g.looked_up {
            card(ui, Some("Keyserver results"), None, |ui| {
                note(ui, &format!("{} key(s) with short ID **{}**. Pick the one whose whole fingerprint matches the slip (mismatched digits in red).", matches.len(), short_id(&g.slip)));
                for &k in &matches {
                    let key = &self.server[k];
                    let picked = g.picked == Some(k);
                    let solved = host.solved();
                    if select_card(ui, picked, CYAN, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(key.uid).font(mono(13.0)).color(TEXT));
                            if solved && key.forged {
                                tag(ui, "uploaded tonight by the DLA", Tone::Bad);
                            }
                        });
                        hex_diff(ui, &fp_display(&g.slip), &fp_display(&key.fp));
                    }) {
                        act = Some((gi, 10 + k as u8));
                    }
                }
                if !g.signed && !g.refused {
                    ui.horizontal(|ui| {
                        if button(ui, "Sign this key", Kind::Primary).clicked() {
                            act = Some((gi, 1));
                        }
                        if button(ui, "Decline to sign", Kind::Normal).clicked() {
                            act = Some((gi, 2));
                        }
                    });
                }
            });
        }
        match act {
            Some((g, 0)) => self.look_up(g),
            Some((g, 1)) => self.sign(g, host),
            Some((g, 2)) => self.refuse(g, host),
            Some((g, k)) if k >= 10 => self.guests[g].picked = Some((k - 10) as usize),
            _ => {}
        }
        if !self.sigs.is_empty() {
            card(ui, Some("Your signatures"), None, |ui| {
                for (who, sig) in &self.sigs {
                    ui.label(RichText::new(format!("{who}: {}", shorten(sig, 16, 12))).font(mono(12.0)).color(GREEN));
                }
            });
        }
        if !self.mistakes.is_empty() {
            card(ui, Some("Caught in time"), Some(RED), |ui| {
                for m in &self.mistakes {
                    ui.label(RichText::new(m).font(mono(12.0)).color(RED));
                }
            });
        }
        if host.solved() {
            ui.label(RichText::new("THE WEB GROWS").font(retro(20.0)).color(GREEN));
            note(ui, "Real short IDs were 8 hex digits. In 2014 the Evil32 project made a colliding key for every one of the ~50,000 keys in the PGP strong set, each in seconds on a GPU. Checking 4 or 8 digits is not checking.");
        }
    }

    fn autoplay(&mut self, host: &mut Host) {
        for g in 0..self.guests.len() {
            self.look_up(g);
            let slip = self.guests[g].slip;
            self.guests[g].picked = self.matches(g).into_iter().find(|k| self.server[*k].fp == slip);
            if self.guests[g].has_id {
                self.sign(g, host);
            } else {
                self.refuse(g, host);
            }
        }
    }
}
