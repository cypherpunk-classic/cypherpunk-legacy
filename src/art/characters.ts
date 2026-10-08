import { addOutline, ctx2d, makeCanvas, paint, type Rows } from './draw';
import { SKIN, type CharPalette } from './palette';

/**
 * Characters are composed from parts: head + torso + legs per facing, then a
 * hairstyle and accessories are painted on top, then the silhouette gets an
 * automatic outline. Frames are 16x24; feet touch the bottom row.
 *
 * Slot letters: H/G hair, S/T skin, E eyes, C/D top, P/Q trousers, B shoes,
 * A/K accents.
 */

export const FRAME_W = 16;
export const FRAME_H = 24;

const HEAD_FRONT: Rows = [
  '................',
  '.....SSSSSS.....',
  '...SSSSSSSSSS...',
  '..SSSSSSSSSSSS..',
  '..SSSSSSSSSSSS..',
  '..SSSSSSSSSSSS..',
  '..SSSSSSSSSSSS..',
  '..SSSESSSSESSS..',
  '..SSSESSSSESSS..',
  '..TSSSSSSSSSST..',
  '...TSSSSSSSST...',
  '....TTTTTTTT....',
];

const HEAD_BACK: Rows = HEAD_FRONT.map((r) => r.replace(/E/g, 'S'));

const HEAD_SIDE: Rows = [
  '................',
  '.....SSSSSS.....',
  '...SSSSSSSSSS...',
  '..SSSSSSSSSSSS..',
  '..SSSSSSSSSSSS..',
  '..SSSSSSSSSSSS..',
  '..SSSSSSSSSSSS..',
  '..SSESSSSSSSSS..',
  '.SSSESSSSSSSSS..',
  '..SSSSSSSSSSST..',
  '...TSSSSSSSST...',
  '....TTTTTTTT....',
];

const BODY_FRONT: Rows = [
  '.....CCCCCC.....',
  '....CCCCCCCC....',
  '...SCCCCCCCCS...',
  '...SCCCCCCCCS...',
  '...SDCCCCCCDS...',
  '....DDDDDDDD....',
];

const BODY_BACK: Rows = [
  '.....CCCCCC.....',
  '....CCCCCCCC....',
  '...SCCCCCCCCS...',
  '...SCCCCCCCCS...',
  '...SDDCCCCDDS...',
  '....DDDDDDDD....',
];

const BODY_SIDE: Rows = [
  '.....CCCCCC.....',
  '.....CCCCCCC....',
  '.....CSCCCCD....',
  '.....CSCCCCD....',
  '.....DSDCCCD....',
  '.....DDDDDDD....',
];

const LEGS_FRONT: Rows[] = [
  ['.....PPPPPP.....', '.....PPQQPP.....', '.....PP..PP.....', '.....PP..PP.....', '.....BB..BB.....', '................'],
  ['.....PPPPPP.....', '.....PPQQPP.....', '.....PP..PP.....', '.....PP..BB.....', '.....BB.........', '................'],
  ['.....PPPPPP.....', '.....PPQQPP.....', '.....PP..PP.....', '.....BB..PP.....', '.........BB.....', '................'],
];

const LEGS_SIDE: Rows[] = [
  ['......PPPPP.....', '......PPPPP.....', '......PP.PP.....', '......PP.PP.....', '.....BBB.BB.....', '................'],
  ['......PPPPP.....', '.....PPP.PPP....', '....PPP...PP....', '....PP.....PP...', '...BBB.....BB...', '................'],
  ['......PPPPP.....', '......PPPP......', '.......PPP......', '.......PP.......', '......BBB.......', '................'],
];

export type Facing = 'down' | 'up' | 'left' | 'right';
export const FACINGS: Facing[] = ['down', 'up', 'left', 'right'];

/** Overlays are 24-row stencils; '.' leaves the pixel below untouched. */
type Overlay = Partial<Record<'down' | 'up' | 'left', Rows>>;

