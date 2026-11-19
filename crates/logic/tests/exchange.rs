//! The Exchange, end to end: the autoplayer arrives in the tram square with
//! the Wharf behind it, works every panel through the real game (scripts,
//! windows, egui), breaks LOGJAM, makes the call nobody else hears and walks
//! out north.

use cpl_logic::game::Mode;
use cpl_logic::lines::exchange::{self as xch, sites};
use cpl_logic::state::MapId;
use cpl_logic::walkthrough::Headless;

fn play(seed: &str) -> Headless {
    let mut h = Headless::exchange(seed);
    let limit = 40 * 60 * 30; // forty minutes of game time at 30 fps
    for _ in 0..limit {
        h.frame();
        if h.game.mode == Mode::Credits {
            break;
        }
    }
    h
}

#[test]
fn the_autoplayer_frees_the_exchange_and_walks_out() {
    let h = play("exchange-e2e");
    let st = h.game.ctx.state_ref();
    let open: Vec<&str> = sites().iter().filter(|s| !st.flag(s.done)).map(|s| s.id).collect();
    assert!(st.flag(xch::FREE), "the Exchange is still theirs after {} frames; unfinished: {open:?}; objective {:?}; trace {}", h.report.frames, st.data.objective, st.trace);
    assert!(open.is_empty(), "{open:?}");
    assert!(st.flag(xch::DIVE_LOGJAM) && st.flag(xch::CODE) && st.flag(xch::ESCROW_SEEN));
    assert!(st.flag(xch::END));
    assert_eq!(h.game.mode, Mode::Credits);
    assert_eq!(st.data.scene, MapId::Exchange);
    assert!(h.game.credits.ending.as_ref().is_some_and(|(t, _)| t.contains("EXCHANGE")));
    assert!(h.report.gotos.iter().all(|m| *m == MapId::Exchange), "{:?}", h.report.gotos);
    let secs = h.report.frames as f32 / 30.0;
    assert!(secs < 20.0 * 60.0, "took {secs:.0} s of game time");
}

#[test]
fn another_seed_plays_through_too() {
    let h = play("a different exchange");
    assert!(h.game.ctx.state_ref().flag(xch::END));
}

#[test]
#[ignore]
fn report() {
    let h = play("exchange-e2e");
    let st = h.game.ctx.state_ref();
    eprintln!("frames {} = {:.0} s", h.report.frames, h.report.frames as f32 / 30.0);
    eprintln!("interactions {:?}", h.report.interactions);
    eprintln!("spawns {:?}", h.report.spawns);
    eprintln!("flags: {:?}", st.data.flags.keys().filter(|k| k.starts_with("x.")).collect::<Vec<_>>());
    eprintln!("trace {} flagged {}", st.trace, st.data.stats.flagged);
    if let Some(d) = &h.game.district
        && let Some(l) = &d.lines
    {
        for p in &l.panels {
            eprintln!("--- {} ---", p.id);
            for x in p.log.iter().rev().take(12).rev() {
                eprintln!("{:6.1} {:>12}: {}", x.t, x.who, x.text);
            }
        }
    }
}
