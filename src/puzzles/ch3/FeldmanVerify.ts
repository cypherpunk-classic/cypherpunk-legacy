import { feldmanCheck, reconstruct, type Share } from '../../crypto/shamir';
import { toHex } from '../../crypto/bytes';
import { xOnly, mul } from '../../crypto/secp';
import { h } from '../../ui/dom';
import { Log, btn, card, lede, math, note, shorten, tag } from '../../ui/widgets';
import type { Puzzle, PuzzleHost } from '../../ui/terminal';
import { vault } from '../../story/world';

/**
 * Chapter 3, part 2. Now over the real secp256k1 scalar field, so the secret
 * is an actual key. You have four shares: two from people, two from dead
 * drops. One dead drop was planted by the Directorate. Feldman commitments
 * C_j = a_j·G let you check each share, y·G == Σ xʲ·C_j, without knowing the
 * secret, and catch the forgery before it poisons the reconstruction.
 */
export class FeldmanVerify implements Puzzle {
  id = 'ch3.feldman';
  title = 'VAULT KEY · VERIFY SHARES';
  subtitle = 'Feldman VSS over secp256k1';
  accent = '#5dff9a';
  private host!: PuzzleHost;
  private shares: { label: string; share: Share; genuine: boolean }[] = [];
  private checked = new Set<number>();
  private chosen = new Set<number>();
  private rows!: HTMLElement;
  private log = new Log(170);
  private reconBtn!: HTMLButtonElement;

  mount(host: PuzzleHost): void {
    this.host = host;
    const v = vault();
    // genuine shares 1,2 (held) + 4 (good drop); forged 3 (planted drop)
    this.shares = [
      { label: 'Share 1 — Mara', share: v.shares[0]!, genuine: true },
      { label: 'Share 2 — Felix', share: v.shares[1]!, genuine: true },
      { label: 'Share 3 — dead drop (loose brick)', share: v.forged3, genuine: false },
      { label: 'Share 4 — dead drop (hollow oak)', share: v.shares[3]!, genuine: true },
    ];
    this.rows = h('div', { class: 'stack', style: 'gap:8px' });
    this.reconBtn = btn('Reconstruct from the 3 chosen shares', () => this.reconstruct(), 'primary', 'feldman-recon');
    this.reconBtn.disabled = true;

    host.body.append(
      h(
        'div',
        { class: 'stack' },
        lede(
          'The Vault key was split five ways over the real curve. You hold four shares; you need three good ones. But the Directorate leaves forged shares in dead drops, hoping you will rebuild the wrong key. Grandpa published <b>commitments</b> so you can tell.',
        ),
        math('dealer publishes <i>C</i><sub>j</sub> = <i>a</i><sub>j</sub>·<i>G</i>.   Share (<i>x</i>, <i>y</i>) is genuine ⟺ <i>y</i>·<i>G</i> = <i>C</i>₀ + <i>x</i>·<i>C</i>₁ + <i>x</i>²·<i>C</i>₂', true),
        card('Public commitments (from Grandpa)', h('div', { class: 'kv' }, ...v.commitments.flatMap((C, j) => [h('div', { class: 'k' }, `C${j}`), h('div', { class: 'v hex' }, shorten(toHex(xOnly(C)), 10, 8))]))),
        card('Shares', note('Verify each share, then choose three that pass. C₀ is the Vault public key itself: y·G for a good share lands on the committed curve.'), this.rows),
        card('Reconstruct', h('div', { class: 'row' }, this.reconBtn), h('div', { style: 'margin-top:8px' }, this.log.el)),
      ),
    );
    this.render();
    host.status('Verify the shares before you trust them.', 'info');
  }

  private render(): void {
    this.rows.replaceChildren(
      ...this.shares.map((s, i) => {
        const done = this.checked.has(i);
        const res = done ? feldmanCheck(s.share, vault().commitments) : null;
        const pass = res?.ok ?? false;
        const pick = h('input', { type: 'checkbox', checked: this.chosen.has(i), disabled: !done, onchange: (e: Event) => this.toggle(i, (e.target as HTMLInputElement).checked) }) as HTMLInputElement;
        return h(
          'div',
          { class: `card ${done ? (pass ? 'good' : 'bad') : ''}`, style: 'padding:8px 12px' },
          h(
            'div',
            { class: 'row', style: 'justify-content:space-between' },
            h('div', null, h('b', null, s.label), h('div', { class: 'hex', style: 'font-size:11px' }, `x = ${s.share.x} · y = ${shorten(s.share.y.toString(16).padStart(64, '0'), 10, 8)}`)),
            done ? tag(pass ? 'VALID' : 'FORGED', pass ? 'ok' : 'bad') : btn('Verify', () => this.verify(i), 'small', `feldman-verify-${i}`),
          ),
          done ? h('label', { class: 'row note', style: 'cursor:pointer;margin-top:6px' }, pick, pass ? 'use this share' : 'do not use') : null,
        );
      }),
    );
    this.reconBtn.disabled = this.chosen.size !== 3;
  }

  private verify(i: number): void {
    const s = this.shares[i]!;
    const res = feldmanCheck(s.share, vault().commitments);
    this.checked.add(i);
    this.log.line(`${s.label}: y·G ${res.ok ? '==' : '≠'} Σ xʲ·Cⱼ  →  ${res.ok ? 'genuine' : 'FORGED, discard'}`, res.ok ? 'ok' : 'bad');
    this.host.sfx(res.ok ? 'confirm' : 'error');
    this.render();
  }

  private toggle(i: number, on: boolean): void {
    if (on) {
      if (this.chosen.size >= 3) {
        this.render();
        return;
      }
      this.chosen.add(i);
    } else this.chosen.delete(i);
    this.render();
  }

  private reconstruct(): void {
    if (this.host.solved()) return;
    const picks = [...this.chosen].map((i) => this.shares[i]!);
    const forged = picks.some((p) => !p.genuine);
    const recon = reconstruct(picks.map((p) => p.share));
    const target = vault().d;
    const Vx = toHex(xOnly(mul(recon)));
    if (forged || recon !== target) {
      this.log.line(`Reconstructed key ${shorten(toHex(xOnly(mul(recon))), 8, 6)} — does NOT match the Vault public key. A bad share poisons everything.`, 'bad');
      this.host.status('That set includes a forged share. The result is a wrong key.', 'bad');
      this.host.sfx('error');
      return;
    }
    this.log.line(`Reconstructed secret · publicly: ${shorten(Vx, 10, 8)} == C₀. It matches the Vault.`, 'ok');
    this.host.sfx('unlock');
    this.host.solve('The Vault key is whole, and genuine. Three good shares, one caught forgery.');
  }

  async autoplay(): Promise<void> {
    for (let i = 0; i < this.shares.length; i++) this.verify(i);
    for (const i of [0, 1, 3]) this.toggle(i, true);
    this.reconstruct();
  }
}
