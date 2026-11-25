//! The close-up of a patch panel: its jacks, the cords between them, the
//! log of what the lines said, and whatever you have plugged in (your
//! handset, your relay, a recorder, your phone). No instructions; the lines
//! call on their own clock and the street outside keeps moving.

use egui::{Align, Align2, Color32, CornerRadius, Layout, Margin, Pos2, RichText, Sense, Stroke, Vec2};

use super::theme::{self, AMBER, CYAN, DIM, FAINT, GOLD, GREEN, LINE2, PANEL, RED, TEXT, VIOLET, mono, retro};
use super::widgets::{Kind, button, button_enabled};
use crate::lines::exchange::{ESCROW_SEEN, RELAY};
use crate::lines::{ANSWER_MAX, CAMERA_ORDERS, End, HOLD_MAX, LineEvent, Lines, ORDERS, Role, Step, Tone, Via, Word};
use crate::signals::Signals;
use crate::state::GameState;

const CORD_COLORS: [Color32; 3] = [theme::rgb(0xffb347), theme::rgb(0x3ef0ff), theme::rgb(0xff3e9a)];

pub struct PanelView {
    pub panel: &'static str,
    /// the script waiting for the close-up to end
    pub reply: u64,
    pub notes: usize,
    /// a jack you picked up a cord at, waiting for its other end
    pub pick: Option<usize>,
    pub countersign: Option<Word>,
    pub order: Option<Word>,
    pub age: f32,
    pub shake: f32,
    /// how many log lines the view has seen (new ones flash)
    seen_log: usize,
    flash: f32,
    say: Option<(String, f32)>,
}

impl PanelView {
    pub fn new(panel: &'static str, reply: u64, log_len: usize) -> Self {
        Self { panel, reply, notes: 0, pick: None, countersign: None, order: None, age: 0.0, shake: 0.0, seen_log: log_len, flash: 9.0, say: None }
    }
}

#[derive(Default)]
pub struct PanelResult {
    pub close: bool,
}

fn tone_color(t: Tone) -> Color32 {
    match t {
        Tone::Plain => TEXT,
        Tone::Noise => FAINT,
        Tone::Relay => CYAN,
        Tone::System => DIM,
        Tone::Good => GREEN,
        Tone::Bad => RED,
    }
}

