import { audio } from '../core/audio';
import { state } from '../core/state';
import { storageAvailable } from '../core/save';
import { CODEX, CODEX_GROUPS, codexById } from '../story/codex';
import { heldItems } from '../story/items';
import { copyText, h, rich } from './dom';
import { openWindow } from './terminal';
import { toast } from './toast';

/** Pause menu: Codex, Keyring, Settings, Game. */

export type MenuTab = 'codex' | 'keyring' | 'settings' | 'game';

export interface MenuActions {
  quitToTitle: () => void;
}

export function openMenu(tab: MenuTab, actions: MenuActions, focusCodex?: string): Promise<void> {
  return openWindow({ title: 'MENU', subtitle: `${state.data.handle} · ${formatTime(state.data.playMs)}`, size: 'medium', id: 'menu' }, (body, close) => {
    body.style.padding = '0';
    const content = h('div', { style: 'padding:14px 16px 18px' });
    const tabs: [MenuTab, string][] = [
      ['codex', 'CODEX'],
      ['keyring', 'KEYRING'],
      ['settings', 'SETTINGS'],
      ['game', 'GAME'],
    ];
    const bar = h('div', { class: 'tabs' });
    const show = (t: MenuTab): void => {
      bar.querySelectorAll('button').forEach((b) => b.classList.toggle('on', b.dataset.tab === t));
      content.replaceChildren();
      if (t === 'codex') content.append(codexView(focusCodex));
      if (t === 'keyring') content.append(keyringView());
      if (t === 'settings') content.append(settingsView());
      if (t === 'game') content.append(gameView(actions, close));
      audio.sfx('blip');
    };
    for (const [t, label] of tabs) bar.append(h('button', { 'data-tab': t, onclick: () => show(t) }, label));
    body.append(bar, content);
    show(tab);
  });
}

function formatTime(ms: number): string {
  const m = Math.floor(ms / 60000);
  return m < 60 ? `${m} min` : `${Math.floor(m / 60)} h ${m % 60} min`;
}

export function codexView(focus?: string): HTMLElement {
  const list = h('div', { class: 'list' });
  const entry = h('div', { class: 'entry' });
  const unlocked = new Set(state.data.codex);
  const select = (id: string): void => {
    const e = codexById(id);
    if (!e) return;
    list.querySelectorAll('button').forEach((b) => b.classList.toggle('on', b.dataset.id === id));
    entry.replaceChildren(
      h('h2', null, e.title),
      h('div', { class: 'meta' }, `${e.group.toUpperCase()} · ${e.era}`),
      ...e.body.split(/\n\s*\n/).map((p) => h('p', { html: rich(p.trim()) })),
      ...(e.ingame ? [h('div', { class: 'ingame', html: rich(e.ingame) })] : []),
    );
  };
  for (const g of CODEX_GROUPS) {
    list.append(h('div', { class: 'grp' }, g.toUpperCase()));
    for (const e of CODEX.filter((x) => x.group === g)) {
      const open = unlocked.has(e.id);
      list.append(
        h(
          'button',
          { class: open ? '' : 'locked', 'data-id': e.id, disabled: !open, onclick: () => open && (select(e.id), audio.sfx('blip')) },
          open ? e.title : '· · · · · ·',
        ),
      );
    }
  }
  const first = focus && unlocked.has(focus) ? focus : CODEX.find((e) => unlocked.has(e.id))?.id;
  if (first) select(first);
  else entry.append(h('p', { class: 'note' }, 'Nothing here yet. Explore, read, and solve: entries unlock as you go.'));
  const count = h('div', { class: 'note', style: 'padding:6px 14px 0' }, `${unlocked.size} / ${CODEX.length} entries`);
  return h('div', null, h('div', { class: 'codex' }, list, entry), count);
}

