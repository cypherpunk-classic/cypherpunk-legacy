//! How it ends: the finale tallies your allies, your choices and your trace,
//! and the credits show the numbers.

use crate::state::{FlagValue, GameState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ending {
    /// many allies, no logs, the stolen key disclosed and burned
    OpenSociety,
    /// the Archive is public, but the network is thin or compromised
    QuietVictory,
    /// the Directorate keeps its grip for now; the code is out there
    LongWinter,
}

impl Ending {
    pub fn title(self) -> &'static str {
        match self {
            Ending::OpenSociety => "An Open Society",
            Ending::QuietVictory => "A Quiet Victory",
            Ending::LongWinter => "The Long Winter",
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Ending::OpenSociety => "open",
            Ending::QuietVictory => "quiet",
            Ending::LongWinter => "winter",
        }
    }
}

/// What you did with Warden's key: "disclose", "publish", "keep" (or "" if never asked).
pub fn key_choice(s: &GameState) -> String {
    match s.get("choice.key") {
        Some(FlagValue::Str(v)) => v.clone(),
        _ => String::new(),
    }
}

/// The tally. Every line of it is shown in the credits.
pub fn tally(s: &GameState) -> Vec<(String, i32)> {
    let mut t = Vec::new();
    t.push((format!("{} allies on your web of trust", s.ally_count()), s.ally_count() as i32));
    if s.flag("book.out") {
        t.push(("Grandpa's code got out as a book".into(), 2));
    }
    match key_choice(s).as_str() {
        "disclose" => t.push(("Warden's key: used once, then disclosed and revoked".into(), 3)),
        "publish" => t.push(("Warden's key: published for anyone to forge with".into(), -1)),
        "keep" => t.push(("Warden's key: kept quietly, as leverage".into(), -3)),
        _ => {}
    }
    if s.flag("choice.snoop") {
        t.push(("you read the co-op's messages".into(), -2));
    }
    if s.flag("choice.mintlog") {
        t.push(("the Commons Mint kept a list of names".into(), -2));
    }
    if s.flag("choice.gaveup") {
        t.push(("your remailer gave up Kestrel's address".into(), -5));
    } else if s.flag("choice.logs") {
        t.push(("your remailer kept a table, and had to shut down".into(), -1));
    }
    if s.data.stats.flagged > 5 {
        t.push((format!("flagged by the Dragnet {} times", s.data.stats.flagged), -1));
    }
    t
}

pub fn score(s: &GameState) -> i32 {
    tally(s).iter().map(|(_, v)| v).sum()
}

pub fn ending(s: &GameState) -> Ending {
    let sc = score(s);
    if sc >= 12 && !s.flag("choice.gaveup") {
        Ending::OpenSociety
    } else if sc >= 6 {
        Ending::QuietVictory
    } else {
        Ending::LongWinter
    }
}

/// Title and lines for the credits.
pub fn summary(s: &GameState) -> (String, Vec<String>) {
    let e = ending(s);
    let st = &s.data.stats;
    let mut lines: Vec<String> = tally(s).into_iter().map(|(t, v)| format!("{t}  ({v:+})")).collect();
    lines.push(String::new());
    lines.push(format!("{} payments in eCash the mint could not link to you", st.payments));
    lines.push(format!("{} Hashcash stamps minted", st.stamps_minted));
    lines.push(format!("{} deep dives", st.deep_dives));
    lines.push(format!("{} of {} floppies found", crate::quests::floppies_found(s), crate::quests::FLOPPIES));
    (e.title().to_string(), lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choices_move_the_ending() {
        let mut s = GameState::new();
        s.new_game("t", 0, Some("ending"));
        for a in ["mara", "felix", "juno", "c2", "ibrahim", "lin", "ada"] {
            s.recruit(a);
        }
        s.set("book.out", true);
        s.set("choice.key", "disclose");
        assert_eq!(ending(&s), Ending::OpenSociety);
        s.set("choice.mintlog", true);
        assert_eq!(ending(&s), Ending::QuietVictory);
        s.set("choice.gaveup", true);
        s.set("choice.key", "keep");
        assert_eq!(ending(&s), Ending::LongWinter);
    }
}
