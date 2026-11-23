//! End-to-end: play the whole game headlessly, through the real scripts,
//! dialogue, egui terminals and puzzles, and check that it ends where it
//! should. Then poke every puzzle on its own, drawing its UI before and
//! after it is solved.

use cpl_logic::game::Mode;
use cpl_logic::puzzle::Host;
use cpl_logic::puzzles;
use cpl_logic::ending::{self, Ending};
use cpl_logic::state::{FlagValue, GameState, MapId, SaveData};
use cpl_logic::walkthrough::{Headless, ROUTE};

fn ui_frame(ctx: &egui::Context, t: f64, f: impl FnMut(&mut egui::Ui)) {
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 720.0))),
        time: Some(t),
        ..Default::default()
    };
    ctx.run_ui(input, f).drop_without_applying_deltas();
}

/// The puzzles on the main route (everything but the side quests).
fn main_puzzles() -> Vec<&'static str> {
    puzzles::ALL.iter().copied().filter(|id| !id.starts_with("q.")).collect()
}

#[test]
fn full_playthrough() {
    let mut h = Headless::new("tester", "playthrough-seed-1");
    if let Err(e) = h.play() {
        panic!("playthrough stalled: {e}\nreport: {:#?}", h.report);
    }
    assert_eq!(h.game.mode, Mode::Credits, "the game should end in the credits");
    let state = h.game.ctx.state_ref();
    let main = main_puzzles();
    for id in &main {
        assert!(state.data.solved.contains_key(*id), "puzzle {id} was never solved; saw {:?}", h.report.puzzles);
    }
    assert_eq!(h.report.puzzles.len(), main.len(), "each puzzle should open exactly once: {:?}", h.report.puzzles);
    assert_eq!(state.data.chapter, 7);
    assert!(state.flag("end.done"));
    assert!(state.has("identity"));
    assert_eq!(state.data.scene, MapId::City);
    assert!(h.report.gotos.contains(&MapId::City));
    // chapters 1 and 2 end in their v4 districts: the harbour and the Exchange's street
    for (m, done) in [(MapId::Wharf, cpl_logic::signals::wharf::END), (MapId::Exchange, cpl_logic::lines::exchange::END)] {
        assert!(h.report.gotos.contains(&m), "never went to {}", m.name());
        assert!(state.flag(done), "{} never finished", m.name());
    }
    let order: Vec<MapId> = h.report.gotos.iter().copied().filter(|m| matches!(m, MapId::Wharf | MapId::Exchange)).collect();
    assert_eq!(order.first(), Some(&MapId::Wharf), "the harbour comes first: {order:?}");
    for id in cpl_logic::codex::UNLOCKS.iter().flat_map(|(_, ids)| ids.iter()) {
        assert!(state.data.codex.iter().any(|c| c == id), "the districts never unlocked {id}");
    }
    // the main story alone, with the principled answers, just reaches the best ending
    assert_eq!(ending::ending(&state), Ending::OpenSociety, "tally: {:?}", ending::tally(&state));
    assert_eq!(state.get("end.kind"), Some(&FlagValue::Str("open".into())));
    assert!(state.data.allies.iter().all(|a| state.cert_ok(a)), "every certification is a real signature");
    assert!(h.game.credits.ending.is_some(), "the credits show the ending");
    println!("frames: {}, interactions: {}, codex: {}, allies: {}", h.report.frames, h.report.interactions.len(), state.data.codex.len(), state.ally_count());

    // the save survives a round trip and keeps every flag
    let json = state.save_json();
    let back: SaveData = GameState::parse_save(&json).expect("save parses");
    assert_eq!(back.flags, state.data.flags);
    assert_eq!(back.solved, state.data.solved);
    assert_eq!(back.allies, state.data.allies);
    assert_eq!(back.wallet.len(), state.data.wallet.len());
}

