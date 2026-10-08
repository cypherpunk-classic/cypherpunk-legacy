import { mod, modInv, gcd } from './bytes';
import { N, Point, mul, type Pt } from './secp';
import type { Drbg } from './drbg';

/**
 * Shamir secret sharing (1979) and Feldman verifiable secret sharing (1987).
 *
 * The dealer picks a random polynomial of degree t-1 with f(0) = secret and
 * hands out shares (i, f(i)). Any t shares determine f by Lagrange
 * interpolation; any t-1 shares are consistent with *every* possible secret.
 *
 * Here the field is Z_n with n the order of secp256k1, so the secret can be a
 * signing key and the dealer can publish Feldman commitments C_j = a_j * G.
 * Share (i, y) is genuine iff  y*G == C_0 + i*C_1 + i^2*C_2 + ...
 */

export interface Share {
  x: bigint;
  y: bigint;
}

export function evalPoly(coeffs: bigint[], x: bigint, p: bigint): bigint {
  let acc = 0n;
  for (let i = coeffs.length - 1; i >= 0; i--) acc = mod(acc * x + coeffs[i]!, p);
  return acc;
}

export interface Dealing {
  coeffs: bigint[]; // a_0 = secret
  shares: Share[];
  commitments: Pt[]; // C_j = a_j * G
}

export function deal(secret: bigint, threshold: number, count: number, rng: Drbg, p: bigint = N): Dealing {
  const coeffs = [mod(secret, p)];
  for (let j = 1; j < threshold; j++) coeffs.push(rng.scalar(p));
  const shares: Share[] = [];
  for (let i = 1; i <= count; i++) shares.push({ x: BigInt(i), y: evalPoly(coeffs, BigInt(i), p) });
  const commitments = p === N ? coeffs.map((a) => mul(a)) : [];
  return { coeffs, shares, commitments };
}

export interface LagrangeTerm {
  x: bigint;
  num: bigint; // prod x_j          (as a plain integer)
  den: bigint; // prod (x_j - x_i)  (as a plain integer, may be negative)
  lambda: bigint; // num/den mod p
}

/** Lagrange basis coefficients at x = 0, both as fractions and reduced mod p. */
export function lagrangeAtZero(xs: bigint[], p: bigint = N): LagrangeTerm[] {
  return xs.map((xi, i) => {
    let num = 1n;
    let den = 1n;
    xs.forEach((xj, j) => {
      if (j === i) return;
      num *= xj;
      den *= xj - xi;
    });
    const g = gcd(num, den);
    if (g > 1n) {
      num /= g;
      den /= g;
    }
    if (den < 0n) {
      num = -num;
      den = -den;
    }
    return { x: xi, num, den, lambda: mod(num * modInv(mod(den, p), p), p) };
  });
}

export function reconstruct(shares: Share[], p: bigint = N): bigint {
  const terms = lagrangeAtZero(
    shares.map((s) => s.x),
    p,
  );
  return terms.reduce((acc, t, i) => mod(acc + t.lambda * shares[i]!.y, p), 0n);
}

/** Feldman check: y*G == sum_j x^j * C_j. Returns both sides for display. */
export function feldmanCheck(share: Share, commitments: Pt[]): { lhs: Pt; rhs: Pt; ok: boolean } {
  const lhs = mul(share.y);
  let rhs = Point.ZERO;
  let xPow = 1n;
  for (const C of commitments) {
    rhs = rhs.add(mul(xPow, C));
    xPow = mod(xPow * share.x, N);
  }
  return { lhs, rhs, ok: lhs.equals(rhs) };
}

// ---------------------------------------------------------------------------
// The toy field GF(31): small enough to draw every point on screen.
// ---------------------------------------------------------------------------

export const TOY_P = 31n;

/** Coefficients [a0, a1, a2] of the unique quadratic through three points mod p. */
export function quadraticThrough(points: [bigint, bigint][], p: bigint = TOY_P): bigint[] {
  if (points.length !== 3) throw new Error('need exactly three points');
  const coeffs = [0n, 0n, 0n];
  points.forEach(([xi, yi], i) => {
    // basis polynomial L_i(x) = prod_{j != i} (x - x_j) / (x_i - x_j)
    let denom = 1n;
    let basis = [1n]; // polynomial coefficients, low degree first
    points.forEach(([xj], j) => {
      if (j === i) return;
      denom = mod(denom * (xi - xj), p);
      const next = new Array<bigint>(basis.length + 1).fill(0n);
      basis.forEach((c, k) => {
        next[k] = mod(next[k]! - c * xj, p);
        next[k + 1] = mod(next[k + 1]! + c, p);
      });
      basis = next;
    });
    const scale = mod(yi * modInv(denom, p), p);
    basis.forEach((c, k) => (coeffs[k] = mod(coeffs[k]! + c * scale, p)));
  });
  return coeffs;
}

/** For two known shares and a guessed secret s, the curve that would explain them. */
export function curveForGuess(s: bigint, known: [bigint, bigint][], p: bigint = TOY_P): bigint[] {
  return quadraticThrough([[0n, s], ...known], p);
}
