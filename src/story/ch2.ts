import { DhBasics } from '../puzzles/ch2/DhBasics';
import { MitmRelay } from '../puzzles/ch2/MitmRelay';
import { Logjam } from '../puzzles/ch2/Logjam';
import { chapter, codex, flag, give, objective, puzzle, say, set, sfx, type Handler } from './api';
import type { WorldScene } from '../world/WorldScene';

/** Chapter 2 · Strangers agree. Felix, the Exchange, Diffie-Hellman and its limits. */

export async function ch2Zone(_w: WorldScene): Promise<void> {}

async function runDh(): Promise<void> {
  const ok = await puzzle(new DhBasics());
  if (!ok) {
    objective('Set up a shared key with Felix (the switchboard at the Exchange).');
    return;
  }
  set('ch2.dh');
  codex('dh', 'dlog');
  await say(
    ['felix', 'See? We never said the key out loud. Eve heard every number and still has nothing. Diffie and Hellman, 1976. Changed everything.'],
    ['felix', 'Now. The Directorate runs its patrols on the same trick, through my old switchboard. But they got sloppy. They never sign their public values.'],
    ['felix', 'Line 7 is about to carry a patrol report. Sit on the switchboard and see what "unauthenticated" really costs them.'],
  );
  objective("Work Felix's switchboard: intercept line 7.");
}

async function runMitm(): Promise<void> {
  const ok = await puzzle(new MitmRelay());
  if (!ok) {
    objective('Intercept the key exchange on line 7.');
    return;
  }
  set('ch2.mitm');
  codex('mitm');
  await say(
    ['felix', 'There it is. You were both ends of the conversation and neither side noticed. That is the man in the middle, and plain Diffie-Hellman has no answer to it.'],
    ['you', 'So how do you stop it?'],
    ['felix', 'You make each side prove who they are. Sign the exchange. We will get to that, at the Mint. For now, one more thing bothers me.'],
    ['felix', 'The Directorate keeps "downgrading links for compliance". That means forcing them onto a weak, old group. Let me show you why that is the same as handing over the keys.'],
  );
  objective("Break the Directorate's downgraded link (Felix's terminal).");
}

async function runLogjam(): Promise<void> {
  const ok = await puzzle(new Logjam());
  if (!ok) {
    objective('Break the export-grade group.');
    return;
  }
  set('ch2.logjam');
  codex('going-dark');
}

export const ch2Handlers: Record<string, Handler> = {
  async 'felix.talk'() {
    if (!flag('ch2.met')) {
      set('ch2.met');
      await say(
        ['felix', 'You must be nightjar\'s kid. He said you\'d turn up when the city needed untangling. He was annoying like that. Right about everything, which made it worse.'],
        ['felix', 'Felix. I ran the Exchange back when it was switchboards and courage. Now it mostly routes the Directorate\'s surveillance, but they haven\'t figured out I kept a key to everything.'],
        ['felix', 'You want my share of the Vault key. Fine. But humour an old engineer first. Do you actually know how two people who have never met agree on a secret with the whole world listening?'],
        ['felix', "Here. Pick a number nobody else knows."],
      );
      await runDh();
      return;
    }
    if (!flag('ch2.dh')) return runDh();
    if (!flag('ch2.mitm')) return runMitm();
    if (!flag('ch2.logjam')) return runLogjam();
    if (!flag('ch2.done')) {
      set('ch2.done');
      chapter(3);
      give('share2');
      sfx('unlock');
      await say(
        ['felix', 'Thirty-one bits. Seconds. "Export-grade" is just a polite word for "we can read it". In the nineties they made everyone use weak crypto on purpose, for export. People are still being bitten by that today.'],
        ['felix', 'You earned this. Share two of five.'],
        ['felix', 'Three things you now know: strangers can agree on a secret in the open; an unsigned exchange has no defence against a relay; and a weak group is no group at all.'],
        ['felix', 'Go see Juno in the Commons. She splits secrets for a living. Tell her the switchboard says hello. I\'ve opened the gate, east side.'],
      );
      objective('Find Juno in the Commons (east of the plaza).');
      return;
    }
    await say(['felix', 'Give Juno my regards. And tell Mara I am NOT giving her a radio.']);
  },

  async 'exchange.kiosk'() {
    if (!flag('ch2.met')) {
      await say(['system', 'EXCHANGE TERMINAL\nThe switchboard operator is right there. Go say hello.']);
      return;
    }
    if (!flag('ch2.dh')) return runDh();
    if (!flag('ch2.mitm')) return runMitm();
    if (!flag('ch2.logjam')) return runLogjam();
    await say(['system', 'EXCHANGE TERMINAL\nAll lines quiet. Felix is pretending to read a newspaper.']);
  },
};
