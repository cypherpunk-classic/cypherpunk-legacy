//! The Dragnet, frame by frame: which sensors see you, how fast your trace
//! rises (the sum of their gains, times your opsec), how fast it falls, and
//! when it flags you. The engine calls [`GameState::dragnet`] every frame
//! you are free to walk; the story's `flagged` script handles the rest.

use crate::maps::{MapDef, Sensor};
use crate::state::GameState;

/// Trace lost per second outside every field.
pub const DECAY: f32 = 14.0;
/// Trace at which an agent stops you.
pub const FLAG_AT: f32 = 100.0;

/// What one frame of the Dragnet did.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tick {
    /// sensors that see you right now
    pub seen: Vec<&'static str>,
    /// the trace just reached [`FLAG_AT`]
    pub flagged: bool,
}

impl GameState {
    /// The sensors switched on right now (by story flags).
    pub fn active_sensors<'a>(&'a self, map: &'a MapDef) -> impl Iterator<Item = &'a Sensor> + 'a {
        map.sensors.iter().filter(|s| self.cond(s.show_if))
    }

    /// Advance the trace by `dt` seconds with the player's body at (x, y) at
    /// sensor time `t`. With the Dragnet turned off in the settings, sensors
    /// still see you (so the fields light up) but the trace only falls.
    pub fn dragnet(&mut self, map: &MapDef, x: f32, y: f32, t: f32, dt: f32) -> Tick {
        let seen: Vec<&Sensor> = self.active_sensors(map).filter(|s| s.sees(x, y, t)).collect();
        let gain: f32 = seen.iter().map(|s| s.gain).sum();
        let ids = seen.iter().map(|s| s.id).collect();
        if gain > 0.0 && self.settings.dragnet {
            self.trace += gain * self.opsec_factor() * dt;
        } else {
            self.trace = (self.trace - DECAY * dt).max(0.0);
        }
        let flagged = self.trace >= FLAG_AT;
        if flagged {
            self.trace = FLAG_AT;
        }
        Tick { seen: ids, flagged }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maps::{SensorKind, map};
    use crate::state::MapId;

    fn city_with(state: &mut GameState) -> MapDef {
        state.new_game("t", 0, Some("dragnet"));
        state.set("city.arrived", true);
        map(MapId::City)
    }

    #[test]
    fn lingering_in_a_field_gets_you_flagged_and_tools_slow_it_down() {
        let mut s = GameState::new();
        let m = city_with(&mut s);
        let wharf = m.sensors.iter().find(|x| x.id == "mast-wharf").unwrap().clone();
        let mut secs = 0.0;
        while !s.dragnet(&m, wharf.x, wharf.y, 0.0, 0.1).flagged {
            secs += 0.1;
            assert!(secs < 60.0);
        }
        let bare = secs;
        assert!((4.0..6.0).contains(&bare), "a mast flags a bare phone in about 4.5 s, took {bare}");
        // leaving the field lets the trace fall
        let far = (wharf.x, wharf.y - 200.0);
        for _ in 0..20 {
            s.dragnet(&m, far.0, far.1, 0.0, 0.1);
        }
        assert!(s.trace < FLAG_AT - 20.0);
        // with the messenger, a Faraday pouch and a wallet it takes much longer
        s.trace = 0.0;
        s.give("sealed-messenger");
        s.give("faraday");
        s.set("wallet.open", true);
        let mut secs = 0.0;
        while !s.dragnet(&m, wharf.x, wharf.y, 0.0, 0.1).flagged {
            secs += 0.1;
        }
        assert!(secs > bare * 2.0, "opsec should more than double the time: {secs} vs {bare}");
    }

    #[test]
    fn cameras_sweep_and_taps_go_dark_after_the_clipper() {
        let mut s = GameState::new();
        let m = city_with(&mut s);
        let cam = m.sensors.iter().find(|x| matches!(x.kind, SensorKind::Camera { .. })).unwrap();
        let SensorKind::Camera { range, .. } = cam.kind else { unreachable!() };
        // a point straight down the camera's centre line, half its range out
        let a = cam.facing(0.0);
        let (px, py) = (cam.x + a.cos() * range * 0.5, cam.y + a.sin() * range * 0.5);
        assert!(cam.sees(px, py, 0.0));
        // a quarter period later it has swept away
        let SensorKind::Camera { period, .. } = cam.kind else { unreachable!() };
        assert!(!cam.sees(px, py, period * 0.25));
        // escrow taps are on until the LEAF is broken
        let taps = |s: &GameState| s.active_sensors(&m).filter(|x| x.id.starts_with("tap-")).count();
        assert_eq!(taps(&s), 2);
        s.set("ch4.clipper", true);
        assert_eq!(taps(&s), 0);
        // and the Dragnet is off after the broadcast
        s.set("end.done", true);
        assert_eq!(s.active_sensors(&m).count(), 0);
    }

    #[test]
    fn story_only_mode_never_flags() {
        let mut s = GameState::new();
        let m = city_with(&mut s);
        s.settings.dragnet = false;
        let mast = m.sensors.iter().find(|x| x.id == "mast-wharf").unwrap().clone();
        for _ in 0..1000 {
            let t = s.dragnet(&m, mast.x, mast.y, 0.0, 0.1);
            assert!(!t.flagged);
            assert_eq!(t.seen, vec!["mast-wharf"]);
        }
        assert_eq!(s.trace, 0.0);
    }
}
