import { bytesToBigInt, concat, utf8, type Bytes } from './bytes';
import { N, compressed, invN, modN, mul, taggedHash, type Pt } from './secp';
import type { Drbg } from './drbg';

/**
 * The Schnorr identification protocol, a Sigma protocol for "I know x with X = x*G".
 *
 *   Prover                         Verifier
 *   r random, R = r*G   --R-->
 *                       <--c--     c random
 *   s = r + c*x         --s-->     accept iff s*G == R + c*X
 *
 * Completeness: s*G = r*G + c*x*G = R + c*X.
 * Special soundness: two accepting transcripts (R, c1, s1), (R, c2, s2) with
 *   c1 != c2 give x = (s1 - s2)/(c1 - c2): whoever can answer two challenges
 *   for one commitment knows x. (The extractor is the nonce-reuse attack.)
 * Honest-verifier zero knowledge: pick c and s first, set R = s*G - c*X; the
 *   simulated transcript has exactly the distribution of a real one.
 * Fiat–Shamir: replace the verifier with c = H(R || X || context).
 */

export interface Transcript {
  R: Pt;
  c: bigint;
  s: bigint;
}

export function commit(rng: Drbg): { r: bigint; R: Pt } {
  const r = rng.scalar(N);
  return { r, R: mul(r) };
}

export const respond = (r: bigint, c: bigint, x: bigint): bigint => modN(r + c * x);

export function verifyTranscript(X: Pt, t: Transcript): boolean {
  return mul(t.s).equals(t.R.add(mul(t.c, X)));
}

/** A transcript produced without the secret. Indistinguishable from a real one. */
export function simulate(X: Pt, c: bigint, s: bigint): Transcript {
  return { R: mul(s).subtract(mul(c, X)), c: modN(c), s: modN(s) };
}

export function extract(t1: Transcript, t2: Transcript): bigint {
  if (!t1.R.equals(t2.R)) throw new Error('transcripts must share the commitment');
  if (modN(t1.c - t2.c) === 0n) throw new Error('challenges must differ');
  return modN((t1.s - t2.s) * invN(t1.c - t2.c));
}

// ---------------------------------------------------------------------------
// One-bit challenges: the training mode where cheating works half the time.
// ---------------------------------------------------------------------------

/**
 * A cheater who does not know x bets on the challenge bit in advance:
 *  - bets 0: R = r*G and answers s = r (works only if c = 0)
 *  - bets 1: picks s, sets R = s*G - X   (works only if c = 1)
 */
export function cheaterCommit(X: Pt, bet: 0 | 1, rng: Drbg): { R: Pt; s: bigint } {
  const s = rng.scalar(N);
  return bet === 0 ? { R: mul(s), s } : { R: mul(s).subtract(X), s };
}

// ---------------------------------------------------------------------------
// Fiat–Shamir
// ---------------------------------------------------------------------------

export interface FsOptions {
  includeKey: boolean;
  includeContext: boolean;
}

export function fsChallenge(R: Pt, X: Pt, context: string, opts: FsOptions = { includeKey: true, includeContext: true }): bigint {
  const parts: Bytes[] = [compressed(R)];
  if (opts.includeKey) parts.push(compressed(X));
  if (opts.includeContext) parts.push(utf8(context));
  return modN(bytesToBigInt(taggedHash('CypherpunkLegacy/fiat-shamir', concat(...parts))));
}

export function fsProve(x: bigint, X: Pt, context: string, rng: Drbg, opts?: FsOptions): Transcript {
  const { r, R } = commit(rng);
  const c = fsChallenge(R, X, context, opts);
  return { R, c, s: respond(r, c, x) };
}

export function fsVerify(X: Pt, context: string, t: Transcript, opts?: FsOptions): boolean {
  return t.c === fsChallenge(t.R, X, context, opts) && verifyTranscript(X, t);
}
