//! The campaign map, end to end: the harbour scores its stars when the mast
//! falls, the map opens and draws both tabs, a star buys kit, and a replay
//! of the finished Wharf runs on fresh pads in a sandbox, then hands the
//! real save back with the stars it won.

use cpl_logic::campaign::{self, OPSEC};
use cpl_logic::signals::wharf::{END, MAST};
use cpl_logic::state::MapId;
use cpl_logic::ui::campaign::Tab;
use cpl_logic::walkthrough::Headless;

fn until(h: &mut Headless, frames: u32, done: impl Fn(&Headless) -> bool) -> bool {
    for _ in 0..frames {
        if done(h) {
            return true;
        }
        h.frame();
    }
    done(h)
}

fn back_in_city(h: &Headless) -> bool {
    h.game.current_map() == MapId::City && h.game.district.is_none() && h.game.idle()
}

#[test]
fn stars_upgrades_and_a_replay_of_the_wharf() {
    let mut h = Headless::wharf("campaign-e2e");
    assert!(until(&mut h, 60 * 60 * 30, |h| h.game.ctx.state_ref().flag(END) && back_in_city(h)), "the harbour never finished");
    let (stars, seed, flagged) = {
        let st = h.game.ctx.state_ref();
        (campaign::stars(&st, "wharf"), st.data.seed.clone(), st.data.stats.flagged)
    };
    assert!(stars >= 2, "the autoplayer reads no notes, so it earns at least two stars (flagged {flagged} times)");
    assert_eq!(stars == 3, flagged == 0);
    {
        let st = h.game.ctx.state_ref();
        assert_eq!(campaign::stars(&st, "interceptor"), 1);
        assert_eq!(campaign::stars(&st, "static"), 1);
        assert!(campaign::replayable(&st, MapId::Wharf));
    }

    // the map opens on both tabs
    h.game.open_campaign();
    assert!(h.game.campaign.is_some() && h.game.modal_open());
    h.frame();
    h.game.campaign.as_mut().unwrap().tab = Tab::Upgrades;
    h.frame();
    h.game.campaign = None;

    // a star buys the first level of opsec
    {
        let mut st = h.game.ctx.state();
        assert!(campaign::buy(&mut st, OPSEC));
    }

    // replay the harbour: a sandbox with its own pads
    h.game.campaign_go("wharf");
    assert!(until(&mut h, 600, |h| h.game.current_map() == MapId::Wharf && h.game.district.is_some()));
    {
        let st = h.game.ctx.state_ref();
        assert!(st.data.replay_of.is_some());
        assert_ne!(st.data.seed, seed);
        assert!(!st.flag(MAST), "the mast is theirs again");
        assert_eq!(campaign::level(&st, OPSEC), 1, "kit comes along");
    }
    assert!(until(&mut h, 60 * 60 * 30, |h| h.game.ctx.state_ref().data.replay_of.is_none() && back_in_city(h)), "the replay never came home");
    let st = h.game.ctx.state_ref();
    assert_eq!(st.data.seed, seed, "the real save is back");
    assert!(st.flag(END) && st.flag(MAST));
    assert_eq!(st.data.stages["wharf"].replays, 1);
    assert!(st.data.stages["wharf"].clears >= 2);
    assert!(campaign::stars(&st, "wharf") >= stars);
    assert_eq!(campaign::level(&st, OPSEC), 1);
    let mara = cpl_logic::maps::map(MapId::City).spawn("mara");
    let pos = st.data.pos.expect("a position to come back to");
    assert!((pos.x, pos.y) == (mara.x, mara.y), "back in front of Mara");
}

