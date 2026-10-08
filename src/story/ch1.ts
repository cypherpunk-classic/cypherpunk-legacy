import { TwoTimePad } from '../puzzles/ch1/TwoTimePad';
import { BitFlip } from '../puzzles/ch1/BitFlip';
import { CH1_MESSAGES } from './world';
import { chapter, codex, flag, give, objective, puzzle, say, set, sfx, type Handler } from './api';
import type { WorldScene } from '../world/WorldScene';

/** Chapter 1 · One time only. Mara, the radio log, the boathouse gate. */

export async function ch1Zone(_w: WorldScene): Promise<void> {
  // Nothing scripted on entry; Mara does the talking.
}

async function runPads(): Promise<void> {
  const ok = await puzzle(new TwoTimePad());
  if (!ok) {
    objective("Decode the two messages in Mara's radio log (use her radio or the kiosk).");
    return;
  }
  set('ch1.otp');
  codex('otp', 'two-time-pad', 'venona');
  await say(
    ['mara', `"${CH1_MESSAGES.p1}"`],
    ['mara', `"${CH1_MESSAGES.p2}"`],
    ['mara', 'Hah. We sent those the week the Directorate moved in. Same pad page, both ways. Tired eyes, cold hands, one mistake.'],
    ['mara', 'The Soviets made that exact mistake in the forties, and it took the Americans forty years of patience to read what they had. You did it in ten minutes. Patience is cheaper now.'],
    ['mara', 'Split five ways. Yes. My share of the Vault key is in the lockbox at the boathouse. East end of the Wharf. The Directorate bolted one of their ticket gates onto the door.'],
    ['mara', 'Your ticket says "guest". See what you can do about that.'],
  );
  objective('Get into the boathouse lockbox (ticket gate at the east end of the Wharf).');
}

async function runGate(): Promise<void> {
  const ok = await puzzle(new BitFlip());
  if (!ok) {
    objective('Get past the boathouse ticket gate.');
    return;
  }
  set('ch1.ctr');
  codex('malleability', 'aead');
  sfx('door');
  await say(
    ['narrator', 'The lockbox clicks open. Inside, sealed in plastic: a card with two numbers on it, x = 1 and a long hex string.'],
    ['nonce', 'A Shamir share. One of five. Bring it to Mara.'],
  );
  give('share1');
  objective('Bring the share back to Mara.');
}

export const ch1Handlers: Record<string, Handler> = {
  async 'mara.talk'() {
    if (!flag('ch1.met')) {
      set('ch1.met');
      await say(
        ['mara', 'Well, well. You have his chin. And his cat, I see.'],
        ['nonce', 'He has *my* human, technically.'],
        ['mara', 'I\'m Mara. Your grandfather and I ran radio together for thirty years: him in the cabin, me on the boats. One-time pads, numbers stations, the whole romantic business.'],
        ['mara', 'A pad is perfect. Truly random key, as long as the message, used exactly once, burned after. Shannon proved nothing beats it. The catch is in the word *once*.'],
        ['mara', 'Last spring the Directorate pulled two of our messages out of the air. I think we used the same page twice. If so, they can read them, given time. I would rather we read them first.'],
        ['mara', 'Here. My radio log. Have a look.'],
      );
      await runPads();
      return;
    }
    if (!flag('ch1.otp')) return runPads();
    if (!flag('ch1.ctr')) {
      await say(['mara', 'The boathouse is at the east end. The gate takes encrypted tickets. Old firmware: counter mode, no tag.'], ['mara', 'You know your own ticket says "role=guest". You know where those letters sit. Think about what XOR does to a known plaintext.']);
      return;
    }
    if (!flag('ch1.done')) {
      set('ch1.done');
      chapter(2);
      await say(
        ['mara', 'You got in. Good. Did you notice what happened when I switched the gate to GCM?'],
        ['you', 'It didn\'t even read my forgery. The tag didn\'t match, so it refused.'],
        ['mara', 'That\'s the second lesson. Encryption hides things. It does not stop people from changing them. For that you need authentication, and these days you get both at once, AEAD, or you get nothing.'],
        ['mara', 'Keep that share safe. It\'s number one of five; three open the Vault.'],
        ['mara', 'Next you need Felix, at the Exchange. He thinks every problem is a protocol problem. He\'s usually right. I\'ve opened the gate for you: west avenue.'],
      );
      sfx('unlock');
      objective('Find Felix at the Exchange (west avenue, through the gate).');
      return;
    }
    await say(['mara', 'Go on, Felix is waiting. And tell him he still owes me a radio.']);
  },

  async 'wharf.radio'() {
    if (!flag('ch1.met')) {
      await say(['narrator', 'An old shortwave radio. A handwritten label: "MARA\'S. DO NOT TOUCH. (YES, YOU.)"']);
      return;
    }
    if (!flag('ch1.otp')) return runPads();
    await say(['narrator', 'The radio crackles: numbers in a flat voice, the same five digits over and over. Somebody somewhere is still using a pad.']);
  },

  async 'wharf.kiosk'() {
    if (!flag('ch1.met')) {
      await say(['system', 'PUBLIC TERMINAL · THE WHARF\nNo session. Talk to the harbourmaster.']);
      return;
    }
    if (!flag('ch1.otp')) return runPads();
    if (!flag('ch1.ctr')) return runGate();
    await say(['system', 'PUBLIC TERMINAL · THE WHARF\nAll tasks complete. Have a nice night.']);
  },

  async 'wharf.boathouse'() {
    if (!flag('ch1.otp')) {
      await say(['narrator', 'A Directorate ticket gate is bolted to the boathouse door. It wants a ticket.'], ['nonce', 'Mara will explain. She explains everything, eventually.']);
      return;
    }
    if (!flag('ch1.ctr')) return runGate();
    await say(['narrator', 'The lockbox stands open and empty. The gate now runs GCM. Mara looks smug about it.']);
  },
};
