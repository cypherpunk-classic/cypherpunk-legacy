import { randomBytes } from '@noble/hashes/utils.js';
import { BIP86_FIRST } from '../../crypto/bip39';
import { explainVerify, sign, sigHex, type Sig } from '../../crypto/bip340';
import { bigIntToBytes, toHex, utf8 } from '../../crypto/bytes';
import { xOnlyHex } from '../../crypto/secp';
import { h } from '../../ui/dom';
import { btn, card, hexBox, kv, lede, math, note, setHex, shorten } from '../../ui/widgets';
import type { Puzzle, PuzzleHost } from '../../ui/terminal';
import { grandpa } from '../../story/world';

/**
 * Prologue, part 2. The gateway knows Grandpa's public key. It sends a fresh
 * challenge; you sign it with the recovered private key (BIP-340 Schnorr);
 * the gateway checks s·G = R + e·P. No password ever crosses the wire.
 */
export class GatewayLogin implements Puzzle {
  id = 'pro.login';
  title = 'CRYPTOPOLIS GATEWAY';
  subtitle = 'challenge–response · BIP-340';
  accent = '#3ef0ff';
  private host!: PuzzleHost;
  private challenge = '';
  private sig: Sig | null = null;
  private tamper!: HTMLInputElement;
  private sigBox!: HTMLElement;
  private verifyBox!: HTMLElement;
  private sendBtn!: HTMLButtonElement;

  mount(host: PuzzleHost): void {
    this.host = host;
    const gp = grandpa();
    const pub = xOnlyHex(gp.P);
    const nonce = toHex(randomBytes(16));
    this.challenge = `CRYPTOPOLIS-LOGIN|user=nightjar|nonce=${nonce}`;
    this.sigBox = h('div', { class: 'note' }, 'Not signed yet.');
    this.verifyBox = h('div');
    this.tamper = h('input', { type: 'checkbox' }) as HTMLInputElement;
    this.sendBtn = btn('Send signature to gateway', () => this.send(), 'primary', 'login-send');
    this.sendBtn.disabled = true;

    host.body.append(
      h(
        'div',
        { class: 'stack' },
        lede(
          'The terminal does not want the seed, and it does not want a password. It wants proof that you hold the private key behind the public key it has on file for <b>nightjar</b>. It sends a random challenge; you sign it; it checks the signature.',
        ),
        h(
          'div',
          { class: 'grid2' },
          card(
            'Your side',
            kv([
              ['derivation path', BIP86_FIRST],
              ['private key', h('span', { class: 'hex red' }, '•••• never leaves this machine ••••')],
              ['public key (x-only)', h('span', { class: 'hex' }, pub)],
            ]),
          ),
          card(
            'Gateway records',
            kv([
              ['user', 'nightjar'],
              ['registered key', h('span', { class: 'hex' }, pub)],
              ['match', h('span', { class: 'tag ok' }, 'SAME KEY')],
            ]),
          ),
        ),
        card(
          'Challenge',
          note('A fresh nonce every time, so an old signature can never be replayed.'),
          hexBox(this.challenge, { tone: 'amber' }),
          h('div', { class: 'row', style: 'margin-top:10px' }, btn('Sign the challenge', () => this.doSign(), 'primary', 'login-sign')),
          h('div', { style: 'margin-top:10px' }, this.sigBox),
        ),
        card(
          'Send',
          h('label', { class: 'row note', style: 'cursor:pointer' }, this.tamper, 'Flip one bit of the signature on the way (see what happens)'),
          h('div', { class: 'row', style: 'margin-top:10px' }, this.sendBtn),
          h('div', { style: 'margin-top:10px' }, this.verifyBox),
        ),
      ),
    );
    host.status('Sign the challenge with the recovered key.', 'info');
  }

  private doSign(): void {
    const gp = grandpa();
    this.sig = sign(utf8(this.challenge), gp.secret, randomBytes(32));
    const hex = sigHex(this.sig);
    this.sigBox.replaceChildren(
      note('A BIP-340 signature is 64 bytes: R.x (the nonce point) followed by s = k + e·d.'),
      hexBox(hex, { label: 'signature  =  R.x ‖ s', testid: 'login-sig' }),
    );
    this.sendBtn.disabled = false;
    this.host.sfx('confirm');
    this.host.status('Signed. Send it to the gateway.', 'info');
  }

  private send(): void {
    if (!this.sig || this.host.solved()) return;
    const gp = grandpa();
    const msg = utf8(this.challenge);
    const sent: Sig = this.tamper.checked ? { r: this.sig.r, s: this.sig.s ^ 1n } : this.sig;
    const ex = explainVerify(sent, msg, bigIntToBytes(gp.P.toAffine().x, 32));
    const box = (p: string, v: string, tone: string): HTMLElement => h('div', null, h('div', { class: 'note' }, p), h('div', { class: `hex ${tone}` }, v));
    this.verifyBox.replaceChildren(
      h('div', { class: 'stack' }, math('<i>e</i> = H<sub>challenge</sub>(<i>R</i>.x ‖ <i>P</i>.x ‖ <i>m</i>)  mod <i>n</i>', true), box('e', shorten(ex.e.toString(16).padStart(64, '0'), 20, 12), ''), box('left side:  s·G  (x coordinate)', ex.lhs.toAffine().x.toString(16).padStart(64, '0'), ex.ok ? 'green' : 'red'), box('right side: R + e·P  (x coordinate)', ex.rhs.toAffine().x.toString(16).padStart(64, '0'), ex.ok ? 'green' : 'red')),
    );
    if (!ex.ok) {
      this.host.status('ACCESS DENIED: s·G ≠ R + e·P. One flipped bit and the equation falls apart.', 'bad');
      this.host.sfx('error');
      return;
    }
    const sigBox = this.sigBox.querySelector('.hexbox');
    if (sigBox) setHex(sigBox as HTMLElement, sigHex(sent));
    this.host.solve('ACCESS GRANTED. Welcome back, nightjar.');
  }

  async autoplay(): Promise<void> {
    this.tamper.checked = false;
    this.doSign();
    this.send();
  }
}
