import { setWorldHooks } from '../world/WorldScene';
import type { WorldScene } from '../world/WorldScene';
import { cabinReady, petNonceCabin, prologueHandlers } from './prologue';
import { cityHandlers, cityReady, cityZone, petNonceCity } from './city';
import { ch1Handlers } from './ch1';
import { ch2Handlers } from './ch2';
import { ch3Handlers } from './ch3';
import { ch4Handlers } from './ch4';
import { ch5Handlers } from './ch5';
import { ch6Handlers } from './ch6';
import { endHandlers } from './epilogue';
import { registerKeyringDetails } from './world';
import { say, type Handler } from './api';

/** Wires story scripts to the world: interactions, zones, scene entry. */

const handlers: Record<string, Handler> = {
  ...prologueHandlers,
  ...cityHandlers,
  ...ch1Handlers,
  ...ch2Handlers,
  ...ch3Handlers,
  ...ch4Handlers,
  ...ch5Handlers,
  ...ch6Handlers,
  ...endHandlers,
};

export function installStory(): void {
  registerKeyringDetails();
  setWorldHooks({
    async interact(scene: WorldScene, id: string) {
      if (id === 'nonce') return scene.map.id === 'cabin' ? petNonceCabin() : petNonceCity(scene);
      const h = handlers[id];
      if (h) await h(scene);
      else await say(['narrator', 'Nothing happens.']);
    },
    async zone(scene, id) {
      await cityZone(scene, id);
    },
    async ready(scene) {
      if (scene.map.id === 'cabin') await cabinReady(scene);
      else await cityReady(scene);
    },
  });
}

export const storyHandlerIds = (): string[] => Object.keys(handlers);
