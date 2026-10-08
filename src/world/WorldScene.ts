import * as Phaser from 'phaser';
import { input } from '../core/input';
import { audio } from '../core/audio';
import { state, type Facing } from '../core/state';
import { TILE_INDEX, SOLID, T } from '../art/tiles';
import { FACINGS } from '../art/characters';
import { charKey } from './textures';
import type { MapDef, MapObject, NpcDef } from './types';
import { hud } from '../ui/hud';

/**
 * One scene class drives every explorable map: tiles, props, people, a cat,
 * lights, collisions and the "press E" interaction loop. What happens when
 * you interact lives in src/story; this file only reports *that* it happened.
 */

export interface WorldHooks {
  interact: (scene: WorldScene, id: string) => Promise<void>;
  zone: (scene: WorldScene, id: string) => Promise<void>;
  ready: (scene: WorldScene) => Promise<void>;
}

let hooks: WorldHooks = { interact: async () => {}, zone: async () => {}, ready: async () => {} };
export const setWorldHooks = (h: WorldHooks): void => {
  hooks = h;
};

const FACING_VEC: Record<Facing, [number, number]> = { down: [0, 1], up: [0, -1], left: [-1, 0], right: [1, 0] };
const idleFrame = (f: Facing): number => FACINGS.indexOf(f) * 3;

export function evalCond(cond: string | undefined): boolean {
  if (!cond) return true;
  return cond.split(',').every((part) => {
    const p = part.trim();
    return p.startsWith('!') ? !state.flag(p.slice(1)) : state.flag(p);
  });
}

interface Target {
  kind: 'obj' | 'npc';
  id: string;
  interact: string;
  label: string;
}

interface LiveObject {
  def: MapObject;
  sprite: Phaser.GameObjects.Sprite;
  body: Phaser.GameObjects.Zone | null;
  light: Phaser.GameObjects.Image | null;
}

interface LiveNpc {
  def: NpcDef;
  sprite: Phaser.Physics.Arcade.Sprite;
  facing: Facing;
  glow: Phaser.GameObjects.Image | null;
  target: { x: number; y: number } | null;
  wait: number;
  scripted: boolean;
}

export class WorldScene extends Phaser.Scene {
  map: MapDef;
  player!: Phaser.Physics.Arcade.Sprite;
  facing: Facing = 'down';
  private shadow!: Phaser.GameObjects.Image;
  private solidGroup!: Phaser.Physics.Arcade.StaticGroup;
  private layer!: Phaser.Tilemaps.TilemapLayer;
  private objects = new Map<string, LiveObject>();
  private npcs = new Map<string, LiveNpc>();
  private cat: Phaser.Physics.Arcade.Sprite | null = null;
  private catState = { target: null as { x: number; y: number } | null, wait: 0 };
  private prompt!: Phaser.GameObjects.Sprite;
  private target: Target | null = null;
  private busy = false;
  private zonesInside = new Set<string>();
  private waterTiles: [number, number][] = [];
  private waterFrame = 0;
  private saveTimer = 0;
  private stepTimer = 0;
  private glows: { img: Phaser.GameObjects.Image; base: number; flicker: boolean; pulse: boolean }[] = [];
  private unsub: (() => void)[] = [];
  private spawnName = 'start';
  private spawnPos: { x: number; y: number; facing: Facing } | null = null;
  playerChar = 'player-a';

  constructor(map: MapDef) {
    super({ key: map.id });
    this.map = map;
  }

  init(data: { spawn?: string; pos?: { x: number; y: number; facing: Facing } | null }): void {
    this.spawnName = data?.spawn ?? 'start';
    this.spawnPos = data?.pos ?? null;
    this.objects.clear();
    this.npcs.clear();
    this.glows = [];
    this.zonesInside.clear();
    this.target = null;
    this.busy = false;
    this.cat = null;
  }