/// Draw the close-up and carry out what the player does.
#[allow(clippy::too_many_arguments)]
pub fn show(ctx: &egui::Context, v: &mut PanelView, lines: &mut Lines, sig: &Signals, st: &mut GameState, dt: f32, (seen, footsteps): (bool, bool), sfx: &mut Vec<&'static str>) -> PanelResult {
    let mut out = PanelResult::default();
    v.age += dt;
    v.shake = (v.shake - dt).max(0.0);
    v.flash += dt;
    if let Some((_, t)) = &mut v.say {
        *t += dt;
    }
    // react to what the lines did since the last frame
    for e in &lines.events {
        match e {
            LineEvent::Tamper { panel } | LineEvent::Alarm { panel } if *panel == v.panel => v.shake = 0.4,
            _ => {}
        }
    }
    let Some(p) = lines.panel(v.panel) else {
        out.close = true;
        return out;
    };
    if p.log.len() != v.seen_log {
        v.seen_log = p.log.len();
        v.flash = 0.0;
    }
    let site = p.site;
    let notes: Vec<&'static str> = sig.site(site).map(|s| s.notes.to_vec()).unwrap_or_default();
    let title = p.title;
    let relay_ok = st.flag(RELAY);
    let visible: Vec<usize> = (0..p.jacks.len()).filter(|&j| p.usable(st, j) || p.jacks[j].needs.is_none()).collect();
    let cords = p.cords.clone();
    let max_cords = p.max_cords;
    let jacks: Vec<(End, &'static str)> = p.jacks.iter().map(|j| (j.end, j.label)).collect();
    let call = p.call.clone();
    let log = p.log.clone();
    let heard = p.heard.clone();
    let rewrite = p.relay.rewrite;
    let (recorder, recording) = (p.recorder, p.recording);
    let phone = p.phone.is_some();
    let forged = p.forged;
    let schedule = p.schedule;
    let off_duty = p.until.is_some_and(|f| st.flag(f));
    let panel_t = p.t;
    let has_relay_jacks = p.jacks.iter().any(|j| matches!(j.end, End::Relay(_)));
    let camera_line = p.jacks.iter().any(|j| matches!(j.end, End::Line(id) if lines.party(id).is_some_and(|x| matches!(x.role, Role::Camera { .. }))));
    let orders_line = camera_line || p.jacks.iter().any(|j| matches!(j.end, End::Line(id) if lines.party(id).is_some_and(|x| matches!(x.role, Role::Post { .. }))));
    let live: Vec<usize> = call.as_ref().map(|c| c.cords.iter().flat_map(|&(a, b)| [a, b]).collect()).unwrap_or_default();

    // actions collected while drawing, applied afterwards
    let mut clicked_jack: Option<usize> = None;
    let mut cancel = false;
    let mut send: Option<(Option<Word>, Word)> = None;
    let mut set_rewrite: Option<Option<Word>> = None;
    let mut turn = false;
    let mut connect = false;
    let mut record = false;
    let mut dial = false;
    let mut set_forged: Option<bool> = None;

    let screen = ctx.content_rect();
    let width = (screen.width() - 24.0).min(1020.0);
    let shake = if v.shake > 0.0 { (v.age * 70.0).sin() * 5.0 * (v.shake / 0.4) } else { 0.0 };
    let frame = egui::Frame::NONE.fill(PANEL).stroke(Stroke::new(1.0, LINE2)).corner_radius(CornerRadius::same(6)).inner_margin(Margin::ZERO).outer_margin(Margin { left: shake.max(0.0) as i8, right: (-shake).max(0.0) as i8, top: 0, bottom: 0 });
    egui::Modal::new(egui::Id::new(("panel", v.panel))).frame(frame).backdrop_color(Color32::from_black_alpha(96)).show(ctx, |ui| {
        ui.set_width(width);
        ui.set_max_height(screen.height() - 30.0);
        // --- header -------------------------------------------------------------------
        egui::Frame::NONE.fill(theme::rgb(0x0e1424)).inner_margin(Margin::symmetric(12, 7)).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(title.to_uppercase()).font(retro(24.0)).color(GOLD));
                let next = match (&call, schedule) {
                    (Some(_), _) => "a call is on the line".to_string(),
                    (None, Some(_)) if off_duty => "the line is quiet now".to_string(),
                    (None, Some((_, period, at))) => {
                        let k = ((panel_t - at) / period).floor().max(-1.0) + 1.0;
                        let left = (at + k * period - panel_t).max(0.0);
                        format!("next call in {left:.0} s")
                    }
                    (None, None) => String::new(),
                };
                ui.label(RichText::new(next).font(mono(12.0)).color(DIM));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if button(ui, "ESC ✕", Kind::Ghost).clicked() {
                        out.close = true;
                    }
                    if !notes.is_empty() {
                        let label = if v.notes == 0 { "?".to_string() } else { format!("? {}/{}", v.notes, notes.len()) };
                        let costs = v.notes < notes.len() && v.notes + 1 > crate::campaign::free_hint_pages(st);
                        let tip = if costs { "Grandpa's notebook: a nudge, then a push, then the answer.\nThe next page costs this run its hint star." } else { "Grandpa's notebook: a nudge, then a push, then the answer." };
                        if button(ui, &label, Kind::Ghost).on_hover_text(tip).clicked() && v.notes < notes.len() {
                            v.notes += 1;
                            crate::campaign::hint_read(st, crate::state::MapId::Exchange, v.notes);
                            sfx.push("blip");
                        }
                    }
                    let c = if st.trace > 75.0 { RED } else if st.trace > 40.0 { AMBER } else { CYAN };
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(90.0, 7.0), Sense::hover());
                    ui.painter().rect_filled(rect, 2.0, theme::rgb(0x111830));
                    let mut fill = rect;
                    fill.set_width(rect.width() * (st.trace / 100.0).clamp(0.0, 1.0));
                    ui.painter().rect_filled(fill, 2.0, c);
                    let (word, wc) = if seen {
                        ("SEEN", RED)
                    } else if footsteps {
                        ("FOOTSTEPS", AMBER)
                    } else {
                        ("TRACE", c)
                    };
                    let blink = !(seen || footsteps) || (v.age * 4.0).fract() < 0.7;
                    ui.label(RichText::new(word).font(retro(14.0)).color(if blink { wc } else { wc.gamma_multiply(0.3) }));
                });
            });
        });
        ui.painter().hline(ui.min_rect().x_range(), ui.cursor().top(), Stroke::new(1.0, GOLD.gamma_multiply(0.4)));

        egui::Frame::NONE.inner_margin(Margin::symmetric(14, 10)).show(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 6.0);
            let left_w = (width - 28.0) * 0.56;
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(left_w);
                    // --- the jacks and the cords ------------------------------------------------
                    let n = visible.len().max(1) as f32;
                    let (field, resp) = ui.allocate_exact_size(Vec2::new(left_w, 210.0), Sense::click());
                    let pnt = ui.painter();
                    pnt.rect_filled(field, 6.0, theme::rgb(0x2a1d14));
                    pnt.rect_stroke(field, 6.0, Stroke::new(2.0, theme::rgb(0x4a3322)), egui::StrokeKind::Inside);
                    // wood grain
                    for i in 0..9 {
                        let y = field.min.y + 12.0 + i as f32 * 22.0;
                        pnt.hline(field.x_range().shrink(8.0), y, Stroke::new(1.0, theme::rgb(0x332418)));
                    }
                    let step = field.width() / n;
                    let jack_y = field.min.y + 92.0;
                    let pos_of = |j: usize| -> Option<Pos2> { visible.iter().position(|&x| x == j).map(|k| Pos2::new(field.min.x + step * (k as f32 + 0.5), jack_y)) };
                    // cords hang below the jacks between their two plugs
                    for (ci, &(a, b)) in cords.iter().enumerate() {
                        let (Some(pa), Some(pb)) = (pos_of(a), pos_of(b)) else { continue };
                        let sag = 22.0 + (pa.x - pb.x).abs() * 0.16;
                        let mid = Pos2::new((pa.x + pb.x) / 2.0, pa.y.max(pb.y) + sag);
                        let pts: Vec<Pos2> = (0..=24)
                            .map(|i| {
                                let t = i as f32 / 24.0;
                                let u = 1.0 - t;
                                Pos2::new(u * u * pa.x + 2.0 * u * t * mid.x + t * t * pb.x, u * u * pa.y + 2.0 * u * t * mid.y + t * t * pb.y)
                            })
                            .collect();
                        let c = CORD_COLORS[ci % CORD_COLORS.len()];
                        let lit = live.contains(&a) && live.contains(&b);
                        pnt.add(egui::Shape::line(pts.clone(), Stroke::new(6.0, theme::rgb(0x120c08))));
                        pnt.add(egui::Shape::line(pts, Stroke::new(if lit { 4.0 } else { 3.0 }, if lit { c } else { c.gamma_multiply(0.75) })));
                    }
                    // the jacks
                    let hover = resp.hover_pos();
                    for &j in &visible {
                        let Some(at) = pos_of(j) else { continue };
                        let (end, label) = jacks[j];
                        let usable = !matches!(end, End::Relay(_)) || relay_ok;
                        let plugged = cords.iter().any(|&(a, b)| a == j || b == j);
                        let near = hover.is_some_and(|h| h.distance(at) < 18.0);
                        let picked = v.pick == Some(j);
                        let ring = if picked {
                            GOLD
                        } else if near && usable {
                            TEXT
                        } else {
                            theme::rgb(0x8a7a66)
                        };
                        pnt.circle_filled(at, 13.0, theme::rgb(0x161009));
                        pnt.circle_stroke(at, 13.0, Stroke::new(if picked { 3.0 } else { 2.0 }, ring));
                        pnt.circle_filled(at, 6.0, if plugged { theme::rgb(0x6e6e6e) } else { theme::rgb(0x050403) });
                        // the line's lamp: lit while a call rides on it
                        let lamp = Pos2::new(at.x, at.y - 50.0);
                        let on = live.contains(&j);
                        let blink = on && (v.age * 3.0).fract() < 0.75;
                        pnt.circle_filled(lamp, 6.0, if blink { theme::rgb(0xffe08a) } else { theme::rgb(0x3a2a14) });
                        if blink {
                            pnt.circle_filled(lamp, 11.0, GOLD.gamma_multiply(0.15));
                        }
                        let col = match end {
                            End::Handset => GREEN,
                            End::Relay(_) => CYAN,
                            End::Monitor(_) => RED,
                            End::Line(_) => theme::rgb(0xe6e2c3),
                        };
                        pnt.text(Pos2::new(at.x, at.y - 20.0), Align2::CENTER_BOTTOM, label, mono(11.5), if usable { col } else { FAINT });
                        if near && resp.clicked() {
                            clicked_jack = Some(j);
                        }
                    }
                    if resp.secondary_clicked() {
                        cancel = true;
                    }
                    if let Some(pk) = v.pick
                        && let (Some(a), Some(h)) = (pos_of(pk), hover)
                    {
                        pnt.line_segment([a, h], Stroke::new(3.0, CORD_COLORS[cords.len() % CORD_COLORS.len()].gamma_multiply(0.7)));
                    }
                    let free = max_cords.saturating_sub(cords.len());
                    pnt.text(field.min + Vec2::new(10.0, 8.0), Align2::LEFT_TOP, format!("CORDS {free} free of {max_cords}"), mono(11.5), DIM);
                    if let Some((msg, t)) = &v.say
                        && *t < 2.5
                    {
                        pnt.text(Pos2::new(field.center().x, field.min.y + 26.0), Align2::CENTER_TOP, msg, mono(12.0), AMBER);
                    }

                    // --- the call on the line ----------------------------------------------------
                    if let Some(c) = &call {
                        let who = jacks[c.caller].1;
                        let far = c.far.map(|f| jacks[f].1).unwrap_or("(nothing)");
                        let via = match c.via {
                            Via::Relay(_) => " through your relay",
                            Via::Monitor => " through the monitor",
                            Via::Direct => "",
                        };
                        ui.label(RichText::new(format!("{who} → {far}{via}")).font(retro(17.0)).color(GOLD));
                        // your handset is ringing
                        let for_you = c.step == Step::Answer && c.far.is_some_and(|f| jacks[f].0 == End::Handset);
                        if for_you {
                            egui::Frame::NONE.fill(theme::rgb(0x0f2418)).stroke(Stroke::new(1.0, GREEN.gamma_multiply(0.6))).corner_radius(CornerRadius::same(4)).inner_margin(Margin::same(8)).show(ui, |ui| {
                                ui.label(RichText::new("ON YOUR HANDSET").font(retro(15.0)).color(GREEN));
                                let ask = match c.challenge {
                                    Some(ch) => format!("{who}: Orders? Challenge: {ch}."),
                                    None => format!("{who}: Orders?"),
                                };
                                ui.label(RichText::new(ask).font(mono(13.0)).color(TEXT));
                                let left = (ANSWER_MAX - c.t).max(0.0);
                                let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 4.0), Sense::hover());
                                let mut f = r;
                                f.set_width(r.width() * left / ANSWER_MAX);
                                ui.painter().rect_filled(f, 2.0, if left < 4.0 { RED } else { GREEN });
                                if c.challenge.is_some() {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(RichText::new("countersign").font(mono(11.5)).color(DIM));
                                        let mut words: Vec<Word> = heard.iter().map(|(_, b)| *b).collect();
                                        words.dedup();
                                        if words.is_empty() {
                                            ui.label(RichText::new("(you have not heard one)").font(mono(11.5)).color(FAINT));
                                        }
                                        for w in words {
                                            let sel = v.countersign == Some(w);
                                            if button(ui, w, if sel { Kind::Primary } else { Kind::Normal }).clicked() {
                                                v.countersign = Some(w);
                                                sfx.push("tick");
                                            }
                                        }
                                    });
                                }
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(RichText::new("order").font(mono(11.5)).color(DIM));
                                    for w in ORDERS {
                                        let sel = v.order == Some(w);
                                        if button(ui, w, if sel { Kind::Primary } else { Kind::Normal }).clicked() {
                                            v.order = Some(w);
                                            sfx.push("tick");
                                        }
                                    }
                                    if button_enabled(ui, v.order.is_some(), "SAY IT", Kind::Danger).clicked()
                                        && let Some(o) = v.order
                                    {
                                        send = Some((v.countersign, o));
                                    }
                                });
                            });
                        }
                        // your relay is holding the call
                        if c.step == Step::Hold
                            && let Some(k) = &c.keys
                        {
                            egui::Frame::NONE.fill(theme::rgb(0x0b2a31)).stroke(Stroke::new(1.0, CYAN.gamma_multiply(0.6))).corner_radius(CornerRadius::same(4)).inner_margin(Margin::same(8)).show(ui, |ui| {
                                ui.label(RichText::new("YOUR RELAY IS HOLDING THE CALL").font(retro(15.0)).color(CYAN));
                                let (near, far_w) = (k.word_near(), k.word_far());
                                let side = |ui: &mut egui::Ui, name: &str, key: Option<u64>, word: Option<String>| {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(format!("{name:<12}")).font(mono(12.0)).color(DIM));
                                        ui.label(RichText::new(format!("key {}", key.map(crate::lines::short).unwrap_or_else(|| "—".into()))).font(mono(12.0)).color(TEXT));
                                        if k.words > 0 {
                                            ui.label(RichText::new(word.unwrap_or_else(|| "(no word)".into())).font(retro(18.0)).color(GOLD));
                                        }
                                    });
                                };
                                side(ui, who, Some(k.k_near), Some(near.clone()));
                                side(ui, far, k.k_far, far_w.clone());
                                if k.words > 0 {
                                    let same = far_w.as_deref() == Some(near.as_str());
                                    ui.label(RichText::new(if same { "The words agree." } else { "The words differ. Both ends will read theirs out." }).font(mono(12.0)).color(if same { GREEN } else { AMBER }));
                                }
                                let left = (HOLD_MAX - c.t).max(0.0);
                                let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 4.0), Sense::hover());
                                let mut f = r;
                                f.set_width(r.width() * left / HOLD_MAX);
                                ui.painter().rect_filled(f, 2.0, if left < 5.0 { RED } else { CYAN });
                                ui.horizontal(|ui| {
                                    if button_enabled(ui, !k.sealed, "TURN THE KNOB", Kind::Normal).on_hover_text("A new number for the far side: a new key there, and a new word.").clicked() {
                                        turn = true;
                                    }
                                    if button(ui, "CONNECT", Kind::Primary).clicked() {
                                        connect = true;
                                    }
                                    if k.sealed {
                                        ui.label(RichText::new("sealed: the knob is locked").font(mono(11.5)).color(FAINT));
                                    }
                                });
                            });
                        }
                        if c.step == Step::Forge {
                            let tries = c.search.as_ref().map(|s| s.tries).unwrap_or(0);
                            ui.label(RichText::new(format!("Mara's chip has turned down {tries} LEAFs. A 16-bit checksum takes about 65,536.")).font(mono(12.5)).color(AMBER));
                        }
                    }

                    // --- what you carry --------------------------------------------------------
                    if has_relay_jacks && relay_ok && orders_line {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("RELAY PASSES ON").font(retro(14.0)).color(CYAN));
                            if button(ui, "WHAT IS SAID", if rewrite.is_none() { Kind::Primary } else { Kind::Ghost }).clicked() {
                                set_rewrite = Some(None);
                            }
                            let orders: &[Word] = if camera_line { &CAMERA_ORDERS } else { &ORDERS };
                            for w in orders {
                                if button(ui, w, if rewrite == Some(*w) { Kind::Primary } else { Kind::Ghost }).on_hover_text("Your relay replaces the order the desk gives with this one.").clicked() {
                                    set_rewrite = Some(Some(*w));
                                }
                            }
                        });
                    }
                    if recorder {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("RECORDER").font(retro(14.0)).color(VIOLET));
                            if recording {
                                ui.label(RichText::new("armed: it takes the next call").font(mono(12.0)).color(VIOLET));
                            } else if button(ui, "ARM", Kind::Normal).on_hover_text("Record the next call on this panel: both numbers of the key exchange, and the noise after.").clicked() {
                                record = true;
                            }
                        });
                    }
                    if phone {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("YOUR PHONE").font(retro(14.0)).color(GREEN));
                            if button_enabled(ui, call.is_none(), "CALL", Kind::Primary).on_hover_text("Call whatever your phone's cord reaches.").clicked() {
                                dial = true;
                            }
                            if st.flag(ESCROW_SEEN) {
                                ui.label(RichText::new("LEAF").font(mono(12.0)).color(DIM));
                                if button(ui, "HONEST", if !forged { Kind::Primary } else { Kind::Ghost }).clicked() {
                                    set_forged = Some(false);
                                }
                                if button(ui, "FORGED", if forged { Kind::Danger } else { Kind::Ghost }).on_hover_text("Feed the far chip random LEAFs until one passes its 16-bit checksum (Blaze, 1994).").clicked() {
                                    set_forged = Some(true);
                                }
                            }
                        });
                    }
                    if v.notes > 0 {
                        ui.add_space(2.0);
                        for (i, n) in notes.iter().take(v.notes).enumerate() {
                            ui.add(egui::Label::new(RichText::new(format!("{}. {n}", i + 1)).font(mono(12.0)).color(if i + 1 == v.notes { GOLD } else { DIM })).wrap());
                        }
                    }
                });
                // --- the log --------------------------------------------------------------------
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new("ON THE LINES").font(retro(15.0)).color(FAINT));
                    egui::Frame::NONE.fill(theme::rgb(0x07090f)).corner_radius(CornerRadius::same(4)).inner_margin(Margin::same(8)).show(ui, |ui| {
                        ui.set_min_height(300.0);
                        egui::ScrollArea::vertical().stick_to_bottom(true).max_height(screen.height() - 220.0).show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            let n = log.len();
                            for (i, l) in log.iter().enumerate() {
                                let fresh = i + 1 == n && v.flash < 0.6;
                                let col = tone_color(l.tone);
                                let col = if fresh { col.lerp_to_gamma(Color32::WHITE, 0.5) } else { col };
                                let mins = (l.t / 60.0) as u32;
                                let text = if l.who.is_empty() { format!("{mins:02}:{:02}  {}", l.t as u32 % 60, l.text) } else { format!("{mins:02}:{:02}  {}: {}", l.t as u32 % 60, l.who, l.text) };
                                ui.add(egui::Label::new(RichText::new(text).font(mono(12.0)).color(col)).wrap());
                            }
                            if log.is_empty() {
                                ui.label(RichText::new("Quiet. The lamps over the jacks light when a line is in use.").font(mono(12.0)).color(FAINT));
                            }
                        });
                    });
                    if !heard.is_empty() {
                        ui.label(RichText::new("HEARD").font(retro(14.0)).color(FAINT));
                        for (ch, cs) in &heard {
                            ui.label(RichText::new(format!("{ch} → {cs}")).font(mono(12.0)).color(GOLD));
                        }
                    }
                });
            });
        });
    });

    // --- carry it out ----------------------------------------------------------------------------
    let panel = v.panel;
    if cancel {
        v.pick = None;
    }
    if let Some(j) = clicked_jack {
        let p = lines.panel(panel).expect("open panel");
        let plugged = p.partner(j).is_some();
        let usable = p.usable(st, j);
        if plugged {
            lines.pull(panel, j);
            v.pick = None;
            sfx.push("plug");
        } else if !usable {
            v.say = Some(("Nothing to plug in there yet.".into(), 0.0));
        } else if let Some(a) = v.pick {
            if a == j {
                v.pick = None;
            } else if lines.plug(st, panel, a, j) {
                v.pick = None;
                sfx.push("plug");
            } else {
                v.say = Some(("That cord will not go there.".into(), 0.0));
                sfx.push("error");
                v.pick = None;
            }
        } else if p.cords.len() >= p.max_cords {
            v.say = Some(("No free cord. Pull one out first.".into(), 0.0));
            sfx.push("error");
        } else {
            v.pick = Some(j);
            sfx.push("tick");
        }
    }
    if let Some((cs, o)) = send
        && lines.answer(panel, cs, o)
    {
        sfx.push("confirm");
        v.countersign = None;
        v.order = None;
    }
    if let Some(w) = set_rewrite {
        lines.set_rewrite(panel, w);
        sfx.push("tick");
    }
    if turn && lines.turn(st, panel) {
        sfx.push("tick");
    }
    if connect && lines.connect(panel) {
        sfx.push("confirm");
    }
    if record && lines.record(panel) {
        sfx.push("confirm");
    }
    if dial && lines.dial(st, panel) {
        sfx.push("ring");
    }
    if let Some(f) = set_forged {
        lines.set_forged(panel, f);
        sfx.push("tick");
    }
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        if v.pick.is_some() {
            v.pick = None;
        } else {
            out.close = true;
        }
    }
    out
}
