//! Menu pages for the v3.1 systems: the wallet, the web of trust, the quest log.

use egui::{Color32, RichText, Stroke};

use super::theme::{self, AMBER, CYAN, DIM, FAINT, GREEN, RED, TEXT, VIOLET, mono, retro};
use super::widgets::{card, note, shorten, sub};
use crate::allies::{STRANGERS, ally_def, trust_to};
use crate::quests::{FLOPPIES, floppies_found, quest_log};
use crate::state::GameState;
use cpl_crypto::wot::{OwnerTrust, Validity, long_id};

pub fn wallet(ui: &mut egui::Ui, s: &GameState, height: f32) {
    egui::ScrollArea::vertical().id_salt("wallet").max_height(height).auto_shrink([false, true]).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("{} cr", s.credits())).font(retro(30.0)).color(GREEN));
            ui.label(RichText::new("in eCash").font(mono(13.0)).color(DIM));
            if s.data.ious > 0 {
                ui.add_space(12.0);
                ui.label(RichText::new(format!("{} cr", s.data.ious)).font(retro(24.0)).color(AMBER));
                ui.label(RichText::new("in IOUs (traceable: they carry your name)").font(mono(12.5)).color(DIM));
            }
        });
        if !s.wallet_open() {
            note(ui, "You have no wallet yet. The Commons Mint issues eCash; until then, people pay you in IOUs with your name on them.");
        }
        if !s.data.wallet.is_empty() {
            card(ui, Some("Tokens"), None, |ui| {
                note(ui, "Each token is a secret and the mint's signature on it, C = k·hash_to_curve(secret). The mint signed a blinded version, so it has never seen either value. When you spend one, it sees them for the first time and has nothing to match them against.");
                egui::Grid::new("tokens").num_columns(3).striped(true).spacing([14.0, 3.0]).show(ui, |ui| {
                    for h in ["amount", "secret", "signature C"] {
                        ui.label(RichText::new(h).font(mono(11.5)).color(DIM));
                    }
                    ui.end_row();
                    for t in &s.data.wallet {
                        ui.label(RichText::new(format!("{} cr", t.amount)).font(mono(12.5)).color(GREEN));
                        ui.label(RichText::new(shorten(&t.secret, 10, 6)).font(mono(12.0)).color(TEXT));
                        ui.label(RichText::new(shorten(&t.c, 10, 6)).font(mono(12.0)).color(TEXT));
                        ui.end_row();
                    }
                });
            });
        }
        if !s.data.stamps.is_empty() {
            card(ui, Some("Hashcash stamps"), None, |ui| {
                for st in &s.data.stamps {
                    ui.label(RichText::new(st).font(mono(11.5)).color(CYAN));
                }
            });
        }
        let st = &s.data.stats;
        card(ui, Some("Record"), None, |ui| {
            egui::Grid::new("stats").num_columns(2).spacing([16.0, 2.0]).show(ui, |ui| {
                for (k, v) in [
                    ("earned", format!("{} cr", st.earned)),
                    ("spent", format!("{} cr in {} payments the mint could not link to you", st.spent, st.payments)),
                    ("stamps minted", st.stamps_minted.to_string()),
                    ("flagged by the Dragnet", st.flagged.to_string()),
                    ("deep dives", st.deep_dives.to_string()),
                ] {
                    ui.label(RichText::new(k).font(mono(12.0)).color(DIM));
                    ui.label(RichText::new(v).font(mono(12.5)).color(TEXT));
                    ui.end_row();
                }
            });
        });
    });
}

fn validity_text(v: &Validity) -> (String, Color32) {
    match v {
        Validity::Ultimate => ("your key".into(), CYAN),
        Validity::Valid { depth: 1, .. } => ("valid: you signed it".into(), GREEN),
        Validity::Valid { depth, full, marginal } => (format!("valid at depth {depth} ({full} full, {marginal} marginal)"), GREEN),
        Validity::Partial { full, marginal } => (format!("not enough: {full} full, {marginal} marginal (need 1 or 3)"), AMBER),
        Validity::Unknown => ("unknown: nobody you trust signed it".into(), RED),
    }
}

