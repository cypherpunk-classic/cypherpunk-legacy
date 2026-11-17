//! The Wharf, live: its signals, the close-up windows the player can have
//! open there (a board, a locker, the district map, a dive), and a player
//! who knows the answers, for autoplay and the tests.

use std::collections::HashMap;

use crate::dive::{self, Dive, Input};
use crate::signals::wharf::{END, MAST, Move};
use crate::signals::{Signals, SiteKind};
use crate::state::GameState;
use crate::ui::board::BoardView;
use crate::ui::citymap::{LockerView, MapView};

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
    /// press E on this (a board, a locker, a card on the ground, a dive)
    Interact(String),
    /// act on the open close-up
    Apply(Move),
    CloseBoard,
}

#[derive(Default)]
pub struct Auto {
    /// the spawn the player was last sent to
    pub at: Option<&'static str>,
    pub wait: f32,
}

pub struct District {
    pub sig: Signals,
    pub board: Option<BoardView>,
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
    pub fn new(st: &mut GameState) -> Self {
        let mut sig = Signals::wharf(st);
        sig.start(st);
        Self { sig, board: None, map: None, locker: None, dive: None, dives: HashMap::new(), dive_input: Input::default(), seen: false, footsteps: false, auto: Auto::default(), shots: Vec::new() }
    }

    /// Any close-up open (the player cannot walk).
    pub fn modal(&self) -> bool {
        self.board.is_some() || self.map.is_some() || self.locker.is_some() || self.dive.is_some()
    }

    /// The dive for `id`, made on first use from the save's seed.
    pub fn dive_mut(&mut self, id: &'static str, seed: &[u8]) -> Option<&mut Box<dyn Dive>> {
        if !self.dives.contains_key(id) {
            let d = dive::make(id, seed)?;
            self.dives.insert(id, d);
        }
        self.dives.get_mut(id)
    }

    /// The next thing a player who knows every answer would do.
    pub fn auto_action(&self, st: &GameState) -> AutoAction {
        if self.dive.is_some() {
            return AutoAction::None;
        }
        let Some(site) = self.sig.next_site(st) else {
            if !st.flag(MAST) || st.flag(END) {
                return AutoAction::None;
            }
            if self.board.is_some() {
                return AutoAction::CloseBoard;
            }
            if self.auto.at != Some("exit") {
                return AutoAction::Travel("exit");
            }
            return AutoAction::Interact("w.exit".into());
        };
        if self.locker.is_some() && !matches!(self.sig.next_move(st, site.id), Some(Move::Dial(..))) {
            // an open locker closes itself once the player has seen what was in it
            return AutoAction::None;
        }
        if self.auto.at != Some(site.spawn) {
            return if self.board.is_some() { AutoAction::CloseBoard } else { AutoAction::Travel(site.spawn) };
        }
        if let SiteKind::Dive(id) = site.kind {
            return if self.board.is_some() { AutoAction::CloseBoard } else { AutoAction::Interact(format!("w.dive.{id}")) };
        }
        let Some(m) = self.sig.next_move(st, site.id) else {
            return if self.board.is_some() { AutoAction::CloseBoard } else { AutoAction::None };
        };
        match &m {
            Move::Wait => AutoAction::None,
            Move::Pick(w) => {
                if self.board.is_some() {
                    AutoAction::CloseBoard
                } else {
                    AutoAction::Interact(format!("w.pick.{w}"))
                }
            }
            Move::Dial(l, _) => {
                if self.board.is_some() {
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
}