export const HAIR: Record<string, Overlay> = {
  short: {
    down: ['....HHHHHHHH....', '...HHHHHHHHHH...', '..HHHHHHHHHHHH..', '..HHHHHHHHHHHH..', '..HHGHHHHHHGHH..', '..HG.GGHHGG.GH..', '..H..........H..'],
    up: ['....HHHHHHHH....', '...HHHHHHHHHH...', '..HHHHHHHHHHHH..', '..HHHHHHHHHHHH..', '..HHHHHHHHHHHH..', '..HHHHHHHHHHHH..', '..HHHHHHHHHHHH..', '..GHHHHHHHHHHG..', '..GGHHHHHHHHGG..', '...GGGGGGGGGG...'],
    left: ['.....HHHHHH.....', '...HHHHHHHHHH...', '..HHHHHHHHHHHH..', '..HHHHHHHHHHHH..', '..GHHHHHHHHHHH..', '..G...GHHHHHHH..', '......GHHHHHHH..', '.......THHHHHH..', '.......THHHHHG..', '........GHHHG...', '.........GGG....'],
  },
  long: {
    down: ['....HHHHHHHH....', '...HHHHHHHHHH...', '..HHHHHHHHHHHH..', '.HHHHHHHHHHHHHH.', '.HHHGHHHHHHGHHH.', '.HHG.GGHHGG.GHH.', '.HH..........HH.', '.HH..........HH.', '.HH..........HH.', '.HG..........GH.', '.GG..........GG.', '.G............G.'],
    up: ['....HHHHHHHH....', '...HHHHHHHHHH...', '..HHHHHHHHHHHH..', '.HHHHHHHHHHHHHH.', '.HHHHHHHHHHHHHH.', '.HHHHHHHHHHHHHH.', '.HHHHHHHHHHHHHH.', '.HHHHHHHHHHHHHH.', '.HHHHHHHHHHHHHH.', '.GHHHHHHHHHHHHG.', '.GGHHHHHHHHHHGG.', '.GGGGGGGGGGGGGG.', '...GGGGGGGGGG...'],
    left: ['.....HHHHHH.....', '...HHHHHHHHHH...', '..HHHHHHHHHHHH..', '.HHHHHHHHHHHHHH.', '.GHHHHHHHHHHHHH.', '..G...GHHHHHHHH.', '......GHHHHHHHH.', '.......THHHHHHH.', '.......THHHHHHH.', '........HHHHHHH.', '........GHHHHHG.', '........GGHHHGG.', '.........GGGGG..', '..........GGG...'],
  },
  bob: {
    down: ['....HHHHHHHH....', '...HHHHHHHHHH...', '..HHHHHHHHHHHH..', '.HHHHHHHHHHHHHH.', '.HHHHHHHHHHHHHH.', '.HHGGGGGGGGGGHH.', '.HH..........HH.', '.HH..........HH.', '.HG..........GH.', '.GG..........GG.'],
    up: ['....HHHHHHHH....', '...HHHHHHHHHH...', '..HHHHHHHHHHHH..', '.HHHHHHHHHHHHHH.', '.HHHHHHHHHHHHHH.', '.HHHHHHHHHHHHHH.', '.HHHHHHHHHHHHHH.', '.HHHHHHHHHHHHHH.', '.GHHHHHHHHHHHHG.', '.GGGGGGGGGGGGGG.'],
    left: ['.....HHHHHH.....', '...HHHHHHHHHH...', '..HHHHHHHHHHHH..', '.HHHHHHHHHHHHHH.', '.GHHHHHHHHHHHHH.', '.GG...GHHHHHHHH.', '......GHHHHHHHH.', '.......THHHHHHH.', '.......GHHHHHHG.', '........GGGGGG..'],
  },
  bald: {
    down: ['................', '................', '................', '..HH........HH..', '..HH........HH..', '..H..........H..'],
    up: ['................', '................', '................', '..HH........HH..', '..HHHHHHHHHHHH..', '..HHHHHHHHHHHH..', '..GHHHHHHHHHHG..', '...GGGGGGGGGG...'],
    left: ['................', '................', '................', '..........HHH...', '.........HHHH...', '.......THHHHH...', '.......TGHHH....', '.........GG.....'],
  },
  spiky: {
    down: ['...H.HH.HH.H....', '...HHHHHHHHHH...', '..HHHHHHHHHHHH..', '.HHHHHHHHHHHHHH.', '..HHHGHHHHGHHH..', '..HGG.GHHG.GGH..', '..H..........H..'],
    up: ['...H.HH.HH.H....', '...HHHHHHHHHH...', '..HHHHHHHHHHHH..', '.HHHHHHHHHHHHHH.', '..HHHHHHHHHHHH..', '..HHHHHHHHHHHH..', '..HHHHHHHHHHHH..', '..GHHHHHHHHHHG..', '..GGHHHHHHHHGG..', '...GGGGGGGGGG...'],
    left: ['....H.HH.HH.H...', '...HHHHHHHHHHH..', '..HHHHHHHHHHHHH.', '..HHHHHHHHHHHHH.', '..GHHHHHHHHHHH..', '..G...GHHHHHHH..', '......GHHHHHHH..', '.......THHHHHH..', '.......THHHHHG..', '........GHHHG...'],
  },
  bun: {
    down: ['......HHHH......', '....HHGGGGHH....', '...HHHHHHHHHH...', '..HHHHHHHHHHHH..', '..HHGHHHHHHGHH..', '..HG.GGHHGG.GH..', '..H..........H..'],
    up: ['......HHHH......', '.....HHGGHH.....', '...HHHHHHHHHH...', '..HHHHHHHHHHHH..', '..HHHHHHHHHHHH..', '..HHHHHHHHHHHH..', '..HHHHHHHHHHHH..', '..GHHHHHHHHHHG..', '..GGHHHHHHHHGG..', '...GGGGGGGGGG...'],
    left: ['.........HHHH...', '.....HHHHGGGHH..', '...HHHHHHHHHHH..', '..HHHHHHHHHHHH..', '..GHHHHHHHHHHH..', '..G...GHHHHHHH..', '......GHHHHHHH..', '.......THHHHHH..', '.......THHHHHG..', '........GHHHG...'],
  },
  hood: {
    down: ['....CCCCCCCC....', '...CCCCCCCCCC...', '..CCCCCCCCCCCC..', '.CCCHHHHHHHHCCC.', '.CCHHGHHHHGHHCC.', '.CCG.GGHHGG.GCC.', '.CC..........CC.', '.CC..........CC.', '.CD..........DC.', '.DD..........DD.', '..D..........D..'],
    up: ['....CCCCCCCC....', '...CCCCCCCCCC...', '..CCCCCCCCCCCC..', '.CCCCCCCCCCCCCC.', '.CCCCCCCCCCCCCC.', '.CCCCCCCCCCCCCC.', '.CCCCCCCCCCCCCC.', '.CCCCCCCCCCCCCC.', '.DCCCCCCCCCCCCD.', '.DDCCCCCCCCCCDD.', '..DDDDDDDDDDDD..'],
    left: ['.....CCCCCC.....', '...CCCCCCCCCCC..', '..CCCCCCCCCCCCC.', '.CHHHHCCCCCCCCC.', '.CGHHHHCCCCCCCC.', '.CG...GCCCCCCCC.', '.C.....CCCCCCCC.', '.D......CCCCCCC.', '........DCCCCCC.', '.........DCCCCD.', '.........DDCCDD.', '..........DDDD..'],
  },
  cap: {
    down: ['....AAAAAAAA....', '...AAAAAAAAAA...', '..AAAAAAAAAAAA..', '..AAAAKKAAAAAA..', '.KKKKKKKKKKKKKK.', '..HG........GH..', '..H..........H..'],
    up: ['....AAAAAAAA....', '...AAAAAAAAAA...', '..AAAAAAAAAAAA..', '..AAAAAAAAAAAA..', '..AAAAAAAAAAAA..', '..HHHHHHHHHHHH..', '..HHHHHHHHHHHH..', '..GHHHHHHHHHHG..', '...GGGGGGGGGG...'],
    left: ['.....AAAAAA.....', '...AAAAAAAAAA...', '..AAAAAAAAAAAA..', '..AAAAAAAAAAAA..', 'KKKKKKAAAAAAAA..', '......GHHHHHHH..', '.......THHHHHH..', '.......THHHHHG..', '........GHHHG...'],
  },
};

