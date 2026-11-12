//! Chapter 5: the remailers. A whistleblower called Kestrel wants to send the
//! Archive's missing volume to the press. First decide which of two keys
//! claiming to be Kestrel's is real (the web of trust), then route a reply
//! through three remailers by wrapping it in layers from the inside out, and
//! finally beat a traffic analyst who watches the remailers' wires.

use cpl_crypto::drbg::Drbg;
use cpl_crypto::onion::{Hop, Policy, Remailer, Trace, fifo_attack, pad, route_packet, seal_layer, simulate};
use cpl_crypto::wot::{OwnerTrust, long_id};
use egui::{Color32, RichText};

use crate::allies::ally_def;
use crate::puzzle::{HandCell, HandSheet, Host, Puzzle, StepView, steps};
use crate::script::Ctx;
use crate::ui::theme::{AMBER, CYAN, DIM, GREEN, RED, TEXT, VIOLET, mono};
use crate::ui::widgets::{Kind, Log, Tone, button, button_enabled, card, lede, note, select_card};

const MESSAGE: &str = "KESTREL: received. Send volume 7 to the Harbour Light. Rosa will publish. --{{handle}}";
const SIGNERS: [&str; 3] = ["mara", "felix", "juno"];

pub struct RemailerPuzzle {
    ctx: Ctx,
    rng: Drbg,
    net: Vec<Remailer>,
    chosen_key: Option<usize>,
    key_ok: bool,
    route: Vec<usize>,
    /// layers sealed so far, innermost first: remailer index per layer
    layers: Vec<usize>,
    packet: Vec<u8>,
    delivered: Option<Vec<Trace>>,
    batch: usize,
    padded: bool,
    analysis: Option<(usize, usize)>,
    log: Log,
}

impl RemailerPuzzle {
    pub fn new(ctx: Ctx) -> Self {
        let w = ctx.world();
        let net = w.remailers().to_vec();
        let rng = w.rng("ch5/remailer");
        ctx.set("ch5.kestrel");
        Self { ctx, rng, net, chosen_key: None, key_ok: false, route: Vec::new(), layers: Vec::new(), packet: Vec::new(), delivered: None, batch: 1, padded: false, analysis: None, log: Log::default() }
    }

    fn message(&self) -> String {
        MESSAGE.replace("{{handle}}", self.ctx.handle().as_str())
    }

    fn choose_key(&mut self, i: usize, host: &mut Host) {
        self.chosen_key = Some(i);
        let web = self.ctx.state_ref().web();
        let id = ["kestrel", "kestrel-dla"][i];
        if !web.valid(id) {
            self.key_ok = false;
            host.status("You can't call that key valid yet: nobody you trust enough has signed it. Adjust how much you trust the people who did, or pick the other key.", Tone::Warn);
            return;
        }
        if i == 1 {
            self.key_ok = false;
            host.status("That one is signed by the DLA's own \"Key Notary\". You trusted the Directorate to vouch for a whistleblower?", Tone::Bad);
            return;
        }
        self.key_ok = true;
        host.sfx("confirm");
        host.status("Valid through your web: signed by friends you trust. Now route the reply so no single remailer knows both ends.", Tone::Ok);
    }

    fn toggle_route(&mut self, r: usize) {
        if let Some(p) = self.route.iter().position(|x| *x == r) {
            self.route.truncate(p);
        } else if self.route.len() < 3 {
            self.route.push(r);
        }
        self.layers.clear();
        self.packet.clear();
        self.delivered = None;
    }

