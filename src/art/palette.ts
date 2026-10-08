/**
 * The game's palette. Pixel art in src/art is written as rows of characters;
 * each character names a colour here. Upper-case letters are *slots* that a
 * character's own palette fills in (hair, skin, clothes...), so one body
 * template serves every person in Cryptopolis.
 */

export const OUTLINE = '#0e0b16';

export const COLORS: Record<string, string> = {
  '0': OUTLINE,
  '1': '#1a1c2c',
  '2': '#2a2440',
  '3': '#333c57',
  '4': '#566c86',
  '5': '#94b0c2',
  '6': '#f4f4f4',
  '7': '#29366f',
  '8': '#3b5dc9',
  '9': '#41a6f6',
  a: '#73eff7',
  b: '#257179',
  c: '#38b764',
  d: '#a7f070',
  e: '#ffcd75',
  f: '#ef7d57',
  g: '#b13e53',
  h: '#5d275d',
  i: '#ff3e9a',
  j: '#2b1810',
  k: '#4a2c1a',
  l: '#7a4a28',
  m: '#a8703f',
  n: '#d9b38c',
  o: '#fff1d0',
  p: '#6e2a1f',
  q: '#9c4a32',
  r: '#3d4554',
  s: '#5f6b7d',
  t: '#1f2330',
  u: '#173326',
  v: '#2a5e3c',
  w: '#e6e2c3',
  x: '#ffb347',
  y: '#ff7a2f',
  z: '#fff35c',
  '#': '#0b0d16',
  '+': '#12172a',
  '=': '#3a2a5c',
  '*': '#c792ff',
  '%': '#7d4bd6',
  '&': '#1d4f58',
  '@': '#a4ffe0',
};

/** Slots used by character templates. */
export interface CharPalette {
  H: string; // hair
  G: string; // hair shade
  S: string; // skin
  T: string; // skin shade
  E: string; // eyes
  C: string; // top
  D: string; // top shade
  P: string; // trousers / skirt
  Q: string; // trousers shade
  B: string; // shoes
  A: string; // accent (scarf, glasses, badge)
  K: string; // accent 2
  V?: string; // visor glow
}

export const SKIN = {
  fair: ['#f6d2b0', '#d9a782'],
  light: ['#eebd93', '#c98f68'],
  tan: ['#c98a5d', '#a26843'],
  brown: ['#9a6440', '#74472b'],
  deep: ['#6b4129', '#4b2c1b'],
} as const;

export function hex(c: string): number {
  return parseInt(c.slice(1), 16);
}

/** UI theme colours, mirrored in styles/main.css custom properties. */
export const UI = {
  bg: '#0b0d16',
  panel: '#121827',
  cyan: '#3ef0ff',
  magenta: '#ff3e9a',
  amber: '#ffb347',
  green: '#5dff9a',
  red: '#ff5566',
  text: '#d7e3f4',
  dim: '#7f8ba3',
};
