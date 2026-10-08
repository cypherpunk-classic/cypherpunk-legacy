import { app } from '../game';
import { Broadcast } from '../puzzles/epilogue/Broadcast';
import { chapter, codex, flag, give, music, objective, puzzle, say, set, sfx, state, wait, type Handler } from './api';
import type { WorldScene } from '../world/WorldScene';
import { ch3Handlers } from './ch3';
import { h } from '../ui/dom';
import { ui } from '../ui/root';


/** Epilogue: the finale at the Vault, the broadcast, and the credits. */

async function finale(w: WorldScene): Promise<void> {
  await w.cutscene(async () => {
    music('finale');
    await w.panTo(w.objectPos('vault')?.x ?? 480, (w.objectPos('vault')?.y ?? 360) - 20, 800);
    await say(
      ['narrator', 'The plaza is full. Mara from the Wharf, Felix from the Exchange, Juno, Ibrahim, Lin, Ada. Everyone who handed you a piece of this.'],
      ['warden', '*(the last broadcast, smaller now)* Stop. Whatever you are about to publish, the Directorate orders you to—'],
      ['nonce', 'He does not get to finish that sentence. You do.'],
    );
    w.follow();
  });
  const ok = await puzzle(new Broadcast());
  if (!ok) return;
  set('end.broadcast');
  codex('manifesto', 'crypto-wars');
  give('identity');
  await w.cutscene(async () => {
    w.burst(w.objectPos('vault')?.x ?? 480, (w.objectPos('vault')?.y ?? 360) - 30, 0x3ef0ff, 60);
    w.flash(700, 0x3ef0ff);
    sfx('success');
    set('warden.on', false);
    w.refresh();
    await say(
      ['narrator', 'The signature goes out to every node at once. Across Cryptopolis, screens that showed Warden\'s face flicker, and show the manifesto instead. The Directorate\'s broadcast simply… stops. You cannot order a verified signature to un-exist.'],
      ['mara', 'He used to say the city was never the point. The code was the point. The code outlives all of us.'],
      ['ada', 'And now the next ones can check our work. That was always the whole bet.'],
      ['nonce', 'Your grandfather would be unbearable right now. In the good way.'],
      ['nonce', 'Thank you, {{handle}}. For finishing his sentence.'],
    );
    set('end.done');
    chapter(7);
  });
  objective('');
  await wait(600);
  rollCredits();
}

function rollCredits(): void {
  ui.showHud(false);
  ui.block('credits');
  const line = (t: string): HTMLElement => h('div', null, t);
  const roll = h(
    'div',
    { class: 'roll' },
    h('h1', null, 'CYPHERPUNK LEGACY'),
    h('div', { class: 'quote' }, '"Cypherpunks write code."'),
    h('div', { class: 'note', style: 'margin-bottom:30px' }, '— Eric Hughes, A Cypherpunk\'s Manifesto, 1993'),
    h('h3', null, 'THE PEOPLE'),
    line('Grandpa (nightjar) — your legacy'),
    line('Mara — one-time pads, and patience'),
    line('Felix — key exchange, and the switchboard'),
    line('Juno — secret sharing, and trust'),
    line('Ibrahim — signatures, and the Archive'),
    line('Lin — zero knowledge, and the mask'),
    line('Ada — the quantum future'),
    line('Nonce — a daemon wearing a cat'),
    h('h3', null, 'THE CRYPTOGRAPHY WAS REAL'),
    line('BIP-39 / BIP-32 seed recovery & fingerprints'),
    line('BIP-340 Schnorr signatures'),
    line('BIP-327 MuSig2 key aggregation'),
    line('BIP-341 Taproot'),
    line('One-time pads & the two-time pad attack'),
    line('AES-CTR malleability vs AES-GCM'),
    line('Diffie–Hellman, MITM & baby-step giant-step'),
    line('Shamir secret sharing & Feldman VSS'),
    line('Schnorr Σ-protocols, simulators & extractors'),
    line('The Fiat–Shamir transform (and its weak form)'),
    line('LLL lattice reduction (exact integer arithmetic)'),
    line('Merkle–Hellman knapsack & the CJLOSS attack'),
    line('Learning With Errors'),
    line('ML-KEM-768 (FIPS 203)'),
    h('h3', null, 'BUILT WITH'),
    line('TypeScript · Phaser 3 · Vite'),
    line('@noble/curves · @noble/hashes · @noble/ciphers'),
    line('@scure/bip39 · @scure/bip32 · @noble/post-quantum'),
    line('by Paul Miller, audited and open'),
    line('Verified against official BIP & NIST test vectors'),
    h('h3', null, 'FOR'),
    line('everyone who wrote the code instead of asking permission,'),
    line('and everyone who read it and found the bugs.'),
    h('div', { class: 'quote', style: 'margin-top:50px' }, 'Privacy is necessary for an open society in the electronic age.'),
    h('div', { style: 'margin-top:60px' }, h('button', { class: 'primary', onclick: () => endCredits() }, 'Return to title')),
    h('div', { style: 'height:120px' }),
  );
  const screen = h('div', { class: 'credits', 'data-testid': 'credits' }, roll);
  ui.screen.append(screen);
  const endCredits = (): void => {
    screen.remove();
    ui.unblock('credits');
    state.save();
    app.title();
  };
  // allow skip
  setTimeout(() => {
    const skip = roll.querySelector('button');
    skip?.scrollIntoView;
  }, 100);
}

export const endHandlers: Record<string, Handler> = {
  async 'plaza.vault'(w) {
    // Overridden for the finale once Ada is done; otherwise fall through handled in ch3.
    if (flag('end.ready') && !flag('end.done')) {
      await finale(w);
      return;
    }
    if (flag('end.done')) {
      await say(['narrator', 'The Vault stands open and lit, the manifesto scrolling across its face. People come here to read it, and to add their own code to the Archive.'], ['nonce', 'We did good. Go get some sleep. I will watch the city.']);
      rollCredits();
      return;
    }
    // not yet at finale: defer to the chapter-3/opening handler
    await ch3Handlers['plaza.vault']!(w);
  },
};
