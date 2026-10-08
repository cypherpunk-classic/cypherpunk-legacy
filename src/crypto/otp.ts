import { asciiToBytes, bytesToAscii, xor, type Bytes } from './bytes';
import type { Drbg } from './drbg';

/**
 * One-time pads and the two-time pad.
 *
 * C = P xor K is perfectly secret if K is uniformly random, as long as the
 * message, and is never used again (Shannon, 1949). Reuse the pad and
 *   C1 xor C2 = (P1 xor K) xor (P2 xor K) = P1 xor P2,
 * the key cancels and what is left is two plaintexts tangled together.
 * Guessing a word in one ("crib") reveals the same span of the other.
 */

export interface TwoTimePad {
  pad: Bytes;
  c1: Bytes;
  c2: Bytes;
  x: Bytes; // c1 xor c2 = p1 xor p2
  p1: string;
  p2: string;
}

export function makeTwoTimePad(p1: string, p2: string, rng: Drbg): TwoTimePad {
  if (p1.length !== p2.length) throw new Error('messages must have equal length');
  const pad = rng.bytes(p1.length);
  const c1 = xor(asciiToBytes(p1), pad);
  const c2 = xor(asciiToBytes(p2), pad);
  return { pad, c1, c2, x: xor(c1, c2), p1, p2 };
}

/** What a guess for one message implies for the other at the same positions. */
export function partner(x: Bytes, guess: string, offset: number): string {
  const span = x.subarray(offset, offset + guess.length);
  return bytesToAscii(xor(span, asciiToBytes(guess)), '·');
}

const PLAUSIBLE = /^[A-Z .,'!?:;\-0-9]$/;

/** How plausible a decoded fragment looks: share of characters that are capitals, digits or punctuation. */
export function plausibility(fragment: string): number {
  if (!fragment.length) return 0;
  let ok = 0;
  for (const ch of fragment) if (PLAUSIBLE.test(ch)) ok++;
  return ok / fragment.length;
}

export interface CribHit {
  offset: number;
  partnerText: string;
  score: number;
}

/** Slide a crib along the whole XOR stream (the classic "crib drag"). */
export function cribDrag(x: Bytes, crib: string): CribHit[] {
  const hits: CribHit[] = [];
  for (let off = 0; off + crib.length <= x.length; off++) {
    const partnerText = partner(x, crib, off);
    hits.push({ offset: off, partnerText, score: plausibility(partnerText) });
  }
  return hits;
}
