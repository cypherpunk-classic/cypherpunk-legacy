import { CAST, PLAYER_LOOKS, drawCatSheet, drawPortrait, type CharDef } from '../art/characters';
import { makeCanvas } from '../art/draw';
import { canvasToDataUrl } from '../ui/dom';
import { state } from '../core/state';

/** Who can talk, how their name looks, and the portrait shown next to them. */

export type SpeakerId =
  | 'you'
  | 'grandpa'
  | 'nonce'
  | 'mara'
  | 'felix'
  | 'juno'
  | 'ibrahim'
  | 'lin'
  | 'ada'
  | 'warden'
  | 'agent'
  | 'system'
  | 'narrator'
  | 'letter';

export interface Speaker {
  id: SpeakerId;
  name: () => string;
  color: string;
  portrait?: string; // data URL
  style?: 'terminal' | 'narration' | 'letter';
}

const COLORS: Partial<Record<SpeakerId, string>> = {
  you: '#3ef0ff',
  grandpa: '#ffcd75',
  letter: '#ffcd75',
  nonce: '#a7f070',
  mara: '#ffcd75',
  felix: '#ef7d57',
  juno: '#5dff9a',
  ibrahim: '#ff6b81',
  lin: '#c792ff',
  ada: '#73b7ff',
  warden: '#ff5566',
  agent: '#ff5566',
  system: '#3ef0ff',
};

const portraits = new Map<string, string>();

function portraitFor(def: CharDef | undefined): string | undefined {
  if (!def) return undefined;
  if (!portraits.has(def.id)) portraits.set(def.id, canvasToDataUrl(drawPortrait(def, 5)));
  return portraits.get(def.id);
}

function catPortrait(): string {
  if (!portraits.has('cat')) {
    const sheet = drawCatSheet();
    const c = makeCanvas(16 * 5, 16 * 5);
    const g = c.getContext('2d')!;
    g.imageSmoothingEnabled = false;
    g.drawImage(sheet, 0, 0, 16, 16, 0, 0, 80, 80);
    portraits.set('cat', canvasToDataUrl(c));
  }
  return portraits.get('cat')!;
}

export function speaker(id: SpeakerId): Speaker {
  const cast = (key: string): CharDef | undefined => CAST.find((c) => c.id === key);
  switch (id) {
    case 'you':
      return { id, name: () => state.data.handle || 'you', color: COLORS.you!, portrait: portraitFor(PLAYER_LOOKS[state.data.look] ?? PLAYER_LOOKS[0]) };
    case 'nonce':
      return { id, name: () => 'Nonce', color: COLORS.nonce!, portrait: catPortrait() };
    case 'system':
      return { id, name: () => 'TERMINAL', color: COLORS.system!, style: 'terminal' };
    case 'narrator':
      return { id, name: () => '', color: '#94b0c2', style: 'narration' };
    case 'letter':
      return { id, name: () => "Grandpa's letter", color: COLORS.letter!, style: 'letter', portrait: portraitFor(cast('grandpa')) };
    case 'agent':
      return { id, name: () => 'DLA Agent', color: COLORS.agent!, portrait: portraitFor(cast('agent')) };
    default: {
      const def = cast(id);
      return { id, name: () => def?.name ?? id, color: COLORS[id] ?? '#d7e3f4', portrait: portraitFor(def) };
    }
  }
}
