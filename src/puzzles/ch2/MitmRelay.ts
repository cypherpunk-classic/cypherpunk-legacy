import { DRONE_GROUP, keypair, sessionKey, shared } from '../../crypto/dh';
import { gcmDecryptText, gcmEncryptText, type GcmBox } from '../../crypto/aead';
import { toHex } from '../../crypto/bytes';
import { state } from '../../core/state';
import { h } from '../../ui/dom';
import { Log, btn, card, kv, lede, note, shorten, typeValue } from '../../ui/widgets';
import type { Puzzle, PuzzleHost } from '../../ui/terminal';

const ORIGINAL = 'PATROL 7 LOG: UNESCROWED KEYS SEEN AT THE EXCHANGE. FLAG FOR AUDIT.';
const REWRITE = 'PATROL 7 LOG: NOTHING TO REPORT AT THE EXCHANGE. ALL CLEAR.';

type Stage = 'keygen' | 'patrol-hello' | 'control-hello' | 'message' | 'done';

/**
 * Chapter 2, part 2. Two Directorate nodes, a field patrol and its control
 * desk, set up a shared key with plain Diffie-Hellman through Felix's old
 * switchboard: neither side signs anything. Whoever sits on the wire can give
 * each side its own public value and end up sharing one key with each. Then the
 * traffic is readable, and rewritable. The whole point is the fix in Chapter 4:
 * authenticate the key exchange and this stops working.
 */
export class MitmRelay implements Puzzle {
  id = 'ch2.mitm';
  title = 'SWITCHBOARD · LINE 7';
  subtitle = 'patrol ⇄ control, unauthenticated DH (64-bit group)';
  accent = '#ef7d57';
  private host!: PuzzleHost;
  private stage: Stage = 'keygen';
  private patrol!: { secret: bigint; public: bigint };
  private control!: { secret: bigint; public: bigint };
  private relay: { secret: bigint; public: bigint } | null = null;
  private toControl = 0n; // the public value control received as "the patrol's"
  private toPatrol = 0n; // the public value the patrol received as "control's"
  private message: GcmBox | null = null;
  private kPatrol: bigint | null = null;
  private kControl: bigint | null = null;
  private log = new Log(200);
  private panel!: HTMLElement;
  private cols!: HTMLElement;

  private grp = DRONE_GROUP;

  mount(host: PuzzleHost): void {
    this.host = host;
    const r = state.rng('ch2/mitm');
    this.patrol = keypair(this.grp, r);
    this.control = keypair(this.grp, r);
    this.message = gcmEncryptText(sessionKey(this.grp, shared(this.grp, this.patrol.secret, this.control.public)), r.bytes(12), ORIGINAL);
    this.panel = h('div');
    this.cols = h('div', { class: 'grid3' });
    host.body.append(
      h(
        'div',
        { class: 'stack' },
        lede(
          'Two Directorate nodes agree on a key with ordinary Diffie-Hellman and talk over Felix\'s switchboard. Nobody signs their public value, and the exchange never says <em>who</em> sent it. You are the switchboard. Give each side a public value of your own and you will share a different key with each, and sit in the middle of the conversation.',
        ),
        this.cols,
        card('Switchboard', this.panel),
        card('Line log', this.log.el),
      ),
    );
    this.render();
    this.log.line('Line 7 open. Key exchange about to begin.', 'dim');
  }

  private render(): void {
    const v = (x: bigint | null | undefined): string => (x === null || x === undefined || x === 0n ? '—' : shorten(x.toString(), 8, 6));
    const kp = this.toPatrol ? shared(this.grp, this.patrol.secret, this.toPatrol) : null;
    const kc = this.toControl ? shared(this.grp, this.control.secret, this.toControl) : null;
    const key = (k: bigint | null): string => (k ? shorten(toHex(sessionKey(this.grp, k)), 6, 4) : '—');
    this.cols.replaceChildren(
      card('Field patrol', kv([
        ['secret a', h('span', { class: 'hex red' }, 'hidden')],
        ['sends A', h('span', { class: 'hex' }, v(this.patrol.public))],
        ['thinks control is', h('span', { class: 'hex' }, v(this.toPatrol))],
        ['its key', h('span', { class: 'hex' }, key(kp))],
      ])),
      card('You (switchboard)', kv([
        ['secret m', h('span', { class: 'hex' }, this.relay ? 'yours' : '—')],
        ['public M', h('span', { class: 'hex' }, v(this.relay?.public))],
        ['key with patrol', h('span', { class: 'hex amber' }, key(this.kPatrol))],
        ['key with control', h('span', { class: 'hex amber' }, key(this.kControl))],
      ])),
      card('Control desk', kv([
        ['secret b', h('span', { class: 'hex red' }, 'hidden')],
        ['sends B', h('span', { class: 'hex' }, v(this.control.public))],
        ['thinks patrol is', h('span', { class: 'hex' }, v(this.toControl))],
        ['its key', h('span', { class: 'hex' }, key(kc))],
      ])),
    );
    this.renderPanel();
  }

