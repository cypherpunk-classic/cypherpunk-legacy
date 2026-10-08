import { challenge, recoverFromNonceReuse, sign, sigHex, verify, publicKey, evenKey, type Sig } from '../../crypto/bip340';
import { bigIntToBytes, toHex, utf8 } from '../../crypto/bytes';

import { h } from '../../ui/dom';
import { btn, card, hexBox, kv, lede, math, note, shorten } from '../../ui/widgets';
import type { Puzzle, PuzzleHost } from '../../ui/terminal';
import { warden } from '../../story/world';

/**
 * Chapter 4, part 1. Warden's signing device reused a nonce across two orders.
 * Two signatures with the same R: s1 = k + e1·d, s2 = k + e2·d, so
 * d = (s1 − s2)/(e1 − e2). Recover the key, then sign a new order of your own.
 */
export class NonceReuse implements Puzzle {
  id = 'ch4.nonce';
  title = "WARDEN'S SIGNING LOG";
  subtitle = 'BIP-340 · two orders, one nonce';
  accent = '#ff5566';
  private host!: PuzzleHost;
  private recovered: bigint | null = null;
  private solveBox!: HTMLElement;
  private forgeBox!: HTMLElement;
  private forgeInput!: HTMLInputElement;

  mount(host: PuzzleHost): void {
    this.host = host;
    const w = warden();
    const e1 = challenge(w.sig1.r, w.P, utf8(w.m1));
    const e2 = challenge(w.sig2.r, w.P, utf8(w.m2));
    const sameR = toHex(w.sig1.r) === toHex(w.sig2.r);
    this.solveBox = h('div');
    this.forgeBox = h('div');
    this.forgeInput = h('input', { type: 'text', value: 'DLA ORDER 0419: UNSEAL THE ARCHIVE. RELEASE TO PUBLIC. AUTH: WARDEN', style: 'width:100%', 'data-testid': 'forge-text' }) as HTMLInputElement;

    host.body.append(
      h(
        'div',
        { class: 'stack' },
        lede(
          'Warden signs every Directorate order with one key. His hardware signer has a broken random number generator: it produced the same nonce <b>R</b> for two different orders. That is fatal.',
        ),
        card(
          "Warden's public key",
          hexBox(toHex(w.P), { label: 'x-only public key (verifies every order below)' }),
        ),
        h(
          'div',
          { class: 'grid2' },
          card('Order 0417', note(`"${w.m1}"`), kv([['R.x', h('span', { class: 'hex' }, shorten(toHex(w.sig1.r), 10, 8))], ['s₁', h('span', { class: 'hex' }, shorten(w.sig1.s.toString(16).padStart(64, '0'), 10, 8))], ['e₁ = H(R‖P‖m₁)', h('span', { class: 'hex' }, shorten(e1.toString(16).padStart(64, '0'), 10, 8))], ['verifies', verify(w.sig1, utf8(w.m1), w.P) ? h('span', { class: 'tag ok' }, 'YES') : h('span', { class: 'tag bad' }, 'NO')]])),
          card('Order 0418', note(`"${w.m2}"`), kv([['R.x', h('span', { class: 'hex' + (sameR ? ' amber' : '') }, shorten(toHex(w.sig2.r), 10, 8))], ['s₂', h('span', { class: 'hex' }, shorten(w.sig2.s.toString(16).padStart(64, '0'), 10, 8))], ['e₂ = H(R‖P‖m₂)', h('span', { class: 'hex' }, shorten(e2.toString(16).padStart(64, '0'), 10, 8))], ['verifies', verify(w.sig2, utf8(w.m2), w.P) ? h('span', { class: 'tag ok' }, 'YES') : h('span', { class: 'tag bad' }, 'NO')]])),
        ),
        card(
          'The leak',
          note(sameR ? 'Both orders carry the <b>same R.x</b>. Same nonce k, two different challenges. Subtract the equations and k cancels:' : 'These do not share a nonce.'),
          math('<i>s</i>₁ = <i>k</i> + <i>e</i>₁·<i>d</i>,   <i>s</i>₂ = <i>k</i> + <i>e</i>₂·<i>d</i>   ⟹   <i>d</i> = (<i>s</i>₁ − <i>s</i>₂) / (<i>e</i>₁ − <i>e</i>₂)  mod <i>n</i>', true),
          h('div', { class: 'row', style: 'margin-top:8px' }, btn("Recover Warden's key", () => this.recover(), 'primary', 'nonce-recover')),
          h('div', { style: 'margin-top:10px' }, this.solveBox),
        ),
        card(
          'Sign a new order',
          note('With Warden\'s private key you can sign anything the Directorate would, and every Directorate verifier will accept it. Write the order that unseals the Archive.'),
          h('div', { style: 'margin-top:8px' }, this.forgeInput),
          h('div', { class: 'row', style: 'margin-top:8px' }, btn('Sign as Warden', () => this.forge(), '', 'forge-sign')),
          h('div', { style: 'margin-top:8px' }, this.forgeBox),
        ),
      ),
    );
    host.status('Two orders share a nonce. Recover the key.', 'info');
  }

