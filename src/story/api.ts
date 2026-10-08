import { audio } from '../core/audio';
import { sleep } from '../core/events';
import { state } from '../core/state';
import { itemDef } from './items';
import { runPuzzle, type Puzzle } from '../ui/terminal';
import { toast } from '../ui/toast';
import { hud } from '../ui/hud';
import type { WorldScene } from '../world/WorldScene';

/** Small helpers that story scripts use; keeps the scripts readable. */

export { say, choose } from '../ui/dialogue';
export { state };

export type Handler = (w: WorldScene) => Promise<void>;

export const flag = (k: string): boolean => state.flag(k);
export const set = (k: string, v: boolean | number | string = true): void => state.set(k, v);

export function objective(text: string): void {
  state.setObjective(text);
}

export function codex(...ids: string[]): void {
  for (const id of ids) state.unlockCodex(id);
}

export function give(item: string): void {
  if (state.has(item)) return;
  state.give(item);
  const def = itemDef(item);
  toast('KEYRING', def ? def.name : item, 'item');
  audio.sfx('pickup');
}

export async function puzzle(p: Puzzle): Promise<boolean> {
  const ok = await runPuzzle(p);
  state.save();
  return ok;
}

export const wait = sleep;

export function chapter(n: number): void {
  state.setChapter(n);
  hud.renderPips();
}

export function location(name: string, sub: string): void {
  hud.showLocation(name, sub);
}

export function sfx(name: Parameters<typeof audio.sfx>[0]): void {
  audio.sfx(name);
}

export function music(t: Parameters<typeof audio.music>[0]): void {
  audio.music(t);
}
