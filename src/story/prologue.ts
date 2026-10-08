import { app } from '../game';
import { SeedRecovery } from '../puzzles/prologue/SeedRecovery';
import { GatewayLogin } from '../puzzles/prologue/GatewayLogin';
import { grandpa } from './world';
import { chapter, choose, codex, flag, give, objective, puzzle, say, set, sfx, wait, type Handler } from './api';
import type { WorldScene } from '../world/WorldScene';

/** Prologue: the cabin, the letter, the coffee ring, the gateway. */

export async function cabinReady(w: WorldScene): Promise<void> {
  if (flag('pro.intro')) return;
  set('pro.intro');
  await w.cutscene(async () => {
    await wait(500);
    await say(
      ['narrator', 'Rain on a tin roof. The smell of old paper, woodsmoke and machine oil.'],
      ['narrator', 'The lawyer said the cabin was yours now, "along with the contents and the cat". He said it like the cat was the problem.'],
      ['you', 'Grandpa\'s place. He never let anyone touch the computer.'],
      ['you', 'Well. He isn\'t here to stop me.'],
    );
  });
  objective("Look around Grandpa's cabin.");
  chapter(0);
}

const letterLines = (): [string, string][] => {
  const g = grandpa().mnemonic;
  const shown = g.words.map((w, i) => (i === g.missingIndex ? `**${w[0]}▒▒▒▒** *(coffee ring)*` : `\`${w}\``));
  return [
    ['letter', 'My dear {{handle}},\n\nIf you are reading this, the lawyer kept his promise and I have run out of time. Don\'t be sad for long. I had a good run and a better argument.'],
    ['letter', 'I left you something in Cryptopolis, the city my friends and I built when the internet was young and we thought we could keep it free. To enter, you will need my key. Here it is, the way we wrote keys down: twelve words.'],
    ['letter', shown.slice(0, 6).map((w, i) => `${i + 1}. ${w}`).join('   ') + '\n' + shown.slice(6).map((w, i) => `${i + 7}. ${w}`).join('   ')],
    ['letter', 'Never type them into anything you don\'t trust, never photograph them, and never, ever tell the Directorate. Trust, but verify.\n\nYour loving Grandpa\n\nP.S. Feed Nonce once a day. Only once. She will lie about this.'],
  ];
};

