//! The v4 districts in the world: every signal board's lamps lit live on
//! its post (the Wharf), the guards walking the rounds the district's clock
//! gives them, the supply drone, and what the street can see of you (for
//! the close-up's header and the trace).

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::sprite::Anchor;
use cpl_crypto::lamps::{Bits, lamp};
use cpl_logic::signals::Glyph;
use cpl_logic::signals::wharf::MAST;
use cpl_logic::state::MapId;

use crate::director::GameRes;
use crate::gfx::Gfx;
use crate::world::{Level, MapEntity, Npc, Player, Pos, Walker, Z_LIGHTS, depth, facing_towards, to_world};

const ON: [u8; 4] = [255, 205, 117, 255];
const OFF: [u8; 4] = [21, 26, 38, 255];
const CLEAR: [u8; 4] = [164, 255, 224, 255];
const SEAL: [u8; 4] = [115, 239, 247, 255];

/// The live face of one board.
#[derive(Component)]
pub struct Lamps {
    board: &'static str,
    image: Handle<Image>,
    shown: Option<(Bits, bool)>,
}

#[derive(Component)]
pub struct Drone;

#[derive(Resource, Default)]
pub struct WharfView {
    built_for: u64,
}

fn face(bits: Bits, clear: bool, sealed: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(64 * 4);
    for y in 0..8 {
        for x in 0..8 {
            let px = if lamp(bits, x, y) {
                if clear {
                    CLEAR
                } else if sealed && y == 7 {
                    SEAL
                } else {
                    ON
                }
            } else {
                OFF
            };
            out.extend_from_slice(&px);
        }
    }
    out
}

/// Build the lamp faces and the drone once per map.
pub fn spawn(mut commands: Commands, level: Res<Level>, mut view: ResMut<WharfView>, gfx: Res<Gfx>, mut images: ResMut<Assets<Image>>, game: NonSend<GameRes>) {
    let Some(map) = level.map.as_ref() else { return };
    if view.built_for == level.serial {
        return;
    }
    view.built_for = level.serial;
    if map.id != MapId::Wharf {
        return;
    }
    let Some(d) = game.district.as_ref() else { return };
    for o in &map.objects {
        let Some(board) = o.id.strip_prefix("sig-") else { continue };
        let Some(b) = d.sig.board(board) else { continue };
        let image = images.add(Image::new(Extent3d { width: 8, height: 8, depth_or_array_layers: 1 }, TextureDimension::D2, face(0, false, false), TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::default()));
        // the sprite's dark face is 16 x 16 at (4, 4) of its 24 x 36 frame
        let at = to_world(o.x.round(), o.y.round() - 24.0, depth(o.y) + 0.01);
        commands.spawn((
            MapEntity,
            Lamps { board: b.id, image: image.clone(), shown: None },
            Sprite { image, custom_size: Some(Vec2::splat(16.0)), ..default() },
            Anchor::CENTER,
            Transform::from_translation(at),
        ));
    }
    if let Some(sheet) = gfx.object("obj-drone") {
        commands.spawn((MapEntity, Drone, sheet.sprite(0), Anchor::BOTTOM_CENTER, Transform::from_translation(to_world(0.0, 0.0, Z_LIGHTS - 1.0)), Visibility::Hidden));
    }
}

/// Keep the faces, the guard and the drone in step with the district.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn update(
    time: Res<Time>,
    mut game: NonSendMut<GameRes>,
    level: Res<Level>,
    mut images: ResMut<Assets<Image>>,
    mut lamps: Query<&mut Lamps>,
    mut npcs: Query<(&Npc, &mut Pos, &mut Walker), Without<Player>>,
    mut drone: Query<(&mut Transform, &mut Visibility, &mut Sprite), With<Drone>>,
    player: Query<&Pos, (With<Player>, Without<Npc>)>,
) {
    let Some(map) = level.map.clone() else { return };
    if !matches!(map.id, MapId::Wharf | MapId::Exchange) {
        return;
    }
    let game = &mut game.0;
    let state = game.ctx.state_ref();
    let Some(d) = game.district.as_mut() else { return };
    let free = state.flag(MAST);
    for mut l in lamps.iter_mut() {
        let Some(b) = d.sig.board(l.board) else { continue };
        // once the mast is ours every board in the district says his sign
        let (bits, clear) = if free {
            (Glyph::Sigil.bits(), true)
        } else {
            let shown = d.sig.shown(&state, l.board);
            (shown, b.reveal.is_some() && b.message().is_some_and(|g| g.bits() == shown))
        };
        if l.shown != Some((bits, clear)) {
            l.shown = Some((bits, clear));
            if let Some(mut img) = images.get_mut(&l.image) {
                img.data = Some(face(bits, clear, b.seal.is_some() && !free));
            }
        }
    }
    let poses = d.poses(&state);
    for (npc, mut pos, mut w) in npcs.iter_mut() {
        let def = &map.npcs[npc.idx];
        // anyone the district walks itself
        if let Some(p) = poses.iter().find(|p| p.id == def.id && !p.flying) {
            pos.p = Vec2::new(p.x, p.y);
            w.moving = p.dx != 0.0 || p.dy != 0.0;
            if w.moving {
                w.facing = facing_towards(Vec2::ZERO, Vec2::new(p.dx, p.dy));
            }
        }
    }
    if let Ok((mut tf, mut vis, mut sprite)) = drone.single_mut()
        && let Some(p) = poses.iter().find(|p| p.id == "w-drone")
    {
        let bob = (time.elapsed_secs() * 5.0).sin() * 1.5;
        tf.translation = to_world(p.x.round(), (p.y + bob).round(), Z_LIGHTS - 1.0);
        vis.set_if_neq(if p.visible { Visibility::Inherited } else { Visibility::Hidden });
        sprite.flip_x = p.dx > 0.0;
    }
    // what the street can see of you (the trace itself runs in the Dragnet)
    if let Ok(p) = player.single() {
        d.footsteps = d.footsteps_at(&state, p.p.x, p.p.y - 6.0);
    }
}