export const ACCESSORIES: Record<string, Overlay> = {
  glasses: {
    down: ['', '', '', '', '', '', '', '...AAAAAAAAAA...', '...A.AA..A.AA...'].map((r) => r.padEnd(16, '.')),
    left: ['', '', '', '', '', '', '', '..AAAA..........', '..A.AA..........'].map((r) => r.padEnd(16, '.')),
  },
  beard: {
    down: ['', '', '', '', '', '', '', '', '', '..G..........G..', '..GH..HHHH..HG..', '...GHHHHHHHHG...', '....GHHHHHHG....', '......GGGG......'].map((r) => r.padEnd(16, '.')),
    left: ['', '', '', '', '', '', '', '', '', '........G.......', '..HHHH.HG.......', '..GHHHHHG.......', '...GHHHG........', '....GGG.........'].map((r) => r.padEnd(16, '.')),
  },
  scarf: {
    down: ['', '', '', '', '', '', '', '', '', '', '', '...AAAAAAAAAA...', '....AAAAAAAA....', '.......KK.......', '.......KK.......'].map((r) => r.padEnd(16, '.')),
    up: ['', '', '', '', '', '', '', '', '', '', '', '...AAAAAAAAAA...', '....AAAAAAAA....'].map((r) => r.padEnd(16, '.')),
    left: ['', '', '', '', '', '', '', '', '', '', '', '...AAAAAAAAA....', '....AAAAAAAA....', '....KK..........', '....KK..........'].map((r) => r.padEnd(16, '.')),
  },
  headset: {
    down: ['', '', '', '', '', '', '.A..........A...', '.A..........A...', '.AKK............', '...K............'].map((r) => r.padEnd(16, '.')),
    up: ['', '', '', '', '', '', '.A............A.', '.A............A.'].map((r) => r.padEnd(16, '.')),
    left: ['', '', '', '', '', '', '........A.......', '........A.......', '..KKKKKKA.......'].map((r) => r.padEnd(16, '.')),
  },
  badge: {
    down: ['', '', '', '', '', '', '', '', '', '', '', '', '', '', '.........AK.....'].map((r) => r.padEnd(16, '.')),
  },
  coat: {
    down: ['', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '....CCCCCCCC....', '....CDD..DDC....'].map((r) => r.padEnd(16, '.')),
    up: ['', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '....CCCCCCCC....', '....CDDDDDDC....'].map((r) => r.padEnd(16, '.')),
    left: ['', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '.....CCCCCCC....', '.....CDD.DDC....'].map((r) => r.padEnd(16, '.')),
  },
  visor: {
    down: ['', '', '', '', '', '', '', '..1111111111111.', '..11VV1111VV11..'].map((r) => r.padEnd(16, '.')),
    left: ['', '', '', '', '', '', '', '.1111111........', '.1VV1111........'].map((r) => r.padEnd(16, '.')),
  },
};

