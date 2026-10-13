//! Small things that make the world feel alive: looping prop animations,
//! flickering lights, animated water, drifting motes, rain, and bursts of
//! light when something important happens.

use bevy::prelude::*;
use cpl_logic::maps::Ambient;

use crate::director::GameRes;
use crate::gfx::Gfx;
use crate::title::TitleEntity;
use crate::world::{Glow, Level, Looping, MapEntity, Pos, TileLayer, Z_PARTICLES, to_world};

#[derive(Resource)]
pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> f32 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        ((x.wrapping_mul(0x2545F4914F6CDD1D) >> 40) as f32) / (1u64 << 24) as f32
    }

    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.next()
    }

    pub fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[((self.next() * items.len() as f32) as usize).min(items.len() - 1)]
    }
}

impl Default for Rng {
    fn default() -> Self {
        Self(0x9E37_79B9_7F4A_7C15)
    }
}

/// Bursts requested by the story, spawned next frame.
#[derive(Resource, Default)]
pub struct Particles {
    pub bursts: Vec<(Vec2, u32, u32)>,
}

impl Particles {
    pub fn clear(&mut self) {
        self.bursts.clear();
    }
}

pub fn burst(p: &mut Particles, at: Vec2, color: u32, n: u32) {
    p.bursts.push((at, color, n.min(200)));
}

