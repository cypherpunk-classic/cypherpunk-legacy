//! A complete route through the story, and a headless driver that plays it.
//!
//! The route is a list of interactions, each repeated until a story flag is
//! set (talking to someone several times is how the story moves on). The
//! headless driver runs the real [`Game`] (scripts, dialogue, terminals, egui
//! and all) against a stand-in engine that answers world commands at once,
//! with every puzzle solved by its own `autoplay`. The in-game autoplay mode
//! walks the same route through the real world.

use crate::game::{EngineReq, Game, Mode};
use crate::maps::map;
use crate::script::{Cmd, Reply};
use crate::state::{GameState, MapId};

/// One leg of the route: interact with `target` until `until` is set.
#[derive(Clone, Copy, Debug)]
pub struct Step {
    pub target: &'static str,
    pub until: &'static str,
    /// The map the target is on.
    pub map: MapId,
}

const fn step(target: &'static str, until: &'static str, map: MapId) -> Step {
    Step { target, until, map }
}

pub const ROUTE: &[Step] = &[
    step("cabin.dresser", "pro.letter", MapId::Cabin),
    step("cabin.computer", "pro.done", MapId::Cabin),
    step("mara.talk", "ch1.otp", MapId::City),
    step("wharf.boathouse", "ch1.ctr", MapId::City),
    step("mara.talk", "ch1.done", MapId::City),
    step("felix.talk", "ch2.done", MapId::City),
    step("juno.talk", "ch3.shamir", MapId::City),
    step("commons.dropA", "ch3.dropA", MapId::City),
    step("commons.dropB", "ch3.dropB", MapId::City),
    step("juno.talk", "ch3.done", MapId::City),
    step("plaza.vault", "vault.opened", MapId::City),
    step("ibrahim.talk", "ch4.nonce", MapId::City),
    step("mint.archive", "ch4.rogue", MapId::City),
    step("ibrahim.talk", "ch4.done", MapId::City),
    step("lin.talk", "ch5.done", MapId::City),
    step("ada.talk", "ch6.done", MapId::City),
    step("plaza.vault", "end.done", MapId::City),
];

/// The next step of the route for the current state, if any.
pub fn next_step(state: &GameState) -> Option<Step> {
    ROUTE.iter().copied().find(|s| !state.flag(s.until))
}

/// What happened during a headless run.
#[derive(Default, Debug)]
pub struct Report {
    pub frames: u64,
    pub interactions: Vec<String>,
    pub puzzles: Vec<String>,
    pub gotos: Vec<MapId>,
    pub downloads: Vec<String>,
    pub world_cmds: Vec<&'static str>,
}

/// The real game with a stand-in engine.
pub struct Headless {
    pub game: Game,
    pub egui: egui::Context,
    pub report: Report,
    pub dt: f32,
    /// Drive dialogue with simulated key presses instead of auto mode
    /// (puzzles still solve themselves).
    pub manual: bool,
    seen_terminal: Option<String>,
    key_down: bool,
}

impl Headless {
    pub fn new(handle: &str, seed_phrase: &str) -> Self {
        let mut game = Game::new(GameState::new(), None);
        game.auto = true;
        game.new_game(handle, 0, Some(seed_phrase));
        let mut h = Self { game, egui: egui::Context::default(), report: Report::default(), dt: 1.0 / 30.0, manual: false, seen_terminal: None, key_down: false };
        h.service();
        h
    }

    /// Like [`Headless::new`], but dialogue is read and advanced with key presses.
    pub fn manual(handle: &str, seed_phrase: &str) -> Self {
        let mut h = Self::new(handle, seed_phrase);
        h.manual = true;
        h.game.auto = false;
        h
    }

    /// Simulated keyboard for manual mode: tap Space through dialogue, solve
    /// terminals with their autoplay, and leave them with Enter.
    fn manual_input(&mut self) -> Vec<egui::Event> {
        let key = |key: egui::Key, pressed: bool| egui::Event::Key { key, physical_key: None, pressed, repeat: false, modifiers: Default::default() };
        let mut events = Vec::new();
        if let Some(ap) = self.game.terminal.as_mut() {
            if !ap.host.solved() {
                ap.puzzle.autoplay(&mut ap.host);
                return events;
            }
            if !self.key_down {
                events.push(key(egui::Key::Enter, true));
                self.key_down = true;
            } else {
                events.push(key(egui::Key::Enter, false));
                self.key_down = false;
            }
            return events;
        }
        if !self.game.dialogue.is_empty() {
            if !self.key_down {
                events.push(key(egui::Key::Space, true));
                self.key_down = true;
            } else {
                events.push(key(egui::Key::Space, false));
                self.key_down = false;
            }
        }
        events
    }

