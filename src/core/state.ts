import { randomBytes } from '@noble/hashes/utils.js';
import { fromHex, toHex } from '../crypto/bytes';
import { Drbg } from '../crypto/drbg';
import { Emitter } from './events';
import { readJson, removeKey, writeJson } from './save';

export type SceneId = 'cabin' | 'city';
export type Facing = 'down' | 'up' | 'left' | 'right';
export type FlagValue = boolean | number | string;

export interface SaveData {
  version: 2;
  seed: string;
  handle: string;
  look: number;
  scene: SceneId;
  pos: { x: number; y: number; facing: Facing } | null;
  flags: Record<string, FlagValue>;
  items: string[];
  codex: string[];
  objective: string;
  chapter: number;
  playMs: number;
  createdAt: number;
  updatedAt: number;
  solved: Record<string, { at: number; attempts: number }>;
}

export interface Settings {
  music: number;
  sfx: number;
  textSpeed: number; // characters per second, 0 = instant
  crt: boolean;
  reducedMotion: boolean;
}

const SAVE_KEY = 'cypherpunk-legacy:save:v2';
const SETTINGS_KEY = 'cypherpunk-legacy:settings:v2';

export const DEFAULT_SETTINGS: Settings = { music: 0.5, sfx: 0.7, textSpeed: 60, crt: true, reducedMotion: false };

export interface StateEvents extends Record<string, unknown> {
  flag: { key: string; value: FlagValue };
  item: { id: string; added: boolean };
  codex: { id: string };
  objective: { text: string };
  chapter: { n: number };
  settings: Settings;
  saved: { at: number };
}

export class GameState extends Emitter<StateEvents> {
  data: SaveData;
  settings: Settings;
  private sessionStart = performance.now();
  private seedOverride: string | null = null;

  constructor() {
    super();
    this.settings = { ...DEFAULT_SETTINGS, ...(readJson<Partial<Settings>>(SETTINGS_KEY) ?? {}) };
    this.data = GameState.blank('anon', 0);
  }

  static blank(handle: string, look: number, seed?: string): SaveData {
    const now = Date.now();
    return {
      version: 2,
      seed: seed ?? toHex(randomBytes(32)),
      handle,
      look,
      scene: 'cabin',
      pos: null,
      flags: {},
      items: [],
      codex: [],
      objective: '',
      chapter: 0,
      playMs: 0,
      createdAt: now,
      updatedAt: now,
      solved: {},
    };
  }

  /** Force a deterministic seed (tests, ?seed=...). */
  setSeedOverride(seed: string | null): void {
    this.seedOverride = seed;
  }

  newGame(handle: string, look: number): void {
    const seed = this.seedOverride ? toHex(new Drbg(new TextEncoder().encode(this.seedOverride), 'seed').bytes(32)) : undefined;
    this.data = GameState.blank(handle, look, seed);
    this.sessionStart = performance.now();
    this.save();
  }

  hasSave(): boolean {
    return readJson<SaveData>(SAVE_KEY)?.version === 2;
  }

  peekSave(): SaveData | null {
    const d = readJson<SaveData>(SAVE_KEY);
    return d?.version === 2 ? d : null;
  }

  load(): boolean {
    const d = this.peekSave();
    if (!d) return false;
    this.data = { ...GameState.blank(d.handle, d.look, d.seed), ...d };
    this.sessionStart = performance.now();
    return true;
  }

  save(): void {
    const now = performance.now();
    this.data.playMs += now - this.sessionStart;
    this.sessionStart = now;
    this.data.updatedAt = Date.now();
    writeJson(SAVE_KEY, this.data);
    this.emit('saved', { at: this.data.updatedAt });
  }

  wipe(): void {
    removeKey(SAVE_KEY);
  }

  exportSave(): string {
    return JSON.stringify(this.data, null, 2);
  }

  importSave(json: string): boolean {
    try {
      const d = JSON.parse(json) as SaveData;
      if (d.version !== 2 || typeof d.seed !== 'string') return false;
      fromHex(d.seed);
      this.data = { ...GameState.blank(d.handle, d.look, d.seed), ...d };
      this.save();
      return true;
    } catch {
      return false;
    }
  }

  // --- flags, items, codex ---------------------------------------------------

  flag(key: string): boolean {
    return Boolean(this.data.flags[key]);
  }

  get<T extends FlagValue>(key: string): T | undefined {
    return this.data.flags[key] as T | undefined;
  }

  set(key: string, value: FlagValue = true): void {
    if (this.data.flags[key] === value) return;
    this.data.flags[key] = value;
    this.emit('flag', { key, value });
  }

  has(item: string): boolean {
    return this.data.items.includes(item);
  }

  give(item: string): void {
    if (this.has(item)) return;
    this.data.items.push(item);
    this.emit('item', { id: item, added: true });
  }

  take(item: string): void {
    const i = this.data.items.indexOf(item);
    if (i < 0) return;
    this.data.items.splice(i, 1);
    this.emit('item', { id: item, added: false });
  }

  unlockCodex(id: string): boolean {
    if (this.data.codex.includes(id)) return false;
    this.data.codex.push(id);
    this.emit('codex', { id });
    return true;
  }

  setObjective(text: string): void {
    this.data.objective = text;
    this.emit('objective', { text });
  }

  setChapter(n: number): void {
    if (n <= this.data.chapter) return;
    this.data.chapter = n;
    this.emit('chapter', { n });
  }

  markSolved(id: string, attempts = 1): void {
    if (!this.data.solved[id]) this.data.solved[id] = { at: Date.now(), attempts };
  }

  // --- randomness ------------------------------------------------------------

  /** Deterministic stream for puzzle content, unique to this save. */
  rng(label: string): Drbg {
    return new Drbg(fromHex(this.data.seed), label);
  }

  // --- settings ----------------------------------------------------------------

  updateSettings(patch: Partial<Settings>): void {
    this.settings = { ...this.settings, ...patch };
    writeJson(SETTINGS_KEY, this.settings);
    this.emit('settings', this.settings);
  }
}

export const state = new GameState();
