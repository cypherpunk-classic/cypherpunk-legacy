import { keyAgg, keyAggCoeff, naiveAggregate, rogueKey } from '../../crypto/musig';
import { sign, verify } from '../../crypto/bip340';
import { toHex, utf8 } from '../../crypto/bytes';
import { compressed, mul, xOnly } from '../../crypto/secp';
import { state } from '../../core/state';
import { h } from '../../ui/dom';
import { Log, btn, card, kv, lede, math, note, shorten, tag } from '../../ui/widgets';
import type { Puzzle, PuzzleHost } from '../../ui/terminal';
import { identity } from '../../story/world';

/**
 * Chapter 4, part 2. The Directorate's "joint custody" offers a 2-of-2 escrow:
 * your key and theirs, so neither can act alone. With naive key aggregation
 * (Q = P1 + P2) they announce a rogue key and seize sole control. BIP-327
 * MuSig2 KeyAgg, which weights each key by a hash of the whole list, stops it.
 */
export class RogueKey implements Puzzle {
  id = 'ch4.rogue';
  title = 'JOINT CUSTODY · KEY AGGREGATION';
  subtitle = 'rogue key vs BIP-327 MuSig2';
  accent = '#ffb347';
  private host!: PuzzleHost;
  private you = identity();
  private dirSecret = 0n;
  private stage: 'naive' | 'musig' = 'naive';
  private panel!: HTMLElement;
  private log = new Log(180);

  mount(host: PuzzleHost): void {
    this.host = host;
    this.dirSecret = state.rng('ch4/dir').scalar(1n << 250n);
    this.panel = h('div');
    host.body.append(
      h(
        'div',
        { class: 'stack' },
        lede(
          'The Directorate offers "joint custody": a 2-of-2 address built from your key and theirs, so, they say, neither side can move anything alone. You sign a contract by publishing your key; they publish theirs. The contract key is the two combined.',
        ),
        card('The deal', this.panel),
        card('Record', this.log.el),
      ),
    );
    this.render();
  }

  private render(): void {
    const yourX = this.you.xonly;
    if (this.stage === 'naive') {
      this.panel.replaceChildren(
        note('Naive aggregation just adds the keys:'),
        math('<i>Q</i> = <i>P</i><sub>you</sub> + <i>P</i><sub>DLA</sub>', true),
        kv([
          ['your key P_you', h('span', { class: 'hex' }, shorten(toHex(yourX), 10, 8))],
          ['you expect', 'a key only the two of you can spend from'],
        ]),
        note('You publish first (you always have to). Now the Directorate announces its key. Watch what they can choose.'),
        h('div', { class: 'row', style: 'margin-top:8px' }, btn('Let the Directorate announce honestly', () => this.naive(false), '', 'rogue-honest'), btn('…or let them play a rogue key', () => this.naive(true), 'primary', 'rogue-attack')),
      );
      this.host.status('See how naive aggregation fails.', 'info');
    } else {
      this.panel.replaceChildren(
        note('MuSig2 (BIP-327) weights each key by a coefficient that depends on the whole key list, so no one can cancel the others:'),
        math('<i>a</i><sub>i</sub> = H(<i>L</i> ‖ <i>P</i><sub>i</sub>),   <i>L</i> = H(<i>P</i>₁ ‖ … ‖ <i>P</i><sub>u</sub>),   <i>Q</i> = <i>a</i>₁<i>P</i>₁ + <i>a</i>₂<i>P</i>₂', true),
        h('div', { class: 'row', style: 'margin-top:8px' }, btn('Let the Directorate try the rogue key again', () => this.musig(), 'primary', 'rogue-musig')),
      );
      this.host.status('Now aggregate the same keys with MuSig2.', 'info');
    }
  }

