import { audio } from '../core/audio';
import { state } from '../core/state';
import { speaker, type SpeakerId } from '../story/speakers';
import { h, onKey, rich } from './dom';
import { ui } from './root';

/**
 * Dialogue box with a typewriter effect. Lines may use the light markup from
 * `rich()`. Space / Enter / E / click completes the line, then advances.
 */

export type Line = [SpeakerId, string];

/** Test hook: when `auto` is on, lines advance by themselves and choices come from the queue (default 0). */
export const debugDialog = { auto: false, choices: [] as number[], kick: null as null | (() => void) };

interface Typing {
  skip: () => void;
  done: Promise<void>;
  finished: () => boolean;
}

function typeInto(el: HTMLElement, html: string, cps: number): Typing {
  el.innerHTML = html;
  const chars: HTMLElement[] = [];
  const walk = (node: Node): void => {
    if (node.nodeType === Node.TEXT_NODE) {
      const text = node.textContent ?? '';
      const frag = document.createDocumentFragment();
      for (const ch of text) {
        const s = document.createElement('span');
        s.textContent = ch;
        s.style.visibility = 'hidden';
        chars.push(s);
        frag.append(s);
      }
      node.parentNode?.replaceChild(frag, node);
    } else node.childNodes.forEach((c) => walk(c));
  };
  Array.from(el.childNodes).forEach(walk);
  let i = 0;
  let finished = false;
  let resolve!: () => void;
  const done = new Promise<void>((r) => (resolve = r));
  const finish = (): void => {
    if (finished) return;
    finished = true;
    chars.forEach((c) => (c.style.visibility = 'visible'));
    resolve();
  };
  if (cps <= 0 || chars.length === 0) finish();
  else {
    const interval = 1000 / cps;
    let last = performance.now();
    const tick = (now: number): void => {
      if (finished) return;
      let n = Math.floor((now - last) / interval);
      if (n > 0) {
        last += n * interval;
        while (n-- > 0 && i < chars.length) {
          chars[i]!.style.visibility = 'visible';
          if (i % 3 === 0 && chars[i]!.textContent !== ' ') audio.sfx('type');
          i++;
        }
      }
      if (i >= chars.length) finish();
      else requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
  }
  return { skip: finish, done, finished: () => finished };
}

function buildBox(who: SpeakerId): { box: HTMLElement; text: HTMLElement; more: HTMLElement; body: HTMLElement } {
  const sp = speaker(who);
  const text = h('div', { class: 'text' });
  const more = h('div', { class: 'more' }, '▼');
  const body = h('div', { class: 'body' }, sp.name() ? h('div', { class: 'name', style: { color: sp.color } }, sp.name()) : null, text);
  const box = h(
    'div',
    { class: `dialogue ${sp.style ?? ''}`, role: 'dialog', 'aria-live': 'polite', 'data-speaker': who },
    sp.portrait ? h('img', { class: 'portrait', src: sp.portrait, alt: '' }) : null,
    body,
    more,
  );
  return { box, text, more, body };
}

let queue: Promise<unknown> = Promise.resolve();

/** Show lines one after another. Resolves once the last one is dismissed. */
export function say(...lines: Line[]): Promise<void> {
  const run = async (): Promise<void> => {
    ui.block('dialogue');
    try {
      for (const [who, txt] of lines) await showLine(who, fill(txt));
    } finally {
      ui.unblock('dialogue');
    }
  };
  const p = queue.then(run);
  queue = p.catch(() => undefined);
  return p;
}

/** Replace {{handle}} with the player's chosen name. */
function fill(txt: string): string {
  return txt.replace(/\{\{handle\}\}/g, state.data.handle || 'anon');
}

function showLine(who: SpeakerId, txt: string): Promise<void> {
  return new Promise((resolve) => {
    const { box, text, more } = buildBox(who);
    more.style.visibility = 'hidden';
    ui.dialogue.append(box);
    const typing = typeInto(text, rich(txt), debugDialog.auto ? 0 : state.settings.textSpeed);
    void typing.done.then(() => (more.style.visibility = 'visible'));
    if (debugDialog.auto) setTimeout(() => advance(), 30);
    debugDialog.kick = () => {
      typing.skip();
      advance();
    };
    const advance = (): void => {
      if (!typing.finished()) {
        typing.skip();
        return;
      }
      cleanup();
      box.remove();
      audio.sfx('blip');
      debugDialog.kick = null;
      resolve();
    };
    const off = onKey((e) => {
      if (['Space', 'Enter', 'KeyE', 'NumpadEnter'].includes(e.code)) {
        e.preventDefault();
        e.stopPropagation();
        if (!e.repeat) advance();
      }
    });
    const click = (): void => advance();
    box.addEventListener('click', click);
    const cleanup = (): void => {
      off();
      box.removeEventListener('click', click);
    };
  });
}

/** Ask a question; resolves with the index of the chosen option. */
export function choose(who: SpeakerId, question: string, options: string[]): Promise<number> {
  const run = (): Promise<number> =>
    new Promise((resolve) => {
      ui.block('dialogue');
      const { box, text, more, body } = buildBox(who);
      more.remove();
      text.innerHTML = rich(fill(question));
      let sel = 0;
      const buttons = options.map((o, i) =>
        h('button', { class: i === 0 ? 'sel' : '', onclick: (e: Event) => (e.stopPropagation(), pick(i)) }, h('span', { class: 'k' }, `${i + 1}`), fill(o)),
      );
      body.append(h('div', { class: 'choices' }, buttons));
      ui.dialogue.append(box);
      const mark = (): void => buttons.forEach((b, i) => b.classList.toggle('sel', i === sel));
      const pick = (i: number): void => {
        debugDialog.kick = null;
        off();
        box.remove();
        audio.sfx('select');
        ui.unblock('dialogue');
        resolve(i);
      };
      if (debugDialog.auto) setTimeout(() => pick(Math.min(options.length - 1, debugDialog.choices.shift() ?? 0)), 30);
      debugDialog.kick = () => pick(Math.min(options.length - 1, debugDialog.choices.shift() ?? 0));
      const off = onKey((e) => {
        const n = parseInt(e.key, 10);
        if (n >= 1 && n <= options.length) {
          e.preventDefault();
          pick(n - 1);
        } else if (e.code === 'ArrowDown' || e.code === 'KeyS') {
          e.preventDefault();
          sel = (sel + 1) % options.length;
          mark();
          audio.sfx('blip');
        } else if (e.code === 'ArrowUp' || e.code === 'KeyW') {
          e.preventDefault();
          sel = (sel + options.length - 1) % options.length;
          mark();
          audio.sfx('blip');
        } else if (['Space', 'Enter', 'KeyE'].includes(e.code)) {
          e.preventDefault();
          e.stopPropagation();
          if (!e.repeat) pick(sel);
        }
      });
    });
  const p = queue.then(run);
  queue = p.catch(() => undefined);
  return p;
}
