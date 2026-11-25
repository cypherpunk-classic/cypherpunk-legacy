//! The Wharf, rebuilt as a place where the puzzles stand in the street.
//!
//! 56 x 38 tiles. Rooms are fenced; every gap in a fence is a gate held by a
//! signal lock (or, once, by a guard and a camera).
//!
//! ```text
//!  yard      | FISH MARKET        [ice house] | HARBOUR SHED | warehouses
//!  ----------+---------arch-------------------M--------------+-----------
//!  BOATHOUSE Q  QUAY PLAZA   c0  CUSTOMS YARD  c1   ALLEY (guard)
//!  ==piers===+==============+=================+=====mole (camera)========
//!   1  2  3                                          | gate W
//!                                                    | mast
//! ```

use std::collections::HashMap;

use crate::Facing;
use crate::maps::{Ambient, CatMode, Light, MapDef, MapObject, NpcDef, Rect, Sensor, SensorKind, Spawn, Zone};
use crate::signals::wharf as flags;
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
            if (x * 31 + y * 17) % 23 == 0 {
                *cell = "sidewalk2";
            }
        }
    }
    let checker = |x: usize, y: usize| if (x + y) % 2 == 1 { "plaza2" } else { "plaza" };
    // the yard behind the boathouse
    fill(&mut g, 1, 1, 10, 11, |x, y| {
        if (x * 7 + y * 5) % 9 == 0 {
            "grass2"
        } else if (x * 3 + y * 11) % 17 == 0 {
            "flowers"
        } else {
            "grass"
        }
    });
    fill(&mut g, 12, 1, 22, 11, checker); // fish market
    fill(&mut g, 27, 1, 7, 5, |_, _| "metal"); // ice house
    fill(&mut g, 35, 1, 12, 11, |_, _| "sand"); // harbour master's yard
    fill(&mut g, 48, 1, 7, 11, |_, y| if (y * 11) % 7 == 0 { "asphalt2" } else { "asphalt" });
    fill(&mut g, 20, 13, 12, 11, checker); // quay plaza
    fill(&mut g, 33, 13, 11, 11, |_, _| "metal"); // customs yard
    fill(&mut g, 45, 13, 10, 11, |x, _| if (x * 13) % 7 == 0 { "asphalt2" } else { "asphalt" }); // the alley
    // the waterfront
    fill(&mut g, 1, 24, 54, 1, |_, _| "dock_h");
    fill(&mut g, 0, 25, W, 1, |_, _| "wateredge");
    fill(&mut g, 0, 26, W, 12, |_, _| "water");
    for x in [3, 9, 15] {
        fill(&mut g, x, 25, 2, 9, |_, _| "dock_v");
    }
    fill(&mut g, 48, 25, 4, 7, |_, _| "dock_v"); // the mole
    fill(&mut g, 47, 32, 6, 5, |_, _| "dock_h"); // the mast's platform
    // the edge of the map
    fill(&mut g, 0, 0, W, 1, |_, _| "hedge");
    fill(&mut g, 0, 0, 1, 25, |_, _| "hedge");
    fill(&mut g, W - 1, 0, 1, 25, |_, _| "hedge");
    // fences between rooms
    fill(&mut g, 1, 12, 54, 1, |_, _| "fence");
    for x in [11, 34, 47] {
        fill(&mut g, x, 1, 1, 11, |_, _| "fence");
    }
    for x in [19, 32, 44] {
        fill(&mut g, x, 13, 1, 11, |_, _| "fence");
        g[24][x] = "railing";
    }
    fill(&mut g, 26, 6, 8, 1, |_, _| "fence"); // ice house
    fill(&mut g, 26, 1, 1, 5, |_, _| "fence");
    fill(&mut g, 48, 31, 4, 1, |_, _| "railing"); // across the mole
    // gates (gaps the barriers stand in)
    fill(&mut g, 25, 12, 2, 1, checker); // the market arch, always open
    fill(&mut g, 19, 18, 1, 2, |_, _| "sidewalk"); // Q
    fill(&mut g, 32, 18, 1, 2, |_, _| "metal"); // c0
    fill(&mut g, 44, 18, 1, 2, |_, _| "asphalt"); // c1
    fill(&mut g, 34, 8, 1, 2, |_, _| "sand"); // M
    g[6][30] = "metal"; // ice house
    fill(&mut g, 49, 31, 2, 1, |_, _| "dock_v"); // W
    fill(&mut g, 22, 0, 2, 1, checker); // the way into the city
    g
}