  private recover(): void {
    const w = warden();
    const sol = recoverFromNonceReuse(w.P, utf8(w.m1), w.sig1, utf8(w.m2), w.sig2);
    this.recovered = sol.d;
    const correct = sol.d === evenKey(w.d);
    this.solveBox.replaceChildren(
      kv([
        ['s₁ − s₂ mod n', h('span', { class: 'hex' }, shorten(sol.sDiff.toString(16).padStart(64, '0'), 10, 8))],
        ['e₁ − e₂ mod n', h('span', { class: 'hex' }, shorten(sol.eDiff.toString(16).padStart(64, '0'), 10, 8))],
        ['(e₁ − e₂)⁻¹ mod n', h('span', { class: 'hex' }, shorten(sol.eDiffInv.toString(16).padStart(64, '0'), 10, 8))],
        ['recovered d', h('span', { class: 'hex big green' }, shorten(sol.d.toString(16).padStart(64, '0'), 12, 10))],
        ['d·G matches his key?', correct ? h('span', { class: 'tag ok' }, 'YES') : h('span', { class: 'tag bad' }, 'NO')],
      ]),
      note('This is not a theoretical attack. The PlayStation 3 signing key fell to a constant nonce in 2010, and Android wallets lost coins to repeated nonces in 2013.'),
    );
    this.host.sfx('unlock');
    this.host.status('Private key recovered. Now sign an order with it.', 'ok');
  }

  private forge(): void {
    if (this.host.solved()) return;
    if (this.recovered === null) {
      this.host.status('Recover the key first.', 'bad');
      this.host.sfx('error');
      return;
    }
    const w = warden();
    const msg = this.forgeInput.value;
    const sig: Sig = sign(utf8(msg), this.recovered, bigIntToBytes(1n, 32));
    const ok = verify(sig, utf8(msg), w.P);
    this.forgeBox.replaceChildren(
      hexBox(sigHex(sig), { label: 'your signature as Warden', tone: ok ? 'green' : 'red' }),
      note(ok ? 'The Directorate\'s own verifiers accept this. The Archive door is listening for exactly this signature.' : 'Signature invalid.'),
    );
    if (!ok) {
      this.host.status('Signature did not verify.', 'bad');
      return;
    }
    // must actually be a different message than the two originals
    if (msg === w.m1 || msg === w.m2) {
      this.host.status('That is one of his old orders. Write a new one.', 'warn');
      return;
    }
    this.host.sfx('unlock');
    this.host.solve('Signed, as Warden, in Warden\'s own hand. The Archive will open for it.');
  }

  async autoplay(): Promise<void> {
    this.recover();
    this.forge();
  }
}

export { publicKey };
