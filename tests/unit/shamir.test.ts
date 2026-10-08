import { describe, expect, it } from 'vitest';
import { mod } from '../../src/crypto/bytes';
import { Drbg } from '../../src/crypto/drbg';
import { N } from '../../src/crypto/secp';
import { TOY_P, curveForGuess, deal, evalPoly, feldmanCheck, lagrangeAtZero, quadraticThrough, reconstruct } from '../../src/crypto/shamir';

describe('Shamir and Feldman', () => {
  const rng = new Drbg(new Uint8Array(32).fill(5), 'shamir');
  const secret = rng.scalar(N);
  const d = deal(secret, 3, 5, rng);

  it('any three of five shares rebuild the secret', () => {
    const idx = [0, 1, 2, 3, 4];
    for (const a of idx) for (const b of idx) for (const c of idx) {
      if (!(a < b && b < c)) continue;
      expect(reconstruct([d.shares[a]!, d.shares[b]!, d.shares[c]!])).toBe(secret);
    }
  });

  it('Lagrange coefficients for {1,2,3} are 3, -3, 1', () => {
    const t = lagrangeAtZero([1n, 2n, 3n]);
    expect(t.map((x) => [x.num, x.den])).toEqual([[3n, 1n], [-3n, 1n], [1n, 1n]]);
    expect(t[1]!.lambda).toBe(N - 3n);
  });

  it('Feldman commitments catch a forged share', () => {
    for (const s of d.shares) expect(feldmanCheck(s, d.commitments).ok).toBe(true);
    const forged = { x: 3n, y: mod(d.shares[2]!.y + 1n, N) };
    expect(feldmanCheck(forged, d.commitments).ok).toBe(false);
    expect(reconstruct([d.shares[0]!, d.shares[1]!, forged])).not.toBe(secret);
  });

  it('two shares are consistent with every secret (toy field)', () => {
    const coeffs = [17n, 5n, 11n];
    const known: [bigint, bigint][] = [[2n, evalPoly(coeffs, 2n, TOY_P)], [5n, evalPoly(coeffs, 5n, TOY_P)]];
    for (let s = 0n; s < TOY_P; s++) {
      const c = curveForGuess(s, known);
      expect(evalPoly(c, 0n, TOY_P)).toBe(s);
      for (const [x, y] of known) expect(evalPoly(c, x, TOY_P)).toBe(y);
    }
    expect(quadraticThrough([[0n, 17n], ...known])).toEqual(coeffs);
  });
});