fn board(id: &'static str, act: &'static str, tex: &'static str, tx: f32, ty: f32, label: &'static str) -> MapObject {
    let (x, y) = on(tx, ty);
    MapObject::new(id, tex, x, y).body(8.0, 5.0).interact(act, label).radius(22.0).light(Light::new("glow-warm", 0.4, 0.3, -22.0))
}

fn prop(id: &'static str, tex: &'static str, tx: f32, ty: f32, bw: f32, bh: f32) -> MapObject {
    let (x, y) = on(tx, ty);
    MapObject::new(id, tex, x, y).body(bw, bh)
}

fn gate(id: &'static str, dir: char, tx: f32, ty: f32, cond: &'static str, act: &'static str) -> MapObject {
    let tex = if dir == 'h' { "obj-barrier_h" } else { "obj-barrier_v" };
    MapObject::new(id, tex, t(tx) + 8.0, t(ty) + 16.0).anim().body(16.0, 16.0).show_if(cond).interact(act, "Look at the gate").light(Light::new("glow-magenta", 0.5, 0.5, -8.0).pulse())
}

fn lamp(id: &'static str, tx: f32, ty: f32) -> MapObject {
    let (x, y) = on(tx, ty);
    MapObject::new(id, "obj-lamp", x, y).body(6.0, 4.0).light(Light::new("glow-warm", 1.1, 0.45, -34.0))
}

/// The Directorate's banner until the mast is yours, a nightjar lantern after.
fn banner(n: usize, tx: f32, ty: f32) -> [MapObject; 2] {
    const B: [&str; 8] = ["w-banner1", "w-banner2", "w-banner3", "w-banner4", "w-banner5", "w-banner6", "w-banner7", "w-banner8"];
    const L: [&str; 8] = ["w-lantern1", "w-lantern2", "w-lantern3", "w-lantern4", "w-lantern5", "w-lantern6", "w-lantern7", "w-lantern8"];
    let (x, y) = on(tx, ty);
    [
        MapObject::new(B[n], "obj-banner", x, y).anim().body(4.0, 3.0).show_if("!w.mast").light(Light::new("glow-red", 0.5, 0.3, -18.0)),
        MapObject::new(L[n], "obj-lantern", x, y).anim().body(4.0, 3.0).show_if("w.mast").light(Light::new("glow-warm", 1.2, 0.6, -18.0).flicker()),
    ]
}

