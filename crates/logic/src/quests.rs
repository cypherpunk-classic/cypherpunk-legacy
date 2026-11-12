//! Side quests: who asked, what is left, what it pays. Progress lives in
//! flags; this module only describes it for the quest log.

use crate::state::GameState;

#[derive(Clone, Copy, Debug)]
pub struct QuestDef {
    pub id: &'static str,
    pub title: &'static str,
    pub giver: &'static str,
    /// set when the quest is offered
    pub start: &'static str,
    /// (flag that completes the stage, what to do in that stage)
    pub stages: &'static [(&'static str, &'static str)],
    pub idea: &'static str,
}

pub const QUESTS: &[QuestDef] = &[
    QuestDef {
        id: "source",
        title: "The Source",
        giver: "Rosa, on the Wharf",
        start: "q.source",
        stages: &[("q.source.done", "Help Rosa work out which of her source's drops are genuine.")],
        idea: "Authenticate before you trust a message, and never reuse a nonce.",
    },
    QuestDef {
        id: "dots",
        title: "Yellow Dots",
        giver: "Theo, at the Exchange",
        start: "q.dots",
        stages: &[("q.dots.done", "Read the tracking dots on Theo's printout before he leaks it.")],
        idea: "Printers sign every page with their serial number and the time.",
    },
    QuestDef {
        id: "ledger",
        title: "The Ledger",
        giver: "Wen, at the Commons",
        start: "q.ledger",
        stages: &[("q.ledger.done", "Find where someone rewrote Wen's hash-chained ledger.")],
        idea: "A hash chain shows exactly where history was changed, if its head was published.",
    },
    QuestDef {
        id: "floppies",
        title: "Grandpa's Floppies",
        giver: "Nonce",
        start: "q.floppies",
        stages: &[("floppy.all", "Find the six floppy disks Grandpa hid around the city.")],
        idea: "The mailing-list years, in his own words.",
    },
];

pub const FLOPPIES: usize = 6;

pub fn floppies_found(s: &GameState) -> usize {
    (1..=FLOPPIES).filter(|i| s.flag(&format!("floppy.{i}"))).count()
}

/// One quest as the log shows it: (definition, current task or None if done).
pub fn quest_log(s: &GameState) -> Vec<(&'static QuestDef, Option<&'static str>)> {
    QUESTS
        .iter()
        .filter(|q| s.flag(q.start))
        .map(|q| (q, q.stages.iter().find(|(f, _)| !s.flag(f)).map(|(_, t)| *t)))
        .collect()
}
