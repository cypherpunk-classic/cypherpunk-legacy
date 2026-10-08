import type { MapDef, MapObject } from '../types';

/** Grandpa's cabin: one room, rain outside, a fire going. 20 x 13 tiles. */

const W = 20;
const H = 13;
const t = (n: number): number => n * 16;

function tiles(): string[][] {
  const g: string[][] = [];
  for (let y = 0; y < H; y++) {
    const row: string[] = [];
    for (let x = 0; x < W; x++) {
      if (y === 0 || y === H - 1 || x === 0 || x === W - 1) row.push('walltop');
      else if (y === 1 || y === 2) row.push((x === 4 || x === 12) && y === 1 ? 'logwindow' : 'logwall');
      else row.push((x * 7 + y * 3) % 5 === 0 ? 'wood2' : 'wood');
    }
    g.push(row);
  }
  // the second window row continues the frame
  g[2]![4] = 'logwall';
  g[2]![12] = 'logwall';
  // hearth stones in front of the fireplace
  g[3]![9] = 'hearth';
  g[3]![10] = 'hearth';
  return g;
}

const objects: MapObject[] = [
  { id: 'rug', tex: 'obj-rug', x: t(9.5) + 4, y: t(7) + 4, floor: true },
  { id: 'bookshelf', tex: 'obj-bookshelf', x: t(2), y: t(3) + 4, body: { w: 30, h: 8 }, interact: 'cabin.bookshelf', label: 'Browse the shelf', radius: 22 },
  { id: 'photo', tex: 'obj-photo', x: t(6.5), y: t(2) + 6, depth: -40, interact: 'cabin.photo', label: 'Look at the photo', iy: 14, radius: 20 },
  { id: 'dresser', tex: 'obj-dresser', x: t(6.5), y: t(3) + 10, body: { w: 22, h: 8 }, interact: 'cabin.dresser', label: 'Open the drawer', frameIf: [['pro.letter', 1]] },
  {
    id: 'fireplace',
    tex: 'obj-fireplace',
    x: t(10),
    y: t(3) + 6,
    anim: true,
    body: { w: 30, h: 10 },
    interact: 'cabin.fireplace',
    label: 'Warm your hands',
    light: { tex: 'glow-fire', scale: 1.6, alpha: 0.55, flicker: true, dy: -10 },
  },
  { id: 'armchair', tex: 'obj-armchair', x: t(13) - 4, y: t(6) + 2, body: { w: 16, h: 8 }, interact: 'cabin.armchair', label: 'Sit for a moment' },
  {
    id: 'desk',
    tex: 'obj-deskcrt',
    x: t(15.5),
    y: t(3) + 14,
    body: { w: 32, h: 12 },
    interact: 'cabin.computer',
    label: 'Use the computer',
    frameIf: [['pro.booted', 1]],
  },
  { id: 'crtglow', tex: 'obj-note', x: t(15.5) - 2, y: t(3) + 4, alpha: 0, showIf: 'pro.booted', light: { tex: 'glow-green', scale: 0.8, alpha: 0.5, pulse: true, dy: -14 } },
  { id: 'door', tex: 'obj-door', x: t(18) + 4, y: t(3), depth: -30, interact: 'cabin.door', label: 'Open the door', iy: 2, radius: 18 },
  { id: 'coatrack', tex: 'obj-coatrack', x: t(18) + 8, y: t(7) + 4, body: { w: 8, h: 5 }, interact: 'cabin.coat', label: 'Check the coat' },
  { id: 'bed', tex: 'obj-bed', x: t(2) + 2, y: t(11) + 14, body: { w: 28, h: 28 }, interact: 'cabin.bed', label: 'Look at the bed', iy: -14, radius: 26 },
  { id: 'sidetable', tex: 'obj-sidetable', x: t(4) + 4, y: t(11) + 6, body: { w: 12, h: 6 }, light: { tex: 'glow-warm', scale: 0.9, alpha: 0.45, dy: -18 } },
  { id: 'plant', tex: 'obj-plant', x: t(18) + 4, y: t(11) + 14, body: { w: 10, h: 6 }, interact: 'cabin.plant', label: 'Inspect the plant' },
  { id: 'boxes', tex: 'obj-boxes', x: t(15) + 8, y: t(11) + 12, body: { w: 18, h: 8 }, interact: 'cabin.boxes', label: 'Open the boxes' },
  { id: 'window1', tex: 'obj-note', x: t(4.5), y: t(2) + 14, alpha: 0, interact: 'cabin.window', label: 'Look outside', radius: 18 },
  { id: 'window2', tex: 'obj-note', x: t(12.5), y: t(2) + 14, alpha: 0, interact: 'cabin.window', label: 'Look outside', radius: 18 },
];

export const cabinMap: MapDef = {
  id: 'cabin',
  title: "Grandpa's Cabin",
  w: W,
  h: H,
  tiles: tiles(),
  objects,
  npcs: [],
  spawns: {
    start: { x: t(17) + 8, y: t(4) + 10, facing: 'down' },
    computer: { x: t(15.5), y: t(5), facing: 'up' },
  },
  music: 'cabin',
  tint: 0x7a7090,
  ambient: 'dust',
  catMode: 'wander',
  catArea: { x: t(6), y: t(5), w: t(7), h: t(4) },
};
