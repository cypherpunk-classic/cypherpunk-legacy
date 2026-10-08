import { app } from '../game';
import { chapter, choose, codex, flag, location, objective, say, set, sfx, wait, type Handler } from './api';
import type { WorldScene } from '../world/WorldScene';
import { ch1Zone } from './ch1';
import { ch3Zone } from './ch3';
import { ch4Zone } from './ch4';
import { ch5Zone } from './ch5';
import { ch6Zone } from './ch6';

/** Cryptopolis in general: arrival, Nonce the daemon, zones, citizens, gates. */

export async function cityReady(w: WorldScene): Promise<void> {
  if (flag('city.arrived')) return;
  set('city.arrived');
  await w.cutscene(async () => {
    w.flash(400);
    sfx('glitch');
    await wait(700);
    await say(
      ['narrator', 'The cabin dissolves into scanlines and re-forms as a street at night. Rain-slick asphalt, neon reflected in black water, the hum of a city that is mostly made of numbers.'],
      ['nonce', 'Took you long enough.'],
      ['you', '…the cat talks.'],
      ['nonce', 'In here I\'m not a cat. I\'m a daemon wearing a cat. Your grandfather thought that was funny. He was right.'],
      ['nonce', 'Welcome to Cryptopolis. He and his friends built it in the nineties: a place where people could talk without being overheard. It\'s smaller now. The Directorate has been *regulating* it.'],
    );
    sfx('warden');
    await say(
      ['warden', '*(broadcast on every screen)* Citizens of Cryptopolis. This is Warden, Directorate of Lawful Access.'],
      ['warden', 'Under the Lawful Access Act, all private keys are to be escrowed with the Directorate by the end of the month. Encryption without escrow is obstruction.'],
      ['warden', 'Those with nothing to hide have nothing to fear. Your keys will be safe with us.'],
      ['nonce', 'They always say that part.'],
      ['nonce', 'Your grandfather left you something in the Vault on Legacy Plaza. It is locked with a key split into five shares; any three open it. His friends hold some of them.'],
      ['nonce', 'Start with Mara. She\'s by the radio shack at the west end of the Wharf. Follow the antenna.'],
    );
  });
  set('warden.on');
  codex('going-dark');
  objective('Find Mara at the radio shack on the Wharf.');
  chapter(1);
}

const ZONES: Record<string, [string, string]> = {
  wharf: ['THE WHARF', 'Chapter 1 · One time only'],
  exchange: ['THE EXCHANGE', 'Chapter 2 · Strangers agree'],
  commons: ['THE COMMONS', 'Chapter 3 · Threshold'],
  plaza: ['LEGACY PLAZA', 'The Vault'],
  mint: ['THE MINT', 'Chapter 4 · Signatures'],
  masks: ['THE MASQUERADE', 'Chapter 5 · Knowledge'],
  observatory: ['THE OBSERVATORY', 'Chapter 6 · After the quantum'],
};

export async function cityZone(w: WorldScene, id: string): Promise<void> {
  const z = ZONES[id];
  if (z) location(z[0], z[1]);
  if (id === 'wharf') await ch1Zone(w);
  if (id === 'commons') await ch3Zone(w);
  if (id === 'mint') await ch4Zone(w);
  if (id === 'masks') await ch5Zone(w);
  if (id === 'observatory') await ch6Zone(w);
}

function hint(): string {
  if (!flag('ch1.done')) return flag('ch1.met') ? "Mara's terminal is the kiosk by her radio. The boathouse gate is at the east end of the Wharf." : 'Mara is by the radio shack at the west end of the Wharf. Look for the antenna.';
  if (!flag('ch2.done')) return 'The Exchange gate is open now: west avenue, through the fence. Felix runs the switchboard.';
  if (!flag('ch3.done')) return 'The Commons, east of the plaza. Juno is in the park. Shares hide in dead drops: a loose brick, a hollow tree.';
  if (!flag('vault.opened')) return 'The Vault. Legacy Plaza. Now. I have waited a year for this.';
  if (!flag('ch4.done')) return 'Ibrahim is at the Mint, north-east. Warden sealed the Archive door there.';
  if (!flag('ch5.done')) return 'The Masquerade, north-west. Lin will teach you to prove things without telling them.';
  if (!flag('ch6.done')) return 'Ada is at the Observatory, north of the plaza. Dress warmly; the future is cold.';
  if (!flag('end.done')) return 'Back to the Vault. Time to finish what he started.';
  return 'You did it. I am going to sleep on a warm server rack for a week.';
}

export async function petNonceCity(_w: WorldScene): Promise<void> {
  sfx('meow');
  const pick = await choose('nonce', 'Mrrp?', ['Where should I go?', 'What are you, really?', 'Good cat.']);
  if (pick === 0) await say(['nonce', hint()]);
  else if (pick === 1)
    await say(
      ['nonce', 'A small program your grandfather wrote to look after the city while he was away. Mostly I keep logs and judge people.'],
      ['nonce', 'He gave me the cat\'s face so that I would not feel like a program. It worked. Please do not tell the cat.'],
    );
  else await say(['nonce', 'I know.']);
}

