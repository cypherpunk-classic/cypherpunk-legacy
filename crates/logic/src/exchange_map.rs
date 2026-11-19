//! The Exchange, the second district rebuilt for v4: the telephone exchange
//! north of the Wharf, where every call in the city goes through the
//! Directorate's desk.
//!
//! 56 x 38 tiles. You come up from the Wharf into the tram square; every way
//! on is held by something a line controls: a gate, two guards with booms
//! who take their orders by phone, and a camera.
//!
//! ```text
//!   annex  |  THE EXCHANGE (its back on the yard)  | block | TOWER YARD  exit
//!  --------+---------------------------------------+-------+--cam gate---------
//!  CALL BOX ROW  boom2  THE YARD (switchboard, Felix)  boom3  CAMERA STREET
//!  --square gate----------------------------------------------------------------
//!   depot   junction box        TRAM SQUARE (agent's round)       kiosk
//! ```

use std::collections::HashMap;

use crate::Facing;
use crate::lines::exchange::{self as flags, POST2_AT, POST3_AT};
use crate::maps::{Ambient, CatMode, Light, MapDef, MapObject, NpcDef, Rect, Sensor, SensorKind, Spawn, Zone};
use crate::state::MapId;

pub const W: usize = 56;
pub const H: usize = 38;

type Grid = Vec<Vec<&'static str>>;

const fn t(n: f32) -> f32 {
    n * 16.0
}

/// Bottom-centre of something standing in the middle of tile (tx, ty).
fn on(tx: f32, ty: f32) -> (f32, f32) {
    (t(tx) + 8.0, t(ty) + 14.0)
}

fn fill(g: &mut Grid, x: usize, y: usize, w: usize, h: usize, f: impl Fn(usize, usize) -> &'static str) {
    for (yy, row) in g.iter_mut().enumerate().take((y + h).min(H)).skip(y) {
        for (xx, cell) in row.iter_mut().enumerate().take((x + w).min(W)).skip(x) {
            *cell = f(xx, yy);
        }
    }
}

fn tiles() -> Grid {
    let mut g: Grid = vec![vec!["sidewalk"; W]; H];
    for (y, row) in g.iter_mut().enumerate() {
        for (x, cell) in row.iter_mut().enumerate() {
            if (x * 29 + y * 13) % 19 == 0 {
                *cell = "sidewalk2";
            }
        }
    }
    let checker = |x: usize, y: usize| if (x + y) % 2 == 1 { "plaza2" } else { "plaza" };
    // the Exchange and its neighbours (out of reach)
    fill(&mut g, 1, 1, 39, 11, checker);
    // the tower yard
    fill(&mut g, 41, 1, 14, 11, |x, y| if (x * 7 + y * 3) % 11 == 0 { "asphalt2" } else { "metal" });
    // call box row: an alley
    fill(&mut g, 1, 13, 13, 11, |x, _| if (x * 13) % 7 == 0 { "asphalt2" } else { "asphalt" });
    // the yard behind the Exchange
    fill(&mut g, 15, 13, 21, 11, checker);
    // camera street
    fill(&mut g, 37, 13, 18, 11, |_, y| if y >= 19 { "asphalt" } else { "sidewalk" });
    fill(&mut g, 37, 21, 18, 1, |x, _| if x % 3 == 0 { "dash_h" } else { "asphalt" });
    // the tram square, with its road and tracks
    fill(&mut g, 1, 25, 54, 12, checker);
    fill(&mut g, 1, 32, 54, 3, |_, _| "asphalt");
    fill(&mut g, 1, 33, 54, 1, |x, _| if x % 3 == 1 { "dash_h" } else { "asphalt" });
    for (x, y) in [(18, 28), (27, 29), (38, 27), (22, 36), (45, 36)] {
        g[y][x] = "plazaglow";
    }
    fill(&mut g, 46, 26, 8, 5, |x, y| if (x + y) % 5 == 0 { "flowers" } else { "grass" });
    // the edge of the map
    fill(&mut g, 0, 0, W, 1, |_, _| "hedge");
    fill(&mut g, 0, 0, 1, H, |_, _| "hedge");
    fill(&mut g, W - 1, 0, 1, H, |_, _| "hedge");
    fill(&mut g, 0, H - 1, W, 1, |_, _| "hedge");
    // fences between rooms
    fill(&mut g, 1, 12, 54, 1, |_, _| "fence");
    fill(&mut g, 40, 1, 1, 11, |_, _| "fence");
    fill(&mut g, 1, 24, 54, 1, |_, _| "fence");
    for x in [14, 36] {
        fill(&mut g, x, 13, 1, 11, |_, _| "fence");
    }
    // gaps the barriers stand in
    fill(&mut g, 7, 24, 2, 1, |_, _| "sidewalk"); // the square gate
    fill(&mut g, 14, 17, 1, 2, |_, _| "asphalt"); // post two's boom
    fill(&mut g, 36, 17, 1, 2, |_, _| "sidewalk"); // post three's boom
    fill(&mut g, 47, 12, 2, 1, |_, _| "metal"); // the tower gate
    fill(&mut g, 43, 0, 2, 1, |_, _| "metal"); // the road north
    g
}

