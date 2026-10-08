import { audio, type Sfx } from '../core/audio';
import { state } from '../core/state';
import { h, onKey } from './dom';
import { ui } from './root';

/**
 * Terminal windows host the puzzles. A puzzle gets a body element to draw in
 * and a small API to report status and success. Every puzzle also implements
 * `autoplay`, which drives its own controls to the solution; the end-to-end
 * test uses it to play the whole game, and it doubles as a living spec.
 */

export type Tone = 'ok' | 'bad' | 'warn' | 'info' | '';

export interface PuzzleHost {
  body: HTMLElement;
  status: (text: string, tone?: Tone) => void;
  solve: (message?: string) => void;
  close: () => void;
  sfx: (s: Sfx) => void;
  actions: HTMLElement;
  solved: () => boolean;
}

export interface Puzzle {
  id: string;
  title: string;
  subtitle?: string;
  accent?: string;
  size?: 'narrow' | 'medium' | 'wide';
  closable?: boolean;
  mount(host: PuzzleHost): void | Promise<void>;
  autoplay?(): Promise<void>;
  destroy?(): void;
}

let current: Puzzle | null = null;

export const currentPuzzle = (): Puzzle | null => current;

/** Open a puzzle; resolves true once solved and dismissed, false if closed. */
export function runPuzzle(p: Puzzle): Promise<boolean> {
  return new Promise((resolve) => {
    ui.block('terminal');
    audio.sfx('power');
    let solved = false;
    let attempts = 0;
    const statusEl = h('div', { class: 'status', 'data-testid': 'terminal-status' });
    const actions = h('div', { class: 'row' });
    const body = h('div', { class: 'wbody' });
    const closeBtn = h('button', { class: 'small ghost', title: 'Close (Esc)', 'data-testid': 'terminal-close', onclick: () => close() }, 'ESC ✕');
    const win = h(
      'div',
      { class: `window ${p.size === 'narrow' ? 'narrow' : p.size === 'medium' ? 'medium' : ''}`, style: p.accent ? `--accent:${p.accent}` : '', 'data-puzzle': p.id, role: 'dialog', 'aria-label': p.title },
      h('div', { class: 'titlebar' }, h('div', { class: 'dots' }, h('i'), h('i'), h('i')), h('div', { class: 'title' }, p.title), p.subtitle ? h('div', { class: 'subtitle' }, p.subtitle) : null, h('div', { class: 'spacer' }), p.closable === false ? null : closeBtn),
      body,
      h('div', { class: 'statusbar' }, statusEl, actions),
    );
    const backdrop = h('div', { class: 'backdrop' }, win);
    ui.modal.append(backdrop);
    current = p;

    const finish = (result: boolean): void => {
      offKey();
      try {
        p.destroy?.();
      } catch (e) {
        console.error(e);
      }
      backdrop.remove();
      current = null;
      ui.unblock('terminal');
      resolve(result);
    };
    const close = (): void => {
      if (solved) return finish(true);
      if (p.closable === false) return;
      audio.sfx('blip');
      finish(false);
    };
    const host: PuzzleHost = {
      body,
      actions,
      status(text, tone = '') {
        statusEl.className = `status ${tone}`;
        statusEl.textContent = text;
        if (tone === 'bad') attempts++;
      },
      solve(message = 'Solved.') {
        if (solved) return;
        solved = true;
        state.markSolved(p.id, attempts + 1);
        audio.sfx('success');
        host.status(message, 'ok');
        win.classList.add('solved');
        const cont = h('button', { class: 'primary', 'data-testid': 'terminal-continue', onclick: () => finish(true) }, 'Continue ▸');
        actions.replaceChildren(cont);
        closeBtn.textContent = 'DONE ✓';
        setTimeout(() => cont.focus(), 50);
      },
      close,
      sfx: (s) => audio.sfx(s),
      solved: () => solved,
    };
    const offKey = onKey((e) => {
      if (e.code === 'Escape') {
        e.preventDefault();
        e.stopPropagation();
        close();
      }
    });
    try {
      void Promise.resolve(p.mount(host)).catch((err) => {
        console.error(err);
        host.status(`Error: ${String(err)}`, 'bad');
      });
    } catch (err) {
      console.error(err);
      host.status(`Error: ${String(err)}`, 'bad');
    }
  });
}

/** A simple modal window that is not a puzzle (menus, codex, readers). */
export function openWindow(opts: { title: string; subtitle?: string; size?: 'narrow' | 'medium' | 'wide'; accent?: string; id?: string }, build: (body: HTMLElement, close: () => void) => void): Promise<void> {
  return new Promise((resolve) => {
    const id = opts.id ?? 'window';
    ui.block(id);
    const body = h('div', { class: 'wbody' });
    const close = (): void => {
      offKey();
      backdrop.remove();
      ui.unblock(id);
      resolve();
    };
    const win = h(
      'div',
      { class: `window ${opts.size === 'narrow' ? 'narrow' : opts.size === 'medium' ? 'medium' : ''}`, style: opts.accent ? `--accent:${opts.accent}` : '', role: 'dialog', 'aria-label': opts.title, 'data-window': id },
      h('div', { class: 'titlebar' }, h('div', { class: 'dots' }, h('i'), h('i'), h('i')), h('div', { class: 'title' }, opts.title), opts.subtitle ? h('div', { class: 'subtitle' }, opts.subtitle) : null, h('div', { class: 'spacer' }), h('button', { class: 'small ghost', onclick: close, 'data-testid': 'window-close' }, 'ESC ✕')),
      body,
    );
    const backdrop = h('div', { class: 'backdrop' }, win);
    backdrop.addEventListener('mousedown', (e) => {
      if (e.target === backdrop) close();
    });
    ui.modal.append(backdrop);
    const offKey = onKey((e) => {
      if (e.code === 'Escape') {
        e.preventDefault();
        e.stopPropagation();
        close();
      }
    });
    build(body, close);
  });
}