#[derive(Component)]
pub struct Particle {
    pub p: Vec2,
    pub vel: Vec2,
    pub age: f32,
    pub life: f32,
    pub alpha: (f32, f32),
    pub size: (f32, f32),
    pub color: Srgba,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EmitterKind {
    Motes,
    Dust,
    Rain,
    TitleMotes,
}

#[derive(Component)]
pub struct Emitter {
    pub kind: EmitterKind,
    pub t: f32,
    /// Area to spawn in, map space.
    pub area: Rect,
}

fn hex(c: u32) -> Srgba {
    Srgba::rgb_u8((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

pub fn spawn_ambient(commands: &mut Commands, level: &Level) {
    let Some(m) = &level.map else { return };
    let size = level.size();
    let kind = match m.ambient {
        Some(Ambient::Motes) => EmitterKind::Motes,
        Some(Ambient::Dust) => EmitterKind::Dust,
        None => return,
    };
    commands.spawn((MapEntity, Emitter { kind, t: 0.0, area: Rect::new(0.0, 0.0, size.x, size.y) }));
}

#[allow(clippy::too_many_arguments)]
pub fn particles(
    mut commands: Commands,
    time: Res<Time>,
    gfx: Res<Gfx>,
    mut rng: ResMut<Rng>,
    mut requests: ResMut<Particles>,
    game: NonSend<GameRes>,
    mut emitters: Query<(&mut Emitter, Has<TitleEntity>)>,
    mut q: Query<(Entity, &mut Particle, &mut Transform, &mut Sprite)>,
) {
    let dt = time.delta_secs().min(0.05);
    let calm = game.ctx.state_ref().settings.reduced_motion;
    let spawn = |commands: &mut Commands, rng: &mut Rng, title: bool, p: Particle, image: Handle<Image>| {
        let mut sprite = Sprite::from_image(image);
        sprite.color = Color::from(p.color.with_alpha(p.alpha.0));
        let tf = Transform::from_translation(to_world(p.p.x, p.p.y, Z_PARTICLES + rng.next() * 0.5)).with_scale(Vec3::splat(p.size.0));
        if title {
            commands.spawn((TitleEntity, p, sprite, tf));
        } else {
            commands.spawn((MapEntity, p, sprite, tf));
        }
    };
    for (at, color, n) in requests.bursts.drain(..) {
        for _ in 0..n {
            let a = rng.range(0.0, std::f32::consts::TAU);
            let s = rng.range(20.0, 70.0);
            let p = Particle { p: at, vel: Vec2::new(a.cos(), a.sin()) * s, age: 0.0, life: 0.9, alpha: (1.0, 0.0), size: (1.0, 0.0), color: hex(color) };
            spawn(&mut commands, &mut rng, false, p, gfx.dot.clone());
        }
    }
    for (mut e, title) in emitters.iter_mut() {
        e.t += dt;
        let (every, per) = match e.kind {
            EmitterKind::Motes => (0.09, 1),
            EmitterKind::Dust => (0.4, 1),
            EmitterKind::Rain => (0.03, 2),
            EmitterKind::TitleMotes => (0.16, 1),
        };
        let every = if calm { every * 3.0 } else { every };
        while e.t >= every {
            e.t -= every;
            for _ in 0..per {
                let a = e.area;
                let at = Vec2::new(rng.range(a.min.x, a.max.x), rng.range(a.min.y, a.max.y));
                let (p, image) = match e.kind {
                    EmitterKind::Motes => {
                        let s = rng.range(0.5, 1.0);
                        let c = rng.pick(&[0x73eff7, 0xff3e9a, 0xc792ff, 0xa7f070]);
                        (Particle { p: at, vel: Vec2::new(rng.range(-3.0, 3.0), rng.range(-10.0, -3.0)), age: 0.0, life: rng.range(3.0, 7.0), alpha: (0.75, 0.0), size: (s, s), color: hex(c) }, gfx.dot.clone())
                    }
                    EmitterKind::Dust => (Particle { p: at, vel: Vec2::new(rng.range(-2.0, 2.0), rng.range(-2.0, 2.0)), age: 0.0, life: rng.range(4.0, 8.0), alpha: (0.35, 0.0), size: (0.5, 0.5), color: hex(0xffcd75) }, gfx.dot.clone()),
                    EmitterKind::Rain => (Particle { p: Vec2::new(at.x, a.min.y), vel: Vec2::new(rng.range(-30.0, -10.0), rng.range(220.0, 300.0)), age: 0.0, life: 1.8, alpha: (0.6, 0.2), size: (1.0, 1.0), color: Srgba::WHITE }, gfx.raindrop.clone()),
                    EmitterKind::TitleMotes => {
                        let c = rng.pick(&[0x73eff7, 0xff3e9a, 0xc792ff]);
                        (Particle { p: at, vel: Vec2::new(0.0, rng.range(-8.0, -2.0)), age: 0.0, life: 6.0, alpha: (0.7, 0.0), size: (0.5, 0.5), color: hex(c) }, gfx.dot.clone())
                    }
                };
                spawn(&mut commands, &mut rng, title, p, image);
            }
        }
    }
    for (e, mut p, mut tf, mut sprite) in q.iter_mut() {
        p.age += dt;
        if p.age >= p.life {
            commands.entity(e).despawn();
            continue;
        }
        let k = p.age / p.life;
        let v = p.vel;
        p.p += v * dt;
        let z = tf.translation.z;
        tf.translation = to_world(p.p.x.round(), p.p.y.round(), z);
        tf.scale = Vec3::splat(p.size.0 + (p.size.1 - p.size.0) * k);
        let a = p.alpha.0 + (p.alpha.1 - p.alpha.0) * k;
        sprite.color = Color::from(p.color.with_alpha(a));
    }
}

/// Props that loop on their own (fire, screens, signs, windows).
pub fn loop_anims(time: Res<Time>, mut q: Query<(&mut Looping, &mut Sprite)>) {
    let dt = time.delta_secs();
    for (mut l, mut sprite) in q.iter_mut() {
        l.t += dt;
        if l.frames.is_empty() {
            continue;
        }
        let i = l.frames[((l.t * l.fps) as usize) % l.frames.len()];
        if let Some(a) = sprite.texture_atlas.as_mut()
            && a.index != i
        {
            a.index = i;
        }
    }
}

/// Lights: flicker, pulse, follow their hologram.
pub fn lights(time: Res<Time>, mut rng: ResMut<Rng>, mut q: Query<(&Glow, &mut MeshMaterial2d<crate::gfx::GlowMaterial>, &mut Transform, &mut Visibility)>, owners: Query<(&Pos, &Visibility), Without<Glow>>, mut tick: Local<f32>) {
    *tick += time.delta_secs();
    let flick = *tick > 1.0 / 15.0;
    if flick {
        *tick = 0.0;
    }
    let t = time.elapsed_secs();
    for (g, mut mat, mut tf, mut vis) in q.iter_mut() {
        if let Some((owner, off)) = g.follow
            && let Ok((p, v)) = owners.get(owner)
        {
            tf.translation = to_world((p.p.x + off.x).round(), (p.p.y + off.y).round(), tf.translation.z);
            vis.set_if_neq(*v);
        }
        if g.levels.len() < 2 {
            continue;
        }
        let idx = if g.flicker {
            if !flick {
                continue;
            }
            (rng.next() * g.levels.len() as f32) as usize
        } else if g.pulse {
            // 0.75 ± 0.25, as levels 0.5 ..= 1.0 in steps of 0.1
            let f = 0.75 + 0.25 * (t / 0.42).sin();
            ((f - 0.5) / 0.1).round() as usize
        } else {
            continue;
        }
        .min(g.levels.len() - 1);
        if mat.0 != g.levels[idx] {
            mat.0 = g.levels[idx].clone();
        }
    }
}

/// Water swaps between its two frames.
pub fn water(time: Res<Time>, mut level: ResMut<Level>, mut q: Query<(&TileLayer, &mut Visibility)>) {
    level.water_t += time.delta_secs();
    if level.water_t < 0.65 {
        return;
    }
    level.water_t = 0.0;
    let mut alt_on = false;
    for (t, v) in q.iter() {
        if t.alt {
            alt_on = *v != Visibility::Hidden;
        }
    }
    for (t, mut v) in q.iter_mut() {
        if t.alt {
            *v = if alt_on { Visibility::Hidden } else { Visibility::Inherited };
        }
    }
}

pub struct FxPlugin;

impl Plugin for FxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Rng>().init_resource::<Particles>();
    }
}
