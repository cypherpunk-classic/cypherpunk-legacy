//! All sound is synthesised: no audio files ship with the game. Sound effects
//! are short envelopes on oscillators and filtered noise; music is a small step
//! sequencer per area, rendered offline into a seamless loop.
//!
//! Everything renders to mono `f32` samples at [`SAMPLE_RATE`] and can be
//! wrapped as a 16-bit WAV with [`wav_bytes`] for the engine to play.

use std::f32::consts::{PI, TAU};

pub const SAMPLE_RATE: u32 = 32_000;
const SR: f32 = SAMPLE_RATE as f32;

/// MIDI note number to frequency.
pub fn note(n: f32) -> f32 {
    440.0 * 2f32.powf((n - 69.0) / 12.0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wave {
    Sine,
    Square,
    Triangle,
    Saw,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filter {
    Low,
    High,
    Band,
}

/// RBJ biquad, the same family as Web Audio's BiquadFilterNode.
#[derive(Clone, Copy)]
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl Biquad {
    fn new(kind: Filter, cutoff: f32, q: f32) -> Self {
        let f = cutoff.min(SR * 0.45).max(10.0);
        let w0 = TAU * f / SR;
        let (s, c) = w0.sin_cos();
        let alpha = s / (2.0 * q);
        let (b0, b1, b2) = match kind {
            Filter::Low => ((1.0 - c) / 2.0, 1.0 - c, (1.0 - c) / 2.0),
            Filter::High => ((1.0 + c) / 2.0, -(1.0 + c), (1.0 + c) / 2.0),
            Filter::Band => (alpha, 0.0, -alpha),
        };
        let a0 = 1.0 + alpha;
        Self { b0: b0 / a0, b1: b1 / a0, b2: b2 / a0, a1: -2.0 * c / a0, a2: (1.0 - alpha) / a0, x1: 0.0, x2: 0.0, y1: 0.0, y2: 0.0 }
    }

    #[inline]
    fn run(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2 - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

/// Small deterministic RNG (xorshift32) so music renders the same every time.
#[derive(Clone)]
pub struct Rng(u32);

impl Rng {
    pub fn new(seed: u32) -> Self {
        Self(seed.max(1))
    }

    pub fn next(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        (x >> 8) as f32 / (1u32 << 24) as f32
    }
}

#[derive(Clone, Copy)]
pub struct ToneOpts {
    pub glide_to: Option<f32>,
    pub attack: f32,
    pub filter: Option<f32>,
    pub detune: f32,
}

impl Default for ToneOpts {
    fn default() -> Self {
        Self { glide_to: None, attack: 0.005, filter: None, detune: 0.0 }
    }
}

fn opts() -> ToneOpts {
    ToneOpts::default()
}

/// A mono buffer we can drop notes into. With `wrap`, anything that runs past
/// the end continues at the start (for seamless music loops).
pub struct Mixer {
    pub buf: Vec<f32>,
    wrap: bool,
    rng: Rng,
}

/// Exponential ramp between two positive values, like AudioParam.exponentialRampToValueAtTime.
#[inline]
fn exp_ramp(v0: f32, v1: f32, frac: f32) -> f32 {
    v0 * (v1 / v0).powf(frac.clamp(0.0, 1.0))
}

#[inline]
fn poly_blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let t = t / dt;
        t + t - t * t - 1.0
    } else if t > 1.0 - dt {
        let t = (t - 1.0) / dt;
        t * t + t + t + 1.0
    } else {
        0.0
    }
}

impl Mixer {
    pub fn new(seconds: f32, wrap: bool, seed: u32) -> Self {
        Self { buf: vec![0.0; (seconds * SR).ceil() as usize], wrap, rng: Rng::new(seed) }
    }

    fn add(&mut self, i: usize, v: f32) {
        let n = self.buf.len();
        if i < n {
            self.buf[i] += v;
        } else if self.wrap && n > 0 {
            self.buf[i % n] += v;
        }
    }

    pub fn random(&mut self) -> f32 {
        self.rng.next()
    }

    /// One oscillator note with an exponential attack/decay envelope.
    pub fn tone(&mut self, freq: f32, t: f32, dur: f32, wave: Wave, peak: f32, o: ToneOpts) {
        let start = (t * SR) as usize;
        let len = ((dur + 0.05) * SR) as usize;
        let mut phase = 0.0f32;
        let mut filter = o.filter.map(|f| Biquad::new(Filter::Low, f, std::f32::consts::FRAC_1_SQRT_2));
        let detune = 2f32.powf(o.detune / 1200.0);
        let attack = o.attack.max(0.001);
        let mut tri_state = 0.0f32;
        for k in 0..len {
            let tt = k as f32 / SR;
            let f = match o.glide_to {
                Some(g) => exp_ramp(freq, g, tt / dur),
                None => freq,
            } * detune;
            let dt = (f / SR).min(0.5);
            let v = match wave {
                Wave::Sine => (TAU * phase).sin(),
                Wave::Saw => (2.0 * phase - 1.0) - poly_blep(phase, dt),
                Wave::Square => {
                    let mut sq = if phase < 0.5 { 1.0 } else { -1.0 };
                    sq += poly_blep(phase, dt);
                    sq -= poly_blep((phase + 0.5) % 1.0, dt);
                    sq
                }
                Wave::Triangle => {
                    // integrated band-limited square
                    let mut sq = if phase < 0.5 { 1.0 } else { -1.0 };
                    sq += poly_blep(phase, dt);
                    sq -= poly_blep((phase + 0.5) % 1.0, dt);
                    tri_state = dt * sq * 4.0 + (1.0 - dt * 0.05) * tri_state;
                    tri_state
                }
            };
            phase = (phase + dt) % 1.0;
            let env = if tt < attack {
                exp_ramp(0.0001, peak, tt / attack)
            } else if tt < dur {
                exp_ramp(peak, 0.0001, (tt - attack) / (dur - attack).max(1e-4))
            } else {
                0.0001 * (1.0 - (tt - dur) / 0.05).max(0.0)
            };
            let mut s = v * env;
            if let Some(fl) = filter.as_mut() {
                s = fl.run(s);
            }
            self.add(start + k, s);
        }
    }

    /// Filtered brownish noise burst with an exponential decay.
    pub fn noise(&mut self, t: f32, dur: f32, peak: f32, cutoff: f32, kind: Filter) {
        let start = (t * SR) as usize;
        let len = ((dur + 0.05) * SR) as usize;
        let mut f = Biquad::new(kind, cutoff, if kind == Filter::Band { 0.6 } else { std::f32::consts::FRAC_1_SQRT_2 });
        let mut brown = 0.0f32;
        for k in 0..len {
            let tt = k as f32 / SR;
            let white = self.rng.next() * 2.0 - 1.0;
            brown = 0.97 * brown + 0.03 * white;
            let n = white * 0.6 + brown * 2.4;
            let env = if tt < dur { exp_ramp(peak, 0.0001, tt / dur) } else { 0.0 };
            let s = f.run(n) * env;
            self.add(start + k, s);
        }
    }

    /// A continuous bed of filtered noise across the whole buffer (rain, city hum).
    pub fn ambience(&mut self, cutoff: f32, kind: Filter, gain: f32) {
        let n = self.buf.len();
        let fade = ((0.5 * SR) as usize).min(n / 4);
        let mut f = Biquad::new(kind, cutoff, 0.6);
        let mut brown = 0.0f32;
        let mut sample = |rng: &mut Rng, f: &mut Biquad| {
            let white = rng.next() * 2.0 - 1.0;
            brown = 0.97 * brown + 0.03 * white;
            f.run(white * 0.6 + brown * 2.4)
        };
        for _ in 0..SAMPLE_RATE {
            sample(&mut self.rng, &mut f); // let the filter settle
        }
        let tmp: Vec<f32> = (0..n + fade).map(|_| sample(&mut self.rng, &mut f)).collect();
        // The extra tail crossfades into the start, so sample n-1 flows into sample 0.
        for i in 0..n {
            let mut v = tmp[i];
            if i < fade {
                let a = i as f32 / fade as f32;
                v = v * a + tmp[n + i] * (1.0 - a);
            }
            self.buf[i] += v * gain;
        }
    }

    pub fn finish(mut self, gain: f32) -> Vec<f32> {
        for s in &mut self.buf {
            *s = (*s * gain).tanh();
        }
        self.buf
    }
}

// ---------------------------------------------------------------------------
// Sound effects
// ---------------------------------------------------------------------------

pub const SFX: &[&str] = &["blip", "type", "confirm", "error", "unlock", "success", "door", "power", "glitch", "meow", "pickup", "warden", "step", "select"];

/// Render one sound effect.
pub fn sfx(name: &str, seed: u32) -> Vec<f32> {
    let mut m = Mixer::new(1.6, false, seed);
    let t = 0.005;
    match name {
        "blip" => m.tone(880.0, t, 0.05, Wave::Square, 0.12, opts()),
        "select" => {
            m.tone(660.0, t, 0.04, Wave::Square, 0.1, opts());
            m.tone(990.0, t + 0.04, 0.05, Wave::Square, 0.1, opts());
        }
        "type" => {
            let f = 1100.0 + m.random() * 300.0;
            m.tone(f, t, 0.02, Wave::Square, 0.04, ToneOpts { filter: Some(3000.0), ..opts() });
        }
        "confirm" => {
            m.tone(660.0, t, 0.08, Wave::Triangle, 0.25, opts());
            m.tone(990.0, t + 0.07, 0.12, Wave::Triangle, 0.25, opts());
        }
        "error" => m.tone(200.0, t, 0.22, Wave::Saw, 0.18, ToneOpts { glide_to: Some(110.0), filter: Some(1200.0), ..opts() }),
        "pickup" => {
            for (i, n) in [72.0, 76.0, 79.0, 84.0].into_iter().enumerate() {
                m.tone(note(n), t + i as f32 * 0.05, 0.12, Wave::Square, 0.12, ToneOpts { filter: Some(4000.0), ..opts() });
            }
        }
        "unlock" => {
            for (i, n) in [60.0, 64.0, 67.0, 72.0, 76.0].into_iter().enumerate() {
                m.tone(note(n), t + i as f32 * 0.07, 0.3, Wave::Triangle, 0.22, opts());
            }
        }
        "success" => {
            for (i, n) in [67.0, 71.0, 74.0, 79.0, 83.0, 86.0].into_iter().enumerate() {
                m.tone(note(n), t + i as f32 * 0.06, 0.45, Wave::Triangle, 0.2, opts());
            }
            m.tone(note(91.0), t + 0.36, 0.8, Wave::Sine, 0.08, opts());
        }
        "door" => {
            m.noise(t, 0.25, 0.4, 400.0, Filter::Low);
            m.tone(110.0, t, 0.2, Wave::Sine, 0.2, ToneOpts { glide_to: Some(70.0), ..opts() });
        }
        "power" => {
            m.tone(60.0, t, 0.9, Wave::Sine, 0.25, ToneOpts { glide_to: Some(1800.0), attack: 0.02, ..opts() });
            m.noise(t, 0.6, 0.15, 6000.0, Filter::High);
        }
        "glitch" => {
            for i in 0..6 {
                let f = 200.0 + m.random() * 2000.0;
                m.tone(f, t + i as f32 * 0.03, 0.03, Wave::Square, 0.08, opts());
            }
        }
        "meow" => {
            m.tone(700.0, t, 0.12, Wave::Triangle, 0.2, ToneOpts { glide_to: Some(950.0), filter: Some(2500.0), ..opts() });
            m.tone(950.0, t + 0.12, 0.25, Wave::Triangle, 0.18, ToneOpts { glide_to: Some(560.0), filter: Some(2200.0), ..opts() });
        }
        "warden" => {
            m.tone(55.0, t, 1.2, Wave::Saw, 0.2, ToneOpts { filter: Some(300.0), attack: 0.1, ..opts() });
            m.tone(82.4, t, 1.2, Wave::Saw, 0.12, ToneOpts { filter: Some(300.0), attack: 0.1, detune: 8.0, ..opts() });
        }
        "step" => m.noise(t, 0.04, 0.05, 900.0, Filter::Low),
        _ => {}
    }
    let mut out = m.finish(1.0);
    // trim trailing silence
    let last = out.iter().rposition(|s| s.abs() > 1e-4).unwrap_or(0);
    out.truncate((last + 64).min(out.len()));
    out
}

// ---------------------------------------------------------------------------
// Music
// ---------------------------------------------------------------------------

pub const TRACKS: &[&str] = &["title", "cabin", "city", "tension", "finale"];

fn bpm(track: &str) -> f32 {
    match track {
        "cabin" => 70.0,
        "tension" => 84.0,
        "finale" => 92.0,
        "title" => 80.0,
        _ => 96.0,
    }
}

/// Render a track as a seamless loop of 8 bars (two passes of the 4-bar progression).
pub fn music(track: &str) -> Vec<f32> {
    let sixteenth = 60.0 / bpm(track) / 4.0;
    let steps = 128;
    let mut m = Mixer::new(sixteenth * steps as f32, true, 0x5eed ^ track.len() as u32 * 7919);
    for s in 0..steps {
        play_step(&mut m, track, s, s as f32 * sixteenth, sixteenth);
    }
    match track {
        "cabin" => m.ambience(1400.0, Filter::Band, 0.18),
        "city" => m.ambience(300.0, Filter::Low, 0.08),
        _ => {}
    }
    m.finish(1.0)
}

fn play_step(m: &mut Mixer, track: &str, s: usize, t: f32, dt: f32) {
    let bar = (s / 16) % 4;
    let i = s % 16;
    match track {
        "cabin" => {
            let chords = [[57.0, 60.0, 64.0, 67.0], [53.0, 57.0, 60.0, 64.0], [48.0, 55.0, 60.0, 64.0], [55.0, 59.0, 62.0, 67.0]];
            let ch = chords[bar];
            if i == 0 {
                for n in ch {
                    m.tone(note(n), t, dt * 16.0, Wave::Sine, 0.07, ToneOpts { attack: 0.6, ..opts() });
                }
            }
            if i % 4 == 2 && m.random() < 0.6 {
                m.tone(note(ch[(i / 2) % 4] + 12.0), t, dt * 6.0, Wave::Triangle, 0.03, ToneOpts { attack: 0.05, ..opts() });
            }
            if m.random() < 0.05 {
                m.noise(t, 0.05, 0.05, 2500.0, Filter::High);
            }
        }
        "title" => {
            let arp = [57.0, 60.0, 64.0, 69.0, 64.0, 60.0, 57.0, 52.0];
            let root = [0.0, -4.0, 3.0, -2.0][bar];
            if i % 2 == 0 {
                m.tone(note(arp[(i / 2) % 8] + root), t, dt * 3.0, Wave::Triangle, 0.05, ToneOpts { filter: Some(2400.0), ..opts() });
            }
            if i == 0 {
                m.tone(note(33.0 + root), t, dt * 16.0, Wave::Saw, 0.06, ToneOpts { filter: Some(400.0), attack: 0.3, ..opts() });
            }
        }
        "tension" => {
            if i % 4 == 0 {
                m.tone(note(if i == 0 { 33.0 } else { 36.0 }), t, dt * 3.0, Wave::Saw, 0.09, ToneOpts { filter: Some(500.0), ..opts() });
            }
            if i == 8 && bar % 2 == 1 {
                m.tone(note(63.0), t, dt * 8.0, Wave::Square, 0.03, ToneOpts { filter: Some(1500.0), ..opts() });
            }
            if i % 2 == 0 {
                m.noise(t, 0.03, 0.04, 6000.0, Filter::High);
            }
        }
        _ => {
            // city + finale: synthwave
            let prog = if track == "finale" {
                [[60.0, 64.0, 67.0], [57.0, 60.0, 64.0], [65.0, 69.0, 72.0], [67.0, 71.0, 74.0]]
            } else {
                [[57.0, 60.0, 64.0], [53.0, 57.0, 60.0], [48.0, 52.0, 55.0], [55.0, 59.0, 62.0]]
            };
            let ch = prog[bar];
            let root = ch[0] - 24.0;
            if i % 4 == 0 || i == 6 || i == 14 {
                m.tone(note(root), t, dt * 1.8, Wave::Saw, 0.08, ToneOpts { filter: Some(700.0), ..opts() });
            }
            let arp_n = ch[[0, 1, 2, 1][i % 4]] + if i >= 8 { 12.0 } else { 0.0 };
            m.tone(note(arp_n), t, dt * 0.9, Wave::Square, 0.025, ToneOpts { filter: Some(2800.0), ..opts() });
            if i == 0 {
                for n in ch {
                    m.tone(note(n), t, dt * 15.0, Wave::Saw, 0.025, ToneOpts { filter: Some(1200.0), attack: 0.4, detune: 6.0, ..opts() });
                }
            }
            if i == 4 || i == 12 {
                m.noise(t, 0.08, 0.08, 1800.0, Filter::High);
            }
            if i % 2 == 0 {
                m.noise(t, 0.02, 0.03, 8000.0, Filter::High);
            }
        }
    }
}

/// Wrap samples as a mono 16-bit PCM WAV file.
pub fn wav_bytes(samples: &[f32]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    out.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32767.0).round() as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

/// A gentle sine "ping", used by tests to check the pipeline.
pub fn sine(freq: f32, seconds: f32) -> Vec<f32> {
    (0..(seconds * SR) as usize).map(|i| (TAU * freq * i as f32 / SR).sin() * 0.5 * (PI * i as f32 / (seconds * SR)).sin()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sound_renders_audibly() {
        for name in SFX {
            let s = sfx(name, 1);
            assert!(!s.is_empty(), "{name}");
            let peak = s.iter().fold(0f32, |a, b| a.max(b.abs()));
            assert!(peak > 0.005 && peak <= 1.0, "{name}: peak {peak}");
            assert!(s.iter().all(|v| v.is_finite()));
        }
    }

    #[test]
    fn music_loops_render() {
        for t in TRACKS {
            let s = music(t);
            assert!(s.len() as f32 / SR > 15.0, "{t}");
            let rms = (s.iter().map(|v| v * v).sum::<f32>() / s.len() as f32).sqrt();
            assert!(rms > 0.003 && rms < 0.5, "{t}: rms {rms}");
        }
        let w = wav_bytes(&sine(440.0, 0.1));
        assert_eq!(&w[0..4], b"RIFF");
        assert_eq!(w.len(), 44 + (0.1 * SR) as usize * 2);
    }
}
