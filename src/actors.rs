//! Everyone who moves: the player (input, collisions, footsteps), the
//! townsfolk (wandering), Nonce the cat (following or wandering), plus the
//! interaction loop: find what the player faces, show the key cap, and start
//! its story script on E.

use bevy::prelude::*;
use cpl_logic::Facing;
use cpl_logic::game::Mode;
use cpl_logic::maps::CatMode;

use crate::director::{Director, GameRes};
use crate::input::Control;
use crate::sound::SfxQueue;
use crate::world::{Aabb, Cat, Level, Npc, Obj, Player, Pos, Prompt, Shadow, Walker, facing_towards, facing_vec, idle_frame, to_world, Z_PROMPT, Z_SHADOW};

const WALK: f32 = 74.0;
const RUN: f32 = 112.0;
const WALK_FRAMES: [usize; 4] = [1, 0, 2, 0];

/// The longest frame that movement integrates in one go: when frames are
/// slower than this the world slows down rather than letting everyone jump.
pub const MAX_DT: f32 = 0.05;

/// How far the player gets this frame with the stick held all the way. The
/// autopilot uses it to land exactly on its waypoints instead of past them.
pub fn player_stride(dt: f32, run: bool, pace: f32) -> f32 {
    (if run { RUN } else { WALK }) * pace * dt.min(MAX_DT)
}

/// Autopilot speed multiplier (1 = a person's pace).
#[derive(Resource)]
pub struct Pace(pub f32);

/// Collect the bodies of everyone standing in the world.
pub fn gather_bodies(mut level: ResMut<Level>, q: Query<(Entity, &Pos, &Walker, Option<&Npc>, Has<Cat>, Has<Player>)>) {
    let Some(map) = level.map.clone() else { return };
    let wander_cat = map.cat_mode == Some(CatMode::Wander);
    let mut actors = Vec::new();
    for (e, pos, w, npc, cat, player) in q.iter() {
        let solid = match npc {
            Some(n) => n.visible && !map.npcs[n.idx].hologram,
            None => player || (cat && wander_cat),
        };
        if solid {
            actors.push((e, Aabb::feet(pos.p, w.hw, w.h)));
        }
    }
    level.actors = actors;
}

pub fn move_player(time: Res<Time>, control: Res<Control>, pace: Res<Pace>, game: NonSend<GameRes>, director: Res<Director>, level: Res<Level>, mut sfx: ResMut<SfxQueue>, mut q: Query<(Entity, &mut Pos, &mut Walker), With<Player>>, mut step: Local<f32>) {
    let Ok((e, mut pos, mut w)) = q.single_mut() else { return };
    let dt = time.delta_secs().min(MAX_DT);
    let frozen = game.mode != Mode::World || game.busy() || director.transition.is_some();
    let mut axis = if frozen { Vec2::ZERO } else { control.axis };
    if axis.length() > 1.0 {
        axis = axis.normalize();
    }
    let auto = control.override_axis.is_some();
    let stride = player_stride(dt, control.run, if auto { pace.0 } else { 1.0 });
    w.running = control.run;
    // a dead zone for sticks and touch; the autopilot may ask for a short, exact step
    let dead = if auto { 1e-3 } else { 0.1 };
    if axis.length() > dead {
        let dir = axis.normalize();
        if dir.x.abs() > dir.y.abs() + 0.05 {
            w.facing = if dir.x < 0.0 { Facing::Left } else { Facing::Right };
        } else if dir.y.abs() > 0.05 {
            w.facing = if dir.y < 0.0 { Facing::Up } else { Facing::Down };
        }
        let (np, _, _) = level.slide(Some(e), pos.p, w.hw, w.h, axis * stride, false);
        let moved = np.distance(pos.p) > 0.01;
        pos.p = np;
        w.moving = moved || axis.length() > dead;
        *step += dt;
        if *step > if control.run { 0.22 } else { 0.3 } {
            *step = 0.0;
            sfx.0.push("step");
        }
    } else {
        w.moving = false;
    }
}

