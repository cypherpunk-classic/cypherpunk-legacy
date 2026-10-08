import { det2, gaussReduce, isGaussReduced, mu2, n2, type Gauss2State, type V2 } from '../../crypto/lattice';
import { state } from '../../core/state';
import { h } from '../../ui/dom';
import { btn, card, lede, math, note, plotCanvas } from '../../ui/widgets';
import type { Puzzle, PuzzleHost } from '../../ui/terminal';

/**
 * Chapter 6, part 1. Lattice reduction in two dimensions, by hand. A lattice
 * is all integer combinations of two basis vectors; a bad basis is long and
 * skewed, a good one short and near-orthogonal. Lagrange–Gauss reduction
 * repeatedly subtracts the nearest multiple of the shorter vector from the
 * longer and swaps. When it stops, the shorter vector is the shortest in the
 * whole lattice. This is LLL's engine, visible.
 */
export class GaussReduce implements Puzzle {
  id = 'ch6.gauss';
  title = 'LATTICE REDUCTION';
  subtitle = 'Lagrange–Gauss, by hand';
  accent = '#73b7ff';
  private host!: PuzzleHost;
  private s!: Gauss2State;
  private shortest = 0n;
  private canvas!: HTMLCanvasElement;
  private ctx!: CanvasRenderingContext2D;
  private cw = 0;
  private chh = 0;
  private info!: HTMLElement;
  private muBox!: HTMLElement;
  private steps = 0;

  mount(host: PuzzleHost): void {
    this.host = host;
    const r = state.rng('ch6/gauss');
    // start from a reduced-ish basis then skew it with random unimodular ops
    let b1: V2 = [4n + BigInt(r.int(2)), 1n];
    let b2: V2 = [1n, 4n + BigInt(r.int(3))];
    for (let i = 0; i < 4; i++) {
      const k = BigInt(r.int(3) + 1) * (r.int(2) ? 1n : -1n);
      if (i % 2 === 0) b2 = [b2[0] + k * b1[0], b2[1] + k * b1[1]];
      else b1 = [b1[0] + k * b2[0], b1[1] + k * b2[1]];
    }
    this.s = { b1, b2 };
    this.shortest = n2(gaussReduce(this.s).state.b1);

    const plot = plotCanvas(440, 400);
    this.canvas = plot.canvas;
    this.ctx = plot.ctx;
    this.cw = plot.w;
    this.chh = plot.h;
    this.info = h('div', { class: 'note' });
    this.muBox = h('div');

    host.body.append(
      h(
        'div',
        { class: 'grid2' },
        h(
          'div',
          { class: 'stack' },
          lede('Both vectors below generate the same infinite grid of points. But one basis is a tangled mess and the equivalent short basis reveals the lattice\'s shortest vector. Reduce it.'),
          math('μ = ⟨b₁,b₂⟩ / ⟨b₁,b₁⟩,   replace b₂ ← b₂ − round(μ)·b₁,   swap if |b₂| &lt; |b₁|', true),
          card('Current basis', this.muBox),
          card(
            'Moves',
            note('Subtract the nearest multiple of the shorter vector from the longer, then swap so the shorter is b₁. Repeat until no subtraction shrinks anything.'),
            h('div', { class: 'row', style: 'margin-top:8px' },
              btn('b₂ ← b₂ − round(μ)·b₁', () => this.reduceStep(), 'primary', 'gauss-reduce'),
              btn('swap b₁ ↔ b₂', () => this.swap(), '', 'gauss-swap'),
            ),
            h('div', { class: 'row', style: 'margin-top:8px' }, btn('Check if reduced', () => this.check(), '', 'gauss-check'), btn('Auto-reduce', () => this.auto(), 'ghost', 'gauss-auto')),
            h('div', { style: 'margin-top:8px' }, this.info),
          ),
        ),
        h('div', null, this.canvas),
      ),
    );
    this.render();
    host.status('Reduce the basis until b₁ is the shortest vector.', 'info');
  }

  private px(x: number): number {
    return this.cw / 2 + x * this.scale;
  }
  private py(y: number): number {
    return this.chh / 2 - y * this.scale;
  }
  private scale = 16;

