//! The game outside the engine: runs story scripts, executes their UI
//! commands, draws every overlay with egui, and tells the engine what to do
//! with the world (load a map, pan the camera, flash the screen).

use std::collections::VecDeque;

use crate::codex::codex_by_id;
use crate::district::{AutoAction, DiveView, District};
use crate::lines::LineEvent;
use crate::lines::exchange::{self as xch, XMove};
use crate::signals::wharf::{MAST, Move};
use crate::signals::{Event as SigEvent, Outcome};
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

/// The static id of a dive named in a script.
fn dive_id(name: &str) -> Option<&'static str> {
    match name {
        "interceptor" => Some("interceptor"),
        "static" => Some("static"),
        "logjam" => Some("logjam"),
        _ => None,
    }
}

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
    Import(Box<SaveData>),
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
    /// The v4 district the player is in (the harbour in chapter 1, the
    /// Exchange's street in chapter 2), while they are in it.
    pub district: Option<District>,
    /// Go straight to chapter 1's harbour on the first frame (`CPL_WHARF=1`, `?wharf`).
    pub launch_wharf: bool,
    /// Or to chapter 2's street (`CPL_EXCHANGE=1`, `?exchange`).
    pub launch_exchange: bool,
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
            district: None,
            launch_wharf: false,
            launch_exchange: false,
        }
    }

    // --- queries --------------------------------------------------------------------

    pub fn modal_open(&self) -> bool {
        !self.dialogue.is_empty() || self.terminal.is_some() || self.menu.is_some() || self.mode != Mode::World || self.district.as_ref().is_some_and(|d| d.modal())
    }

    /// Only a signal board's close-up is open: the street outside keeps watching you.
    pub fn at_board_only(&self) -> bool {
        self.dialogue.is_empty() && self.terminal.is_none() && self.menu.is_none() && self.mode == Mode::World && self.district.as_ref().is_some_and(|d| d.at_board())
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

    /// Open one puzzle's terminal on its own (development previews and
    /// screenshots: `CPL_PUZZLE=ch4.nonce`). Returns false if busy or unknown.
    pub fn preview_puzzle(&mut self, id: &str) -> bool {
        if self.busy() {
            return false;
        }
        let c = self.ctx.clone();
        let Some(p) = crate::puzzles::make(id, &c) else { return false };
        self.runner.spawn(format!("preview:{id}"), true, Box::pin(async move {
            c.puzzle(p).await;
        }));
        self.pump();
        true
    }

    /// Close the open terminal as if the player left it (previews).
    pub fn close_terminal(&mut self) {
        if let Some(ap) = self.terminal.take() {
            let ok = ap.host.solved();
            self.ctx.shared().reply(ap.reply, Reply::Bool(ok));
            self.close_modal();
            self.pump();
        }
    }

    /// The Dragnet's trace reached 100 with the player at (x, y).
    pub fn flagged(&mut self, x: f32, y: f32) {
        if self.at_board_only() {
            self.close_board();
            self.pump();
        }
        if self.busy() {
            return;
        }
        let fut = story::flagged(self.ctx.clone(), x, y);
        self.runner.spawn("flagged", true, fut);
        self.pump();
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
        if matches!(map, MapId::Wharf | MapId::Exchange) {
            if self.district.as_ref().is_none_or(|d| d.which != map) {
                let d = District::new(&mut self.ctx.state(), map);
                self.district = Some(d);
            }
        } else {
            self.district = None;
        }
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

    /// A new game that skips ahead to chapter 1's harbour (the Wharf
    /// district), with the prologue and the first half of the chapter done
    /// (development, the tests, `CPL_WHARF=1`).
    pub fn start_wharf(&mut self, seed_phrase: Option<&str>) {
        self.skip_to(seed_phrase, MapId::Wharf);
    }

    /// The same, skipping ahead to chapter 2's street (the Exchange
    /// district), the harbour behind you (`CPL_EXCHANGE=1`).
    pub fn start_exchange(&mut self, seed_phrase: Option<&str>) {
        self.skip_to(seed_phrase, MapId::Exchange);
    }

    fn skip_to(&mut self, seed_phrase: Option<&str>, district: MapId) {
        self.reset_session();
        let (handle, look) = self.stored_save.as_ref().map(|s| (s.handle.clone(), s.look)).unwrap_or_else(|| ("nightjar".to_string(), 0));
        {
            let mut s = self.ctx.state();
            s.new_game(&handle, look, seed_phrase);
            let mut flags = vec!["pro.intro", "pro.letter", "pro.note", "pro.booted", "pro.seed", "pro.login", "pro.done", "pro.toCity", "pro.metCat", "city.arrived", "warden.on", "ch1.met", "ch1.otp", "ch1.ctr", crate::story::ch1::HARBOUR_SENT];
            let mut items = vec!["letter", "descriptor", "gateway-key", "share1"];
            let mut chapter = 1;
            if district == MapId::Exchange {
                flags.extend([MAST, crate::signals::wharf::END, "w.intro", "w.started", "ch1.done", "ch2.met", "ch2.dh", "ch2.mitm", "ch2.logjam", "ch2.bookIntro", "ch2.book", "book.out", crate::story::ch2::STREET_SENT]);
                items.push("sealed-messenger");
                chapter = 2;
            }
            for f in flags {
                s.set(f, true);
            }
            for i in items {
                s.give(i);
            }
            s.set_chapter(chapter);
            s.data.scene = district;
            s.save();
        }
        self.engine.push(EngineReq::PersistSave);
        self.engine.push(EngineReq::StartWorld);
    }

    /// Walk (well, fade) to a spawn in the district.
    pub fn travel(&mut self, spawn: &'static str) {
        let c = self.ctx.clone();
        let map = self.district.as_ref().map(|d| d.which).unwrap_or(MapId::City);
        self.runner.spawn(format!("travel:{spawn}"), true, Box::pin(async move { c.goto(map, Some(spawn)).await }));
        self.pump();
    }

    /// Step away from the open signal board or patch panel.
    pub fn close_board(&mut self) {
        if let Some(d) = self.district.as_mut() {
            let reply = d.board.take().map(|v| v.reply).or_else(|| d.panel.take().map(|v| v.reply));
            if let Some(r) = reply {
                self.ctx.shared().reply(r, Reply::Done);
                self.close_modal();
            }
        }
    }

    fn close_locker(&mut self) {
        if let Some(d) = self.district.as_mut()
            && let Some(v) = d.locker.take()
        {
            self.ctx.shared().reply(v.reply, Reply::Done);
            self.close_modal();
        }
    }

    fn close_dive(&mut self, cleared: bool) {
        if let Some(d) = self.district.as_mut()
            && let Some(v) = d.dive.take()
        {
            if cleared {
                let flag = d.dives.get(v.id).map(|x| x.done_flag());
                if let Some(f) = flag {
                    let mut st = self.ctx.state();
                    if !st.flag(f) {
                        st.set(f, true);
                    }
                }
                d.shots.push(format!("dive-{}", v.id));
            }
            self.ctx.shared().reply(v.reply, Reply::Bool(cleared));
            self.close_modal();
            self.audio.push(AudioReq::Music(Some("city")));
        }
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
        self.district = None;
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
        if self.launch_wharf && self.mode == Mode::Title {
            self.launch_wharf = false;
            self.start_wharf(None);
        }
        if self.launch_exchange && self.mode == Mode::Title {
            self.launch_exchange = false;
            self.start_exchange(None);
        }
        self.update_district(dt);
    }

    /// The district's clock, its dives, what its signals and lines did, the
    /// codex entries they earned, and the autoplayer.
    fn update_district(&mut self, dt: f32) {
        let Some(d) = self.district.as_mut() else { return };
        if self.mode != Mode::World {
            return;
        }
        let seed = self.ctx.state_ref().data.seed.clone();
        {
            let mut st = self.ctx.state();
            d.tick(&mut st, dt);
        }
        // a dive runs on the game clock: the player's input, or its own bot
        if let Some(v) = d.dive.as_mut() {
            for at in [4.0, 20.0, 45.0] {
                if v.age < at && v.age + dt >= at {
                    d.shots.push(format!("dive-{}-{}s", v.id, at as u32));
                }
            }
            v.age += dt;
            v.shake = (v.shake - dt).max(0.0);
            let id = v.id;
            let input = std::mem::take(&mut d.dive_input);
            let auto = self.auto;
            let mut closing = None;
            if let Some(dv) = d.dives.get_mut(id) {
                let input = if auto { dv.bot(dt) } else { input };
                let mut fx = crate::dive::Fx::default();
                dv.update(dt, &input, &mut fx);
                for s in fx.sfx {
                    self.audio.push(AudioReq::Sfx(s));
                }
                if fx.shake > 0.0
                    && let Some(v) = d.dive.as_mut()
                {
                    v.shake = v.shake.max(fx.shake);
                }
                if dv.status() == crate::dive::Status::Cleared
                    && let Some(v) = d.dive.as_mut()
                {
                    let t = v.cleared_for.get_or_insert(0.0);
                    *t += dt;
                    if *t > 2.6 {
                        closing = Some(true);
                    }
                }
            } else {
                let st = self.ctx.state_ref();
                let _ = d.dive_mut(id, seed.as_bytes(), &st);
            }
            if let Some(c) = closing {
                self.close_dive(c);
            }
        }
        // what the signals did
        let Some(d) = self.district.as_mut() else { return };
        let events: Vec<SigEvent> = std::mem::take(&mut d.sig.events);
        let open_board = d.board.as_ref().map(|v| v.board);
        for e in events {
            match e {
                SigEvent::Solved { board, flag } => {
                    d.shots.push(flag.trim_start_matches("w.").replace('.', "-"));
                    let label = d.sig.board(board).map(|b| b.title).unwrap_or("");
                    let (title, body) = match flag {
                        f if f.starts_with("w.gate.") => ("GATE OPEN", label.to_string()),
                        crate::signals::wharf::GUARD_WEST => ("NEW ORDERS", "The guard turns and heads west.".to_string()),
                        crate::signals::wharf::CAM_OFF => ("CAMERA", "Asleep.".to_string()),
                        crate::signals::wharf::C1 => ("SIGN LOCKER", "Open. And the ice house with it.".to_string()),
                        crate::signals::wharf::PAD_MAST => ("A PAD", "This card is the mast console's pad.".to_string()),
                        crate::signals::wharf::PAD_MARKET => ("A PAD", "This card is the market console's pad.".to_string()),
                        MAST => ("THE MAST", "It answers to you now.".to_string()),
                        _ => ("IN CLEAR", label.to_string()),
                    };
                    self.hud.toast(title, &body, ToastKind::Item);
                    if flag.starts_with("w.gate.") {
                        self.audio.push(AudioReq::Sfx("door"));
                        let def = self.ctx.shared().map_def();
                        let want = format!("!{flag}");
                        for o in def.objects.iter().filter(|o| o.show_if == Some(want.as_str())) {
                            self.engine.push(EngineReq::World { id: 0, cmd: Cmd::Burst { x: o.x, y: o.y - 8.0, color: 0x5dff9a, n: 18 } });
                        }
                    } else if open_board != Some(board) {
                        self.audio.push(AudioReq::Sfx("success"));
                    }
                    if flag == MAST {
                        let c = self.ctx.clone();
                        self.runner.spawn("mast", false, Box::pin(crate::story::wharf::mast_freed(c)));
                    }
                }
                SigEvent::Revealed { .. } => self.audio.push(AudioReq::Sfx("success")),
                SigEvent::Alarm { .. } => self.engine.push(EngineReq::World { id: 0, cmd: Cmd::Flash { ms: 260.0, color: 0xff3355 } }),
                SigEvent::SealBroken { .. } => self.engine.push(EngineReq::World { id: 0, cmd: Cmd::Flash { ms: 200.0, color: 0x73eff7 } }),
                SigEvent::Buzz { .. } => {}
                SigEvent::Card { id } => {
                    let c = self.ctx.state_ref().data.cards.iter().find(|c| c.id == id).cloned();
                    if let Some(c) = c {
                        let body = if c.note.is_empty() { c.name.clone() } else { format!("{} · {}", c.name, c.note) };
                        self.hud.toast("CARD", &body, ToastKind::Codex);
                    }
                }
                SigEvent::Learned { glyph } => self.hud.toast("GLYPH BOOK", glyph.label(), ToastKind::Info),
                SigEvent::Locker { id, open } => {
                    if open {
                        d.shots.push(format!("locker-{id}"));
                    }
                }
                SigEvent::Broadcast { board } => {
                    if open_board == Some(board) {
                        self.audio.push(AudioReq::Sfx("repaint"));
                    }
                }
            }
        }
        self.line_events();
        crate::codex::unlock_due(&mut self.ctx.state());
        self.pump();
        if self.auto {
            self.wharf_auto(dt);
        }
    }

    /// What the Exchange's lines did: rings in the street, toasts, sounds.
    fn line_events(&mut self) {
        let Some(d) = self.district.as_mut() else { return };
        let Some(lines) = d.lines.as_mut() else { return };
        let events: Vec<LineEvent> = std::mem::take(&mut lines.events);
        let open = d.panel.as_ref().map(|v| v.panel);
        for e in events {
            match e {
                LineEvent::Ring { panel, party } => {
                    let def = self.ctx.shared().map_def();
                    if let Some(o) = def.object(&format!("x-box-{party}")) {
                        self.engine.push(EngineReq::World { id: 0, cmd: Cmd::Burst { x: o.x, y: o.y - 18.0, color: 0xffcd75, n: 8 } });
                    }
                    if open == Some(panel) {
                        self.audio.push(AudioReq::Sfx("ring"));
                    }
                }
                LineEvent::Plug { .. } => {}
                LineEvent::Solved { panel, flag } => {
                    d.shots.push(flag.trim_start_matches("x.").replace('.', "-"));
                    let (title, body) = match flag {
                        xch::GATE_SQUARE => ("GATE OPEN", "The square gate's latch holds."),
                        xch::POST2 => ("NEW ORDERS", "Post two walks off west."),
                        xch::POST3 => ("NEW ORDERS", "Post three goes in through the back door."),
                        xch::CAM_OFF => ("CAMERA", "Asleep. The tower gate opens with it."),
                        xch::FREE => ("THE EXCHANGE", "A call nobody else can hear."),
                        _ => ("DONE", ""),
                    };
                    self.hud.toast(title, body, ToastKind::Item);
                    // the HUD's one line says where things stand, never what to press
                    let next = match flag {
                        xch::GATE_SQUARE => Some("The square gate is open. In call box row a guard takes his orders by phone."),
                        xch::POST2 => Some("Post two has gone west, and his boom is up. Felix is waiting in the yard behind the Exchange."),
                        xch::POST3 => Some("Post three has gone inside. In camera street the camera's line reads a word off its key."),
                        xch::CAM_OFF => Some("The camera sleeps and the tower yard is open. The desk calls the tower there, sealed and export grade."),
                        _ => None,
                    };
                    if let Some(t) = next {
                        self.ctx.state().set_objective(t);
                    }
                    if matches!(flag, xch::GATE_SQUARE | xch::POST2 | xch::POST3 | xch::CAM_OFF) {
                        self.audio.push(AudioReq::Sfx("door"));
                        let def = self.ctx.shared().map_def();
                        let want = format!("!{flag}");
                        for o in def.objects.iter().filter(|o| o.show_if == Some(want.as_str()) && o.tex.starts_with("obj-barrier")) {
                            self.engine.push(EngineReq::World { id: 0, cmd: Cmd::Burst { x: o.x, y: o.y - 8.0, color: 0x5dff9a, n: 18 } });
                        }
                    } else if open != Some(panel) {
                        self.audio.push(AudioReq::Sfx("success"));
                    }
                    if flag == xch::FREE {
                        let c = self.ctx.clone();
                        self.runner.spawn("free", false, Box::pin(crate::story::exchange::freed(c)));
                    }
                }
                LineEvent::Alarm { .. } | LineEvent::Tamper { .. } => {
                    self.engine.push(EngineReq::World { id: 0, cmd: Cmd::Flash { ms: 260.0, color: 0xff3355 } });
                    self.audio.push(AudioReq::Sfx("alarm"));
                }
                LineEvent::NoCarrier { panel } => {
                    if open == Some(panel) {
                        self.audio.push(AudioReq::Sfx("error"));
                    }
                }
                LineEvent::Heard { .. } => {
                    self.audio.push(AudioReq::Sfx("blip"));
                }
                LineEvent::Taped { .. } => {
                    d.shots.push("tape".into());
                    self.hud.toast("TAPE", "Both numbers of the tower's key exchange, and the noise after.", ToastKind::Codex);
                    self.ctx.state().set_objective("You have the tower line on tape. The cabinet in the tower yard, LOGJAM, can take its numbers apart.");
                    self.audio.push(AudioReq::Sfx("pickup"));
                }
                LineEvent::TapeRead => {
                    let code: String = xch::code(&self.ctx.state_ref()).iter().map(|d| d.to_string()).collect();
                    self.hud.toast("THE TAPE PLAYS", &format!("TOWER, MAINTENANCE {code}"), ToastKind::Item);
                    self.ctx.state().set_objective(&format!("The tape says TOWER, MAINTENANCE {code}. The tower door is beside the tower."));
                    self.audio.push(AudioReq::Sfx("success"));
                }
                LineEvent::Escrow { .. } => {
                    self.engine.push(EngineReq::World { id: 0, cmd: Cmd::Flash { ms: 220.0, color: 0xff3355 } });
                    self.ctx.state().set_objective("The escrow desk reads your key out of every call's LEAF. The receiving phone only checks sixteen bits of it.");
                }
                LineEvent::Forged { .. } => {}
            }
        }
    }

    /// A player who knows every answer, one move at a time.
    fn wharf_auto(&mut self, dt: f32) {
        if !self.dialogue.is_empty() || self.terminal.is_some() || self.menu.is_some() {
            return;
        }
        let pace = (self.auto_pace * 0.4).max(0.0);
        let action = {
            let Some(d) = self.district.as_mut() else { return };
            d.auto.wait -= dt;
            if d.auto.wait > 0.0 {
                return;
            }
            // a locker that has opened is closed after a look inside
            if let Some(v) = &d.locker
                && v.opened.is_some_and(|t| t > 0.6 + pace)
            {
                d.auto.wait = pace;
                None
            } else {
                let a = d.auto_action(&self.ctx.state_ref());
                // the map only stays open on the way somewhere
                if d.map.is_some() && !matches!(a, AutoAction::Travel(_)) {
                    d.map = None;
                }
                Some(a)
            }
        };
        let Some(action) = action else {
            self.close_locker();
            return;
        };
        let busy = self.runner.blocking() || self.ctx.0.borrow().cutscene > 0;
        let Some(d) = self.district.as_mut() else { return };
        match action {
            AutoAction::None => {}
            AutoAction::Travel(spawn) => {
                if busy || d.board.is_some() || d.locker.is_some() || d.dive.is_some() {
                    return;
                }
                // look at the map first, the way a player finds the next place
                if d.map.is_none() && pace > 0.0 && spawn != "exit" {
                    let mut v = crate::ui::citymap::MapView::new(&d.sig, &self.ctx.state_ref());
                    if let Some(i) = d.sig.sites.iter().position(|s| s.spawn == spawn) {
                        v.cursor = i;
                    }
                    d.map = Some(v);
                    d.auto.wait = pace * 2.5;
                    d.shots.push(format!("map-{spawn}"));
                    return;
                }
                d.map = None;
                d.auto.at = Some(spawn);
                d.auto.wait = pace;
                self.travel(spawn);
            }
            AutoAction::Interact(id) => {
                if busy {
                    return;
                }
                d.auto.wait = pace;
                self.interact(&id);
            }
            AutoAction::CloseBoard => {
                d.auto.wait = pace * 0.5;
                self.close_board();
            }
            AutoAction::Line(m) => {
                d.auto.wait = pace * if matches!(m, XMove::Turn(_)) { 0.25 } else { 1.0 };
                let mut st = self.ctx.state();
                if let Some(l) = d.lines.as_mut()
                    && l.apply(&mut st, &m)
                {
                    let sfx = match m {
                        XMove::Plug(..) | XMove::Pull(..) => "plug",
                        XMove::Turn(_) | XMove::Rewrite(..) | XMove::Forge(..) => "tick",
                        XMove::Dial(_) => "ring",
                        _ => "confirm",
                    };
                    self.audio.push(AudioReq::Sfx(sfx));
                    if let Some(v) = d.panel.as_mut() {
                        v.pick = None;
                    }
                }
            }
            AutoAction::Apply(m) => {
                d.auto.wait = pace;
                let mut st = self.ctx.state();
                match &m {
                    Move::Dial(l, digits) => {
                        if let Some(v) = d.locker.as_mut()
                            && v.id == *l
                        {
                            v.digits = digits.clone();
                            if d.sig.dial(&mut st, l, digits) {
                                v.opened = Some(0.0);
                                self.audio.push(AudioReq::Sfx("unlock"));
                            }
                        }
                    }
                    Move::Cut(refs) => {
                        if let Some(v) = d.board.as_mut() {
                            if v.table != *refs {
                                v.table = refs.clone();
                                self.audio.push(AudioReq::Sfx("blip"));
                            } else {
                                d.sig.cut(&mut st, refs);
                                v.table.clear();
                                self.audio.push(AudioReq::Sfx("pickup"));
                            }
                        }
                    }
                    Move::Pull(b) => {
                        let o = d.sig.pull(&mut st, b);
                        let mut sfx = vec!["lever"];
                        if let Some(v) = d.board.as_mut() {
                            crate::ui::board::after_pull(v, o, &mut sfx);
                        }
                        let _ = matches!(o, Outcome::Done(..));
                        self.audio.extend(sfx.into_iter().map(AudioReq::Sfx));
                    }
                    other => {
                        let sfx = match other {
                            Move::Toggle(..) => "tick",
                            Move::Copy(_) => "pickup",
                            Move::Slot(..) => "confirm",
                            _ => "blip",
                        };
                        d.sig.apply(&mut st, other);
                        self.audio.push(AudioReq::Sfx(sfx));
                    }
                }
                d.sig.note_pads(&mut st);
            }
        }
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
                let (handle, hints, deep) = {
                    let s = self.ctx.state_ref();
                    (s.handle().to_string(), s.settings.hints, !s.data.badges.iter().any(|b| b == p.id()))
                };
                self.terminal = Some(ActivePuzzle::new(id, p, &handle, hints, deep));
                self.auto_term = 0.0;
            }
            Cmd::Wait(s) => self.waits.push((id, s)),
            Cmd::Board(b) => {
                let opened = self.district.as_mut().and_then(|d| {
                    if let Some(p) = d.lines.as_ref().and_then(|l| l.panel(&b)) {
                        let (pid, len) = (p.id, p.log.len());
                        d.panel = Some(crate::ui::patch::PanelView::new(pid, id, len));
                        d.shots.push(format!("panel-{pid}"));
                        return Some(());
                    }
                    let bid = d.sig.board(&b)?.id;
                    let mut st = self.ctx.state();
                    d.sig.visit(&mut st, bid);
                    let shown = d.sig.shown(&st, bid);
                    d.board = Some(crate::ui::board::BoardView::new(bid, id, shown));
                    d.shots.push(format!("board-{bid}"));
                    Some(())
                });
                if opened.is_none() {
                    self.ctx.shared().reply(id, Reply::Done);
                } else {
                    self.audio.push(AudioReq::Sfx("power"));
                }
            }
            Cmd::Locker(l) => {
                let opened = self.district.as_mut().and_then(|d| {
                    let lk = d.sig.locker(&l)?;
                    let (lid, len, flag) = (lk.id, lk.code.len(), lk.flag);
                    let open = self.ctx.state_ref().flag(flag);
                    d.locker = Some(crate::ui::citymap::LockerView::new(lid, id, len, open));
                    Some(())
                });
                if opened.is_none() {
                    self.ctx.shared().reply(id, Reply::Done);
                }
            }
            Cmd::Dive(name) => {
                let seed = self.ctx.state_ref().data.seed.clone();
                let opened = {
                    let st = self.ctx.state_ref();
                    self.district.as_mut().and_then(|d| {
                        let did = d.dive_mut(dive_id(&name)?, seed.as_bytes(), &st)?.id();
                        d.dive = Some(DiveView { id: did, reply: id, age: 0.0, shake: 0.0, cleared_for: None });
                        Some(())
                    })
                };
                if opened.is_none() {
                    self.ctx.shared().reply(id, Reply::Bool(false));
                } else {
                    self.audio.push(AudioReq::Music(Some("tension")));
                }
            }
            Cmd::Signal(what) => {
                if let Some(d) = self.district.as_mut() {
                    let mut st = self.ctx.state();
                    match what.as_str() {
                        "flagged" => {
                            d.sig.reset_boards(&mut st);
                            if let Some(l) = d.lines.as_mut() {
                                l.reset();
                            }
                            d.auto.at = Some("start");
                        }
                        w if w.starts_with("pick:") => {
                            d.sig.pick(&mut st, &w[5..]);
                        }
                        _ => {}
                    }
                    d.sig.note_pads(&mut st);
                }
            }
            Cmd::Sfx(n) => self.audio.push(AudioReq::Sfx(n)),
            Cmd::Music(t) => self.audio.push(AudioReq::Music(t)),
            Cmd::Location { name, sub } => self.hud.show_location(&name, &sub),
            Cmd::Credits => {
                self.mode = Mode::Credits;
                self.credits = CreditsState::default();
                if self.ctx.state_ref().flag("end.done") {
                    self.credits.ending = Some(crate::ending::summary(&self.ctx.state_ref()));
                }
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
                StateEvent::Credits(n, why) => {
                    let (title, body) = if n >= 0 { ("CREDITS", format!("+{n} cr · {why}")) } else { ("PAID", format!("{n} cr · {why}")) };
                    self.hud.toast(title, &body, if n >= 0 { ToastKind::Item } else { ToastKind::Info });
                    self.audio.push(AudioReq::Sfx("coin"));
                }
                StateEvent::Ally(id) => {
                    let name = crate::allies::ally_def(&id).map(|a| a.name).unwrap_or(&id).to_string();
                    self.hud.toast("ALLY", &format!("{name} is on your web of trust"), ToastKind::Codex);
                    self.audio.push(AudioReq::Sfx("unlock"));
                }
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

    fn draw_district(&mut self, ui_ctx: &egui::Context, dt: f32, sfx: &mut Vec<&'static str>) {
        let auto = self.auto;
        let mut close_board = false;
        let mut close_locker = false;
        let mut close_dive = false;
        let mut travel: Option<&'static str> = None;
        let Some(d) = self.district.as_mut() else { return };
        if self.dialogue.is_empty() && self.terminal.is_none() && self.menu.is_none() {
            if let Some(v) = d.board.as_mut() {
                let mut st = self.ctx.state();
                let r = crate::ui::board::show(ui_ctx, v, &mut d.sig, &mut st, dt, (d.seen, d.footsteps), sfx);
                close_board = r.close;
            } else if let (Some(v), Some(lines)) = (d.panel.as_mut(), d.lines.as_mut()) {
                let mut st = self.ctx.state();
                let r = crate::ui::patch::show(ui_ctx, v, lines, &d.sig, &mut st, dt, (d.seen, d.footsteps), sfx);
                close_board = r.close;
            } else if let Some(v) = d.locker.as_mut() {
                let mut st = self.ctx.state();
                if crate::ui::citymap::locker(ui_ctx, v, &mut d.sig, &mut st, dt, sfx).is_some() {
                    close_locker = true;
                }
            } else if let Some(v) = d.dive.as_mut() {
                if let Some(dv) = d.dives.get(v.id) {
                    close_dive = crate::ui::dive::show(ui_ctx, v, dv.as_ref(), &mut d.dive_input, auto);
                }
            } else if let Some(v) = d.map.as_mut() {
                let def = self.ctx.shared().map_def();
                let st = self.ctx.state_ref();
                match crate::ui::citymap::show(ui_ctx, v, &d.sig, &st, &def, dt, sfx) {
                    crate::ui::citymap::MapAction::None => {}
                    crate::ui::citymap::MapAction::Close => d.map = None,
                    crate::ui::citymap::MapAction::Travel(spawn) => {
                        d.map = None;
                        travel = Some(spawn);
                    }
                }
                if d.map.is_none() {
                    drop(st);
                    self.close_modal();
                }
            }
        }
        if close_board {
            self.close_board();
        }
        if close_locker {
            self.close_locker();
        }
        if close_dive {
            let cleared = self.district.as_ref().and_then(|d| d.dive.as_ref()).is_some_and(|v| v.cleared_for.is_some());
            self.close_dive(cleared);
        }
        if let Some(spawn) = travel {
            if let Some(d) = self.district.as_mut() {
                d.auto.at = Some(spawn);
            }
            self.travel(spawn);
        }
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
        let hidden = self.terminal.is_some() || self.ctx.0.borrow().cutscene > 0 && self.dialogue.is_empty() || self.district.as_ref().is_some_and(|d| d.dive.is_some() || d.map.is_some());
        if !hidden {
            let clicks = {
                let state = self.ctx.state_ref();
                self.hud.show(ui_ctx, &state)
            };
            if !self.busy() && self.menu.is_none() {
                let (esc, c_key) = ui_ctx.input(|i| (i.key_pressed(egui::Key::Escape), i.key_pressed(egui::Key::C)));
                let m_key = ui_ctx.input(|i| i.key_pressed(egui::Key::M));
                if clicks.menu || (esc && self.input_ready()) {
                    self.open_menu(Tab::Keyring, None);
                } else if clicks.codex || (c_key && self.input_ready() && !ui_ctx.egui_wants_keyboard_input()) {
                    self.open_menu(Tab::Codex, None);
                } else if (clicks.map || (m_key && self.input_ready() && !ui_ctx.egui_wants_keyboard_input()))
                    && let Some(d) = self.district.as_mut()
                {
                    d.map = Some(crate::ui::citymap::MapView::new(&d.sig, &self.ctx.state_ref()));
                    self.audio.push(AudioReq::Sfx("blip"));
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
                if ap.host.deep_done {
                    let mut s = self.ctx.state();
                    let id = ap.puzzle.id().to_string();
                    if !s.data.badges.contains(&id) {
                        s.data.badges.push(id);
                        s.data.stats.deep_dives += 1;
                        s.earn(4, "a deep dive");
                    }
                }
                self.ctx.shared().reply(ap.reply, Reply::Bool(ok));
                self.close_modal();
            }
        }
        // a v4 district: a board or a panel, a locker, the map, a dive
        self.draw_district(ui_ctx, dt, sfx);
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
                        self.engine.push(EngineReq::Import(Box::new(d)));
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
                MenuAction::SetTrust(id, t) => {
                    self.ctx.state().set_trust(&id, crate::allies::trust_from(t));
                }
            }
        }
    }
}