pub fn npc_ai(time: Res<Time>, game: NonSend<GameRes>, level: Res<Level>, mut q: Query<(Entity, &mut Pos, &mut Walker, &mut Npc)>) {
    let Some(map) = level.map.clone() else { return };
    let dt = time.delta_secs().min(MAX_DT);
    let busy = game.busy();
    for (e, mut pos, mut w, mut npc) in q.iter_mut() {
        if !npc.visible {
            continue;
        }
        let def = &map.npcs[npc.idx];
        match npc.target {
            Some(t) if def.wander.is_some() => {
                let d = t - pos.p;
                if d.length() < 2.0 {
                    npc.target = None;
                    npc.wait = 1.5 + rand01(e, time.elapsed_secs()) * 4.0;
                    w.moving = false;
                } else {
                    let v = d.normalize() * 32.0 * dt;
                    w.facing = facing_towards(Vec2::ZERO, d);
                    let (np, bx, by) = level.slide(Some(e), pos.p, w.hw, w.h, v, false);
                    pos.p = np;
                    w.moving = true;
                    if bx || by {
                        npc.target = None;
                        npc.wait = 1.0 + rand01(e, time.elapsed_secs() + 1.0) * 2.0;
                        w.moving = false;
                    }
                }
            }
            _ => {
                w.moving = false;
                if let Some(r) = def.wander
                    && !busy
                {
                    npc.wait -= dt;
                    if npc.wait <= 0.0 {
                        let t = time.elapsed_secs();
                        npc.target = Some(Vec2::new(r.x + rand01(e, t) * r.w, r.y + rand01(e, t + 7.0) * r.h));
                    }
                }
            }
        }
    }
}

/// A cheap deterministic "random" in [0, 1) per entity and time.
fn rand01(e: Entity, t: f32) -> f32 {
    let mut x = (e.to_bits() as u32).wrapping_mul(2654435761) ^ (t * 1000.0) as u32;
    x ^= x >> 13;
    x = x.wrapping_mul(0x5bd1e995);
    x ^= x >> 15;
    (x % 10_000) as f32 / 10_000.0
}

pub fn cat_ai(time: Res<Time>, game: NonSend<GameRes>, level: Res<Level>, player: Query<(&Pos, &Walker), (With<Player>, Without<Cat>)>, mut q: Query<(Entity, &mut Pos, &mut Cat, &mut Sprite), Without<Player>>) {
    let Some(map) = level.map.clone() else { return };
    let Ok((ppos, pw)) = player.single() else { return };
    let dt = time.delta_secs().min(MAX_DT);
    for (e, mut pos, mut cat, mut sprite) in q.iter_mut() {
        let mut goal: Option<(Vec2, f32)> = None;
        match map.cat_mode {
            Some(CatMode::Follow) => {
                let f = facing_vec(pw.facing);
                let g = Vec2::new(ppos.p.x - f.x * 20.0 - 10.0, ppos.p.y - f.y * 14.0 + 4.0);
                let d = g.distance(pos.p);
                if d > 220.0 {
                    pos.p = g;
                } else if d > 26.0 {
                    goal = Some((g, (60.0 + d).min(120.0)));
                }
            }
            Some(CatMode::Wander) => {
                cat.wait -= dt;
                if let Some(t) = cat.target {
                    if t.distance(pos.p) < 3.0 {
                        cat.target = None;
                        cat.wait = 2.5 + rand01(e, time.elapsed_secs()) * 5.0;
                    } else {
                        goal = Some((t, 38.0));
                    }
                } else if cat.wait <= 0.0
                    && !game.busy()
                    && let Some(a) = map.cat_area
                {
                    let t = time.elapsed_secs();
                    cat.target = Some(Vec2::new(a.x + rand01(e, t) * a.w, a.y + rand01(e, t + 3.0) * a.h));
                }
            }
            None => {}
        }
        let frame = match goal {
            Some((g, speed)) => {
                let d = g - pos.p;
                let v = d.normalize_or_zero() * speed * dt;
                let tiles_only = map.cat_mode == Some(CatMode::Follow);
                let (np, bx, by) = level.slide(Some(e), pos.p, 5.0, 5.0, v, tiles_only);
                pos.p = np;
                if (bx && by) && map.cat_mode == Some(CatMode::Wander) {
                    cat.target = None;
                    cat.wait = 1.5;
                }
                cat.last_left = d.x < 0.0;
                let step = ((time.elapsed_secs() * 7.0) as usize) % 2;
                if cat.last_left { 3 + step } else { 6 + step }
            }
            None => {
                if cat.last_left {
                    2
                } else {
                    5
                }
            }
        };
        if let Some(a) = sprite.texture_atlas.as_mut() {
            a.index = frame;
        }
    }
}

