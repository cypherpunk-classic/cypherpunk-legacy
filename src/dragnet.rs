//! The Dragnet in the world: each sensor's field drawn on the ground (masts
//! pulse, camera cones sweep), the trace advanced while you are free to walk,
//! a ping when a sensor picks you up, and the agent called in at 100. The
//! rules themselves live in `cpl_logic::dragnet`, so the headless tests and
//! this module agree on who sees what.

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;
use cpl_logic::game::Mode;
use cpl_logic::maps::SensorKind;

use crate::director::{Director, GameRes};
use crate::gfx::{Additive, GlowMaterial};
use crate::sound::SfxQueue;
use crate::world::{Level, MapEntity, Player, Pos, Z_FLOOR, to_world};

/// The Directorate's red, as an additive light.
const RED: (f32, f32, f32) = (1.0, 0.22, 0.32);
const Z_FIELDS: f32 = Z_FLOOR + 0.4;
/// Seconds per mast pulse.
const PULSE: f32 = 1.8;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Part {
    /// the whole field, faint
    Fill,
    /// a mast's rim, or the bright core near a camera's lens
    Edge,
    /// a ring running outwards from a mast
    Pulse,
}

/// One drawn piece of a sensor's field.
#[derive(Component)]
pub struct Field {
    sensor: usize,
    part: Part,
    material: Handle<GlowMaterial>,
    alpha: f32,
}

/// Which level the fields were built for, and which sensors saw the player last frame.
#[derive(Resource, Default)]
pub struct DragnetView {
    built_for: u64,
    seen: Vec<&'static str>,
}

fn material(materials: &mut Assets<GlowMaterial>) -> Handle<GlowMaterial> {
    materials.add(GlowMaterial { base: ColorMaterial { color: Color::srgba(RED.0, RED.1, RED.2, 0.0), ..default() }, extension: Additive {} })
}

/// Build the fields once per map (they are map entities, so they go when it does).
pub fn spawn_fields(mut commands: Commands, level: Res<Level>, mut view: ResMut<DragnetView>, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<GlowMaterial>>) {
    let Some(map) = level.map.as_ref() else { return };
    if view.built_for == level.serial {
        return;
    }
    view.built_for = level.serial;
    view.seen.clear();
    for (i, s) in map.sensors.iter().enumerate() {
        let at = to_world(s.x.round(), s.y.round(), Z_FIELDS);
        let mut piece = |mesh: Mesh, part: Part, z: f32| {
            let m = material(&mut materials);
            commands.spawn((
                MapEntity,
                Field { sensor: i, part, material: m.clone(), alpha: 0.0 },
                Mesh2d(meshes.add(mesh)),
                MeshMaterial2d(m),
                Transform::from_translation(at + Vec3::Z * z),
                Visibility::Hidden,
            ));
        };
        match s.kind {
            SensorKind::Mast { radius } => {
                piece(Circle::new(radius).mesh().resolution(48).build(), Part::Fill, 0.0);
                piece(Annulus::new(radius - 1.5, radius).mesh().resolution(48).build(), Part::Edge, 0.01);
                piece(Annulus::new(radius - 3.0, radius).mesh().resolution(48).build(), Part::Pulse, 0.02);
            }
            SensorKind::Camera { range, half_angle, .. } => {
                piece(CircularSector::new(range, half_angle).mesh().resolution(24).build(), Part::Fill, 0.0);
                piece(CircularSector::new(range * 0.4, half_angle).mesh().resolution(12).build(), Part::Edge, 0.01);
            }
        }
    }
}

/// Animate the fields and run the trace.
#[allow(clippy::too_many_arguments)]
pub fn update(
    time: Res<Time>,
    mut game: NonSendMut<GameRes>,
    director: Res<Director>,
    level: Res<Level>,
    mut view: ResMut<DragnetView>,
    mut sfx: ResMut<SfxQueue>,
    player: Query<&Pos, With<Player>>,
    mut fields: Query<(&mut Field, &mut Transform, &mut Visibility)>,
    mut materials: ResMut<Assets<GlowMaterial>>,
) {
    let Some(map) = level.map.clone() else { return };
    if map.sensors.is_empty() {
        return;
    }
    let t = time.elapsed_secs();
    let dt = time.delta_secs().min(0.1);
    // on the Wharf the street keeps watching while you work a board
    let free = game.mode == Mode::World && (!game.busy() || game.at_board_only()) && director.transition.is_none();

    // the trace (only while you can walk; a conversation stops the clock)
    if free && let Ok(p) = player.single() {
        let (x, y) = (p.p.x, p.p.y - 6.0);
        let extra = game.district.as_ref().map(|d| ("w-guard", d.sig.exposure(&game.ctx.state_ref(), x, y)));
        let (tick, on) = {
            let mut s = game.ctx.state();
            let tick = s.dragnet_with(&map, x, y, t, dt, extra);
            (tick, s.settings.dragnet)
        };
        if let Some(d) = game.district.as_mut() {
            d.seen = !tick.seen.is_empty();
        }
        if on && tick.seen.iter().any(|id| !view.seen.contains(id)) {
            sfx.0.push("ping");
        }
        view.seen = tick.seen;
        if tick.flagged {
            game.flagged(p.p.x, p.p.y);
        }
    } else if !free {
        view.seen.clear();
    }

    // the fields
    let state = game.ctx.state_ref();
    let calm = state.settings.reduced_motion;
    for (mut f, mut tf, mut vis) in fields.iter_mut() {
        let Some(s) = map.sensors.get(f.sensor) else { continue };
        let active = state.cond(s.show_if);
        let want = if active { Visibility::Inherited } else { Visibility::Hidden };
        if *vis != want {
            *vis = want;
        }
        if !active {
            continue;
        }
        let hot = view.seen.contains(&s.id);
        let alpha = match (s.kind, f.part) {
            (_, Part::Fill) => {
                if hot {
                    0.16
                } else {
                    0.06
                }
            }
            (SensorKind::Mast { .. }, Part::Edge) => {
                if hot {
                    0.6
                } else {
                    0.3
                }
            }
            (SensorKind::Camera { .. }, Part::Edge) => {
                if hot {
                    0.2
                } else {
                    0.08
                }
            }
            (_, Part::Pulse) => {
                if calm {
                    0.0
                } else {
                    let phase = (t / PULSE + f.sensor as f32 * 0.37).fract();
                    tf.scale = Vec3::splat(0.15 + 0.85 * phase);
                    0.35 * (1.0 - phase) * if hot { 1.6 } else { 1.0 }
                }
            }
        };
        if let SensorKind::Camera { .. } = s.kind {
            // the mesh points along +y; the map's angle is clockwise from east (y down)
            tf.rotation = Quat::from_rotation_z(-s.facing(t) - FRAC_PI_2);
        }
        if (alpha - f.alpha).abs() > 0.004
            && let Some(mut m) = materials.get_mut(&f.material)
        {
            m.base.color = Color::srgba(RED.0, RED.1, RED.2, alpha);
            f.alpha = alpha;
        }
    }
}

pub struct DragnetPlugin;

impl Plugin for DragnetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DragnetView>();
    }
}