fn prop(id: &'static str, tex: &'static str, tx: f32, ty: f32, bw: f32, bh: f32) -> MapObject {
    let (x, y) = on(tx, ty);
    MapObject::new(id, tex, x, y).body(bw, bh)
}

fn jackbox(id: &'static str, act: &'static str, tx: f32, ty: f32, label: &'static str) -> MapObject {
    let (x, y) = on(tx, ty);
    MapObject::new(id, "obj-jackbox", x, y).anim().body(12.0, 5.0).interact(act, label).radius(22.0).light(Light::new("glow-warm", 0.4, 0.3, -18.0))
}

fn gate(id: &'static str, dir: char, tx: f32, ty: f32, cond: &'static str, act: &'static str) -> MapObject {
    let tex = if dir == 'h' { "obj-barrier_h" } else { "obj-barrier_v" };
    MapObject::new(id, tex, t(tx) + 8.0, t(ty) + 16.0).anim().body(16.0, 16.0).show_if(cond).interact(act, "Look at the gate").light(Light::new("glow-magenta", 0.5, 0.5, -8.0).pulse())
}

fn lamp(id: &'static str, tx: f32, ty: f32) -> MapObject {
    let (x, y) = on(tx, ty);
    MapObject::new(id, "obj-lamp", x, y).body(6.0, 4.0).light(Light::new("glow-warm", 1.1, 0.45, -34.0))
}

/// A call box on its pole: a party's end of a line (`x-box-<party>`; it sparks when it rings).
fn callbox(id: &'static str, tx: f32, ty: f32) -> MapObject {
    prop(id, "obj-payphone", tx, ty, 10.0, 4.0).anim()
}

