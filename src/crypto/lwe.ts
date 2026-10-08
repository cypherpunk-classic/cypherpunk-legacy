import { mod, modInv } from './bytes';
import type { Drbg } from './drbg';

/**
 * Learning With Errors (Regev, 2005), at toy size.
 *
 * Secret s in Z_q^n. Public samples (a_i, b_i = <a_i, s> + e_i mod q) with
 * small errors e_i. Without the errors, s falls to Gaussian elimination; with
 * them, recovering s is believed hard even for quantum computers, and that
 * hardness is what ML-KEM (FIPS 203) builds on.
 *
 * Regev encryption of one bit: pick a random subset S of samples,
 *   u = sum_{i in S} a_i,   v = sum_{i in S} b_i + bit * floor(q/2).
 * Decryption computes v - <u, s> = bit * floor(q/2) + (sum of the chosen e_i):
 * close to 0 means 0, close to q/2 means 1, as long as the noise stays below q/4.
 */

export interface LweParams {
  n: number;
  q: number;
  m: number;
  noise: number; // errors uniform in [-noise, noise]
}

export const TOY_LWE: LweParams = { n: 4, q: 97, m: 12, noise: 1 };

export interface LweKeys {
  params: LweParams;
  s: number[];
  A: number[][];
  e: number[];
  b: number[];
}

const m_ = (x: number, q: number): number => ((x % q) + q) % q;

export function lweKeygen(params: LweParams, rng: Drbg): LweKeys {
  const { n, q, m, noise } = params;
  const s = Array.from({ length: n }, () => rng.int(q));
  const A = Array.from({ length: m }, () => Array.from({ length: n }, () => rng.int(q)));
  const e = Array.from({ length: m }, () => rng.int(2 * noise + 1) - noise);
  const b = A.map((row, i) => m_(row.reduce((acc, a, j) => acc + a * s[j]!, 0) + e[i]!, q));
  return { params, s, A, e, b };
}

export interface LweCiphertext {
  subset: number[];
  u: number[];
  v: number;
}

export function lweEncrypt(keys: LweKeys, bit: 0 | 1, rng: Drbg): LweCiphertext {
  const { n, q, m } = keys.params;
  let subset: number[] = [];
  while (subset.length === 0) subset = Array.from({ length: m }, (_, i) => i).filter(() => rng.int(2) === 1);
  const u = new Array<number>(n).fill(0);
  let v = bit * Math.floor(q / 2);
  for (const i of subset) {
    keys.A[i]!.forEach((a, j) => (u[j] = m_(u[j]! + a, q)));
    v = m_(v + keys.b[i]!, q);
  }
  return { subset, u, v };
}

/** The "phase" v - <u,s> mod q: where the value lands on the circle. */
export function lwePhase(keys: LweKeys, ct: LweCiphertext): number {
  const { q } = keys.params;
  return m_(ct.v - ct.u.reduce((acc, x, j) => acc + x * keys.s[j]!, 0), q);
}

export function lweDecrypt(keys: LweKeys, ct: LweCiphertext): 0 | 1 {
  const { q } = keys.params;
  const ph = lwePhase(keys, ct);
  const distToHalf = Math.abs(ph - Math.floor(q / 2));
  const distToZero = Math.min(ph, q - ph);
  return distToHalf < distToZero ? 1 : 0;
}

/** Signed noise carried by a ciphertext: the sum of the errors of the chosen samples. */
export const ciphertextNoise = (keys: LweKeys, ct: LweCiphertext): number => ct.subset.reduce((acc, i) => acc + keys.e[i]!, 0);

/**
 * Solve A s = b (mod prime q) from the first n samples by Gaussian elimination.
 * Works perfectly without noise; with noise it confidently returns garbage.
 */
export function gaussianSolve(A: number[][], b: number[], q: number): number[] | null {
  const n = A[0]!.length;
  const M = A.slice(0, n).map((row, i) => [...row.map((x) => m_(x, q)), m_(b[i]!, q)]);
  for (let col = 0; col < n; col++) {
    let pivot = -1;
    for (let r = col; r < n; r++) if (M[r]![col] !== 0) {
      pivot = r;
      break;
    }
    if (pivot < 0) return null;
    [M[col], M[pivot]] = [M[pivot]!, M[col]!];
    const inv = Number(modInv(BigInt(M[col]![col]!), BigInt(q)));
    M[col] = M[col]!.map((x) => m_(x * inv, q));
    for (let r = 0; r < n; r++) {
      if (r === col || M[r]![col] === 0) continue;
      const f = M[r]![col]!;
      M[r] = M[r]!.map((x, j) => m_(x - f * M[col]![j]!, q));
    }
  }
  return M.map((row) => row[n]!);
}

/** How many of all m samples a candidate secret explains exactly (no error). */
export function residuals(keys: LweKeys, cand: number[]): number[] {
  const { q } = keys.params;
  return keys.A.map((row, i) => {
    const r = m_(keys.b[i]! - row.reduce((acc, a, j) => acc + a * cand[j]!, 0), q);
    return r > q / 2 ? r - q : r;
  });
}

export { mod };