/// Walk cycles for the player and NPCs.
pub fn animate_walkers(time: Res<Time>, mut q: Query<(&mut Walker, &mut Sprite), Without<Cat>>) {
    let dt = time.delta_secs();
    for (mut w, mut sprite) in q.iter_mut() {
        let frame = if w.moving {
            w.anim += dt * 8.0 * if w.running { 1.5 } else { 1.0 };
            idle_frame(w.facing) + WALK_FRAMES[(w.anim as usize) % 4]
        } else {
            w.anim = 0.0;
            idle_frame(w.facing)
        };
        if let Some(a) = sprite.texture_atlas.as_mut()
            && a.index != frame
        {
            a.index = frame;
        }
    }
}

/// Write map positions into transforms (rounded to whole pixels) and keep shadows underfoot.
pub fn sync_positions(mut q: Query<(&Pos, &mut Transform), (Changed<Pos>, Without<Shadow>)>, mut shadows: Query<(&Shadow, &mut Transform, &mut Visibility)>, owners: Query<(&Pos, &Visibility), Without<Shadow>>) {
    for (pos, mut tf) in q.iter_mut() {
        tf.translation = pos.translation();
    }
    for (s, mut tf, mut vis) in shadows.iter_mut() {
        if let Ok((p, v)) = owners.get(s.0) {
            tf.translation = to_world(p.p.x.round(), p.p.y.round() - 1.0, Z_SHADOW);
            vis.set_if_neq(*v);
        }
    }
}

/// What the player would interact with right now.
#[derive(Resource, Default, Clone, Debug)]
pub struct Target {
    pub current: Option<(String, String)>,
    pub kind_npc: Option<Entity>,
    /// The autopilot's choice wins over nearer things (like a cat underfoot).
    pub prefer: Option<String>,
}