    /// Answer everything the game asked of the engine.
    fn service(&mut self) {
        for _ in 0..16 {
            let reqs: Vec<EngineReq> = std::mem::take(&mut self.game.engine);
            if reqs.is_empty() {
                return;
            }
            for r in reqs {
                match r {
                    EngineReq::StartWorld => {
                        let scene = self.game.ctx.state_ref().data.scene;
                        self.game.map_ready(scene);
                    }
                    EngineReq::World { id, cmd } => {
                        self.report.world_cmds.push(cmd.name());
                        match cmd {
                            Cmd::Goto { map: m, spawn } => {
                                if let Some(s) = &spawn {
                                    assert!(map(m).spawns.contains_key(s.as_str()), "unknown spawn {s} on {}", m.name());
                                }
                                self.report.gotos.push(m);
                                self.game.map_ready(m);
                                self.game.complete(id, Reply::Done);
                            }
                            Cmd::PanTo { .. } => self.game.complete(id, Reply::Done),
                            Cmd::SetObjectFrame { id: obj, .. } => {
                                let m = self.game.current_map();
                                assert!(map(m).object(&obj).is_some(), "unknown object {obj} on {}", m.name());
                            }
                            _ => {}
                        }
                    }
                    EngineReq::Download { name, .. } => self.report.downloads.push(name),
                    EngineReq::Title | EngineReq::PersistSave | EngineReq::PersistSettings | EngineReq::Import(_) => {}
                }
            }
        }
    }

    /// One frame: update, draw (egui), service the engine.
    pub fn frame(&mut self) {
        let dt = self.dt;
        self.game.update(dt);
        self.service();
        let events = if self.manual { self.manual_input() } else { Vec::new() };
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 720.0))),
            time: Some(self.game.time),
            predicted_dt: dt,
            events,
            ..Default::default()
        };
        self.egui.begin_pass(input);
        self.game.draw(&self.egui, dt);
        self.egui.end_pass().drop_without_applying_deltas();
        self.service();
        if let Some(ap) = &self.game.terminal {
            let id = ap.puzzle.id().to_string();
            if self.seen_terminal.as_deref() != Some(&id) {
                self.report.puzzles.push(id.clone());
                self.seen_terminal = Some(id);
            }
        } else {
            self.seen_terminal = None;
        }
        self.report.frames += 1;
    }

    /// Run frames until nothing is happening (or give up).
    pub fn settle(&mut self, max_frames: u32) -> bool {
        for _ in 0..max_frames {
            self.frame();
            if self.game.idle() || self.game.mode == Mode::Credits {
                return true;
            }
        }
        false
    }

    /// Interact with something, as if the player pressed E next to it.
    pub fn interact(&mut self, id: &str) -> bool {
        // let the "window just closed" guard expire, like a player would
        for _ in 0..8 {
            if self.game.input_ready() {
                break;
            }
            self.frame();
        }
        self.report.interactions.push(id.to_string());
        let ok = self.game.interact(id);
        self.settle(20_000);
        ok
    }

    /// Play the whole route. Returns an error describing the first step that stalled.
    pub fn play(&mut self) -> Result<(), String> {
        if !self.settle(5_000) {
            return Err("the opening never settled".into());
        }
        for s in ROUTE {
            let mut tries = 0;
            while !self.game.ctx.state_ref().flag(s.until) {
                if self.game.mode == Mode::Credits {
                    break;
                }
                if self.game.current_map() != s.map {
                    return Err(format!("{} is on {}, but the game is on {}", s.target, s.map.name(), self.game.current_map().name()));
                }
                tries += 1;
                if tries > 8 {
                    return Err(format!("{} never set {} (objective: {:?})", s.target, s.until, self.game.ctx.state_ref().data.objective));
                }
                if !self.interact(s.target) {
                    return Err(format!("could not interact with {}", s.target));
                }
            }
        }
        Ok(())
    }
}
