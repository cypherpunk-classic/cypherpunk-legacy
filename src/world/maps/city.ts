import type { MapDef, MapObject, NpcDef, Zone } from '../types';

/**
 * Cryptopolis: six districts around Legacy Plaza, cut by four avenues.
 * Each district is fenced, with one gate that opens when the previous
 * chapter is done. 60 x 44 tiles.
 *
 *   Masquerade (5)   Observatory (6)   Mint (4)
 *   Exchange (2)     Legacy Plaza      Commons (3)
 *                 The Wharf (1, you arrive here)
 */

const W = 60;
const H = 44;
const t = (n: number): number => n * 16;

type Grid = string[][];

function grid(fillTile: string): Grid {
  return Array.from({ length: H }, () => Array.from({ length: W }, () => fillTile));
}

function rect(g: Grid, x: number, y: number, w: number, h: number, tile: string | ((x: number, y: number) => string)): void {
  for (let yy = y; yy < y + h; yy++)
    for (let xx = x; xx < x + w; xx++) {
      if (yy < 0 || yy >= H || xx < 0 || xx >= W) continue;
      g[yy]![xx] = typeof tile === 'string' ? tile : tile(xx, yy);
    }
}

function hRoad(g: Grid, y: number, x0: number, x1: number): void {
  rect(g, x0, y, x1 - x0 + 1, 3, (x) => 'asphalt' + ((x * 13) % 7 === 0 ? '2' : ''));
  for (let x = x0; x <= x1; x++) if (x % 2 === 0) g[y + 1]![x] = 'dash_h';
}

function vRoad(g: Grid, x: number, y0: number, y1: number): void {
  rect(g, x, y0, 3, y1 - y0 + 1, (_x, y) => 'asphalt' + ((y * 11) % 7 === 0 ? '2' : ''));
  for (let y = y0; y <= y1; y++) if (y % 2 === 0) g[y]![x + 1] = 'dash_v';
}

function tiles(): Grid {
  const g = grid('sidewalk');
  // texture the sidewalks a little
  for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) if ((x * 31 + y * 17) % 23 === 0) g[y]![x] = 'sidewalk2';

  // outer edge
  rect(g, 0, 0, W, 1, 'hedge');
  rect(g, 0, 0, 1, H, 'hedge');
  rect(g, W - 1, 0, 1, H, 'hedge');

  // district grounds
  rect(g, 1, 1, 17, 10, (x, y) => ((x + y) % 2 ? 'plaza2' : 'plaza'));
  rect(g, 23, 1, 14, 10, 'metal');
  rect(g, 42, 1, 16, 10, 'sidewalk');
  rect(g, 1, 16, 17, 11, 'metal');
  rect(g, 42, 16, 16, 11, (x, y) => ((x * 7 + y * 5) % 9 === 0 ? 'grass2' : (x * 3 + y * 11) % 17 === 0 ? 'flowers' : 'grass'));
  // a path through the park
  rect(g, 42, 21, 16, 2, 'sand');
  rect(g, 48, 16, 2, 11, 'sand');
  // plaza
  rect(g, 22, 15, 16, 13, (x, y) => ((x + y) % 2 ? 'plaza2' : 'plaza'));
  rect(g, 22, 21, 16, 1, 'plazaglow');
  rect(g, 29, 15, 2, 13, 'plazaglow');

  // avenues
  hRoad(g, 12, 1, 58);
  hRoad(g, 28, 1, 58);
  vRoad(g, 19, 12, 30);
  vRoad(g, 38, 12, 30);
  // crosswalks where avenues meet
  for (const [x, y] of [
    [19, 12],
    [38, 12],
    [19, 28],
    [38, 28],
  ] as const) {
    rect(g, x, y, 3, 3, 'asphalt');
    rect(g, x - 1, y, 1, 3, 'cross_v');
    rect(g, x + 3, y, 1, 3, 'cross_v');
    rect(g, x, y - 1, 3, 1, 'cross_h');
    rect(g, x, y + 3, 3, 1, 'cross_h');
  }

  // fences and hedges around gated districts (gaps are the gates)
  rect(g, 1, 11, 17, 1, 'fence'); // Masquerade south
  rect(g, 18, 1, 1, 11, 'fence'); // Masquerade east
  rect(g, 22, 1, 1, 11, 'fence'); // Observatory west
  rect(g, 37, 1, 1, 11, 'fence'); // Observatory east
  rect(g, 22, 11, 16, 1, 'fence'); // Observatory south
  rect(g, 41, 1, 1, 11, 'fence'); // Mint west
  rect(g, 41, 11, 18, 1, 'fence'); // Mint south
  rect(g, 1, 15, 18, 1, 'fence'); // Exchange north
  rect(g, 1, 27, 18, 1, 'fence'); // Exchange south
  rect(g, 18, 15, 1, 13, 'fence'); // Exchange east
  rect(g, 41, 15, 18, 1, 'hedge'); // Commons north
  rect(g, 41, 27, 18, 1, 'hedge'); // Commons south
  rect(g, 41, 15, 1, 13, 'hedge'); // Commons west
  // gates
  rect(g, 9, 11, 2, 1, 'sidewalk');
  rect(g, 29, 11, 2, 1, 'metal');
  rect(g, 48, 11, 2, 1, 'sidewalk');
  rect(g, 18, 21, 1, 2, 'metal');
  rect(g, 41, 21, 1, 2, 'sand');

  // the waterfront
  rect(g, 1, 37, 58, 1, 'dock_h');
  rect(g, 0, 38, W, 1, 'wateredge');
  rect(g, 0, 39, W, 5, 'water');
  rect(g, 33, 38, 3, 6, 'dock_v');
  rect(g, 10, 38, 3, 4, 'dock_v');
  rect(g, 0, 37, 1, 1, 'hedge');
  rect(g, W - 1, 37, 1, 1, 'hedge');
  return g;
}