  create(): void {
    const m = this.map;
    const W = m.w * T;
    const H = m.h * T;
    this.cameras.main.setBackgroundColor('#07080e');
    this.physics.world.setBounds(0, 0, W, H);

    // Tiles
    const data = m.tiles.map((row) => row.map((name) => TILE_INDEX[name] ?? 0));
    const tilemap = this.make.tilemap({ data, tileWidth: T, tileHeight: T });
    const tileset = tilemap.addTilesetImage('tiles', 'tiles', T, T, 0, 0)!;
    this.layer = tilemap.createLayer(0, tileset, 0, 0)!;
    this.layer.setDepth(-100);
    const solidIdx = [...SOLID].map((n) => TILE_INDEX[n]!).filter((i) => i !== undefined);
    this.layer.setCollision(solidIdx);
    this.waterTiles = [];
    m.tiles.forEach((row, y) => row.forEach((name, x) => (name === 'water' ? this.waterTiles.push([x, y]) : null)));

    this.solidGroup = this.physics.add.staticGroup();

    // Player
    this.playerChar = `player-${['a', 'b', 'c'][state.data.look] ?? 'a'}`;
    const sp = this.spawnPos ?? m.spawns[this.spawnName] ?? m.spawns.start!;
    this.facing = sp.facing;
    this.player = this.physics.add.sprite(sp.x, sp.y, charKey(this.playerChar), idleFrame(this.facing));
    this.player.setOrigin(0.5, 1);
    const body = this.player.body as Phaser.Physics.Arcade.Body;
    body.setSize(10, 6);
    body.setOffset(3, 18);
    this.player.setCollideWorldBounds(true);
    this.shadow = this.add.image(sp.x, sp.y, 'shadow').setOrigin(0.5, 0.6).setDepth(-50).setAlpha(0.9);

    // Props and people
    for (const def of m.objects) this.spawnObject(def);
    for (const def of m.npcs) this.spawnNpc(def);
    this.spawnCat();

    this.physics.add.collider(this.player, this.layer);
    this.physics.add.collider(this.player, this.solidGroup);

    // Night tint over everything, then additive lights on top
    const tint = this.add.rectangle(0, 0, W, H, m.tint, 1).setOrigin(0).setDepth(90000);
    tint.setBlendMode(Phaser.BlendModes.MULTIPLY);
    if (m.ambient === 'motes') this.addMotes(W, H);
    if (m.ambient === 'dust') this.addDust(W, H);

    this.prompt = this.add.sprite(0, 0, 'prompt', 0).setDepth(100002).setVisible(false).setOrigin(0.5, 1);
    this.prompt.play('prompt-bob');

    // Camera
    this.cameras.main.startFollow(this.player, true, 0.18, 0.18);
    this.cameras.main.roundPixels = true;
    this.fitCamera();
    this.scale.on('resize', this.fitCamera, this);
    this.events.once('shutdown', () => {
      this.scale.off('resize', this.fitCamera, this);
      this.unsub.forEach((u) => u());
      this.unsub = [];
    });

    // React to story flags (gates opening, people appearing)
    this.unsub.push(state.on('flag', () => this.refresh()));

    // Water animation
    if (this.waterTiles.length)
      this.time.addEvent({
        delay: 650,
        loop: true,
        callback: () => {
          this.waterFrame ^= 1;
          const idx = TILE_INDEX[this.waterFrame ? 'water_b' : 'water']!;
          for (const [x, y] of this.waterTiles) this.layer.putTileAt(idx, x, y);
        },
      });

    audio.music(m.music);
    state.data.scene = m.id;
    void hooks.ready(this);
  }

  // --------------------------------------------------------------- spawning

