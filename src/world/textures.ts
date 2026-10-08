import * as Phaser from 'phaser';
import { CAST, PLAYER_LOOKS, FRAME_H, FRAME_W, FACINGS, drawCatSheet, drawCharacterSheet } from '../art/characters';
import { OBJECTS, drawObjectSheet, objectFrameSize, type ObjectArt } from '../art/objects';
import { BUILDINGS, EXTRA_OBJECTS, SIGNS } from '../art/buildings';
import { drawTileset, T } from '../art/tiles';
import { glowCanvas, makeCanvas, ctx2d, addOutline } from '../art/draw';
import { drawText } from '../art/font';

/**
 * Turn every code-defined drawing into a Phaser texture, and register the
 * animations that use them. Called once from BootScene.
 */

const ANIM_RATES: Record<string, number> = {
  fireplace: 7,
  kiosk: 1.5,
  vending: 1,
  antenna: 1.2,
  boat: 1,
  barrier_h: 5,
  barrier_v: 5,
  drone: 12,
  server: 2,
  coin: 2,
  projector: 2,
  pylon: 2,
  gatepad: 4,
  shard: 3,
  radio: 2,
  billboard: 1,
  checkpoint: 2,
};

/** Objects whose frames are states, not animation. */
const STATEFUL = new Set(['deskcrt', 'dresser', 'tree', 'vault', 'brickdrop', 'mailbox', 'archive']);

function addSheet(scene: Phaser.Scene, key: string, canvas: HTMLCanvasElement, fw: number, fh: number): number {
  if (scene.textures.exists(key)) scene.textures.remove(key);
  const tex = scene.textures.addCanvas(key, canvas)!;
  const cols = Math.floor(canvas.width / fw);
  const rows = Math.floor(canvas.height / fh);
  let i = 0;
  for (let r = 0; r < rows; r++)
    for (let c = 0; c < cols; c++) {
      tex.add(i, 0, c * fw, r * fh, fw, fh);
      i++;
    }
  return i;
}

function addObject(scene: Phaser.Scene, key: string, art: ObjectArt): void {
  const { w, h } = objectFrameSize(art);
  const n = addSheet(scene, key, drawObjectSheet(art), w, h);
  const name = key.replace(/^(obj|bld|sign)-/, '');
  const rate = key.startsWith('bld-') ? 0.4 : key.startsWith('sign-') ? 0.7 : ANIM_RATES[name];
  if (n > 1 && rate && !STATEFUL.has(name)) {
    scene.anims.create({ key: `${key}-anim`, frames: scene.anims.generateFrameNumbers(key, { start: 0, end: n - 1 }), frameRate: rate, repeat: -1 });
  }
}

export function charKey(id: string): string {
  return `char-${id}`;
}

export function generateTextures(scene: Phaser.Scene): void {
  // Tiles
  addSheet(scene, 'tiles', drawTileset(), T, T);

  // Characters
  for (const def of [...PLAYER_LOOKS, ...CAST]) {
    const key = charKey(def.id);
    addSheet(scene, key, drawCharacterSheet(def), FRAME_W, FRAME_H);
    FACINGS.forEach((facing, fi) => {
      scene.anims.create({
        key: `${key}-walk-${facing}`,
        frames: [1, 0, 2, 0].map((f) => ({ key, frame: fi * 3 + f })),
        frameRate: 8,
        repeat: -1,
      });
    });
  }

  // The cat
  addSheet(scene, 'cat', drawCatSheet(), 16, 16);
  scene.anims.create({ key: 'cat-walk-left', frames: [3, 4].map((f) => ({ key: 'cat', frame: f })), frameRate: 7, repeat: -1 });
  scene.anims.create({ key: 'cat-walk-right', frames: [6, 7].map((f) => ({ key: 'cat', frame: f })), frameRate: 7, repeat: -1 });

  // Props, buildings, signs
  for (const [k, art] of Object.entries(OBJECTS)) addObject(scene, `obj-${k}`, art);
  for (const [k, art] of Object.entries(EXTRA_OBJECTS)) addObject(scene, `obj-${k}`, art);
  for (const [k, art] of Object.entries(BUILDINGS)) addObject(scene, `bld-${k}`, art);
  for (const [k, art] of Object.entries(SIGNS)) addObject(scene, `sign-${k}`, art);

  // Lights
  const glows: [string, number, string][] = [
    ['glow-warm', 48, '#ffb347'],
    ['glow-fire', 64, '#ff7a2f'],
    ['glow-cyan', 48, '#3ef0ff'],
    ['glow-magenta', 48, '#ff3e9a'],
    ['glow-green', 40, '#5dff9a'],
    ['glow-gold', 48, '#ffcd75'],
    ['glow-blue', 48, '#73b7ff'],
    ['glow-violet', 48, '#c792ff'],
    ['glow-red', 40, '#ff5566'],
    ['glow-white', 64, '#dfe8ff'],
  ];
  for (const [k, r, c] of glows) if (!scene.textures.exists(k)) scene.textures.addCanvas(k, glowCanvas(r, c));

  // Interaction prompt: a tiny key cap that bobs
  const prompt = makeCanvas(22, 12);
  const pc = ctx2d(prompt);
  for (let f = 0; f < 2; f++) {
    const x0 = f * 11;
    pc.fillStyle = '#e6f6ff';
    pc.fillRect(x0 + 1, 1 + f, 9, 8);
    pc.fillStyle = '#9ab8d0';
    pc.fillRect(x0 + 1, 8 + f, 9, 1);
    drawText(pc, 'E', x0 + 4, 2 + f, '#0e0b16');
  }
  addOutline(prompt);
  addSheet(scene, 'prompt', prompt, 11, 12);
  scene.anims.create({ key: 'prompt-bob', frames: [0, 1].map((f) => ({ key: 'prompt', frame: f })), frameRate: 2.5, repeat: -1 });

  // Soft shadow and 1px particle
  const sh = makeCanvas(12, 5);
  const sg = ctx2d(sh);
  sg.fillStyle = 'rgba(0,0,0,0.35)';
  sg.fillRect(2, 0, 8, 5);
  sg.fillRect(0, 1, 12, 3);
  scene.textures.addCanvas('shadow', sh);
  const dot = makeCanvas(2, 2);
  ctx2d(dot).fillStyle = '#ffffff';
  ctx2d(dot).fillRect(0, 0, 2, 2);
  scene.textures.addCanvas('dot', dot);
  const drop = makeCanvas(1, 4);
  const dg = ctx2d(drop);
  dg.fillStyle = 'rgba(160,200,255,0.55)';
  dg.fillRect(0, 0, 1, 4);
  scene.textures.addCanvas('raindrop', drop);
}
