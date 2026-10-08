import { ShamirReconstruct } from '../puzzles/ch3/ShamirReconstruct';
import { FeldmanVerify } from '../puzzles/ch3/FeldmanVerify';
import { chapter, codex, flag, give, objective, puzzle, say, set, sfx, type Handler } from './api';
import type { WorldScene } from '../world/WorldScene';

/** Chapter 3 · Threshold. Juno, Shamir sharing, dead drops, Feldman VSS. */

export async function ch3Zone(_w: WorldScene): Promise<void> {}

async function runShamir(): Promise<void> {
  const ok = await puzzle(new ShamirReconstruct());
  if (!ok) {
    objective("Learn how shares work at Juno's workshop.");
    return;
  }
  set('ch3.shamir');
  codex('shamir');
  await say(
    ['juno', 'Feel it? Two shares and the secret is anywhere. A thousand people with two shares each learn nothing. Three shares and it collapses to one answer.'],
    ['juno', 'That is Shamir, 1979. We split the Vault key five ways. You have Mara\'s and Felix\'s. You need one more good one.'],
    ['juno', 'I would just give you mine, but the Directorate got clever. They leave fake shares in our dead drops. A loose brick by the north wall, the hollow of the old oak. One of those is poisoned.'],
    ['juno', 'Go collect them both. Then come back and we will sort the real from the fake. Properly.'],
  );
  objective('Collect the two dead-drop shares in the Commons (loose brick; hollow tree).');
}

async function runFeldman(): Promise<void> {
  const ok = await puzzle(new FeldmanVerify());
  if (!ok) {
    objective('Verify the shares and reconstruct the Vault key.');
    return;
  }
  set('ch3.feldman');
  codex('feldman');
  give('vault-key');
}

export const ch3Handlers: Record<string, Handler> = {
  async 'juno.talk'() {
    if (!flag('ch3.met')) {
      set('ch3.met');
      await say(
        ['juno', 'You are the one undoing the Directorate\'s knots. Word travels. I\'m Juno. I split secrets so that no single person, me included, can leak them.'],
        ['juno', 'Your grandfather and I designed the Vault lock together. Five shares, any three open it, any two are useless. Even to us.'],
        ['juno', 'Before I trust you with the real thing, let me show you why two is useless and three is everything. Here, in miniature.'],
      );
      await runShamir();
      return;
    }
    if (!flag('ch3.shamir')) return runShamir();
    if (!flag('ch3.gotDrops')) {
      const a = flag('ch3.dropA');
      const b = flag('ch3.dropB');
      if (!a && !b) await say(['juno', 'Both drops are still out there. The loose brick is by the north wall; the hollow oak is east. Bring me what you find.']);
      else await say(['juno', `You have ${a ? 'the brick' : 'the oak'} share. Still need ${a ? 'the oak (east)' : 'the brick (north wall)'}.`]);
      return;
    }
    if (!flag('ch3.feldman')) {
      await say(['juno', 'Four shares, then. Two from friends, two from the ground. One of the ground ones lies. Let\'s find out which, without ever exposing the secret.']);
      return runFeldman();
    }
    if (!flag('ch3.done')) {
      set('ch3.done');
      chapter(3);
      sfx('unlock');
      await say(
        ['juno', 'There it is. The Vault key, whole and verified. You caught the forgery before it ever touched the reconstruction. That is what commitments are for.'],
        ['juno', 'Go. The Vault has waited a year for him, and now for you. Legacy Plaza, the monolith in the centre.'],
        ['nonce', 'Finally.'],
      );
      objective('Open the Vault on Legacy Plaza.');
      return;
    }
    await say(['juno', 'The Vault is yours to open. We will be watching the plaza. All of us.']);
  },

  async 'commons.dropA'() {
    if (!flag('ch3.shamir')) {
      await say(['narrator', 'A brick in the north wall sits slightly proud of the others. Chalk mark beside it. But you do not yet know what to do with what is behind it.']);
      return;
    }
    if (flag('ch3.dropA')) {
      await say(['narrator', 'The brick is back in place. Nothing more behind it.']);
      return;
    }
    set('ch3.dropA');
    give('share3');
    sfx('pickup');
    await say(['narrator', 'Behind the loose brick: a share card, x = 3. The chalk mark looks fresh, and a little too neat.'], ['nonce', 'Directorate neat. Verify that one carefully.']);
    this_checkDrops();
  },

  async 'commons.dropB'() {
    if (!flag('ch3.shamir')) {
      await say(['narrator', 'The old oak has a hollow at shoulder height. Something is tucked inside, but not yet.']);
      return;
    }
    if (flag('ch3.dropB')) {
      await say(['narrator', 'The hollow is empty now.']);
      return;
    }
    set('ch3.dropB');
    give('share4');
    sfx('pickup');
    await say(['narrator', 'Inside the hollow oak: a share card, x = 4, wrapped in oilcloth the way Grandpa wrapped everything.']);
    this_checkDrops();
  },

  async 'commons.kiosk'() {
    if (!flag('ch3.met')) {
      await say(['system', 'COMMONS TERMINAL\nCommunity board. Talk to Juno in the park.']);
      return;
    }
    if (!flag('ch3.shamir')) return runShamir();
    if (flag('ch3.gotDrops') && !flag('ch3.feldman')) return runFeldman();
    await say(['system', 'COMMONS TERMINAL\nShare verification complete. The Commons thanks you.']);
  },

  async 'commons.mailbox'() {
    await say(['narrator', 'A community mailbox, painted over with a mask stencil. A notice inside: "DROP ETIQUETTE: take only what is yours. Verify everything. Trust the math."']);
  },

  async 'plaza.vault'(w) {
    if (flag('vault.opened')) {
      await say(['narrator', 'The Vault stands open, its five sockets lit. Inside, the space where the letter was.']);
      return;
    }
    if (!state.has('vault-key')) {
      await say(['narrator', 'A black monolith, taller than you. Five sockets glow faintly along its face; three are dark.'], ['nonce', 'You need the Vault key. Three good shares. Juno, in the Commons.']);
      return;
    }
    await w.cutscene(() => openVault(w));
  },
};

