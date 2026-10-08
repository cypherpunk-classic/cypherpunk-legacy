import { bytesToBigInt, concat, equalBytes, type Bytes } from './bytes';
import { Point, compressed, modN, mul, taggedHash, type Pt } from './secp';

/**
 * Key aggregation for multi-signatures.
 *
 * Naive aggregation Q = P1 + P2 is broken by the rogue-key attack: whoever
 * announces their key last can announce P2 = P_attacker - P1, making
 * Q = P_attacker, a key they alone control.
 *
 * BIP-327 (MuSig2) KeyAgg weights every key by a coefficient that depends on
 * the whole key list:  a_i = H_coef(L || P_i),  L = H_list(P_1 || ... || P_u),
 * so changing your own key changes everybody's coefficients and the attacker
 * is left chasing a moving target. (The second distinct key gets a_i = 1, a
 * BIP-327 optimisation that keeps the scheme secure.)
 */

export function naiveAggregate(keys: Pt[]): Pt {
  return keys.reduce((acc, P) => acc.add(P), Point.ZERO);
}

/** The rogue key: announce P_attacker - sum(other keys). */
export function rogueKey(target: Pt, others: Pt[]): Pt {
  return target.subtract(naiveAggregate(others));
}

export function hashKeys(pks: Bytes[]): Bytes {
  return taggedHash('KeyAgg list', concat(...pks));
}

export function secondKey(pks: Bytes[]): Bytes {
  const first = pks[0]!;
  for (const pk of pks) if (!equalBytes(pk, first)) return pk;
  return new Uint8Array(33);
}

export function keyAggCoeff(pks: Bytes[], pk: Bytes): bigint {
  const pk2 = secondKey(pks);
  if (equalBytes(pk, pk2)) return 1n;
  const L = hashKeys(pks);
  return modN(bytesToBigInt(taggedHash('KeyAgg coefficient', L, pk)));
}

export interface KeyAggResult {
  Q: Pt;
  coefficients: bigint[];
  L: Bytes;
}

/** BIP-327 KeyAgg over 33-byte compressed public keys (order matters). */
export function keyAgg(pks: Bytes[]): KeyAggResult {
  const L = hashKeys(pks);
  const coefficients = pks.map((pk) => keyAggCoeff(pks, pk));
  let Q = Point.ZERO;
  pks.forEach((pk, i) => {
    Q = Q.add(Point.fromBytes(pk).multiply(coefficients[i]!));
  });
  if (Q.is0()) throw new Error('aggregate key is infinity');
  return { Q, coefficients, L };
}

export const keyAggPoints = (keys: Pt[]): KeyAggResult => keyAgg(keys.map(compressed));

/**
 * What happens if an attacker tries the naive rogue-key trick against KeyAgg:
 * they announce P2 = T - P1, but the aggregate is a1*P1 + a2*P2, not P1 + P2.
 * Iterating "fix it up with the new coefficient" never converges.
 */
export function rogueAttemptAgainstKeyAgg(P1: Pt, target: Pt, rounds = 4): { announced: Pt; Q: Pt; hitsTarget: boolean }[] {
  const out: { announced: Pt; Q: Pt; hitsTarget: boolean }[] = [];
  let a1 = 1n;
  for (let i = 0; i < rounds; i++) {
    const announced = target.subtract(mul(a1, P1));
    const res = keyAggPoints([P1, announced]);
    out.push({ announced, Q: res.Q, hitsTarget: res.Q.equals(target) });
    a1 = res.coefficients[0]!;
  }
  return out;
}