pub fn wharf() -> MapDef {
    let mut objects = vec![
        {
            let (x, y) = on(12.0, 19.0);
            MapObject::new("w-lantern", "obj-lantern", x, y).anim().interact("rest.wharf", "Rest at the lantern").radius(18.0).light(Light::new("glow-warm", 0.9, 0.55, -14.0).pulse())
        },
        // --- boathouse and piers (start) ----------------------------------------------
        MapObject::new("w-boathouse", "bld-boathouse", t(5.0) + 8.0, t(17.0)).anim().body(128.0, 34.0),
        prop("w-cabinet", "obj-cabinet", 12.0, 13.0, 16.0, 6.0).anim().interact("w.dive.interceptor", "Play INTERCEPTOR").light(Light::new("glow-violet", 0.7, 0.5, -24.0).pulse()),
        prop("w-locker-kit", "obj-locker", 15.0, 13.0, 12.0, 5.0).interact("w.locker.kit", "Try the locker").frame_if(&[(flags::KIT, 1)]),
        board("sig-a1", "w.board.a1", "obj-sig_amber", 3.5, 31.0, "Look at the lamp board"),
        board("sig-a2", "w.board.a2", "obj-sig_amber", 9.5, 31.0, "Look at the lamp board"),
        board("sig-a3", "w.board.a3", "obj-sig_amber", 15.5, 31.0, "Look at the lamp board"),
        MapObject::new("w-card-gull", "obj-card", t(10.0), t(28.0)).anim().floor().show_if("!w.card.gull").interact("w.pick.gull", "Pick up the card").radius(16.0).light(Light::new("glow-gold", 0.3, 0.5, -3.0).pulse()),
        board("sig-q", "w.board.q", "obj-sig_grey_l", 17.5, 16.0, "Look at the gate's board"),
        prop("w-poster-q", "obj-poster", 16.0, 15.0, 0.0, 0.0),
        gate("w-gate-q1", 'v', 19.0, 18.0, "!w.gate.q", "w.gate.q"),
        gate("w-gate-q2", 'v', 19.0, 19.0, "!w.gate.q", "w.gate.q"),
        prop("w-boat1", "obj-boat", 6.5, 28.0, 0.0, 0.0).anim().depth(-40.0),
        prop("w-boat2", "obj-boat", 12.5, 30.0, 0.0, 0.0).anim().depth(-40.0),
        prop("w-crate1", "obj-crate", 2.0, 22.0, 16.0, 10.0),
        prop("w-barrel1", "obj-barrel", 3.2, 22.0, 12.0, 8.0),
        prop("w-fish1", "obj-fishcrate", 13.0, 23.0, 14.0, 8.0),
        lamp("w-lamp1", 11.0, 17.0),
        lamp("w-lamp2", 6.0, 23.0),
        // --- quay plaza -----------------------------------------------------------------
        MapObject::new("w-radioshack", "bld-radioshack", t(23.0) + 8.0, t(17.0)).anim().body(80.0, 30.0),
        prop("w-antenna", "obj-antenna", 20.5, 16.0, 8.0, 6.0).anim(),
        {
            let (x, y) = on(47.0, 30.0);
            MapObject::new("w-lamplighter", "obj-drone", x, y).anim().show_if("w.mast").interact("w.dive.lamplighter", "Face the Lamplighter").radius(24.0).light(Light::new("glow-magenta", 0.8, 0.6, -18.0).pulse())
        },
        prop("w-radio", "obj-radio", 27.0, 17.0, 24.0, 8.0).anim().interact("w.dive.static", "Tune the radio").light(Light::new("glow-green", 0.5, 0.4, -12.0).pulse()),
        board("sig-b0", "w.board.b0", "obj-sig_teal_l", 30.5, 16.0, "Look at the training rig"),
        prop("w-rigmonitor", "obj-rigmonitor", 29.0, 16.0, 10.0, 4.0).anim().light(Light::new("glow-cyan", 0.4, 0.35, -14.0)),
        gate("w-gate-c01", 'v', 32.0, 18.0, "!w.gate.c0", "w.gate.c0"),
        gate("w-gate-c02", 'v', 32.0, 19.0, "!w.gate.c0", "w.gate.c0"),
        prop("w-bench1", "obj-bench", 25.5, 22.0, 28.0, 8.0),
        lamp("w-lamp3", 21.0, 22.0),
        lamp("w-lamp4", 30.0, 22.0),
        // --- customs yard ---------------------------------------------------------------
        MapObject::new("w-customs", "bld-customs", t(37.0), t(17.0)).anim().body(96.0, 30.0),
        board("sig-b1", "w.board.b1", "obj-sig_red_l", 42.5, 16.0, "Look at the gate's board"),
        gate("w-gate-c11", 'v', 44.0, 18.0, "!w.gate.c1", "w.gate.c1"),
        gate("w-gate-c12", 'v', 44.0, 19.0, "!w.gate.c1", "w.gate.c1"),
        prop("w-cont1", "obj-container_r", 36.0, 23.0, 32.0, 12.0),
        prop("w-cont2", "obj-container_b", 39.0, 23.0, 32.0, 12.0),
        prop("w-cone1", "obj-cone", 41.5, 20.5, 6.0, 4.0),
        lamp("w-lamp5", 33.5, 17.5),
        // --- the alley --------------------------------------------------------------------
        board("sig-b2", "w.board.b2", "obj-sig_red_g", 45.3, 16.6, "Look at the patrol orders"),
        // an island of containers the guard walks around
        prop("w-cont3", "obj-container_g", 49.0, 18.0, 32.0, 12.0),
        prop("w-cont4", "obj-container_r", 51.0, 18.0, 32.0, 12.0),
        prop("w-cont5", "obj-container_b", 49.0, 20.0, 32.0, 12.0),
        prop("w-cont6", "obj-container_g", 51.0, 20.0, 32.0, 12.0),
        MapObject::new("w-cam", "obj-camera", t(53.0) + 8.0, t(24.0) + 12.0).anim().body(6.0, 4.0).show_if("!w.cam.off").light(Light::new("glow-red", 0.3, 0.45, -26.0).pulse()),
        MapObject::new("w-cam-off", "obj-camera", t(53.0) + 8.0, t(24.0) + 12.0).body(6.0, 4.0).show_if("w.cam.off"),
        // --- the mole and the mast ----------------------------------------------------------
        board("sig-b3", "w.board.b3", "obj-sig_violet_g", 48.2, 27.0, "Look at the camera's board"),
        board("sig-d1", "w.board.d1", "obj-sig_orange_l", 48.2, 30.0, "Look at the gate's board"),
        gate("w-gate-w1", 'h', 49.0, 31.0, "!w.gate.w", "w.gate.w"),
        gate("w-gate-w2", 'h', 50.0, 31.0, "!w.gate.w", "w.gate.w"),
        board("sig-mast", "w.board.mast", "obj-sig_white_g", 48.0, 34.0, "Look at the mast's board"),
        MapObject::new("w-mast", "obj-mast", t(51.0) + 8.0, t(35.0) + 12.0).anim().body(6.0, 4.0).show_if("!w.mast").light(Light::new("glow-red", 0.6, 0.6, -42.0).pulse()),
        MapObject::new("w-mast-free", "obj-mast", t(51.0) + 8.0, t(35.0) + 12.0).body(6.0, 4.0).show_if("w.mast").light(Light::new("glow-warm", 1.6, 0.7, -42.0).pulse()),
        prop("w-bollard1", "obj-bollard", 48.0, 25.6, 6.0, 4.0),
        prop("w-bollard2", "obj-bollard", 51.0, 25.6, 6.0, 4.0),
        // --- fish market ----------------------------------------------------------------------
        board("sig-c1n", "w.board.c1n", "obj-sig_green", 15.0, 3.0, "Look at the notice board"),
        prop("w-console-market", "obj-console_green", 16.6, 3.0, 12.0, 5.0).anim(),
        board("sig-c1m", "w.board.c1m", "obj-sig_green", 18.2, 3.0, "Look at the memo board"),
        board("sig-c1l", "w.board.c1l", "obj-sig_grey_l", 21.5, 3.0, "Look at the sign locker"),
        prop("w-locker-sign", "obj-locker", 22.8, 3.0, 12.0, 5.0).frame_if(&[(flags::C1, 1)]),
        prop("w-stall1", "obj-stall", 14.0, 8.0, 30.0, 8.0),
        prop("w-stall2", "obj-stall", 19.0, 9.5, 30.0, 8.0),
        prop("w-fish2", "obj-fishcrate", 16.0, 8.4, 14.0, 8.0),
        prop("w-fish3", "obj-fishcrate", 21.2, 9.8, 14.0, 8.0),
        prop("w-fish4", "obj-fishcrate", 12.8, 10.6, 14.0, 8.0),
        lamp("w-lamp6", 24.0, 10.0),
        lamp("w-lamp7", 13.0, 5.0),
        // the ice house
        board("sig-c2a", "w.board.c2a", "obj-sig_blue", 27.6, 2.0, "Look at the lamp board"),
        prop("w-console-ice", "obj-console_blue", 29.2, 2.0, 12.0, 5.0).anim(),
        board("sig-c2b", "w.board.c2b", "obj-sig_blue", 30.8, 2.0, "Look at the lamp board"),
        prop("w-locker-ice", "obj-locker", 32.6, 2.4, 12.0, 5.0).interact("w.locker.ice", "Try the locker").frame_if(&[(flags::SIGIL, 1)]),
        gate("w-gate-ice", 'h', 30.0, 6.0, "!w.c1", "w.gate.ice"),
        board("sig-c3", "w.board.c3", "obj-sig_green_g", 33.0, 6.6, "Look at the gate's board"),
        gate("w-gate-m1", 'v', 34.0, 8.0, "!w.gate.m", "w.gate.m"),
        gate("w-gate-m2", 'v', 34.0, 9.0, "!w.gate.m", "w.gate.m"),
        // the way into the rest of the city
        gate("w-gate-city1", 'h', 22.0, 0.0, "!w.mast", "w.gate.city"),
        gate("w-gate-city2", 'h', 23.0, 0.0, "!w.mast", "w.gate.city"),
        // --- harbour master's shed ----------------------------------------------------------
        MapObject::new("w-shed", "bld-shed", t(42.0), t(5.0)).anim().body(96.0, 28.0),
        board("sig-tide", "w.board.tide", "obj-sig_white", 36.2, 3.0, "Look at the tide board"),
        prop("w-console-mast", "obj-console_white", 37.8, 3.0, 12.0, 5.0).anim(),
        prop("w-crate2", "obj-crate", 45.0, 10.0, 16.0, 10.0),
        prop("w-barrel2", "obj-barrel", 43.8, 10.2, 12.0, 8.0),
        lamp("w-lamp8", 36.0, 10.0),
        // --- out of reach ---------------------------------------------------------------------
        MapObject::new("w-warehouse", "bld-warehouse", t(51.5), t(9.0)).anim().body(112.0, 34.0),
        prop("w-tree1", "obj-tree", 3.0, 4.0, 10.0, 6.0).anim(),
        prop("w-tree2", "obj-tree", 8.0, 7.0, 10.0, 6.0).anim(),
        prop("w-tree3", "obj-tree", 5.0, 10.0, 10.0, 6.0).anim(),
    ];
    for (n, (x, y)) in [(13.5, 16.0), (20.5, 13.6), (31.0, 13.6), (33.5, 13.6), (43.0, 13.6), (12.5, 1.6), (33.0, 9.6), (47.3, 33.0)].into_iter().enumerate() {
        objects.extend(banner(n, x, y));
    }

    let npc = |id: &'static str, ch: &'static str, tx: f32, ty: f32, f: Facing, act: Option<&'static str>, label: Option<&'static str>| {
        let (x, y) = on(tx, ty);
        NpcDef { id, char: ch, x, y, facing: f, interact: act, label, show_if: None, wander: None, hologram: false }
    };
    let roam = |x: f32, y: f32, w: f32, h: f32| Some(Rect { x: t(x), y: t(y), w: t(w), h: t(h) });
    let npcs = vec![
        npc("w-mara", "mara", 8.0, 18.5, Facing::Down, Some("w.mara"), Some("Talk to Mara")),
        // the guard is walked by the district's clock, not by the engine
        NpcDef { show_if: Some("!w.mast"), ..npc("w-guard", "agent", 46.0, 15.0, Facing::Right, None, None) },
        NpcDef { wander: roam(13.0, 5.0, 10.0, 2.0), ..npc("w-fishwife", "citizen2", 17.0, 6.0, Facing::Down, Some("w.citizen.1"), None) },
        // the clerk trains on the rig, so he hangs about the plaza; the docker works the customs yard
        NpcDef { wander: roam(21.0, 19.0, 9.0, 3.0), ..npc("w-clerk", "citizen1", 24.0, 20.0, Facing::Left, Some("w.citizen.3"), None) },
        NpcDef { wander: roam(34.0, 19.0, 6.0, 2.0), ..npc("w-docker", "citizen4", 36.0, 19.5, Facing::Right, Some("w.citizen.2"), None) },
    ];

    let zone = |id: &'static str, x: f32, y: f32, w: f32, h: f32| Zone { id, rect: Rect { x: t(x), y: t(y), w: t(w), h: t(h) } };
    let zones = vec![
        zone("w.z.market", 12.0, 1.0, 22.0, 11.0),
        zone("w.z.plaza", 20.0, 13.0, 12.0, 11.0),
        zone("w.z.customs", 33.0, 13.0, 11.0, 11.0),
        zone("w.z.alley", 45.0, 13.0, 10.0, 12.0),
        zone("w.z.shed", 35.0, 1.0, 12.0, 11.0),
        zone("w.z.mole", 48.0, 26.0, 4.0, 5.0),
        zone("w.z.mast", 47.0, 32.0, 6.0, 5.0),
        zone("w.exit", 22.0, 0.0, 2.0, 1.0),
    ];

    let mut spawns = HashMap::new();
    let mut sp = |name: &'static str, tx: f32, ty: f32, facing: Facing| {
        let (x, y) = on(tx, ty);
        spawns.insert(name, Spawn { x, y, facing });
    };
    sp("start", 10.0, 19.5, Facing::Down);
    sp("pier", 6.5, 23.0, Facing::Down);
    sp("cabinet", 12.0, 14.6, Facing::Up);
    sp("quay", 17.5, 18.0, Facing::Up);
    sp("rig", 30.5, 18.0, Facing::Up);
    sp("radio", 27.0, 18.6, Facing::Up);
    sp("customs", 42.5, 18.0, Facing::Up);
    sp("patrol", 42.5, 19.0, Facing::Right);
    sp("camera", 46.5, 23.0, Facing::Down);
    sp("market", 16.6, 5.0, Facing::Up);
    sp("ice", 29.2, 4.2, Facing::Up);
    sp("backgate", 32.5, 8.6, Facing::Up);
    sp("shed", 36.6, 5.0, Facing::Up);
    sp("mole", 50.0, 28.6, Facing::Down);
    sp("mast", 49.5, 33.0, Facing::Down);
    sp("exit", 22.5, 1.0, Facing::Up);

    let cam = objects.iter().find(|o| o.id == "w-cam").expect("camera sprite");
    let sensors = vec![Sensor {
        id: "w-cam",
        kind: SensorKind::Camera { range: 112.0, half_angle: 0.30, center: 2.5, sweep: 0.9, period: 12.0 },
        x: cam.x,
        y: cam.y - 20.0,
        gain: 60.0,
        show_if: Some("!w.cam.off"),
    }];

    MapDef {
        id: MapId::Wharf,
        title: "The Wharf",
        w: W,
        h: H,
        tiles: tiles(),
        objects,
        npcs,
        spawns,
        zones,
        music: "city",
        tint: 0x6e7cb0,
        ambient: Some(Ambient::Motes),
        cat_mode: Some(CatMode::Follow),
        cat_area: None,
        sensors,
    }
}