/// The web of trust. Returns a trust change the player made.
pub fn network(ui: &mut egui::Ui, s: &GameState, height: f32) -> Option<(String, u8)> {
    let mut change = None;
    let web = s.web();
    egui::ScrollArea::vertical().id_salt("network").max_height(height).auto_shrink([false, true]).show(ui, |ui| {
        if web.ids.len() <= 1 {
            note(ui, "Nobody yet. You need your own key first (it is in Grandpa's Vault); then sign the keys of people you have checked in person.");
            return;
        }
        // the graph
        let size = egui::vec2(ui.available_width().min(560.0), 230.0);
        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        let p = ui.painter_at(rect);
        p.rect_filled(rect, 4.0, theme::rgb(0x080c18));
        let c = rect.center();
        let n = web.ids.len();
        let pos: Vec<egui::Pos2> = (0..n)
            .map(|i| {
                if i == 0 {
                    return c;
                }
                let stranger = STRANGERS.iter().any(|x| x.id == web.ids[i]);
                let r = if stranger { 100.0 } else { 74.0 };
                let a = (i as f32 - 1.0) / (n as f32 - 1.0).max(1.0) * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
                c + egui::vec2(a.cos() * r * 2.2, a.sin() * r)
            })
            .collect();
        for &(a, b) in &web.certs {
            let col = if a == 0 { CYAN.gamma_multiply(0.6) } else { VIOLET.gamma_multiply(0.45) };
            p.line_segment([pos[a], pos[b]], Stroke::new(1.0, col));
            let mid = pos[a] + (pos[b] - pos[a]) * 0.8;
            p.circle_filled(mid, 2.0, col);
        }
        for i in 0..n {
            let (_, col) = validity_text(&web.validity[i]);
            p.circle_filled(pos[i], if i == 0 { 7.0 } else { 5.0 }, col);
            let name = if i == 0 { s.handle().to_string() } else { ally_def(&web.ids[i]).map(|a| a.name.to_string()).unwrap_or_else(|| web.ids[i].clone()) };
            p.text(pos[i] + egui::vec2(0.0, -9.0), egui::Align2::CENTER_BOTTOM, name, mono(11.0), TEXT);
        }
        note(ui, "Lines are signatures; a dot marks the signed end. Cyan: yours. A key is valid if you signed it, or if one key you trust **fully** or three you trust **marginally** signed it (GnuPG's rules, up to five steps away). Trust is about the person's judgement, not their identity.");
        ui.add_space(4.0);
        egui::Grid::new("web").num_columns(4).striped(true).spacing([10.0, 4.0]).show(ui, |ui| {
            for h in ["key", "long ID", "validity", "you trust their signatures"] {
                ui.label(RichText::new(h).font(mono(11.5)).color(DIM));
            }
            ui.end_row();
            for (i, id) in web.ids.iter().enumerate() {
                let node = &web.nodes[i];
                ui.label(RichText::new(&node.uid).font(mono(12.0)).color(TEXT));
                ui.label(RichText::new(long_id(&node.fp)).font(mono(12.0)).color(FAINT));
                let (txt, col) = validity_text(&web.validity[i]);
                ui.label(RichText::new(txt).font(mono(12.0)).color(col));
                if i == 0 {
                    ui.label(RichText::new("ultimate").font(mono(12.0)).color(CYAN));
                } else {
                    let cur = node.trust;
                    let mut sel = cur;
                    egui::ComboBox::from_id_salt(("trust", id.as_str())).selected_text(sel.label()).show_ui(ui, |ui| {
                        for t in [OwnerTrust::Unknown, OwnerTrust::Never, OwnerTrust::Marginal, OwnerTrust::Full] {
                            ui.selectable_value(&mut sel, t, t.label());
                        }
                    });
                    if sel != cur {
                        change = Some((id.clone(), trust_to(sel)));
                    }
                }
                ui.end_row();
            }
        });
    });
    change
}

pub fn quests(ui: &mut egui::Ui, s: &GameState, height: f32) {
    egui::ScrollArea::vertical().id_salt("quests").max_height(height).auto_shrink([false, true]).show(ui, |ui| {
        let log = quest_log(s);
        if log.is_empty() {
            note(ui, "No side quests yet. People around the city will ask for help; talk to everyone.");
        }
        for (q, task) in log {
            card(ui, Some(q.title), if task.is_none() { Some(GREEN) } else { None }, |ui| {
                ui.label(RichText::new(format!("from {}", q.giver)).font(mono(12.0)).color(DIM));
                match task {
                    Some(t) => {
                        sub(ui, "To do");
                        note(ui, t);
                    }
                    None => {
                        ui.label(RichText::new("DONE").font(retro(16.0)).color(GREEN));
                    }
                }
                if q.id == "floppies" {
                    ui.label(RichText::new(format!("{} / {FLOPPIES} found", floppies_found(s))).font(mono(12.5)).color(AMBER));
                }
                note(ui, q.idea);
            });
        }
    });
}
