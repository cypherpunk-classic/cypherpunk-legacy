import { gcd, mod, modInv } from './bytes';
import { lll, type Basis, type LllResult } from './lattice';
import type { Drbg } from './drbg';

/**
 * The Merkle–Hellman knapsack cryptosystem (1978) and the lattice attack that
 * killed it.
 *
 * Private key: a superincreasing sequence w (each term exceeds the sum of all
 * before it), a modulus q > sum(w) and a multiplier r coprime to q.
 * Public key: a_i = r * w_i mod q.   Encrypt bits x: c = sum x_i * a_i.
 * Decrypt: c * r^-1 mod q = sum x_i * w_i, which is easy to solve greedily.
 *
 * Shamir broke the basic scheme in 1982; Lagarias–Odlyzko (1985) and Coster,
 * Joux, LaMacchia, Odlyzko, Schnorr and Stern (1992) showed that *any*
 * low-density subset sum falls to lattice reduction. With the CJLOSS basis
 *   b_i     = (0, .., 2, .., 0,  N*a_i)     i = 1..n
 *   b_{n+1} = (1, .., 1, .., 1,  N*c)
 * the vector sum x_i*b_i - b_{n+1} = (2x_1 - 1, ..., 2x_n - 1, 0) has entries
 * +-1, length sqrt(n), and LLL tends to find it.
 */

export interface MhPrivate {
  w: bigint[];
  q: bigint;
  r: bigint;
}

export interface MhKeys {
  priv: MhPrivate;
  pub: bigint[];
}

export function mhKeygen(n: number, rng: Drbg, startBits = 24, stepBits = 8): MhKeys {
  const w: bigint[] = [];
  let sum = 0n;
  for (let i = 0; i < n; i++) {
    const next = (i === 0 ? rng.range(1n << BigInt(startBits - 1), 1n << BigInt(startBits)) : sum + rng.range(1n, 1n << BigInt(stepBits))) as bigint;
    w.push(next);
    sum += next;
  }
  const q = sum + rng.range(1n, sum);
  let r: bigint;
  do r = rng.range(q / 4n, q - 1n);
  while (gcd(r, q) !== 1n);
  return { priv: { w, q, r }, pub: w.map((wi) => mod(r * wi, q)) };
}

export const mhEncrypt = (pub: bigint[], bits: number[]): bigint => bits.reduce((acc, x, i) => acc + (x ? pub[i]! : 0n), 0n);

export function mhDecrypt(priv: MhPrivate, c: bigint): number[] {
  let t = mod(c * modInv(priv.r, priv.q), priv.q);
  const bits = new Array<number>(priv.w.length).fill(0);
  for (let i = priv.w.length - 1; i >= 0; i--) {
    if (t >= priv.w[i]!) {
      bits[i] = 1;
      t -= priv.w[i]!;
    }
  }
  if (t !== 0n) throw new Error('not a valid ciphertext');
  return bits;
}

export function density(pub: bigint[]): number {
  const maxBits = Math.max(...pub.map((a) => a.toString(2).length));
  return pub.length / maxBits;
}

export function textToBits(text: string): number[] {
  const out: number[] = [];
  for (const ch of text) {
    const code = ch.charCodeAt(0);
    for (let i = 7; i >= 0; i--) out.push((code >> i) & 1);
  }
  return out;
}

export function bitsToText(bits: number[]): string {
  let s = '';
  for (let i = 0; i + 8 <= bits.length; i += 8) {
    let code = 0;
    for (let j = 0; j < 8; j++) code = (code << 1) | bits[i + j]!;
    s += String.fromCharCode(code);
  }
  return s;
}

/** Split text into blocks of n bits (n multiple of 8), padding with spaces. */
export function encryptText(pub: bigint[], text: string): bigint[] {
  const n = pub.length;
  const charsPerBlock = n / 8;
  const padded = text.padEnd(Math.ceil(text.length / charsPerBlock) * charsPerBlock, ' ');
  const out: bigint[] = [];
  for (let i = 0; i < padded.length; i += charsPerBlock) out.push(mhEncrypt(pub, textToBits(padded.slice(i, i + charsPerBlock))));
  return out;
}

/** Scaling factor N for the last column; it must exceed sqrt(n)/2. */
export const cjScale = (n: number): bigint => BigInt(Math.ceil(Math.sqrt(n)) + 1) * 4n;

export function cjBasis(pub: bigint[], c: bigint, N: bigint = cjScale(pub.length)): Basis {
  const n = pub.length;
  const rows: Basis = [];
  for (let i = 0; i < n; i++) {
    const row = new Array<bigint>(n + 1).fill(0n);
    row[i] = 2n;
    row[n] = N * pub[i]!;
    rows.push(row);
  }
  rows.push([...new Array<bigint>(n).fill(1n), N * c]);
  return rows;
}

/** Look for a row of the form (+-1, ..., +-1, 0) that decodes to a valid solution. */
export function decodeSolution(reduced: Basis, pub: bigint[], c: bigint): { bits: number[]; row: number } | null {
  const n = pub.length;
  for (let r = 0; r < reduced.length; r++) {
    const v = reduced[r]!;
    if (v[n] !== 0n) continue;
    if (!v.slice(0, n).every((x) => x === 1n || x === -1n)) continue;
    for (const sign of [1n, -1n]) {
      const bits = v.slice(0, n).map((x) => (x * sign === 1n ? 1 : 0));
      if (mhEncrypt(pub, bits) === c) return { bits, row: r };
    }
  }
  return null;
}

export interface AttackResult {
  bits: number[] | null;
  row: number;
  lll: LllResult;
  attempts: number;
}

/**
 * Reduce the CJLOSS lattice and read off the plaintext bits. LLL is not an SVP
 * oracle, so on an unlucky block we retry from a reordered (but equivalent)
 * starting basis, which steers LLL down a different path.
 */
export function attackBlock(pub: bigint[], c: bigint, attempts = 4): AttackResult {
  const base = cjBasis(pub, c);
  let last: LllResult | null = null;
  for (let a = 0; a < attempts; a++) {
    const start = a === 0 ? base : rotateRows(base, a);
    const res = lll(start);
    last = res;
    const sol = decodeSolution(res.basis, pub, c);
    if (sol) return { bits: sol.bits, row: sol.row, lll: res, attempts: a + 1 };
  }
  return { bits: null, row: -1, lll: last!, attempts };
}

/** A deterministic reordering of the basis rows (the lattice is unchanged). */
function rotateRows(b: Basis, k: number): Basis {
  const idx = b.map((_, i) => i);
  const step = 2 * k + 1;
  const out: Basis = [];
  const used = new Set<number>();
  let i = (k * 7) % b.length;
  while (out.length < b.length) {
    while (used.has(i)) i = (i + 1) % b.length;
    used.add(i);
    out.push(b[idx[i]!]!);
    i = (i + step) % b.length;
  }
  return out;
}

export interface KnapsackPuzzle {
  keys: MhKeys;
  message: string;
  blocks: bigint[];
}

/**
 * Generate a key and ciphertext for which the lattice attack is known to work
 * on every block (LLL is deterministic, so checking once is enough).
 */
export function makeKnapsackPuzzle(rng: Drbg, message: string, n = 32): KnapsackPuzzle {
  for (let tries = 0; tries < 20; tries++) {
    const keys = mhKeygen(n, rng);
    const blocks = encryptText(keys.pub, message);
    if (blocks.every((c) => attackBlock(keys.pub, c).bits !== null)) return { keys, message, blocks };
  }
  throw new Error('could not generate an attackable knapsack instance');
}
