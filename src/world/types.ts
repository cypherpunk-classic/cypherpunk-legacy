import type { Track } from '../core/audio';
import type { Facing, SceneId } from '../core/state';

export interface Light {
  tex: string;
  scale?: number;
  alpha?: number;
  flicker?: boolean;
  pulse?: boolean;
  dx?: number;
  dy?: number;
}

export interface MapObject {
  id: string;
  tex: string;
  /** Bottom-centre of the sprite, in pixels. */
  x: number;
  y: number;
  frame?: number;
  anim?: boolean;
  /** Collision box anchored at the bottom-centre; dy lifts it. */
  body?: { w: number; h: number; dy?: number } | null;
  interact?: string;
  label?: string;
  radius?: number;
  /** Where the interaction point is, relative to the bottom-centre. */
  ix?: number;
  iy?: number;
  floor?: boolean;
  depth?: number;
  /** Flag condition: "a", "!a", or "a,!b" (all must hold). */
  showIf?: string;
  light?: Light;
  frameIf?: [string, number][];
  flipX?: boolean;
  alpha?: number;
}

export interface NpcDef {
  id: string;
  char: string;
  x: number;
  y: number;
  facing: Facing;
  interact?: string;
  label?: string;
  showIf?: string;
  wander?: { x: number; y: number; w: number; h: number };
  hologram?: boolean;
}

export interface Zone {
  id: string;
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface MapDef {
  id: SceneId;
  title: string;
  w: number;
  h: number;
  tiles: string[][];
  objects: MapObject[];
  npcs: NpcDef[];
  spawns: Record<string, { x: number; y: number; facing: Facing }>;
  zones?: Zone[];
  music: Track;
  /** Night tint multiplied over the map. */
  tint: number;
  ambient?: 'motes' | 'dust';
  catMode?: 'wander' | 'follow';
  catArea?: { x: number; y: number; w: number; h: number };
}
