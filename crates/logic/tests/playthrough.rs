//! End-to-end: play the whole game headlessly, through the real scripts,
//! dialogue, egui terminals and puzzles, and check that it ends where it
//! should. Then poke every puzzle on its own, drawing its UI before and
//! after it is solved.

use std::rc::Rc;

use cpl_logic::game::Mode;
use cpl_logic::puzzle::Host;
use cpl_logic::puzzles;
use cpl_logic::state::{GameState, MapId, SaveData};
use cpl_logic::walkthrough::{Headless, ROUTE};
use cpl_logic::world::World;

fn ui_frame(ctx: &egui::Context, t: f64, f: impl FnMut(&mut egui::Ui)) {
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 720.0))),
        time: Some(t),
        ..Default::default()
    };
    ctx.run_ui(input, f).drop_without_applying_deltas();
}

#[test]
fn full_playthrough() {
    let mut h = Headless::new("tester", "playthrough-seed-1");
    if let Err(e) = h.play() {
        panic!("playthrough stalled: {e}\nreport: {:#?}", h.report);
    }
    assert_eq!(h.game.mode, Mode::Credits, "the game should end in the credits");
    let state = h.game.ctx.state_ref();
    for id in puzzles::ALL {
        assert!(state.data.solved.contains_key(*id), "puzzle {id} was never solved; saw {:?}", h.report.puzzles);
    }
    assert_eq!(h.report.puzzles.len(), puzzles::ALL.len(), "each puzzle should open exactly once: {:?}", h.report.puzzles);
    assert_eq!(state.data.chapter, 7);
    assert!(state.flag("end.done"));
    assert!(state.has("identity"));
    assert_eq!(state.data.scene, MapId::City);
    assert!(h.report.gotos.contains(&MapId::City));
    println!("frames: {}, interactions: {}, codex: {}", h.report.frames, h.report.interactions.len(), state.data.codex.len());

    // the save survives a round trip and keeps every flag
    let json = state.save_json();
    let back: SaveData = GameState::parse_save(&json).expect("save parses");
    assert_eq!(back.flags, state.data.flags);
    assert_eq!(back.solved, state.data.solved);
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
    for s in ROUTE {
        assert!(story.contains(&s.target), "{} is not a story interaction", s.target);
        let m = cpl_logic::maps::map(s.map);
        assert!(m.interactions().contains(&s.target), "{} is not placed on {}", s.target, s.map.name());
    }
}

#[test]
fn every_puzzle_draws_and_solves() {
    let data = GameState::blank("tester", 0, Some("11".repeat(32)));
    let world = Rc::new(World::new(&data));
    let ctx = egui::Context::default();
    cpl_logic::ui::theme::install_fonts(&ctx);
    for id in puzzles::ALL {
        let mut p = puzzles::make(id, world.clone()).unwrap_or_else(|| panic!("no constructor for {id}"));
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
