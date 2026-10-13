//! Chapter 4, part 3. Grandpa's Vault was a Taproot output: an internal key
//! tweaked by a Merkle tree of spending scripts. Only the leaves actually used
//! are ever revealed. Try leaf pairs until one reproduces the published
//! address, learning what he committed to without him ever spending it.

use std::rc::Rc;

use cpl_crypto::bytes::to_hex;
use cpl_crypto::secp::x_only;
use cpl_crypto::taproot::{LEAF_VERSION_TAPSCRIPT, tap_branch_hash, tap_leaf_hash, tweak_key};
use egui::{Color32, RichText};

use crate::puzzle::{Host, Puzzle};
use crate::ui::theme::{self, AMBER, CYAN, DIM, GREEN, RED, TEXT, mono};
use crate::ui::widgets::{Kind, Tone, button, card, lede, math, note, shorten, tag};
use crate::world::{World, hex32};

struct Outcome {
    root: [u8; 32],
    tweak: String,
    address: String,
    matched: bool,
    names: String,
}

pub struct TaprootReveal {
    world: Rc<World>,
    internal_x: [u8; 32],
    selected: Vec<String>,
    outcome: Option<Outcome>,
}

impl TaprootReveal {
    pub fn new(world: Rc<World>) -> Self {
        let internal_x = x_only(&world.grandpa().p);
        Self { world, internal_x, selected: Vec::new(), outcome: None }
    }

    fn toggle(&mut self, id: &str, on: bool) {
        if on {
            if self.selected.len() >= 2 || self.selected.iter().any(|s| s == id) {
                return;
            }
            self.selected.push(id.to_string());
        } else {
            self.selected.retain(|s| s != id);
        }
    }

    fn check(&mut self, host: &mut Host) {
        if host.solved() {
            return;
        }
        if self.selected.len() != 2 {
            host.status("Pick exactly two.", Tone::Warn);
            return;
        }
        let tp = self.world.taproot();
        let leaf = |id: &str| tp.leaves.iter().find(|l| l.id == id).expect("known leaf");
        let (a, b) = (leaf(&self.selected[0]), leaf(&self.selected[1]));
        let root = tap_branch_hash(&tap_leaf_hash(&a.script, LEAF_VERSION_TAPSCRIPT), &tap_leaf_hash(&b.script, LEAF_VERSION_TAPSCRIPT));
        let Ok(tw) = tweak_key(&self.internal_x, Some(&root), "bc") else { return };
        let matched = tw.address == tp.target.address;
        // name the pair in the order Grandpa drafted them
        let (first, second) = if tp.leaves.iter().position(|l| l.id == a.id) < tp.leaves.iter().position(|l| l.id == b.id) { (a, b) } else { (b, a) };
        self.outcome = Some(Outcome { root, tweak: hex32(&tw.tweak), address: tw.address, matched, names: format!("{}  ·  {}", first.label, second.label) });
        if !matched {
            host.status("Those two leaves give a different address. Try another pair.", Tone::Bad);
            host.sfx("error");
            return;
        }
        host.sfx("unlock");
        host.solve("Taproot confirmed. The scripts he hid were a recovery path and an inheritance path. Both lead to you.");
    }
}

