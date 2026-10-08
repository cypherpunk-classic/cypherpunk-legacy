import { log2Big, roundDiv } from './bytes';

/**
 * Lattice reduction with exact integer arithmetic.
 *
 * `lll` is the integral LLL algorithm (Cohen, "A Course in Computational
 * Algebraic Number Theory", Algorithm 2.6.7, after de Weger). Instead of the
 * rational Gram–Schmidt coefficients mu_{i,j} it keeps the integers
 *   d_i       = det(Gram matrix of b_1..b_i)          (so |b_i*|^2 = d_i / d_{i-1})
 *   lam_{i,j} = d_j * mu_{i,j}
 * and every division it performs is exact. No floating point, no surprises.
 */

export type Vec = bigint[];
export type Basis = Vec[];

export const dot = (u: Vec, v: Vec): bigint => u.reduce((acc, x, i) => acc + x * v[i]!, 0n);
export const sub = (u: Vec, v: Vec, q = 1n): Vec => u.map((x, i) => x - q * v[i]!);
export const norm2 = (u: Vec): bigint => dot(u, u);
const abs = (x: bigint): bigint => (x < 0n ? -x : x);

/** Gram determinants d_0..d_n of a basis (d_0 = 1), via the same exact recursion LLL uses. */
export function gramDeterminants(basis: Basis): bigint[] {
  const n = basis.length;
  const b = [[] as Vec, ...basis];
  const d: bigint[] = new Array<bigint>(n + 1).fill(0n);
  d[0] = 1n;
  const lam: bigint[][] = Array.from({ length: n + 1 }, () => new Array<bigint>(n + 1).fill(0n));
  for (let k = 1; k <= n; k++) {
    for (let j = 1; j <= k; j++) {
      let u = dot(b[k]!, b[j]!);
      for (let i = 1; i <= j - 1; i++) u = (d[i]! * u - lam[k]![i]! * lam[j]![i]!) / d[i - 1]!;
      if (j < k) lam[k]![j] = u;
      else d[k] = u;
    }
  }
  return d;
}

/** log2 |b_i*| for i = 1..n: the "profile" of a basis. Reduction flattens it. */
export function profileFromDets(d: bigint[]): number[] {
  const out: number[] = [];
  for (let i = 1; i < d.length; i++) out.push((log2Big(d[i]!) - log2Big(d[i - 1]!)) / 2);
  return out;
}

export const profile = (basis: Basis): number[] => profileFromDets(gramDeterminants(basis));

export interface LllTraceFrame {
  swaps: number;
  reductions: number;
  k: number;
  profile: number[];
}

export interface LllResult {
  basis: Basis;
  swaps: number;
  reductions: number;
  profileBefore: number[];
  profileAfter: number[];
  trace: LllTraceFrame[];
  ms: number;
}

/**
 * LLL-reduce the rows of `basis` with parameter delta = num/den (default 0.99).
 * Rows must be linearly independent.
 */
export function lll(basis: Basis, delta: [bigint, bigint] = [99n, 100n], traceEvery = 1): LllResult {
  const t0 = performance.now();
  const n = basis.length;
  const [dn, dd] = delta;
  const b: Vec[] = [[], ...basis.map((r) => r.slice())];
  const d: bigint[] = new Array<bigint>(n + 1).fill(0n);
  const lam: bigint[][] = Array.from({ length: n + 1 }, () => new Array<bigint>(n + 1).fill(0n));
  const profileBefore = profile(basis);
  const trace: LllTraceFrame[] = [];
  let swaps = 0;
  let reductions = 0;

  const currentProfile = (kmax: number): number[] => {
    const p = profileBefore.slice();
    for (let i = 1; i <= kmax; i++) p[i - 1] = (log2Big(d[i]!) - log2Big(d[i - 1]!)) / 2;
    return p;
  };

  const redi = (k: number, l: number): void => {
    if (2n * abs(lam[k]![l]!) > d[l]!) {
      const q = roundDiv(lam[k]![l]!, d[l]!);
      b[k] = sub(b[k]!, b[l]!, q);
      lam[k]![l] = lam[k]![l]! - q * d[l]!;
      for (let i = 1; i <= l - 1; i++) lam[k]![i] = lam[k]![i]! - q * lam[l]![i]!;
      reductions++;
    }
  };

  let kmax = 1;
  const swapi = (k: number): void => {
    [b[k], b[k - 1]] = [b[k - 1]!, b[k]!];
    for (let j = 1; j <= k - 2; j++) [lam[k]![j], lam[k - 1]![j]] = [lam[k - 1]![j]!, lam[k]![j]!];
    const L = lam[k]![k - 1]!;
    const B = (d[k - 2]! * d[k]! + L * L) / d[k - 1]!;
    for (let i = k + 1; i <= kmax; i++) {
      const t = lam[i]![k]!;
      lam[i]![k] = (d[k]! * lam[i]![k - 1]! - L * t) / d[k - 1]!;
      lam[i]![k - 1] = (B * t + L * lam[i]![k]!) / d[k]!;
    }
    d[k - 1] = B;
    swaps++;
  };

  d[0] = 1n;
  if (n >= 1) d[1] = dot(b[1]!, b[1]!);
  let k = 2;
  while (k <= n) {
    if (k > kmax) {
      kmax = k;
      for (let j = 1; j <= k; j++) {
        let u = dot(b[k]!, b[j]!);
        for (let i = 1; i <= j - 1; i++) u = (d[i]! * u - lam[k]![i]! * lam[j]![i]!) / d[i - 1]!;
        if (j < k) lam[k]![j] = u;
        else {
          if (u === 0n) throw new Error('basis vectors are linearly dependent');
          d[k] = u;
        }
      }
    }
    for (;;) {
      redi(k, k - 1);
      const L = lam[k]![k - 1]!;
      if (dd * d[k]! * d[k - 2]! < dn * d[k - 1]! * d[k - 1]! - dd * L * L) {
        swapi(k);
        if (swaps % traceEvery === 0) trace.push({ swaps, reductions, k, profile: currentProfile(kmax) });
        k = Math.max(2, k - 1);
        continue;
      }
      for (let l = k - 2; l >= 1; l--) redi(k, l);
      k++;
      break;
    }
  }
  const out = b.slice(1);
  const profileAfter = profileFromDets(d);
  trace.push({ swaps, reductions, k: n, profile: profileAfter });
  return { basis: out, swaps, reductions, profileBefore, profileAfter, trace, ms: performance.now() - t0 };
}

