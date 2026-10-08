import { app } from '../game';
import { state } from './state';
import { currentPuzzle } from '../ui/terminal';
import { debugDialog } from '../ui/dialogue';

/**
 * Test and debugging hooks, installed only with ?debug in the URL.
 * The end-to-end test drives the real UI through these: it triggers
 * interactions without walking, lets dialogue auto-advance, and asks the
 * open puzzle to play itself through its own controls.
 */
export function installDebug(): void {
  const api = {
    state,
    app,
    flags: () => ({ ...state.data.flags }),
    interact: async (id: string) => {
      if (!app.world) throw new Error('no world scene');
      await app.world.trigger(id);
    },
    puzzleId: () => currentPuzzle()?.id ?? null,
    autoplay: async () => {
      const p = currentPuzzle();
      if (!p?.autoplay) throw new Error('no puzzle with autoplay open');
      await p.autoplay();
    },
    autoDialog: (on: boolean, choices: number[] = []) => {
      debugDialog.auto = on;
      debugDialog.choices = choices.slice();
      if (on) debugDialog.kick?.();
    },
    queueChoices: (choices: number[]) => {
      debugDialog.choices.push(...choices);
    },
    teleport: (x: number, y: number) => {
      app.world?.player.setPosition(x, y);
    },
    scene: () => app.world?.map.id ?? null,
    busy: () => Boolean((app.world as unknown as { busy?: boolean } | null)?.busy),
    idle: () => !document.body.classList.contains('ui-blocked') && !Boolean((app.world as unknown as { busy?: boolean } | null)?.busy),
  };
  (window as unknown as { cpl: typeof api }).cpl = api;
}