  private renderPanel(): void {
    const p = this.panel;
    p.replaceChildren();
    switch (this.stage) {
      case 'keygen':
        p.append(note('First make yourself a key pair in the same group, so you have a public value to substitute for each side.'), h('div', { class: 'row', style: 'margin-top:8px' }, btn('Generate switchboard key m, M = g^m', () => this.keygen(), 'primary', 'mitm-keygen')));
        this.host.status('Prepare a relay key.', 'info');
        break;
      case 'patrol-hello':
        p.append(
          h('div', { class: 'hex' }, `▶ PATROL → CONTROL   KEYX  A = ${this.patrol.public}`),
          h('div', { class: 'row', style: 'margin-top:8px' }, btn('Forward A unchanged', () => this.patrolHello(false), '', 'mitm-fwd-a'), btn('Substitute your M', () => this.patrolHello(true), 'primary', 'mitm-swap-a')),
        );
        this.host.status('The patrol\'s half of the exchange is passing through.', 'info');
        break;
      case 'control-hello':
        p.append(
          h('div', { class: 'hex' }, `◀ CONTROL → PATROL   KEYX  B = ${this.control.public}`),
          h('div', { class: 'row', style: 'margin-top:8px' }, btn('Forward B unchanged', () => this.controlHello(false), '', 'mitm-fwd-b'), btn('Substitute your M', () => this.controlHello(true), 'primary', 'mitm-swap-b')),
        );
        this.host.status('The control desk answers.', 'info');
        break;
      case 'message': {
        const canRead = this.kPatrol !== null;
        const canWrite = this.kControl !== null;
        const read = canRead ? gcmDecryptText(sessionKey(this.grp, this.kPatrol!), this.message!) : null;
        const ta = h('textarea', { rows: '3', style: 'width:100%', 'data-testid': 'mitm-text' }) as HTMLTextAreaElement;
        if (read?.ok) ta.value = REWRITE;
        const out = h('div');
        p.append(
          h('div', { class: 'hex' }, `▶ PATROL → CONTROL   MSG  ${this.message!.ct.length} bytes, GCM`),
          h('div', { style: 'margin-top:8px' }, canRead
            ? note(`Decrypted with your shared-with-patrol key:`)
            : note('You do not share a key with the patrol, so this is just ciphertext to you. (Forward both exchanges unchanged and you learn nothing.)')),
          ...(read?.ok ? [h('div', { class: 'hex big green' }, read.text)] : []),
          ...(canRead ? [h('div', { style: 'margin-top:10px' }, note('Re-encrypt whatever you like for the control desk:'), ta)] : []),
          h('div', { class: 'row', style: 'margin-top:8px' },
            btn('Relay to control desk', () => this.deliver(ta.value, canWrite), 'primary', 'mitm-deliver'),
          ),
          out,
        );
        this.outEl = out;
        this.host.status(canRead ? 'Read it, change it, pass it on.' : 'Deliver the message to see whether you learned anything.', canRead ? 'info' : 'warn');
        break;
      }
      case 'done':
        p.append(note('Line 7 closed. Felix saw the whole thing from the switchboard.'));
        break;
    }
  }

  private outEl: HTMLElement | null = null;

  private keygen(): void {
    this.relay = keypair(this.grp, state.rng('ch2/mitm-relay-' + Date.now()));
    this.log.line(`You generated M = g^m = ${this.relay.public}.`, 'info');
    this.stage = 'patrol-hello';
    this.host.sfx('confirm');
    this.render();
  }

  private patrolHello(swap: boolean): void {
    this.toControl = swap ? this.relay!.public : this.patrol.public;
    if (swap) {
      this.kPatrol = shared(this.grp, this.relay!.secret, this.patrol.public);
      this.log.line('You sent control your M instead of A. Control will share a key with YOU, not the patrol.', 'warn');
    } else this.log.line('You forwarded A unchanged.', 'dim');
    this.stage = 'control-hello';
    this.host.sfx('blip');
    this.render();
  }

  private controlHello(swap: boolean): void {
    this.toPatrol = swap ? this.relay!.public : this.control.public;
    if (swap) {
      this.kControl = shared(this.grp, this.relay!.secret, this.control.public);
      this.log.line('You sent the patrol your M instead of B. The patrol will share a key with YOU.', 'warn');
    } else this.log.line('You forwarded B unchanged.', 'dim');
    this.stage = 'message';
    this.host.sfx('blip');
    this.render();
  }

  private deliver(text: string, canWrite: boolean): void {
    if (this.host.solved()) return;
    const out = this.outEl!;
    // What the control desk actually ends up with.
    const forwarded = canWrite ? gcmEncryptText(sessionKey(this.grp, this.kControl!), this.message!.nonce, text) : this.message!;
    const kc = this.toControl ? shared(this.grp, this.control.secret, this.toControl) : null;
    const seen = kc ? gcmDecryptText(sessionKey(this.grp, kc), forwarded) : { ok: false as const, error: 'no key' };
    if (!this.kPatrol || !this.kControl) {
      out.replaceChildren(note('You forwarded the exchange honestly, so you never held a key. The control desk read the patrol\'s real message and you read nothing. That is Diffie-Hellman working as intended.'));
      this.log.line('Control received the original message intact. You saw nothing.', 'dim');
      this.host.status('Relayed honestly. To read or change the traffic, substitute your key on BOTH exchanges.', 'warn');
      this.stage = 'done';
      setTimeout(() => this.render(), 1200);
      return;
    }
    out.replaceChildren(
      note('The control desk decrypts with the key it thinks it shares with the patrol, and reads:'),
      h('div', { class: 'hex big ' + (seen.ok ? 'amber' : 'red') }, seen.ok ? seen.text : '(garbled)'),
      note('It has no way to tell this did not come straight from the patrol. Nothing in the exchange carried an identity.'),
    );
    this.log.line('Control accepted your rewritten message as genuine.', 'bad');
    this.host.sfx('unlock');
    this.host.solve('You sat in the middle of the whole conversation. The cure is authentication: make each side sign its public value, and your substituted key no longer matches a signature. That is Chapter 4.');
    this.stage = 'done';
  }

  async autoplay(): Promise<void> {
    this.keygen();
    this.patrolHello(true);
    this.controlHello(true);
    const ta = this.host.body.querySelector('[data-testid=mitm-text]') as HTMLTextAreaElement | null;
    if (ta) typeValue(ta, REWRITE);
    this.deliver(REWRITE, true);
  }
}
