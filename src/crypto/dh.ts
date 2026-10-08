import { sha256 } from '@noble/hashes/sha2.js';
import { bigIntToBytes, bitLength, modPow, mod, type Bytes } from './bytes';
import type { Drbg } from './drbg';

/**
 * Finite-field Diffie–Hellman in prime-order subgroups of Z_p^*, with p = 2q + 1
 * a safe prime and g = 4 (a quadratic residue, hence of order q).
 *
 *   Alice: a, A = g^a      Bob: b, B = g^b      shared K = B^a = A^b = g^(ab)
 *
 * Nothing in the exchange says *who* sent A or B. A relay that swaps both for
 * its own g^m ends up sharing g^(am) with Alice and g^(bm) with Bob: the
 * man-in-the-middle attack. The cure is authentication (signatures, Chapter 4).
 */

export interface Group {
  name: string;
  p: bigint;
  q: bigint;
  g: bigint;
}

/** Small enough to do by hand (or by a very patient cat). */
export const TOY_GROUP: Group = { name: 'toy', p: 1019n, q: 509n, g: 4n };

/** 31-bit subgroup order: breakable in milliseconds with baby-step giant-step. */
export const WEAK_GROUP: Group = { name: 'weak-33', p: 4294992683n, q: 2147496341n, g: 4n };

/** 64-bit safe prime: big enough that the MITM is the easier attack in-game. */
export const DRONE_GROUP: Group = { name: 'drone-64', p: 18446741874686296103n, q: 9223370937343148051n, g: 4n };

export const pow = (grp: Group, base: bigint, e: bigint): bigint => modPow(base, mod(e, grp.q), grp.p);
export const pub = (grp: Group, secret: bigint): bigint => pow(grp, grp.g, secret);

export function isGroupElement(grp: Group, y: bigint): boolean {
  return y > 1n && y < grp.p && modPow(y, grp.q, grp.p) === 1n;
}

export function keypair(grp: Group, rng: Drbg): { secret: bigint; public: bigint } {
  const secret = rng.range(2n, grp.q - 1n);
  return { secret, public: pub(grp, secret) };
}

export function shared(grp: Group, mySecret: bigint, theirPublic: bigint): bigint {
  return pow(grp, theirPublic, mySecret);
}

/** Session key = SHA-256(big-endian bytes of K). */
export function sessionKey(grp: Group, K: bigint): Bytes {
  const len = Math.ceil(bitLength(grp.p) / 8);
  return sha256(bigIntToBytes(K, len));
}

export interface BsgsResult {
  x: bigint | null;
  babySteps: number;
  giantSteps: number;
  ms: number;
}

/**
 * Baby-step giant-step: solve g^x = y in a group of order q with ~2*sqrt(q)
 * multiplications and sqrt(q) memory. Write x = i*m + j with m = ceil(sqrt q):
 * store g^j for all j, then walk y * (g^-m)^i until a stored value appears.
 */
export function bsgs(grp: Group, y: bigint, onProgress?: (frac: number) => void, mOverride?: number): BsgsResult {
  const t0 = performance.now();
  const m = BigInt(mOverride ?? Math.ceil(Math.sqrt(Number(grp.q))));
  const table = new Map<bigint, bigint>();
  let e = 1n;
  for (let j = 0n; j < m; j++) {
    if (!table.has(e)) table.set(e, j);
    e = (e * grp.g) % grp.p;
  }
  const factor = modPow(grp.g, grp.q - m, grp.p); // g^(-m)
  let gamma = mod(y, grp.p);
  let giant = 0;
  const giants = (grp.q + m - 1n) / m;
  for (let i = 0n; i <= giants; i++) {
    giant++;
    const j = table.get(gamma);
    if (j !== undefined) {
      return { x: mod(i * m + j, grp.q), babySteps: Number(m), giantSteps: giant, ms: performance.now() - t0 };
    }
    gamma = (gamma * factor) % grp.p;
    if (onProgress && (i & 4095n) === 0n) onProgress(Number(i) / Number(giants));
  }
  return { x: null, babySteps: Number(m), giantSteps: giant, ms: performance.now() - t0 };
}

/** Rough cost of BSGS for a group of `bits`-bit order, as a human sentence. */
export function bsgsCost(bits: number): { steps: number; log2steps: number } {
  return { steps: Math.pow(2, bits / 2), log2steps: bits / 2 };
}

/** The same algorithm, yielding to the event loop so a UI can show progress. */
export async function bsgsAsync(grp: Group, y: bigint, m: number, onProgress: (phase: 'baby' | 'giant', frac: number) => void): Promise<BsgsResult> {
  const t0 = performance.now();
  const M = BigInt(m);
  const table = new Map<bigint, bigint>();
  let e = 1n;
  const chunk = 20000;
  for (let j = 0n; j < M; j++) {
    if (!table.has(e)) table.set(e, j);
    e = (e * grp.g) % grp.p;
    if (j % BigInt(chunk) === 0n) {
      onProgress('baby', Number(j) / m);
      await new Promise((r) => setTimeout(r, 0));
    }
  }
  const factor = modPow(grp.g, grp.q - M, grp.p);
  let gamma = mod(y, grp.p);
  const giants = (grp.q + M - 1n) / M;
  for (let i = 0n; i <= giants; i++) {
    const j = table.get(gamma);
    if (j !== undefined) return { x: mod(i * M + j, grp.q), babySteps: m, giantSteps: Number(i) + 1, ms: performance.now() - t0 };
    gamma = (gamma * factor) % grp.p;
    if (i % BigInt(chunk) === 0n) {
      onProgress('giant', Number(i) / Number(giants));
      await new Promise((r) => setTimeout(r, 0));
    }
  }
  return { x: null, babySteps: m, giantSteps: Number(giants), ms: performance.now() - t0 };
}
