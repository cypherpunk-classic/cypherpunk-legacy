//! Cypherpunk Legacy: an educational pixel-art adventure through real
//! cryptography, on Bevy. The story, puzzles and interface live in
//! `cpl-logic`; this crate is the engine side: the world you walk around,
//! the camera, sound, saving, and the autopilot.

mod actors;
mod autopilot;
mod director;
mod fx;
mod gfx;
mod input;
mod platform;
mod sound;
mod title;
mod view;
mod world;

use bevy::prelude::*;
use bevy_egui::EguiPlugin;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
enum Phase {
    Input,
    Story,
    Move,
    React,
    Present,
}

fn main() {
    let launch = platform::launch();
    let game = director::build_game(&launch);
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Cypherpunk Legacy".into(),
                        canvas: Some("#game".into()),
                        fit_canvas_to_parent: true,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(EguiPlugin::default())
        .insert_resource(ClearColor(Color::srgb_u8(7, 8, 14)))
        .insert_non_send(director::GameRes(game))
        .insert_resource(autopilot::Autopilot::from_launch(&launch))
        .insert_resource(actors::Pace(launch.speed.max(0.1)))
        .init_resource::<world::Level>()
        .add_plugins((gfx::GfxPlugin, view::ViewPlugin, director::DirectorPlugin, input::InputPlugin, fx::FxPlugin, sound::SoundPlugin, actors::ActorsPlugin, autopilot::AutopilotPlugin))
        .configure_sets(Update, (Phase::Input, Phase::Story, Phase::Move, Phase::React, Phase::Present).chain())
        .add_systems(Startup, start_title)
        .add_systems(Update, (autopilot::drive, input::gather).chain().in_set(Phase::Input))
        .add_systems(Update, director::run.in_set(Phase::Story))
        .add_systems(Update, (world::refresh_system, actors::gather_bodies, actors::move_player, actors::npc_ai, actors::cat_ai).chain().in_set(Phase::Move))
        .add_systems(Update, (actors::interact, actors::zones, actors::animate_walkers, actors::sync_positions).chain().in_set(Phase::React))
        .add_systems(Update, (view::update_camera, fx::particles, fx::loop_anims, fx::lights, fx::water, title::animate, sound::play).in_set(Phase::Present))
        .run();
}

fn start_title(mut commands: Commands, mut gfx: ResMut<gfx::Gfx>, mut images: ResMut<Assets<Image>>, mut materials: ResMut<Assets<gfx::GlowMaterial>>) {
    title::spawn(&mut commands, &mut gfx, &mut images, &mut materials);
}
