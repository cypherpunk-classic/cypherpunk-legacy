import { extract, respond, verifyTranscript, fsChallenge, fsVerify, type Transcript } from '../../crypto/sigma';
import { toHex } from '../../crypto/bytes';
import { N, compressed, mul, xOnly } from '../../crypto/secp';
import { h } from '../../ui/dom';
import { Log, btn, card, kv, lede, math, note, shorten } from '../../ui/widgets';
import type { Puzzle, PuzzleHost } from '../../ui/terminal';
import { checkpointKey } from '../../story/world';

/**
 * Chapter 5, part 3. The checkpoint's own prover is careless: it answers two
 * different challenges using the same commitment R. Two accepting transcripts
 * (R, c1, s1), (R, c2, s2) yield x = (s1 − s2)/(c1 − c2): the extractor, which
 * is also the soundness proof. Then Fiat–Shamir turns the interactive proof
 * into a signature, and the "weak" version shows why the hash must bind the
 * whole statement.
 */
export class ExtractorFS implements Puzzle {
  id = 'ch5.extract';
  title = 'REWIND THE WARDEN';
  subtitle = 'special soundness → extractor → Fiat–Shamir';
  accent = '#c792ff';
  private host!: PuzzleHost;
  private x = checkpointKey().x;
  private X = mul(checkpointKey().x);
  private r = checkpointKey().r;
  private R = mul(checkpointKey().r);
  private c1 = 7n;
  private c2 = 0n;
  private extracted: bigint | null = null;
  private log = new Log(170);
  private step2!: HTMLElement;
  private step3!: HTMLElement;

  mount(host: PuzzleHost): void {
    this.host = host;
    this.c2 = 13n + (checkpointKey().x % 9973n);
    this.step2 = h('div');
    this.step3 = h('div');
    const t1: Transcript = { R: this.R, c: this.c1, s: respond(this.r, this.c1, this.x) };
    const t2: Transcript = { R: this.R, c: this.c2, s: respond(this.r, this.c2, this.x) };

    host.body.append(
      h(
        'div',
        { class: 'stack' },
        lede(
          'The checkpoint\'s master prover has a flaw: when you challenge it twice, it answers from the <b>same</b> commitment R. That is exactly the situation the soundness proof exploits. Two answers to two challenges reveal the witness.',
        ),
        card(
          'Two transcripts, one commitment',
          kv([
            ['R (shared)', h('span', { class: 'hex amber' }, shorten(toHex(compressed(this.R)), 10, 8))],
            ['c₁, s₁', h('span', { class: 'hex' }, `${this.c1} , ${shorten(t1.s.toString(16), 8, 6)}`)],
            ['c₂, s₂', h('span', { class: 'hex' }, `${this.c2} , ${shorten(t2.s.toString(16), 8, 6)}`)],
            ['both verify', verifyTranscript(this.X, t1) && verifyTranscript(this.X, t2) ? h('span', { class: 'tag ok' }, 'YES') : h('span', { class: 'tag bad' }, 'NO')],
          ]),
          math('<i>s</i>₁ = <i>r</i> + <i>c</i>₁<i>x</i>,  <i>s</i>₂ = <i>r</i> + <i>c</i>₂<i>x</i>   ⟹   <i>x</i> = (<i>s</i>₁ − <i>s</i>₂)/(<i>c</i>₁ − <i>c</i>₂)', true),
          h('div', { class: 'row', style: 'margin-top:8px' }, btn('Extract the witness x', () => this.doExtract(t1, t2), 'primary', 'extract-run')),
          h('div', { style: 'margin-top:8px' }, this.step2),
        ),
        card('Make it non-interactive (Fiat–Shamir)', note('With x, you can produce proofs nobody needs to challenge: let the hash be the challenge, c = H(R ‖ X ‖ context). That is a Schnorr signature. But the hash must include everything.'), this.step3),
        card('Transcript', this.log.el),
      ),
    );
    this.log.line('Prover reused commitment R across challenges c₁ and c₂.', 'warn');
    host.status('Rewind the prover: extract its key.', 'info');
  }

