import { commit, respond, simulate, verifyTranscript, type Transcript } from '../../crypto/sigma';
import { toHex } from '../../crypto/bytes';
import { N, compressed, mul } from '../../crypto/secp';
import { state } from '../../core/state';
import { h } from '../../ui/dom';
import { btn, card, kv, lede, math, note, shorten } from '../../ui/widgets';
import type { Puzzle, PuzzleHost } from '../../ui/terminal';
import { identity } from '../../story/world';

/**
 * Chapter 5, part 2. Why does the proof leak nothing? Because a transcript can
 * be *faked* without the secret, with exactly the same distribution. This is
 * the real-or-simulated game: you are shown valid transcripts, some from an
 * honest prover, some forged by the simulator, and you cannot do better than a
 * coin flip at telling them apart. If a transcript proved anything, you could.
 */
export class SimulatorGame implements Puzzle {
  id = 'ch5.sim';
  title = 'REAL OR SIMULATED?';
  subtitle = 'honest-verifier zero knowledge';
  accent = '#c792ff';
  private host!: PuzzleHost;
  private X = mul(identity().x);
  private total = 8;
  private round = 0;
  private correct = 0;
  private current!: { t: Transcript; real: boolean };
  private panel!: HTMLElement;
  private score!: HTMLElement;

  mount(host: PuzzleHost): void {
    this.host = host;
    this.panel = h('div');
    this.score = h('div', { class: 'note' });
    host.body.append(
      h(
        'div',
        { class: 'stack' },
        lede(
          'Here is why the checkpoint learned nothing. A <b>real</b> transcript comes from the honest protocol: pick r, get c, answer s = r + c·x. A <b>simulated</b> one is forged with no secret at all: pick c and s first, then set R = s·G − c·X. Both verify. Both have the same distribution. Can you tell which is which?',
        ),
        math('real: <i>R</i>=<i>r</i>·<i>G</i>, <i>s</i>=<i>r</i>+<i>c</i>·<i>x</i>     simulated: choose <i>c</i>,<i>s</i>, set <i>R</i>=<i>s</i>·<i>G</i>−<i>c</i>·<i>X</i>', true),
        card('Transcript', this.panel),
        this.score,
      ),
    );
    this.next();
    host.status('Guess real or simulated. Eight rounds.', 'info');
  }

  private next(): void {
    if (this.round >= this.total) {
      this.finish();
      return;
    }
    const rng = state.rng('ch5/sim-' + this.round + '-' + Math.random());
    const real = rng.int(2) === 0;
    let t: Transcript;
    if (real) {
      const k = commit(rng);
      const c = rng.scalar(N);
      t = { R: k.R, c, s: respond(k.r, c, identity().x) };
    } else {
      t = simulate(this.X, rng.scalar(N), rng.scalar(N));
    }
    this.current = { t, real };
    const valid = verifyTranscript(this.X, t);
    this.panel.replaceChildren(
      kv([
        ['R', h('span', { class: 'hex' }, shorten(toHex(compressed(t.R)), 12, 10))],
        ['c', h('span', { class: 'hex' }, shorten(t.c.toString(16).padStart(64, '0'), 12, 10))],
        ['s', h('span', { class: 'hex' }, shorten(t.s.toString(16).padStart(64, '0'), 12, 10))],
        ['s·G = R + c·X ?', valid ? h('span', { class: 'tag ok' }, 'VALID') : h('span', { class: 'tag bad' }, 'INVALID')],
      ]),
      note('Both kinds verify, so "valid" tells you nothing.'),
      h('div', { class: 'row', style: 'margin-top:10px' }, btn('Real (honest prover)', () => this.guess(true), 'primary', 'sim-real'), btn('Simulated (no secret)', () => this.guess(false), '', 'sim-fake')),
    );
    this.score.textContent = `Round ${this.round + 1} of ${this.total} · correct so far: ${this.correct}`;
  }

  private guess(real: boolean): void {
    const right = real === this.current.real;
    if (right) this.correct++;
    this.host.sfx(right ? 'confirm' : 'error');
    this.panel.append(h('div', { class: 'note', style: `margin-top:8px;color:${right ? 'var(--green)' : 'var(--red)'}` }, `It was ${this.current.real ? 'real' : 'simulated'}. You guessed ${right ? 'right' : 'wrong'}.`));
    this.round++;
    setTimeout(() => this.next(), 650);
  }

  private finish(): void {
    const pct = Math.round((this.correct / this.total) * 100);
    this.panel.replaceChildren(
      card(
        `You scored ${this.correct} / ${this.total} (${pct}%)`,
        note(
          `Random guessing is 50%. That is the point: there is <b>no</b> strategy that does better, because a simulator with no secret produces transcripts indistinguishable from the real ones. A transcript therefore proves, to anyone who was not the verifier, exactly nothing. That is what "zero knowledge" means.`,
        ),
      ),
    );
    this.host.sfx('unlock');
    this.host.solve('You could not beat a coin flip. Neither can the Directorate. The proofs leak nothing.');
  }

  async autoplay(): Promise<void> {
    // Answer each round correctly (we happen to know, since we built it), then
    // let the game reach its conclusion.
    this.correct = this.total;
    this.round = this.total;
    this.finish();
  }
}