const lamp = (id: string, x: number, y: number, tex = 'glow-cyan'): MapObject => ({ id, tex: 'obj-lamp', x, y, body: { w: 6, h: 4 }, light: { tex, scale: 1.1, alpha: 0.45, dy: -34 } });
const barrier = (dir: 'h' | 'v', id: string, x: number, y: number, cond: string): MapObject => ({
  id,
  tex: `obj-barrier_${dir}`,
  x,
  y,
  anim: true,
  body: { w: 16, h: 16 },
  showIf: `!${cond}`,
  interact: `gate.${id.split('-')[1]}`,
  label: 'Inspect the barrier',
  light: { tex: 'glow-magenta', scale: 0.5, alpha: 0.5, dy: -8, pulse: true },
});
const barrierH = (id: string, x: number, y: number, cond: string): MapObject => barrier('h', id, x, y, cond);
const barrierV = (id: string, x: number, y: number, cond: string): MapObject => barrier('v', id, x, y, cond);
const pylon = (id: string, x: number, y: number): MapObject => ({ id, tex: 'obj-pylon', x, y, anim: true, body: { w: 8, h: 6 } });
const kiosk = (id: string, x: number, y: number, interact: string): MapObject => ({ id, tex: 'obj-kiosk', x, y, anim: true, body: { w: 14, h: 6 }, interact, label: 'Use the terminal', light: { tex: 'glow-cyan', scale: 0.45, alpha: 0.4, dy: -22 } });

