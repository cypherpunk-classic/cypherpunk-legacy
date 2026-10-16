//! Autoplay through the real world: the same route the headless tests use
//! (`cpl_logic::walkthrough`), but walked on foot. A* over an 8-pixel grid
//! finds a way to each person or prop, the player turns to face it and
//! presses E; dialogue and puzzles play themselves (`Game::auto`). Used for
//! the attract-mode demo (`?autoplay=demo`) and as an end-to-end test of the
//! engine side (`CPL_AUTOPLAY=1 CPL_SHOTS=dir CPL_EXIT_AT_END=1`).
//!
//! Walking has to work at any frame rate: CI renders on a software Vulkan
//! driver at a dozen frames a second, where one frame's stride is longer than
//! a grid cell. So paths are cut down to their corners and every step is
//! scaled to land on the next corner exactly, never past it.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use cpl_logic::game::Mode;
use cpl_logic::walkthrough::{Step, next_step};

use crate::actors::{Pace, Target, player_stride};
use crate::director::{Director, GameRes};
use crate::input::Control;
use crate::platform::Launch;
use crate::world::{Aabb, Level, Npc, Player, Pos, Walker, facing_towards, facing_vec};

const CELL: f32 = 8.0;

/// With `CPL_EXIT_AT_END` (the CI run), a step of the route still unfinished
/// after this many seconds of game time ends the run with an error, a
/// screenshot and a report of where things stand, instead of leaving the job
/// to walk in circles until its timeout.
const STEP_LIMIT: f64 = 300.0;

#[derive(Resource, Default)]
pub struct Autopilot {
    pub on: bool,
    pub seed: Option<String>,
    started: bool,
    path: Vec<Vec2>,
    goal: Option<(String, Vec2)>,
    stuck_t: f32,
    last_p: Vec2,
    replans: u32,
    pub finished_at: Option<f64>,
    last_step: Option<&'static str>,
    shots: Option<String>,
    shot_n: u32,
    shot_at: Vec<(f64, String)>,
    exit_at_end: bool,
    map_seen: Option<cpl_logic::state::MapId>,
    /// Cells to steer clear of (where we got stuck), for the current leg.
    avoid: Vec<usize>,
    debug_t: f64,
    reach_t: f32,
    /// How long A* has found no way to the target (a gate the story just
    /// opened only lets people through from the next frame on).
    no_path_t: f32,
    every: Option<f32>,
    next_every: f64,
    /// Frames drawn so far, and (frames, wall-clock seconds, game seconds)
    /// when the game and the current step began, for the fps report and the
    /// watchdog.
    frames: u64,
    run_start: (u64, f64),
    step_start: (u64, f64, f64),
    /// Set once the run has failed: the app exits after the last screenshot.
    fail_at: Option<f64>,
}

impl Autopilot {
    pub fn from_launch(l: &Launch) -> Self {
        Self { on: l.autoplay, seed: l.seed.clone(), shots: l.shots.clone(), exit_at_end: l.exit_at_end, every: l.shot_every, ..default() }
    }

    fn shot_later(&mut self, now: f64, delay: f64, name: &str) {
        if self.shots.is_some() {
            self.shot_at.push((now + delay, name.to_string()));
        }
    }

    /// Average frames per second since (frames, wall-clock seconds).
    fn fps_since(&self, (frames, wall): (u64, f64), wall_now: f64) -> f64 {
        (self.frames - frames) as f64 / (wall_now - wall).max(1e-3)
    }
}

/// Keep only the corners of a grid path (and its first and last points).
/// Between two corners the path runs straight along a row, a column or a
/// diagonal, so walking straight from corner to corner crosses exactly the
/// cells A* chose.
fn corners(path: Vec<Vec2>) -> Vec<Vec2> {
    let dir = |a: Vec2, b: Vec2| ((b - a) / CELL).round().as_ivec2();
    let mut out = Vec::with_capacity(path.len());
    for (i, &p) in path.iter().enumerate() {
        if i == 0 || i + 1 == path.len() || dir(path[i - 1], p) != dir(p, path[i + 1]) {
            out.push(p);
        }
    }
    out
}

