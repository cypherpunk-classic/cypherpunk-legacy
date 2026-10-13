//! Sound: every effect and music loop is synthesised by `cpl-audio` the
//! first time it is needed, wrapped as WAV, and played through Bevy's audio.

use std::collections::HashMap;

use bevy::audio::{AudioSink, AudioSinkPlayback, PlaybackMode, Volume};
use bevy::prelude::*;
use cpl_logic::game::AudioReq;

use crate::director::GameRes;

/// Effects requested by world systems (footsteps).
#[derive(Resource, Default)]
pub struct SfxQueue(pub Vec<&'static str>);

#[derive(Resource, Default)]
pub struct Bank {
    sfx: HashMap<(&'static str, u32), Handle<AudioSource>>,
    music: HashMap<&'static str, Handle<AudioSource>>,
    /// Round-robin variant per effect.
    turn: HashMap<&'static str, u32>,
    last: HashMap<&'static str, f64>,
    current: Option<&'static str>,
}

#[derive(Component)]
pub struct Music {
    /// Fading out (seconds left of the fade).
    pub leaving: Option<f32>,
    pub level: f32,
}

fn source(samples: Vec<f32>) -> AudioSource {
    AudioSource { bytes: cpl_audio::wav_bytes(&samples).into() }
}

const VARIANTS: u32 = 3;

pub fn play(mut commands: Commands, time: Res<Time>, mut game: NonSendMut<GameRes>, mut queue: ResMut<SfxQueue>, mut bank: ResMut<Bank>, mut sources: ResMut<Assets<AudioSource>>, mut music: Query<(Entity, &mut Music, Option<&mut AudioSink>)>) {
    let (vol_sfx, vol_music) = {
        let s = &game.ctx.state_ref().settings;
        (s.sfx * 0.5, s.music * 0.35)
    };
    let now = time.elapsed_secs_f64();
    let mut reqs: Vec<AudioReq> = std::mem::take(&mut game.audio);
    reqs.extend(queue.0.drain(..).map(AudioReq::Sfx));
    for r in reqs {
        match r {
            AudioReq::Sfx(name) => {
                if vol_sfx <= 0.001 || !cpl_audio::SFX.contains(&name) {
                    continue;
                }
                // don't stack the same tiny sound within a few frames
                let gap = if name == "type" { 0.045 } else { 0.03 };
                if bank.last.get(name).is_some_and(|t| now - t < gap) {
                    continue;
                }
                bank.last.insert(name, now);
                let turn = bank.turn.entry(name).or_insert(0);
                let variant = *turn % VARIANTS;
                *turn += 1;
                let handle = bank.sfx.entry((name, variant)).or_insert_with(|| sources.add(source(cpl_audio::sfx(name, variant)))).clone();
                commands.spawn((AudioPlayer::new(handle), PlaybackSettings { mode: PlaybackMode::Despawn, volume: Volume::Linear(vol_sfx), ..default() }));
            }
            AudioReq::Music(track) => {
                if bank.current == track {
                    continue;
                }
                bank.current = track;
                for (_, mut m, _) in music.iter_mut() {
                    if m.leaving.is_none() {
                        m.leaving = Some(0.8);
                    }
                }
                if let Some(t) = track
                    && cpl_audio::TRACKS.contains(&t)
                {
                    let handle = bank.music.entry(t).or_insert_with(|| sources.add(source(cpl_audio::music(t)))).clone();
                    commands.spawn((AudioPlayer::new(handle), PlaybackSettings { mode: PlaybackMode::Loop, volume: Volume::Linear(0.0), ..default() }, Music { leaving: None, level: 0.0 }));
                }
            }
        }
    }
    // fades
    let dt = time.delta_secs();
    for (e, mut m, sink) in music.iter_mut() {
        let m = &mut *m;
        match m.leaving {
            Some(left) => {
                m.leaving = Some(left - dt);
                m.level = (m.level - dt / 0.8).max(0.0);
                if left - dt <= 0.0 {
                    commands.entity(e).despawn();
                    continue;
                }
            }
            None => m.level = (m.level + dt / 1.2).min(1.0),
        }
        if let Some(mut sink) = sink {
            sink.set_volume(Volume::Linear(m.level * vol_music));
        }
    }
}

pub struct SoundPlugin;

impl Plugin for SoundPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SfxQueue>().init_resource::<Bank>();
    }
}
