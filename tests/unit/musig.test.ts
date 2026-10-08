import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { fromHex, toHex, utf8 } from '../../src/crypto/bytes';
import { keyAgg, naiveAggregate, rogueAttemptAgainstKeyAgg, rogueKey } from '../../src/crypto/musig';
import { mul, xOnly } from '../../src/crypto/secp';
import { sign, verify } from '../../src/crypto/bip340';

const v = JSON.parse(readFileSync(new URL('../vectors/key_agg_vectors.json', import.meta.url), 'utf8'));

describe('BIP-327 KeyAgg and the rogue-key attack', () => {
  it('matches the official KeyAgg vectors', () => {
    for (const tc of v.valid_test_cases) {
      const pks = tc.key_indices.map((i: number) => fromHex(v.pubkeys[i]));
      expect(toHex(xOnly(keyAgg(pks).Q)).toUpperCase()).toBe(tc.expected);
    }
  });

  it('naive aggregation lets the last signer take everything', () => {
    const victim = mul(0x1111n);
    const attackerSecret = 0x2222n;
    const announced = rogueKey(mul(attackerSecret), [victim]);
    const Q = naiveAggregate([victim, announced]);
    expect(Q.equals(mul(attackerSecret))).toBe(true);
    const m = utf8('spend the joint coins');
    expect(verify(sign(m, attackerSecret), m, xOnly(Q))).toBe(true);
  });

  it('KeyAgg coefficients make the same trick miss', () => {
    const victim = mul(0x1111n);
    const target = mul(0x2222n);
    for (const step of rogueAttemptAgainstKeyAgg(victim, target, 5)) expect(step.hitsTarget).toBe(false);
  });
});
