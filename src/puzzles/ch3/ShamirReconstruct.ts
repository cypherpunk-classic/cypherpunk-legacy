import { TOY_P, curveForGuess, evalPoly, quadraticThrough } from '../../crypto/shamir';
import { h } from '../../ui/dom';
import { btn, card, lede, math, note, plotCanvas, typeValue } from '../../ui/widgets';
import type { Puzzle, PuzzleHost } from '../../ui/terminal';
import { state } from '../../core/state';

/**
 * Chapter 3, part 1. Shamir sharing in the small field GF(31), drawn on a
 * grid you can see. Two shares leave the secret f(0) completely undetermined:
 * every candidate secret has its own curve through the two points. Collect the
 * third share and the parabola, and the secret, snap into place.
 */
export class ShamirReconstruct implements Puzzle {
  id = 'ch3.shamir';
  title = "JUNO'S WORKSHOP · GF(31)";
  subtitle = 'threshold 3 of 5';
  accent = '#5dff9a';
  private host!: PuzzleHost;
  private coeffs!: bigint[];
  private shares: [bigint, bigint][] = [];
  private known: [bigint, bigint][] = [];
  private haveThird = false;
  private guessIn!: HTMLInputElement;
  private canvas!: HTMLCanvasElement;
  private ctx!: CanvasRenderingContext2D;
  private cw = 0;
  private ch = 0;
  private status2!: HTMLElement;
  private thirdBtn!: HTMLButtonElement;

  mount(host: PuzzleHost): void {
    this.host = host;
    const r = state.rng('ch3/toy');
    const secret = r.range(5n, 25n);
    const a = r.range(1n, 3n);
    const b = r.range(1n, 10n);
    this.coeffs = [secret, b, a];
    for (const x of [1n, 2n, 3n, 4n]) this.shares.push([x, evalPoly(this.coeffs, x, TOY_P)]);
    this.known = [this.shares[0]!, this.shares[1]!]; // shares at x=1, x=2

    const plot = plotCanvas(460, 360);
    this.canvas = plot.canvas;
    this.ctx = plot.ctx;
    this.cw = plot.w;
    this.ch = plot.h;
    this.guessIn = h('input', { type: 'number', min: '0', max: '30', value: '0', style: 'width:90px', 'data-testid': 'shamir-guess' }) as HTMLInputElement;
    this.guessIn.addEventListener('input', () => this.draw());
    this.status2 = h('div', { class: 'note' });
    this.thirdBtn = btn('Add Juno\'s share (x = 3)', () => this.addThird(), '', 'shamir-third');

    host.body.append(
      h(
        'div',
        { class: 'grid2' },
        h(
          'div',
          { class: 'stack' },
          lede(
            'A secret is hidden as <b>f(0)</b> of a degree-2 polynomial over the field GF(31). You were handed two shares, Mara\'s and Felix\'s. The dot you are dragging is a guess for the secret.',
          ),
          math('<i>f</i>(<i>x</i>) = <i>a</i>₂<i>x</i>² + <i>a</i>₁<i>x</i> + <i>s</i>   (mod 31),   secret = <i>f</i>(0)', true),
          card(
            'Known shares',
            h('div', { class: 'kv' }, h('div', { class: 'k' }, 'Mara (x=1)'), h('div', { class: 'v hex green' }, `y = ${this.known[0]![1]}`), h('div', { class: 'k' }, 'Felix (x=2)'), h('div', { class: 'v hex green' }, `y = ${this.known[1]![1]}`)),
          ),
          card(
            'Guess the secret',
            h('div', { class: 'row' }, h('span', { class: 'note' }, 'secret f(0) ='), this.guessIn),
            note('With only two shares, <b>every</b> value of f(0) from 0 to 30 gives a valid parabola through both points. Two shares reveal nothing: that is the whole idea.'),
            h('div', { class: 'row', style: 'margin-top:8px' }, this.thirdBtn),
            h('div', { style: 'margin-top:6px' }, this.status2),
            h('div', { class: 'row', style: 'margin-top:8px' }, btn('Lock in the secret', () => this.check(), 'primary', 'shamir-check')),
          ),
        ),
        h('div', null, this.canvas),
      ),
    );
    this.draw();
    host.status('Two shares: the secret could be anything. Slide the guess and watch.', 'info');
  }

  private px(x: number): number {
    return 40 + (x / 5) * (this.cw - 60);
  }
  private py(y: number): number {
    return this.ch - 30 - (y / 31) * (this.ch - 50);
  }

