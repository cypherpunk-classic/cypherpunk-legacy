import { GaussReduce } from '../puzzles/ch6/GaussReduce';
import { KnapsackBreak } from '../puzzles/ch6/KnapsackBreak';
import { LweMlkem } from '../puzzles/ch6/LweMlkem';
import { chapter, codex, flag, give, objective, puzzle, say, set, sfx, type Handler } from './api';
import type { WorldScene } from '../world/WorldScene';

/** Chapter 6 · After the quantum. Ada, lattices, the knapsack, LWE and ML-KEM. */

export async function ch6Zone(_w: WorldScene): Promise<void> {}

async function runGauss(): Promise<void> {
  const ok = await puzzle(new GaussReduce());
  if (!ok) {
    objective("Reduce the lattice basis at Ada's workshop.");
    return;
  }
  set('ch6.gauss');
  codex('lattice', 'shor');
  await say(
    ['ada', 'Good. That pairwise reduce-and-swap, run across many dimensions, is LLL: Lenstra, Lenstra and Lovász, 1982. Polynomial time, and it finds surprisingly short vectors.'],
    ['ada', 'Lattices are the double edge of the next era. They break old schemes that hid structure badly, and they build new ones that hide it well. Let me show you the breaking first. The Directorate will hate it.'],
  );
  objective("Break the Directorate's QSAFE channel (Ada's terminal).");
}

async function runKnapsack(): Promise<void> {
  const ok = await puzzle(new KnapsackBreak());
  if (!ok) {
    objective('Break the QSAFE knapsack with LLL.');
    return;
  }
  set('ch6.knapsack');
  codex('knapsack');
  await say(
    ['ada', 'Their "quantum-safe" channel was a 1978 knapsack with a 2024 logo. LLL peeled it open. Hiding an easy problem inside a hard-looking one only works until someone brings the right lattice.'],
    ['ada', 'But do not walk away thinking lattices are just a weapon. The real post-quantum schemes are built on lattice problems nobody knows how to break, classically or quantumly. Here. Build one instead of breaking one.'],
  );
  objective("Build a post-quantum key (Ada's terminal).");
}

async function runPq(): Promise<void> {
  const ok = await puzzle(new LweMlkem());
  if (!ok) {
    objective('Understand LWE and run an ML-KEM encapsulation.');
    return;
  }
  set('ch6.pq');
  codex('lwe', 'mlkem', 'nist-pqc');
  give('pq-keys');
}

export const ch6Handlers: Record<string, Handler> = {
  async 'ada.talk'() {
    if (!flag('ch6.met')) {
      set('ch6.met');
      await say(
        ['ada', 'You made it past the checkpoints without leaving a name. Lin\'s work. I\'m Ada. I was the youngest in your grandfather\'s circle, and now I am the one who stays up worrying about the end of all of it.'],
        ['ada', 'Everything you have done tonight, every signature, every key exchange, rests on two problems being hard: factoring, and discrete logs. A large enough quantum computer solves both in an afternoon. Shor, 1994.'],
        ['ada', 'It does not exist yet. But the Directorate is recording everything now, to decrypt later. Harvest now, decrypt later. So we move the ground before the flood. Start with the shape of the new ground: a lattice.'],
      );
      await runGauss();
      return;
    }
    if (!flag('ch6.gauss')) return runGauss();
    if (!flag('ch6.knapsack')) return runKnapsack();
    if (!flag('ch6.pq')) return runPq();
    if (!flag('ch6.done')) {
      set('ch6.done');
      chapter(6);
      sfx('unlock');
      await say(
        ['ada', 'ML-KEM. FIPS 203, standardised in 2024, from a scheme called Kyber. Bigger keys than a curve, and it does not fall to Shor. The browsers already use it. Signal already uses it. The flood is coming and the arks are being built in the open, where we can all check them.'],
        ['ada', 'That is the whole thing, really. Your grandfather\'s whole bet. Not any one algorithm, those come and go, but doing it in public, so the next people can check our work and build on it.'],
        ['ada', 'You have the Archive, the keys, and now the map of what comes next. There is only one thing left to do with all of it.'],
        ['nonce', 'He means the Vault. The real one. Go home, {{handle}}. Finish it.'],
      );
      objective('Return to the Vault on Legacy Plaza to finish what Grandpa started.');
      set('end.ready');
      return;
    }
    await say(['ada', 'Go to the plaza. We will all be there. We have been waiting a long time for someone to finish his sentence.']);
  },

  async 'obs.telescope'() {
    await say(['narrator', 'Through the telescope: not stars, but the lattice of the city itself, points of light on an infinite grid, stretching past the horizon in every direction.'], ['ada', 'Every secure thing is a hard question about one of those grids. Pretty, isn\'t it.']);
  },

  async 'obs.whiteboard'() {
    await say(['narrator', 'A whiteboard dense with Ada\'s handwriting: "SVP · CVP · LWE · SIS · NTRU · Kyber=ML-KEM · Dilithium=ML-DSA · SPHINCS+=SLH-DSA". In the corner: "SIKE — broken 2022, one laptop, one hour. Do it in the open."']);
    codex('nist-pqc');
  },

  async 'obs.kiosk'() {
    if (!flag('ch6.met')) {
      await say(['system', 'OBSERVATORY TERMINAL\nPost-quantum research. Speak with Ada.']);
      return;
    }
    if (!flag('ch6.gauss')) return runGauss();
    if (!flag('ch6.knapsack')) return runKnapsack();
    if (!flag('ch6.pq')) return runPq();
    await say(['system', 'OBSERVATORY TERMINAL\nMigration complete. The ground has moved.']);
  },
};