  private spawnObject(def: MapObject): void {
    const s = this.add.sprite(def.x, def.y, def.tex, def.frame ?? 0).setOrigin(0.5, 1);
    if (def.flipX) s.setFlipX(true);
    if (def.alpha !== undefined) s.setAlpha(def.alpha);
    s.setDepth(def.floor ? -60 : def.y + (def.depth ?? 0));
    if (def.anim && this.anims.exists(`${def.tex}-anim`)) s.play({ key: `${def.tex}-anim`, startFrame: Math.floor(Math.random() * 2) });
    let body: Phaser.GameObjects.Zone | null = null;
    if (def.body) {
      const dy = def.body.dy ?? 0;
      body = this.add.zone(def.x, def.y - dy - def.body.h / 2, def.body.w, def.body.h);
      this.solidGroup.add(body);
    }
    let light: Phaser.GameObjects.Image | null = null;
    if (def.light) {
      light = this.add
        .image(def.x + (def.light.dx ?? 0), def.y + (def.light.dy ?? -s.height / 2), def.light.tex)
        .setBlendMode(Phaser.BlendModes.ADD)
        .setDepth(90001)
        .setScale(def.light.scale ?? 1)
        .setAlpha(def.light.alpha ?? 0.5);
      this.glows.push({ img: light, base: def.light.alpha ?? 0.5, flicker: Boolean(def.light.flicker), pulse: Boolean(def.light.pulse) });
    }
    const live: LiveObject = { def, sprite: s, body, light };
    this.objects.set(def.id, live);
    this.applyObjectState(live);
  }

  private applyObjectState(o: LiveObject): void {
    const visible = evalCond(o.def.showIf);
    o.sprite.setVisible(visible);
    o.light?.setVisible(visible);
    if (o.body) {
      const b = o.body.body as Phaser.Physics.Arcade.StaticBody;
      b.enable = visible;
    }
    if (o.def.frameIf) {
      let frame = o.def.frame ?? 0;
      for (const [cond, f] of o.def.frameIf) if (evalCond(cond)) frame = f;
      if (!o.def.anim) o.sprite.setFrame(frame);
    }
  }

  private spawnNpc(def: NpcDef): void {
    const s = this.physics.add.sprite(def.x, def.y, charKey(def.char), idleFrame(def.facing)).setOrigin(0.5, 1);
    const b = s.body as Phaser.Physics.Arcade.Body;
    b.setSize(10, 6);
    b.setOffset(3, 18);
    b.setImmovable(true);
    s.setDepth(def.y);
    let glow: Phaser.GameObjects.Image | null = null;
    if (def.hologram) {
      s.setTint(0x73eff7);
      s.setAlpha(0.78);
      glow = this.add.image(def.x, def.y - 10, 'glow-cyan').setBlendMode(Phaser.BlendModes.ADD).setDepth(90001).setAlpha(0.45).setScale(0.8);
      this.tweens.add({ targets: s, alpha: { from: 0.55, to: 0.85 }, duration: 140, yoyo: true, repeat: -1, repeatDelay: 1800 });
    }
    const live: LiveNpc = { def, sprite: s, facing: def.facing, glow, target: null, wait: 1000 + Math.random() * 3000, scripted: false };
    this.npcs.set(def.id, live);
    if (!def.hologram) this.physics.add.collider(this.player, s);
    if (def.wander) {
      this.physics.add.collider(s, this.layer);
      this.physics.add.collider(s, this.solidGroup);
    }
    this.applyNpcState(live);
  }

  private applyNpcState(n: LiveNpc): void {
    const visible = evalCond(n.def.showIf);
    n.sprite.setVisible(visible);
    (n.sprite.body as Phaser.Physics.Arcade.Body).enable = visible;
    n.glow?.setVisible(visible);
  }

  private spawnCat(): void {
    const m = this.map;
    if (!m.catMode || !state.flag('pro.metCat') && m.catMode === 'follow') return;
    const start = m.catMode === 'follow' ? { x: this.player.x - 18, y: this.player.y + 4 } : { x: (m.catArea?.x ?? 100) + 40, y: (m.catArea?.y ?? 100) + 40 };
    const c = this.physics.add.sprite(start.x, start.y, 'cat', 0).setOrigin(0.5, 1);
    const b = c.body as Phaser.Physics.Arcade.Body;
    b.setSize(10, 5);
    b.setOffset(3, 10);
    this.physics.add.collider(c, this.layer);
    if (m.catMode === 'wander') {
      this.physics.add.collider(c, this.solidGroup);
      b.setImmovable(true);
      this.physics.add.collider(this.player, c);
    }
    this.cat = c;
  }