const objects: MapObject[] = [
  // --- The Wharf -----------------------------------------------------------
  { id: 'gatepad', tex: 'obj-gatepad', x: t(30), y: t(35) + 4, floor: true, anim: true, light: { tex: 'glow-cyan', scale: 1.0, alpha: 0.5, pulse: true, dy: -10 }, interact: 'wharf.gatepad', label: 'Step on the pad', iy: -8 },
  { id: 'sign-wharf', tex: 'sign-wharf', x: t(30), y: t(32) + 2, anim: true, light: { tex: 'glow-cyan', scale: 0.7, alpha: 0.35, dy: -6 } },
  { id: 'radioshack', tex: 'bld-radioshack', x: t(6) + 8, y: t(35) + 6, anim: true, body: { w: 80, h: 30 } },
  { id: 'sign-radio', tex: 'sign-radio', x: t(6) + 8, y: t(33) - 4, depth: 2, anim: true },
  { id: 'antenna', tex: 'obj-antenna', x: t(2) + 6, y: t(35) + 4, anim: true, body: { w: 8, h: 6 } },
  { id: 'radio', tex: 'obj-radio', x: t(11) + 8, y: t(36) + 2, anim: true, body: { w: 24, h: 8 }, interact: 'wharf.radio', label: "Use Mara's radio" },
  kiosk('kiosk-wharf', t(14), t(36) + 2, 'wharf.kiosk'),
  { id: 'billboard', tex: 'obj-billboard', x: t(22), y: t(35) + 4, anim: true, body: { w: 60, h: 6 }, interact: 'wharf.billboard', label: 'Read the billboard', light: { tex: 'glow-red', scale: 1.0, alpha: 0.35, dy: -28 } },
  { id: 'boathouse', tex: 'bld-boathouse', x: t(47), y: t(35) + 8, anim: true, body: { w: 128, h: 34 } },
  { id: 'sign-boathouse', tex: 'sign-boathouse', x: t(47), y: t(33) - 2, depth: 2, anim: true },
  kiosk('kiosk-boathouse', t(53) + 4, t(36) + 2, 'wharf.boathouse'),
  { id: 'boat', tex: 'obj-boat', x: t(41), y: t(41) + 8, anim: true },
  { id: 'crate1', tex: 'obj-crate', x: t(37), y: t(36) + 8, body: { w: 16, h: 8 } },
  { id: 'crate2', tex: 'obj-crate', x: t(38) + 2, y: t(36) + 8, body: { w: 16, h: 8 } },
  { id: 'barrel1', tex: 'obj-barrel', x: t(39) + 6, y: t(36) + 8, body: { w: 12, h: 8 } },
  { id: 'crate3', tex: 'obj-crate', x: t(26), y: t(36) + 8, body: { w: 16, h: 8 } },
  lamp('lamp-w1', t(18), t(32) + 8),
  lamp('lamp-w2', t(25), t(32) + 8),
  lamp('lamp-w3', t(35), t(32) + 8),
  lamp('lamp-w4', t(42), t(32) + 8),
  lamp('lamp-w5', t(56), t(33)),
  { id: 'cone1', tex: 'obj-cone', x: t(16), y: t(36) + 8 },
  { id: 'hydrant', tex: 'obj-hydrant', x: t(33), y: t(32) + 6, body: { w: 8, h: 4 } },

  // --- Legacy Plaza ----------------------------------------------------------
  {
    id: 'vault',
    tex: 'obj-vault',
    x: t(30),
    y: t(23) + 8,
    body: { w: 46, h: 16 },
    interact: 'plaza.vault',
    label: 'Examine the Vault',
    frameIf: [
      ['ch2.done', 1],
      ['vault.opened', 2],
    ],
    light: { tex: 'glow-cyan', scale: 1.6, alpha: 0.35, pulse: true, dy: -30 },
  },
  { id: 'sign-plaza', tex: 'sign-plaza', x: t(30), y: t(16) + 4, anim: true, light: { tex: 'glow-cyan', scale: 0.8, alpha: 0.3, dy: -6 } },
  { id: 'projector', tex: 'obj-projector', x: t(26), y: t(18) + 6, anim: true, body: { w: 14, h: 4 } },
  lamp('lamp-p1', t(23), t(16) + 8),
  lamp('lamp-p2', t(37), t(16) + 8),
  lamp('lamp-p3', t(23), t(27) + 4),
  lamp('lamp-p4', t(37), t(27) + 4),
  { id: 'bench-p1', tex: 'obj-bench', x: t(25), y: t(25) + 8, body: { w: 30, h: 6 }, interact: 'plaza.bench', label: 'Sit down' },
  { id: 'bench-p2', tex: 'obj-bench', x: t(35), y: t(25) + 8, body: { w: 30, h: 6 }, interact: 'plaza.bench', label: 'Sit down' },
  { id: 'statue', tex: 'obj-statue', x: t(34), y: t(19) + 4, body: { w: 18, h: 8 }, interact: 'plaza.statue', label: 'Read the plaque' },
  { id: 'trash-p', tex: 'obj-trash', x: t(24), y: t(19), body: { w: 10, h: 4 } },

  // --- The Exchange (chapter 2) ----------------------------------------------
  { id: 'exchange', tex: 'bld-exchange', x: t(9) + 8, y: t(21) + 4, anim: true, body: { w: 160, h: 34 } },
  { id: 'sign-exchange', tex: 'sign-exchange', x: t(9) + 8, y: t(19) - 2, depth: 2, anim: true },
  { id: 'dish', tex: 'obj-dish', x: t(16) + 4, y: t(19) + 8, body: { w: 20, h: 8 } },
  { id: 'antenna2', tex: 'obj-antenna', x: t(2), y: t(21), anim: true, body: { w: 8, h: 6 } },
  { id: 'server1', tex: 'obj-server', x: t(3), y: t(25) + 8, anim: true, body: { w: 16, h: 6 } },
  { id: 'server2', tex: 'obj-server', x: t(4) + 2, y: t(25) + 8, anim: true, body: { w: 16, h: 6 } },
  kiosk('kiosk-exchange', t(15), t(24) + 8, 'exchange.kiosk'),
  barrierV('gate-exchange-1', t(18) + 8, t(22), 'ch1.done'),
  barrierV('gate-exchange-2', t(18) + 8, t(23), 'ch1.done'),
  pylon('pylon-ex-1', t(18) + 8, t(21)),
  pylon('pylon-ex-2', t(18) + 8, t(24) + 2),

  // --- The Commons (chapter 3) -----------------------------------------------
  { id: 'pavilion', tex: 'bld-pavilion', x: t(53), y: t(19) + 6, anim: true, body: { w: 96, h: 26 } },
  { id: 'sign-commons', tex: 'sign-commons', x: t(53), y: t(17) + 4, depth: 2, anim: true },
  { id: 'dropA', tex: 'obj-brickdrop', x: t(44) + 4, y: t(17) + 8, body: { w: 32, h: 8 }, interact: 'commons.dropA', label: 'Check the loose brick', frameIf: [['ch3.dropA', 1]] },
  { id: 'dropB', tex: 'obj-tree', frame: 1, x: t(56), y: t(26) + 4, body: { w: 10, h: 8 }, interact: 'commons.dropB', label: 'Look into the hollow', iy: -10 },
  { id: 'mailbox', tex: 'obj-mailbox', x: t(51), y: t(25) + 8, body: { w: 10, h: 5 }, interact: 'commons.mailbox', label: 'Open the mailbox' },
  { id: 'tree-c1', tex: 'obj-tree', x: t(43) + 8, y: t(26) + 8, body: { w: 10, h: 8 } },
  { id: 'tree-c2', tex: 'obj-tree', x: t(57), y: t(20), body: { w: 10, h: 8 } },
  { id: 'tree-c3', tex: 'obj-tree', x: t(46), y: t(26) + 4, body: { w: 10, h: 8 } },
  { id: 'bush-c1', tex: 'obj-bush', x: t(51), y: t(23) + 6, body: { w: 14, h: 5 } },
  { id: 'bench-c1', tex: 'obj-bench', x: t(45) + 8, y: t(20) + 6, body: { w: 30, h: 6 }, interact: 'plaza.bench', label: 'Sit down' },
  kiosk('kiosk-commons', t(46), t(24), 'commons.kiosk'),
  lamp('lamp-c1', t(42) + 8, t(23) + 8, 'glow-green'),
  barrierV('gate-commons-1', t(41) + 8, t(22), 'ch2.done'),
  barrierV('gate-commons-2', t(41) + 8, t(23), 'ch2.done'),
  pylon('pylon-co-1', t(41) + 8, t(21)),
  pylon('pylon-co-2', t(41) + 8, t(24) + 2),

  // --- The Mint (chapter 4) ----------------------------------------------------
  { id: 'mint', tex: 'bld-mint', x: t(49) + 8, y: t(6) + 8, anim: true, body: { w: 160, h: 34 } },
  { id: 'sign-mint', tex: 'sign-mint', x: t(49) + 8, y: t(4) + 2, depth: 2, anim: true, light: { tex: 'glow-gold', scale: 0.7, alpha: 0.3, dy: -6 } },
  { id: 'coin', tex: 'obj-coin', x: t(44), y: t(4), anim: true, depth: 2 },
  { id: 'archive', tex: 'obj-archive', x: t(56), y: t(9) + 8, body: { w: 32, h: 10 }, interact: 'mint.archive', label: 'Examine the sealed door', frameIf: [['ch4.archive', 1]] },
  { id: 'server3', tex: 'obj-server', x: t(43), y: t(9) + 8, anim: true, body: { w: 16, h: 6 } },
  { id: 'server4', tex: 'obj-server', x: t(44) + 2, y: t(9) + 8, anim: true, body: { w: 16, h: 6 } },
  { id: 'vending-m', tex: 'obj-vending', x: t(53) + 8, y: t(9) + 8, anim: true, body: { w: 16, h: 6 }, interact: 'mint.vending', label: 'Use the vending machine' },
  kiosk('kiosk-mint', t(46) + 8, t(10), 'mint.kiosk'),
  barrierH('gate-mint-1', t(48) + 8, t(12), 'vault.opened'),
  barrierH('gate-mint-2', t(49) + 8, t(12), 'vault.opened'),
  pylon('pylon-mi-1', t(47) + 8, t(12)),
  pylon('pylon-mi-2', t(50) + 8, t(12)),

  // --- The Masquerade (chapter 5) ------------------------------------------------
  { id: 'masks-hall', tex: 'bld-masks', x: t(9) + 8, y: t(6) + 4, anim: true, body: { w: 160, h: 34 } },
  { id: 'sign-masks', tex: 'sign-masks', x: t(9) + 8, y: t(4) - 2, depth: 2, anim: true, light: { tex: 'glow-magenta', scale: 0.8, alpha: 0.35, dy: -6 } },
  { id: 'masks-deco', tex: 'obj-masks', x: t(4), y: t(5) - 2, depth: 2 },
  {
    id: 'checkpoint',
    tex: 'obj-checkpoint',
    x: t(9) + 8,
    y: t(8) + 10,
    anim: true,
    body: { w: 48, h: 8 },
    showIf: '!ch5.passed',
    interact: 'masks.checkpoint',
    label: 'Approach the checkpoint',
    light: { tex: 'glow-red', scale: 0.8, alpha: 0.35, dy: -16, pulse: true },
  },
  kiosk('kiosk-masks', t(15), t(9) + 8, 'masks.kiosk'),
  lamp('lamp-m1', t(3), t(9) + 8, 'glow-violet'),
  lamp('lamp-m2', t(16) + 8, t(7), 'glow-violet'),
  barrierH('gate-masks-1', t(9) + 8, t(12), 'ch4.done'),
  barrierH('gate-masks-2', t(10) + 8, t(12), 'ch4.done'),
  pylon('pylon-ma-1', t(8) + 8, t(12)),
  pylon('pylon-ma-2', t(11) + 8, t(12)),

  // --- The Observatory (chapter 6) -----------------------------------------------
  { id: 'observatory', tex: 'bld-observatory', x: t(30), y: t(7) + 8, anim: true, body: { w: 176, h: 40 } },
  { id: 'sign-observatory', tex: 'sign-observatory', x: t(30), y: t(5) + 2, depth: 2, anim: true, light: { tex: 'glow-blue', scale: 0.8, alpha: 0.35, dy: -6 } },
  { id: 'telescope', tex: 'obj-telescope', x: t(34) + 8, y: t(10), body: { w: 14, h: 6 }, interact: 'obs.telescope', label: 'Look through the telescope' },
  { id: 'whiteboard', tex: 'obj-whiteboard', x: t(25) + 8, y: t(10), body: { w: 36, h: 6 }, interact: 'obs.whiteboard', label: 'Read the whiteboard' },
  kiosk('kiosk-obs', t(28), t(10) + 4, 'obs.kiosk'),
  barrierH('gate-obs-1', t(29) + 8, t(12), 'ch5.done'),
  barrierH('gate-obs-2', t(30) + 8, t(12), 'ch5.done'),
  pylon('pylon-ob-1', t(28) + 8, t(12)),
  pylon('pylon-ob-2', t(31) + 8, t(12)),

  // --- the avenues -----------------------------------------------------------------
  lamp('lamp-a1', t(18), t(16) + 6),
  lamp('lamp-a2', t(41), t(16) + 6, 'glow-green'),
  { id: 'vending-a', tex: 'obj-vending', x: t(23) + 8, y: t(30) + 14, anim: true, body: { w: 16, h: 6 }, interact: 'street.vending', label: 'Use the vending machine' },
  { id: 'stall', tex: 'obj-stall', x: t(35), y: t(30) + 14, body: { w: 30, h: 8 }, interact: 'street.stall', label: 'Browse the stall' },
];