export interface CharDef {
  id: string;
  name: string;
  palette: CharPalette;
  hair: keyof typeof HAIR;
  accessories?: (keyof typeof ACCESSORIES)[];
}

function pal(skin: readonly [string, string], hair: [string, string], top: [string, string], pants: [string, string], shoes: string, accent: [string, string] = ['#73eff7', '#257179'], eyes = '#1a1c2c'): CharPalette {
  return { S: skin[0], T: skin[1], H: hair[0], G: hair[1], C: top[0], D: top[1], P: pants[0], Q: pants[1], B: shoes, E: eyes, A: accent[0], K: accent[1], V: '#ff5566' };
}

export const PLAYER_LOOKS: CharDef[] = [
  { id: 'player-a', name: 'Hoodie', hair: 'hood', palette: pal(SKIN.light, ['#3b2a20', '#22160f'], ['#2a9d8f', '#1d6d64'], ['#2a2440', '#1a1c2c'], '#e6e2c3') },
  { id: 'player-b', name: 'Bob', hair: 'bob', palette: pal(SKIN.tan, ['#1a1c2c', '#0e0b16'], ['#ef7d57', '#b8552f'], ['#29366f', '#1a1c2c'], '#f4f4f4') },
  { id: 'player-c', name: 'Spikes', hair: 'spiky', palette: pal(SKIN.brown, ['#ff3e9a', '#b13e53'], ['#3b5dc9', '#29366f'], ['#333c57', '#1a1c2c'], '#ffcd75') },
];

