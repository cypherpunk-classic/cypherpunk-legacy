import { audio } from '../core/audio';
import { state } from '../core/state';
import { PLAYER_LOOKS, drawCharacterSheet } from '../art/characters';
import { makeCanvas } from '../art/draw';
import { canvasToDataUrl, h } from './dom';
import { codexView } from './menu';
import { openWindow } from './terminal';
import { ui } from './root';

export interface TitleActions {
  onContinue: () => void;
  onNewGame: (handle: string, look: number) => void;
  onSettings: () => void;
}

export function showTitle(actions: TitleActions): void {
  ui.showHud(false);
  ui.screen.replaceChildren();
  const save = state.peekSave();
  const menu = h('div', { class: 'title-menu' });
  if (save) {
    menu.append(
      h(
        'button',
        { class: 'primary', 'data-testid': 'continue', onclick: () => (audio.sfx('confirm'), hide(), actions.onContinue()) },
        'CONTINUE',
      ),
    );
  }
  menu.append(
    h('button', { class: save ? '' : 'primary', 'data-testid': 'new-game', onclick: () => (audio.sfx('select'), newGame(actions, Boolean(save))) }, 'NEW GAME'),
    h('button', { onclick: () => (audio.sfx('select'), actions.onSettings()) }, 'SETTINGS'),
    h('button', { onclick: () => (audio.sfx('select'), about()) }, 'ABOUT'),
  );
  const screen = h(
    'div',
    { class: 'title-screen', 'data-testid': 'title' },
    h('div', { class: 'logo' }, 'CYPHERPUNK', h('small', null, 'LEGACY')),
    h(
      'p',
      { class: 'tagline' },
      'Your grandfather left you a cabin, a cat, and twelve words. Behind them waits a city he helped build, and an argument about privacy that never ended. Every key, signature and attack in this game is real cryptography, computed in your browser.',
    ),
    menu,
    save ? h('div', { class: 'note', style: 'margin-top:10px' }, `Saved game: ${save.handle} · chapter ${save.chapter === 0 ? 'P' : save.chapter}`) : null,
    h('div', { class: 'title-foot' }, 'A game about the crypto wars · MIT licensed · ', h('span', null, 'v2.0')),
  );
  ui.screen.append(screen);
  const hide = (): void => screen.remove();
}

function lookPreview(i: number): string {
  const sheet = drawCharacterSheet(PLAYER_LOOKS[i]!);
  const c = makeCanvas(16 * 4, 24 * 4);
  const g = c.getContext('2d')!;
  g.imageSmoothingEnabled = false;
  g.drawImage(sheet, 0, 0, 16, 24, 0, 0, 64, 96);
  return canvasToDataUrl(c);
}

function newGame(actions: TitleActions, overwrite: boolean): void {
  void openWindow({ title: 'NEW GAME', subtitle: overwrite ? 'This replaces your current save.' : 'Who are you?', size: 'narrow', id: 'newgame' }, (body, close) => {
    let look = 0;
    const input = h('input', { type: 'text', maxlength: '16', value: '', placeholder: 'anon', autocomplete: 'off', spellcheck: 'false', 'data-testid': 'handle', style: 'width:100%;font-size:18px' }) as HTMLInputElement;
    const looks = h('div', { class: 'looks' });
    const renderLooks = (): void => {
      looks.replaceChildren(
        ...PLAYER_LOOKS.map((l, i) =>
          h('button', { class: i === look ? 'on' : '', onclick: () => ((look = i), renderLooks(), audio.sfx('blip')) }, h('img', { src: lookPreview(i), alt: l.name }), l.name),
        ),
      );
    };
    renderLooks();
    const err = h('div', { class: 'note', style: 'color:var(--red);min-height:18px' });
    const begin = (): void => {
      const handle = (input.value.trim() || 'anon').toLowerCase();
      if (!/^[a-z0-9_.-]{1,16}$/.test(handle)) {
        err.textContent = 'Letters, digits, dot, dash and underscore only (max 16).';
        audio.sfx('error');
        return;
      }
      audio.sfx('confirm');
      close();
      ui.screen.replaceChildren();
      actions.onNewGame(handle, look);
    };
    input.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') begin();
    });
    body.append(
      h(
        'div',
        { class: 'stack' },
        h('p', { class: 'lede' }, 'Pick a handle. In 1993 everyone on the list had one; your grandfather was "nightjar".'),
        input,
        err,
        h('div', { class: 'note' }, 'Look'),
        looks,
        h('div', { class: 'row', style: 'justify-content:flex-end' }, h('button', { class: 'primary', 'data-testid': 'begin', onclick: begin }, 'Begin ▸')),
      ),
    );
    setTimeout(() => input.focus(), 30);
  });
}

function about(): void {
  void openWindow({ title: 'ABOUT', size: 'medium', id: 'about' }, (body) => {
    body.append(
      h(
        'div',
        { class: 'stack' },
        h(
          'p',
          { class: 'lede' },
          'Cypherpunk Legacy is a short adventure about cryptography and the people who fought to make it public. It runs entirely in your browser: no server, no tracking, no accounts.',
        ),
        h(
          'p',
          { class: 'lede' },
          'All cryptography is real. Seeds follow BIP-39 and BIP-32, signatures follow BIP-340, key aggregation BIP-327, addresses BIP-341. Secret sharing, zero-knowledge proofs, lattice reduction (integral LLL), LWE and the NIST post-quantum standards are implemented or called from audited libraries (noble and scure by Paul Miller), and verified against official test vectors.',
        ),
        h('p', { class: 'note' }, 'Characters and organisations in Cryptopolis are fictional. The history in the Codex is not. Entries you have unlocked:'),
        codexView(),
      ),
    );
  });
}