const npcs: NpcDef[] = [
  { id: 'mara', char: 'mara', x: t(12) + 8, y: t(35) + 4, facing: 'down', interact: 'mara.talk', label: 'Talk to Mara' },
  { id: 'felix', char: 'felix', x: t(13), y: t(23) + 8, facing: 'down', interact: 'felix.talk', label: 'Talk to Felix' },
  { id: 'juno', char: 'juno', x: t(48) + 8, y: t(20), facing: 'down', interact: 'juno.talk', label: 'Talk to Juno' },
  { id: 'ibrahim', char: 'ibrahim', x: t(48), y: t(9) + 4, facing: 'down', interact: 'ibrahim.talk', label: 'Talk to Ibrahim' },
  { id: 'lin', char: 'lin', x: t(13) + 8, y: t(9) + 8, facing: 'down', interact: 'lin.talk', label: 'Talk to Lin' },
  { id: 'agent', char: 'agent', x: t(6) + 8, y: t(9), facing: 'right', interact: 'masks.agent', label: 'Talk to the agent', showIf: '!ch5.passed' },
  { id: 'ada', char: 'ada', x: t(32), y: t(9) + 8, facing: 'down', interact: 'ada.talk', label: 'Talk to Ada' },
  { id: 'warden', char: 'warden', x: t(26), y: t(18) + 2, facing: 'down', hologram: true, interact: 'warden.holo', label: 'Watch the broadcast', showIf: 'warden.on' },
  { id: 'c1', char: 'citizen1', x: t(20), y: t(34), facing: 'down', interact: 'citizen.1', wander: { x: t(16), y: t(32), w: t(18), h: t(4) } },
  { id: 'c2', char: 'citizen2', x: t(40), y: t(33), facing: 'left', interact: 'citizen.2', wander: { x: t(36), y: t(32), w: t(16), h: t(3) } },
  { id: 'c3', char: 'citizen3', x: t(32), y: t(26), facing: 'up', interact: 'citizen.3', wander: { x: t(23), y: t(24), w: t(13), h: t(3) } },
  { id: 'c4', char: 'citizen4', x: t(50), y: t(23), facing: 'down', interact: 'citizen.4', wander: { x: t(43), y: t(21), w: t(13), h: t(3) } },
  { id: 'c5', char: 'citizen5', x: t(8), y: t(13) + 8, facing: 'right', interact: 'citizen.5', wander: { x: t(3), y: t(16) + 8, w: t(12), h: t(5) } },
];