export const CAST: CharDef[] = [
  { id: 'grandpa', name: 'Grandpa', hair: 'bald', accessories: ['beard', 'glasses'], palette: pal(SKIN.fair, ['#e6e2c3', '#94b0c2'], ['#8b5a2b', '#5c3a21'], ['#566c86', '#333c57'], '#4a2c1a', ['#c7c7c7', '#94b0c2']) },
  { id: 'mara', name: 'Mara', hair: 'long', accessories: ['coat'], palette: pal(SKIN.tan, ['#c9ced6', '#8a93a3'], ['#ffcd75', '#d49a3c'], ['#29366f', '#1a1c2c'], '#2b1810') },
  { id: 'felix', name: 'Felix', hair: 'bald', accessories: ['headset', 'beard'], palette: pal(SKIN.fair, ['#a8703f', '#7a4a28'], ['#ef7d57', '#b8552f'], ['#333c57', '#1a1c2c'], '#1a1c2c', ['#3d4554', '#73eff7']) },
  { id: 'juno', name: 'Juno', hair: 'bob', accessories: ['scarf'], palette: pal(SKIN.deep, ['#1a1c2c', '#0e0b16'], ['#38b764', '#257179'], ['#3d4554', '#1f2330'], '#e6e2c3', ['#ffcd75', '#ef7d57']) },
  { id: 'ibrahim', name: 'Ibrahim', hair: 'short', accessories: ['beard'], palette: pal(SKIN.brown, ['#1a1c2c', '#0e0b16'], ['#b13e53', '#7a2638'], ['#3d4554', '#1f2330'], '#2b1810') },
  { id: 'lin', name: 'Lin', hair: 'long', accessories: ['glasses'], palette: pal(SKIN.light, ['#1a1c2c', '#0e0b16'], ['#7d4bd6', '#5d275d'], ['#1a1c2c', '#0e0b16'], '#c792ff', ['#c792ff', '#7d4bd6']) },
  { id: 'ada', name: 'Ada', hair: 'bun', accessories: ['glasses', 'coat'], palette: pal(SKIN.fair, ['#f4f4f4', '#94b0c2'], ['#e6e2c3', '#94b0c2'], ['#29366f', '#1a1c2c'], '#1a1c2c', ['#41a6f6', '#29366f']) },
  { id: 'warden', name: 'Warden', hair: 'cap', accessories: ['visor', 'badge'], palette: pal(SKIN.fair, ['#1a1c2c', '#0e0b16'], ['#333c57', '#1f2330'], ['#1f2330', '#0e0b16'], '#0e0b16', ['#262a36', '#1f2330']) },
  { id: 'agent', name: 'DLA Agent', hair: 'cap', accessories: ['visor'], palette: pal(SKIN.tan, ['#1a1c2c', '#0e0b16'], ['#3d4554', '#1f2330'], ['#1f2330', '#0e0b16'], '#0e0b16', ['#3d4554', '#1f2330']) },
  { id: 'citizen1', name: 'Citizen', hair: 'short', palette: pal(SKIN.deep, ['#2b1810', '#1a1c2c'], ['#41a6f6', '#3b5dc9'], ['#e6e2c3', '#94b0c2'], '#1a1c2c') },
  { id: 'citizen2', name: 'Citizen', hair: 'long', palette: pal(SKIN.fair, ['#ffcd75', '#d49a3c'], ['#ff3e9a', '#b13e53'], ['#1a1c2c', '#0e0b16'], '#f4f4f4') },
  { id: 'citizen3', name: 'Citizen', hair: 'spiky', accessories: ['glasses'], palette: pal(SKIN.tan, ['#38b764', '#257179'], ['#1a1c2c', '#0e0b16'], ['#566c86', '#333c57'], '#ef7d57') },
  { id: 'citizen4', name: 'Citizen', hair: 'bun', palette: pal(SKIN.brown, ['#5d275d', '#2a2440'], ['#a7f070', '#38b764'], ['#29366f', '#1a1c2c'], '#f4f4f4') },
  { id: 'citizen5', name: 'Citizen', hair: 'hood', palette: pal(SKIN.light, ['#7a4a28', '#4a2c1a'], ['#566c86', '#333c57'], ['#2a2440', '#1a1c2c'], '#ffcd75') },
];