#[test]
fn the_campaign_map_travels_between_quarters() {
    let mut h = Headless::new("t", "campaign-travel");
    h.play_until(Some("ch1.harbour")).unwrap();
    assert!(until(&mut h, 2000, |h| h.game.idle()));
    // from the city straight into the harbour, then back up to the radio shack
    if h.game.current_map() != MapId::Wharf {
        h.game.campaign_go("wharf");
        assert!(until(&mut h, 600, |h| h.game.current_map() == MapId::Wharf && h.game.idle()));
    }
    h.game.campaign_go("radio");
    assert!(until(&mut h, 600, |h| h.game.current_map() == MapId::City && h.game.idle()));
    assert_eq!(h.report.spawns.last().map(String::as_str), Some("safe-wharf"));
    // a road the story has not opened goes nowhere
    h.game.campaign_go("mint");
    assert!(until(&mut h, 60, |h| h.game.idle()));
    assert_eq!(h.game.current_map(), MapId::City);
    assert_eq!(h.report.spawns.last().map(String::as_str), Some("safe-wharf"));
}

#[test]
fn the_story_scores_a_quarter_and_its_gauntlet_replays_it() {
    let mut h = Headless::new("t", "campaign-gauntlet");
    h.play_until(Some("ch1.harbour")).unwrap();
    assert!(until(&mut h, 2000, |h| h.game.idle()));
    let (radio, cabin, seed) = {
        let st = h.game.ctx.state_ref();
        (campaign::stars(&st, "radio"), campaign::stars(&st, "cabin"), st.data.seed.clone())
    };
    assert!(radio >= 1 && cabin >= 1, "the prologue and the radio shack scored when their chapters closed: {cabin} {radio}");
    // the gauntlet: both of the shack's terminals again, solved by the autoplayer
    h.game.replay_node("radio");
    assert!(until(&mut h, 200, |h| h.game.terminal.is_some()), "the first terminal never opened");
    assert!(h.game.ctx.state_ref().data.replay_of.is_some());
    let mut seen = Vec::new();
    assert!(
        until(&mut h, 30 * 60 * 10, |h| {
            h.game.ctx.state_ref().data.replay_of.is_none() && h.game.idle()
        }),
        "the gauntlet never came home"
    );
    for p in &h.report.puzzles {
        if p.starts_with("ch1.") && !seen.contains(p) {
            seen.push(p.clone());
        }
    }
    let st = h.game.ctx.state_ref();
    assert_eq!(st.data.seed, seed);
    assert!(st.data.flags.get(campaign::GAUNTLET).is_none());
    assert_eq!(st.data.stages["radio"].replays, 1);
    assert!(campaign::stars(&st, "radio") >= radio);
    assert!(h.game.stage_card.as_ref().is_some_and(|(r, _)| r.stage == "radio" && r.score.cleared()), "the card comes home with you");
    assert_eq!(seen, ["ch1.otp", "ch1.ctr"]);
}

#[test]
fn walking_away_from_a_gauntlet_ends_it_with_nothing_scored() {
    let mut h = Headless::new("t", "campaign-quit");
    h.play_until(Some("ch1.harbour")).unwrap();
    assert!(until(&mut h, 2000, |h| h.game.idle()));
    let before = h.game.ctx.state_ref().data.stages["radio"].clone();
    h.game.auto = false;
    h.game.replay_node("radio");
    assert!(until(&mut h, 200, |h| h.game.terminal.is_some()));
    h.game.close_terminal();
    assert!(until(&mut h, 600, |h| h.game.ctx.state_ref().data.replay_of.is_none() && h.game.idle()));
    let st = h.game.ctx.state_ref();
    assert_eq!(st.data.stages["radio"].stars, before.stars);
    assert_eq!(st.data.stages["radio"].clears, before.clears);
}

/// Play the Wharf until the mast is yours and the street is quiet.
fn to_the_mast(seed: &str) -> Headless {
    let mut h = Headless::wharf(seed);
    assert!(until(&mut h, 60 * 60 * 30, |h| h.game.ctx.state_ref().flag(MAST) && h.game.idle()), "the mast never fell");
    h
}

