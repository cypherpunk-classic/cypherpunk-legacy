//! A v4 district, live: its puzzles (the Wharf's lamp boards or the
//! Exchange's lines), the close-up windows the player can have open there
//! (a board or a patch panel, a locker, the district map, a dive), and a
//! player who knows the answers, for autoplay and the tests.

use std::collections::HashMap;

use crate::dive::{self, Dive, Input};
use crate::lines::Lines;
use crate::lines::exchange::{self as xch, XMove};
use crate::signals::wharf::{END, MAST, Move};
use crate::signals::{Pose, Signals, SiteKind, Watcher};
use crate::state::{GameState, MapId};
use crate::ui::board::BoardView;
use crate::ui::citymap::{LockerView, MapView};
use crate::ui::patch::PanelView;

pub struct DiveView {
    pub id: &'static str,
    pub reply: u64,
    pub age: f32,
    pub shake: f32,
    /// seconds since the last level fell (the window closes itself)
    pub cleared_for: Option<f32>,
}

/// What the autoplayer wants done next.
#[derive(Clone, Debug, PartialEq)]
pub enum AutoAction {
    None,
    /// go to this spawn
    Travel(&'static str),
    /// press E on this (a board, a panel, a locker, a card on the ground, a dive, someone)
    Interact(String),
    /// act on the open lamp board or locker
    Apply(Move),
    /// act on the open patch panel
    Line(XMove),
    CloseBoard,
}

#[derive(Default)]
pub struct Auto {
    /// the spawn the player was last sent to
    pub at: Option<&'static str>,
    pub wait: f32,
}

pub struct District {
    /// which district this is
    pub which: MapId,
    /// the Wharf's boards; on the Exchange only its sites and its locker
    pub sig: Signals,
    /// the Exchange's lines
    pub lines: Option<Lines>,
    pub board: Option<BoardView>,
    pub panel: Option<PanelView>,
    pub map: Option<MapView>,
    pub locker: Option<LockerView>,
    pub dive: Option<DiveView>,
    /// dives in progress (step out and back in, and you are where you were)
    pub dives: HashMap<&'static str, Box<dyn Dive>>,
    /// what the player pressed in the dive window this frame (filled by the UI)
    pub dive_input: Input,
    /// a sensor or the guard has the player right now (set by the engine)
    pub seen: bool,
    /// the guard is close and coming closer (set by the engine)
    pub footsteps: bool,
    pub auto: Auto,
    /// milestones the autopilot photographs
    pub shots: Vec<String>,
}

impl District {
    pub fn new(st: &mut GameState, which: MapId) -> Self {
        let (sig, lines) = if which == MapId::Exchange {
            (Signals::exchange(st), Some(Lines::exchange(st)))
        } else {
            let mut sig = Signals::wharf(st);
            sig.start(st);
            (sig, None)
        };
        Self { which, sig, lines, board: None, panel: None, map: None, locker: None, dive: None, dives: HashMap::new(), dive_input: Input::default(), seen: false, footsteps: false, auto: Auto::default(), shots: Vec::new() }
    }

    /// Any close-up open (the player cannot walk).
    pub fn modal(&self) -> bool {
        self.board.is_some() || self.panel.is_some() || self.map.is_some() || self.locker.is_some() || self.dive.is_some()
    }

    /// A board or panel open and nothing over it (the street keeps watching).
    pub fn at_board(&self) -> bool {
        (self.board.is_some() || self.panel.is_some()) && self.map.is_none() && self.locker.is_none() && self.dive.is_none()
    }

    /// The dive for `id`, made on first use from the save's seed.
    pub fn dive_mut(&mut self, id: &'static str, seed: &[u8], st: &GameState) -> Option<&mut Box<dyn Dive>> {
        if !self.dives.contains_key(id) {
            let d: Box<dyn Dive> = match (id, &self.lines) {
                // LOGJAM breaks the numbers on your tape
                ("logjam", Some(l)) => Box::new(dive::logjam::Logjam::new(seed, l.logjam_targets(st))),
                _ => dive::make(id, seed)?,
            };
            self.dives.insert(id, d);
        }
        self.dives.get_mut(id)
    }

    /// Advance the district's clock: boards, lines, patrols.
    pub fn tick(&mut self, st: &mut GameState, dt: f32) {
        self.sig.tick(st, dt);
        if let Some(l) = self.lines.as_mut() {
            l.tick(st, dt);
            l.after(st);
        }
    }

    pub fn poses(&self, st: &GameState) -> Vec<Pose> {
        match &self.lines {
            Some(l) => l.poses(st, self.sig.t),
            None => self.sig.poses(st),
        }
    }

    pub fn watchers(&self, st: &GameState) -> Vec<Watcher> {
        match &self.lines {
            Some(l) => l.watchers(st, self.sig.t),
            None => self.sig.watchers(st),
        }
    }

    pub fn footsteps_at(&self, st: &GameState, x: f32, y: f32) -> bool {
        match &self.lines {
            Some(l) => l.footsteps(st, self.sig.t, x, y),
            None => self.sig.footsteps(st, x, y),
        }
    }