  private naive(attack: boolean): void {
    const youP = mul(this.you.x);
    let dlaPub;
    if (attack) {
      dlaPub = rogueKey(mul(this.dirSecret), [youP]); // P_dla = T - P_you
      this.log.line('Directorate announces P_DLA = (their target key) − P_you.', 'warn');
    } else {
      dlaPub = mul(this.dirSecret);
      this.log.line('Directorate announces an honest P_DLA = d_DLA·G.', 'dim');
    }
    const Q = naiveAggregate([youP, dlaPub]);
    this.log.line(`Contract key Q = P_you + P_DLA = ${shorten(toHex(xOnly(Q)), 8, 6)}`, 'info');
    if (attack) {
      // The attacker can now sign for Q alone: Q = target = dirSecret·G
      const canSpendAlone = Q.equals(mul(this.dirSecret));
      const m = utf8('MOVE EVERYTHING TO THE DIRECTORATE');
      const soloSig = sign(m, this.dirSecret);
      const accepted = verify(soloSig, m, xOnly(Q));
      this.log.line(`Q == (their own key alone)? ${canSpendAlone ? 'YES' : 'no'}`, canSpendAlone ? 'bad' : 'ok');
      this.log.line(`Directorate signs alone, no key of yours involved… contract accepts? ${accepted ? 'YES' : 'no'}`, accepted ? 'bad' : 'ok');
      this.panel.append(
        h('div', { class: 'card bad', style: 'margin-top:10px' }, h('b', { style: 'color:var(--red)' }, 'Rogue-key attack succeeded.'), note('They chose P_DLA = T − P_you, so Q = T, a key they alone hold. "Joint custody" is sole Directorate custody. Now do it properly.')),
        h('div', { class: 'row', style: 'margin-top:8px' }, btn('Switch to MuSig2 →', () => ((this.stage = 'musig'), this.render()), 'primary', 'rogue-to-musig')),
      );
      this.host.sfx('error');
      this.host.status('They took sole control. MuSig2 is the fix.', 'bad');
    } else {
      this.log.line('Honestly, Q needs both keys. But nothing forces them to be honest.', 'ok');
      this.panel.append(note('That only worked because they chose to play fair. A real attacker will not. Make them.'), h('div', { class: 'row', style: 'margin-top:8px' }, btn('Replay, with the rogue key', () => this.naive(true), 'primary', 'rogue-attack2')));
    }
  }

  private musig(): void {
    if (this.host.solved()) return;
    const youP = mul(this.you.x);
    const target = mul(this.dirSecret);
    // They try the same trick: announce T - P_you
    const rogue = rogueKey(target, [youP]);
    const pks = [compressed(youP), compressed(rogue)];
    const agg = keyAgg(pks);
    const a1 = keyAggCoeff(pks, pks[0]!);
    const a2 = keyAggCoeff(pks, pks[1]!);
    const hitTarget = agg.Q.equals(target);
    this.log.line(`MuSig2 coefficients: a_you = ${shorten(a1.toString(16), 6, 4)}, a_DLA = ${shorten(a2.toString(16), 6, 4)}`, 'info');
    this.log.line(`Q = a_you·P_you + a_DLA·P_DLA = ${shorten(toHex(xOnly(agg.Q)), 8, 6)}`, 'info');
    this.log.line(`Q equals their target key? ${hitTarget ? 'YES' : 'NO'}`, hitTarget ? 'bad' : 'ok');
    this.panel.append(
      h('div', { class: 'card good', style: 'margin-top:10px' }, h('b', { style: 'color:var(--green)' }, 'Rogue key defeated.'), note('Their rogue P_DLA was chosen to cancel P_you under addition. But MuSig2 multiplies each key by a_i first, and a_i depends on the full key list, so they would have to know the coefficients before choosing a key that depends on them. They cannot. The aggregate genuinely needs both.')),
    );
    this.host.sfx('unlock');
    this.host.solve('Real joint custody: MuSig2 key aggregation. The same signature on-chain as a single key, but no one can seize it.');
  }

  async autoplay(): Promise<void> {
    this.naive(true);
    this.stage = 'musig';
    this.render();
    this.musig();
  }
}

export { tag };
