/**
 * All sound is synthesised with the Web Audio API: no audio files ship with
 * the game. Sound effects are short envelopes on oscillators and filtered
 * noise; music is a small step sequencer per area.
 */

export type Sfx =
  | 'blip'
  | 'type'
  | 'confirm'
  | 'error'
  | 'unlock'
  | 'success'
  | 'door'
  | 'power'
  | 'glitch'
  | 'meow'
  | 'pickup'
  | 'warden'
  | 'step'
  | 'select';

export type Track = 'title' | 'cabin' | 'city' | 'tension' | 'finale' | null;

const NOTE = (n: number): number => 440 * Math.pow(2, (n - 69) / 12);

class AudioEngine {
  private ctx: AudioContext | null = null;
  private master!: GainNode;
  private musicBus!: GainNode;
  private sfxBus!: GainNode;
  private noiseBuf: AudioBuffer | null = null;
  private track: Track = null;
  private pending: Track = null;
  private step = 0;
  private nextTime = 0;
  private timer: number | null = null;
  private ambience: { src: AudioBufferSourceNode; gain: GainNode } | null = null;
  private vol = { music: 0.5, sfx: 0.7 };

  /** Browsers only allow audio after a user gesture; call from input handlers. */
  unlock(): void {
    if (this.ctx) {
      if (this.ctx.state === 'suspended') void this.ctx.resume();
      return;
    }
    try {
      const AC = window.AudioContext ?? (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
      this.ctx = new AC();
    } catch {
      return;
    }
    const ctx = this.ctx;
    this.master = ctx.createGain();
    this.master.gain.value = 0.9;
    this.master.connect(ctx.destination);
    this.musicBus = ctx.createGain();
    this.sfxBus = ctx.createGain();
    this.musicBus.connect(this.master);
    this.sfxBus.connect(this.master);
    this.applyVolumes();
    const len = ctx.sampleRate * 2;
    this.noiseBuf = ctx.createBuffer(1, len, ctx.sampleRate);
    const data = this.noiseBuf.getChannelData(0);
    let b = 0;
    for (let i = 0; i < len; i++) {
      const white = Math.random() * 2 - 1;
      b = 0.97 * b + 0.03 * white; // brownish noise for rain/fire
      data[i] = white * 0.6 + b * 2.4;
    }
    if (this.pending !== null) this.music(this.pending);
  }

  setVolumes(music: number, sfx: number): void {
    this.vol = { music, sfx };
    this.applyVolumes();
  }

  private applyVolumes(): void {
    if (!this.ctx) return;
    this.musicBus.gain.setTargetAtTime(this.vol.music * 0.35, this.ctx.currentTime, 0.1);
    this.sfxBus.gain.setTargetAtTime(this.vol.sfx * 0.5, this.ctx.currentTime, 0.05);
  }

  // --- primitives ------------------------------------------------------------

  private tone(
    bus: GainNode,
    freq: number,
    t: number,
    dur: number,
    type: OscillatorType,
    peak: number,
    opts: { glideTo?: number; attack?: number; filter?: number; detune?: number } = {},
  ): void {
    const ctx = this.ctx!;
    const osc = ctx.createOscillator();
    osc.type = type;
    osc.frequency.setValueAtTime(freq, t);
    if (opts.detune) osc.detune.value = opts.detune;
    if (opts.glideTo) osc.frequency.exponentialRampToValueAtTime(opts.glideTo, t + dur);
    const g = ctx.createGain();
    const a = opts.attack ?? 0.005;
    g.gain.setValueAtTime(0.0001, t);
    g.gain.exponentialRampToValueAtTime(peak, t + a);
    g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
    let node: AudioNode = osc;
    if (opts.filter) {
      const f = ctx.createBiquadFilter();
      f.type = 'lowpass';
      f.frequency.value = opts.filter;
      osc.connect(f);
      node = f;
    }
    node.connect(g);
    g.connect(bus);
    osc.start(t);
    osc.stop(t + dur + 0.05);
  }

  private noise(bus: GainNode, t: number, dur: number, peak: number, filter: number, type: BiquadFilterType = 'lowpass'): void {
    if (!this.noiseBuf) return;
    const ctx = this.ctx!;
    const src = ctx.createBufferSource();
    src.buffer = this.noiseBuf;
    const f = ctx.createBiquadFilter();
    f.type = type;
    f.frequency.value = filter;
    const g = ctx.createGain();
    g.gain.setValueAtTime(peak, t);
    g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
    src.connect(f);
    f.connect(g);
    g.connect(bus);
    src.start(t, Math.random());
    src.stop(t + dur + 0.05);
  }

  // --- sound effects -----------------------------------------------------------

  sfx(name: Sfx): void {
    if (!this.ctx || this.vol.sfx <= 0) return;
    const t = this.ctx.currentTime + 0.005;
    const b = this.sfxBus;
    switch (name) {
      case 'blip':
        this.tone(b, 880, t, 0.05, 'square', 0.12);
        break;
      case 'select':
        this.tone(b, 660, t, 0.04, 'square', 0.1);
        this.tone(b, 990, t + 0.04, 0.05, 'square', 0.1);
        break;
      case 'type':
        this.tone(b, 1100 + Math.random() * 300, t, 0.02, 'square', 0.04, { filter: 3000 });
        break;
      case 'confirm':
        this.tone(b, 660, t, 0.08, 'triangle', 0.25);
        this.tone(b, 990, t + 0.07, 0.12, 'triangle', 0.25);
        break;
      case 'error':
        this.tone(b, 200, t, 0.22, 'sawtooth', 0.18, { glideTo: 110, filter: 1200 });
        break;
      case 'pickup':
        [72, 76, 79, 84].forEach((n, i) => this.tone(b, NOTE(n), t + i * 0.05, 0.12, 'square', 0.12, { filter: 4000 }));
        break;
      case 'unlock':
        [60, 64, 67, 72, 76].forEach((n, i) => this.tone(b, NOTE(n), t + i * 0.07, 0.3, 'triangle', 0.22));
        break;
      case 'success':
        [67, 71, 74, 79, 83, 86].forEach((n, i) => this.tone(b, NOTE(n), t + i * 0.06, 0.45, 'triangle', 0.2));
        this.tone(b, NOTE(91), t + 0.36, 0.8, 'sine', 0.08);
        break;
      case 'door':
        this.noise(b, t, 0.25, 0.4, 400);
        this.tone(b, 110, t, 0.2, 'sine', 0.2, { glideTo: 70 });
        break;
      case 'power':
        this.tone(b, 60, t, 0.9, 'sine', 0.25, { glideTo: 1800, attack: 0.02 });
        this.noise(b, t, 0.6, 0.15, 6000, 'highpass');
        break;
      case 'glitch':
        for (let i = 0; i < 6; i++) this.tone(b, 200 + Math.random() * 2000, t + i * 0.03, 0.03, 'square', 0.08);
        break;
      case 'meow':
        this.tone(b, 700, t, 0.12, 'triangle', 0.2, { glideTo: 950, filter: 2500 });
        this.tone(b, 950, t + 0.12, 0.25, 'triangle', 0.18, { glideTo: 560, filter: 2200 });
        break;
      case 'warden':
        this.tone(b, 55, t, 1.2, 'sawtooth', 0.2, { filter: 300, attack: 0.1 });
        this.tone(b, 82.4, t, 1.2, 'sawtooth', 0.12, { filter: 300, attack: 0.1, detune: 8 });
        break;
      case 'step':
        this.noise(b, t, 0.04, 0.05, 900);
        break;
    }
  }

  // --- music -----------------------------------------------------------------

  music(track: Track): void {
    this.pending = track;
    if (!this.ctx) return;
    if (track === this.track) return;
    this.track = track;
    this.step = 0;
    this.nextTime = this.ctx.currentTime + 0.1;
    this.setAmbience(track === 'cabin' ? 'rain' : track === 'city' ? 'city' : null);
    if (this.timer !== null) window.clearInterval(this.timer);
    this.timer = null;
    if (track) this.timer = window.setInterval(() => this.schedule(), 25);
  }

  private setAmbience(kind: 'rain' | 'city' | null): void {
    const ctx = this.ctx;
    if (!ctx || !this.noiseBuf) return;
    if (this.ambience) {
      const old = this.ambience;
      old.gain.gain.setTargetAtTime(0.0001, ctx.currentTime, 0.4);
      setTimeout(() => old.src.stop(), 2000);
      this.ambience = null;
    }
    if (!kind) return;
    const src = ctx.createBufferSource();
    src.buffer = this.noiseBuf;
    src.loop = true;
    const f = ctx.createBiquadFilter();
    f.type = kind === 'rain' ? 'bandpass' : 'lowpass';
    f.frequency.value = kind === 'rain' ? 1400 : 300;
    f.Q.value = 0.6;
    const g = ctx.createGain();
    g.gain.value = 0.0001;
    g.gain.setTargetAtTime(kind === 'rain' ? 0.18 : 0.08, ctx.currentTime, 1.0);
    src.connect(f);
    f.connect(g);
    g.connect(this.musicBus);
    src.start();
    this.ambience = { src, gain: g };
  }

  private schedule(): void {
    const ctx = this.ctx;
    if (!ctx || !this.track) return;
    const bpm = this.track === 'cabin' ? 70 : this.track === 'tension' ? 84 : this.track === 'finale' ? 92 : this.track === 'title' ? 80 : 96;
    const sixteenth = 60 / bpm / 4;
    while (this.nextTime < ctx.currentTime + 0.12) {
      this.playStep(this.track, this.step, this.nextTime, sixteenth);
      this.nextTime += sixteenth;
      this.step++;
    }
  }

  private playStep(track: NonNullable<Track>, s: number, t: number, dt: number): void {
    const b = this.musicBus;
    const bar = Math.floor(s / 16) % 4;
    const i = s % 16;
    if (track === 'cabin') {
      const chords = [
        [57, 60, 64, 67],
        [53, 57, 60, 64],
        [48, 55, 60, 64],
        [55, 59, 62, 67],
      ];
      const ch = chords[bar]!;
      if (i === 0) ch.forEach((n) => this.tone(b, NOTE(n), t, dt * 16, 'sine', 0.07, { attack: 0.6 }));
      if (i % 4 === 2 && Math.random() < 0.6) this.tone(b, NOTE(ch[(i / 2) % 4]! + 12), t, dt * 6, 'triangle', 0.03, { attack: 0.05 });
      if (Math.random() < 0.05) this.noise(b, t, 0.05, 0.05, 2500, 'highpass');
      return;
    }
    if (track === 'title') {
      const arp = [57, 60, 64, 69, 64, 60, 57, 52];
      const root = [0, -4, 3, -2][bar]!;
      if (i % 2 === 0) this.tone(b, NOTE(arp[(i / 2) % 8]! + root), t, dt * 3, 'triangle', 0.05, { filter: 2400 });
      if (i === 0) this.tone(b, NOTE(33 + root), t, dt * 16, 'sawtooth', 0.06, { filter: 400, attack: 0.3 });
      return;
    }
    if (track === 'tension') {
      if (i % 4 === 0) this.tone(b, NOTE(i === 0 ? 33 : 36), t, dt * 3, 'sawtooth', 0.09, { filter: 500 });
      if (i === 8 && bar % 2 === 1) this.tone(b, NOTE(63), t, dt * 8, 'square', 0.03, { filter: 1500 });
      if (i % 2 === 0) this.noise(b, t, 0.03, 0.04, 6000, 'highpass');
      return;
    }
    // city + finale: synthwave
    const prog = track === 'finale' ? [[60, 64, 67], [57, 60, 64], [65, 69, 72], [67, 71, 74]] : [[57, 60, 64], [53, 57, 60], [48, 52, 55], [55, 59, 62]];
    const ch = prog[bar]!;
    const root = ch[0]! - 24;
    if (i % 4 === 0 || i === 6 || i === 14) this.tone(b, NOTE(root), t, dt * 1.8, 'sawtooth', 0.08, { filter: 700 });
    const arpN = ch[[0, 1, 2, 1][i % 4]!]! + (i >= 8 ? 12 : 0);
    this.tone(b, NOTE(arpN), t, dt * 0.9, 'square', 0.025, { filter: 2800 });
    if (i === 0) ch.forEach((n) => this.tone(b, NOTE(n), t, dt * 15, 'sawtooth', 0.025, { filter: 1200, attack: 0.4, detune: 6 }));
    if (i === 4 || i === 12) this.noise(b, t, 0.08, 0.08, 1800, 'highpass');
    if (i % 2 === 0) this.noise(b, t, 0.02, 0.03, 8000, 'highpass');
  }
}

export const audio = new AudioEngine();
