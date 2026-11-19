//! The Wharf, end to end: the autoplayer starts on the boathouse steps, works
//! every board, dial and dive through the real game (scripts, windows,
//! egui), frees the mast and walks out, up the road to the Exchange.

use cpl_logic::game::Mode;
use cpl_logic::lines::exchange as xch;
use cpl_logic::signals::wharf::{DIVE_INTERCEPTOR, DIVE_STATIC, END, MAST};
use cpl_logic::state::MapId;
use cpl_logic::walkthrough::Headless;

/// Play until the Wharf is behind you (or, with `through`, to the end of v4).
fn play(seed: &str, through: bool) -> Headless {
    let mut h = Headless::wharf(seed);
    let limit = 60 * 60 * 30; // an hour of game time at 30 fps
    for _ in 0..limit {
        h.frame();
        if h.game.mode == Mode::Credits {
            break;
        }
        let st = h.game.ctx.state_ref();
        if !through && st.flag(END) && st.data.scene == MapId::Exchange && h.game.district.as_ref().is_some_and(|d| d.which == MapId::Exchange) {
            break;
        }
    }
    h
}

#[test]
fn the_autoplayer_frees_the_wharf_and_walks_out() {
    let h = play("wharf-e2e", false);
    let st = h.game.ctx.state_ref();
    let open: Vec<&str> = cpl_logic::signals::wharf::sites().iter().filter(|s| !st.flag(s.done)).map(|s| s.id).collect();
    assert!(st.flag(MAST), "the mast is still theirs after {} frames; unfinished: {open:?}; objective {:?}", h.report.frames, st.data.objective);
    assert!(st.flag(DIVE_INTERCEPTOR) && st.flag(DIVE_STATIC), "both dives broken");
    assert!(st.flag(END));
    assert!(open.is_empty(), "{open:?}");
    // out of the north gate and up the road
    assert_eq!(st.data.scene, MapId::Exchange);
    let (last, rest) = h.report.gotos.split_last().expect("some travel");
    assert_eq!(*last, MapId::Exchange);
    assert!(rest.iter().all(|m| *m == MapId::Wharf), "the route stayed on the Wharf until the end");
    let secs = h.report.frames as f32 / 30.0;
    assert!(secs < 25.0 * 60.0, "took {secs:.0} s of game time");
}

#[test]
fn a_second_seed_plays_through_both_districts() {
    let h = play("another harbour", true);
    let st = h.game.ctx.state_ref();
    assert!(st.flag(END), "the Wharf");
    assert!(st.flag(xch::FREE) && st.flag(xch::END), "and the Exchange; objective {:?}", st.data.objective);
    assert_eq!(h.game.mode, Mode::Credits);
    assert!(h.game.credits.ending.as_ref().is_some_and(|(t, _)| t.contains("EXCHANGE")));
}

#[test]
#[ignore]
fn report() {
    let h = play("wharf-e2e", false);
    let st = h.game.ctx.state_ref();
    eprintln!("frames {} = {:.0} s", h.report.frames, h.report.frames as f32 / 30.0);
    eprintln!("interactions {:?}", h.report.interactions);
    eprintln!("spawns {:?}", h.report.spawns);
    eprintln!("cards: {:?}", st.data.cards.iter().map(|c| (&c.name, &c.note)).collect::<Vec<_>>());
    eprintln!("flags: {:?}", st.data.flags.keys().collect::<Vec<_>>());
    eprintln!("trace {} flagged {}", st.trace, st.data.stats.flagged);
}
