import { COLORS, OUTLINE } from './palette';

/**
 * Tiny raster toolkit for code-defined pixel art. Everything draws into plain
 * HTMLCanvasElements which BootScene hands to Phaser as textures, so the game
 * ships with zero image files.
 */

export type Rows = readonly string[];
export type Slots = Partial<Record<string, string>>;

export function makeCanvas(w: number, h: number): HTMLCanvasElement {
  const c = document.createElement('canvas');
  c.width = w;
  c.height = h;
  return c;
}

export function ctx2d(c: HTMLCanvasElement): CanvasRenderingContext2D {
  const ctx = c.getContext('2d', { willReadFrequently: true });
  if (!ctx) throw new Error('2d canvas unavailable');
  ctx.imageSmoothingEnabled = false;
  return ctx;
}

/** Resolve a pixel character to a colour (slot palette first, then the global palette). */
export function colorOf(ch: string, slots?: Slots): string | null {
  if (ch === '.' || ch === ' ') return null;
  return slots?.[ch] ?? COLORS[ch] ?? null;
}

export interface PaintOpts {
  slots?: Slots;
  mirror?: boolean;
  /** Characters to skip (treated as transparent). */
  skip?: string;
}

export function paint(ctx: CanvasRenderingContext2D, rows: Rows, x0: number, y0: number, opts: PaintOpts = {}): void {
  const w = Math.max(...rows.map((r) => r.length));
  rows.forEach((row, y) => {
    for (let x = 0; x < row.length; x++) {
      const ch = row[x]!;
      if (opts.skip?.includes(ch)) continue;
      const col = colorOf(ch, opts.slots);
      if (!col) continue;
      ctx.fillStyle = col;
      const px = opts.mirror ? x0 + (w - 1 - x) : x0 + x;
      ctx.fillRect(px, y0 + y, 1, 1);
    }
  });
}

export function rowsSize(rows: Rows): { w: number; h: number } {
  return { w: Math.max(...rows.map((r) => r.length)), h: rows.length };
}

/** Render rows to a new canvas, optionally with an automatic 1px outline. */
export function rowsToCanvas(rows: Rows, opts: PaintOpts & { outline?: boolean; pad?: number } = {}): HTMLCanvasElement {
  const { w, h } = rowsSize(rows);
  const pad = opts.pad ?? (opts.outline ? 1 : 0);
  const c = makeCanvas(w + 2 * pad, h + 2 * pad);
  const ctx = ctx2d(c);
  paint(ctx, rows, pad, pad, opts);
  if (opts.outline) addOutline(c);
  return c;
}

/**
 * Add a 1px outline around every opaque region (4-neighbourhood). Sprites are
 * authored as flat fills; the silhouette line is computed, never hand-drawn.
 */
export function addOutline(c: HTMLCanvasElement, color = OUTLINE, region?: { x: number; y: number; w: number; h: number }): void {
  const ctx = ctx2d(c);
  const r = region ?? { x: 0, y: 0, w: c.width, h: c.height };
  const img = ctx.getImageData(r.x, r.y, r.w, r.h);
  const a = (x: number, y: number): number => (x < 0 || y < 0 || x >= r.w || y >= r.h ? 0 : img.data[(y * r.w + x) * 4 + 3]!);
  const add: [number, number][] = [];
  for (let y = 0; y < r.h; y++)
    for (let x = 0; x < r.w; x++) {
      if (a(x, y) > 0) continue;
      if (a(x - 1, y) > 0 || a(x + 1, y) > 0 || a(x, y - 1) > 0 || a(x, y + 1) > 0) add.push([x, y]);
    }
  ctx.fillStyle = color;
  for (const [x, y] of add) ctx.fillRect(r.x + x, r.y + y, 1, 1);
}

/** Deterministic tiny PRNG for procedural textures (mulberry32). */
export function prng(seed: number): () => number {
  let t = seed >>> 0;
  return () => {
    t += 0x6d2b79f5;
    let r = Math.imul(t ^ (t >>> 15), 1 | t);
    r ^= r + Math.imul(r ^ (r >>> 7), 61 | r);
    return ((r ^ (r >>> 14)) >>> 0) / 4294967296;
  };
}

export function fill(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, color: string): void {
  ctx.fillStyle = color;
  ctx.fillRect(x, y, w, h);
}

export function px(ctx: CanvasRenderingContext2D, x: number, y: number, color: string): void {
  ctx.fillStyle = color;
  ctx.fillRect(x, y, 1, 1);
}

/** Soft radial glow texture for additive lighting. */
export function glowCanvas(radius: number, color: string, falloff = 2.2): HTMLCanvasElement {
  const size = radius * 2;
  const c = makeCanvas(size, size);
  const ctx = c.getContext('2d')!;
  const [r, g, b] = [1, 3, 5].map((i) => parseInt(color.slice(i, i + 2), 16));
  const img = ctx.createImageData(size, size);
  for (let y = 0; y < size; y++)
    for (let x = 0; x < size; x++) {
      const d = Math.hypot(x + 0.5 - radius, y + 0.5 - radius) / radius;
      const v = d >= 1 ? 0 : Math.pow(1 - d, falloff);
      const i = (y * size + x) * 4;
      img.data[i] = r!;
      img.data[i + 1] = g!;
      img.data[i + 2] = b!;
      img.data[i + 3] = Math.round(255 * v);
    }
  ctx.putImageData(img, 0, 0);
  return c;
}

/** Scale a canvas by an integer factor with nearest-neighbour sampling. */
export function scaleCanvas(src: HTMLCanvasElement, k: number): HTMLCanvasElement {
  const c = makeCanvas(src.width * k, src.height * k);
  const ctx = c.getContext('2d')!;
  ctx.imageSmoothingEnabled = false;
  ctx.drawImage(src, 0, 0, c.width, c.height);
  return c;
}