  /** Re-evaluate visibility conditions after a story flag changes. */
  refresh(): void {
    for (const o of this.objects.values()) this.applyObjectState(o);
    for (const n of this.npcs.values()) this.applyNpcState(n);
    if (!this.cat && this.map.catMode === 'follow' && state.flag('pro.metCat')) this.spawnCat();
  }

  // ------------------------------------------------------------------ update

  override update(_t: number, dtMs: number): void {
    const dt = Math.min(dtMs, 50);
    this.updatePlayer();
    this.updateNpcs(dt);
    this.updateCat(dt);
    this.updateLights();
    this.updateTarget();
    this.updateZones();
    this.player.setDepth(this.player.y);
    this.shadow.setPosition(this.player.x, this.player.y);
    this.saveTimer += dt;
    if (this.saveTimer > 15000 && !this.busy) {
      this.saveTimer = 0;
      this.savePosition();
    }
    if (input.consume('act') && this.target && !this.busy) void this.trigger(this.target.interact);
  }

  private updatePlayer(): void {
    const ax = this.busy ? { x: 0, y: 0 } : input.axis();
    const speed = input.running() ? 112 : 74;
    let vx = ax.x;
    let vy = ax.y;
    const len = Math.hypot(vx, vy);
    if (len > 1) {
      vx /= len;
      vy /= len;
    }
    this.player.setVelocity(vx * speed, vy * speed);
    const key = charKey(this.playerChar);
    if (len > 0.1) {
      if (Math.abs(vx) > Math.abs(vy) + 0.05) this.facing = vx < 0 ? 'left' : 'right';
      else if (Math.abs(vy) > 0.05) this.facing = vy < 0 ? 'up' : 'down';
      const anim = `${key}-walk-${this.facing}`;
      if (this.player.anims.currentAnim?.key !== anim || !this.player.anims.isPlaying) this.player.play(anim);
      this.player.anims.timeScale = input.running() ? 1.5 : 1;
      this.stepTimer += this.game.loop.delta;
      if (this.stepTimer > (input.running() ? 220 : 300)) {
        this.stepTimer = 0;
        audio.sfx('step');
      }
    } else {
      this.player.anims.stop();
      this.player.setFrame(idleFrame(this.facing));
    }
  }

  private updateNpcs(dt: number): void {
    for (const n of this.npcs.values()) {
      if (!n.sprite.visible) continue;
      const s = n.sprite;
      if ((n.def.wander || n.scripted) && n.target) {
        const dx = n.target.x - s.x;
        const dy = n.target.y - s.y;
        const d = Math.hypot(dx, dy);
        if (d < 2) {
          s.setVelocity(0, 0);
          n.target = null;
          n.wait = 1500 + Math.random() * 4000;
          s.anims.stop();
          s.setFrame(idleFrame(n.facing));
        } else {
          const sp = n.scripted ? 60 : 32;
          s.setVelocity((dx / d) * sp, (dy / d) * sp);
          n.facing = Math.abs(dx) > Math.abs(dy) ? (dx < 0 ? 'left' : 'right') : dy < 0 ? 'up' : 'down';
          const anim = `${charKey(n.def.char)}-walk-${n.facing}`;
          if (s.anims.currentAnim?.key !== anim || !s.anims.isPlaying) s.play(anim);
          const body = s.body as Phaser.Physics.Arcade.Body;
          if (!n.scripted && (body.blocked.left || body.blocked.right || body.blocked.up || body.blocked.down)) n.target = null;
        }
      } else if (n.def.wander && !this.busy) {
        n.wait -= dt;
        if (n.wait <= 0) {
          const w = n.def.wander;
          n.target = { x: w.x + Math.random() * w.w, y: w.y + Math.random() * w.h };
        }
      }
      s.setDepth(s.y);
      n.glow?.setPosition(s.x, s.y - 10);
    }
  }