import { state } from './api';
import { vault } from './world';
import { gcmDecryptText } from '../crypto/aead';

async function openVault(w: WorldScene): Promise<void> {
  sfx('power');
  w.setObjectFrame('vault', 2);
  w.burst(w.objectPos('vault')?.x ?? 480, (w.objectPos('vault')?.y ?? 360) - 40, 0x3ef0ff, 40);
  w.flash(500);
  set('vault.opened');
  await say(
    ['narrator', 'You press the Vault key into the lock. Five sockets flare cyan, one after another. The monolith splits with a sound like a held breath let go.'],
    ['narrator', 'Inside: a single envelope, and a small data shard on a chain.'],
  );
  set('ch3.vaultletter');
  await readVaultLetter();
  chapter(4);
  set('warden.on');
  sfx('warden');
  await say(
    ['warden', '*(every screen in the plaza turns red)* Unauthorised Vault access detected. Legacy Plaza is now a restricted zone.'],
    ['warden', 'Whoever you are: the Archive you are looking for is already ours. Order 0417. It is sealed at the Mint, under a Directorate key. You will never open it.'],
    ['nonce', 'He sounds nervous. Good. Ibrahim is at the Mint. He has been studying Warden\'s signatures for months. The gate is open.'],
  );
  objective('Find Ibrahim at the Mint (north-east). The Archive is sealed there.');
}

async function readVaultLetter(): Promise<void> {
  const v = vault();
  const res = gcmDecryptText(v.letterKey, v.letter);
  const text = res.ok ? res.text : '(the letter is water-damaged beyond reading)';
  const paras = text.split(/\n\s*\n/);
  codex('manifesto');
  for (const p of paras) await say(['letter', p]);
  give('identity');
}

export { openVault };

function this_checkDrops(): void {
  if (flag('ch3.dropA') && flag('ch3.dropB')) {
    set('ch3.gotDrops');
    objective('Bring the shares to Juno to verify and reconstruct.');
  }
}