/** Check the two LLL conditions (size reduction and Lovász) exactly. */
export function isLllReduced(basis: Basis, delta: [bigint, bigint] = [99n, 100n]): boolean {
  const n = basis.length;
  const b = [[] as Vec, ...basis];
  const d: bigint[] = new Array<bigint>(n + 1).fill(0n);
  d[0] = 1n;
  const lam: bigint[][] = Array.from({ length: n + 1 }, () => new Array<bigint>(n + 1).fill(0n));
  for (let k = 1; k <= n; k++)
    for (let j = 1; j <= k; j++) {
      let u = dot(b[k]!, b[j]!);
      for (let i = 1; i <= j - 1; i++) u = (d[i]! * u - lam[k]![i]! * lam[j]![i]!) / d[i - 1]!;
      if (j < k) lam[k]![j] = u;
      else d[k] = u;
    }
  for (let k = 2; k <= n; k++) {
    for (let j = 1; j < k; j++) if (2n * abs(lam[k]![j]!) > d[j]!) return false;
    const L = lam[k]![k - 1]!;
    if (delta[1] * d[k]! * d[k - 2]! < delta[0] * d[k - 1]! * d[k - 1]! - delta[1] * L * L) return false;
  }
  return true;
}

// ---------------------------------------------------------------------------
// Two dimensions: Lagrange–Gauss reduction, one move at a time.
// ---------------------------------------------------------------------------

export type V2 = [bigint, bigint];

export interface Gauss2State {
  b1: V2;
  b2: V2;
}

/** mu = <b1,b2>/<b1,b1> as an exact fraction and as a float for display. */
export function mu2(s: Gauss2State): { num: bigint; den: bigint; value: number; nearest: bigint } {
  const num = s.b1[0] * s.b2[0] + s.b1[1] * s.b2[1];
  const den = s.b1[0] * s.b1[0] + s.b1[1] * s.b1[1];
  return { num, den, value: Number(num) / Number(den), nearest: roundDiv(num, den) };
}

export const n2 = (v: V2): bigint => v[0] * v[0] + v[1] * v[1];

export function gaussStep(s: Gauss2State, k: bigint): Gauss2State {
  return { b1: s.b1, b2: [s.b2[0] - k * s.b1[0], s.b2[1] - k * s.b1[1]] };
}

export const gaussSwap = (s: Gauss2State): Gauss2State => ({ b1: s.b2, b2: s.b1 });

export const det2 = (s: Gauss2State): bigint => s.b1[0] * s.b2[1] - s.b1[1] * s.b2[0];

/** Reduced: |mu| <= 1/2 and |b1| <= |b2|. Then b1 is a shortest nonzero vector. */
export function isGaussReduced(s: Gauss2State): boolean {
  const { num, den } = mu2(s);
  return 2n * abs(num) <= den && n2(s.b1) <= n2(s.b2);
}

/** Full Lagrange–Gauss reduction (used to check the player's answer). */
export function gaussReduce(s0: Gauss2State): { state: Gauss2State; steps: number } {
  let s = n2(s0.b1) <= n2(s0.b2) ? s0 : gaussSwap(s0);
  let steps = 0;
  for (;;) {
    s = gaussStep(s, mu2(s).nearest);
    steps++;
    if (n2(s.b2) >= n2(s.b1)) return { state: s, steps };
    s = gaussSwap(s);
  }
}
