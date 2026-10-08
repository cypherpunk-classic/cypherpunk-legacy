import { TOY_LWE, ciphertextNoise, gaussianSolve, lweDecrypt, lweEncrypt, lweKeygen, lwePhase, type LweKeys } from '../../crypto/lwe';
import { kemDecapsulate, kemEncapsulate, kemKeygen, KEM_SIZES } from '../../crypto/pq';
import { toHex } from '../../crypto/bytes';
import { state } from '../../core/state';
import { h } from '../../ui/dom';
import { Log, btn, card, kv, lede, math, note, plotCanvas, shorten } from '../../ui/widgets';
import type { Puzzle, PuzzleHost } from '../../ui/terminal';

/**
 * Chapter 6, part 3. The other direction: building security on a lattice
 * problem instead of breaking one. Learning With Errors adds small noise to
 * linear equations; without noise, Gaussian elimination solves them instantly,
 * with noise, it is believed hard even for quantum computers. Then the real
 * thing: an ML-KEM-768 (FIPS 203) encapsulation, the standard the world is
 * actually migrating to.
 */
export class LweMlkem implements Puzzle {
  id = 'ch6.pq';
  title = 'AFTER THE QUANTUM';
  subtitle = 'LWE → ML-KEM-768 (FIPS 203)';
  accent = '#73b7ff';
  private host!: PuzzleHost;
  private noise = 1;
  private keys!: LweKeys;
  private canvas!: HTMLCanvasElement;
  private ctx!: CanvasRenderingContext2D;
  private cw = 0;
  private chh = 0;
  private lweBox!: HTMLElement;
  private kemBox!: HTMLElement;
  private log = new Log(150);
  private noiseRange!: HTMLInputElement;
  private kemDone = false;

  mount(host: PuzzleHost): void {
    this.host = host;
    this.rekey();
    const plot = plotCanvas(460, 150);
    this.canvas = plot.canvas;
    this.ctx = plot.ctx;
    this.cw = plot.w;
    this.chh = plot.h;
    this.lweBox = h('div');
    this.kemBox = h('div');
    this.noiseRange = h('input', { type: 'range', min: '0', max: '6', value: '1', style: 'flex:1', 'data-testid': 'lwe-noise' }) as HTMLInputElement;
    this.noiseRange.addEventListener('input', () => {
      this.noise = Number(this.noiseRange.value);
      this.rekey();
      this.renderLwe();
    });

    host.body.append(
      h(
        'div',
        { class: 'stack' },
        lede(
          'Everything so far, Diffie–Hellman, Schnorr, your own keys, breaks the day a large quantum computer runs Shor\'s algorithm. Traffic recorded today can be decrypted then: harvest now, decrypt later. So the ground is shifting to problems quantum computers do not help with. Lattices.',
        ),
        math('LWE: given <i>A</i>, <i>b</i> = <i>A·s</i> + <i>e</i> (small <i>e</i>), find <i>s</i>.   No noise → Gaussian elimination. Small noise → believed hard.', true),
        card(
          'Learning With Errors (toy: n=4, q=97)',
          h('div', { class: 'row' }, h('span', { class: 'note' }, 'error size:'), this.noiseRange, h('span', { class: 'note', 'data-testid': 'noise-label' }, '')),
          h('div', { class: 'grid2', style: 'margin-top:8px' }, this.lweBox, h('div', null, note('phase of each ciphertext bit (distance from 0 or q/2 must stay clear):'), this.canvas)),
        ),
        card(
          'The standard: ML-KEM-768',
          note('Built on module-LWE, standardised as FIPS 203 in August 2024 (from CRYSTALS-Kyber). Browsers and Signal already use hybrid key exchange with it. Run a real encapsulation.'),
          h('div', { class: 'row', style: 'margin-top:8px' }, btn('Generate keypair + encapsulate', () => this.runKem(), 'primary', 'kem-run')),
          h('div', { style: 'margin-top:8px' }, this.kemBox),
        ),
        this.log.el,
      ),
    );
    this.renderLwe();
    host.status('Push the noise up and down. See where decryption breaks.', 'info');
  }

  private rekey(): void {
    this.keys = lweKeygen({ ...TOY_LWE, noise: this.noise }, state.rng('ch6/lwe-' + this.noise));
  }