  private draw(): void {
    const ctx = this.ctx;
    ctx.clearRect(0, 0, this.cw, this.ch);
    ctx.fillStyle = '#070b16';
    ctx.fillRect(0, 0, this.cw, this.ch);
    // grid
    ctx.strokeStyle = '#17203a';
    ctx.lineWidth = 1;
    for (let x = 0; x <= 5; x++) {
      ctx.beginPath();
      ctx.moveTo(this.px(x), this.py(0));
      ctx.lineTo(this.px(x), this.py(31));
      ctx.stroke();
    }
    for (let y = 0; y <= 30; y += 5) {
      ctx.beginPath();
      ctx.moveTo(this.px(0), this.py(y));
      ctx.lineTo(this.px(5), this.py(y));
      ctx.stroke();
    }
    // axes labels
    ctx.fillStyle = '#56627d';
    ctx.font = '11px monospace';
    ctx.fillText('f(0) = secret', this.px(0) - 6, this.py(31) - 8);
    for (let x = 0; x <= 4; x++) ctx.fillText(`x=${x}`, this.px(x) - 8, this.ch - 12);
    // the y-axis (x=0) highlighted
    ctx.strokeStyle = '#ffb347';
    ctx.setLineDash([3, 3]);
    ctx.beginPath();
    ctx.moveTo(this.px(0), this.py(0));
    ctx.lineTo(this.px(0), this.py(31));
    ctx.stroke();
    ctx.setLineDash([]);

    const guess = BigInt(Math.max(0, Math.min(30, Math.floor(Number(this.guessIn.value) || 0))));

    if (!this.haveThird) {
      // draw several candidate curves faintly
      for (let s = 0; s <= 30; s += 3) {
        try {
          const c = curveForGuess(BigInt(s), this.known);
          this.plotCurve(c, 'rgba(90,110,150,0.25)');
        } catch {
          /* singular, skip */
        }
      }
      try {
        this.plotCurve(curveForGuess(guess, this.known), '#3ef0ff');
      } catch {
        /* skip */
      }
    } else {
      this.plotCurve(this.coeffs, '#5dff9a');
    }

    // known points
    for (const [x, y] of this.known) this.dot(Number(x), Number(y), '#5dff9a', 5);
    if (this.haveThird) this.dot(3, Number(this.shares[2]![1]), '#5dff9a', 5);
    // the guessed/true secret on the y-axis
    const secretVal = this.haveThird ? Number(this.coeffs[0]) : Number(guess);
    this.dot(0, secretVal, this.haveThird ? '#5dff9a' : '#3ef0ff', 6, !this.haveThird);
    ctx.fillStyle = this.haveThird ? '#5dff9a' : '#3ef0ff';
    ctx.font = 'bold 13px monospace';
    ctx.fillText(`${secretVal}`, this.px(0) - 28, this.py(secretVal) + 4);
  }

  private plotCurve(coeffs: bigint[], color: string): void {
    // A polynomial over GF(31) is only defined at integer x; connect the
    // integer points with straight segments so the shape is readable.
    const ctx = this.ctx;
    ctx.strokeStyle = color;
    ctx.lineWidth = color === '#3ef0ff' || color === '#5dff9a' ? 2 : 1;
    ctx.beginPath();
    for (let x = 0; x <= 5; x++) {
      const y = Number(evalPoly(coeffs, BigInt(x), TOY_P));
      if (x === 0) ctx.moveTo(this.px(x), this.py(y));
      else ctx.lineTo(this.px(x), this.py(y));
    }
    ctx.stroke();
  }

  private dot(x: number, y: number, color: string, r: number, ring = false): void {
    const ctx = this.ctx;
    ctx.fillStyle = color;
    ctx.beginPath();
    ctx.arc(this.px(x), this.py(y), r, 0, Math.PI * 2);
    ctx.fill();
    if (ring) {
      ctx.strokeStyle = color;
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      ctx.arc(this.px(x), this.py(y), r + 4, 0, Math.PI * 2);
      ctx.stroke();
    }
  }

  private addThird(): void {
    this.haveThird = true;
    this.thirdBtn.disabled = true;
    this.guessIn.disabled = true;
    const [x3, y3] = this.shares[2]!;
    const rebuilt = quadraticThrough([this.known[0]!, this.known[1]!, [x3, y3]]);
    typeValue(this.guessIn, String(rebuilt[0]));
    this.status2.innerHTML = `Three points fix one parabola. Interpolating at x = 0 gives the secret: <b style="color:var(--green)">f(0) = ${rebuilt[0]}</b>.`;
    this.host.sfx('confirm');
    this.draw();
    this.host.status('Now lock it in.', 'info');
  }

  private check(): void {
    if (this.host.solved()) return;
    const guess = BigInt(Math.floor(Number(this.guessIn.value) || -1));
    if (!this.haveThird) {
      this.host.status('With two shares you are only guessing. Collect the third share first.', 'warn');
      this.host.sfx('error');
      return;
    }
    if (guess !== this.coeffs[0]) {
      this.host.status('That is not the secret. Let the interpolation compute it.', 'bad');
      this.host.sfx('error');
      return;
    }
    this.host.sfx('unlock');
    this.host.solve(`Secret recovered: ${this.coeffs[0]}. Three of five shares, and not one fewer.`);
  }

  async autoplay(): Promise<void> {
    this.addThird();
    this.check();
  }
}
