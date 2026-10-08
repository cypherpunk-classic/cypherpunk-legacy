import { COLORS as C } from './palette';
import { ctx2d, fill, makeCanvas, prng, px } from './draw';

/**
 * Ground and wall tiles (16x16), painted procedurally. The tileset texture is
 * one row-major sheet; maps refer to tiles by name and `TILE_INDEX` resolves
 * the frame number. Tiles listed in `SOLID` block movement.
 */

export const T = 16;

type Painter = (g: CanvasRenderingContext2D, x: number, y: number) => void;

function noise(g: CanvasRenderingContext2D, x: number, y: number, seed: number, colors: string[], density: number): void {
  const r = prng(seed);
  for (let j = 0; j < T; j++)
    for (let i = 0; i < T; i++) {
      if (r() < density) px(g, x + i, y + j, colors[Math.floor(r() * colors.length)]!);
    }
}

const woodFloor = (variant: number): Painter => (g, x, y) => {
  fill(g, x, y, T, T, C.l!);
  const r = prng(11 + variant);
  for (let row = 0; row < 4; row++) {
    const yy = y + row * 4;
    fill(g, x, yy + 3, T, 1, C.k!);
    const seam = (row * 7 + variant * 5) % 16;
    fill(g, x + seam, yy, 1, 3, C.k!);
    for (let i = 0; i < 5; i++) px(g, x + Math.floor(r() * 16), yy + Math.floor(r() * 3), r() < 0.5 ? C.m! : '#6b4022');
  }
};

const logWall: Painter = (g, x, y) => {
  fill(g, x, y, T, T, C.m!);
  for (let row = 0; row < 4; row++) {
    const yy = y + row * 4;
    fill(g, x, yy, T, 1, '#b97f49');
    fill(g, x, yy + 2, T, 1, C.l!);
    fill(g, x, yy + 3, T, 1, C.k!);
  }
  px(g, x + 3, y + 1, C.l!);
  px(g, x + 11, y + 9, C.l!);
  px(g, x + 7, y + 13, C.l!);
};

const logWallWindow: Painter = (g, x, y) => {
  logWall(g, x, y);
  fill(g, x + 2, y + 2, 12, 11, C.k!);
  fill(g, x + 3, y + 3, 10, 9, '#16203a');
  const r = prng(3);
  for (let i = 0; i < 9; i++) px(g, x + 3 + Math.floor(r() * 10), y + 3 + Math.floor(r() * 9), r() < 0.5 ? '#3a5a8a' : '#2a3f66');
  fill(g, x + 7, y + 3, 1, 9, C.k!);
  fill(g, x + 3, y + 7, 10, 1, C.k!);
  fill(g, x + 1, y + 13, 14, 2, C.n!);
  fill(g, x + 1, y + 14, 14, 1, C.m!);
};

const wallTop: Painter = (g, x, y) => {
  fill(g, x, y, T, T, C.j!);
  fill(g, x, y + T - 2, T, 2, C.k!);
  noise(g, x, y, 5, ['#3a2216', '#24140c'], 0.08);
};

const hearth: Painter = (g, x, y) => {
  fill(g, x, y, T, T, '#4b4f5c');
  for (const [a, b, w, h] of [[0, 0, 7, 5], [8, 0, 8, 5], [0, 6, 4, 4], [5, 6, 6, 4], [12, 6, 4, 4], [0, 11, 9, 5], [10, 11, 6, 5]] as const) {
    fill(g, x + a, y + b, w - 1, h - 1, '#5f6b7d');
    px(g, x + a, y + b, '#7a8699');
  }
};

const doorTile: Painter = (g, x, y) => {
  logWall(g, x, y);
  fill(g, x + 2, y, 12, T, C.j!);
  fill(g, x + 3, y + 1, 10, T - 1, C.l!);
  for (let i = 0; i < 3; i++) fill(g, x + 3 + i * 4, y + 1, 1, T - 1, C.k!);
  px(g, x + 11, y + 9, C.e!);
};

const asphalt = (v: number): Painter => (g, x, y) => {
  fill(g, x, y, T, T, C.t!);
  noise(g, x, y, 21 + v, ['#262a38', '#191c27', '#2b3040'], 0.18);
};

