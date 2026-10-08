import { SigmaCheckpoint } from '../puzzles/ch5/SigmaCheckpoint';
import { SimulatorGame } from '../puzzles/ch5/SimulatorGame';
import { ExtractorFS } from '../puzzles/ch5/ExtractorFS';
import { chapter, codex, flag, give, objective, puzzle, say, set, sfx, type Handler } from './api';
import type { WorldScene } from '../world/WorldScene';

/** Chapter 5 · Knowledge. Lin, the checkpoint, zero knowledge, the extractor. */

export async function ch5Zone(_w: WorldScene): Promise<void> {}

async function runSigma(): Promise<void> {
  const ok = await puzzle(new SigmaCheckpoint());
  if (!ok) {
    objective('Pass the checkpoint without revealing your key.');
    return;
  }
  set('ch5.sigma');
  codex('sigma');
  await say(
    ['lin', 'You passed. And the checkpoint still has no idea what your key is. That is a zero-knowledge proof: you proved you know x such that X = x·G, and leaked nothing.'],
    ['lin', 'Goldwasser, Micali and Rackoff, 1985. The whole field grows out of it. But I can see you do not quite believe the "leaked nothing" part. Nobody does at first. Let me prove it to you.'],
  );
  objective("Play Lin's real-or-simulated game.");
}

async function runSim(): Promise<void> {
  const ok = await puzzle(new SimulatorGame());
  if (!ok) {
    objective('Tell the real transcripts from the simulated ones (you cannot).');
    return;
  }
  set('ch5.sim');
  await say(
    ['lin', 'Fifty-fifty. Every time. Because I can forge a transcript with no secret that looks exactly like a real one. If a transcript revealed anything about x, you could have used it to beat the game. You could not.'],
    ['lin', 'Now the sharp edge. The same fact that makes the proof sound, two answers reveal the secret, is a weapon if a prover is careless with its commitment. And the Directorate\'s checkpoint master is very careless.'],
  );
  objective("Turn the checkpoint's own proof against it (Lin's terminal).");
}

async function runExtract(): Promise<void> {
  const ok = await puzzle(new ExtractorFS());
  if (!ok) {
    objective('Extract the checkpoint key and forge a clearance proof.');
    return;
  }
  set('ch5.extract');
  codex('fiat-shamir', 'weak-fs');
  give('checkpoint-key');
}

export const ch5Handlers: Record<string, Handler> = {
  async 'masks.checkpoint'() {
    if (!flag('ch5.met')) {
      await say(['agent', 'Halt. Present your key for inspection. Routine. Everyone does it.'], ['nonce', 'Do not hand that over. Talk to Lin first. The one in the mask. Obviously.']);
      return;
    }
    if (!flag('ch5.sigma')) return runSigma();
    if (flag('ch5.passed')) {
      await say(['narrator', 'The checkpoint scanner flickers and waves you through. It no longer asks for your name.']);
      return;
    }
    await say(['lin', 'You can clear it properly once we are done. Come here first.']);
  },

  async 'masks.agent'() {
    if (!flag('ch5.extract')) {
      await say(['agent', 'Present your key. It is for your safety. We log it, we protect it, we would never misuse it.'], ['nonce', '*(quietly)* They lost the last three key logs to breaches. Do not.']);
      return;
    }
    await say(['agent', '*(the agent squints at your proof, finds it valid, and cannot explain why it feels wrong)* …cleared. Move along.']);
  },

  async 'lin.talk'() {
    if (!flag('ch5.met')) {
      set('ch5.met');
      await say(
        ['lin', 'New face. You have the look of someone the Directorate wants a name for. I\'m Lin. I make proofs that convince without confessing.'],
        ['lin', 'That checkpoint wants your key "for inspection". Hand it over and they can impersonate you forever. But you do not have to refuse and you do not have to comply. There is a third option.'],
        ['lin', 'You prove you hold the key, without showing it. The checkpoint ends up certain, and empty-handed. Watch. Then you try.'],
      );
      await runSigma();
      return;
    }
    if (!flag('ch5.sigma')) return runSigma();
    if (!flag('ch5.sim')) return runSim();
    if (!flag('ch5.extract')) return runExtract();
    if (!flag('ch5.done')) {
      set('ch5.done');
      set('ch5.passed');
      chapter(6);
      sfx('unlock');
      await say(
        ['lin', 'You extracted their master key by rewinding a sloppy prover, then wrapped it in Fiat–Shamir so you never have to interact with a checkpoint again. And you saw why the hash must bind the context, or every clearance works everywhere.'],
        ['lin', 'That last bug is not a toy. In 2022 a whole family of real proof systems shipped with weak Fiat–Shamir. "Frozen Heart", they called it.'],
        ['lin', 'The checkpoints are yours now. Walk where you like; tell them nothing.'],
        ['lin', 'One person left you should meet. Ada, at the Observatory. She does not worry about today\'s Directorate. She worries about the computer that breaks all of this at once. The gate is open.'],
      );
      objective('Find Ada at the Observatory (north of the plaza).');
      return;
    }
    await say(['lin', 'Give Ada my regards. And tell her I still think the quantum computers are further off than she says. She will disagree. She is usually right.']);
  },

  async 'masks.kiosk'() {
    if (!flag('ch5.met')) {
      await say(['system', 'MASQUERADE TERMINAL\nIdentity services. Find Lin.']);
      return;
    }
    if (!flag('ch5.sigma')) return runSigma();
    if (!flag('ch5.sim')) return runSim();
    if (!flag('ch5.extract')) return runExtract();
    await say(['system', 'MASQUERADE TERMINAL\nClearance proofs: active. Checkpoints: bypassed.']);
  },
};