#[test]
fn the_lamplighter_falls_to_prediction_and_comes_back_for_a_rematch() {
    let mut h = to_the_mast("campaign-boss");
    assert!(h.game.interact("w.dive.lamplighter"));
    assert!(until(&mut h, 600, |h| h.game.district.as_ref().is_some_and(|d| d.dive.is_some())), "the fight never started");
    assert!(until(&mut h, 30 * 60 * 10, |h| h.game.ctx.state_ref().flag(cpl_logic::dive::lamplighter::DIVE_LAMPLIGHTER)), "the lamp never went dark");
    {
        let st = h.game.ctx.state_ref();
        assert_eq!(campaign::stars(&st, "lamplighter"), 3, "the bot predicts every beat: no hits, never traced");
        assert!(st.flag("w.lamplighter.met"));
    }
    assert!(h.game.stage_card.as_ref().is_some_and(|(r, _)| r.stage == "lamplighter" && r.headline == "BROKEN"));
    // walk out, then fight it again from the map
    assert!(until(&mut h, 60 * 60 * 5, |h| h.game.ctx.state_ref().flag(END) && back_in_city(h)));
    let seed = h.game.ctx.state_ref().data.seed.clone();
    h.game.campaign_go("lamplighter");
    assert!(until(&mut h, 600, |h| h.game.current_map() == MapId::Wharf && h.game.idle()));
    {
        let st = h.game.ctx.state_ref();
        assert!(st.data.replay_of.is_some());
        assert!(st.flag(MAST) && !st.flag(cpl_logic::dive::lamplighter::DIVE_LAMPLIGHTER));
    }
    assert!(campaign::settle(&mut h.game.ctx.state(), MapId::Wharf).is_none(), "a rematch does not score the district");
    assert!(h.game.interact("w.dive.lamplighter"));
    assert!(until(&mut h, 30 * 60 * 10, |h| h.game.ctx.state_ref().flag(cpl_logic::dive::lamplighter::DIVE_LAMPLIGHTER)));
    h.game.campaign = None;
    {
        let mut st = h.game.ctx.state();
        st.set(END, true);
    }
    h.game.end_replay();
    assert!(until(&mut h, 600, |h| back_in_city(h)));
    let st = h.game.ctx.state_ref();
    assert_eq!(st.data.seed, seed);
    assert_eq!(st.data.stages["lamplighter"].replays, 1);
    assert_eq!(st.data.stages["wharf"].replays, 0, "a rematch is not a district replay");
}

#[test]
fn rest_banks_your_cards_and_a_fall_drops_the_rest_where_you_fell() {
    let mut h = to_the_mast("campaign-souls");
    // drive it by hand from here (dialogue by key presses), so nobody walks out
    h.game.auto = false;
    h.manual = true;
    let n0 = h.game.ctx.state_ref().data.cards.len();
    assert!(n0 > 0);
    // rest: the trace clears, every card in the pocket is banked, and the shop opens
    h.game.ctx.state().trace = 60.0;
    assert!(h.interact("rest.wharf"));
    assert!(h.game.campaign.as_ref().is_some_and(|v| v.at_rest), "resting opens the kit");
    h.game.campaign = None;
    {
        let st = h.game.ctx.state_ref();
        assert_eq!(st.trace, 0.0);
        assert_eq!(st.data.banked.len(), n0);
        assert!(campaign::loose(&st).is_empty());
    }
    // a new rubbing after resting is loose
    {
        let mut st = h.game.ctx.state();
        let mut c = st.data.cards.iter().find(|c| c.kind != cpl_logic::signals::CardKind::Pad).cloned().expect("a rubbing or a cut");
        c.id = 9_999;
        c.name = "LOOSE".into();
        st.data.cards.push(c);
    }
    h.game.flagged(400.0, 300.0);
    assert!(until(&mut h, 3000, |h| h.game.idle()));
    {
        let st = h.game.ctx.state_ref();
        assert_eq!(st.data.cards.len(), n0, "only the loose card fell");
        let d = st.data.dropped.as_ref().expect("dropped");
        assert_eq!((d.map, d.cards.len(), d.x, d.y), (MapId::Wharf, 1, 400.0, 300.0));
    }
    h.game.pick_up_at(100.0, 100.0);
    assert!(h.game.ctx.state_ref().data.dropped.is_some(), "too far to pick up");
    h.game.pick_up_at(405.0, 296.0);
    let st = h.game.ctx.state_ref();
    assert!(st.data.dropped.is_none());
    assert!(st.data.cards.iter().any(|c| c.id == 9_999));
}