  private updateCat(dt: number): void {
    const c = this.cat;
    if (!c) return;
    let tx: number | null = null;
    let ty = 0;
    if (this.map.catMode === 'follow') {
      const [fx, fy] = FACING_VEC[this.facing];
      const gx = this.player.x - fx * 20 - 10;
      const gy = this.player.y - fy * 14 + 4;
      const d = Math.hypot(gx - c.x, gy - c.y);
      if (d > 26) {
        tx = gx;
        ty = gy;
      }
      if (d > 220) c.setPosition(gx, gy);
    } else {
      this.catState.wait -= dt;
      if (this.catState.target) {
        tx = this.catState.target.x;
        ty = this.catState.target.y;
        if (Math.hypot(tx - c.x, ty - c.y) < 3) {
          this.catState.target = null;
          this.catState.wait = 2500 + Math.random() * 5000;
          tx = null;
        }
      } else if (this.catState.wait <= 0 && this.map.catArea && !this.busy) {
        const a = this.map.catArea;
        this.catState.target = { x: a.x + Math.random() * a.w, y: a.y + Math.random() * a.h };
      }
    }
    if (tx !== null) {
      const dx = tx - c.x;
      const dy = ty - c.y;
      const d = Math.hypot(dx, dy) || 1;
      const sp = this.map.catMode === 'follow' ? Math.min(120, 60 + d) : 38;
      c.setVelocity((dx / d) * sp, (dy / d) * sp);
      const anim = dx < 0 ? 'cat-walk-left' : 'cat-walk-right';
      if (c.anims.currentAnim?.key !== anim || !c.anims.isPlaying) c.play(anim);
    } else {
      c.setVelocity(0, 0);
      if (c.anims.isPlaying) {
        const last = c.anims.currentAnim?.key;
        c.anims.stop();
        c.setFrame(last === 'cat-walk-right' ? 5 : 2);
      }
    }
    c.setDepth(c.y);
  }

  private updateLights(): void {
    const t = this.time.now;
    for (const l of this.glows) {
      if (l.flicker) l.img.setAlpha(l.base * (0.82 + Math.random() * 0.22));
      else if (l.pulse) l.img.setAlpha(l.base * (0.75 + 0.25 * Math.sin(t / 420)));
    }
  }

  private updateTarget(): void {
    if (this.busy || !input.worldEnabled) {
      this.prompt.setVisible(false);
      return;
    }
    const [fx, fy] = FACING_VEC[this.facing];
    const px = this.player.x + fx * 9;
    const py = this.player.y - 4 + fy * 9;
    let best: Target | null = null;
    let bestD = Infinity;
    let promptAt = { x: 0, y: 0 };
    for (const o of this.objects.values()) {
      if (!o.def.interact || !o.sprite.visible) continue;
      const ix = o.def.x + (o.def.ix ?? 0);
      const iy = o.def.y + (o.def.iy ?? -4);
      const d = Math.hypot(ix - px, iy - py);
      if (d < (o.def.radius ?? 20) && d < bestD) {
        bestD = d;
        best = { kind: 'obj', id: o.def.id, interact: o.def.interact, label: o.def.label ?? 'Examine' };
        promptAt = { x: ix, y: Math.min(iy - 10, o.def.y - Math.min(o.sprite.height, 34)) - 2 };
      }
    }
    for (const n of this.npcs.values()) {
      if (!n.def.interact || !n.sprite.visible) continue;
      const d = Math.hypot(n.sprite.x - px, n.sprite.y - 6 - py);
      if (d < 22 && d < bestD) {
        bestD = d;
        best = { kind: 'npc', id: n.def.id, interact: n.def.interact, label: n.def.label ?? 'Talk' };
        promptAt = { x: n.sprite.x, y: n.sprite.y - 26 };
      }
    }
    if (this.cat) {
      const d = Math.hypot(this.cat.x - px, this.cat.y - 4 - py);
      if (d < 16 && d < bestD) {
        best = { kind: 'npc', id: 'nonce', interact: 'nonce', label: 'Pet Nonce' };
        promptAt = { x: this.cat.x, y: this.cat.y - 16 };
      }
    }
    const changed = best?.interact !== this.target?.interact;
    this.target = best;
    if (best) {
      this.prompt.setPosition(Math.round(promptAt.x), Math.round(promptAt.y)).setVisible(true);
      if (changed) hud.setHint(`<kbd>E</kbd> ${best.label}`);
    } else {
      this.prompt.setVisible(false);
      if (changed) hud.setHint(null);
    }
  }