impl Puzzle for TaprootReveal {
    fn id(&self) -> &'static str {
        "ch4.taproot"
    }

    fn title(&self) -> &str {
        "GRANDPA'S TAPROOT"
    }

    fn subtitle(&self) -> Option<&str> {
        Some("BIP-341 · key path + hidden scripts")
    }

    fn accent(&self) -> Color32 {
        theme::rgb(0x73eff7)
    }

    fn open(&mut self, host: &mut Host) {
        host.status("Choose two spending conditions and check the address.", Tone::Info);
    }

    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        let world = self.world.clone();
        let tp = world.taproot();
        lede(ui, "The Vault address was a Taproot output. Its key commits to a Merkle tree of spending scripts via a tweak. From the address alone you cannot see the scripts, but if you can guess which leaves are in the tree, you can confirm them, because they must reproduce the address.");
        math(ui, "Q = P + t·G,   t = H_TapTweak(P.x ‖ merkleRoot),   address = bech32m(Q.x)");
        card(ui, Some("What Grandpa published"), None, |ui| {
            egui::Grid::new("tr-pub").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                ui.label(RichText::new("internal key P").font(mono(12.5)).color(DIM));
                ui.label(RichText::new(shorten(&to_hex(&self.internal_x), 10, 8)).font(mono(12.5)).color(TEXT));
                ui.end_row();
                ui.label(RichText::new("Vault address").font(mono(12.5)).color(DIM));
                ui.add(egui::Label::new(RichText::new(&tp.target.address).font(mono(14.5)).color(AMBER)).wrap());
                ui.end_row();
            });
        });
        let mut toggles: Vec<(String, bool)> = Vec::new();
        let mut check = false;
        card(ui, Some("Candidate spending conditions"), None, |ui| {
            note(ui, "Grandpa drafted four. Pick the two you think he committed to. The tree is a single branch of exactly two leaves.");
            for leaf in &tp.leaves {
                let on = self.selected.iter().any(|s| *s == leaf.id);
                theme::card_frame(if on { Some(CYAN) } else { None }).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal_top(|ui| {
                        let mut v = on;
                        if ui.checkbox(&mut v, "").changed() {
                            toggles.push((leaf.id.clone(), v));
                        }
                        ui.vertical(|ui| {
                            ui.label(RichText::new(&leaf.label).font(mono(13.5)).color(TEXT).strong());
                            ui.add(egui::Label::new(RichText::new(&leaf.asm).font(mono(11.0)).color(DIM)).wrap());
                            ui.label(RichText::new(format!("TapLeaf hash {}", shorten(&to_hex(&tap_leaf_hash(&leaf.script, LEAF_VERSION_TAPSCRIPT)), 8, 6))).font(mono(10.5)).color(theme::FAINT));
                        });
                    });
                });
            }
            ui.horizontal(|ui| {
                if button(ui, "Build tree & compare address", Kind::Primary).clicked() {
                    check = true;
                }
                ui.label(RichText::new("selected:").font(mono(12.5)).color(DIM));
                tag(ui, &format!("{} / 2", self.selected.len()), if self.selected.len() == 2 { Tone::Info } else { Tone::Dim });
            });
        });
        for (id, on) in toggles {
            self.toggle(&id, on);
        }
        if check {
            self.check(host);
        }
        if let Some(o) = &self.outcome {
            let color = if o.matched { GREEN } else { RED };
            card(ui, Some(if o.matched { "Address matches" } else { "Address does not match" }), Some(color), |ui| {
                egui::Grid::new("tr-out").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                    let rows: [(&str, String, Color32); 4] = [
                        ("merkle root", shorten(&to_hex(&o.root), 10, 8), TEXT),
                        ("tweak t", shorten(&o.tweak, 10, 8), TEXT),
                        ("resulting address", o.address.clone(), color),
                        ("target", tp.target.address.clone(), AMBER),
                    ];
                    for (k, v, c) in rows {
                        ui.label(RichText::new(k).font(mono(12.5)).color(DIM));
                        ui.add(egui::Label::new(RichText::new(v).font(mono(12.5)).color(c)).wrap());
                        ui.end_row();
                    }
                });
                if o.matched {
                    note(ui, &format!("The tree was: **{}**. Everything else, including the Directorate's \"escrow\" leaf, was never committed. He built in a way for you to inherit, and left the Directorate out.", o.names));
                }
            });
        }
    }

    fn autoplay(&mut self, host: &mut Host) {
        let (a, b) = self.world.taproot().correct;
        self.toggle(a, true);
        self.toggle(b, true);
        self.check(host);
    }
}
