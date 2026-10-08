import { TOY_GROUP, pub, shared } from '../../crypto/dh';
import { state } from '../../core/state';
import { h } from '../../ui/dom';
import { btn, card, kv, lede, math, note, typeValue } from '../../ui/widgets';
import type { Puzzle, PuzzleHost } from '../../ui/terminal';

/**
 * Chapter 2, part 1. Diffie–Hellman with numbers small enough to watch:
 * p = 1019, g = 4 (order 509). Pick a secret, publish g^a, combine with
 * Felix's public value, and end up with the same key he has. Then watch an
 * eavesdropper break such a tiny group by trying every exponent.
 */
export class DhBasics implements Puzzle {
  id = 'ch2.dh';
  title = 'SWITCHBOARD · KEY EXCHANGE';
  subtitle = 'p = 1019, g = 4';
  accent = '#ef7d57';
  private host!: PuzzleHost;
  private aIn!: HTMLInputElement;
  private A: bigint | null = null;
  private K: bigint | null = null;
  private steps!: HTMLElement;
  private mine!: HTMLElement;
  private felix!: HTMLElement;
  private eve!: HTMLElement;
  private b = 0n;
  private B = 0n;
  private sent = false;

  mount(host: PuzzleHost): void {
    this.host = host;
    const g = TOY_GROUP;
    const r = state.rng('ch2/felix-toy');
    this.b = r.range(2n, g.q - 1n);
    this.B = pub(g, this.b);
    this.aIn = h('input', { type: 'number', min: '2', max: String(g.q - 1n), value: '', placeholder: '2 … 508', style: 'width:130px', 'data-testid': 'dh-a' }) as HTMLInputElement;
    this.steps = h('div', { class: 'log', style: 'max-height:160px' });
    this.mine = h('div');
    this.felix = h('div');
    this.eve = h('div');
    host.body.append(
      h(
        'div',
        { class: 'stack' },
        lede(
          'In 1976 Diffie and Hellman showed two strangers how to agree on a secret while everyone listens. Everything below is public except the two secret exponents. Here the numbers are tiny so you can see them.',
        ),
        math('<i>A</i> = <i>g</i><sup><i>a</i></sup> mod <i>p</i>,   <i>B</i> = <i>g</i><sup><i>b</i></sup> mod <i>p</i>,   <i>K</i> = <i>B</i><sup><i>a</i></sup> = <i>A</i><sup><i>b</i></sup> = <i>g</i><sup><i>ab</i></sup>', true),
        h(
          'div',
          { class: 'grid2' },
          card(
            'You',
            h('div', { class: 'row' }, h('span', { class: 'note' }, 'secret a ='), this.aIn, btn('Random', () => typeValue(this.aIn, String(2 + Math.floor(Math.random() * 500))), 'small')),
            h('div', { class: 'row', style: 'margin-top:8px' }, btn('Compute A = g^a mod p', () => this.computeA(), 'primary', 'dh-compute-a'), btn('Send A to Felix', () => this.send(), '', 'dh-send')),
            h('div', { style: 'margin-top:8px' }, this.steps),
            h('div', { style: 'margin-top:8px' }, this.mine),
          ),
          card('Felix', kv([['secret b', h('span', { class: 'hex red' }, '(only Felix knows)')], ['public B', h('span', { class: 'hex big' }, String(this.B))]]), h('div', { style: 'margin-top:8px' }, this.felix)),
        ),
        card('Eve, listening on the line', note('Eve sees p, g, A and B. To get K she needs a or b: a <span class="term">discrete logarithm</span>.'), h('div', { style: 'margin-top:8px' }, this.eve)),
      ),
    );
    host.status('Choose a secret exponent a between 2 and 508.', 'info');
  }

  private computeA(): void {
    const g = TOY_GROUP;
    const a = BigInt(Math.floor(Number(this.aIn.value)));
    if (!(a >= 2n && a < g.q)) {
      this.host.status('Pick a between 2 and 508.', 'bad');
      return;
    }
    // square-and-multiply, shown
    this.steps.replaceChildren();
    const bits = a.toString(2);
    let acc = 1n;
    const line = (t: string, tone = ''): void => {
      this.steps.append(h('div', { class: tone }, t));
    };
    line(`a = ${a} = 0b${bits}  (square-and-multiply, mod ${g.p})`, 'dim');
    for (const bit of bits) {
      acc = (acc * acc) % g.p;
      if (bit === '1') {
        acc = (acc * g.g) % g.p;
        line(`square, multiply by g → ${acc}`);
      } else line(`square → ${acc}`);
    }
    this.A = acc;
    line(`A = ${acc}`, 'ok');
    this.mine.replaceChildren(kv([['public A', h('span', { class: 'hex big' }, String(acc))]]));
    this.sent = false;
    this.K = null;
    this.host.sfx('confirm');
    this.host.status('Now send A to Felix. It is safe to say out loud.', 'info');
  }

  private send(): void {
    if (this.A === null) {
      this.host.status('Compute A first.', 'bad');
      return;
    }
    this.sent = true;
    const g = TOY_GROUP;
    const a = BigInt(Math.floor(Number(this.aIn.value)));
    this.K = shared(g, a, this.B);
    const kf = shared(g, this.b, this.A);
    this.felix.replaceChildren(kv([['Felix computes', h('span', { class: 'math', html: `<i>A</i><sup><i>b</i></sup> mod <i>p</i>` })], ['his key', h('span', { class: 'hex big green' }, String(kf))]]));
    this.mine.append(
      h('div', { style: 'margin-top:6px' }, kv([['you compute', h('span', { class: 'math', html: `<i>B</i><sup><i>a</i></sup> mod <i>p</i>` })], ['your key', h('span', { class: 'hex big green' }, String(this.K))]])),
    );
    if (kf !== this.K) {
      this.host.status('Keys differ? That should be impossible.', 'bad');
      return;
    }
    this.host.sfx('unlock');
    this.eve.replaceChildren(
      h('div', { class: 'row' }, btn('Let Eve try every exponent', () => this.brute(), 'small', 'dh-eve'), h('span', { class: 'note' }, `only ${g.q - 1n} possibilities in this toy group`)),
    );
    this.host.solve(`Same key on both sides: K = ${this.K}. Nobody ever sent it.`);
  }

  private brute(): void {
    const g = TOY_GROUP;
    const t0 = performance.now();
    let found = -1n;
    for (let x = 1n; x < g.q; x++) {
      if (pub(g, x) === this.A) {
        found = x;
        break;
      }
    }
    const ms = performance.now() - t0;
    this.eve.append(
      note(
        `Eve found a = ${found} in ${ms.toFixed(2)} ms by brute force, and with it K. With a 2048-bit prime, or a 256-bit elliptic curve, the best known classical attacks need about 2<sup>112</sup> to 2<sup>128</sup> steps instead.`,
      ),
    );
  }

  async autoplay(): Promise<void> {
    typeValue(this.aIn, '277');
    this.computeA();
    this.send();
    void this.sent;
  }
}