pub fn exchange() -> MapDef {
    let objects = vec![
        // --- the north side, out of reach ---------------------------------------------
        MapObject::new("x-annex", "bld-block_a", t(7.0) + 8.0, t(12.0)).anim().body(112.0, 30.0),
        MapObject::new("x-exchange", "bld-exchange", t(25.0) + 8.0, t(12.0)).anim().body(160.0, 30.0),
        MapObject::new("x-block", "bld-block_b", t(35.0), t(12.0)).anim().body(112.0, 30.0),
        // --- the tram square (start) --------------------------------------------------
        MapObject::new("x-depot", "bld-warehouse", t(5.5) + 8.0, t(31.0)).anim().body(112.0, 30.0),
        callbox("x-box-depot", 11.5, 30.0),
        jackbox("x-jb-junction", "x.panel.junction", 10.5, 25.0, "Open the junction box"),
        gate("x-gate-sq1", 'h', 7.0, 24.0, "!x.gate.square", "x.gate.square"),
        gate("x-gate-sq2", 'h', 8.0, 24.0, "!x.gate.square", "x.gate.square"),
        prop("x-kiosk", "obj-kiosk", 31.0, 27.0, 26.0, 8.0).anim().light(Light::new("glow-warm", 0.8, 0.4, -20.0)),
        prop("x-bench1", "obj-bench", 20.0, 27.0, 28.0, 8.0),
        prop("x-bench2", "obj-bench", 38.0, 30.6, 28.0, 8.0),
        prop("x-statue", "obj-statue", 49.5, 28.0, 18.0, 8.0),
        prop("x-hydrant", "obj-hydrant", 14.0, 26.0, 6.0, 4.0),
        prop("x-trash1", "obj-trash", 24.5, 26.0, 8.0, 5.0),
        prop("x-tree1", "obj-tree", 47.0, 26.5, 10.0, 6.0).anim(),
        prop("x-tree2", "obj-tree", 52.5, 29.5, 10.0, 6.0).anim(),
        lamp("x-lamp1", 16.0, 31.0),
        lamp("x-lamp2", 34.0, 31.0),
        lamp("x-lamp3", 26.0, 36.0),
        // --- call box row ---------------------------------------------------------------
        jackbox("x-jb-callbox", "x.panel.callbox", 4.0, 13.0, "Open the junction box"),
        callbox("x-box-post2", 13.0, 15.6),
        gate("x-boom2a", 'v', 14.0, 17.0, "!x.post2", "x.gate.boom2"),
        gate("x-boom2b", 'v', 14.0, 18.0, "!x.post2", "x.gate.boom2"),
        prop("x-trash2", "obj-trash", 2.0, 17.0, 8.0, 5.0),
        prop("x-mailbox", "obj-mailbox", 9.0, 13.2, 8.0, 5.0),
        prop("x-boxes1", "obj-boxes", 2.5, 23.0, 14.0, 8.0),
        lamp("x-lamp4", 8.0, 20.0),
        // --- the yard ---------------------------------------------------------------------
        MapObject::new("x-switchboard", "obj-switchboard", t(24.0) + 8.0, t(13.0) + 14.0).anim().body(28.0, 6.0).interact("x.panel.switchboard", "Use the switchboard").radius(24.0).light(Light::new("glow-warm", 0.7, 0.4, -22.0).pulse()),
        jackbox("x-jb-yard", "x.panel.yardbox", 31.0, 13.0, "Open the junction box"),
        callbox("x-box-post3", 35.0, 15.6),
        gate("x-boom3a", 'v', 36.0, 17.0, "!x.post3", "x.gate.boom3"),
        gate("x-boom3b", 'v', 36.0, 18.0, "!x.post3", "x.gate.boom3"),
        prop("x-crate1", "obj-crate", 17.0, 22.0, 16.0, 10.0),
        prop("x-crate2", "obj-crate", 18.2, 22.6, 16.0, 10.0),
        prop("x-bench3", "obj-bench", 25.0, 20.5, 28.0, 8.0),
        lamp("x-lamp5", 20.0, 16.0),
        lamp("x-lamp6", 33.0, 21.0),
        // --- camera street ------------------------------------------------------------------
        jackbox("x-jb-cam", "x.panel.cambox", 40.0, 20.0, "Open the junction box"),
        callbox("x-box-camera", 51.0, 13.2),
        MapObject::new("x-cam", "obj-camera", t(52.5) + 8.0, t(13.0) + 12.0).anim().body(6.0, 4.0).show_if("!x.cam.off").light(Light::new("glow-red", 0.3, 0.45, -26.0).pulse()),
        MapObject::new("x-cam-off", "obj-camera", t(52.5) + 8.0, t(13.0) + 12.0).body(6.0, 4.0).show_if("x.cam.off"),
        gate("x-gate-tw1", 'h', 47.0, 12.0, "!x.cam.off", "x.gate.tower"),
        gate("x-gate-tw2", 'h', 48.0, 12.0, "!x.cam.off", "x.gate.tower"),
        prop("x-vending", "obj-vending", 38.0, 14.0, 14.0, 6.0).anim(),
        prop("x-hydrant2", "obj-hydrant", 45.0, 18.0, 6.0, 4.0),
        lamp("x-lamp7", 43.0, 15.0),
        lamp("x-lamp8", 50.0, 22.6),
        // --- the tower yard -------------------------------------------------------------------
        MapObject::new("x-tower", "bld-tower", t(50.0) + 8.0, t(6.0)).anim().body(96.0, 28.0),
        jackbox("x-jb-tower", "x.panel.towerbox", 44.0, 4.0, "Open the junction box"),
        callbox("x-box-deskt", 45.5, 4.2),
        prop("x-locker-door", "obj-locker", 50.5, 6.6, 12.0, 5.0).interact("x.locker.door", "Try the tower door").frame_if(&[(flags::TOWER, 1)]),
        prop("x-cabinet", "obj-cabinet", 42.5, 9.0, 16.0, 6.0).anim().interact("x.dive.logjam", "Play LOGJAM").light(Light::new("glow-violet", 0.7, 0.5, -24.0).pulse()),
        prop("x-dish", "obj-dish", 53.0, 9.5, 18.0, 8.0).anim(),
        prop("x-server", "obj-server", 45.0, 10.0, 12.0, 6.0).anim().light(Light::new("glow-green", 0.4, 0.3, -16.0)),
        gate("x-gate-north1", 'h', 43.0, 0.0, "!x.free", "x.gate.north"),
        gate("x-gate-north2", 'h', 44.0, 0.0, "!x.free", "x.gate.north"),
        lamp("x-lamp9", 47.0, 9.5),
    ];

    let npc = |id: &'static str, ch: &'static str, tx: f32, ty: f32, f: Facing, act: Option<&'static str>, label: Option<&'static str>| {
        let (x, y) = on(tx, ty);
        NpcDef { id, char: ch, x, y, facing: f, interact: act, label, show_if: None, wander: None, hologram: false }
    };
    let at = |id: &'static str, ch: &'static str, (x, y): (f32, f32), f: Facing| NpcDef { id, char: ch, x, y, facing: f, interact: None, label: None, show_if: None, wander: None, hologram: false };
    let roam = |x: f32, y: f32, w: f32, h: f32| Some(Rect { x: t(x), y: t(y), w: t(w), h: t(h) });
    let npcs = vec![
        NpcDef { show_if: Some("!x.post2"), ..npc("x-felix", "felix", 32.5, 27.6, Facing::Down, Some("x.felix"), Some("Talk to Felix")) },
        NpcDef { show_if: Some("x.post2"), ..npc("x-felix2", "felix", 27.5, 14.4, Facing::Down, Some("x.felix"), Some("Talk to Felix")) },
        // the agent is walked by the district's clock, not by the engine
        NpcDef { show_if: Some("!x.free"), ..npc("x-agent", "agent", 13.0, 28.2, Facing::Right, None, None) },
        NpcDef { show_if: Some("!x.post2"), ..at("x-post2", "agent", POST2_AT, Facing::Left) },
        NpcDef { show_if: Some("x.post2"), ..npc("x-post2-west", "agent", 2.5, 20.6, Facing::Right, None, None) },
        NpcDef { show_if: Some("!x.post3"), ..at("x-post3", "agent", POST3_AT, Facing::Left) },
        NpcDef { wander: roam(14.0, 34.0, 10.0, 2.0), ..npc("x-conductor", "citizen1", 16.0, 35.0, Facing::Up, Some("x.citizen.1"), None) },
        NpcDef { wander: roam(36.0, 25.6, 8.0, 2.0), ..npc("x-reader", "citizen3", 38.0, 26.0, Facing::Down, Some("x.citizen.2"), None) },
        NpcDef { wander: roam(17.0, 15.0, 6.0, 3.0), ..npc("x-linesman", "citizen5", 19.0, 16.0, Facing::Right, Some("x.citizen.3"), None) },
    ];

    let zone = |id: &'static str, x: f32, y: f32, w: f32, h: f32| Zone { id, rect: Rect { x: t(x), y: t(y), w: t(w), h: t(h) } };
    let zones = vec![
        zone("x.z.square", 1.0, 25.0, 54.0, 12.0),
        zone("x.z.row", 1.0, 13.0, 13.0, 11.0),
        zone("x.z.yard", 15.0, 13.0, 21.0, 11.0),
        zone("x.z.street", 37.0, 13.0, 18.0, 11.0),
        zone("x.z.tower", 41.0, 1.0, 14.0, 11.0),
        zone("x.exit", 43.0, 0.0, 2.0, 1.0),
    ];

    let mut spawns = HashMap::new();
    let mut sp = |name: &'static str, tx: f32, ty: f32, facing: Facing| {
        let (x, y) = on(tx, ty);
        spawns.insert(name, Spawn { x, y, facing });
    };
    sp("start", 27.5, 35.0, Facing::Up);
    sp("junction", 10.5, 26.6, Facing::Up);
    sp("callbox", 4.0, 14.6, Facing::Up);
    sp("yard", 31.0, 14.6, Facing::Up);
    sp("switchboard", 24.0, 15.0, Facing::Up);
    sp("camera", 40.0, 21.6, Facing::Up);
    sp("tower", 44.0, 5.6, Facing::Up);
    sp("cellar", 42.5, 10.4, Facing::Up);
    sp("door", 50.5, 7.8, Facing::Up);
    sp("exit", 43.5, 1.0, Facing::Up);

    let cam = objects.iter().find(|o| o.id == "x-cam").expect("camera sprite");
    let sensors = vec![Sensor {
        id: "x-cam",
        kind: SensorKind::Camera { range: 120.0, half_angle: 0.34, center: 3.25, sweep: 0.75, period: 10.0 },
        x: cam.x,
        y: cam.y - 20.0,
        gain: 60.0,
        show_if: Some("!x.cam.off"),
    }];

    MapDef {
        id: MapId::Exchange,
        title: "The Exchange",
        w: W,
        h: H,
        tiles: tiles(),
        objects,
        npcs,
        spawns,
        zones,
        music: "city",
        tint: 0x7a6eb0,
        ambient: Some(Ambient::Motes),
        cat_mode: Some(CatMode::Follow),
        cat_area: None,
        sensors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lines::Lines;
    use crate::lines::exchange::agent_walk;
    use crate::signals::Signals;
    use crate::state::GameState;

    fn solid(m: &MapDef, x: f32, y: f32) -> bool {
        cpl_art::tiles::is_solid(m.tile((x / 16.0).floor() as i32, (y / 16.0).floor() as i32))
    }

    #[test]
    fn every_panel_site_and_spawn_is_on_the_map() {
        let m = exchange();
        let mut st = GameState::new();
        st.new_game("t", 0, Some("map"));
        let lines = Lines::exchange(&st);
        let sig = Signals::exchange(&st);
        let acts = m.interactions();
        for p in &lines.panels {
            assert!(acts.contains(&format!("x.panel.{}", p.id).as_str()), "no object opens panel {}", p.id);
            // every party that places calls has a call box in the street to light up
            if let Some((j, ..)) = p.schedule
                && let crate::lines::End::Line(party) = p.jacks[j].end
            {
                assert!(m.object(&format!("x-box-{party}")).is_some(), "no call box for {party}");
            }
        }
        for s in &sig.sites {
            let sp = m.spawns.get(s.spawn).unwrap_or_else(|| panic!("site {} has no spawn {}", s.id, s.spawn));
            assert!(!solid(&m, sp.x, sp.y - 4.0), "spawn {} is inside something solid", s.spawn);
        }
        for l in &sig.lockers {
            assert!(acts.contains(&format!("x.locker.{}", l.id).as_str()), "no locker {} on the map", l.id);
        }
        assert!(acts.contains(&"x.dive.logjam"));
        for (x, y, _) in &agent_walk().pts {
            assert!(!solid(&m, *x, *y), "agent waypoint ({}, {}) is solid", x / 16.0, y / 16.0);
        }
    }

    /// Flood the walkable tiles from the start with some gates open, and
    /// check that each part of the district is cut off until its line gives way.
    #[test]
    fn gates_hold_until_their_lines_give_way() {
        let m = exchange();
        let mut st = GameState::new();
        st.new_game("t", 0, Some("map"));
        let reach = |st: &GameState| -> Vec<&'static str> {
            let mut blocked = vec![false; m.w * m.h];
            for (i, tile) in m.tiles.iter().flatten().enumerate() {
                blocked[i] = cpl_art::tiles::is_solid(tile);
            }
            for o in &m.objects {
                if o.tex.starts_with("obj-barrier") && st.cond(o.show_if) {
                    let (tx, ty) = ((o.x / 16.0).floor() as usize, ((o.y - 8.0) / 16.0).floor() as usize);
                    blocked[ty * m.w + tx] = true;
                }
            }
            let s = m.spawn("start");
            let mut seen = vec![false; m.w * m.h];
            let mut stack = vec![((s.x / 16.0) as usize, (s.y / 16.0) as usize)];
            while let Some((x, y)) = stack.pop() {
                if x >= m.w || y >= m.h || seen[y * m.w + x] || blocked[y * m.w + x] {
                    continue;
                }
                seen[y * m.w + x] = true;
                stack.extend([(x + 1, y), (x.wrapping_sub(1), y), (x, y + 1), (x, y.wrapping_sub(1))]);
            }
            let mut names: Vec<&'static str> = m.spawns.iter().filter(|(_, s)| seen[((s.y - 4.0) / 16.0) as usize * m.w + (s.x / 16.0) as usize]).map(|(n, _)| *n).collect();
            names.sort_unstable();
            names
        };
        let mut expect: Vec<&'static str> = vec!["junction", "start"];
        assert_eq!(reach(&st), expect);
        for (flag, opens) in [
            (flags::GATE_SQUARE, &["callbox"][..]),
            (flags::POST2, &["switchboard", "yard"][..]),
            (flags::POST3, &["camera"][..]),
            // the exit spawn stands in front of the north gate; the gate itself waits for the last call
            (flags::CAM_OFF, &["cellar", "door", "exit", "tower"][..]),
        ] {
            st.set(flag, true);
            expect.extend(opens);
            expect.sort_unstable();
            assert_eq!(reach(&st), expect, "after {flag}");
        }
        assert_eq!(expect.len(), m.spawns.len(), "every spawn is reachable in the end");
        let north = m.objects.iter().filter(|o| o.id.starts_with("x-gate-north")).count();
        assert_eq!(north, 2);
        assert!(m.objects.iter().filter(|o| o.id.starts_with("x-gate-north")).all(|o| o.show_if == Some("!x.free")));
    }
}
