//! A counter that takes eCash. Every purchase runs the real payment: your
//! tokens go to the shop, the shop deposits them at the Commons Mint, the mint
//! checks each signature and its spent list, and your change comes back as
//! freshly blinded tokens. Not a puzzle; it just lives in a terminal.

use egui::{Color32, RichText};

use crate::economy::{Receipt, WARES, Ware};
use crate::puzzle::{Host, Puzzle, Size};
use crate::script::Ctx;
use crate::ui::theme::{AMBER, DIM, GREEN, TEXT, mono, retro};
use crate::ui::widgets::{Kind, Tone, button_enabled, card, lede, note};

pub struct Shop {
    ctx: Ctx,
    shop: &'static str,
    title: &'static str,
    receipt: Option<(String, Receipt)>,
    /// what was bought in this visit (ids)
    pub bought: Vec<&'static str>,
}

impl Shop {
    pub fn new(ctx: Ctx, shop: &'static str, title: &'static str) -> Self {
        Self { ctx, shop, title, receipt: None, bought: Vec::new() }
    }

    fn wares(&self) -> Vec<&'static Ware> {
        WARES.iter().filter(|w| w.shop == self.shop).collect()
    }

    fn owned(&self, w: &Ware) -> bool {
        w.item.is_some_and(|i| self.ctx.has(i))
    }

    fn buy(&mut self, w: &'static Ware, host: &mut Host) {
        let r = self.ctx.state().pay(w.price, w.name);
        match r {
            Ok(r) => {
                if let Some(item) = w.item {
                    self.ctx.give(item);
                }
                self.ctx.state().set(&format!("bought.{}", w.id), true);
                self.bought.push(w.id);
                host.sfx("coin");
                host.status(format!("{} is yours. The mint accepted {} token(s) it had never seen.", w.name, r.tokens.len()), Tone::Ok);
                self.receipt = Some((w.name.to_string(), r));
            }
            Err(e) => {
                host.sfx("error");
                host.status(e, Tone::Warn);
            }
        }
    }
}

impl Puzzle for Shop {
    fn id(&self) -> &'static str {
        "shop"
    }

    fn title(&self) -> &str {
        self.title
    }

    fn subtitle(&self) -> Option<&str> {
        Some("eCash only · the Commons Mint")
    }

    fn accent(&self) -> Color32 {
        GREEN
    }

    fn size(&self) -> Size {
        Size::Medium
    }

    fn open(&mut self, host: &mut Host) {
        let s = self.ctx.state_ref();
        if !s.wallet_open() {
            host.status("Cash only, and the only cash here is the Commons Mint's. You have no wallet yet.", Tone::Warn);
        } else {
            host.status(format!("You have {} cr.", s.credits()), Tone::Info);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        let (credits, open) = {
            let s = self.ctx.state_ref();
            (s.credits(), s.wallet_open())
        };
        lede(ui, "Pay with blind-signed tokens and the shop learns the colour of your money and nothing else. Neither does the mint.");
        let mut buy = None;
        for w in self.wares() {
            card(ui, None, None, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(w.name).font(retro(20.0)).color(AMBER));
                    ui.label(RichText::new(format!("{} cr", w.price)).font(mono(14.0)).color(GREEN));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let owned = self.owned(w);
                        let label = if owned { "owned" } else { "Buy" };
                        if button_enabled(ui, open && !owned && credits >= w.price, label, Kind::Primary).clicked() {
                            buy = Some(w);
                        }
                    });
                });
                note(ui, w.desc);
            });
        }
        if let Some(w) = buy {
            self.buy(w, host);
        }
        if let Some((what, r)) = &self.receipt {
            card(ui, Some("Receipt"), None, |ui| {
                ui.label(RichText::new(format!("{what}: {} cr", r.paid)).font(mono(13.0)).color(TEXT));
                for (amount, secret) in &r.tokens {
                    ui.label(RichText::new(format!("handed over {amount} cr token, secret {secret}… → mint: C = k·H(secret) ✓, not on the spent list ✓")).font(mono(12.0)).color(DIM));
                }
                if r.change > 0 {
                    ui.label(RichText::new(format!("change: {} cr, re-issued as new blind tokens", r.change)).font(mono(12.0)).color(DIM));
                }
            });
        }
    }

    /// Buy the first thing on the counter not bought yet (one per visit), then leave.
    fn autoplay(&mut self, host: &mut Host) {
        if self.bought.is_empty() {
            let credits = self.ctx.state_ref().credits();
            let next = self.wares().into_iter().find(|w| !self.owned(w) && !self.ctx.flag(&format!("bought.{}", w.id)) && w.price <= credits);
            if let Some(w) = next {
                self.buy(w, host);
                return;
            }
        }
        host.solve("");
    }
}
