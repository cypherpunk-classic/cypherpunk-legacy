import { state } from '../core/state';
import { input } from '../core/input';
import { h } from './dom';
import { ui } from './root';

/**
 * Heads-up display: current objective, chapter progress pips, menu buttons,
 * the big district title card and a contextual key hint.
 */

export const CHAPTER_LABELS = ['P', '1', '2', '3', '4', '5', '6'];
export const CHAPTER_NAMES = [
  'Prologue: The Cabin',
  'Chapter 1: One Time Only',
  'Chapter 2: Strangers Agree',
  'Chapter 3: Threshold',
  'Chapter 4: Signatures',
  'Chapter 5: Knowledge',
  'Chapter 6: After the Quantum',
];

class Hud {
  private objective!: HTMLElement;
  private objText!: HTMLElement;
  private pips!: HTMLElement;
  private location!: HTMLElement;
  private hint!: HTMLElement;
  private locTimer = 0;
  onMenu: () => void = () => {};
  onCodex: () => void = () => {};

  mount(): void {
    this.objText = h('div', { class: 'text' });
    this.objective = h('div', { class: 'hud-objective', 'data-testid': 'objective' }, h('div', { class: 'label' }, 'OBJECTIVE'), this.objText);
    this.pips = h('div', { class: 'hud-pips', title: 'Chapters' });
    const codexBtn = h('button', { class: 'hud-btn', title: 'Codex (C)', onclick: () => this.onCodex() }, 'CODEX');
    const menuBtn = h('button', { class: 'hud-btn', title: 'Menu (Esc)', 'data-testid': 'menu-button', onclick: () => this.onMenu() }, 'MENU');
    this.location = h('div', { class: 'hud-location' }, h('div', { class: 'name' }), h('div', { class: 'sub' }));
    this.hint = h('div', { class: 'hud-hint hidden' });
    ui.hud.append(this.objective, h('div', { class: 'hud-right' }, this.pips, codexBtn, menuBtn), this.location, this.hint);
    state.on('objective', ({ text }) => this.setObjective(text, true));
    state.on('chapter', () => this.renderPips());
    state.on('flag', () => this.renderPips());
    window.addEventListener('keydown', (e) => {
      if (!input.worldEnabled) return;
      if (e.code === 'KeyC' && !e.metaKey && !e.ctrlKey) this.onCodex();
      if (e.code === 'Escape') this.onMenu();
    });
  }

  refresh(): void {
    this.setObjective(state.data.objective, false);
    this.renderPips();
  }

  setObjective(text: string, flash: boolean): void {
    this.objText.textContent = text || '—';
    this.objective.classList.toggle('hidden', !text);
    if (flash) {
      this.objective.classList.remove('flash');
      void this.objective.offsetWidth;
      this.objective.classList.add('flash');
    }
  }

  renderPips(): void {
    const done = (i: number): boolean => (i === 0 ? state.flag('pro.done') : state.flag(`ch${i}.done`));
    this.pips.replaceChildren(
      ...CHAPTER_LABELS.map((l, i) => {
        const cls = done(i) ? 'pip done' : state.data.chapter === i ? 'pip current' : 'pip';
        return h('div', { class: cls, title: CHAPTER_NAMES[i] }, l);
      }),
    );
  }

  showLocation(name: string, sub: string): void {
    const [n, s] = Array.from(this.location.children) as HTMLElement[];
    n!.textContent = name;
    s!.textContent = sub;
    this.location.classList.add('show');
    window.clearTimeout(this.locTimer);
    this.locTimer = window.setTimeout(() => this.location.classList.remove('show'), 2600);
  }

  setHint(html: string | null): void {
    if (!html) {
      this.hint.classList.add('hidden');
      return;
    }
    this.hint.innerHTML = html;
    this.hint.classList.remove('hidden');
  }
}

export const hud = new Hud();