const roadDash = (vertical: boolean): Painter => (g, x, y) => {
  asphalt(0)(g, x, y);
  if (vertical) fill(g, x + 7, y + 3, 2, 10, '#c9b458');
  else fill(g, x + 3, y + 7, 10, 2, '#c9b458');
};

const crosswalk = (vertical: boolean): Painter => (g, x, y) => {
  asphalt(1)(g, x, y);
  for (let i = 1; i < 16; i += 4) {
    if (vertical) fill(g, x + 1, y + i, 14, 2, '#c7ccd6');
    else fill(g, x + i, y + 1, 2, 14, '#c7ccd6');
  }
};

const sidewalk = (v: number): Painter => (g, x, y) => {
  fill(g, x, y, T, T, '#4a5263');
  fill(g, x, y, T, 1, '#596274');
  fill(g, x, y, 1, T, '#596274');
  fill(g, x + 8, y, 1, T, '#3d4554');
  fill(g, x, y + 8, T, 1, '#3d4554');
  noise(g, x, y, 31 + v, ['#535c6e', '#444c5c'], 0.1);
  if (v === 1) {
    px(g, x + 3, y + 4, '#2f3644');
    px(g, x + 4, y + 5, '#2f3644');
    px(g, x + 5, y + 5, '#2f3644');
  }
};

const plaza = (v: number): Painter => (g, x, y) => {
  fill(g, x, y, T, T, v === 0 ? '#1d2236' : '#22283f');
  fill(g, x, y, T, 1, '#2c3452');
  fill(g, x, y, 1, T, '#2c3452');
  noise(g, x, y, 41 + v, ['#262d47', '#1a1f33'], 0.08);
};

const plazaGlow: Painter = (g, x, y) => {
  plaza(0)(g, x, y);
  fill(g, x, y + 7, T, 2, '#1f6b78');
  fill(g, x + 7, y, 2, T, '#1f6b78');
  fill(g, x + 7, y + 7, 2, 2, C.a!);
};

const grass = (v: number): Painter => (g, x, y) => {
  fill(g, x, y, T, T, '#1f4a33');
  noise(g, x, y, 51 + v, ['#2a5e3c', '#173326', '#2f6b44'], 0.35);
  if (v === 1) for (const [a, b] of [[3, 4], [10, 2], [6, 11], [13, 12]] as const) {
    px(g, x + a, y + b, '#3f8a55');
    px(g, x + a, y + b - 1, '#3f8a55');
  }
};

const flowers: Painter = (g, x, y) => {
  grass(0)(g, x, y);
  for (const [a, b, c] of [[3, 3, C.i], [11, 5, C.e], [6, 10, C.a], [12, 12, C.i], [2, 13, C['*']]] as const) {
    px(g, x + a, y + b, c!);
    px(g, x + a, y + b + 1, '#2f6b44');
  }
};

const water = (frame: number): Painter => (g, x, y) => {
  fill(g, x, y, T, T, '#11203a');
  const r = prng(61);
  for (let i = 0; i < 6; i++) {
    const wx = Math.floor(r() * 12);
    const wy = Math.floor(r() * 15);
    fill(g, x + ((wx + frame * 2) % 13), y + wy, 3, 1, i % 2 ? '#1e3a63' : '#24497a');
  }
  if (frame === 1) px(g, x + 9, y + 4, '#3e6fa8');
  else px(g, x + 4, y + 10, '#3e6fa8');
};

const waterEdge: Painter = (g, x, y) => {
  water(0)(g, x, y);
  fill(g, x, y, T, 3, '#4a5263');
  fill(g, x, y + 3, T, 1, '#8fb8d8');
  for (let i = 0; i < 16; i += 3) px(g, x + i, y + 4, '#5d86b0');
};

