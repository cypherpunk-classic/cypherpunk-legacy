import { NonceReuse } from '../puzzles/ch4/NonceReuse';
import { RogueKey } from '../puzzles/ch4/RogueKey';
import { TaprootReveal } from '../puzzles/ch4/TaprootReveal';
import { chapter, codex, flag, give, objective, puzzle, say, set, sfx, type Handler } from './api';
import type { WorldScene } from '../world/WorldScene';

/** Chapter 4 · Signatures. Ibrahim, Warden's nonce, rogue keys, Taproot. */

export async function ch4Zone(_w: WorldScene): Promise<void> {}

async function runNonce(): Promise<void> {
  const ok = await puzzle(new NonceReuse());
  if (!ok) {
    objective("Study Warden's signing log at the Mint.");
    return;
  }
  set('ch4.nonce');
  give('warden-key');
  codex('schnorr', 'nonce-reuse');
  await say(
    ['ibrahim', 'There it is. One reused nonce and his whole key falls out. You could sign the Directorate\'s surrender if you wanted.'],
    ['ibrahim', 'This is not hypothetical. Sony signed every PlayStation 3 with a constant nonce and lost the console in 2010. Android wallets repeated nonces in 2013 and people lost real money. Warden should have used deterministic nonces, RFC 6979, BIP-340. He bought the cheap signer.'],
    ['ibrahim', 'Use his key. Sign an order that unseals the Archive. The door at the back of the Mint is waiting for Warden\'s signature; it does not care that it is you holding the pen.'],
  );
  set('ch4.archive');
  objective('Unseal the Archive door at the Mint with the forged order.');
}

async function runArchive(w: WorldScene): Promise<void> {
  if (!flag('ch4.nonce')) {
    await say(['narrator', 'A vault door, sealed with a Directorate signature lock: ORDER 0417. It wants a newer, signed order to open.'], ['nonce', 'You need Warden\'s key. Ibrahim has the log.']);
    return;
  }
  await w.cutscene(async () => {
    sfx('door');
    w.setObjectFrame('archive', 1);
    w.burst(w.objectPos('archive')?.x ?? 0, (w.objectPos('archive')?.y ?? 0) - 20, 0x73eff7, 30);
    await say(
      ['narrator', 'You feed the signed order into the lock. It checks the signature against Warden\'s public key, finds it valid, and has no way to know it did not come from him.'],
      ['narrator', 'The Archive door rolls open. Inside: shelf after shelf of it. Thirty years of letters, source code, arguments, mistakes.'],
      ['ibrahim', 'The things we wrote so the next people would not start from zero. Welcome to the actual legacy.'],
    );
    give('archive');
    set('ch4.opened');
  });
  if (!flag('ch4.rogue')) {
    await say(['ibrahim', 'Before you go. The Directorate is pushing "joint custody" on everyone now. Two-of-two, they say, so no one acts alone. It is a lie, and I can show you exactly how.']);
    await runRogue();
  }
}

async function runRogue(): Promise<void> {
  const ok = await puzzle(new RogueKey());
  if (!ok) {
    objective("See through the Directorate's joint-custody offer.");
    return;
  }
  set('ch4.rogue');
  codex('musig', 'rogue-key');
  await say(
    ['ibrahim', 'Naive key aggregation let them announce a key that cancels yours, and take sole control of a "shared" address. MuSig2 stops it by weighting every key with a hash of the whole set.'],
    ['ibrahim', 'One more thing, and this is the good one. Your grandfather used all of this himself. The Vault you opened was a Taproot address. Let me show you what he hid in it.'],
  );
  objective("Examine Grandpa's Taproot address (Ibrahim's terminal).");
}

async function runTaproot(): Promise<void> {
  const ok = await puzzle(new TaprootReveal());
  if (!ok) {
    objective("Find the scripts hidden in Grandpa's Taproot address.");
    return;
  }
  set('ch4.taproot');
  codex('taproot');
}

export const ch4Handlers: Record<string, Handler> = {
  async 'ibrahim.talk'() {
    if (!flag('ch4.met')) {
      set('ch4.met');
      await say(
        ['ibrahim', 'You opened the Vault. Half the city felt that. I\'m Ibrahim. I used to audit the Directorate\'s cryptography, before I started auditing it <em>against</em> them.'],
        ['ibrahim', 'They sealed the Archive, your grandfather\'s Archive, behind one of Warden\'s signatures. Order 0417. Everything he and his friends ever wrote, locked up for "curation".'],
        ['ibrahim', 'Here is the thing about signatures. They prove a message came from the holder of a key. They are only as safe as that key, and Warden has been careless. Look at his signing log.'],
      );
      await runNonce();
      return;
    }
    if (!flag('ch4.nonce')) return runNonce();
    if (!flag('ch4.opened')) {
      await say(['ibrahim', 'The Archive door is at the back of the Mint. Feed it the order you signed.']);
      return;
    }
    if (!flag('ch4.rogue')) return runRogue();
    if (!flag('ch4.taproot')) return runTaproot();
    if (!flag('ch4.done')) {
      set('ch4.done');
      chapter(5);
      sfx('unlock');
      await say(
        ['ibrahim', 'A recovery path and an inheritance path. He set it up years ago so that if anything happened to him, it would come to you, cooperatively if possible, and alone if it had to be. The Directorate\'s escrow leaf? Never committed. He drafted it and threw it out.'],
        ['ibrahim', 'He trusted the math to carry his intent across his own death. That is the whole cypherpunk bet, really.'],
        ['ibrahim', 'You have the Archive. Now you need to move through the city without the Directorate cataloguing your every step. Lin, at the Masquerade, proves things without revealing them. Go learn how. The gate is open.'],
      );
      objective('Find Lin at the Masquerade (north-west).');
      return;
    }
    await say(['ibrahim', 'Tell Lin I said the Fiat–Shamir heuristic is not a suggestion. She\'ll know.']);
  },

  async 'mint.archive'(w) {
    await runArchive(w);
  },

  async 'mint.kiosk'() {
    if (!flag('ch4.met')) {
      await say(['system', 'MINT TERMINAL\nAuditor on site. Speak with Ibrahim.']);
      return;
    }
    if (!flag('ch4.nonce')) return runNonce();
    if (flag('ch4.opened') && !flag('ch4.rogue')) return runRogue();
    if (flag('ch4.rogue') && !flag('ch4.taproot')) return runTaproot();
    await say(['system', 'MINT TERMINAL\nArchive: OPEN. Audit complete.']);
  },

  async 'mint.vending'() {
    await say(['narrator', 'The machine sells snacks for satoshis. Someone scratched into the panel: "NEVER REUSE A NONCE." Below it, smaller: "ask Warden."']);
  },
};
