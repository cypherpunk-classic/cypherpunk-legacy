import { commit, respond, verifyTranscript, cheaterCommit } from '../../crypto/sigma';
import { toHex } from '../../crypto/bytes';
import { N, compressed, mul } from '../../crypto/secp';
import { state } from '../../core/state';
import { h } from '../../ui/dom';
import { Log, btn, card, kv, lede, math, note, shorten } from '../../ui/widgets';
import type { Puzzle, PuzzleHost } from '../../ui/terminal';
import { identity } from '../../story/world';

/**
 * Chapter 5, part 1. The Directorate checkpoint wants to see your key. Instead
 * you prove you *hold* it, revealing nothing, with the Schnorr identification
 * protocol: commit R = r·G, receive challenge c, answer s = r + c·x; the
 * verifier checks s·G = R + c·X. A full-size challenge makes guessing hopeless;
 * a one-bit challenge lets a liar through half the time, which the puzzle shows.
 */
export class SigmaCheckpoint implements Puzzle {
  id = 'ch5.sigma';
  title = 'CHECKPOINT · IDENTIFY YOURSELF';
  subtitle = 'Schnorr identification (honest prover)';
  accent = '#c792ff';
  private host!: PuzzleHost;
  private x = identity().x;
  private X = mul(identity().x);
  private r = 0n;
  private R = mul(0n);
  private c = 0n;
  private round = 0;
  private needed = 3;
  private passed = 0;
  private panel!: HTMLElement;
  private log = new Log(190);

  mount(host: PuzzleHost): void {
    this.host = host;
    this.panel = h('div');
    host.body.append(
      h(
        'div',
        { class: 'stack' },
        lede(
          'The checkpoint has your public key X on file. The old way: hand over your private key so it can check. You will not. Instead you prove you know x such that X = x·G, and the checkpoint learns nothing else.',
        ),
        math('commit <i>R</i> = <i>r</i>·<i>G</i>  →  challenge <i>c</i>  →  respond <i>s</i> = <i>r</i> + <i>c</i>·<i>x</i>.   Accept iff <i>s</i>·<i>G</i> = <i>R</i> + <i>c</i>·<i>X</i>', true),
        card('Public', kv([['your public key X', h('span', { class: 'hex' }, shorten(toHex(compressed(this.X)), 10, 8))], ['your secret x', h('span', { class: 'hex red' }, 'never transmitted')]])),
        card(`Round ${this.round + 1} of ${this.needed}`, this.panel),
        card('Transcript', this.log.el),
      ),
    );
    this.renderCommit();
    host.status('Pass three rounds to clear the checkpoint.', 'info');
  }

  private panelTitle(): HTMLElement | null {
    const card = this.panel.closest('.card');
    const title = card?.querySelector('h3');
    if (title) title.textContent = `Round ${this.round + 1} of ${this.needed}`;
    return null;
  }

  private renderCommit(): void {
    this.panelTitle();
    this.panel.replaceChildren(
      note('Pick a fresh random nonce r and send its commitment R = r·G. (Never reuse r: that is the whole of Chapter 4.)'),
      h('div', { class: 'row', style: 'margin-top:8px' }, btn('Generate nonce and commit R', () => this.doCommit(), 'primary', 'sigma-commit')),
    );
  }

  private doCommit(): void {
    const k = commit(state.rng('ch5/sigma-' + this.round + '-' + Math.random()));
    this.r = k.r;
    this.R = k.R;
    this.c = state.rng('ch5/chal-' + this.round + '-' + Date.now()).scalar(N);
    this.log.line(`round ${this.round + 1}: R = ${shorten(toHex(compressed(this.R)), 8, 6)}`, 'dim');
    this.log.line(`checkpoint challenges c = ${shorten(this.c.toString(16), 8, 6)}`, 'info');
    this.panel.replaceChildren(
      kv([['your R', h('span', { class: 'hex' }, shorten(toHex(compressed(this.R)), 10, 8))], ['challenge c', h('span', { class: 'hex amber' }, shorten(this.c.toString(16).padStart(64, '0'), 10, 8))]]),
      note('Answer with s = r + c·x. The checkpoint will verify s·G = R + c·X.'),
      h('div', { class: 'row', style: 'margin-top:8px' }, btn('Respond s = r + c·x', () => this.doRespond(true), 'primary', 'sigma-respond'), btn('Try to bluff (random s)', () => this.doRespond(false), 'ghost', 'sigma-bluff')),
    );
    this.host.sfx('blip');
  }

  private doRespond(honest: boolean): void {
    const s = honest ? respond(this.r, this.c, this.x) : state.rng('bluff-' + Math.random()).scalar(N);
    const ok = verifyTranscript(this.X, { R: this.R, c: this.c, s });
    this.log.line(`s = ${shorten(s.toString(16), 8, 6)} · verify s·G = R + c·X → ${ok ? 'PASS' : 'FAIL'}`, ok ? 'ok' : 'bad');
    if (!ok) {
      this.host.sfx('error');
      this.passed = 0;
      this.host.status('Verification failed. A bluff against a full-size challenge has no chance. Start over.', 'bad');
      this.round = 0;
      this.renderCommit();
      this.panelTitle();
      return;
    }
    this.host.sfx('confirm');
    this.passed++;
    this.round++;
    if (this.passed >= this.needed) {
      this.host.solve('Checkpoint cleared. It is convinced you hold the key, and it has learned nothing about it.');
      return;
    }
    this.host.status(`Round passed (${this.passed}/${this.needed}). The checkpoint still knows nothing about x.`, 'ok');
    this.renderCommit();
    this.panelTitle();
  }

  async autoplay(): Promise<void> {
    for (let i = 0; i < this.needed; i++) {
      this.doCommit();
      this.doRespond(true);
    }
    void cheaterCommit;
  }
}