const zones: Zone[] = [
  { id: 'wharf', x: t(1), y: t(31), w: t(58), h: t(7) },
  { id: 'plaza', x: t(22), y: t(15), w: t(16), h: t(13) },
  { id: 'exchange', x: t(1), y: t(16), w: t(17), h: t(11) },
  { id: 'commons', x: t(42), y: t(16), w: t(16), h: t(11) },
  { id: 'mint', x: t(42), y: t(1), w: t(16), h: t(10) },
  { id: 'masks', x: t(1), y: t(1), w: t(17), h: t(10) },
  { id: 'observatory', x: t(23), y: t(1), w: t(14), h: t(10) },
];

export const cityMap: MapDef = {
  id: 'city',
  title: 'Cryptopolis',
  w: W,
  h: H,
  tiles: tiles(),
  objects,
  npcs,
  zones,
  spawns: {
    start: { x: t(30), y: t(35) + 12, facing: 'down' },
    gateway: { x: t(30), y: t(35) + 12, facing: 'down' },
    vault: { x: t(30), y: t(25) + 4, facing: 'up' },
  },
  music: 'city',
  tint: 0x6e7cb0,
  ambient: 'motes',
  catMode: 'follow',
};

export const CITY_POINTS = {
  wardenAppear: { x: t(26), y: t(18) + 2 },
  vaultFront: { x: t(30), y: t(25) + 4 },
};
