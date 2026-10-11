//! Glue between the engine-independent game (`cpl-logic`) and the Bevy
//! world: runs the story each frame, draws its egui interface, and carries
//! out what it asks of the world (change maps, pan the camera, flash the
//! screen, write the save).

use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass};
use cpl_logic::game::{EngineReq, Game, Mode};
use cpl_logic::maps::map;
use cpl_logic::script::{Cmd, Reply};
use cpl_logic::state::{GameState, MapId, Pos as SavePos};
use cpl_logic::ui::hud::ToastKind;

use crate::fx::{self, Particles};
use crate::gfx::{Gfx, GlowMaterial};
use crate::platform;
use crate::title::{self, TitleEntity};
use crate::view::{CamMode, CameraCtl, Screen};
use crate::world::{self, Arrival, Level, MapEntity, Obj, Player, Pos, SpawnCtx, Walker};

/// The game itself. Not `Send` (story scripts share state through `Rc`), so
/// it lives on the main thread as a non-send resource.
pub struct GameRes(pub Game);

impl std::ops::Deref for GameRes {
    type Target = Game;
    fn deref(&self) -> &Game {
        &self.0
    }
}

impl std::ops::DerefMut for GameRes {
    fn deref_mut(&mut self) -> &mut Game {
        &mut self.0
    }
}

/// A map change in progress (fade out, swap, fade in).
pub struct Transition {
    pub to: Target,
    /// Goto requests waiting for the fade-in to finish.
    pub reply: Option<u64>,
    pub swapped: bool,
}

pub enum Target {
    Map { id: MapId, spawn: Option<String>, resume: bool },
    Title,
}

#[derive(Resource, Default)]
pub struct Director {
    pub transition: Option<Transition>,
    /// Replies to send once their effect has finished.
    pub later: Vec<u64>,
    pub autosave: f32,
}

pub fn build_game(launch: &platform::Launch) -> Game {
    let mut state = GameState::new();
    if let Some(s) = platform::load_settings().and_then(|j| serde_json::from_str(&j).ok()) {
        state.settings = s;
    }
    let stored = platform::load_save().and_then(|j| GameState::parse_save(&j));
    let mut game = Game::new(state, stored);
    game.auto = launch.autoplay;
    game.auto_pace = launch.pace;
    game
}

