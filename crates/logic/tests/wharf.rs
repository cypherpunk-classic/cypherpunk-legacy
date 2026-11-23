//! Chapter 1's harbour, end to end: Mara has sent the player down to the
//! Wharf district; the autoplayer works every board, dial and dive through
//! the real game (scripts, windows, egui), frees the mast, walks back up to
//! the radio shack, and the story carries on in the city.

use cpl_logic::codex::UNLOCKS;
use cpl_logic::game::Mode;
use cpl_logic::signals::wharf::{DIVE_INTERCEPTOR, DIVE_STATIC, END, MAST};
use cpl_logic::state::MapId;
use cpl_logic::walkthrough::Headless;

/// Play until the harbour is behind you and the city has loaded.
fn play(seed: &str) -> Headless {
    let mut h = Headless::wharf(seed);
    let limit = 60 * 60 * 30; // an hour of game time at 30 fps
    for _ in 0..limit {
        h.frame();
        let st = h.game.ctx.state_ref();
        if st.flag(END) && h.game.current_map() == MapId::City && h.game.district.is_none() && h.game.idle() {
            break;
        }
    }
    h
}

#[test]
fn the_autoplayer_frees_the_harbour_and_walks_back_up() {
    let h = play("wharf-e2e");
    let st = h.game.ctx.state_ref();
    let open: Vec<&str> = cpl_logic::signals::wharf::sites().iter().filter(|s| !st.flag(s.done)).map(|s| s.id).collect();
    assert!(st.flag(MAST), "the mast is still theirs after {} frames; unfinished: {open:?}; objective {:?}", h.report.frames, st.data.objective);
    assert!(st.flag(DIVE_INTERCEPTOR) && st.flag(DIVE_STATIC), "both dives broken");
    assert!(st.flag(END));
    assert!(open.is_empty(), "{open:?}");
    // out of the north gate and back up to Mara
    assert_eq!(st.data.scene, MapId::City);
    assert_eq!(h.report.gotos.last(), Some(&MapId::City));
    assert_eq!(h.report.spawns.last().map(String::as_str), Some("mara"));
    assert!(!st.flag("ch1.done"), "Mara closes the chapter, not the walk");
    // the harbour's puzzles taught what the codex says they did
    for (flag, ids) in UNLOCKS.iter().filter(|(f, _)| f.starts_with("w.")) {
        assert!(st.flag(flag));
        for id in *ids {
            assert!(st.data.codex.iter().any(|c| c == id), "{id} never unlocked");
        }
    }
    let secs = h.report.frames as f32 / 30.0;
    assert!(secs < 25.0 * 60.0, "took {secs:.0} s of game time");
}

#[test]
fn mara_closes_chapter_one_and_sends_you_to_felix() {
    let mut h = play("another harbour");
    assert!(h.interact("mara.talk"));
    let st = h.game.ctx.state_ref();
    assert!(st.flag("ch1.done"), "objective {:?}", st.data.objective);
    assert_eq!(st.data.chapter, 2);
    assert!(st.has("share1") && st.has("sealed-messenger"));
    assert_eq!(h.game.mode, Mode::World);
}

#[test]
#[ignore]
fn report() {
    let h = play("wharf-e2e");
    let st = h.game.ctx.state_ref();
    eprintln!("frames {} = {:.0} s", h.report.frames, h.report.frames as f32 / 30.0);
    eprintln!("spawns {:?}", h.report.spawns);
    eprintln!("cards: {:?}", st.data.cards.iter().map(|c| (&c.name, &c.note)).collect::<Vec<_>>());
    eprintln!("codex: {:?}", st.data.codex);
    eprintln!("trace {} flagged {}", st.trace, st.data.stats.flagged);
}