  private render(): void {
    const ctx = this.ctx;
    // auto-scale so both vectors fit
    const maxc = Math.max(4, ...[this.s.b1, this.s.b2].flatMap((v) => [Math.abs(Number(v[0])), Math.abs(Number(v[1]))]));
    this.scale = Math.min(26, (this.cw / 2 - 20) / (maxc + 1));
    ctx.fillStyle = '#070b16';
    ctx.fillRect(0, 0, this.cw, this.chh);
    // lattice points
    const range = Math.ceil(maxc / 1) + 3;
    ctx.fillStyle = '#1b2440';
    for (let i = -range; i <= range; i++)
      for (let j = -range; j <= range; j++) {
        const x = i * Number(this.s.b1[0]) + j * Number(this.s.b2[0]);
        const y = i * Number(this.s.b1[1]) + j * Number(this.s.b2[1]);
        if (Math.abs(x) <= maxc + 2 && Math.abs(y) <= maxc + 2) {
          ctx.beginPath();
          ctx.arc(this.px(x), this.py(y), 2, 0, Math.PI * 2);
          ctx.fill();
        }
      }
    // axes
    ctx.strokeStyle = '#17203a';
    ctx.beginPath();
    ctx.moveTo(0, this.py(0));
    ctx.lineTo(this.cw, this.py(0));
    ctx.moveTo(this.px(0), 0);
    ctx.lineTo(this.px(0), this.chh);
    ctx.stroke();
    // shortest-vector circle
    const shr = Math.sqrt(Number(this.shortest));
    ctx.strokeStyle = 'rgba(93,255,154,0.25)';
    ctx.setLineDash([3, 3]);
    ctx.beginPath();
    ctx.arc(this.px(0), this.py(0), shr * this.scale, 0, Math.PI * 2);
    ctx.stroke();
    ctx.setLineDash([]);
    // vectors
    this.arrow(this.s.b1, '#3ef0ff', 'b₁');
    this.arrow(this.s.b2, '#ff3e9a', 'b₂');
    this.renderInfo();
  }

  private arrow(v: V2, color: string, label: string): void {
    const ctx = this.ctx;
    const x = this.px(Number(v[0]));
    const y = this.py(Number(v[1]));
    ctx.strokeStyle = color;
    ctx.fillStyle = color;
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.moveTo(this.px(0), this.py(0));
    ctx.lineTo(x, y);
    ctx.stroke();
    ctx.beginPath();
    ctx.arc(x, y, 3.5, 0, Math.PI * 2);
    ctx.fill();
    ctx.font = 'bold 13px monospace';
    ctx.fillText(label, x + 5, y - 4);
  }

  private renderInfo(): void {
    const m = mu2(this.s);
    const len1 = Math.sqrt(Number(n2(this.s.b1)));
    const len2 = Math.sqrt(Number(n2(this.s.b2)));
    this.muBox.replaceChildren(
      h('div', { class: 'kv' },
        h('div', { class: 'k' }, 'b₁'), h('div', { class: 'v hex' }, `(${this.s.b1[0]}, ${this.s.b1[1]})  |b₁| = ${len1.toFixed(2)}`),
        h('div', { class: 'k' }, 'b₂'), h('div', { class: 'v hex' }, `(${this.s.b2[0]}, ${this.s.b2[1]})  |b₂| = ${len2.toFixed(2)}`),
        h('div', { class: 'k' }, 'μ'), h('div', { class: 'v hex' }, `${m.value.toFixed(3)}  → round(μ) = ${m.nearest}`),
        h('div', { class: 'k' }, 'det (area)'), h('div', { class: 'v hex' }, `${det2(this.s)}  (never changes)`),
      ),
    );
  }

  private reduceStep(): void {
    const m = mu2(this.s);
    if (m.nearest === 0n) {
      this.info.innerHTML = '<span style="color:var(--amber)">round(μ) = 0: subtracting would not help. Try a swap.</span>';
      this.host.sfx('error');
      return;
    }
    this.s = { b1: this.s.b1, b2: [this.s.b2[0] - m.nearest * this.s.b1[0], this.s.b2[1] - m.nearest * this.s.b1[1]] };
    this.steps++;
    this.host.sfx('blip');
    this.info.textContent = `Subtracted ${m.nearest}·b₁ from b₂.`;
    this.render();
  }

  private swap(): void {
    this.s = { b1: this.s.b2, b2: this.s.b1 };
    this.steps++;
    this.host.sfx('blip');
    this.info.textContent = 'Swapped.';
    this.render();
  }

  private check(): void {
    if (this.host.solved()) return;
    if (isGaussReduced(this.s)) {
      this.host.sfx('unlock');
      this.host.solve(`Reduced in ${this.steps} moves. b₁ is now a shortest nonzero vector of the lattice. In high dimensions this exact loop, run pairwise, is LLL.`);
    } else {
      const m = mu2(this.s);
      this.info.innerHTML = n2(this.s.b1) > n2(this.s.b2) ? '<span style="color:var(--amber)">Not reduced: b₂ is shorter than b₁. Swap them.</span>' : `<span style="color:var(--amber)">Not reduced: |μ| = ${Math.abs(m.value).toFixed(2)} &gt; 0.5. Subtract round(μ)·b₁ from b₂.</span>`;
      this.host.sfx('error');
    }
  }

  private auto(): void {
    let guard = 0;
    while (!isGaussReduced(this.s) && guard++ < 50) {
      if (n2(this.s.b1) > n2(this.s.b2)) this.swap();
      else {
        const m = mu2(this.s);
        if (m.nearest === 0n) break;
        this.reduceStep();
      }
    }
    this.render();
    this.check();
  }

  async autoplay(): Promise<void> {
    this.auto();
  }
}