/// The object that stands for a board in the world.
pub fn board_object(board: &str) -> String {
    format!("sig-{board}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signals::Signals;
    use crate::state::GameState;

    fn solid(m: &MapDef, x: f32, y: f32) -> bool {
        cpl_art::tiles::is_solid(m.tile((x / 16.0).floor() as i32, (y / 16.0).floor() as i32))
    }

    #[test]
    fn every_board_site_and_spawn_is_on_the_map() {
        let m = wharf();
        let mut st = GameState::new();
        st.new_game("t", 0, Some("map"));
        let sig = Signals::wharf(&st);
        for b in &sig.boards {
            let o = m.object(&board_object(b.id)).unwrap_or_else(|| panic!("no object for board {}", b.id));
            assert!(o.interact.is_some_and(|i| i == format!("w.board.{}", b.id)), "{} interacts as {:?}", b.id, o.interact);
            assert!(!solid(&m, o.x, o.y), "board {} stands in a wall", b.id);
        }
        for s in &sig.sites {
            let sp = m.spawns.get(s.spawn).unwrap_or_else(|| panic!("site {} has no spawn {}", s.id, s.spawn));
            assert!(!solid(&m, sp.x, sp.y - 4.0), "spawn {} is inside something solid", s.spawn);
        }
        for l in &sig.lockers {
            assert!(m.interactions().contains(&format!("w.locker.{}", l.id).as_str()), "no locker {} on the map", l.id);
        }
        // the guard's round and his later post are on open ground
        for w in [flags::guard_east(), flags::guard_post()] {
            for (x, y, _) in &w.pts {
                assert!(!solid(&m, *x, *y), "guard waypoint ({}, {}) is solid", x / 16.0, y / 16.0);
            }
        }
    }

    /// Flood the walkable tiles from the start with some gates open, and
    /// check that each room is cut off until its own lock gives way.
    #[test]
    fn gates_hold_until_their_locks_open() {
        let m = wharf();
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
        let mut expect: Vec<&'static str> = vec!["cabinet", "pier", "quay", "start"];
        assert_eq!(reach(&st), expect);
        for (flag, opens) in [
            (flags::GATE_Q, &["backgate", "exit", "market", "radio", "rig"][..]),
            (flags::GATE_C0, &["customs", "patrol"][..]),
            (flags::GATE_C1, &["camera", "mole"][..]),
            (flags::C1, &["ice"][..]),
            (flags::GATE_M, &["shed"][..]),
            (flags::GATE_W, &["mast"][..]),
        ] {
            st.set(flag, true);
            expect.extend(opens);
            expect.sort_unstable();
            assert_eq!(reach(&st), expect, "after {flag}");
        }
        assert_eq!(expect.len(), m.spawns.len(), "every spawn is reachable in the end");
    }
}