  private updateZones(): void {
    for (const z of this.map.zones ?? []) {
      const inside = this.player.x >= z.x && this.player.x < z.x + z.w && this.player.y >= z.y && this.player.y < z.y + z.h;
      if (inside && !this.zonesInside.has(z.id)) {
        this.zonesInside.add(z.id);
        void hooks.zone(this, z.id);
      } else if (!inside) this.zonesInside.delete(z.id);
    }
  }

  // --------------------------------------------------------------- interaction

  async trigger(interactId: string): Promise<void> {
    if (this.busy) return;
    this.busy = true;
    this.prompt.setVisible(false);
    hud.setHint(null);
    this.player.setVelocity(0, 0);
    this.player.anims.stop();
    this.player.setFrame(idleFrame(this.facing));
    try {
      const npc = this.target?.kind === 'npc' ? this.npcs.get(this.target.id) : undefined;
      if (npc && !npc.def.hologram) this.npcFace(npc.def.id, this.facingTowards(npc.sprite.x, npc.sprite.y, this.player.x, this.player.y));
      await hooks.interact(this, interactId);
    } catch (e) {
      console.error(e);
    } finally {
      this.busy = false;
      this.target = null;
      input.flush(150);
      this.savePosition();
    }
  }

  /** Run a story routine with player control suspended. */
  async cutscene(fn: () => Promise<void>): Promise<void> {
    const was = this.busy;
    this.busy = true;
    this.player.setVelocity(0, 0);
    this.player.anims.stop();
    this.player.setFrame(idleFrame(this.facing));
    try {
      await fn();
    } finally {
      this.busy = was;
      input.flush(150);
    }
  }

  savePosition(): void {
    state.data.scene = this.map.id;
    state.data.pos = { x: Math.round(this.player.x), y: Math.round(this.player.y), facing: this.facing };
    state.save();
  }

  // --------------------------------------------------------------- story api

  facingTowards(fromX: number, fromY: number, toX: number, toY: number): Facing {
    const dx = toX - fromX;
    const dy = toY - fromY;
    return Math.abs(dx) > Math.abs(dy) ? (dx < 0 ? 'left' : 'right') : dy < 0 ? 'up' : 'down';
  }

  npcFace(id: string, f: Facing): void {
    const n = this.npcs.get(id);
    if (!n) return;
    n.facing = f;
    n.sprite.anims.stop();
    n.sprite.setFrame(idleFrame(f));
  }

  playerFace(f: Facing): void {
    this.facing = f;
    this.player.setFrame(idleFrame(f));
  }

  npcPos(id: string): { x: number; y: number } | null {
    const n = this.npcs.get(id);
    return n ? { x: n.sprite.x, y: n.sprite.y } : null;
  }

  /** Walk an NPC to a point; resolves on arrival. */
  npcWalk(id: string, x: number, y: number): Promise<void> {
    const n = this.npcs.get(id);
    if (!n) return Promise.resolve();
    n.scripted = true;
    n.target = { x, y };
    return new Promise((resolve) => {
      const check = (): void => {
        if (!n.target) {
          n.scripted = false;
          resolve();
        } else this.time.delayedCall(50, check);
      };
      check();
    });
  }

  /** Walk the player to a point (used by cutscenes). */
  playerWalk(x: number, y: number, speed = 70): Promise<void> {
    return new Promise((resolve) => {
      const key = charKey(this.playerChar);
      const step = (): void => {
        const dx = x - this.player.x;
        const dy = y - this.player.y;
        const d = Math.hypot(dx, dy);
        if (d < 2) {
          this.player.setVelocity(0, 0);
          this.player.anims.stop();
          this.player.setFrame(idleFrame(this.facing));
          resolve();
          return;
        }
        this.facing = this.facingTowards(0, 0, dx, dy);
        this.player.setVelocity((dx / d) * speed, (dy / d) * speed);
        const anim = `${key}-walk-${this.facing}`;
        if (this.player.anims.currentAnim?.key !== anim || !this.player.anims.isPlaying) this.player.play(anim);
        this.time.delayedCall(30, step);
      };
      step();
    });
  }

