import { copyText, h } from './dom';

/** Building blocks shared by the puzzle terminals. */

type Kids = (Node | string | null | undefined | false)[];

export function card(title: string | null, ...kids: Kids): HTMLElement {
  return h('div', { class: 'card' }, title ? h('h3', null, title) : null, ...kids);
}

export function sub(title: string, ...kids: Kids): HTMLElement {
  return h('div', null, h('h4', { style: 'margin:0 0 6px;font-family:var(--retro);font-weight:normal;font-size:17px;color:var(--amber)' }, title), ...kids);
}

export function lede(html: string): HTMLElement {
  return h('p', { class: 'lede', html });
}

export function note(html: string): HTMLElement {
  return h('div', { class: 'note', html });
}

export function btn(label: string, onclick: () => void, cls = '', testid?: string): HTMLButtonElement {
  return h('button', { class: cls, onclick, 'data-testid': testid }, label);
}

export function math(html: string, block = false): HTMLElement {
  return h(block ? 'div' : 'span', { class: `math${block ? ' block' : ''}`, html });
}

/** Hex value in a box with a copy button. */
export function hexBox(value: string, opts: { tone?: string; big?: boolean; label?: string; testid?: string } = {}): HTMLElement {
  const v = h('div', { class: `hex ${opts.tone ?? ''} ${opts.big ? 'big' : ''}`, 'data-testid': opts.testid }, value);
  const copy = h('button', {
    class: 'copy',
    title: 'Copy',
    onclick: async () => {
      const ok = await copyText(v.textContent ?? '');
      copy.textContent = ok ? '✓' : '✗';
      setTimeout(() => (copy.textContent = 'copy'), 900);
    },
  }, 'copy');
  return h('div', null, opts.label ? h('div', { class: 'note', style: 'margin-bottom:3px' }, opts.label) : null, h('div', { class: 'hexbox' }, v, copy));
}

export function setHex(box: HTMLElement, value: string): void {
  const v = box.querySelector('.hex');
  if (v) v.textContent = value;
}

export function kv(rows: [string, Node | string][]): HTMLElement {
  return h('div', { class: 'kv' }, ...rows.flatMap(([k, v]) => [h('div', { class: 'k' }, k), h('div', { class: 'v' }, v)]));
}

export function tag(text: string, tone: '' | 'ok' | 'bad' | 'info' | 'warn' = ''): HTMLElement {
  return h('span', { class: `tag ${tone}` }, text);
}

export interface BitsOpts {
  group?: number;
  marks?: (i: number) => string;
}

export function bitsView(bits: string, opts: BitsOpts = {}): HTMLElement {
  const g = opts.group ?? 8;
  return h(
    'div',
    { class: 'bits' },
    ...Array.from(bits).map((b, i) => h('span', { class: `${b === '1' ? 'one' : ''} ${(i + 1) % g === 0 ? 'gap' : ''} ${opts.marks?.(i) ?? ''}` }, b)),
  );
}

export class Log {
  el: HTMLElement;
  constructor(maxHeight?: number) {
    this.el = h('div', { class: 'log', style: maxHeight ? `max-height:${maxHeight}px` : '' });
  }
  line(text: string, tone: '' | 'ok' | 'bad' | 'info' | 'warn' | 'dim' = ''): void {
    this.el.append(h('div', { class: tone }, text));
    this.el.scrollTop = this.el.scrollHeight;
  }
  clear(): void {
    this.el.replaceChildren();
  }
}

export function meter(): { el: HTMLElement; set: (frac: number) => void } {
  const bar = h('i');
  const el = h('div', { class: 'meter' }, bar);
  return { el, set: (f) => (bar.style.width = `${Math.max(0, Math.min(1, f)) * 100}%`) };
}

export function shorten(hex: string, head = 10, tail = 8): string {
  return hex.length <= head + tail + 1 ? hex : `${hex.slice(0, head)}…${hex.slice(-tail)}`;
}

/** Set an input's value the way a user would, firing input events. */
export function typeValue(input: HTMLInputElement | HTMLTextAreaElement, value: string): void {
  input.value = value;
  input.dispatchEvent(new Event('input', { bubbles: true }));
  input.dispatchEvent(new Event('change', { bubbles: true }));
}

export const pause = (ms: number): Promise<void> => new Promise((r) => setTimeout(r, ms));

/** HiDPI-aware canvas for plots drawn by puzzles. */
export function plotCanvas(cssW: number, cssH: number): { canvas: HTMLCanvasElement; ctx: CanvasRenderingContext2D; w: number; h: number } {
  const dpr = Math.min(2, window.devicePixelRatio || 1);
  const canvas = h('canvas', { class: 'plot' }) as HTMLCanvasElement;
  canvas.width = Math.round(cssW * dpr);
  canvas.height = Math.round(cssH * dpr);
  canvas.style.aspectRatio = `${cssW} / ${cssH}`;
  const ctx = canvas.getContext('2d')!;
  ctx.scale(dpr, dpr);
  return { canvas, ctx, w: cssW, h: cssH };
}