#[test]
fn side_quests_floppies_and_shops() {
    let mut h = Headless::new("completionist", "playthrough-seed-3");
    h.side = true;
    if let Err(e) = h.play() {
        panic!("side-quest playthrough stalled: {e}\nreport: {:#?}", h.report);
    }
    assert_eq!(h.game.mode, Mode::Credits);
    let state = h.game.ctx.state_ref();
    for id in puzzles::ALL {
        assert!(state.data.solved.contains_key(*id), "puzzle {id} was never solved; saw {:?}", h.report.puzzles);
    }
    assert_eq!(cpl_logic::quests::floppies_found(&state), cpl_logic::quests::FLOPPIES);
    assert!(state.flag("floppy.all"));
    for a in ["rosa", "theo", "wen", "c1", "c3", "c4", "c5"] {
        assert!(state.is_ally(a), "{a} should have joined");
    }
    for item in ["faraday", "mesh-radio"] {
        assert!(state.has(item), "bought {item}");
    }
    assert!(state.data.stats.payments >= 4, "paid in eCash: {:?}", state.data.stats);
    assert_eq!(ending::ending(&state), Ending::OpenSociety);
    // everything in the codex can be found
    let missing: Vec<&str> = cpl_logic::codex::CODEX.iter().map(|e| e.id).filter(|id| !state.data.codex.iter().any(|c| c == id)).collect();
    assert!(missing.is_empty(), "codex entries never unlocked: {missing:?}");
    // the mint's spent list holds every token you ever paid with, and none of them twice
    let spent = &state.data.mint_spent;
    let mut uniq = spent.clone();
    uniq.sort();
    uniq.dedup();
    assert_eq!(uniq.len(), spent.len());
    println!("completionist: {} frames, {} allies, {} cr left, {} payments", h.report.frames, state.ally_count(), state.credits(), state.data.stats.payments);
}

#[test]
fn choices_change_the_ending() {
    // the choices come in story order: read the co-op's mail, take the mint's
    // licence, keep Warden's key, keep a remailer table, give up the address
    // when the court order comes
    let mut h = Headless::new("cynic", "playthrough-seed-4");
    h.game.choices = [1, 1, 2, 1, 1].into_iter().collect();
    if let Err(e) = h.play() {
        panic!("cynical playthrough stalled: {e}\nreport: {:#?}", h.report);
    }
    {
        let state = h.game.ctx.state_ref();
        for f in ["choice.snoop", "choice.mintlog", "choice.logs", "choice.gaveup", "mintlog.seized"] {
            assert!(state.flag(f), "{f} should be set");
        }
        assert_eq!(ending::key_choice(&state), "keep");
        assert!(!state.is_ally("c2"), "Ines does not sign for someone who read her mail");
        assert_eq!(ending::ending(&state), Ending::LongWinter, "tally: {:?}", ending::tally(&state));
        assert!(state.flag("end.done"));
    }

    // somewhere in between: publish the key, take the licence
    let mut h = Headless::new("pragmatist", "playthrough-seed-5");
    h.game.choices = [0, 1, 1, 0].into_iter().collect();
    if let Err(e) = h.play() {
        panic!("pragmatic playthrough stalled: {e}\nreport: {:#?}", h.report);
    }
    let state = h.game.ctx.state_ref();
    assert_eq!(ending::key_choice(&state), "publish");
    assert_eq!(ending::ending(&state), Ending::QuietVictory, "tally: {:?}", ending::tally(&state));
}

#[test]
fn the_dragnet_flags_you_and_a_safe_house_takes_you_in() {
    let mut h = Headless::new("loiterer", "playthrough-seed-6");
    h.play_until(Some("ch1.done")).expect("chapter 1 plays");
    assert!(h.game.ctx.state_ref().flag("ch1.done"));
    // stand under the Wharf mast with nothing to hide behind
    let city = cpl_logic::maps::map(MapId::City);
    let mast = city.sensors.iter().find(|s| s.id == "mast-wharf").unwrap().clone();
    let mut frames = 0;
    loop {
        let tick = h.game.ctx.state().dragnet(&city, mast.x, mast.y, 0.0, 1.0 / 30.0);
        frames += 1;
        if tick.flagged {
            break;
        }
        assert!(frames < 30 * 30, "a mast should flag a loiterer within half a minute");
    }
    let (credits, ious) = {
        let s = h.game.ctx.state_ref();
        (s.credits(), s.data.ious)
    };
    h.game.flagged(mast.x, mast.y);
    assert!(h.settle(20_000));
    let s = h.game.ctx.state_ref();
    assert_eq!(s.data.stats.flagged, 1);
    assert_eq!(s.trace, 0.0, "the trace starts over");
    assert!(s.data.codex.iter().any(|c| c == "metadata"));
    let to = h.report.spawns.last().expect("sent somewhere");
    assert!(to.starts_with("safe-"), "sent to {to}");
    assert_eq!(s.credits() + s.data.ious, (credits + ious).saturating_sub(2), "a fine of two credits");
    drop(s);
    // the story carries on from the safe house
    h.play().expect("the rest of the route");
    assert_eq!(h.game.mode, Mode::Credits);
}

