//! The story runtime.
//!
//! Story scripts are ordinary `async fn`s. They read and write game state
//! directly, and *ask the engine* for everything else (show dialogue, open a
//! puzzle, pan the camera) by sending a [`Cmd`] and awaiting its [`Reply`].
//! A director on the engine side executes the commands; in tests a headless
//! director answers them instantly. Futures are polled with a no-op waker once
//! per frame (or until they stop making progress), so no async runtime is
//! needed and the same code runs natively and in the browser.

use std::cell::{Ref, RefCell, RefMut};
use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use crate::maps::{MapDef, map};
use crate::puzzle::Puzzle;
use crate::speakers::Sp;
use crate::state::{GameState, MapId};
use crate::world::World;

pub type BoxFut = Pin<Box<dyn Future<Output = ()>>>;

/// A request from a script to the engine.
pub enum Cmd {
    // --- blocking: the script waits for a reply ---
    Say(Vec<(Sp, String)>),
    Choose { sp: Sp, question: String, options: Vec<String> },
    Puzzle(Box<dyn Puzzle>),
    Wait(f32),
    Goto { map: MapId, spawn: Option<String> },
    PanTo { x: f32, y: f32, ms: f32 },
    // --- fire and forget ---
    Follow,
    Flash { ms: f32, color: u32 },
    Burst { x: f32, y: f32, color: u32, n: u32 },
    Shake { ms: f32 },
    SetObjectFrame { id: String, frame: usize },
    Sfx(&'static str),
    Music(Option<&'static str>),
    Location { name: String, sub: String },
    SavePosition,
    Credits,
}

impl Cmd {
    pub fn blocking(&self) -> bool {
        matches!(self, Cmd::Say(_) | Cmd::Choose { .. } | Cmd::Puzzle(_) | Cmd::Wait(_) | Cmd::Goto { .. } | Cmd::PanTo { .. })
    }

    pub fn name(&self) -> &'static str {
        match self {
            Cmd::Say(_) => "say",
            Cmd::Choose { .. } => "choose",
            Cmd::Puzzle(_) => "puzzle",
            Cmd::Wait(_) => "wait",
            Cmd::Goto { .. } => "goto",
            Cmd::PanTo { .. } => "pan",
            Cmd::Follow => "follow",
            Cmd::Flash { .. } => "flash",
            Cmd::Burst { .. } => "burst",
            Cmd::Shake { .. } => "shake",
            Cmd::SetObjectFrame { .. } => "frame",
            Cmd::Sfx(_) => "sfx",
            Cmd::Music(_) => "music",
            Cmd::Location { .. } => "location",
            Cmd::SavePosition => "save",
            Cmd::Credits => "credits",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reply {
    Done,
    Choice(usize),
    Bool(bool),
}

/// State shared between the scripts and the engine.
pub struct Shared {
    pub state: GameState,
    pub queue: VecDeque<(u64, Cmd)>,
    replies: HashMap<u64, Reply>,
    next_id: u64,
    /// Nesting depth of cutscenes (player control suspended while > 0).
    pub cutscene: u32,
    /// Which map the world is showing.
    pub map: MapId,
    map_def: Option<(MapId, Rc<MapDef>)>,
}

impl Shared {
    pub fn new(state: GameState) -> Self {
        Self { state, queue: VecDeque::new(), replies: HashMap::new(), next_id: 1, cutscene: 0, map: MapId::Cabin, map_def: None }
    }

    pub fn reply(&mut self, id: u64, r: Reply) {
        self.replies.insert(id, r);
    }

    pub fn map_def(&mut self) -> Rc<MapDef> {
        match &self.map_def {
            Some((id, m)) if *id == self.map => m.clone(),
            _ => {
                let m = Rc::new(map(self.map));
                self.map_def = Some((self.map, m.clone()));
                m
            }
        }
    }
}

/// The handle scripts hold. Cheap to clone.
#[derive(Clone)]
pub struct Ctx(pub Rc<RefCell<Shared>>);

pub struct WaitReply {
    ctx: Ctx,
    id: u64,
}

impl Future for WaitReply {
    type Output = Reply;
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Reply> {
        match self.ctx.0.borrow_mut().replies.remove(&self.id) {
            Some(r) => Poll::Ready(r),
            None => Poll::Pending,
        }
    }
}

/// Decrements the cutscene counter when dropped.
struct CutsceneGuard(Ctx);

impl Drop for CutsceneGuard {
    fn drop(&mut self) {
        let mut s = self.0.0.borrow_mut();
        s.cutscene = s.cutscene.saturating_sub(1);
    }
}

impl Ctx {
    pub fn new(state: GameState) -> Self {
        Self(Rc::new(RefCell::new(Shared::new(state))))
    }

    pub fn shared(&self) -> RefMut<'_, Shared> {
        self.0.borrow_mut()
    }

    pub fn state(&self) -> RefMut<'_, GameState> {
        RefMut::map(self.0.borrow_mut(), |s| &mut s.state)
    }

    pub fn state_ref(&self) -> Ref<'_, GameState> {
        Ref::map(self.0.borrow(), |s| &s.state)
    }

    pub fn world(&self) -> Rc<World> {
        self.state_ref().world()
    }

    pub fn send(&self, cmd: Cmd) -> u64 {
        let mut s = self.0.borrow_mut();
        let id = s.next_id;
        s.next_id += 1;
        s.queue.push_back((id, cmd));
        id
    }

    pub fn request(&self, cmd: Cmd) -> WaitReply {
        let id = self.send(cmd);
        WaitReply { ctx: self.clone(), id }
    }

    // --- state ------------------------------------------------------------------

    pub fn flag(&self, k: &str) -> bool {
        self.state_ref().flag(k)
    }

    pub fn set(&self, k: &str) {
        self.state().set(k, true);
    }

    pub fn clear(&self, k: &str) {
        self.state().set(k, false);
    }

    pub fn has(&self, item: &str) -> bool {
        self.state_ref().has(item)
    }

    pub fn objective(&self, text: &str) {
        self.state().set_objective(text);
    }

    pub fn codex(&self, ids: &[&str]) {
        let mut s = self.state();
        for id in ids {
            s.unlock_codex(id);
        }
    }

    pub fn give(&self, item: &str) {
        self.state().give(item);
    }

    pub fn chapter(&self, n: u32) {
        self.state().set_chapter(n);
    }

    pub fn handle(&self) -> String {
        self.state_ref().handle().to_string()
    }

    // --- fire and forget ------------------------------------------------------------

    pub fn sfx(&self, name: &'static str) {
        self.send(Cmd::Sfx(name));
    }

    pub fn music(&self, track: &'static str) {
        self.send(Cmd::Music(Some(track)));
    }

    pub fn location(&self, name: &str, sub: &str) {
        self.send(Cmd::Location { name: name.into(), sub: sub.into() });
    }

    pub fn follow(&self) {
        self.send(Cmd::Follow);
    }

    pub fn flash(&self, ms: f32, color: u32) {
        self.send(Cmd::Flash { ms, color });
    }

    pub fn burst(&self, x: f32, y: f32, color: u32, n: u32) {
        self.send(Cmd::Burst { x, y, color, n });
    }

    pub fn set_object_frame(&self, id: &str, frame: usize) {
        self.send(Cmd::SetObjectFrame { id: id.into(), frame });
    }

    pub fn save_position(&self) {
        self.send(Cmd::SavePosition);
    }

    pub fn credits(&self) {
        self.send(Cmd::Credits);
    }

    /// Bottom-centre of a map object on the current map.
    pub fn object_pos(&self, id: &str) -> Option<(f32, f32)> {
        let m = self.shared().map_def();
        m.object(id).map(|o| (o.x, o.y))
    }

    pub fn current_map(&self) -> MapId {
        self.0.borrow().map
    }

    // --- blocking -------------------------------------------------------------------

    /// Show lines one after another; resolves once the last is dismissed.
    pub async fn say(&self, lines: Vec<(Sp, String)>) {
        let filled = {
            let s = self.state_ref();
            lines.into_iter().map(|(sp, t)| (sp, s.fill(&t))).collect()
        };
        self.request(Cmd::Say(filled)).await;
    }

    pub async fn say1(&self, sp: Sp, text: &str) {
        self.say(vec![(sp, text.to_string())]).await;
    }

    /// Ask a question; resolves with the index of the chosen option.
    pub async fn choose(&self, sp: Sp, question: &str, options: &[&str]) -> usize {
        let (question, options) = {
            let s = self.state_ref();
            (s.fill(question), options.iter().map(|o| s.fill(o)).collect())
        };
        match self.request(Cmd::Choose { sp, question, options }).await {
            Reply::Choice(i) => i,
            _ => 0,
        }
    }

    /// Open a puzzle terminal; true once solved and dismissed, false if closed.
    pub async fn puzzle(&self, p: Box<dyn Puzzle>) -> bool {
        let ok = matches!(self.request(Cmd::Puzzle(p)).await, Reply::Bool(true));
        self.state().save();
        ok
    }

    pub async fn wait(&self, ms: f32) {
        self.request(Cmd::Wait(ms / 1000.0)).await;
    }

    pub async fn goto(&self, map: MapId, spawn: Option<&str>) {
        self.request(Cmd::Goto { map, spawn: spawn.map(String::from) }).await;
    }

    pub async fn pan_to(&self, x: f32, y: f32, ms: f32) {
        self.request(Cmd::PanTo { x, y, ms }).await;
    }

    /// Run a routine with player control suspended.
    pub async fn cutscene<F: Future<Output = ()>>(&self, f: F) {
        self.0.borrow_mut().cutscene += 1;
        let _guard = CutsceneGuard(self.clone());
        f.await;
    }
}

/// Shorthand for dialogue: `say!(c, (Narrator, "..."), (You, format!(..))).await`.
#[macro_export]
macro_rules! say {
    ($c:expr, $( ($sp:ident, $txt:expr) ),+ $(,)?) => {
        $c.say(vec![ $( ($crate::speakers::Sp::$sp, ::std::string::String::from($txt)) ),+ ])
    };
}

pub struct Task {
    pub name: String,
    /// Interactions block player control; background hooks do not (unless they run a cutscene).
    pub blocking: bool,
    fut: BoxFut,
}

/// A tiny executor for story scripts.
#[derive(Default)]
pub struct Runner {
    pub tasks: Vec<Task>,
}

impl Runner {
    pub fn spawn(&mut self, name: impl Into<String>, blocking: bool, fut: BoxFut) {
        self.tasks.push(Task { name: name.into(), blocking, fut });
    }

    /// Poll every task once. Returns how many finished.
    pub fn poll(&mut self) -> usize {
        let mut cx = Context::from_waker(Waker::noop());
        let before = self.tasks.len();
        self.tasks.retain_mut(|t| t.fut.as_mut().poll(&mut cx).is_pending());
        before - self.tasks.len()
    }

    pub fn blocking(&self) -> bool {
        self.tasks.iter().any(|t| t.blocking)
    }

    pub fn idle(&self) -> bool {
        self.tasks.is_empty()
    }

    pub fn clear(&mut self) {
        self.tasks.clear();
    }
}
