import { pairAddresses, tapLeafHash, tweakKey, type Leaf } from '../../crypto/taproot';
import { toHex } from '../../crypto/bytes';
import { xOnly } from '../../crypto/secp';
import { h } from '../../ui/dom';
import { btn, card, kv, lede, math, note, shorten, tag } from '../../ui/widgets';
import type { Puzzle, PuzzleHost } from '../../ui/terminal';
import { grandpa, taproot } from '../../story/world';

/**
 * Chapter 4, part 3. Grandpa's Vault was a Taproot output: an internal key
 * tweaked by a Merkle tree of spending scripts. Only the leaves actually used
 * are ever revealed. Try leaf pairs until one reproduces the published address,
 * learning what he committed to without him ever spending it.
 */
export class TaprootReveal implements Puzzle {
  id = 'ch4.taproot';
  title = "GRANDPA'S TAPROOT";
  subtitle = 'BIP-341 · key path + hidden scripts';
  accent = '#73eff7';
  private host!: PuzzleHost;
  private leaves: Leaf[] = [];
  private pairs!: ReturnType<typeof pairAddresses>;
  private targetAddr = '';
  private internalX!: Uint8Array;
  private selected: Set<string> = new Set();
  private grid!: HTMLElement;
  private result!: HTMLElement;

  mount(host: PuzzleHost): void {
    this.host = host;
    const tp = taproot();
    this.leaves = tp.leaves;
    this.internalX = xOnly(grandpa().P);
    this.targetAddr = tp.target.address;
    this.pairs = pairAddresses(this.internalX, this.leaves);
    this.grid = h('div', { class: 'stack', style: 'gap:6px' });
    this.result = h('div');

    host.body.append(
      h(
        'div',
        { class: 'stack' },
        lede(
          'The Vault address was a Taproot output. Its key commits to a Merkle tree of spending scripts via a tweak. From the address alone you cannot see the scripts, but if you can guess which leaves are in the tree, you can confirm them, because they must reproduce the address.',
        ),
        math('<i>Q</i> = <i>P</i> + <i>t</i>·<i>G</i>,   <i>t</i> = H<sub>TapTweak</sub>(<i>P</i>.x ‖ merkleRoot),   address = bech32m(<i>Q</i>.x)', true),
        card('What Grandpa published', kv([['internal key P', h('span', { class: 'hex' }, shorten(toHex(this.internalX), 10, 8))], ['Vault address', h('span', { class: 'hex big amber', 'data-testid': 'target-addr' }, this.targetAddr)]])),
        card(
          'Candidate spending conditions',
          note('Grandpa drafted four. Pick the two you think he committed to. The tree is a single branch of exactly two leaves.'),
          this.grid,
          h('div', { class: 'row', style: 'margin-top:8px' }, btn('Build tree & compare address', () => this.check(), 'primary', 'taproot-check'), h('span', { class: 'note' }, 'selected: '), this.counter),
        ),
        this.result,
      ),
    );
    this.renderGrid();
    host.status('Choose two spending conditions and check the address.', 'info');
  }

  private counter = h('span', { class: 'tag' }, '0 / 2');

  private renderGrid(): void {
    this.grid.replaceChildren(
      ...this.leaves.map((leaf) =>
        h(
          'label',
          { class: `card ${this.selected.has(leaf.id) ? 'accent' : ''}`, style: 'padding:8px 12px;cursor:pointer;display:flex;gap:10px;align-items:flex-start' },
          h('input', { type: 'checkbox', checked: this.selected.has(leaf.id), 'data-leaf': leaf.id, onchange: (e: Event) => this.toggle(leaf.id, (e.target as HTMLInputElement).checked) }),
          h('div', null, h('b', null, leaf.label), h('div', { class: 'hex', style: 'font-size:11px;color:var(--dim)' }, leaf.asm), h('div', { class: 'hex', style: 'font-size:10.5px' }, `TapLeaf hash ${shorten(toHex(tapLeafHash(leaf.script)), 8, 6)}`)),
        ),
      ),
    );
    this.counter.textContent = `${this.selected.size} / 2`;
    this.counter.className = `tag ${this.selected.size === 2 ? 'info' : ''}`;
  }

  private toggle(id: string, on: boolean): void {
    if (on) {
      if (this.selected.size >= 2) {
        this.renderGrid();
        return;
      }
      this.selected.add(id);
    } else this.selected.delete(id);
    this.renderGrid();
  }

  private check(): void {
    if (this.host.solved()) return;
    if (this.selected.size !== 2) {
      this.host.status('Pick exactly two.', 'warn');
      return;
    }
    const [a, b] = [...this.selected];
    const pair = this.pairs.find((p) => (p.a.id === a && p.b.id === b) || (p.a.id === b && p.b.id === a))!;
    const match = pair.address === this.targetAddr;
    const tweak = tweakKey(this.internalX, pair.root);
    this.result.replaceChildren(
      card(
        match ? 'Address matches' : 'Address does not match',
        kv([
          ['merkle root', h('span', { class: 'hex' }, shorten(toHex(pair.root), 10, 8))],
          ['tweak t', h('span', { class: 'hex' }, shorten(tweak.tweak.toString(16).padStart(64, '0'), 10, 8))],
          ['resulting address', h('span', { class: `hex ${match ? 'green' : 'red'}` }, pair.address)],
          ['target', h('span', { class: 'hex amber' }, this.targetAddr)],
        ]),
      ),
    );
    (this.result.firstChild as HTMLElement).classList.add(match ? 'good' : 'bad');
    if (!match) {
      this.host.status('Those two leaves give a different address. Try another pair.', 'bad');
      this.host.sfx('error');
      return;
    }
    const names = [pair.a, pair.b].map((l) => l.label).join('  ·  ');
    this.host.sfx('unlock');
    this.result.append(note(`The tree was: <b>${names}</b>. Everything else, including the Directorate's "escrow" leaf, was never committed. He built in a way for you to inherit, and left the Directorate out.`));
    this.host.solve('Taproot confirmed. The scripts he hid were a recovery path and an inheritance path. Both lead to you.');
  }

  async autoplay(): Promise<void> {
    const [a, b] = taproot().correct;
    this.toggle(a, true);
    this.toggle(b, true);
    this.check();
  }
}

export { tag };
