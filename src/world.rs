//! The explorable world: building a map's entities from its definition,
//! keeping them in step with story flags, and the collision queries that
//! movement uses. Positions follow the map data's convention (pixels, y
//! down, origin top-left); [`to_world`] converts to Bevy's y-up space.

use std::sync::Arc;

use bevy::prelude::*;
use bevy::sprite::Anchor;
use cpl_logic::Facing;
use cpl_logic::maps::{CatMode, MapDef, T};
use cpl_logic::state::{GameState, MapId};

use crate::gfx::{Gfx, GlowMaterial, image_from_canvas};

/// Depth bands (Bevy z). Everything standing in the world is y-sorted
/// between `Z_ACTORS` and `Z_ACTORS + 10`.
pub const Z_TILES: f32 = 0.0;
pub const Z_FLOOR: f32 = 2.0;
pub const Z_SHADOW: f32 = 3.0;
pub const Z_ACTORS: f32 = 10.0;
pub const Z_LIGHTS: f32 = 40.0;
pub const Z_PARTICLES: f32 = 45.0;
pub const Z_PROMPT: f32 = 50.0;

pub fn to_world(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, -y, z)
}

pub fn depth(y: f32) -> f32 {
    Z_ACTORS + (y / 100.0).clamp(0.0, 9.9)
}

pub fn facing_vec(f: Facing) -> Vec2 {
    match f {
        Facing::Down => Vec2::new(0.0, 1.0),
        Facing::Up => Vec2::new(0.0, -1.0),
        Facing::Left => Vec2::new(-1.0, 0.0),
        Facing::Right => Vec2::new(1.0, 0.0),
    }
}

pub fn facing_towards(from: Vec2, to: Vec2) -> Facing {
    let d = to - from;
    if d.x.abs() > d.y.abs() {
        if d.x < 0.0 { Facing::Left } else { Facing::Right }
    } else if d.y < 0.0 {
        Facing::Up
    } else {
        Facing::Down
    }
}

pub fn idle_frame(f: Facing) -> usize {
    f.index() * 3
}

/// Map-space colour multiply (the night or lamplight over a map).
pub fn tint_color(tint: u32) -> Color {
    Color::srgb_u8((tint >> 16) as u8, (tint >> 8) as u8, tint as u8)
}

