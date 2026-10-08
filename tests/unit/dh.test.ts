import { describe, expect, it } from 'vitest';
import { isProbablePrime, modPow } from '../../src/crypto/bytes';
import { Drbg } from '../../src/crypto/drbg';
import { DRONE_GROUP, TOY_GROUP, WEAK_GROUP, bsgs, isGroupElement, keypair, pub, sessionKey, shared } from '../../src/crypto/dh';
import { gcmDecryptText, gcmEncryptText } from '../../src/crypto/aead';

describe('Diffie–Hellman', () => {
  it('uses safe primes and generators of prime order', () => {
    for (const g of [TOY_GROUP, WEAK_GROUP, DRONE_GROUP]) {
      expect(isProbablePrime(g.p)).toBe(true);
      expect(isProbablePrime(g.q)).toBe(true);
      expect(g.p).toBe(2n * g.q + 1n);
      expect(modPow(g.g, g.q, g.p)).toBe(1n);
      expect(g.g).not.toBe(1n);
    }
  });

  it('agrees on a key, and a relay in the middle can read and rewrite everything', () => {
    const rng = new Drbg(new Uint8Array(32).fill(1), 'dh');
    const grp = DRONE_GROUP;
    const drone = keypair(grp, rng);
    const command = keypair(grp, rng);
    expect(shared(grp, drone.secret, command.public)).toBe(shared(grp, command.secret, drone.public));

    const mallory = keypair(grp, rng);
    const kDrone = shared(grp, drone.secret, mallory.public); // drone thinks this is command
    const kCommand = shared(grp, command.secret, mallory.public);
    expect(shared(grp, mallory.secret, drone.public)).toBe(kDrone);
    expect(shared(grp, mallory.secret, command.public)).toBe(kCommand);

    const n = new Uint8Array(12);
    const report = gcmEncryptText(sessionKey(grp, kDrone), n, 'TARGET: THE EXCHANGE');
    const read = gcmDecryptText(sessionKey(grp, kDrone), report);
    expect(read.ok && read.text).toBe('TARGET: THE EXCHANGE');
    const forged = gcmEncryptText(sessionKey(grp, kCommand), n, 'ALL CLEAR');
    const got = gcmDecryptText(sessionKey(grp, kCommand), forged);
    expect(got.ok && got.text).toBe('ALL CLEAR');
    expect(isGroupElement(grp, mallory.public)).toBe(true);
  });

  it('baby-step giant-step breaks a 31-bit group', () => {
    const x = 1234567891n % WEAK_GROUP.q;
    const res = bsgs(WEAK_GROUP, pub(WEAK_GROUP, x));
    expect(res.x).toBe(x);
    expect(res.babySteps).toBeLessThan(50_000);
  });
});