export const prologueHandlers: Record<string, Handler> = {
  async 'cabin.dresser'() {
    if (flag('pro.letter')) {
      await say(['narrator', 'The drawer is empty except for a pencil stub and a receipt for floppy disks, dated 1994.']);
      return;
    }
    sfx('door');
    await say(['narrator', 'The top drawer sticks, then gives. Inside: an envelope with your name on it in blue ink. Someone set a coffee mug on it, more than once.']);
    set('pro.letter');
    give('letter');
    await say(...(letterLines() as [never, string][]));
    await say(['you', 'Twelve words, and the coffee ate one of them. Word ' + (grandpa().mnemonic.missingIndex + 1) + ' starts with "' + grandpa().mnemonic.firstLetter + '". That\'s all that\'s left.']);
    codex('bip39');
    objective("Take the letter to Grandpa's computer.");
  },

  async 'cabin.computer'(w) {
    if (flag('pro.done')) {
      const go = await choose('system', 'CRYPTOPOLIS GATEWAY v3.1 — session ready.', ['Enter Cryptopolis', 'Not yet']);
      if (go === 0) await enterCity(w);
      return;
    }
    if (!flag('pro.letter')) {
      await say(
        ['narrator', 'A beige CRT, older than you. A yellow sticky note clings to the bezel.'],
        ['narrator', 'You press the power button. The screen hums: ENTER RECOVERY PHRASE (12 WORDS).'],
        ['you', "Twelve words. I don't have twelve words. He'd have written them down somewhere close by — the drawers, maybe. I should search the room first."],
      );
      objective('Search the cabin for the twelve words (try the chest of drawers).');
      return;
    }
    if (!flag('pro.note')) {
      await say(
        ['narrator', 'The sticky note is covered in Grandpa\'s tiny capitals:'],
        ['system', grandpa().descriptor],
        ['you', 'tr(...)... a wallet descriptor. Taproot, path 86h/0h/0h. And that bracketed number in front is underlined twice.'],
        ['you', 'The master fingerprint. If I get the seed right, the fingerprint will match. If not, it won\'t.'],
      );
      set('pro.note');
      give('descriptor');
      codex('bip32');
    }
    if (!flag('pro.booted')) {
      sfx('power');
      set('pro.booted');
      await wait(400);
      await say(['system', 'CRYPTOPOLIS GATEWAY v3.1\nnode: cabin-03 · link: up · user: nightjar\nENTER RECOVERY PHRASE (12 WORDS)']);
    }
    if (!flag('pro.seed')) {
      const ok = await puzzle(new SeedRecovery());
      if (!ok) {
        objective('Recover the missing word of the seed phrase.');
        return;
      }
      set('pro.seed');
      give('gateway-key');
      await say(
        ['you', 'Twelve words, and the fingerprint matches. That\'s his seed.'],
        ['system', 'SEED ACCEPTED. Deriving identity key m/86\'/0\'/0\'/0/0 … done.\nREQUESTING CHALLENGE FROM GATEWAY …'],
      );
    }
    if (!flag('pro.login')) {
      const ok = await puzzle(new GatewayLogin());
      if (!ok) {
        objective('Sign the gateway challenge.');
        return;
      }
      set('pro.login');
      codex('schnorr', 'hash');
    }
    set('pro.done');
    chapter(1);
    await say(
      ['system', 'WELCOME BACK, NIGHTJAR.\nLast login: 391 days ago.'],
      ['narrator', 'Something brushes against your leg. Nonce is sitting on the desk, staring at the screen like she has been waiting for this for a year.'],
      ['nonce', 'Mrrp.'],
    );
    set('pro.metCat');
    await enterCity(w);
  },

  async 'cabin.bookshelf'() {
    await say(
      ['narrator', 'Paperbacks about codes and codebreakers, a cryptography textbook with a cracked spine, and a fat binder labelled PGP SOURCE, PRINTED — DO NOT EXPORT (HA).'],
      ['you', 'He printed source code so it could leave the country as a *book*. That was a thing people had to do?'],
    );
    codex('crypto-wars', 'pgp');
  },

  async 'cabin.photo'() {
    await say(
      ['narrator', 'A faded photo: a dozen people around a folding table covered in laptops and pizza boxes. Grandpa is the one in the cardigan, grinning.'],
      ['narrator', 'On the back, in pencil: "Bay Area, 1993. nightjar, mara, felix, juno, rook, ibrahim, lin, ada. Cypherpunks write code."'],
    );
    codex('cypherpunks-list', 'manifesto');
  },

  async 'cabin.fireplace'() {
    await say(['narrator', 'The fire has burned low but steady. Someone, the lawyer maybe, laid it fresh this morning. A small brass key hangs from a nail on the mantel: "SPARE. NOT A SECRET."']);
  },

  async 'cabin.armchair'() {
    await say(['narrator', 'The armchair is shaped exactly like Grandpa. You decide not to sit in it yet.']);
  },

  async 'cabin.bed'() {
    await say(['narrator', 'A patchwork quilt. Under the pillow: a paperback on number theory, bookmarked at a chapter called "Primes Are Not Rare."']);
  },

  async 'cabin.plant'() {
    await say(['narrator', 'A very determined fern. The soil is still damp. Somebody has been watering it.']);
  },

  async 'cabin.coat'() {
    await say(['narrator', 'A blue jacket and a red scarf. In the pocket: a ticket stub, "CRYPTO \'93", and a mint that has seen things.']);
  },

  async 'cabin.boxes'() {
    await say(
      ['narrator', 'Moving boxes, half packed. One is labelled PGP in marker. Inside: floppy disks, a modem, and a stack of mailing-list printouts with a rubber band around them.'],
      ['narrator', 'The top page: "Subject: Re: export controls are a munitions ban on mathematics."'],
    );
    codex('bernstein');
  },

  async 'cabin.window'() {
    await say(['narrator', 'Rain, pine trees, nothing else for miles. The cabin\'s only line to the outside world is a cable that runs from the roof into the dark.']);
  },

  async 'cabin.door'() {
    if (flag('pro.done')) await say(['you', 'The rain hasn\'t let up. Cryptopolis is warmer anyway. The computer is right there.']);
    else await say(['you', 'Not in this rain. And not before I find out what he left me.']);
  },
};

export async function enterCity(w: WorldScene): Promise<void> {
  w.savePosition();
  set('pro.toCity');
  await app.goto('city', 'gateway');
}

export async function petNonceCabin(): Promise<void> {
  if (!flag('pro.metCat')) {
    set('pro.metCat');
    await say(
      ['narrator', 'A grey cat with a pink collar regards you from the rug. The tag on the collar says NONCE. On the back, engraved: USE ONCE.'],
      ['nonce', 'Mrrrow.'],
      ['you', 'Grandpa named his cat after a number you\'re only supposed to use once. Of course he did.'],
    );
    sfx('meow');
    return;
  }
  sfx('meow');
  const lines = [
    'Nonce headbutts your hand, then looks pointedly at her empty bowl.',
    'Nonce purrs. She has definitely already eaten. She will tell you otherwise.',
    'Nonce stares at the computer, then at you, then at the computer.',
  ];
  await say(['narrator', lines[Math.floor(Math.random() * lines.length)]!]);
}