#[test]
fn playthrough_with_keyboard() {
    // dialogue typed out and advanced with Space, choices taken with Space,
    // terminals left with Enter: the same input paths a player uses
    let mut h = Headless::manual("keys", "playthrough-seed-2");
    if let Err(e) = h.play() {
        panic!("keyboard playthrough stalled: {e}\nreport: {:#?}", h.report);
    }
    assert_eq!(h.game.mode, Mode::Credits);
    assert!(h.game.ctx.state_ref().flag("end.done"));
    println!("keyboard playthrough: {} frames", h.report.frames);
}

#[test]
fn route_targets_exist() {
    let story: Vec<&str> = cpl_logic::story::INTERACTIONS.to_vec();
    for s in ROUTE.iter().filter(|s| s.target != cpl_logic::walkthrough::DISTRICT) {
        assert!(story.contains(&s.target), "{} is not a story interaction", s.target);
        let m = cpl_logic::maps::map(s.map);
        assert!(m.interactions().contains(&s.target), "{} is not placed on {}", s.target, s.map.name());
    }
}

#[test]
fn every_puzzle_draws_and_solves() {
    let mut state = GameState::new();
    state.new_game("tester", 0, Some("puzzle gallery"));
    for a in ["mara", "felix", "juno"] {
        state.recruit(a);
    }
    let script = cpl_logic::script::Ctx::new(state);
    let ctx = egui::Context::default();
    cpl_logic::ui::theme::install_fonts(&ctx);
    for id in puzzles::ALL {
        let mut p = puzzles::make(id, &script).unwrap_or_else(|| panic!("no constructor for {id}"));
        assert_eq!(p.id(), *id);
        let mut host = Host::new("tester");
        p.open(&mut host);
        for i in 0..3 {
            host.time = i as f64 * 0.5;
            ui_frame(&ctx, host.time, |ui| p.ui(ui, &mut host));
        }
        assert!(!host.solved(), "{id} solved itself before anyone touched it");
        p.autoplay(&mut host);
        assert!(host.solved(), "{id}: autoplay did not solve it (status: {})", host.status);
        for i in 3..6 {
            host.time = i as f64 * 0.5;
            ui_frame(&ctx, host.time, |ui| p.ui(ui, &mut host));
        }
        assert!(host.solved());
        // a deep dive, if there is one, can be completed
        if let Some(mut sheet) = p.deep_sheet() {
            assert!(!sheet.cells.is_empty() && !sheet.done(), "{id}: deep dive starts empty");
            sheet.fill();
            assert!(sheet.done(), "{id}: deep dive answers do not check out");
        }
    }
}

/// Every menu tab and title page draws without panicking (narrow and wide
/// screens, first frames included), and Esc / C open the menu in the world.
#[test]
fn menus_and_title_pages_draw() {
    use cpl_logic::ui::menu::Tab;
    use cpl_logic::ui::title::TitlePage;
    let key = |key: egui::Key| egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() };
    for size in [egui::vec2(1280.0, 720.0), egui::vec2(380.0, 640.0)] {
        let mut h = Headless::new("menus", "menu-seed");
        h.settle(2000);
        let frame = |h: &mut Headless, events: Vec<egui::Event>| {
            h.game.update(1.0 / 30.0);
            let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)), events, ..Default::default() };
            h.egui.begin_pass(input);
            h.game.draw(&h.egui, 1.0 / 30.0);
            h.egui.end_pass().drop_without_applying_deltas();
        };
        for _ in 0..12 {
            frame(&mut h, vec![]);
        }
        assert!(h.game.menu.is_none());
        frame(&mut h, vec![key(egui::Key::Escape)]);
        assert!(h.game.menu.is_some(), "Esc should open the menu");
        for tab in [Tab::Codex, Tab::Keyring, Tab::Settings, Tab::Game] {
            h.game.menu.as_mut().unwrap().tab = tab;
            for _ in 0..3 {
                frame(&mut h, vec![]);
            }
        }
        frame(&mut h, vec![key(egui::Key::Escape)]);
        for _ in 0..8 {
            frame(&mut h, vec![]);
        }
        assert!(h.game.menu.is_none(), "Esc should close the menu");
        frame(&mut h, vec![key(egui::Key::C)]);
        assert!(h.game.menu.as_ref().is_some_and(|m| m.tab == Tab::Codex), "C should open the codex");
        // the title screen and its pages
        h.game.menu = None;
        h.game.quit_to_title();
        for page in [TitlePage::Main, TitlePage::NewGame, TitlePage::Settings, TitlePage::About] {
            h.game.title.page = page;
            for _ in 0..3 {
                frame(&mut h, vec![]);
            }
        }
    }
}