/// Pick the nearest interactable in front of the player, place the key cap,
/// set the hint, and start the interaction when asked.
#[allow(clippy::too_many_arguments)]
pub fn interact(
    control: Res<Control>,
    mut game: NonSendMut<GameRes>,
    director: Res<Director>,
    level: Res<Level>,
    mut target: ResMut<Target>,
    mut player: Query<(&Pos, &mut Walker), With<Player>>,
    objs: Query<(&Obj, &Sprite, &Visibility)>,
    mut npcs: Query<(Entity, &Pos, &Npc, &mut Walker), Without<Player>>,
    cats: Query<&Pos, With<Cat>>,
    mut prompt: Query<(&mut Transform, &mut Visibility), (With<Prompt>, Without<Obj>)>,
    images: Res<Assets<TextureAtlasLayout>>,
) {
    let Some(map) = level.map.clone() else { return };
    let Ok((ppos, mut pw)) = player.single_mut() else { return };
    let Ok((mut ptf, mut pvis)) = prompt.single_mut() else { return };
    let busy = game.mode != Mode::World || game.busy() || director.transition.is_some();
    if busy {
        pvis.set_if_neq(Visibility::Hidden);
        if target.current.take().is_some() {
            game.hud.hint = None;
        }
        return;
    }
    let f = facing_vec(pw.facing);
    let probe = Vec2::new(ppos.p.x + f.x * 9.0, ppos.p.y - 4.0 + f.y * 9.0);
    let mut best: Option<(f32, String, String, Vec2, Option<Entity>)> = None;
    let prefer = target.prefer.clone();
    // nearer wins, except that a preferred target always beats the rest
    let better = |best: &Option<(f32, String, String, Vec2, Option<Entity>)>, d: f32, id: &str| -> bool {
        match best {
            None => true,
            Some(b) => {
                let pb = prefer.as_deref() == Some(b.1.as_str());
                let pi = prefer.as_deref() == Some(id);
                if pb != pi { pi } else { d < b.0 }
            }
        }
    };
    for (obj, sprite, vis) in objs.iter() {
        let o = &map.objects[obj.idx];
        let Some(id) = o.interact else { continue };
        if *vis == Visibility::Hidden {
            continue;
        }
        let ip = Vec2::new(o.x + o.ix, o.y + o.iy);
        let d = ip.distance(probe);
        if d < o.radius && better(&best, d, id) {
            let h = sprite.texture_atlas.as_ref().and_then(|a| images.get(&a.layout)).and_then(|l| l.textures.first()).map(|r| r.height() as f32).unwrap_or(16.0);
            let at = Vec2::new(ip.x, (ip.y - 10.0).min(o.y - h.min(34.0)) - 2.0);
            best = Some((d, id.to_string(), o.label.unwrap_or("Examine").to_string(), at, None));
        }
    }
    for (e, pos, npc, _) in npcs.iter() {
        let n = &map.npcs[npc.idx];
        let Some(id) = n.interact else { continue };
        if !npc.visible {
            continue;
        }
        let d = Vec2::new(pos.p.x, pos.p.y - 6.0).distance(probe);
        if d < 22.0 && better(&best, d, id) {
            best = Some((d, id.to_string(), n.label.unwrap_or("Talk").to_string(), Vec2::new(pos.p.x, pos.p.y - 26.0), Some(e)));
        }
    }
    if let Some(e) = level.cat
        && let Ok(c) = cats.get(e)
    {
        let d = Vec2::new(c.p.x, c.p.y - 4.0).distance(probe);
        if d < 16.0 && better(&best, d, "nonce") {
            best = Some((d, "nonce".into(), "Pet Nonce".into(), Vec2::new(c.p.x, c.p.y - 16.0), None));
        }
    }
    // remember which NPC it is, so they can turn to face the player
    let npc_entity = best.as_ref().and_then(|b| b.4);
    match &best {
        Some((_, id, label, at, _)) => {
            ptf.translation = to_world(at.x.round(), at.y.round(), Z_PROMPT);
            pvis.set_if_neq(Visibility::Inherited);
            let changed = target.current.as_ref().map(|c| &c.0) != Some(id);
            if changed || game.hud.hint.is_none() {
                game.hud.hint = Some(("E".into(), label.clone()));
            }
            target.current = Some((id.clone(), label.clone()));
        }
        None => {
            pvis.set_if_neq(Visibility::Hidden);
            if target.current.take().is_some() {
                game.hud.hint = None;
            }
        }
    }
    target.kind_npc = npc_entity;
    if control.act
        && game.input_ready()
        && let Some((_, id, _, _, _)) = best
    {
        if let Some(e) = npc_entity
            && let Ok((_, npos, npc, mut nw)) = npcs.get_mut(e)
            && !map.npcs[npc.idx].hologram
        {
            nw.facing = facing_towards(npos.p, ppos.p);
            nw.moving = false;
        }
        pw.moving = false;
        game.hud.hint = None;
        target.current = None;
        pvis.set_if_neq(Visibility::Hidden);
        game.interact(&id);
    }
}

/// Zones: district title cards and other walk-in triggers.
pub fn zones(mut game: NonSendMut<GameRes>, level: Res<Level>, player: Query<&Pos, With<Player>>, mut inside: Local<Vec<&'static str>>, mut last_map: Local<Option<cpl_logic::state::MapId>>) {
    let Some(map) = level.map.clone() else { return };
    if *last_map != Some(map.id) {
        inside.clear();
        *last_map = Some(map.id);
    }
    let Ok(p) = player.single() else { return };
    for z in &map.zones {
        let is_in = z.rect.contains(p.p.x, p.p.y);
        let was = inside.contains(&z.id);
        if is_in && !was {
            inside.push(z.id);
            game.zone(z.id);
        } else if !is_in && was {
            inside.retain(|i| *i != z.id);
        }
    }
}

pub struct ActorsPlugin;

impl Plugin for ActorsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Target>();
    }
}
