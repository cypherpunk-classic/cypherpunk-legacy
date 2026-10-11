//! Behind the title menu: Cryptopolis at night, drifting slowly, built from
//! the same sprites as the game itself.

use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::fx::{Emitter, EmitterKind};
use crate::gfx::{Gfx, GlowMaterial};
use crate::world::{Looping, to_world};

#[derive(Component)]
pub struct TitleEntity;

/// Parallax layers sway sideways; `amp` is the swing in pixels.
#[derive(Component)]
pub struct Sway {
    pub base: Vec3,
    pub amp: f32,
}

/// Drones drift back and forth.
#[derive(Component)]
pub struct Drift {
    pub base: Vec2,
    pub dx: f32,
    pub period: f32,
}

fn color(hex: u32, a: f32) -> Color {
    Color::srgba_u8((hex >> 16) as u8, (hex >> 8) as u8, hex as u8, (a * 255.0) as u8)
}

pub fn spawn(commands: &mut Commands, gfx: &mut Gfx, images: &mut Assets<Image>, materials: &mut Assets<GlowMaterial>) {
    // stars
    let mut seed = 0x1234_5678u32;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed % 100_000) as f32 / 100_000.0
    };
    for _ in 0..140 {
        let c = [0x73eff7, 0xdfe8ff, 0xc792ff, 0xffffff][(rnd() * 4.0) as usize % 4];
        let (x, y) = ((rnd() * 1000.0 - 500.0).round(), (rnd() * 300.0 - 260.0).round());
        let mut s = Sprite::from_image(gfx.dot.clone());
        s.color = color(c, 0.2 + rnd() * 0.7);
        s.custom_size = Some(Vec2::ONE);
        commands.spawn((TitleEntity, s, Anchor::TOP_LEFT, Transform::from_translation(to_world(x, y, 1.0))));
    }
    // the moon
    if let Some(m) = gfx.glow_material(materials, "glow-white", 0.3) {
        let size = gfx.glow_size(images, "glow-white") * 1.4;
        commands.spawn((TitleEntity, Mesh2d(gfx.quad.clone()), MeshMaterial2d(m), Transform::from_translation(to_world(120.0, -170.0, 2.0)).with_scale(size.extend(1.0))));
    }
    // far skyline, tinted into the night
    let far = ["bld-tower", "bld-block_a", "bld-observatory", "bld-block_b", "bld-mint", "bld-tower", "bld-masks", "bld-block_a", "bld-exchange"];
    let mut x = -560.0;
    for (i, k) in far.iter().enumerate() {
        let Some(sheet) = gfx.object(k).cloned() else { continue };
        let mut s = sheet.sprite(i % sheet.frames.max(1));
        s.color = color(0x6070a0, 0.9);
        let base = to_world(x, 30.0, 3.0 + i as f32 * 0.01);
        let mut e = commands.spawn((TitleEntity, s, Anchor::BOTTOM_LEFT, Transform::from_translation(base), Sway { base, amp: 40.0 }));
        if let Some(rate) = sheet.rate {
            e.insert(Looping { frames: (0..sheet.frames).collect(), fps: rate, t: i as f32 * 0.7 });
        }
        x += sheet.size.x as f32 - 6.0;
    }
    // the street in front
    let near = ["bld-radioshack", "bld-boathouse", "bld-block_b", "bld-pavilion", "bld-exchange", "bld-radioshack", "bld-block_a", "bld-boathouse"];
    let mut x = -520.0;
    for (i, k) in near.iter().enumerate() {
        let Some(sheet) = gfx.object(k).cloned() else { continue };
        let s = sheet.sprite(0);
        let base = to_world(x, 70.0, 5.0 + i as f32 * 0.01);
        let mut e = commands.spawn((TitleEntity, s, Anchor::BOTTOM_LEFT, Transform::from_translation(base), Sway { base, amp: 70.0 }));
        if let Some(rate) = sheet.rate {
            e.insert(Looping { frames: (0..sheet.frames).collect(), fps: rate, t: i as f32 * 0.9 });
        }
        x += sheet.size.x as f32 + 10.0;
    }
    let signs = ["sign-wharf", "sign-exchange", "sign-mint", "sign-masks", "sign-observatory"];
    for (i, k) in signs.iter().enumerate() {
        let sx = -420.0 + i as f32 * 190.0;
        if let Some(sheet) = gfx.object(k).cloned() {
            let base = to_world(sx, 6.0, 6.0);
            let mut e = commands.spawn((TitleEntity, sheet.sprite(0), Anchor::BOTTOM_CENTER, Transform::from_translation(base), Sway { base, amp: 70.0 }));
            if let Some(rate) = sheet.rate {
                e.insert(Looping { frames: (0..sheet.frames).collect(), fps: rate, t: i as f32 });
            }
        }
        if let Some(m) = gfx.glow_material(materials, "glow-cyan", 0.25) {
            let size = gfx.glow_size(images, "glow-cyan");
            let base = to_world(sx, 0.0, 30.0);
            commands.spawn((TitleEntity, Mesh2d(gfx.quad.clone()), MeshMaterial2d(m), Transform::from_translation(base).with_scale(size.extend(1.0)), Sway { base, amp: 70.0 }));
        }
    }
    let street = Sprite { color: color(0x111421, 1.0), custom_size: Some(Vec2::new(1400.0, 80.0)), ..default() };
    let base = to_world(-700.0, 70.0, 4.5);
    commands.spawn((TitleEntity, street, Anchor::TOP_LEFT, Transform::from_translation(base), Sway { base, amp: 70.0 }));
    let mut lx = -680.0;
    while lx < 700.0 {
        let mark = Sprite { color: color(0xc9b458, 1.0), custom_size: Some(Vec2::new(18.0, 2.0)), ..default() };
        let base = to_world(lx, 100.0, 4.6);
        commands.spawn((TitleEntity, mark, Anchor::TOP_LEFT, Transform::from_translation(base), Sway { base, amp: 70.0 }));
        lx += 40.0;
    }
    // drones on patrol
    if let Some(sheet) = gfx.object("obj-drone").cloned() {
        for i in 0..3 {
            let base = Vec2::new(-300.0 + i as f32 * 260.0, -90.0 - i as f32 * 20.0);
            commands.spawn((
                TitleEntity,
                sheet.sprite(0),
                Anchor::CENTER,
                Transform::from_translation(to_world(base.x, base.y, 8.0)),
                Drift { base, dx: 160.0, period: 9.0 + i as f32 * 2.0 },
                Looping { frames: (0..sheet.frames).collect(), fps: sheet.rate.unwrap_or(12.0), t: 0.0 },
            ));
        }
    }
    commands.spawn((TitleEntity, Emitter { kind: EmitterKind::Rain, t: 0.0, area: Rect::new(-600.0, -280.0, 600.0, -280.0) }));
    commands.spawn((TitleEntity, Emitter { kind: EmitterKind::TitleMotes, t: 0.0, area: Rect::new(-500.0, -100.0, 500.0, 80.0) }));
}

pub fn animate(time: Res<Time>, mut sway: Query<(&Sway, &mut Transform), Without<Drift>>, mut drift: Query<(&Drift, &mut Transform), Without<Sway>>) {
    let t = time.elapsed_secs();
    let s = (t / 9.0).sin();
    for (sw, mut tf) in sway.iter_mut() {
        tf.translation.x = (sw.base.x + s * sw.amp).round();
    }
    for (d, mut tf) in drift.iter_mut() {
        let k = 0.5 - 0.5 * (t / d.period * std::f32::consts::TAU).cos();
        let y = d.base.y + 6.0 * (0.5 - 0.5 * (t / 1.2 * std::f32::consts::PI).cos());
        tf.translation = to_world((d.base.x + d.dx * k).round(), y.round(), tf.translation.z);
    }
}
