import { WEAK_GROUP, bsgsAsync, pub } from '../../crypto/dh';
import { state } from '../../core/state';
import { h } from '../../ui/dom';
import { btn, card, kv, lede, math, meter, note, shorten, typeValue } from '../../ui/widgets';
import type { Puzzle, PuzzleHost } from '../../ui/terminal';

/**
 * Chapter 2, part 3. The Directorate forces links down to a weak,
 * "export-grade" group (echoing the real Logjam attack on 512-bit DH). With a
 * 31-bit subgroup, baby-step giant-step recovers the secret exponent in about
 * 2·sqrt(q) steps instead of q, so the "downgraded for compliance" channel is
 * readable in seconds.
 */
export class Logjam implements Puzzle {
  id = 'ch2.logjam';
  title = 'DOWNGRADE · EXPORT-GRADE DH';
  subtitle = 'q ≈ 2³¹, baby-step giant-step';
  accent = '#ffb347';
  private host!: PuzzleHost;
  private target = 0n;
  private secret = 0n;
  private mInput!: HTMLInputElement;
  private bars = { baby: meter(), giant: meter() };
  private out!: HTMLElement;
  private running = false;

  mount(host: PuzzleHost): void {
    this.host = host;
    const g = WEAK_GROUP;
    const r = state.rng('ch2/logjam');
    this.secret = r.range(1000000n, g.q - 1n);
    this.target = pub(g, this.secret);
    const sqrtq = Math.ceil(Math.sqrt(Number(g.q)));
    this.mInput = h('input', { type: 'number', value: String(sqrtq), style: 'width:120px', 'data-testid': 'bsgs-m' }) as HTMLInputElement;
    this.out = h('div', { class: 'note' });
    host.body.append(
      h(
        'div',
        { class: 'stack' },
        lede(
          'To comply with the Lawful Access Act, the Directorate downgrades every link it cannot read to an old "export-grade" group: a legacy of the 1990s, when strong cryptography was export-controlled. This one has a 31-bit subgroup. That was the real Logjam attack in 2015, and it is why weak groups are dangerous even today.',
        ),
        card(
          'Intercepted exchange',
          kv([
            ['group order q', `${g.q}  (about 2^31)`],
            ['generator g', String(g.g)],
            ['public value A = g^a', h('span', { class: 'hex big amber' }, String(this.target))],
            ['secret a', h('span', { class: 'hex red' }, 'this is what we want')],
          ]),
        ),
        card(
          'Baby-step giant-step',
          note('Brute force would try up to q ≈ 2 billion exponents. Shanks\'s trick writes a = i·m + j with m ≈ √q: tabulate g^j for all j (baby steps), then step A·(g^−m)^i until a table entry matches (giant steps). About 2√q work, √q memory.'),
          math('<i>a</i> = <i>i</i>·<i>m</i> + <i>j</i>,   store <i>g</i><sup><i>j</i></sup> for <i>j</i> &lt; <i>m</i>,   seek <i>A</i>·(<i>g</i><sup>−<i>m</i></sup>)<sup><i>i</i></sup> ∈ table', true),
          h('div', { class: 'row', style: 'margin-top:6px' }, h('span', { class: 'note' }, 'step size m ='), this.mInput, btn('optimal √q', () => typeValue(this.mInput, String(Math.ceil(Math.sqrt(Number(g.q))))), 'small')),
          h('div', { style: 'margin-top:10px' }, h('div', { class: 'note' }, 'baby steps (building table)'), this.bars.baby.el, h('div', { class: 'note', style: 'margin-top:6px' }, 'giant steps (searching)'), this.bars.giant.el),
          h('div', { class: 'row', style: 'margin-top:10px' }, btn('Run the attack', () => void this.run(), 'primary', 'bsgs-run')),
          h('div', { style: 'margin-top:8px' }, this.out),
        ),
      ),
    );
    host.status('Recover the secret exponent from the public value.', 'info');
  }

  private async run(): Promise<void> {
    if (this.running || this.host.solved()) return;
    this.running = true;
    const g = WEAK_GROUP;
    const m = Math.max(1, Math.floor(Number(this.mInput.value) || Math.sqrt(Number(g.q))));
    this.bars.baby.set(0);
    this.bars.giant.set(0);
    this.out.textContent = 'Running…';
    const res = await bsgsAsync(g, this.target, m, (phase, frac) => this.bars[phase].set(frac));
    this.bars.baby.set(1);
    this.running = false;
    if (res.x === null || pub(g, res.x) !== this.target) {
      this.out.innerHTML = `<span style="color:var(--red)">No match with m = ${m}. The step size must satisfy m · (giant steps) ≥ q; try √q.</span>`;
      this.host.status('Attack failed with that step size.', 'bad');
      this.host.sfx('error');
      return;
    }
    this.bars.giant.set(1);
    this.out.innerHTML = `Recovered <b style="color:var(--green)">a = ${res.x}</b> in <b>${res.ms.toFixed(0)} ms</b> — ${res.babySteps.toLocaleString()} baby steps + ${res.giantSteps.toLocaleString()} giant steps. Brute force would have been up to ${g.q.toLocaleString()}.`;
    this.host.sfx('unlock');
    this.host.solve(`Secret exponent recovered: a = ${res.x}. “Export-grade” means readable. Strong groups (2048-bit, or elliptic curves) put √q out of reach — until quantum computers, which is Chapter 6.`);
  }

  async autoplay(): Promise<void> {
    typeValue(this.mInput, String(Math.ceil(Math.sqrt(Number(WEAK_GROUP.q)))));
    await this.run();
  }
}

export { shorten };