const CITIZEN_LINES: string[][] = [
  ['A dockworker shrugs.', '"Escrow my keys? With the people who lost the last escrow database? Sure. Right after they find it."'],
  ['A woman with a laptop sticker that says "NO".', '"My grandmother used PGP to write to her sister during a war. People forget why this matters until they need it."'],
  ['A kid in a hoodie.', '"Did you know you can break a one-time pad if somebody uses it twice? My teacher says the Soviets did. Twice."'],
  ['A park regular, feeding pixelated pigeons.', '"Shares are like pigeons. Three of them come back, you have got a secret. Two of them, you have got nothing. Even if you think you have."'],
  ['A courier, checking a delivery tablet.', '"If the Directorate can read every message, so can whoever breaks into the Directorate. That is just arithmetic."'],
];

export const cityHandlers: Record<string, Handler> = {
  async 'citizen.1'() {
    await say(['narrator', CITIZEN_LINES[0]![0]!], ['narrator', CITIZEN_LINES[0]![1]!]);
  },
  async 'citizen.2'() {
    await say(['narrator', CITIZEN_LINES[1]![0]!], ['narrator', CITIZEN_LINES[1]![1]!]);
  },
  async 'citizen.3'() {
    await say(['narrator', CITIZEN_LINES[2]![0]!], ['narrator', CITIZEN_LINES[2]![1]!]);
    codex('venona');
  },
  async 'citizen.4'() {
    await say(['narrator', CITIZEN_LINES[3]![0]!], ['narrator', CITIZEN_LINES[3]![1]!]);
  },
  async 'citizen.5'() {
    await say(['narrator', CITIZEN_LINES[4]![0]!], ['narrator', CITIZEN_LINES[4]![1]!]);
  },

  async 'gate.exchange'() {
    await say(['narrator', 'A shimmering barrier: DIRECTORATE OF LAWFUL ACCESS — RESTRICTED ZONE.'], ['nonce', 'Mara can get this open. Help her first.']);
  },
  async 'gate.commons'() {
    await say(['narrator', 'The barrier crackles. A placard: CLOSED FOR YOUR SAFETY.'], ['nonce', 'Felix has the override for the Commons. One thing at a time.']);
  },
  async 'gate.mint'() {
    await say(['narrator', 'The Mint is sealed: NOTICE OF LAWFUL SEIZURE.'], ['nonce', 'Open the Vault first. Your grandfather wrote down what to do about this.']);
  },
  async 'gate.masks'() {
    await say(['narrator', 'The barrier pulses in time with a checkpoint scanner beyond it.'], ['nonce', 'Finish at the Mint. Ibrahim will know who can let you in.']);
  },
  async 'gate.obs'() {
    await say(['narrator', 'Beyond the barrier, the Observatory dome is open to the sky.'], ['nonce', 'Ada only takes visitors who can pass a checkpoint without showing their papers. Lin first.']);
  },

  async 'wharf.gatepad'() {
    const c = await choose('system', 'GATEWAY PAD — link to cabin-03', ['Return to the cabin', 'Stay in Cryptopolis']);
    if (c === 0) await app.goto('cabin', 'computer');
  },
  async 'wharf.billboard'() {
    await say(['narrator', 'LAWFUL ACCESS KEEPS YOU SAFE. In smaller print: "Key escrow is mandatory for all citizens. Keys are stored securely in a single, highly secure location."'], ['you', 'A single location. What could go wrong.']);
    codex('clipper');
  },
  async 'plaza.bench'() {
    await say(['narrator', 'You sit for a while. Rain drums on the neon. Somewhere a modem sings a song from 1996.']);
  },
  async 'plaza.statue'() {
    await say(['narrator', 'A statue holding a key over its head. The plaque reads: "TO EVERYONE WHO WROTE THE CODE INSTEAD OF ASKING PERMISSION."'], ['narrator', 'Below, smaller, scratched in by hand: "and to everyone who read it and found the bugs."']);
    codex('manifesto');
  },
  async 'street.vending'() {
    await say(['narrator', 'The vending machine offers PADS (single use), SALT (per user), and PEPPER (keep it secret). You are out of coins.']);
  },
  async 'street.stall'() {
    await say(['narrator', 'A stall selling second-hand hardware: modems, a box of smart cards, and a sign reading "ENTROPY: FRESHLY HARVESTED FROM LAVA LAMPS."']);
  },
  async 'mint.vending'() {
    await say(['narrator', 'The machine sells snacks for satoshis. A note taped to it: "If you reuse your nonce, I keep your change."']);
  },

  async 'warden.holo'() {
    sfx('warden');
    if (!flag('vault.opened'))
      await say(['warden', '*(looping)* …all private keys are to be escrowed with the Directorate by the end of the month. Those with nothing to hide have nothing to fear…']);
    else if (!flag('ch4.done'))
      await say(['warden', '*(looping)* The Archive at the Mint has been secured under Order 0417. The Directorate will curate its contents for public safety.']);
    else if (!flag('ch5.done'))
      await say(['warden', '*(looping)* Checkpoints at the Masquerade are for your protection. Present your key for verification. Cooperation is appreciated.']);
    else if (!flag('ch6.done'))
      await say(['warden', '*(looping)* Project HARVEST proceeds on schedule. Encryption is a temporary inconvenience.']);
    else await say(['narrator', 'The projector flickers, shows a test pattern, and gives up.']);
  },
};

export { hint as nonceHint };
