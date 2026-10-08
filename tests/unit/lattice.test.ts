import { describe, expect, it } from 'vitest';
import { Drbg } from '../../src/crypto/drbg';
import { gaussReduce, gramDeterminants, isGaussReduced, isLllReduced, lll, n2, det2, type Basis } from '../../src/crypto/lattice';
import { attackBlock, bitsToText, density, makeKnapsackPuzzle, mhDecrypt, textToBits } from '../../src/crypto/knapsack';

describe('LLL', () => {
  it('returns an LLL-reduced basis of the same lattice', () => {
    const rng = new Drbg(new Uint8Array(32).fill(2), 'lll');
    for (let trial = 0; trial < 5; trial++) {
      const n = 6;
      const B: Basis = Array.from({ length: n }, () => Array.from({ length: n }, () => rng.range(-1000n, 1000n)));
      const before = gramDeterminants(B);
      if (before[n] === 0n) continue;
      const res = lll(B);
      expect(isLllReduced(res.basis)).toBe(true);
      expect(gramDeterminants(res.basis)[n]).toBe(before[n]); // same covolume
      expect(res.profileAfter.length).toBe(n);
    }
  });
});

describe('Lagrange–Gauss in two dimensions', () => {
  it('finds a shortest vector and keeps the area', () => {
    const s0 = { b1: [1013n, 397n] as [bigint, bigint], b2: [1271n, 498n] as [bigint, bigint] };
    const { state } = gaussReduce(s0);
    expect(isGaussReduced(state)).toBe(true);
    expect(det2(state) === det2(s0) || det2(state) === -det2(s0)).toBe(true);
    let best = n2(state.b1);
    for (let a = -60n; a <= 60n; a++) for (let b = -60n; b <= 60n; b++) {
      if (a === 0n && b === 0n) continue;
      const v: [bigint, bigint] = [a * s0.b1[0] + b * s0.b2[0], a * s0.b1[1] + b * s0.b2[1]];
      if (n2(v) < best) best = n2(v);
    }
    expect(n2(state.b1)).toBe(best);
  });
});

describe('Merkle–Hellman falls to LLL', () => {
  it('decrypts with the private key and without it', () => {
    const rng = new Drbg(new Uint8Array(32).fill(4), 'mh');
    const msg = 'HARVEST GOES LIVE AT DAWN';
    const { keys, blocks: cts } = makeKnapsackPuzzle(rng, msg);
    expect(density(keys.pub)).toBeLessThan(0.7);
    const padded = msg.padEnd(cts.length * 4, ' ');
    let recovered = '';
    cts.forEach((c, i) => {
      expect(bitsToText(mhDecrypt(keys.priv, c))).toBe(padded.slice(i * 4, i * 4 + 4));
      const atk = attackBlock(keys.pub, c);
      if (atk.bits) recovered += bitsToText(atk.bits);
      else recovered += '????';
    });
    expect(recovered.trimEnd()).toBe(msg);
    expect(textToBits('A')).toEqual([0, 1, 0, 0, 0, 0, 0, 1]);
  });
});