  private renderLwe(): void {
    const label = this.host.body.querySelector('[data-testid=noise-label]');
    if (label) label.textContent = this.noise === 0 ? 'none (insecure)' : `±${this.noise}`;
    // encrypt a known bitstring and try to decrypt
    const rng = state.rng('ch6/lwe-enc-' + this.noise);
    const bits = [1, 0, 1, 1, 0, 0, 1, 0];
    let correct = 0;
    const cts = bits.map((b) => lweEncrypt(this.keys, b as 0 | 1, rng));
    const got = cts.map((ct, i) => {
      const d = lweDecrypt(this.keys, ct);
      if (d === bits[i]) correct++;
      return d;
    });
    // attempt to break by Gaussian elimination (ignores noise)
    const guess = gaussianSolve(this.keys.A, this.keys.b, this.keys.params.q);
    const sExact = guess && guess.every((v, i) => v === this.keys.s[i]);
    this.lweBox.replaceChildren(
      kv([
        ['secret s', h('span', { class: 'hex' }, `[${this.keys.s.join(', ')}]`)],
        ['sent bits', h('span', { class: 'hex' }, bits.join(''))],
        ['decrypted', h('span', { class: `hex ${correct === bits.length ? 'green' : 'red'}` }, got.join(''))],
        ['ciphertext noise', h('span', { class: 'hex' }, `up to ±${Math.max(...cts.map((c) => Math.abs(ciphertextNoise(this.keys, c))))}  (fails past q/4 ≈ 24)`)],
        ['Gaussian elimination', sExact ? h('span', { class: 'tag bad' }, 'RECOVERS s') : h('span', { class: 'tag ok' }, 'returns garbage')],
      ]),
      note(this.noise === 0 ? 'With no noise, the equations are exact and s falls out instantly. That is not encryption; that is linear algebra.' : 'The same small noise that is a nuisance for decryption is what makes recovering s hard.'),
    );
    this.drawPhases(cts.map((c) => lwePhase(this.keys, c)), bits);
  }

  private drawPhases(phases: number[], bits: number[]): void {
    const ctx = this.ctx;
    const q = this.keys.params.q;
    ctx.fillStyle = '#070b16';
    ctx.fillRect(0, 0, this.cw, this.chh);
    const cx = (p: number): number => 20 + (p / q) * (this.cw - 40);
    // zones
    ctx.fillStyle = 'rgba(93,255,154,0.08)';
    ctx.fillRect(cx(0) - 10, 10, 20 + ((q / 4) / q) * (this.cw - 40), this.chh - 30);
    ctx.fillRect(cx(q) - ((q / 4) / q) * (this.cw - 40), 10, ((q / 4) / q) * (this.cw - 40) + 10, this.chh - 30);
    ctx.fillStyle = 'rgba(255,62,154,0.08)';
    ctx.fillRect(cx(q / 2) - ((q / 4) / q) * (this.cw - 40), 10, ((q / 2) / q) * (this.cw - 40), this.chh - 30);
    ctx.strokeStyle = '#17203a';
    for (const p of [0, q / 2, q]) {
      ctx.beginPath();
      ctx.moveTo(cx(p), 10);
      ctx.lineTo(cx(p), this.chh - 20);
      ctx.stroke();
    }
    ctx.fillStyle = '#56627d';
    ctx.font = '10px monospace';
    ctx.fillText('0 → bit 0', cx(0), this.chh - 6);
    ctx.fillText('q/2 → bit 1', cx(q / 2) - 20, this.chh - 6);
    phases.forEach((p, i) => {
      ctx.fillStyle = bits[i] ? '#ff3e9a' : '#5dff9a';
      ctx.beginPath();
      ctx.arc(cx(p), this.chh / 2 - 6, 4, 0, Math.PI * 2);
      ctx.fill();
    });
  }

  private runKem(): void {
    const kp = kemKeygen();
    const { cipherText, sharedSecret } = kemEncapsulate(kp.publicKey);
    const recovered = kemDecapsulate(cipherText, kp.secretKey);
    const match = toHex(recovered) === toHex(sharedSecret);
    this.kemBox.replaceChildren(
      kv([
        ['public key', `${kp.publicKey.length} bytes  (${shorten(toHex(kp.publicKey), 8, 6)})`],
        ['ciphertext', `${cipherText.length} bytes`],
        ['shared secret', h('span', { class: 'hex green' }, toHex(sharedSecret))],
        ['decapsulated', h('span', { class: `hex ${match ? 'green' : 'red'}` }, shorten(toHex(recovered), 10, 8))],
        ['agree', match ? h('span', { class: 'tag ok' }, 'YES') : h('span', { class: 'tag bad' }, 'NO')],
      ]),
      note(`ML-KEM-768 sizes: pk ${KEM_SIZES.publicKey} B, ct ${KEM_SIZES.cipherText} B, secret ${KEM_SIZES.sharedSecret} B. Bigger than a 32-byte curve key, but it survives Shor.`),
    );
    this.log.line('ML-KEM-768 encapsulation: both sides share the same 32-byte secret.', 'ok');
    this.kemDone = true;
    this.host.sfx('unlock');
    if (match) this.host.solve('A real post-quantum key agreement, computed in your browser. This is the ground the next generation stands on.');
  }

  async autoplay(): Promise<void> {
    this.runKem();
    void this.kemDone;
  }
}
