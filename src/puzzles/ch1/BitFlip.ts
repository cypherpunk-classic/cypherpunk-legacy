import { ctrDecrypt, ctrEncrypt, gcmDecryptText, gcmEncryptText, type CtrTicket, type GcmBox } from '../../crypto/aead';
import { asciiToBytes, toHex } from '../../crypto/bytes';
import { state } from '../../core/state';
import { h } from '../../ui/dom';
import { btn, card, lede, math, note } from '../../ui/widgets';
import type { Puzzle, PuzzleHost } from '../../ui/terminal';
import { gateKey } from '../../story/world';

/**
 * Chapter 1, part 2. The boathouse gate checks encrypted tickets. With AES-CTR
 * you can rewrite "role=guest" into "role=admin" without the key, by flipping
 * ciphertext bits. Then the gate is upgraded to AES-GCM and the same trick is
 * rejected outright.
 */
export class BitFlip implements Puzzle {
  id = 'ch1.ctr';
  title = 'BOATHOUSE TICKET GATE';
  subtitle = 'AES-256-CTR (no tag)';
  accent = '#73eff7';
  private host!: PuzzleHost;
  private stage: 'ctr' | 'gcm' = 'ctr';
  private plain = '';
  private ticket!: CtrTicket;
  private box!: GcmBox;
  private work!: Uint8Array; // the ciphertext being edited
  private selected = 5;
  private grid!: HTMLElement;
  private bitRow!: HTMLElement;
  private helper!: HTMLElement;
  private result!: HTMLElement;
  private wrap!: HTMLElement;
  private gcmRejected = false;

  mount(host: PuzzleHost): void {
    this.host = host;
    const handle = state.data.handle.slice(0, 16);
    this.plain = `role=guest;user=${handle};pier=09`;
    const { key, nonce } = gateKey();
    this.ticket = ctrEncrypt(key, nonce, this.plain);
    this.box = gcmEncryptText(key, nonce.slice(0, 12), this.plain);
    this.work = this.ticket.ct.slice();
    this.wrap = h('div', { class: 'stack' });
    host.body.append(this.wrap);
    this.renderStage();
  }

  private renderStage(): void {
    this.grid = h('div', { style: 'overflow-x:auto' });
    this.bitRow = h('div');
    this.helper = h('div');
    this.result = h('div');
    const intro =
      this.stage === 'ctr'
        ? lede(
            'Mara\'s lockbox is behind a Directorate ticket gate. Your ticket says <code>role=guest</code>. The gate decrypts tickets with AES in counter mode, which XORs a keystream into the bytes. You do not know the key. You do know your own ticket\'s plaintext.',
          )
        : lede(
            'Mara has swapped the gate\'s firmware for AES-GCM: the same counter-mode encryption, plus a 16-byte authentication tag computed over the ciphertext. Try your trick again.',
          );
    this.wrap.replaceChildren(
      intro,
      math(
        this.stage === 'ctr' ? '<i>C</i> = <i>P</i> ⊕ keystream   ⇒   (<i>C</i> ⊕ Δ) decrypts to (<i>P</i> ⊕ Δ)' : '<i>C</i> = <i>P</i> ⊕ keystream,   tag = GHASH<sub><i>H</i></sub>(<i>C</i>) ⊕ E<sub><i>K</i></sub>(nonce ‖ 1)',
        true,
      ),
      card('Your ticket', note('Click a byte to see and flip its bits. The top line is what you know the gate will decrypt it to.'), h('div', { style: 'margin-top:8px' }, this.grid), h('div', { style: 'margin-top:10px' }, this.bitRow)),
      h('div', { class: 'grid2' }, card('XOR helper', this.helper), card('Present the ticket', h('div', { class: 'row' }, btn('Present to gate', () => this.present(), 'primary', 'ticket-present'), btn('Reset ticket', () => this.reset())), h('div', { style: 'margin-top:10px' }, this.result))),
    );
    this.renderGrid();
    this.host.status(this.stage === 'ctr' ? 'Goal: make the gate read role=admin.' : 'Goal: see what the tag does to forgeries.', 'info');
  }

  private predicted(i: number): number {
    const orig = this.stage === 'ctr' ? this.ticket.ct : this.box.ct;
    return this.plain.charCodeAt(i) ^ orig[i]! ^ this.work[i]!;
  }

  private renderGrid(): void {
    const n = this.plain.length;
    const cells = h('div', { class: 'cells', style: 'gap:3px' });
    for (let i = 0; i < n; i++) {
      const p = this.predicted(i);
      const changed = p !== this.plain.charCodeAt(i);
      const ch = p >= 32 && p < 127 ? String.fromCharCode(p) : '·';
      cells.append(
        h(
          'div',
          {
            class: 'card',
            style: `padding:3px 4px;min-width:30px;text-align:center;cursor:pointer;${i === this.selected ? 'border-color:var(--cyan)' : ''}`,
            onclick: () => {
              this.selected = i;
              this.renderGrid();
            },
            'data-byte': String(i),
          },
          h('div', { style: `font-size:15px;color:${changed ? 'var(--amber)' : 'var(--text)'}` }, ch),
          h('div', { class: 'hex', style: 'font-size:10.5px;color:var(--dim)' }, this.work[i]!.toString(16).padStart(2, '0')),
        ),
      );
    }
    if (this.stage === 'gcm') {
      const tag = this.work.slice(n);
      cells.append(h('div', { class: 'card', style: 'padding:3px 6px;border-color:var(--violet)' }, h('div', { style: 'font-size:12px;color:var(--violet)' }, 'TAG'), h('div', { class: 'hex', style: 'font-size:10.5px' }, toHex(tag).slice(0, 12) + '…')));
    }
    this.grid.replaceChildren(cells);
    this.renderBits();
    this.renderHelper();
  }