const dock = (vertical: boolean): Painter => (g, x, y) => {
  fill(g, x, y, T, T, C.l!);
  for (let i = 0; i < 4; i++) {
    if (vertical) fill(g, x + i * 4 + 3, y, 1, T, C.k!);
    else fill(g, x, y + i * 4 + 3, T, 1, C.k!);
  }
  noise(g, x, y, 71, [C.m!, '#6b4022'], 0.06);
  if (vertical) {
    px(g, x + 1, y + 2, C.s!);
    px(g, x + 9, y + 12, C.s!);
  } else {
    px(g, x + 2, y + 1, C.s!);
    px(g, x + 12, y + 9, C.s!);
  }
};

const hedge: Painter = (g, x, y) => {
  fill(g, x, y, T, T, '#173326');
  noise(g, x, y, 81, ['#2a5e3c', '#1f4a33', '#2f6b44'], 0.6);
  fill(g, x, y + T - 3, T, 3, '#10241a');
};

const fence: Painter = (g, x, y) => {
  sidewalk(0)(g, x, y);
  fill(g, x, y + 4, T, 2, '#5f6b7d');
  fill(g, x, y + 10, T, 2, '#5f6b7d');
  for (let i = 1; i < 16; i += 5) fill(g, x + i, y + 2, 2, 12, '#7a8699');
};

const railing: Painter = (g, x, y) => {
  waterEdge(g, x, y);
  fill(g, x, y, T, 1, '#94b0c2');
  for (let i = 1; i < 16; i += 5) fill(g, x + i, y, 1, 3, '#94b0c2');
};

const voidTile: Painter = (g, x, y) => {
  fill(g, x, y, T, T, '#07080e');
  px(g, x + 8, y + 8, '#141a2e');
};

const sand: Painter = (g, x, y) => {
  fill(g, x, y, T, T, '#3a3346');
  noise(g, x, y, 91, ['#463e55', '#2f2a3a'], 0.3);
};

const metalFloor: Painter = (g, x, y) => {
  fill(g, x, y, T, T, '#262b3b');
  fill(g, x, y, T, 1, '#323a50');
  fill(g, x, y, 1, T, '#323a50');
  px(g, x + 2, y + 2, '#4a5470');
  px(g, x + 13, y + 2, '#4a5470');
  px(g, x + 2, y + 13, '#4a5470');
  px(g, x + 13, y + 13, '#4a5470');
};

export const TILE_PAINTERS: [string, Painter][] = [
  ['empty', () => {}],
  ['wood', woodFloor(0)],
  ['wood2', woodFloor(1)],
  ['logwall', logWall],
  ['logwindow', logWallWindow],
  ['walltop', wallTop],
  ['hearth', hearth],
  ['door', doorTile],
  ['asphalt', asphalt(0)],
  ['asphalt2', asphalt(1)],
  ['dash_h', roadDash(false)],
  ['dash_v', roadDash(true)],
  ['cross_h', crosswalk(false)],
  ['cross_v', crosswalk(true)],
  ['sidewalk', sidewalk(0)],
  ['sidewalk2', sidewalk(1)],
  ['plaza', plaza(0)],
  ['plaza2', plaza(1)],
  ['plazaglow', plazaGlow],
  ['grass', grass(0)],
  ['grass2', grass(1)],
  ['flowers', flowers],
  ['water', water(0)],
  ['water_b', water(1)],
  ['wateredge', waterEdge],
  ['dock_v', dock(true)],
  ['dock_h', dock(false)],
  ['hedge', hedge],
  ['fence', fence],
  ['railing', railing],
  ['void', voidTile],
  ['sand', sand],
  ['metal', metalFloor],
];

export const TILE_INDEX: Record<string, number> = Object.fromEntries(TILE_PAINTERS.map(([n], i) => [n, i]));

export const SOLID = new Set(['logwall', 'logwindow', 'walltop', 'door', 'water', 'water_b', 'wateredge', 'hedge', 'fence', 'railing', 'void']);

export const TILESET_COLS = 16;

export function drawTileset(): HTMLCanvasElement {
  const rows = Math.ceil(TILE_PAINTERS.length / TILESET_COLS);
  const c = makeCanvas(TILESET_COLS * T, rows * T);
  const g = ctx2d(c);
  TILE_PAINTERS.forEach(([, paint], i) => paint(g, (i % TILESET_COLS) * T, Math.floor(i / TILESET_COLS) * T));
  return c;
}
