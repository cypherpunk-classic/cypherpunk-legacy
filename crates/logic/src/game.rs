//! The game outside the engine: runs story scripts, executes their UI
//! commands, draws every overlay with egui, and tells the engine what to do
//! with the world (load a map, pan the camera, flash the screen).

use std::collections::VecDeque;

use crate::codex::codex_by_id;
use crate::items::item_def;
use crate::script::{Cmd, Ctx, Reply, Runner};
use crate::state::{GameState, MapId, SaveData, StateEvent};
use crate::story;
use crate::ui::dialogue::{self, DialogueEnv, DialogueItem};
use crate::ui::hud::{HudState, ToastKind};
use crate::ui::menu::{self, MenuAction, MenuState, Tab};
use crate::ui::portraits::Portraits;
use crate::ui::terminal::{self, ActivePuzzle};
use crate::ui::theme;
use crate::ui::title::{self, CreditsState, TitleAction, TitleState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Title,
    World,
    Credits,
}

/// Work for the engine.
pub enum EngineReq {
    /// Build the world for the current save (scene + position), then call [`Game::map_ready`].
    StartWorld,
    /// A world command; blocking ones must be answered with [`Game::complete`].
    World { id: u64, cmd: Cmd },
    /// Tear the world down and show the title backdrop.
    Title,
    /// Write the save / settings to storage.
    PersistSave,
    PersistSettings,
    /// Replace the stored save (import).
    Import(SaveData),
    /// Hand a text file to the player.
    Download { name: String, contents: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioReq {
    Sfx(&'static str),
    Music(Option<&'static str>),
}

pub struct Game {
    pub ctx: Ctx,
    pub runner: Runner,
    pub mode: Mode,
    pub dialogue: VecDeque<DialogueItem>,
    pub terminal: Option<ActivePuzzle>,
    pub menu: Option<MenuState>,
    pub hud: HudState,
    pub title: TitleState,
    pub credits: CreditsState,
    pub portraits: Portraits,
    pub engine: Vec<EngineReq>,
    pub audio: Vec<AudioReq>,
    waits: Vec<(u64, f32)>,
    /// Autoplay: dialogue advances itself, choices come from `choices` (default 0), puzzles solve themselves.
    pub auto: bool,
    pub choices: VecDeque<usize>,
    pub time: f64,
    /// The save currently in storage (offered as "Continue").
    pub stored_save: Option<SaveData>,
    pub storage_ok: bool,
    /// Autoplay pacing in seconds (0 = as fast as possible; a demo uses about a second).
    pub auto_pace: f32,
    last_modal_close: f64,
    style_scale: Option<f32>,
    auto_timer: f32,
    auto_term: f32,
}

impl Game {
    pub fn new(state: GameState, stored_save: Option<SaveData>) -> Self {
        Self {
            ctx: Ctx::new(state),
            runner: Runner::default(),
            mode: Mode::Title,
            dialogue: VecDeque::new(),
            terminal: None,
            menu: None,
            hud: HudState::default(),
            title: TitleState::default(),
            credits: CreditsState::default(),
            portraits: Portraits::default(),
            engine: Vec::new(),
            audio: vec![AudioReq::Music(Some("title"))],
            waits: Vec::new(),
            auto: false,
            choices: VecDeque::new(),
            time: 0.0,
            stored_save,
            storage_ok: true,
            auto_pace: 0.0,
            last_modal_close: -1.0,
            style_scale: None,
            auto_timer: 0.0,
            auto_term: 0.0,
        }
    }

    // --- queries --------------------------------------------------------------------

    pub fn modal_open(&self) -> bool {
        !self.dialogue.is_empty() || self.terminal.is_some() || self.menu.is_some() || self.mode != Mode::World
    }

    /// Player control is suspended (interaction running, cutscene, or a window open).
    pub fn busy(&self) -> bool {
        self.runner.blocking() || self.ctx.0.borrow().cutscene > 0 || self.modal_open()
    }

    /// The world may react to the interact key (not busy, and not the same key press that closed a window).
    pub fn input_ready(&self) -> bool {
        !self.busy() && self.time - self.last_modal_close > 0.15
    }

    /// Why the player cannot move right now (for logs and debugging).
    pub fn busy_reason(&self) -> String {
        let s = self.ctx.0.borrow();
        let tasks: Vec<&str> = self.runner.tasks.iter().map(|t| t.name.as_str()).collect();
        format!(
            "tasks={tasks:?} dialogue={} terminal={} menu={} mode={:?} cutscene={} queue={} waits={}",
            self.dialogue.len(),
            self.terminal.as_ref().map(|t| t.puzzle.id()).unwrap_or("-"),
            self.menu.is_some(),
            self.mode,
            s.cutscene,
            s.queue.len(),
            self.waits.len()
        )
    }

    /// Nothing is happening: no scripts, no windows, no pending requests.
    pub fn idle(&self) -> bool {
        self.runner.idle() && !self.modal_open() && self.ctx.0.borrow().queue.is_empty() && self.waits.is_empty() && self.ctx.0.borrow().cutscene == 0
    }

    pub fn current_map(&self) -> MapId {
        self.ctx.0.borrow().map
    }

    // --- story entry points ------------------------------------------------------------

    /// Run an interaction (from pressing E near something). Returns false if busy or unknown.
    pub fn interact(&mut self, id: &str) -> bool {
        if self.busy() {
            return false;
        }
        let fut = story::interaction(self.ctx.clone(), id);
        self.runner.spawn(id, true, fut);
        self.pump();
        true
    }

    /// The player walked into a zone.
    pub fn zone(&mut self, id: &str) {
        let fut = story::zone(self.ctx.clone(), id);
        self.runner.spawn(format!("zone:{id}"), false, fut);
        self.pump();
    }

    /// The engine finished building a map.
    pub fn map_ready(&mut self, map: MapId) {
        {
            let mut s = self.ctx.shared();
            s.map = map;
            s.state.data.scene = map;
        }
        self.mode = Mode::World;
        let m = crate::maps::map(map);
        self.audio.push(AudioReq::Music(Some(m.music)));
        let fut = story::ready(self.ctx.clone(), map);
        self.runner.spawn(format!("ready:{}", map.name()), false, fut);
        self.pump();
    }

    /// Answer a blocking world command.
    pub fn complete(&mut self, id: u64, reply: Reply) {
        self.ctx.shared().reply(id, reply);
        self.pump();
    }

    // --- lifecycle ------------------------------------------------------------------

    pub fn new_game(&mut self, handle: &str, look: usize, seed_phrase: Option<&str>) {
        self.reset_session();
        {
            let mut s = self.ctx.state();
            s.new_game(handle, look, seed_phrase);
        }
        self.engine.push(EngineReq::PersistSave);
        self.engine.push(EngineReq::StartWorld);
    }

    pub fn continue_game(&mut self) -> bool {
        let Some(d) = self.stored_save.clone() else { return false };
        self.reset_session();
        if !self.ctx.state().load(d) {
            return false;
        }
        self.engine.push(EngineReq::StartWorld);
        true
    }

    pub fn quit_to_title(&mut self) {
        self.ctx.state().save();
        self.reset_session();
        self.mode = Mode::Title;
        self.engine.push(EngineReq::Title);
        self.audio.push(AudioReq::Music(Some("title")));
    }

    fn reset_session(&mut self) {
        self.runner.clear();
        self.dialogue.clear();
        self.terminal = None;
        self.menu = None;
        self.waits.clear();
        self.hud = HudState::default();
        self.credits = CreditsState::default();
        let mut s = self.ctx.shared();
        s.queue.clear();
        s.cutscene = 0;
    }

    // --- per frame -------------------------------------------------------------------

    /// Advance timers and scripts. Call once per frame before drawing.
    pub fn update(&mut self, dt: f32) {
        self.time += dt as f64;
        self.hud.tick(dt);
        let mut done = Vec::new();
        for (id, left) in &mut self.waits {
            *left -= dt;
            if *left <= 0.0 {
                done.push(*id);
            }
        }
        self.waits.retain(|(id, _)| !done.contains(id));
        for id in done {
            self.ctx.shared().reply(id, Reply::Done);
        }
        self.pump();
        self.drain_events();
    }

    /// Poll scripts and execute the commands they send, until nothing moves.
    pub fn pump(&mut self) {
        for _ in 0..64 {
            self.runner.poll();
            let cmds: Vec<(u64, Cmd)> = self.ctx.shared().queue.drain(..).collect();
            if cmds.is_empty() {
                break;
            }
            for (id, cmd) in cmds {
                self.execute(id, cmd);
            }
        }
    }

    fn execute(&mut self, id: u64, cmd: Cmd) {
        match cmd {
            Cmd::Say(lines) => self.dialogue.push_back(DialogueItem::lines(id, lines)),
            Cmd::Choose { sp, question, options } => self.dialogue.push_back(DialogueItem::choice(id, sp, question, options)),
            Cmd::Puzzle(p) => {
                self.audio.push(AudioReq::Sfx("power"));
                let handle = self.ctx.state_ref().handle().to_string();
                self.terminal = Some(ActivePuzzle::new(id, p, &handle));
                self.auto_term = 0.0;
            }
            Cmd::Wait(s) => self.waits.push((id, s)),
            Cmd::Sfx(n) => self.audio.push(AudioReq::Sfx(n)),
            Cmd::Music(t) => self.audio.push(AudioReq::Music(t)),
            Cmd::Location { name, sub } => self.hud.show_location(&name, &sub),
            Cmd::Credits => {
                self.mode = Mode::Credits;
                self.credits = CreditsState::default();
                self.audio.push(AudioReq::Music(Some("finale")));
            }
            cmd => self.engine.push(EngineReq::World { id, cmd }),
        }
    }

    fn drain_events(&mut self) {
        let events: Vec<StateEvent> = std::mem::take(&mut self.ctx.state().events);
        for e in events {
            match e {
                StateEvent::Item { id, added: true } => {
                    let name = item_def(&id).map(|d| d.name).unwrap_or(&id).to_string();
                    self.hud.toast("KEYRING", &name, ToastKind::Item);
                    self.audio.push(AudioReq::Sfx("pickup"));
                }
                StateEvent::Codex(id) => {
                    if let Some(e) = codex_by_id(&id) {
                        self.hud.toast("CODEX", e.title, ToastKind::Codex);
                    }
                    self.audio.push(AudioReq::Sfx("pickup"));
                }
                StateEvent::Objective(t) if !t.is_empty() => self.hud.objective_flash = 1.2,
                StateEvent::Saved => {
                    self.stored_save = Some(self.ctx.state_ref().data.clone());
                    self.engine.push(EngineReq::PersistSave);
                }
                _ => {}
            }
        }
    }

    fn close_modal(&mut self) {
        self.last_modal_close = self.time;
    }

    pub fn open_menu(&mut self, tab: Tab, focus: Option<&str>) {
        if self.mode != Mode::World || self.busy() {
            return;
        }
        let mut m = MenuState::new(tab, focus, &self.ctx.state_ref());
        m.storage_ok = self.storage_ok;
        self.menu = Some(m);
        self.audio.push(AudioReq::Sfx("blip"));
    }

    /// Draw every overlay for this frame and react to what the player clicked.
    pub fn draw(&mut self, ui_ctx: &egui::Context, dt: f32) {
        let scale = self.ctx.state_ref().settings.ui_scale;
        if self.style_scale.is_none() {
            // fonts registered now are usable from the next frame on
            theme::install_fonts(ui_ctx);
            theme::apply_style(ui_ctx, scale);
            self.style_scale = Some(scale);
            ui_ctx.request_repaint();
            return;
        }
        if self.style_scale != Some(scale) {
            theme::apply_style(ui_ctx, scale);
            self.style_scale = Some(scale);
        }
        let mut sfx: Vec<&'static str> = Vec::new();
        match self.mode {
            Mode::Title => self.draw_title(ui_ctx, &mut sfx),
            Mode::Credits => {
                if title::credits(ui_ctx, &mut self.credits, dt) {
                    self.quit_to_title();
                }
            }
            Mode::World => self.draw_world_ui(ui_ctx, dt, &mut sfx),
        }
        self.hud.show_overlays(ui_ctx);
        for s in sfx {
            self.audio.push(AudioReq::Sfx(s));
        }
        self.pump();
        self.drain_events();
    }

    fn draw_title(&mut self, ui_ctx: &egui::Context, sfx: &mut Vec<&'static str>) {
        let stored = self.stored_save.clone();
        let action = {
            let state = self.ctx.state_ref();
            title::show(ui_ctx, &mut self.title, stored.as_ref(), &state, &mut self.portraits, self.time, sfx)
        };
        match action {
            TitleAction::None => {}
            TitleAction::Continue => {
                self.continue_game();
            }
            TitleAction::NewGame { handle, look } => self.new_game(&handle, look, None),
            TitleAction::Settings(s) => {
                self.ctx.state().update_settings(s);
                self.engine.push(EngineReq::PersistSettings);
            }
        }
    }

    fn draw_world_ui(&mut self, ui_ctx: &egui::Context, dt: f32, sfx: &mut Vec<&'static str>) {
        // HUD
        let hidden = self.terminal.is_some() || self.ctx.0.borrow().cutscene > 0 && self.dialogue.is_empty();
        if !hidden {
            let clicks = {
                let state = self.ctx.state_ref();
                self.hud.show(ui_ctx, &state)
            };
            if !self.busy() && self.menu.is_none() {
                let (esc, c_key) = ui_ctx.input(|i| (i.key_pressed(egui::Key::Escape), i.key_pressed(egui::Key::C)));
                if clicks.menu || (esc && self.input_ready()) {
                    self.open_menu(Tab::Keyring, None);
                } else if clicks.codex || (c_key && self.input_ready() && !ui_ctx.egui_wants_keyboard_input()) {
                    self.open_menu(Tab::Codex, None);
                }
            }
        }
        // dialogue (one item at a time, in order)
        if self.terminal.is_none() && self.menu.is_none() {
            let (handle, look, cps) = {
                let s = self.ctx.state_ref();
                (s.handle().to_string(), s.data.look, s.settings.text_speed)
            };
            let auto = self.auto;
            if let Some(item) = self.dialogue.front_mut() {
                let mut reply = dialogue::show(ui_ctx, item, DialogueEnv { handle: &handle, look, cps: if auto { 0.0 } else { cps }, dt, time: self.time, portraits: &mut self.portraits, sfx });
                if auto && reply.is_none() {
                    self.auto_timer += dt;
                    if self.auto_timer > self.auto_pace.max(0.03) {
                        self.auto_timer = 0.0;
                        reply = Some(match &item.kind {
                            dialogue::DialogueKind::Choice { options, .. } => Reply::Choice(self.choices.pop_front().unwrap_or(0).min(options.len() - 1)),
                            _ => Reply::Done,
                        });
                    }
                }
                if let Some(r) = reply {
                    let id = item.reply;
                    self.dialogue.pop_front();
                    self.ctx.shared().reply(id, r);
                    if self.dialogue.is_empty() {
                        self.close_modal();
                    }
                }
            }
        }
        // terminal
        if let Some(ap) = self.terminal.as_mut() {
            let mut result = terminal::show(ui_ctx, ap, dt);
            if self.auto && result.is_none() {
                self.auto_term += dt;
                if !ap.host.solved() {
                    if self.auto_term >= self.auto_pace {
                        ap.puzzle.autoplay(&mut ap.host);
                        self.auto_term = 0.0;
                    }
                } else if self.auto_term >= self.auto_pace * 1.5 {
                    result = Some(true);
                }
            }
            sfx.append(&mut ap.host.sfx);
            for (name, contents) in ap.host.downloads.drain(..) {
                self.engine.push(EngineReq::Download { name, contents });
            }
            if let Some(ok) = result {
                let ap = self.terminal.take().unwrap();
                if ok {
                    self.ctx.state().mark_solved(ap.puzzle.id(), ap.host.attempts + 1);
                }
                self.ctx.shared().reply(ap.reply, Reply::Bool(ok));
                self.close_modal();
            }
        }
        // menu
        if let Some(m) = self.menu.as_mut() {
            let action = {
                let state = self.ctx.state_ref();
                menu::show(ui_ctx, m, &state, sfx)
            };
            match action {
                MenuAction::None => {}
                MenuAction::Close => {
                    self.menu = None;
                    self.close_modal();
                }
                MenuAction::SaveNow => {
                    self.ctx.state().save();
                    self.engine.push(EngineReq::World { id: 0, cmd: Cmd::SavePosition });
                    self.hud.toast("SAVED", "Progress written to storage.", ToastKind::Info);
                }
                MenuAction::CopySave => {
                    let json = self.ctx.state_ref().save_json();
                    ui_ctx.copy_text(json);
                    self.hud.toast("EXPORT", "Save copied to clipboard.", ToastKind::Info);
                }
                MenuAction::Import(text) => match GameState::parse_save(&text) {
                    Some(d) => {
                        self.stored_save = Some(d.clone());
                        self.engine.push(EngineReq::Import(d));
                        self.hud.toast("IMPORTED", "Save loaded. Returning to title.", ToastKind::Info);
                        self.menu = None;
                        self.reset_session();
                        self.mode = Mode::Title;
                        self.engine.push(EngineReq::Title);
                    }
                    None => self.hud.toast("IMPORT", "That does not look like a save file.", ToastKind::Warn),
                },
                MenuAction::QuitToTitle => {
                    self.menu = None;
                    self.engine.push(EngineReq::World { id: 0, cmd: Cmd::SavePosition });
                    self.quit_to_title();
                }
                MenuAction::Settings(s) => {
                    self.ctx.state().update_settings(s);
                    self.engine.push(EngineReq::PersistSettings);
                }
            }
        }
    }
}
