//! The Wharf preview, end to end: the autoplayer starts on the boathouse
//! steps, works every board, dial and dive through the real game (scripts,
//! windows, egui), frees the mast and walks out.

use cpl_logic::game::Mode;
use cpl_logic::signals::wharf::{DIVE_INTERCEPTOR, DIVE_STATIC, END, MAST};
use cpl_logic::state::MapId;
use cpl_logic::walkthrough::Headless;

fn play(seed: &str) -> Headless {
    let mut h = Headless::wharf(seed);
    let limit = 30 * 60 * 30; // half an hour of game time at 30 fps
    for _ in 0..limit {
        h.frame();
        if h.game.mode == Mode::Credits {
            break;
        }
    }
    h
}

#[test]
fn the_autoplayer_frees_the_wharf_and_walks_out() {
    let h = play("wharf-e2e");
    let st = h.game.ctx.state_ref();
    let open: Vec<&str> = cpl_logic::signals::wharf::sites().iter().filter(|s| !st.flag(s.done)).map(|s| s.id).collect();
    assert!(st.flag(MAST), "the mast is still theirs after {} frames; unfinished: {open:?}; objective {:?}", h.report.frames, st.data.objective);
    assert!(st.flag(DIVE_INTERCEPTOR) && st.flag(DIVE_STATIC), "both dives broken");
    assert!(st.flag(END));
    assert_eq!(h.game.mode, Mode::Credits);
    assert_eq!(st.data.scene, MapId::Wharf);
    assert!(h.game.credits.ending.as_ref().is_some_and(|(t, _)| t.contains("WHARF")));
    assert!(open.is_empty(), "{open:?}");
    // the route only ever went to spawns that exist, and never to the city
    assert!(h.report.gotos.iter().all(|m| *m == MapId::Wharf));
    let secs = h.report.frames as f32 / 30.0;
    assert!(secs < 25.0 * 60.0, "took {secs:.0} s of game time");
}

#[test]
fn a_second_seed_plays_through_too() {
    let h = play("another harbour");
    assert!(h.game.ctx.state_ref().flag(END));
}

#[test]
#[ignore]
fn report() {
    let h = play("wharf-e2e");
    let st = h.game.ctx.state_ref();
    eprintln!("frames {} = {:.0} s", h.report.frames, h.report.frames as f32 / 30.0);
    eprintln!("interactions {:?}", h.report.interactions);
    eprintln!("spawns {:?}", h.report.spawns);
    eprintln!("cards: {:?}", st.data.cards.iter().map(|c| (&c.name, &c.note)).collect::<Vec<_>>());
    eprintln!("flags: {:?}", st.data.flags.keys().collect::<Vec<_>>());
    eprintln!("trace {} flagged {}", st.trace, st.data.stats.flagged);
    eprintln!("ending {:?}", h.game.credits.ending);
}
