import { input } from '../core/input';
import { h } from './dom';
import { ui } from './root';

/** On-screen joystick and action button for touch devices. */
export function mountTouchControls(): void {
  const coarse = window.matchMedia?.('(pointer: coarse)').matches || 'ontouchstart' in window;
  if (!coarse) return;
  const knob = h('div', { class: 'knob' });
  const stick = h('div', { class: 'joystick' }, knob);
  const abtn = h('button', { class: 'abtn', 'aria-label': 'Interact' }, 'A');
  ui.touch.append(stick, abtn);

  let active: number | null = null;
  const center = { x: 0, y: 0 };
  const R = 52;
  const move = (e: PointerEvent): void => {
    if (e.pointerId !== active) return;
    let dx = e.clientX - center.x;
    let dy = e.clientY - center.y;
    const d = Math.hypot(dx, dy);
    if (d > R) {
      dx = (dx / d) * R;
      dy = (dy / d) * R;
    }
    knob.style.transform = `translate(${dx}px, ${dy}px)`;
    const dead = 0.25;
    const nx = dx / R;
    const ny = dy / R;
    input.setTouchAxis(Math.abs(nx) > dead ? nx : 0, Math.abs(ny) > dead ? ny : 0);
  };
  stick.addEventListener('pointerdown', (e) => {
    active = e.pointerId;
    const r = stick.getBoundingClientRect();
    center.x = r.left + r.width / 2;
    center.y = r.top + r.height / 2;
    stick.setPointerCapture(e.pointerId);
    move(e);
  });
  stick.addEventListener('pointermove', move);
  const end = (e: PointerEvent): void => {
    if (e.pointerId !== active) return;
    active = null;
    knob.style.transform = '';
    input.setTouchAxis(0, 0);
  };
  stick.addEventListener('pointerup', end);
  stick.addEventListener('pointercancel', end);
  abtn.addEventListener('pointerdown', (e) => {
    e.preventDefault();
    input.tap('act');
  });
}