fn mul(a: Color, b: u32, alpha: f32) -> Color {
    let a = a.to_srgba();
    let b = tint_color(b).to_srgba();
    Color::srgba(a.red * b.red, a.green * b.green, a.blue * b.blue, alpha)
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// Everything that belongs to the current map (despawned with it).
#[derive(Component)]
pub struct MapEntity;

/// A baked tile layer; `alt` is the second water frame.
#[derive(Component)]
pub struct TileLayer {
    pub alt: bool,
}

/// Position in map space (bottom-centre for sprites).
#[derive(Component, Clone, Copy, Debug)]
pub struct Pos {
    pub p: Vec2,
    /// +0.5 for odd-width sprites so texels land on whole pixels.
    pub odd: f32,
    /// Added to y for depth sorting.
    pub bias: f32,
    pub floor: bool,
}

impl Pos {
    pub fn new(x: f32, y: f32, width: u32) -> Self {
        Self { p: Vec2::new(x, y), odd: if width % 2 == 1 { 0.5 } else { 0.0 }, bias: 0.0, floor: false }
    }

    pub fn translation(&self) -> Vec3 {
        let z = if self.floor { Z_FLOOR } else { depth(self.p.y + self.bias) };
        to_world(self.p.x.round() + self.odd, self.p.y.round(), z)
    }
}

/// A map prop.
#[derive(Component)]
pub struct Obj {
    pub idx: usize,
    /// Last frame chosen by its `frame_if` rules (so explicit frame changes stick until the rules change).
    pub rule_frame: Option<usize>,
    /// The story set this frame explicitly.
    pub pinned: bool,
}

/// A loop animation over atlas frames.
#[derive(Component)]
pub struct Looping {
    pub frames: Vec<usize>,
    pub fps: f32,
    pub t: f32,
}

/// Something that walks: the player, NPCs, the cat.
#[derive(Component)]
pub struct Walker {
    pub facing: Facing,
    pub moving: bool,
    pub running: bool,
    pub anim: f32,
    /// Collision box: half width and height above the feet.
    pub hw: f32,
    pub h: f32,
}

impl Walker {
    pub fn new(facing: Facing) -> Self {
        Self { facing, moving: false, running: false, anim: 0.0, hw: 5.0, h: 6.0 }
    }
}

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct Npc {
    pub idx: usize,
    pub target: Option<Vec2>,
    pub wait: f32,
    pub visible: bool,
}

#[derive(Component)]
pub struct Cat {
    pub target: Option<Vec2>,
    pub wait: f32,
    pub last_left: bool,
}

/// A shadow that follows its owner.
#[derive(Component)]
pub struct Shadow(pub Entity);

/// An additive light. `levels` are materials at a few strengths for flicker and pulse.
#[derive(Component)]
pub struct Glow {
    pub levels: Vec<Handle<GlowMaterial>>,
    pub flicker: bool,
    pub pulse: bool,
    /// Follows this entity (NPC holograms), with an offset.
    pub follow: Option<(Entity, Vec2)>,
    /// Belongs to this object (hidden with it).
    pub object: Option<usize>,
}

/// The "press E" key cap.
#[derive(Component)]
pub struct Prompt;

// ---------------------------------------------------------------------------
// The level
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub struct Aabb {
    pub min: Vec2,
    pub max: Vec2,
}

impl Aabb {
    pub fn feet(p: Vec2, hw: f32, h: f32) -> Self {
        Self { min: Vec2::new(p.x - hw, p.y - h), max: Vec2::new(p.x + hw, p.y) }
    }

    pub fn overlaps(&self, o: &Aabb) -> bool {
        self.min.x < o.max.x && self.max.x > o.min.x && self.min.y < o.max.y && self.max.y > o.min.y
    }
}

/// The map currently on screen.
#[derive(Resource, Default)]
pub struct Level {
    pub map: Option<Arc<MapDef>>,
    pub solid: Vec<bool>,
    /// Object bodies and whether each is currently active.
    pub bodies: Vec<Option<(Aabb, bool)>>,
    pub object_visible: Vec<bool>,
    pub objects: Vec<Entity>,
    pub npcs: Vec<Entity>,
    pub cat: Option<Entity>,
    /// Dynamic bodies this frame: NPCs, a wandering cat, the player.
    pub actors: Vec<(Entity, Aabb)>,
    pub water_t: f32,
}

impl Level {
    pub fn id(&self) -> Option<MapId> {
        self.map.as_ref().map(|m| m.id)
    }

    pub fn size(&self) -> Vec2 {
        self.map.as_ref().map(|m| Vec2::new(m.w as f32 * T, m.h as f32 * T)).unwrap_or(Vec2::ZERO)
    }

    pub fn tile_solid(&self, tx: i32, ty: i32) -> bool {
        let Some(m) = &self.map else { return true };
        if tx < 0 || ty < 0 || tx >= m.w as i32 || ty >= m.h as i32 {
            return true;
        }
        self.solid[ty as usize * m.w + tx as usize]
    }

    /// Would a box at this place hit a wall, a prop, or someone else?
    pub fn blocked(&self, b: Aabb, me: Option<Entity>) -> bool {
        self.blocked_by(b, me, false)
    }

    /// `tiles_only` ignores props and people (the cat squeezes past everything but walls).
    pub fn blocked_by(&self, b: Aabb, me: Option<Entity>, tiles_only: bool) -> bool {
        let size = self.size();
        if b.min.x < 0.0 || b.min.y < 0.0 || b.max.x > size.x || b.max.y > size.y {
            return true;
        }
        let (tx0, ty0) = ((b.min.x / T).floor() as i32, (b.min.y / T).floor() as i32);
        let (tx1, ty1) = (((b.max.x - 0.001) / T).floor() as i32, ((b.max.y - 0.001) / T).floor() as i32);
        for ty in ty0..=ty1 {
            for tx in tx0..=tx1 {
                if self.tile_solid(tx, ty) {
                    return true;
                }
            }
        }
        if tiles_only {
            return false;
        }
        if self.bodies.iter().flatten().any(|(a, on)| *on && a.overlaps(&b)) {
            return true;
        }
        self.actors.iter().any(|(e, a)| Some(*e) != me && a.overlaps(&b))
    }

    /// Move a box by `delta`, sliding along whatever it hits. Returns the
    /// new position and whether movement was blocked on each axis.
    pub fn slide(&self, me: Option<Entity>, pos: Vec2, hw: f32, h: f32, delta: Vec2, tiles_only: bool) -> (Vec2, bool, bool) {
        let steps = (delta.length() / 2.0).ceil().max(1.0) as usize;
        let step = delta / steps as f32;
        let mut p = pos;
        let (mut bx, mut by) = (false, false);
        for _ in 0..steps {
            if step.x != 0.0 && !bx {
                let (np, hit) = self.advance(me, p, hw, h, Vec2::new(step.x, 0.0), tiles_only);
                p = np;
                bx |= hit;
            }
            if step.y != 0.0 && !by {
                let (np, hit) = self.advance(me, p, hw, h, Vec2::new(0.0, step.y), tiles_only);
                p = np;
                by |= hit;
            }
        }
        (p, bx, by)
    }

    fn advance(&self, me: Option<Entity>, p: Vec2, hw: f32, h: f32, d: Vec2, tiles_only: bool) -> (Vec2, bool) {
        if !self.blocked_by(Aabb::feet(p + d, hw, h), me, tiles_only) {
            return (p + d, false);
        }
        // creep up to the contact point
        let (mut lo, mut hi) = (0.0f32, 1.0f32);
        for _ in 0..5 {
            let mid = (lo + hi) / 2.0;
            if self.blocked_by(Aabb::feet(p + d * mid, hw, h), me, tiles_only) {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        (p + d * lo, true)
    }
}

/// Where the player appears.
pub enum Arrival<'a> {
    Spawn(&'a str),
    At { x: f32, y: f32, facing: Facing },
}

pub struct SpawnCtx<'a, 'w, 's> {
    pub commands: &'a mut Commands<'w, 's>,
    pub gfx: &'a mut Gfx,
    pub images: &'a mut Assets<Image>,
    pub materials: &'a mut Assets<GlowMaterial>,
}

fn light_levels(cx: &mut SpawnCtx, tex: &str, alpha: f32, flicker: bool, pulse: bool) -> Vec<Handle<GlowMaterial>> {
    let factors: &[f32] = if flicker {
        &[0.82, 0.88, 0.94, 1.0, 1.04]
    } else if pulse {
        &[0.5, 0.6, 0.7, 0.8, 0.9, 1.0]
    } else {
        &[1.0]
    };
    factors.iter().filter_map(|f| cx.gfx.glow_material(cx.materials, tex, alpha * f)).collect()
}

fn spawn_light(cx: &mut SpawnCtx, tex: &str, at: Vec2, scale: f32, alpha: f32, flicker: bool, pulse: bool, object: Option<usize>, follow: Option<(Entity, Vec2)>) -> Option<Entity> {
    let levels = light_levels(cx, tex, alpha, flicker, pulse);
    let first = levels.last()?.clone();
    let size = cx.gfx.glow_size(cx.images, tex) * scale;
    Some(
        cx.commands
            .spawn((
                MapEntity,
                Mesh2d(cx.gfx.quad.clone()),
                MeshMaterial2d(first),
                Transform::from_translation(to_world(at.x.round(), at.y.round(), Z_LIGHTS)).with_scale(size.extend(1.0)),
                Glow { levels, flicker, pulse, follow, object },
            ))
            .id(),
    )
}

/// Build every entity of a map. The caller decides when (behind a fade).
pub fn spawn_level(cx: &mut SpawnCtx, map: MapDef, state: &GameState, arrival: Arrival) -> Level {
    let map = Arc::new(map);
    let tint = map.tint;
    let tint_c = tint_color(tint);
    // tiles
    let (a, b) = crate::gfx::bake_tiles(&map);
    let la = cx.images.add(image_from_canvas(&a));
    cx.commands.spawn((MapEntity, TileLayer { alt: false }, Sprite { image: la, color: tint_c, ..default() }, Anchor::TOP_LEFT, Transform::from_translation(to_world(0.0, 0.0, Z_TILES))));
    if let Some(b) = b {
        let lb = cx.images.add(image_from_canvas(&b));
        cx.commands.spawn((MapEntity, TileLayer { alt: true }, Sprite { image: lb, color: tint_c, ..default() }, Anchor::TOP_LEFT, Transform::from_translation(to_world(0.0, 0.0, Z_TILES + 0.1)), Visibility::Hidden));
    }
    let solid: Vec<bool> = map.tiles.iter().flatten().map(|n| cpl_art::tiles::is_solid(n)).collect();

    // props
    let mut objects = Vec::new();
    let mut bodies = Vec::new();
    let mut object_visible = Vec::new();
    for (idx, o) in map.objects.iter().enumerate() {
        let visible = state.cond(o.show_if);
        let Some(sheet) = cx.gfx.object(o.tex).cloned() else {
            warn!("missing texture {} for {}", o.tex, o.id);
            objects.push(Entity::PLACEHOLDER);
            bodies.push(None);
            object_visible.push(false);
            continue;
        };
        let mut frame = o.frame;
        let mut rule_frame = None;
        if !o.frame_if.is_empty() {
            for (cond, f) in &o.frame_if {
                if state.cond(Some(cond)) {
                    frame = *f;
                }
            }
            rule_frame = Some(frame);
        }
        let mut sprite = sheet.sprite(frame);
        sprite.color = Color::from(tint_c.to_srgba().with_alpha(o.alpha));
        let mut pos = Pos::new(o.x, o.y, sheet.size.x);
        pos.bias = o.depth;
        pos.floor = o.floor;
        let mut e = cx.commands.spawn((MapEntity, Obj { idx, rule_frame, pinned: false }, sprite, Anchor::BOTTOM_CENTER, Transform::from_translation(pos.translation()), pos, if visible { Visibility::Inherited } else { Visibility::Hidden }));
        if o.anim
            && let Some(rate) = sheet.rate
        {
            let start = (idx * 7) % sheet.frames.max(1);
            let frames: Vec<usize> = (0..sheet.frames).map(|i| (i + start) % sheet.frames).collect();
            e.insert(Looping { frames, fps: rate, t: 0.0 });
        }
        objects.push(e.id());
        bodies.push(o.body.map(|b| {
            let aabb = Aabb { min: Vec2::new(o.x - b.w / 2.0, o.y - b.dy - b.h), max: Vec2::new(o.x + b.w / 2.0, o.y - b.dy) };
            (aabb, visible)
        }));
        object_visible.push(visible);
        if let Some(l) = &o.light {
            let dy = l.dy.unwrap_or(-(sheet.size.y as f32) / 2.0);
            if let Some(le) = spawn_light(cx, l.tex, Vec2::new(o.x + l.dx, o.y + dy), l.scale, l.alpha, l.flicker, l.pulse, Some(idx), None)
                && !visible
            {
                cx.commands.entity(le).insert(Visibility::Hidden);
            }
        }
    }

    // people
    let mut npcs = Vec::new();
    for (idx, n) in map.npcs.iter().enumerate() {
        let visible = state.cond(n.show_if);
        let sheet = cx.gfx.char_sheet(n.char).clone();
        let mut sprite = sheet.sprite(idle_frame(n.facing));
        sprite.color = if n.hologram { mul(tint_c, 0x73eff7, 0.78) } else { tint_c };
        let pos = Pos::new(n.x, n.y, sheet.size.x);
        let id = cx
            .commands
            .spawn((
                MapEntity,
                Npc { idx, target: None, wait: 1.0 + (idx as f32 * 0.37).fract() * 3.0, visible },
                Walker::new(n.facing),
                sprite,
                Anchor::BOTTOM_CENTER,
                Transform::from_translation(pos.translation()),
                pos,
                if visible { Visibility::Inherited } else { Visibility::Hidden },
            ))
            .id();
        if n.hologram {
            spawn_light(cx, "glow-cyan", Vec2::new(n.x, n.y - 10.0), 0.8, 0.45, false, false, None, Some((id, Vec2::new(0.0, -10.0))));
        } else {
            spawn_shadow(cx, id, n.x, n.y);
        }
        npcs.push(id);
    }

    // the player
    let (start, facing) = match arrival {
        Arrival::Spawn(name) => {
            let s = map.spawn(name);
            (Vec2::new(s.x, s.y), s.facing)
        }
        Arrival::At { x, y, facing } => (Vec2::new(x, y), facing),
    };
    let look = cpl_art::characters::player_looks().get(state.data.look).map(|d| d.id).unwrap_or("player-a");
    let sheet = cx.gfx.char_sheet(look).clone();
    let mut sprite = sheet.sprite(idle_frame(facing));
    sprite.color = tint_c;
    let pos = Pos::new(start.x, start.y, sheet.size.x);
    let player = cx.commands.spawn((MapEntity, Player, Walker::new(facing), sprite, Anchor::BOTTOM_CENTER, Transform::from_translation(pos.translation()), pos)).id();
    spawn_shadow(cx, player, start.x, start.y);

    // the prompt
    let mut prompt = cx.gfx.prompt.sprite(0);
    prompt.color = Color::WHITE;
    cx.commands.spawn((MapEntity, Prompt, prompt, Anchor::BOTTOM_CENTER, Transform::from_translation(to_world(0.0, 0.0, Z_PROMPT)), Visibility::Hidden, Looping { frames: vec![0, 1], fps: 2.5, t: 0.0 }));

    let mut level = Level { map: Some(map), solid, bodies, object_visible, objects, npcs, cat: None, actors: Vec::new(), water_t: 0.0 };
    spawn_cat(cx, &mut level, state, start);
    level
}

fn spawn_shadow(cx: &mut SpawnCtx, owner: Entity, x: f32, y: f32) {
    cx.commands.spawn((MapEntity, Shadow(owner), Sprite { image: cx.gfx.shadow.clone(), color: Color::srgba(1.0, 1.0, 1.0, 0.9), ..default() }, Anchor::CENTER, Transform::from_translation(to_world(x.round(), y.round() - 1.0, Z_SHADOW))));
}

/// Nonce appears once you have met her (in follow mode) or always (wandering).
pub fn spawn_cat(cx: &mut SpawnCtx, level: &mut Level, state: &GameState, near: Vec2) {
    let Some(m) = level.map.clone() else { return };
    let Some(mode) = m.cat_mode else { return };
    if level.cat.is_some() || (mode == CatMode::Follow && !state.flag("pro.metCat")) {
        return;
    }
    let start = match mode {
        CatMode::Follow => near + Vec2::new(-18.0, 4.0),
        CatMode::Wander => m.cat_area.map(|a| Vec2::new(a.x + 40.0, a.y + 40.0)).unwrap_or(Vec2::new(140.0, 140.0)),
    };
    let mut sprite = cx.gfx.cat.sprite(0);
    sprite.color = tint_color(m.tint);
    let pos = Pos::new(start.x, start.y, 16);
    let mut w = Walker::new(Facing::Down);
    w.h = 5.0;
    let cat = cx.commands.spawn((MapEntity, Cat { target: None, wait: 2.0, last_left: true }, w, sprite, Anchor::BOTTOM_CENTER, Transform::from_translation(pos.translation()), pos)).id();
    spawn_shadow(cx, cat, start.x, start.y);
    level.cat = Some(cat);
}

pub fn despawn_level(commands: &mut Commands, q: &Query<Entity, With<MapEntity>>, level: &mut Level) {
    for e in q.iter() {
        commands.entity(e).despawn();
    }
    *level = Level::default();
}

/// Keep props and people in step with the story's flags.
pub fn refresh(level: &mut Level, state: &GameState, objs: &mut Query<(&mut Obj, &mut Sprite, &mut Visibility), Without<Npc>>, npcs: &mut Query<(&mut Npc, &mut Visibility), Without<Obj>>, glows: &mut Query<(&Glow, &mut Visibility), (Without<Obj>, Without<Npc>)>) {
    let Some(map) = level.map.clone() else { return };
    for (idx, o) in map.objects.iter().enumerate() {
        let visible = state.cond(o.show_if);
        let Some(&e) = level.objects.get(idx) else { continue };
        if e == Entity::PLACEHOLDER {
            continue;
        }
        if let Ok((mut obj, mut sprite, mut vis)) = objs.get_mut(e) {
            vis.set_if_neq(if visible { Visibility::Inherited } else { Visibility::Hidden });
            if !o.frame_if.is_empty() && !o.anim {
                let mut frame = o.frame;
                for (cond, f) in &o.frame_if {
                    if state.cond(Some(cond)) {
                        frame = *f;
                    }
                }
                if obj.rule_frame != Some(frame) {
                    obj.rule_frame = Some(frame);
                    obj.pinned = false;
                    if let Some(atlas) = sprite.texture_atlas.as_mut() {
                        atlas.index = frame;
                    }
                }
            }
        }
        level.object_visible[idx] = visible;
        if let Some(Some((_, on))) = level.bodies.get_mut(idx) {
            *on = visible;
        }
    }
    for (idx, n) in map.npcs.iter().enumerate() {
        let visible = state.cond(n.show_if);
        if let Some(&e) = level.npcs.get(idx)
            && let Ok((mut npc, mut vis)) = npcs.get_mut(e)
        {
            npc.visible = visible;
            vis.set_if_neq(if visible { Visibility::Inherited } else { Visibility::Hidden });
        }
    }
    for (g, mut vis) in glows.iter_mut() {
        if let Some(idx) = g.object {
            let v = level.object_visible.get(idx).copied().unwrap_or(true);
            vis.set_if_neq(if v { Visibility::Inherited } else { Visibility::Hidden });
        }
    }
}

/// System: apply story flags to the world, and let Nonce join you once met.
#[allow(clippy::too_many_arguments)]
pub fn refresh_system(
    mut commands: Commands,
    game: NonSend<crate::director::GameRes>,
    mut level: ResMut<Level>,
    mut gfx: ResMut<Gfx>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<GlowMaterial>>,
    mut objs: Query<(&mut Obj, &mut Sprite, &mut Visibility), Without<Npc>>,
    mut npcs: Query<(&mut Npc, &mut Visibility), Without<Obj>>,
    mut glows: Query<(&Glow, &mut Visibility), (Without<Obj>, Without<Npc>)>,
    player: Query<&Pos, With<Player>>,
) {
    if level.map.is_none() {
        return;
    }
    let state = game.ctx.state_ref();
    refresh(&mut level, &state, &mut objs, &mut npcs, &mut glows);
    if level.cat.is_none()
        && level.map.as_ref().is_some_and(|m| m.cat_mode == Some(CatMode::Follow))
        && state.flag("pro.metCat")
        && let Ok(p) = player.single()
    {
        let near = p.p;
        let mut cx = SpawnCtx { commands: &mut commands, gfx: &mut gfx, images: &mut images, materials: &mut materials };
        spawn_cat(&mut cx, &mut level, &state, near);
    }
}