#[allow(clippy::too_many_arguments)]
pub fn run(
    mut commands: Commands,
    time: Res<Time>,
    mut game: NonSendMut<GameRes>,
    mut director: ResMut<Director>,
    mut level: ResMut<Level>,
    mut gfx: ResMut<Gfx>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<GlowMaterial>>,
    mut cam: ResMut<CameraCtl>,
    mut screen: ResMut<Screen>,
    mut particles: ResMut<Particles>,
    map_entities: Query<Entity, With<MapEntity>>,
    title_entities: Query<Entity, With<TitleEntity>>,
    player: Query<(&Pos, &Walker), With<Player>>,
    mut objs: Query<(&mut Obj, &mut Sprite)>,
) {
    let dt = time.delta_secs().min(0.1);
    // replies whose effects have finished (camera pans)
    for id in cam.finished.drain(..) {
        game.complete(id, Reply::Done);
    }
    for id in director.later.drain(..) {
        game.complete(id, Reply::Done);
    }
    game.update(dt);

    for req in std::mem::take(&mut game.engine) {
        match req {
            EngineReq::StartWorld => {
                let scene = game.ctx.state_ref().data.scene;
                screen.fade = 1.0;
                director.transition = Some(Transition { to: Target::Map { id: scene, spawn: None, resume: true }, reply: None, swapped: false });
            }
            EngineReq::Title => {
                screen.fade = 1.0;
                director.transition = Some(Transition { to: Target::Title, reply: None, swapped: false });
            }
            EngineReq::World { id, cmd } => match cmd {
                Cmd::Goto { map: m, spawn } => {
                    director.transition = Some(Transition { to: Target::Map { id: m, spawn, resume: false }, reply: Some(id), swapped: false });
                }
                Cmd::PanTo { x, y, ms } => {
                    let from = cam.center;
                    cam.mode = CamMode::Pan { from, to: Vec2::new(x, y), t: 0.0, dur: (ms / 1000.0).max(0.01), reply: id };
                }
                Cmd::Follow => cam.mode = CamMode::Follow,
                Cmd::Flash { ms, color } => screen.flash(color, ms / 1000.0),
                Cmd::Shake { ms } => {
                    if !game.ctx.state_ref().settings.reduced_motion {
                        cam.shake = ms / 1000.0;
                    }
                }
                Cmd::Burst { x, y, color, n } => fx::burst(&mut particles, Vec2::new(x, y), color, n),
                Cmd::SetObjectFrame { id: obj, frame } => {
                    if let Some(m) = level.map.clone()
                        && let Some(idx) = m.objects.iter().position(|o| o.id == obj)
                        && let Some(&e) = level.objects.get(idx)
                        && let Ok((mut o, mut sprite)) = objs.get_mut(e)
                    {
                        o.pinned = true;
                        if let Some(a) = sprite.texture_atlas.as_mut() {
                            a.index = frame;
                        }
                        commands.entity(e).remove::<world::Looping>();
                    }
                }
                Cmd::SavePosition => save_position(&mut game, &level, &player),
                _ => {}
            },
            EngineReq::PersistSave => {
                let json = game.ctx.state_ref().save_json();
                game.storage_ok = platform::store_save(&json);
            }
            EngineReq::PersistSettings => {
                let s = game.ctx.state_ref().settings.clone();
                if let Ok(json) = serde_json::to_string(&s) {
                    platform::store_settings(&json);
                }
            }
            EngineReq::Import(d) => {
                if let Ok(json) = serde_json::to_string(&d) {
                    game.storage_ok = platform::store_save(&json);
                }
            }
            EngineReq::Download { name, contents } => match platform::download(&name, &contents) {
                Some(path) => game.hud.toast("SAVED", &format!("Certificate written to {path} (and copied to the clipboard)."), ToastKind::Info),
                None => game.hud.toast("COPIED", "Certificate copied to the clipboard.", ToastKind::Info),
            },
        }
    }

    // a map change: fade out, swap behind the black, fade back in
    if let Some(tr) = director.transition.as_mut() {
        if !tr.swapped {
            screen.fade = (screen.fade + dt / 0.35).min(1.0);
            if screen.fade >= 1.0 {
                tr.swapped = true;
                world::despawn_level(&mut commands, &map_entities, &mut level);
                for e in title_entities.iter() {
                    commands.entity(e).despawn();
                }
                particles.clear();
                match &tr.to {
                    Target::Title => {
                        title::spawn(&mut commands, &mut gfx, &mut images, &mut materials);
                        cam.mode = CamMode::Title;
                        cam.snap = true;
                    }
                    Target::Map { id, spawn, resume } => {
                        let def = map(*id);
                        let state = game.ctx.state_ref();
                        let saved = if *resume { state.data.pos.filter(|_| state.data.scene == *id) } else { None };
                        let arrival = match (saved, spawn) {
                            (Some(p), _) => Arrival::At { x: p.x, y: p.y, facing: p.facing },
                            (None, Some(s)) => Arrival::Spawn(s.as_str()),
                            (None, None) => Arrival::Spawn("start"),
                        };
                        let mut cx = SpawnCtx { commands: &mut commands, gfx: &mut gfx, images: &mut images, materials: &mut materials };
                        *level = world::spawn_level(&mut cx, def, &state, arrival);
                        drop(state);
                        fx::spawn_ambient(&mut commands, &level);
                        cam.mode = CamMode::Follow;
                        cam.snap = true;
                        let id = *id;
                        game.map_ready(id);
                        game.ctx.state().save();
                    }
                }
            }
        } else {
            screen.fade = (screen.fade - dt / 0.45).max(0.0);
            if screen.fade <= 0.0 {
                let tr = director.transition.take().unwrap();
                if let Some(id) = tr.reply {
                    game.complete(id, Reply::Done);
                }
            }
        }
    }

    // autosave every 15 s while exploring
    director.autosave += dt;
    if director.autosave > 15.0 && game.mode == Mode::World && !game.busy() && director.transition.is_none() {
        director.autosave = 0.0;
        save_position(&mut game, &level, &player);
    }
}

pub fn save_position(game: &mut Game, level: &Level, player: &Query<(&Pos, &Walker), With<Player>>) {
    let Some(id) = level.id() else { return };
    let Ok((pos, walker)) = player.single() else { return };
    let mut s = game.ctx.state();
    s.data.scene = id;
    s.data.pos = Some(SavePos { x: pos.p.x.round(), y: pos.p.y.round(), facing: walker.facing });
    s.save();
}

/// Draw the egui interface for this frame.
pub fn draw_ui(mut contexts: EguiContexts, time: Res<Time>, mut game: NonSendMut<GameRes>) -> Result {
    let ctx = contexts.ctx_mut()?;
    let dt = time.delta_secs().min(0.1);
    game.draw(ctx, dt);
    Ok(())
}

pub struct DirectorPlugin;

impl Plugin for DirectorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Director>().add_systems(EguiPrimaryContextPass, draw_ui);
    }
}