  private doExtract(t1: Transcript, t2: Transcript): void {
    this.extracted = extract(t1, t2);
    const ok = this.extracted === this.x;
    this.step2.replaceChildren(
      kv([['extracted x', h('span', { class: 'hex big green' }, shorten(this.extracted.toString(16).padStart(64, '0'), 12, 10))], ['x·G == X ?', ok ? h('span', { class: 'tag ok' }, 'YES') : h('span', { class: 'tag bad' }, 'NO')]]),
      note('This <em>is</em> the soundness proof: anyone who can answer two challenges for one commitment must know x. It is also the nonce-reuse attack from Chapter 4, wearing different notation.'),
    );
    this.renderFS();
    this.host.sfx('unlock');
    this.host.status('Key extracted. Now forge a non-interactive proof.', 'ok');
  }

  private renderFS(): void {
    if (this.extracted === null) return;
    const ctxIn = h('input', { type: 'text', value: 'checkpoint-7|clearance=all', style: 'width:100%', 'data-testid': 'fs-context' }) as HTMLInputElement;
    const weak = h('input', { type: 'checkbox' }) as HTMLInputElement;
    const out = h('div');
    this.step3.replaceChildren(
      h('div', { class: 'row' }, h('span', { class: 'note' }, 'context:'), ctxIn),
      h('label', { class: 'row note', style: 'cursor:pointer;margin-top:6px' }, weak, 'use weak Fiat–Shamir (hash omits the context)'),
      h('div', { class: 'row', style: 'margin-top:8px' }, btn('Forge non-interactive proof', () => this.forge(ctxIn.value, weak.checked, out), 'primary', 'fs-forge')),
      out,
    );
  }

  private forge(context: string, weakFS: boolean, out: HTMLElement): void {
    if (this.host.solved()) return;
    const x = this.extracted!;
    const r = N - 12345n + BigInt(Math.floor(Math.random() * 1e9));
    const R = mul(r);
    const opts = { includeKey: true, includeContext: !weakFS };
    const c = fsChallenge(R, this.X, context, opts);
    const s = respond(r, c, x);
    const t: Transcript = { R, c, s };
    const ok = fsVerify(this.X, context, t, opts);
    // Demonstrate the replay bug: a proof made for one context verifies for another.
    const replayOk = !weakFS ? false : fsVerify(this.X, context + '|DIFFERENT', t, opts);
    out.replaceChildren(
      kv([
        ['c = H(R‖X' + (weakFS ? '' : '‖ctx') + ')', h('span', { class: 'hex' }, shorten(c.toString(16).padStart(64, '0'), 10, 8))],
        ['proof (R.x, s)', h('span', { class: 'hex green' }, `${shorten(toHex(xOnly(R)), 8, 6)} , ${shorten(s.toString(16), 8, 6)}`)],
        ['verifies for this context', ok ? h('span', { class: 'tag ok' }, 'YES') : h('span', { class: 'tag bad' }, 'NO')],
      ]),
      weakFS
        ? h('div', { class: 'card bad', style: 'margin-top:8px' }, note(`Weak Fiat–Shamir: the same proof also verifies for a <em>different</em> context (${replayOk ? 'YES — replayable!' : 'no'}). Leaving the context out of the hash means a clearance for one gate works at every gate. This is the 2022 "Frozen Heart" class of bugs.`))
        : note('Strong Fiat–Shamir: the hash binds R, the key, and the context, so this proof is good for this context and no other.'),
    );
    this.host.sfx(weakFS ? 'error' : 'confirm');
    if (!weakFS && ok) {
      this.host.solve('A sound, non-interactive proof of clearance, bound to its context. You can pass every checkpoint now, and tell them nothing.');
    } else if (weakFS) {
      this.host.status('See the danger, then switch off weak mode to finish.', 'warn');
    }
  }

  async autoplay(): Promise<void> {
    const t1: Transcript = { R: this.R, c: this.c1, s: respond(this.r, this.c1, this.x) };
    const t2: Transcript = { R: this.R, c: this.c2, s: respond(this.r, this.c2, this.x) };
    this.doExtract(t1, t2);
    const forgeBtn = this.step3.querySelector('[data-testid=fs-forge]') as HTMLButtonElement | null;
    forgeBtn?.click();
  }
}