function keyringView(): HTMLElement {
  const items = heldItems();
  if (!items.length) return h('p', { class: 'note' }, 'Your keyring is empty.');
  return h(
    'div',
    { class: 'keyring' },
    ...items.map((it) =>
      h(
        'div',
        { class: 'card' },
        h('h3', null, it.name),
        h('div', { class: 'note', style: 'margin-bottom:8px' }, it.desc),
        it.details ? h('div', { class: 'kv' }, ...it.details().flatMap(([k, v]) => [h('div', { class: 'k' }, k), h('div', { class: 'v hex' }, v)])) : null,
      ),
    ),
  );
}

function settingsView(): HTMLElement {
  const s = state.settings;
  const range = (value: number, on: (v: number) => void): HTMLInputElement => {
    const el = h('input', { type: 'range', min: '0', max: '100', value: String(Math.round(value * 100)) }) as HTMLInputElement;
    el.addEventListener('input', () => on(Number(el.value) / 100));
    return el;
  };
  const speed = h('select', null, ...[
    ['30', 'Slow'],
    ['60', 'Normal'],
    ['120', 'Fast'],
    ['0', 'Instant'],
  ].map(([v, l]) => h('option', { value: v, selected: Number(v) === s.textSpeed }, l))) as HTMLSelectElement;
  speed.addEventListener('change', () => state.updateSettings({ textSpeed: Number(speed.value) }));
  const check = (on: boolean, fn: (v: boolean) => void): HTMLInputElement => {
    const el = h('input', { type: 'checkbox', checked: on }) as HTMLInputElement;
    el.addEventListener('change', () => fn(el.checked));
    return el;
  };
  return h(
    'div',
    { class: 'settings' },
    h(
      'div',
      { class: 'rowset' },
      h('label', null, 'Music volume'),
      range(s.music, (v) => state.updateSettings({ music: v })),
      h('label', null, 'Effects volume'),
      range(s.sfx, (v) => {
        state.updateSettings({ sfx: v });
        audio.sfx('blip');
      }),
      h('label', null, 'Text speed'),
      speed,
      h('label', null, 'CRT scanlines'),
      check(s.crt, (v) => state.updateSettings({ crt: v })),
      h('label', null, 'Reduce motion'),
      check(s.reducedMotion, (v) => state.updateSettings({ reducedMotion: v })),
    ),
    h('p', { class: 'note', style: 'margin-top:18px' }, 'Controls: WASD or arrow keys to move, Shift to walk faster, E / Space / Enter to interact, C for the codex, Esc for this menu.'),
  );
}

function gameView(actions: MenuActions, close: () => void): HTMLElement {
  const io = h('textarea', { rows: '6', style: 'width:100%;font-size:11px', placeholder: 'Paste a save here to import it.' }) as HTMLTextAreaElement;
  return h(
    'div',
    { class: 'stack' },
    h(
      'div',
      { class: 'row' },
      h('button', { class: 'primary', onclick: () => (state.save(), toast('SAVED', 'Progress written to this browser.')) }, 'Save now'),
      h('button', { onclick: async () => toast('EXPORT', (await copyText(state.exportSave())) ? 'Save copied to clipboard.' : 'Copy failed; use the box below.') }, 'Copy save to clipboard'),
      h('button', { onclick: () => (io.value = state.exportSave()) }, 'Show save'),
      h(
        'button',
        {
          onclick: () => {
            if (state.importSave(io.value)) {
              toast('IMPORTED', 'Save loaded. Returning to title.');
              close();
              actions.quitToTitle();
            } else toast('IMPORT', 'That does not look like a save file.', 'warn');
          },
        },
        'Import from box',
      ),
    ),
    io,
    storageAvailable() ? null : h('p', { class: 'note', style: 'color:var(--amber)' }, 'This browser is not allowing storage here, so progress lasts only until the tab closes. Copy your save to keep it.'),
    h(
      'div',
      { class: 'row' },
      h('button', { class: 'danger', onclick: () => (state.save(), close(), actions.quitToTitle()) }, 'Save and quit to title'),
    ),
    h('p', { class: 'note' }, 'Your save contains a random 32-byte seed. Every key, pad and share in your playthrough is derived from it, so no two players see the same puzzles.'),
  );
}
