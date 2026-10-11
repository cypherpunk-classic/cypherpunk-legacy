//! Puzzles live in terminal windows. A puzzle draws its body with egui and
//! reports status and success through a [`Host`]. Every puzzle also
//! implements `autoplay`, which drives its own controls to the solution; the
//! end-to-end tests use it to play the whole game, and it doubles as a living
//! specification of how each puzzle is meant to be solved.

use crate::ui::widgets::Tone;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Size {
    Narrow,
    Medium,
    Wide,
}

impl Size {
    pub fn width(self) -> f32 {
        match self {
            Size::Narrow => 560.0,
            Size::Medium => 760.0,
            Size::Wide => 980.0,
        }
    }
}

/// What a puzzle can ask of its terminal.
#[derive(Default)]
pub struct Host {
    pub status: String,
    pub tone: Tone,
    solved: bool,
    pub attempts: u32,
    /// Sound effects requested this frame.
    pub sfx: Vec<&'static str>,
    /// Seconds since the terminal opened.
    pub time: f64,
    /// Seconds since the last frame.
    pub dt: f32,
    /// The player's handle (some puzzles put it in their messages).
    pub handle: String,
    /// Files the puzzle offers for download: (file name, contents).
    pub downloads: Vec<(String, String)>,
}

impl Host {
    pub fn new(handle: &str) -> Self {
        Self { handle: handle.to_string(), ..Default::default() }
    }

    pub fn status(&mut self, text: impl Into<String>, tone: Tone) {
        self.status = text.into();
        self.tone = tone;
        if tone == Tone::Bad {
            self.attempts += 1;
        }
    }

    /// Mark the puzzle solved (idempotent).
    pub fn solve(&mut self, message: impl Into<String>) {
        if self.solved {
            return;
        }
        self.solved = true;
        self.sfx.push("success");
        self.status = message.into();
        self.tone = Tone::Ok;
    }

    pub fn solved(&self) -> bool {
        self.solved
    }

    pub fn sfx(&mut self, name: &'static str) {
        self.sfx.push(name);
    }

    /// Offer a text file to the player (saved to disk natively, downloaded on the web).
    pub fn download(&mut self, name: impl Into<String>, contents: impl Into<String>) {
        self.downloads.push((name.into(), contents.into()));
    }
}

pub trait Puzzle {
    /// Stable id, e.g. `"pro.seed"` (recorded in the save when solved).
    fn id(&self) -> &'static str;
    fn title(&self) -> &str;
    fn subtitle(&self) -> Option<&str> {
        None
    }
    /// Accent colour for the window chrome.
    fn accent(&self) -> egui::Color32 {
        crate::ui::theme::CYAN
    }
    fn size(&self) -> Size {
        Size::Wide
    }
    fn closable(&self) -> bool {
        true
    }
    /// Called once when the terminal opens (set the initial status line).
    fn open(&mut self, _host: &mut Host) {}
    /// Draw the body. Called every frame while the terminal is open.
    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host);
    /// Solve the puzzle by working its own controls.
    fn autoplay(&mut self, host: &mut Host);
}