    /// Seal one layer for remailer `r` around what we have so far.
    fn seal(&mut self, r: usize, host: &mut Host) {
        if self.route.len() < 3 {
            host.status("Pick three remailers first.", Tone::Warn);
            return;
        }
        let inner = if self.layers.is_empty() {
            let m = self.message().into_bytes();
            if self.padded { pad(&m, 160) } else { m }
        } else {
            self.packet.clone()
        };
        // the hop inside this layer: where this remailer sends the inner packet
        let hop = match self.layers.last() {
            None => Hop::Deliver("kestrel@nym".into()),
            Some(&prev) => Hop::Forward(self.net[prev].name.clone()),
        };
        self.packet = seal_layer(&self.net[r].public, &hop, &inner, &mut self.rng);
        self.layers.push(r);
        self.log.line(format!("sealed a layer for {} ({:?}) → packet {} bytes", self.net[r].name, hop, self.packet.len()), Tone::Info);
        host.sfx("blip");
    }

    fn send(&mut self, host: &mut Host) {
        let Some(&first) = self.layers.last() else { return };
        match route_packet(&self.net, &self.net[first].name.clone(), &self.packet) {
            Ok((to, body, traces)) => {
                let ok_route = self.layers.iter().rev().copied().collect::<Vec<_>>() == self.route;
                for t in &traces {
                    self.log.line(format!("{} peeled one layer ({} bytes in) and learned only: {:?}", t.remailer, t.size_in, t.hop), Tone::Ok);
                }
                let _ = body;
                if to == "kestrel@nym" && traces.len() == 3 && ok_route {
                    self.delivered = Some(traces);
                    host.sfx("unlock");
                    host.status("Delivered. No remailer knew both who sent it and who received it. But someone is watching the wires...", Tone::Ok);
                } else {
                    host.status("It arrived, but not along the route you picked. Seal the last hop first, the first hop last.", Tone::Warn);
                }
            }
            Err(e) => {
                self.log.line(format!("{} could not open the packet: {e:?}", self.net[first].name), Tone::Bad);
                host.status("The first remailer can't open its layer: the outermost layer must be sealed for the first hop. Wrap from the inside out.", Tone::Bad);
                host.sfx("error");
                self.layers.clear();
                self.packet.clear();
            }
        }
    }

    fn analyse(&mut self, host: &mut Host) {
        let mut rng = self.ctx.world().rng(&format!("ch5/mix/{}", self.batch));
        let arrivals: Vec<f64> = (0..24).map(|i| i as f64 * 0.6 + rng.int(10) as f64 * 0.03).collect();
        let policy = if self.batch <= 1 { Policy::Immediate { delay: 0.4 } } else { Policy::Batch { batch: self.batch } };
        let s = simulate(&arrivals, policy, &mut rng);
        let mut linked = fifo_attack(&s);
        if !self.padded {
            // without padding, sizes shrink hop by hop and give the position away
            linked = linked.max(24 / 2);
        }
        self.analysis = Some((linked, 24));
        let expected = 24 / self.batch.max(1);
        if self.padded && self.batch >= 6 {
            host.sfx("unlock");
            host.solve(format!("The analyst linked {linked} of 24 by luck (about {expected} expected). Batching hides the order, padding hides the sizes, layers hide the content."));
        } else if !self.padded {
            host.status("Sizes give it away: each remailer strips a layer of about 70 bytes, so lengths match up hop by hop. Pad to a fixed size.", Tone::Bad);
        } else {
            host.status(format!("The analyst links {linked} of 24 messages by timing (about {expected} expected). Hold more messages before releasing them."), Tone::Warn);
        }
    }
}

