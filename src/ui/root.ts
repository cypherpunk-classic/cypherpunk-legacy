import { input } from '../core/input';
import { h } from './dom';

/**
 * The DOM overlay that sits on top of the Phaser canvas. Anything with text
 * lives here: crisp at every zoom level, selectable, accessible, testable.
 *
 * Layers (bottom to top): hud, prompt, dialogue, modal, toast, fade.
 * While any blocking element is open the world stops reading input.
 */

class UiRoot {
  root!: HTMLElement;
  hud!: HTMLElement;
  dialogue!: HTMLElement;
  modal!: HTMLElement;
  toasts!: HTMLElement;
  fade!: HTMLElement;
  touch!: HTMLElement;
  screen!: HTMLElement;
  private blockers = new Set<string>();

  mount(root: HTMLElement): void {
    this.root = root;
    this.screen = h('div', { class: 'layer layer-screen' });
    this.hud = h('div', { class: 'layer layer-hud hidden' });
    this.dialogue = h('div', { class: 'layer layer-dialogue' });
    this.modal = h('div', { class: 'layer layer-modal' });
    this.touch = h('div', { class: 'layer layer-touch' });
    this.toasts = h('div', { class: 'layer layer-toasts', 'aria-live': 'polite' });
    this.fade = h('div', { class: 'layer layer-fade' });
    root.append(this.screen, this.hud, this.touch, this.dialogue, this.modal, this.toasts, this.fade);
  }

  block(id: string): void {
    this.blockers.add(id);
    input.setWorldEnabled(false);
    document.body.classList.toggle('ui-blocked', true);
  }

  unblock(id: string): void {
    this.blockers.delete(id);
    if (this.blockers.size === 0) {
      input.setWorldEnabled(true);
      document.body.classList.toggle('ui-blocked', false);
    }
  }

  get blocked(): boolean {
    return this.blockers.size > 0;
  }

  isOpen(id: string): boolean {
    return this.blockers.has(id);
  }

  showHud(on: boolean): void {
    this.hud.classList.toggle('hidden', !on);
    this.touch.classList.toggle('hidden', !on);
  }

  /** Fade the whole screen to a colour and back. */
  async fadeOut(ms = 400, color = '#000'): Promise<void> {
    this.fade.style.background = color;
    this.fade.style.transition = `opacity ${ms}ms ease`;
    this.fade.classList.add('on');
    await new Promise((r) => setTimeout(r, ms));
  }

  async fadeIn(ms = 400): Promise<void> {
    this.fade.style.transition = `opacity ${ms}ms ease`;
    this.fade.classList.remove('on');
    await new Promise((r) => setTimeout(r, ms));
  }
}

export const ui = new UiRoot();
