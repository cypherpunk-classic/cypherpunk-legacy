import { describe, expect, it } from 'vitest';
import { Drbg } from '../../src/crypto/drbg';
import { N, mul } from '../../src/crypto/secp';
import { cheaterCommit, commit, extract, fsProve, fsVerify, respond, simulate, verifyTranscript } from '../../src/crypto/sigma';

describe('Schnorr identification', () => {
  const rng = new Drbg(new Uint8Array(32).fill(9), 'sigma');
  const x = rng.scalar(N);
  const X = mul(x);

  it('is complete', () => {
    const { r, R } = commit(rng);
    const c = rng.scalar(N);
    expect(verifyTranscript(X, { R, c, s: respond(r, c, x) })).toBe(true);
  });

  it('simulated transcripts verify without the secret', () => {
    const t = simulate(X, rng.scalar(N), rng.scalar(N));
    expect(verifyTranscript(X, t)).toBe(true);
  });

  it('rewinding extracts the witness', () => {
    const { r, R } = commit(rng);
    const c1 = 5n, c2 = 11n;
    expect(extract({ R, c: c1, s: respond(r, c1, x) }, { R, c: c2, s: respond(r, c2, x) })).toBe(x);
  });

  it('a cheater with one-bit challenges wins exactly when the bet is right', () => {
    for (const bet of [0, 1] as const) for (const c of [0n, 1n]) {
      const { R, s } = cheaterCommit(X, bet, rng);
      expect(verifyTranscript(X, { R, c, s })).toBe(BigInt(bet) === c);
    }
  });

  it('Fiat–Shamir binds the proof to its context', () => {
    const t = fsProve(x, X, 'checkpoint-7|session-42', rng);
    expect(fsVerify(X, 'checkpoint-7|session-42', t)).toBe(true);
    expect(fsVerify(X, 'checkpoint-7|session-43', t)).toBe(false);
    const weak = { includeKey: true, includeContext: false };
    const replay = fsProve(x, X, 'session-1', rng, weak);
    expect(fsVerify(X, 'session-2', replay, weak)).toBe(true); // the replay bug
  });
});