impl Puzzle for RemailerPuzzle {
    fn id(&self) -> &'static str {
        "ch5.remail"
    }

    fn title(&self) -> &str {
        "THE REMAILERS"
    }

    fn subtitle(&self) -> Option<&str> {
        Some("onion layers · mixing · web of trust")
    }

    fn accent(&self) -> Color32 {
        VIOLET
    }

    fn open(&mut self, host: &mut Host) {
        host.status("Two keys say they are Kestrel's. Which one do you trust?", Tone::Info);
    }

    fn steps(&self) -> Vec<StepView> {
        steps(&[
            ("Decide which Kestrel key is real", "Check who signed each key. Set how much you trust those signers until the right key becomes valid.", self.key_ok),
            ("Choose three remailers", "Any three, in the order the message should travel.", self.route.len() == 3),
            ("Wrap the layers from the inside out", "The innermost layer is for the last remailer; the outermost for the first.", self.layers.len() == 3),
            ("Send it", "Each remailer peels one layer and learns only the next hop.", self.delivered.is_some()),
            ("Beat the traffic analyst", "Pad every message to one size and release them in batches of six or more.", self.analysis.is_some() && self.padded && self.batch >= 6),
        ])
    }

    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        lede(ui, "Kestrel works inside the Directorate and has the Archive's missing volume. Lin runs one of the city's remailers. A reply must reach Kestrel without any single machine knowing who wrote it to whom, and without the Directorate learning it from the timing.");
        let mut act: Option<(u8, usize)> = None;
        // --- 1. which key ---
        let web = self.ctx.state_ref().web();
        card(ui, Some("1 · Two keys called Kestrel"), None, |ui| {
            for (i, id) in ["kestrel", "kestrel-dla"].iter().enumerate() {
                let Some(n) = web.index(id) else { continue };
                let signers: Vec<String> = web.certs.iter().filter(|(_, t)| *t == n).map(|(s, _)| ally_def(&web.ids[*s]).map(|a| a.name.to_string()).unwrap_or_else(|| "DLA Key Notary".into())).collect();
                let valid = web.validity[n].is_valid();
                if select_card(ui, self.chosen_key == Some(i), CYAN, |ui| {
                    ui.label(RichText::new(format!("{} · key {}", web.nodes[n].uid, long_id(&web.nodes[n].fp))).font(mono(13.0)).color(TEXT));
                    ui.label(RichText::new(format!("signed by: {}", if signers.is_empty() { "nobody".into() } else { signers.join(", ") })).font(mono(12.0)).color(DIM));
                    ui.label(RichText::new(if valid { "VALID for you" } else { "not valid for you (yet)" }).font(mono(12.0)).color(if valid { GREEN } else { AMBER }));
                }) {
                    act = Some((1, i));
                }
            }
            note(ui, "How much do you trust each signer to check keys properly? One **full** or three **marginal** signers make a key valid.");
            ui.horizontal_wrapped(|ui| {
                for (j, id) in SIGNERS.iter().enumerate() {
                    if !self.ctx.state_ref().is_ally(id) {
                        continue;
                    }
                    let cur = self.ctx.state_ref().owner_trust(id);
                    let mut sel = cur;
                    ui.label(RichText::new(ally_def(id).map(|a| a.name).unwrap_or(id)).font(mono(12.5)).color(TEXT));
                    egui::ComboBox::from_id_salt(("rtrust", *id)).width(90.0).selected_text(sel.label()).show_ui(ui, |ui| {
                        for t in [OwnerTrust::Unknown, OwnerTrust::Never, OwnerTrust::Marginal, OwnerTrust::Full] {
                            ui.selectable_value(&mut sel, t, t.label());
                        }
                    });
                    if sel != cur {
                        act = Some((10 + j as u8, crate::allies::trust_to(sel) as usize));
                    }
                }
            });
        });
        // --- 2-4. the route ---
        if self.key_ok {
            card(ui, Some("2 · The route"), None, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for (i, r) in self.net.iter().enumerate() {
                        let pos = self.route.iter().position(|x| *x == i);
                        let label = match pos {
                            Some(p) => format!("{}. {}", p + 1, r.name),
                            None => r.name.clone(),
                        };
                        if ui.add(egui::Button::new(RichText::new(label).font(mono(13.5)).color(if pos.is_some() { CYAN } else { TEXT }))).clicked() {
                            act = Some((2, i));
                        }
                    }
                });
                ui.label(RichText::new(format!("route: you → {} → Kestrel", self.route.iter().map(|i| self.net[*i].name.as_str()).collect::<Vec<_>>().join(" → "))).font(mono(12.5)).color(DIM));
                ui.checkbox(&mut self.padded, RichText::new("pad the message to a fixed 160 bytes before sealing").font(mono(12.5)).color(TEXT));
            });
            if self.route.len() == 3 {
                card(ui, Some("3 · Seal the layers"), None, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for &i in &self.route.clone() {
                            if button_enabled(ui, self.layers.len() < 3 && !self.layers.contains(&i), &format!("Seal for {}", self.net[i].name), Kind::Normal).clicked() {
                                act = Some((3, i));
                            }
                        }
                        if button(ui, "Start over", Kind::Ghost).clicked() {
                            act = Some((5, 0));
                        }
                    });
                    let onion: Vec<&str> = self.layers.iter().rev().map(|i| self.net[*i].name.as_str()).collect();
                    ui.label(RichText::new(format!("onion, outside in: [{}] message", onion.join(" [ "))).font(mono(12.5)).color(AMBER));
                    if button_enabled(ui, self.layers.len() == 3 && self.delivered.is_none(), "Send", Kind::Primary).clicked() {
                        act = Some((4, 0));
                    }
                });
            }
        }
        // --- 5. the analyst ---
        if self.delivered.is_some() {
            card(ui, Some("4 · The traffic analyst"), Some(RED), |ui| {
                note(ui, "The Directorate cannot read the packets, but it sees every one go into a remailer and come out. A remailer that forwards each message at once gives away who talks to whom by timing alone.");
                ui.add(egui::Slider::new(&mut self.batch, 1..=10).text("release in batches of"));
                if button(ui, "Run the analyst against this setting", Kind::Primary).clicked() {
                    act = Some((6, 0));
                }
                if let Some((linked, n)) = self.analysis {
                    ui.label(RichText::new(format!("analyst links {linked} of {n} messages")).font(mono(14.0)).color(if linked <= 6 { GREEN } else { RED }));
                }
            });
        }
        match act {
            Some((1, i)) => self.choose_key(i, host),
            Some((2, i)) => self.toggle_route(i),
            Some((3, i)) => self.seal(i, host),
            Some((4, _)) => self.send(host),
            Some((5, _)) => {
                self.layers.clear();
                self.packet.clear();
            }
            Some((6, _)) => self.analyse(host),
            Some((k, t)) if (10..13).contains(&k) => {
                let id = SIGNERS[(k - 10) as usize];
                self.ctx.state().set_trust(id, crate::allies::trust_from(t as u8));
                if let Some(i) = self.chosen_key {
                    self.choose_key(i, host);
                }
            }
            _ => {}
        }
        self.log.show(ui, "remail-log", 130.0);
    }

    fn autoplay(&mut self, host: &mut Host) {
        for id in SIGNERS {
            if self.ctx.state_ref().is_ally(id) {
                self.ctx.state().set_trust(id, OwnerTrust::Marginal);
            }
        }
        if !self.ctx.state_ref().web().valid("kestrel") {
            // fewer than three friends signed: trust Felix fully
            self.ctx.state().set_trust("felix", OwnerTrust::Full);
        }
        self.choose_key(0, host);
        self.route = vec![0, 2, 1];
        self.padded = true;
        for r in [1, 2, 0] {
            self.seal(r, host);
        }
        self.send(host);
        self.batch = 8;
        self.analyse(host);
    }

    fn deep_sheet(&self) -> Option<HandSheet> {
        Some(HandSheet::new(
            "how much does mixing hide?",
            "24 messages pass through a threshold mix. If the analyst simply guesses that the k-th message in is the k-th out, how many does it link on average?",
            120.0,
            vec![
                HandCell::new("batch size 1 (no mixing)", "every guess is right", 24),
                HandCell::new("batch size 4", "a random order within each batch: 1 in 4 per message", 6),
                HandCell::new("batch size 8", "", 3),
                HandCell::new("with batch 8, one message is linked 1 time in …", "", 8),
            ],
        ))
    }
}