function frameRows(facing: 'down' | 'up' | 'left', frame: number): Rows {
  const head = facing === 'down' ? HEAD_FRONT : facing === 'up' ? HEAD_BACK : HEAD_SIDE;
  const body = facing === 'down' ? BODY_FRONT : facing === 'up' ? BODY_BACK : BODY_SIDE;
  const legs = facing === 'left' ? LEGS_SIDE[frame]! : LEGS_FRONT[frame]!;
  return [...head, ...body, ...legs];
}

/** Frame order in a character sheet: facing-major, 3 frames each (idle, stepA, stepB). */
export function frameIndex(facing: Facing, frame: number): number {
  return FACINGS.indexOf(facing) * 3 + frame;
}

export function drawCharacterSheet(def: CharDef): HTMLCanvasElement {
  const sheet = makeCanvas(FRAME_W * 12, FRAME_H);
  const ctx = ctx2d(sheet);
  FACINGS.forEach((facing, fi) => {
    const base: 'down' | 'up' | 'left' = facing === 'right' ? 'left' : facing;
    const mirror = facing === 'right';
    for (let f = 0; f < 3; f++) {
      const x0 = (fi * 3 + f) * FRAME_W;
      // body bob: step frames sit one pixel lower
      const bob = f === 0 ? 0 : 0;
      const tmp = makeCanvas(FRAME_W, FRAME_H);
      const t = ctx2d(tmp);
      const slots = def.palette as unknown as Record<string, string>;
      paint(t, frameRows(base, f), 0, bob, { slots, mirror: false });
      const hair = HAIR[def.hair]?.[base];
      if (hair) paint(t, hair, 0, bob, { slots });
      for (const acc of def.accessories ?? []) {
        const ov = ACCESSORIES[acc]?.[base];
        if (ov) paint(t, ov, 0, bob, { slots });
      }
      addOutline(tmp);
      if (mirror) {
        ctx.save();
        ctx.translate(x0 + FRAME_W, 0);
        ctx.scale(-1, 1);
        ctx.drawImage(tmp, 0, 0);
        ctx.restore();
      } else ctx.drawImage(tmp, x0, 0);
    }
  });
  return sheet;
}

/** A 4x portrait cropped from the front-facing idle frame. */
export function drawPortrait(def: CharDef, scale = 4): HTMLCanvasElement {
  const sheet = drawCharacterSheet(def);
  const crop = { x: 0, y: 0, w: 16, h: 15 };
  const c = makeCanvas(crop.w * scale, crop.h * scale);
  const ctx = c.getContext('2d')!;
  ctx.imageSmoothingEnabled = false;
  ctx.drawImage(sheet, crop.x, crop.y, crop.w, crop.h, 0, 0, c.width, c.height);
  return c;
}

