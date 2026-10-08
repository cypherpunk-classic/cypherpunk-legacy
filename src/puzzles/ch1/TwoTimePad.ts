import { toHex, xor, asciiToBytes } from '../../crypto/bytes';
import { cribDrag, makeTwoTimePad, partner, plausibility, type TwoTimePad as Ttp } from '../../crypto/otp';
import { state } from '../../core/state';
import { h } from '../../ui/dom';
import { btn, card, hexBox, lede, math, note, typeValue } from '../../ui/widgets';
import type { Puzzle, PuzzleHost } from '../../ui/terminal';
import { CH1_MESSAGES } from '../../story/world';

const OK = /^[A-Z .,:;'!?\-0-9]$/;

/**
 * Chapter 1, part 1. Two radio messages were encrypted with the same pad
 * page. XOR the ciphertexts and the pad cancels: C1 ⊕ C2 = P1 ⊕ P2. Guess a
 * word in one message and the other message shows through.
 */
export class TwoTimePad implements Puzzle {
  id = 'ch1.otp';
  title = 'RADIO LOG · PAD PAGE 17';
  subtitle = 'two messages, one pad';
  accent = '#ffcd75';
  private host!: PuzzleHost;
  private t!: Ttp;
  private a: (string | null)[] = [];
  private b: (string | null)[] = [];
  private cellsA: HTMLInputElement[] = [];
  private cellsB: HTMLInputElement[] = [];
  private cribInput!: HTMLInputElement;
  private offset!: HTMLInputElement;
  private preview!: HTMLElement;
  private scan!: HTMLElement;
  private progress!: HTMLElement;

  mount(host: PuzzleHost): void {
    this.host = host;
    this.t = makeTwoTimePad(CH1_MESSAGES.p1, CH1_MESSAGES.p2, state.rng('ch1/pad'));
    const n = this.t.x.length;
    this.a = new Array(n).fill(null);
    this.b = new Array(n).fill(null);

    this.cribInput = h('input', { type: 'text', value: ' THE ', spellcheck: 'false', style: 'width:180px;text-transform:uppercase', 'data-testid': 'crib' }) as HTMLInputElement;
    this.offset = h('input', { type: 'range', min: '0', max: String(n - 1), value: '0', style: 'flex:1;min-width:160px;accent-color:var(--amber)' }) as HTMLInputElement;
    this.preview = h('div', { class: 'hex', style: 'min-height:20px' });
    this.scan = h('div', { class: 'pill-list', style: 'margin-top:8px' });
    this.progress = h('div', { class: 'note' });
    this.cribInput.addEventListener('input', () => this.updatePreview());
    this.offset.addEventListener('input', () => this.updatePreview());

    const rows = h('div', { class: 'stack', style: 'gap:10px' });
    const per = 17;
    for (let start = 0; start < n; start += per) {
      const idx = h('div', { class: 'cells' });
      const ra = h('div', { class: 'cells' });
      const rb = h('div', { class: 'cells' });
      for (let i = start; i < Math.min(n, start + per); i++) {
        idx.append(h('div', { class: 'cell idx' }, String(i)));
        const ca = this.cell(i, 'a');
        const cb = this.cell(i, 'b');
        this.cellsA[i] = ca;
        this.cellsB[i] = cb;
        ra.append(ca);
        rb.append(cb);
      }
      rows.append(h('div', null, idx, h('div', { class: 'row', style: 'gap:6px;flex-wrap:nowrap' }, h('span', { class: 'note', style: 'width:18px' }, 'A'), ra), h('div', { class: 'row', style: 'gap:6px;flex-wrap:nowrap;margin-top:2px' }, h('span', { class: 'note', style: 'width:18px' }, 'B'), rb)));
    }

    host.body.append(
      h(
        'div',
        { class: 'stack' },
        lede(
          'A one-time pad is unbreakable, once. These two messages from Grandpa\'s radio log were encrypted with the <b>same</b> pad page. XOR the two ciphertexts and the pad disappears:',
        ),
        math('<i>C</i><sub>1</sub> ⊕ <i>C</i><sub>2</sub> = (<i>P</i><sub>1</sub> ⊕ <i>K</i>) ⊕ (<i>P</i><sub>2</sub> ⊕ <i>K</i>) = <i>P</i><sub>1</sub> ⊕ <i>P</i><sub>2</sub>', true),
        h('div', { class: 'grid2' }, hexBox(toHex(this.t.c1), { label: 'C₁ (message A, intercepted)' }), hexBox(toHex(this.t.c2), { label: 'C₂ (message B, intercepted)' })),
        card(
          'Crib dragging',
          note('Guess a word that probably appears in one message (a <b>crib</b>), slide it along, and read what it implies for the other message at the same spot. Real words mean you guessed right.'),
          h('div', { class: 'row', style: 'margin-top:8px' }, this.cribInput, this.offset),
          h('div', { style: 'margin-top:6px' }, this.preview),
          h(
            'div',
            { class: 'row', style: 'margin-top:8px' },
            btn('Place crib in A here', () => this.placeCrib('a'), 'primary', 'crib-place-a'),
            btn('Place crib in B here', () => this.placeCrib('b'), '', 'crib-place-b'),
            btn('Scan all positions', () => this.runScan()),
          ),
          this.scan,
        ),
        card('Workbench', note('Type into either row. Whatever you put in A forces the letter in B at the same position, and vice versa. Amber letters are derived; red ones are not plausible text, so a guess nearby is wrong.'), h('div', { style: 'margin-top:10px;overflow-x:auto' }, rows), h('div', { style: 'margin-top:8px' }, this.progress), h('div', { class: 'row', style: 'margin-top:8px' }, btn('Clear workbench', () => this.clearAll(), 'small'))),
      ),
    );
    this.updatePreview();
    this.render();
    host.status('Mara\'s hint: who writes to whom? Names make good cribs.', 'info');
  }

  private cell(i: number, row: 'a' | 'b'): HTMLInputElement {
    const el = h('input', { class: 'cell input', maxlength: '1', 'data-i': String(i), 'data-row': row, autocomplete: 'off', spellcheck: 'false' }) as HTMLInputElement;
    el.addEventListener('input', () => {
      const v = el.value.toUpperCase().slice(-1);
      this.setChar(i, row, v || null);
      this.render();
      if (v) (row === 'a' ? this.cellsA : this.cellsB)[i + 1]?.focus();
    });
    el.addEventListener('keydown', (e) => {
      if (e.key === 'Backspace' && !el.value) (row === 'a' ? this.cellsA : this.cellsB)[i - 1]?.focus();
      if (e.key === 'ArrowRight') (row === 'a' ? this.cellsA : this.cellsB)[i + 1]?.focus();
      if (e.key === 'ArrowLeft') (row === 'a' ? this.cellsA : this.cellsB)[i - 1]?.focus();
    });
    el.addEventListener('focus', () => {
      this.offset.value = String(i);
      this.updatePreview();
    });
    return el;
  }

  private setChar(i: number, row: 'a' | 'b', ch: string | null): void {
    const mine = row === 'a' ? this.a : this.b;
    const other = row === 'a' ? this.b : this.a;
    mine[i] = ch;
    other[i] = ch ? String.fromCharCode(this.t.x[i]! ^ ch.charCodeAt(0)) : null;
  }

  private render(): void {
    const n = this.t.x.length;
    let good = 0;
    for (let i = 0; i < n; i++) {
      for (const [cells, arr] of [
        [this.cellsA, this.a],
        [this.cellsB, this.b],
      ] as const) {
        const c = cells[i]!;
        const v = arr[i];
        if (document.activeElement !== c) c.value = v ?? '';
        c.classList.toggle('odd', Boolean(v && !OK.test(v)));
      }
      const ok = this.a[i] && this.b[i] && OK.test(this.a[i]!) && OK.test(this.b[i]!);
      if (ok) good++;
    }
    this.progress.textContent = `${good} / ${n} positions read as text in both messages.`;
    const A = this.a.join('');
    const B = this.b.join('');
    if (A.length === n && B.length === n) {
      const p1 = CH1_MESSAGES.p1;
      const p2 = CH1_MESSAGES.p2;
      if ((A === p1 && B === p2) || (A === p2 && B === p1)) {
        this.cellsA.forEach((c) => (c.disabled = true));
        this.cellsB.forEach((c) => (c.disabled = true));
        this.host.solve('Both messages recovered. The pad was never needed.');
      } else this.host.status('Everything is filled in, but some of it is not quite right. Look for red letters or odd words.', 'warn');
    }
  }

  private updatePreview(): void {
    const crib = this.cribInput.value.toUpperCase();
    const n = this.t.x.length;
    this.offset.max = String(Math.max(0, n - crib.length));
    const off = Math.min(Number(this.offset.value), n - crib.length);
    if (!crib) {
      this.preview.textContent = '';
      return;
    }
    const p = partner(this.t.x, crib, off);
    const score = plausibility(p);
    this.preview.innerHTML = `offset ${off}: crib <b style="color:var(--amber)">“${crib}”</b> in one message ⇒ <b style="color:${score === 1 ? 'var(--green)' : score > 0.7 ? 'var(--amber)' : 'var(--red)'}">“${p}”</b> in the other`;
  }

  private placeCrib(row: 'a' | 'b'): void {
    const crib = this.cribInput.value.toUpperCase();
    const off = Number(this.offset.value);
    if (!crib || off + crib.length > this.t.x.length) return;
    for (let i = 0; i < crib.length; i++) this.setChar(off + i, row, crib[i]!);
    this.host.sfx('select');
    this.render();
  }

  private runScan(): void {
    const crib = this.cribInput.value.toUpperCase();
    if (!crib) return;
    const hits = cribDrag(this.t.x, crib)
      .sort((p, q) => q.score - p.score)
      .slice(0, 6);
    this.scan.replaceChildren(
      ...hits.map((hh) =>
        h(
          'span',
          {
            class: `pill ${hh.score === 1 ? 'hit' : ''}`,
            title: 'Jump to this offset',
            onclick: () => {
              this.offset.value = String(hh.offset);
              this.updatePreview();
            },
          },
          `@${hh.offset}: “${hh.partnerText}”`,
        ),
      ),
    );
  }

  private clearAll(): void {
    this.a.fill(null);
    this.b.fill(null);
    this.render();
  }

  async autoplay(): Promise<void> {
    typeValue(this.cribInput, 'NIGHTJAR');
    for (let i = 0; i < CH1_MESSAGES.p1.length; i++) this.setChar(i, 'a', CH1_MESSAGES.p1[i]!);
    this.render();
  }
}

export const xorCheck = (a: string, b: string): string => toHex(xor(asciiToBytes(a), asciiToBytes(b)));