  private renderBits(): void {
    const i = this.selected;
    const v = this.work[i]!;
    const bits = h('div', { class: 'row', style: 'gap:4px' });
    for (let b = 7; b >= 0; b--) {
      const on = (v >> b) & 1;
      bits.append(
        h(
          'button',
          {
            class: 'small',
            style: `width:30px;${on ? 'background:#10283a;color:#bff8ff' : ''}`,
            'data-bit': String(b),
            onclick: () => {
              this.work[i] = this.work[i]! ^ (1 << b);
              this.host.sfx('blip');
              this.renderGrid();
            },
          },
          String(on),
        ),
      );
    }
    const p = this.predicted(i);
    this.bitRow.replaceChildren(
      h('div', { class: 'row' }, h('span', { class: 'note' }, `byte ${i} · ciphertext bits (click to flip):`), bits, h('span', { class: 'note' }, `→ decrypts to “${String.fromCharCode(p)}” (0x${p.toString(16).padStart(2, '0')})`)),
    );
  }

  private renderHelper(): void {
    const i = this.selected;
    const want = h('input', { type: 'text', maxlength: '1', value: '', style: 'width:44px;text-align:center', placeholder: '?', 'data-testid': 'xor-want' }) as HTMLInputElement;
    const out = h('div', { class: 'note', style: 'margin-top:6px' });
    const cur = this.predicted(i);
    const update = (): void => {
      const w = want.value;
      if (!w) {
        out.textContent = 'Type the character you want at this position.';
        return;
      }
      const delta = cur ^ w.charCodeAt(0);
      out.innerHTML = `“${String.fromCharCode(cur)}” = ${cur.toString(2).padStart(8, '0')}<br>“${w}” = ${w.charCodeAt(0).toString(2).padStart(8, '0')}<br>flip mask Δ = <b style="color:var(--amber)">${delta.toString(2).padStart(8, '0')}</b>`;
    };
    want.addEventListener('input', update);
    const apply = btn('Apply Δ to this byte', () => {
      const w = want.value;
      if (!w) return;
      this.work[i] = this.work[i]! ^ (cur ^ w.charCodeAt(0));
      this.host.sfx('select');
      if (i < this.plain.length - 1) this.selected = i + 1;
      this.renderGrid();
    }, 'small', 'xor-apply');
    this.helper.replaceChildren(note(`Byte ${i} currently decrypts to “${String.fromCharCode(cur)}”.`), h('div', { class: 'row', style: 'margin-top:6px' }, want, apply), out);
    update();
  }

  private reset(): void {
    this.work = (this.stage === 'ctr' ? this.ticket.ct : this.box.ct).slice();
    this.renderGrid();
  }

  private present(): void {
    if (this.host.solved()) return;
    const { key } = gateKey();
    if (this.stage === 'ctr') {
      const text = ctrDecrypt(key, { nonce: this.ticket.nonce, ct: this.work });
      const role = /role=([^;]*)/.exec(text)?.[1] ?? '?';
      this.result.replaceChildren(h('div', { class: 'note' }, 'Gate decrypted your ticket to:'), h('div', { class: `hex big ${role === 'admin' ? 'green' : 'amber'}` }, text));
      if (role === 'admin') {
        this.host.sfx('unlock');
        this.host.status('GATE: welcome, admin. (It never knew the key was not yours.)', 'ok');
        setTimeout(() => {
          this.stage = 'gcm';
          this.work = this.box.ct.slice();
          this.selected = 5;
          this.renderStage();
          this.host.status('The gate reboots with new firmware: AES-256-GCM.', 'warn');
        }, 1600);
      } else {
        this.host.status(role === 'guest' ? 'GATE: guests may not open lockboxes.' : `GATE: unknown role “${role}”.`, 'bad');
        this.host.sfx('error');
      }
      return;
    }
    const res = gcmDecryptText(key, { nonce: this.box.nonce, ct: this.work });
    const modified = toHex(this.work) !== toHex(this.box.ct);
    if (!res.ok) {
      this.gcmRejected = true;
      this.result.replaceChildren(h('div', { class: 'hex big red' }, 'REJECTED: authentication tag mismatch'), note('The gate never decrypted your edit. It recomputed the tag over the ciphertext it received, saw it differ, and refused to say anything else.'));
      this.host.sfx('error');
      this.host.status('Forgery detected. That is what authenticated encryption is for.', 'ok');
      setTimeout(() => this.host.solve('Lesson learned: encryption hides, authentication protects. Use AEAD.'), 900);
      return;
    }
    this.result.replaceChildren(h('div', { class: 'hex big amber' }, res.text), note(modified ? 'Hm, your edit survived? That should not happen.' : 'Your unmodified ticket decrypts fine, and still says guest. Now try changing it.'));
    this.host.status(this.gcmRejected ? 'Still a guest.' : 'Valid, but you are still a guest. Edit the ticket and present it again.', 'warn');
  }

  async autoplay(): Promise<void> {
    const target = this.plain.replace('role=guest', 'role=admin');
    const a = asciiToBytes(this.plain);
    const b = asciiToBytes(target);
    for (let i = 0; i < a.length; i++) this.work[i] = this.work[i]! ^ a[i]! ^ b[i]!;
    this.present();
    await new Promise((r) => setTimeout(r, 1800));
    this.work[5] = this.work[5]! ^ 0x01;
    this.present();
    await new Promise((r) => setTimeout(r, 1100));
  }
}