// ---------------------------------------------------------------------------
// Nonce the cat (16x16): sit, walk A, walk B for down/up/left.
// ---------------------------------------------------------------------------

const CAT_PAL = { H: '#3d4554', G: '#1f2330', S: '#f4f4f4', E: '#a7f070', A: '#ff3e9a', K: '#ffcd75' };

const CAT_SIDE: Rows[] = [
  [
    '................',
    '................',
    '................',
    '..H...H.........',
    '..HH.HH.........',
    '..HHHHH.........',
    '..HEHEH.........',
    '..HHSHH.........',
    '...AAAHHHHHH..H.',
    '...HHHHHHHHHH.H.',
    '...HHHHHHHHHHHH.',
    '...HHHHHHHHHH...',
    '...HHGHHHHGHH...',
    '...HH.H..H.HH...',
    '................',
    '................',
  ],
  [
    '................',
    '................',
    '................',
    '..H...H.........',
    '..HH.HH.........',
    '..HHHHH.........',
    '..HEHEH.........',
    '..HHSHH.........',
    '...AAAHHHHHH...H',
    '...HHHHHHHHHH.H.',
    '...HHHHHHHHHHHH.',
    '...HHHHHHHHHH...',
    '..HHG.HHHH.GHH..',
    '..H.......H.....',
    '................',
    '................',
  ],
  [
    '................',
    '................',
    '................',
    '..H...H.........',
    '..HH.HH.........',
    '..HHHHH.........',
    '..HEHEH.........',
    '..HHSHH.........',
    '...AAAHHHHHH..H.',
    '...HHHHHHHHHHH..',
    '...HHHHHHHHHH...',
    '...HHHHHHHHHH...',
    '....HGHHHHGH....',
    '....H.H..H.H....',
    '................',
    '................',
  ],
];

const CAT_DOWN: Rows[] = [
  [
    '................',
    '................',
    '....H....H......',
    '....HH..HH......',
    '....HHHHHH......',
    '....HEHHEH......',
    '....HHSSHH......',
    '.....AAAA.......',
    '....HHHHHH......',
    '...HHHHHHHH.....',
    '...HHHSSHHH.....',
    '...HHHSSHHH.H...',
    '...HHHHHHHHHH...',
    '....HH..HH......',
    '................',
    '................',
  ],
];

const CAT_UP: Rows[] = [
  [
    '................',
    '................',
    '....H....H......',
    '....HH..HH......',
    '....HHHHHH......',
    '....HHHHHH......',
    '....HHHHHH......',
    '.....AAAA.......',
    '....HHHHHH......',
    '...HHHHHHHH.....',
    '...HHHHHHHH.....',
    '...HHHHHHHH.....',
    '...HHHHHHHH.....',
    '....HH..HH......',
    '.......HH.......',
    '........H.......',
  ],
];

/** Cat sheet: [down0, up0, left0, left1, left2, right0, right1, right2] */
export function drawCatSheet(): HTMLCanvasElement {
  const frames: { rows: Rows; mirror: boolean }[] = [
    { rows: CAT_DOWN[0]!, mirror: false },
    { rows: CAT_UP[0]!, mirror: false },
    ...CAT_SIDE.map((r) => ({ rows: r, mirror: false })),
    ...CAT_SIDE.map((r) => ({ rows: r, mirror: true })),
  ];
  const sheet = makeCanvas(16 * frames.length, 16);
  const ctx = ctx2d(sheet);
  frames.forEach((fr, i) => {
    const tmp = makeCanvas(16, 16);
    paint(ctx2d(tmp), fr.rows, 0, 0, { slots: CAT_PAL });
    addOutline(tmp);
    if (fr.mirror) {
      ctx.save();
      ctx.translate(i * 16 + 16, 0);
      ctx.scale(-1, 1);
      ctx.drawImage(tmp, 0, 0);
      ctx.restore();
    } else ctx.drawImage(tmp, i * 16, 0);
  });
  return sheet;
}
