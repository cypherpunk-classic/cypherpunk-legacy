import { h } from './dom';
import { ui } from './root';

export type ToastKind = 'info' | 'codex' | 'item' | 'warn';

export function toast(title: string, body: string, kind: ToastKind = 'info', ms = 3600): void {
  const el = h('div', { class: `toast toast-${kind}` }, h('span', { class: 't' }, title), body);
  ui.toasts.append(el);
  while (ui.toasts.children.length > 4) ui.toasts.firstElementChild?.remove();
  setTimeout(() => {
    el.classList.add('out');
    setTimeout(() => el.remove(), 320);
  }, ms);
}