    pub fn exposure(&self, st: &GameState, x: f32, y: f32) -> f32 {
        match &self.lines {
            Some(l) => l.exposure(st, self.sig.t, x, y),
            None => self.sig.exposure(st, x, y),
        }
    }

    /// The flag that says the district is done, and the one set on walking out.
    pub fn goal(&self) -> (&'static str, &'static str) {
        if self.lines.is_some() { (xch::FREE, xch::END) } else { (MAST, END) }
    }

    /// The next thing a player who knows every answer would do.
    pub fn auto_action(&self, st: &GameState) -> AutoAction {
        if self.dive.is_some() {
            return AutoAction::None;
        }
        let open = self.board.is_some() || self.panel.is_some();
        let Some(site) = self.sig.next_site(st) else {
            let (free, end) = self.goal();
            if !st.flag(free) || st.flag(end) {
                return AutoAction::None;
            }
            if open {
                return AutoAction::CloseBoard;
            }
            if self.auto.at != Some("exit") {
                return AutoAction::Travel("exit");
            }
            return AutoAction::Interact(if self.lines.is_some() { "x.exit" } else { "w.exit" }.into());
        };
        if let Some(lines) = &self.lines {
            return self.exchange_action(lines, st, site.id, site.spawn, site.kind);
        }
        if self.locker.is_some() && !matches!(self.sig.next_move(st, site.id), Some(Move::Dial(..))) {
            // an open locker closes itself once the player has seen what was in it
            return AutoAction::None;
        }
        if self.auto.at != Some(site.spawn) {
            return if open { AutoAction::CloseBoard } else { AutoAction::Travel(site.spawn) };
        }
        if let SiteKind::Dive(id) = site.kind {
            return if open { AutoAction::CloseBoard } else { AutoAction::Interact(format!("w.dive.{id}")) };
        }
        let Some(m) = self.sig.next_move(st, site.id) else {
            return if open { AutoAction::CloseBoard } else { AutoAction::None };
        };
        match &m {
            Move::Wait => AutoAction::None,
            Move::Pick(w) => {
                if open {
                    AutoAction::CloseBoard
                } else {
                    AutoAction::Interact(format!("w.pick.{w}"))
                }
            }
            Move::Dial(l, _) => {
                if open {
                    AutoAction::CloseBoard
                } else if self.locker.as_ref().is_some_and(|v| v.id == *l) {
                    AutoAction::Apply(m)
                } else {
                    AutoAction::Interact(format!("w.locker.{l}"))
                }
            }
            // the light table is in every close-up; use one at this site
            Move::Cut(_) => match &self.board {
                Some(_) => AutoAction::Apply(m),
                None => AutoAction::Interact(format!("w.board.{}", self.sig.first_board(site.id).unwrap_or("a1"))),
            },
            _ => {
                let b = m.board().unwrap_or("a1");
                match &self.board {
                    Some(v) if v.board == b => AutoAction::Apply(m),
                    Some(_) => AutoAction::CloseBoard,
                    None => AutoAction::Interact(format!("w.board.{b}")),
                }
            }
        }
    }

    fn exchange_action(&self, lines: &Lines, st: &GameState, site: &'static str, spawn: &'static str, kind: SiteKind) -> AutoAction {
        let open = self.panel.is_some();
        let m = lines.next_move(st, site);
        if self.locker.is_some() && !matches!(m, Some(XMove::Locker(..))) {
            return AutoAction::None;
        }
        if self.auto.at != Some(spawn) {
            return if open { AutoAction::CloseBoard } else { AutoAction::Travel(spawn) };
        }
        if let SiteKind::Dive(id) = kind {
            return if open { AutoAction::CloseBoard } else { AutoAction::Interact(format!("x.dive.{id}")) };
        }
        let here = xch::panel_of(site);
        match m {
            None => {
                if open {
                    AutoAction::CloseBoard
                } else {
                    AutoAction::None
                }
            }
            Some(XMove::Felix) => {
                if open {
                    AutoAction::CloseBoard
                } else {
                    AutoAction::Interact("x.felix".into())
                }
            }
            Some(XMove::Locker(l, digits)) => {
                if open {
                    AutoAction::CloseBoard
                } else if self.locker.as_ref().is_some_and(|v| v.id == l) {
                    AutoAction::Apply(Move::Dial(l, digits))
                } else {
                    AutoAction::Interact(format!("x.locker.{l}"))
                }
            }
            Some(m) => {
                // wait at the panel, watching it; act on it when there is something to do
                let p = m.panel().or(here).unwrap_or("junction");
                match &self.panel {
                    Some(v) if v.panel == p => {
                        if m == XMove::Wait {
                            AutoAction::None
                        } else {
                            AutoAction::Line(m)
                        }
                    }
                    Some(_) => AutoAction::CloseBoard,
                    None => AutoAction::Interact(format!("x.panel.{p}")),
                }
            }
        }
    }
}