  setObjectFrame(id: string, frame: number): void {
    const o = this.objects.get(id);
    if (!o) return;
    o.sprite.anims.stop();
    o.sprite.setFrame(frame);
  }

  objectPos(id: string): { x: number; y: number } | null {
    const o = this.objects.get(id);
    return o ? { x: o.def.x, y: o.def.y } : null;
  }

  panTo(x: number, y: number, ms = 900): Promise<void> {
    return new Promise((resolve) => {
      this.cameras.main.stopFollow();
      this.cameras.main.pan(x, y, ms, 'Sine.easeInOut', false, (_c, p) => {
        if (p >= 1) resolve();
      });
    });
  }

  follow(): void {
    this.cameras.main.startFollow(this.player, true, 0.18, 0.18);
  }

  shake(ms = 300, intensity = 0.006): void {
    if (state.settings.reducedMotion) return;
    this.cameras.main.shake(ms, intensity);
  }

  flash(ms = 250, color = 0x73eff7): void {
    const c = Phaser.Display.Color.IntegerToRGB(color);
    this.cameras.main.flash(ms, c.r, c.g, c.b);
  }

  /** A short burst of light particles at a point. */
  burst(x: number, y: number, tint = 0x73eff7, n = 24): void {
    const e = this.add.particles(x, y, 'dot', {
      speed: { min: 20, max: 70 },
      angle: { min: 0, max: 360 },
      lifespan: 900,
      scale: { start: 1, end: 0 },
      alpha: { start: 1, end: 0 },
      tint,
      blendMode: Phaser.BlendModes.ADD,
      emitting: false,
    });
    e.setDepth(100001);
    e.explode(n);
    this.time.delayedCall(1200, () => e.destroy());
  }

  // -------------------------------------------------------------- ambience

  private addMotes(W: number, H: number): void {
    const e = this.add.particles(0, 0, 'dot', {
      x: { min: 0, max: W },
      y: { min: 0, max: H },
      lifespan: { min: 3000, max: 7000 },
      speedY: { min: -10, max: -3 },
      speedX: { min: -3, max: 3 },
      scale: { min: 0.5, max: 1 },
      alpha: { start: 0.75, end: 0 },
      tint: [0x73eff7, 0xff3e9a, 0xc792ff, 0xa7f070],
      frequency: 90,
      quantity: 1,
      blendMode: Phaser.BlendModes.ADD,
    });
    e.setDepth(90002);
  }

  private addDust(W: number, H: number): void {
    const e = this.add.particles(0, 0, 'dot', {
      x: { min: 0, max: W },
      y: { min: 0, max: H },
      lifespan: { min: 4000, max: 8000 },
      speedY: { min: -2, max: 2 },
      speedX: { min: -2, max: 2 },
      scale: 0.5,
      alpha: { start: 0.35, end: 0 },
      tint: 0xffcd75,
      frequency: 400,
      blendMode: Phaser.BlendModes.ADD,
    });
    e.setDepth(90002);
  }

  private fitCamera(): void {
    const cam = this.cameras.main;
    const vw = this.scale.width;
    const vh = this.scale.height;
    const zoom = Phaser.Math.Clamp(Math.floor(Math.min(vw / 300, vh / 190)), 2, 6);
    cam.setZoom(zoom);
    const W = this.map.w * T;
    const H = this.map.h * T;
    const viewW = vw / zoom;
    const viewH = vh / zoom;
    const bx = W < viewW ? -(viewW - W) / 2 : 0;
    const by = H < viewH ? -(viewH - H) / 2 : 0;
    cam.setBounds(bx, by, Math.max(W, viewW), Math.max(H, viewH));
  }
}
