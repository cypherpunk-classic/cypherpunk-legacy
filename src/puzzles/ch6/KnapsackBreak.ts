import { attackBlock, bitsToText, density, makeKnapsackPuzzle, type KnapsackPuzzle } from '../../crypto/knapsack';
import { log2Big } from '../../crypto/bytes';
import { state } from '../../core/state';
import { h } from '../../ui/dom';
import { Log, btn, card, kv, lede, math, note, plotCanvas } from '../../ui/widgets';
import type { Puzzle, PuzzleHost } from '../../ui/terminal';
import { KNAPSACK_MESSAGE } from '../../story/world';

/**
 * Chapter 6, part 2. The Directorate's "quantum-safe QSAFE" channel is a
 * relabelled Merkle–Hellman knapsack. We build the CJLOSS lattice from the
 * public key and a ciphertext, run the real integral LLL, and watch the
 * Gram–Schmidt profile flatten as reduction finds the short vector that holds
 * the plaintext bits. "Hiding structure in a lattice" cuts both ways.
 */
export class KnapsackBreak implements Puzzle {
  id = 'ch6.knapsack';
  title = 'QSAFE CHANNEL · BREAK IN';
  subtitle = 'Merkle–Hellman + LLL (CJLOSS)';
  accent = '#73b7ff';
  private host!: PuzzleHost;
  private puzzle!: KnapsackPuzzle;
  private blockIdx = 0;
  private recovered = '';
  private canvas!: HTMLCanvasElement;
  private ctx!: CanvasRenderingContext2D;
  private cw = 0;
  private chh = 0;
  private log = new Log(150);
  private out!: HTMLElement;
  private running = false;

  async mount(host: PuzzleHost): Promise<void> {
    this.host = host;
    host.status('Generating an attackable QSAFE instance…', 'info');
    this.puzzle = makeKnapsackPuzzle(state.rng('ch6/knapsack'), KNAPSACK_MESSAGE, 32);
    const plot = plotCanvas(460, 220);
    this.canvas = plot.canvas;
    this.ctx = plot.ctx;
    this.cw = plot.w;
    this.chh = plot.h;
    this.out = h('div');

    host.body.append(
      h(
        'div',
        { class: 'stack' },
        lede(
          'The Directorate rebranded an old public-key scheme as "QSAFE" and promised it is quantum-safe. It is a Merkle–Hellman knapsack from 1978, broken since 1982. Encryption hides an easy subset-sum inside a hard-looking one; lattice reduction digs it back out.',
        ),
        math('CJLOSS basis: <i>b</i><sub>i</sub> = (0,…,2,…,0, <i>N·a</i><sub>i</sub>),   <i>b</i><sub>n+1</sub> = (1,…,1, <i>N·c</i>).   Target: (±1,…,±1, 0)', true),
        card(
          'Intercepted',
          kv([
            ['public key', `${this.puzzle.keys.pub.length} weights, density ${density(this.puzzle.keys.pub).toFixed(2)} (low = breakable)`],
            ['ciphertext blocks', String(this.puzzle.blocks.length)],
          ]),
        ),
        h(
          'div',
          { class: 'grid2' },
          card('Gram–Schmidt profile (log₂ |bᵢ*|)', note('A skewed basis has a steep profile. LLL flattens it; that is reduction, made visible.'), this.canvas),
          card('Attack', note('Run LLL on each block\'s lattice. The ±1 short vector it finds decodes to the plaintext bits.'), h('div', { class: 'row', style: 'margin-top:8px' }, btn('Break next block', () => void this.breakBlock(), 'primary', 'knap-break'), btn('Break all', () => void this.breakAll(), '', 'knap-all')), h('div', { style: 'margin-top:8px' }, this.out), this.log.el),
        ),
      ),
    );
    this.drawProfile(null, null);
    host.status('Run the lattice attack on the QSAFE ciphertext.', 'info');
  }

  private drawProfile(before: number[] | null, after: number[] | null): void {
    const ctx = this.ctx;
    ctx.fillStyle = '#070b16';
    ctx.fillRect(0, 0, this.cw, this.chh);
    const all = [...(before ?? []), ...(after ?? [])];
    if (!all.length) {
      ctx.fillStyle = '#56627d';
      ctx.font = '12px monospace';
      ctx.fillText('run the attack to see the profile', 20, this.chh / 2);
      return;
    }
    const lo = Math.min(...all) - 0.5;
    const hi = Math.max(...all) + 0.5;
    const n = Math.max(before?.length ?? 0, after?.length ?? 0);
    const px = (i: number): number => 30 + (i / (n - 1)) * (this.cw - 50);
    const py = (v: number): number => this.chh - 25 - ((v - lo) / (hi - lo)) * (this.chh - 45);
    const series = (prof: number[], color: string, label: string, yOff: number): void => {
      ctx.strokeStyle = color;
      ctx.fillStyle = color;
      ctx.lineWidth = 2;
      ctx.beginPath();
      prof.forEach((v, i) => (i === 0 ? ctx.moveTo(px(i), py(v)) : ctx.lineTo(px(i), py(v))));
      ctx.stroke();
      prof.forEach((v, i) => {
        ctx.beginPath();
        ctx.arc(px(i), py(v), 2, 0, Math.PI * 2);
        ctx.fill();
      });
      ctx.font = '11px monospace';
      ctx.fillText(label, this.cw - 120, 16 + yOff);
    };
    if (before) series(before, '#ff6b81', 'before (skewed)', 0);
    if (after) series(after, '#5dff9a', 'after LLL (flat)', 14);
  }

  private async breakBlock(): Promise<void> {
    if (this.running || this.host.solved()) return;
    if (this.blockIdx >= this.puzzle.blocks.length) return;
    this.running = true;
    const c = this.puzzle.blocks[this.blockIdx]!;
    const res = attackBlock(this.puzzle.keys.pub, c);
    this.drawProfile(res.lll.profileBefore, res.lll.profileAfter);
    if (res.bits) {
      const text = bitsToText(res.bits);
      this.recovered += text;
      this.log.line(`block ${this.blockIdx + 1}: LLL ${res.lll.swaps} swaps, ${res.lll.ms.toFixed(0)}ms → "${text}"`, 'ok');
    } else this.log.line(`block ${this.blockIdx + 1}: no short vector found`, 'bad');
    this.blockIdx++;
    this.out.replaceChildren(h('div', { class: 'hex big green' }, this.recovered || '…'));
    this.host.sfx('confirm');
    this.running = false;
    if (this.blockIdx >= this.puzzle.blocks.length) this.finish();
  }

  private async breakAll(): Promise<void> {
    while (this.blockIdx < this.puzzle.blocks.length && !this.host.solved()) {
      await this.breakBlock();
      await new Promise((r) => setTimeout(r, 120));
    }
  }

  private finish(): void {
    const msg = this.recovered.trimEnd();
    this.out.replaceChildren(h('div', { class: 'hex big green' }, msg));
    void log2Big;
    this.host.sfx('unlock');
    this.host.solve(`QSAFE broken: "${msg}". A knapsack is not quantum-safe; it is barely classically safe. Real post-quantum crypto needs the hard lattice problems, not the easy ones in disguise.`);
  }

  async autoplay(): Promise<void> {
    await this.breakAll();
  }
}