/// The grid cell under a point, if it is on the map.
fn cell_of(level: &Level, p: Vec2) -> Option<usize> {
    let size = level.size();
    let (gw, gh) = ((size.x / CELL) as i32, (size.y / CELL) as i32);
    let (x, y) = ((p.x / CELL).floor() as i32, (p.y / CELL).floor() as i32);
    (x >= 0 && y >= 0 && x < gw && y < gh).then(|| (y * gw + x) as usize)
}

#[derive(Copy, Clone, PartialEq)]
struct Node {
    f: f32,
    i: usize,
}

impl Eq for Node {}

impl Ord for Node {
    fn cmp(&self, o: &Self) -> Ordering {
        o.f.partial_cmp(&self.f).unwrap_or(Ordering::Equal).then_with(|| self.i.cmp(&o.i))
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

/// Where the player can stand to use the interaction at `ip` (within `radius`).
fn can_reach(feet: Vec2, ip: Vec2, radius: f32) -> bool {
    let f = facing_vec(facing_towards(feet, ip));
    let probe = Vec2::new(feet.x + f.x * 9.0, feet.y - 4.0 + f.y * 9.0);
    probe.distance(ip) < radius * 0.8
}

/// A* from the player's feet to any spot that reaches the interaction point.
fn find_path(level: &Level, me: Entity, from: Vec2, ip: Vec2, radius: f32, avoid: &[usize]) -> Option<Vec<Vec2>> {
    let size = level.size();
    let (gw, gh) = ((size.x / CELL) as i32, (size.y / CELL) as i32);
    if gw <= 0 || gh <= 0 {
        return None;
    }
    let center = |i: usize| Vec2::new((i as i32 % gw) as f32 * CELL + CELL / 2.0, (i as i32 / gw) as f32 * CELL + CELL / 2.0);
    let n = (gw * gh) as usize;
    let mut walk = vec![None::<bool>; n];
    let mut walkable = |i: usize| -> bool { *walk[i].get_or_insert_with(|| !avoid.contains(&i) && !level.blocked(Aabb::feet(center(i), 5.0, 6.0), Some(me))) };
    let start = {
        let cx = ((from.x / CELL) as i32).clamp(0, gw - 1);
        let cy = ((from.y / CELL) as i32).clamp(0, gh - 1);
        (cy * gw + cx) as usize
    };
    let mut g = vec![f32::INFINITY; n];
    let mut came = vec![usize::MAX; n];
    let mut open = BinaryHeap::new();
    g[start] = 0.0;
    open.push(Node { f: center(start).distance(ip), i: start });
    let mut found = None;
    let mut expanded = 0;
    while let Some(Node { i, .. }) = open.pop() {
        expanded += 1;
        if expanded > 60_000 {
            break;
        }
        let c = center(i);
        if (i == start && can_reach(from, ip, radius)) || (i != start && can_reach(c, ip, radius)) {
            found = Some(i);
            break;
        }
        let (x, y) = (i as i32 % gw, i as i32 / gw);
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= gw || ny >= gh {
                continue;
            }
            let j = (ny * gw + nx) as usize;
            if !walkable(j) {
                continue;
            }
            if dx != 0 && dy != 0 {
                // no cutting corners
                let a = (y * gw + nx) as usize;
                let b = (ny * gw + x) as usize;
                if !walkable(a) || !walkable(b) {
                    continue;
                }
            }
            let cost = g[i] + if dx != 0 && dy != 0 { 1.414 } else { 1.0 } * CELL;
            if cost < g[j] {
                g[j] = cost;
                came[j] = i;
                open.push(Node { f: cost + center(j).distance(ip) * 0.9, i: j });
            }
        }
    }
    let end = found?;
    let mut path = Vec::new();
    let mut i = end;
    while i != start && i != usize::MAX {
        path.push(center(i));
        i = came[i];
    }
    path.reverse();
    Some(path)
}

/// The interaction point and reach of a route target on the current map.
fn locate(step: &Step, level: &Level, npcs: &Query<(&Pos, &Npc)>) -> Option<(Vec2, f32)> {
    let map = level.map.as_ref()?;
    if let Some((_, o)) = map.objects.iter().enumerate().find(|(i, o)| o.interact == Some(step.target) && level.object_visible.get(*i).copied().unwrap_or(false)) {
        return Some((Vec2::new(o.x + o.ix, o.y + o.iy), o.radius));
    }
    for (p, n) in npcs.iter() {
        if n.visible && map.npcs[n.idx].interact == Some(step.target) {
            return Some((Vec2::new(p.p.x, p.p.y - 6.0), 22.0));
        }
    }
    None
}

#[allow(clippy::too_many_arguments)]
pub fn drive(
    mut commands: Commands,
    time: Res<Time>,
    real: Res<Time<Real>>,
    pace: Res<Pace>,
    mut ap: ResMut<Autopilot>,
    mut game: NonSendMut<GameRes>,
    director: Res<Director>,
    level: Res<Level>,
    mut target: ResMut<Target>,
    mut control: ResMut<Control>,
    mut player: Query<(Entity, &Pos, &mut Walker), With<Player>>,
    npcs: Query<(&Pos, &Npc)>,
    mut exit: MessageWriter<AppExit>,
) {
    if !ap.on {
        return;
    }
    let now = time.elapsed_secs_f64();
    let wall = real.elapsed_secs_f64();
    let dt = time.delta_secs();
    ap.frames += 1;
    control.override_axis = Some(Vec2::ZERO);
    // screenshots that are due
    if let Some(every) = ap.every
        && now >= ap.next_every
    {
        ap.next_every = now + every as f64;
        ap.shot_later(now, 0.0, "tick");
    }
    if let Some(dir) = ap.shots.clone() {
        let due: Vec<String> = ap.shot_at.iter().filter(|(t, _)| *t <= now).map(|(_, n)| n.clone()).collect();
        ap.shot_at.retain(|(t, _)| *t > now);
        for name in due {
            ap.shot_n += 1;
            let path = format!("{dir}/{:02}-{name}.png", ap.shot_n);
            info!("[autopilot] screenshot {path}");
            commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
        }
    }
    // a failed run ends once its last screenshot has had time to be saved
    if let Some(t) = ap.fail_at {
        if now >= t {
            exit.write(AppExit::error());
        }
        return;
    }
    match game.mode {
        Mode::Title => {
            if !ap.started && now > 1.5 && ap.shot_n == 0 && ap.shot_at.is_empty() {
                ap.shot_later(now, 0.0, "title");
            }
            if !ap.started && now > 3.0 {
                ap.started = true;
                let seed = ap.seed.clone().unwrap_or_else(|| "autopilot".into());
                info!("[autopilot] new game, seed {seed:?}");
                game.new_game("nightjar", 0, Some(seed.as_str()));
                ap.run_start = (ap.frames, wall);
                ap.step_start = (ap.frames, wall, now);
            }
            return;
        }
        Mode::Credits => {
            if ap.finished_at.is_none() {
                ap.finished_at = Some(now);
                info!("[autopilot] the end, after {:.0} s ({:.0} fps on average)", now, ap.fps_since(ap.run_start, wall));
                ap.shot_later(now, 4.0, "credits");
            }
            if ap.exit_at_end && ap.finished_at.is_some_and(|t| now - t > 6.0) {
                exit.write(AppExit::Success);
            }
            return;
        }
        Mode::World => {}
    }
    // the watchdog (CI only): a step that never finishes is a failure, not a hang
    if ap.exit_at_end && now - ap.step_start.2 > STEP_LIMIT {
        let doing = if game.busy() { game.busy_reason() } else { format!("walking, {} waypoints to go", ap.path.len()) };
        let at = player.single().map(|(_, p, _)| p.p).ok();
        error!(
            "[autopilot] giving up: {} still not done after {:.0} s ({doing}; map {:?}, player at {at:?}, {:.0} fps)",
            ap.last_step.unwrap_or("the first step"),
            now - ap.step_start.2,
            level.id().map(|m| m.name()),
            ap.fps_since((ap.step_start.0, ap.step_start.1), wall),
        );
        ap.shot_later(now, 0.0, "stuck");
        ap.fail_at = Some(now + 1.0);
        return;
    }
    if director.transition.is_some() {
        return;
    }
    if ap.map_seen != level.id() {
        ap.map_seen = level.id();
        ap.path.clear();
        ap.goal = None;
        if let Some(id) = level.id() {
            ap.shot_later(now, 2.0, id.name());
        }
    }
    if game.busy() {
        ap.stuck_t = 0.0;
        if now - ap.debug_t > 5.0 {
            ap.debug_t = now;
            info!("[autopilot] waiting: {}", game.busy_reason());
        }
        return;
    }
    let state_step = next_step(&game.ctx.state_ref());
    let Some(step) = state_step else { return };
    if ap.last_step != Some(step.until) {
        if let Some(prev) = ap.last_step {
            info!("[autopilot] {prev} done at {now:.0} s ({:.0} fps)", ap.fps_since((ap.step_start.0, ap.step_start.1), wall));
            ap.shot_later(now, 0.6, prev);
        }
        ap.step_start = (ap.frames, wall, now);
        ap.last_step = Some(step.until);
        ap.path.clear();
        ap.goal = None;
        ap.replans = 0;
        ap.avoid.clear();
    }
    if level.id() != Some(step.map) {
        return;
    }
    let Ok((me, pos, mut walker)) = player.single_mut() else { return };
    target.prefer = Some(step.target.to_string());
    let Some((ip, radius)) = locate(&step, &level, &npcs) else {
        warn!("[autopilot] cannot find {} on this map", step.target);
        return;
    };
    // close enough: face it, and press E once it is the selected target
    if can_reach(pos.p, ip, radius) {
        ap.path.clear();
        walker.facing = facing_towards(pos.p, ip);
        if target.current.as_ref().is_some_and(|(id, _)| id == step.target) && game.input_ready() {
            control.override_act = true;
            ap.goal = None;
            ap.reach_t = 0.0;
        } else {
            ap.reach_t += dt;
            if ap.reach_t > 2.0 {
                ap.reach_t = 0.0;
                info!("[autopilot] at {:?} facing {}, but the target is {:?}; stepping closer", pos.p, step.target, target.current);
                // step towards it a little
                control.override_axis = Some((ip - pos.p).normalize_or_zero() * 0.5);
            }
        }
        return;
    }
    ap.reach_t = 0.0;
    let moved_goal = ap.goal.as_ref().is_none_or(|(id, g)| id != step.target || g.distance(ip) > 12.0);
    if ap.path.is_empty() || moved_goal {
        match find_path(&level, me, pos.p, ip, radius, &ap.avoid) {
            Some(p) => {
                ap.path = corners(p);
                ap.goal = Some((step.target.to_string(), ip));
                ap.no_path_t = 0.0;
            }
            None => {
                ap.no_path_t += dt;
                if ap.no_path_t > 1.0 {
                    ap.no_path_t = 0.0;
                    warn!("[autopilot] no path to {} from {:?}", step.target, pos.p);
                }
                ap.goal = None;
                return;
            }
        }
    }
    // Follow the path, landing exactly on each corner. Aiming at a corner that
    // is closer than one frame's stride would carry the player past it, then
    // back past it, for ever: at a low frame rate that is every corner.
    while ap.path.first().is_some_and(|w| w.distance(pos.p) < 2.0) {
        ap.path.remove(0);
    }
    if let Some(&next) = ap.path.first() {
        let stride = player_stride(dt, true, pace.0).max(1e-3);
        control.override_axis = Some((next - pos.p).clamp_length_max(stride) / stride);
    }
    if now - ap.debug_t > 3.0 {
        ap.debug_t = now;
        debug!("[autopilot] at {:?} → {} ({:?}), {} waypoints", pos.p, step.target, ip, ap.path.len());
    }
    // stuck? re-plan, then nudge
    if pos.p.distance(ap.last_p) > 1.0 {
        ap.last_p = pos.p;
        ap.stuck_t = 0.0;
    } else {
        ap.stuck_t += dt;
        if ap.stuck_t > 0.8 {
            ap.stuck_t = 0.0;
            ap.replans += 1;
            // steer around whatever is just ahead next time
            if let Some(&next) = ap.path.first()
                && let Some(c) = cell_of(&level, pos.p + (next - pos.p).normalize_or_zero() * CELL)
            {
                ap.avoid.push(c);
            }
            info!("[autopilot] stuck at {:?} heading for {} (next {:?}), re-planning", pos.p, step.target, ap.path.first());
            ap.path.clear();
            if ap.replans > 6 {
                warn!("[autopilot] stuck near {:?} going to {}", pos.p, step.target);
                ap.replans = 0;
            }
        }
    }
}

pub struct AutopilotPlugin;

impl Plugin for AutopilotPlugin {
    fn build(&self, _app: &mut App) {}
}
