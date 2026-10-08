import { audio } from './audio';

/**
 * One input layer for keyboard and touch. The world scenes poll it; UI panels
 * listen to DOM events directly. Phaser's own keyboard plugin is not used, so
 * it can never swallow keystrokes meant for text fields in puzzles.
 */

export type Action = 'up' | 'down' | 'left' | 'right' | 'act' | 'menu' | 'run';

const KEYMAP: Record<string, Action> = {
  ArrowUp: 'up',
  KeyW: 'up',
  ArrowDown: 'down',
  KeyS: 'down',
  ArrowLeft: 'left',
  KeyA: 'left',
  ArrowRight: 'right',
  KeyD: 'right',
  KeyE: 'act',
  Space: 'act',
  Enter: 'act',
  Escape: 'menu',
  ShiftLeft: 'run',
  ShiftRight: 'run',
};

function isTyping(el: EventTarget | null): boolean {
  if (!(el instanceof HTMLElement)) return false;
  return el.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(el.tagName);
}

class InputManager {
  private held = new Set<Action>();
  private pressed = new Set<Action>();
  private touchAxis = { x: 0, y: 0 };
  /** False while dialogue, terminals or menus own the screen. */
  worldEnabled = true;
  private cooldownUntil = 0;

  constructor() {
    window.addEventListener('keydown', (e) => {
      audio.unlock();
      if (isTyping(e.target)) return;
      const a = KEYMAP[e.code];
      if (!a) return;
      if (['up', 'down', 'left', 'right'].includes(a) || e.code === 'Space') e.preventDefault();
      if (!e.repeat) this.pressed.add(a);
      this.held.add(a);
    });
    window.addEventListener('keyup', (e) => {
      const a = KEYMAP[e.code];
      if (a) this.held.delete(a);
    });
    window.addEventListener('blur', () => this.held.clear());
    window.addEventListener('pointerdown', () => audio.unlock(), { capture: true });
  }

  /** Movement vector in [-1, 1]^2. */
  axis(): { x: number; y: number } {
    if (!this.worldEnabled) return { x: 0, y: 0 };
    let x = this.touchAxis.x;
    let y = this.touchAxis.y;
    if (this.held.has('left')) x -= 1;
    if (this.held.has('right')) x += 1;
    if (this.held.has('up')) y -= 1;
    if (this.held.has('down')) y += 1;
    return { x: Math.max(-1, Math.min(1, x)), y: Math.max(-1, Math.min(1, y)) };
  }

  running(): boolean {
    return this.held.has('run');
  }

  /** True once per press of `a` while the world has focus. */
  consume(a: Action): boolean {
    const was = this.pressed.has(a);
    this.pressed.delete(a);
    if (!this.worldEnabled || performance.now() < this.cooldownUntil) return false;
    return was;
  }

  /** Forget stale presses (e.g. the key that closed a dialogue). */
  flush(cooldownMs = 120): void {
    this.pressed.clear();
    this.cooldownUntil = performance.now() + cooldownMs;
  }

  setTouchAxis(x: number, y: number): void {
    this.touchAxis = { x, y };
  }

  tap(a: Action): void {
    this.pressed.add(a);
  }

  setWorldEnabled(on: boolean): void {
    this.worldEnabled = on;
    if (!on) this.touchAxis = { x: 0, y: 0 };
    this.flush();
  }
}

export const input = new InputManager();
