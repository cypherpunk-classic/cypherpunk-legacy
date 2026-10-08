import { describe, expect, it } from 'vitest';
import { Drbg } from '../../src/crypto/drbg';
import { TOY_LWE, ciphertextNoise, gaussianSolve, lweDecrypt, lweEncrypt, lweKeygen, residuals } from '../../src/crypto/lwe';
import { dsaKeygen, dsaSign, dsaVerify, kemDecapsulate, kemEncapsulate, kemKeygen, KEM_SIZES } from '../../src/crypto/pq';
import { toHex, utf8 } from '../../src/crypto/bytes';

describe('LWE toy', () => {
  it('decrypts correctly while the noise is small', () => {
    const rng = new Drbg(new Uint8Array(32).fill(1), 'lwe');
    const keys = lweKeygen(TOY_LWE, rng);
    for (let i = 0; i < 40; i++) {
      const bit = (i % 2) as 0 | 1;
      const ct = lweEncrypt(keys, bit, rng);
      expect(Math.abs(ciphertextNoise(keys, ct))).toBeLessThanOrEqual(12);
      expect(lweDecrypt(keys, ct)).toBe(bit);
    }
  });
  it('Gaussian elimination recovers s only without noise', () => {
    const rng = new Drbg(new Uint8Array(32).fill(2), 'lwe');
    const clean = lweKeygen({ ...TOY_LWE, noise: 0 }, rng);
    expect(gaussianSolve(clean.A, clean.b, clean.params.q)).toEqual(clean.s);
    const noisy = lweKeygen({ ...TOY_LWE, noise: 2 }, rng);
    const guess = gaussianSolve(noisy.A, noisy.b, noisy.params.q);
    if (guess) expect(residuals(noisy, guess).some((r) => r !== 0)).toBe(true);
  });
});

describe('FIPS 203 / 204', () => {
  it('ML-KEM-768 agrees on a shared secret', () => {
    const k = kemKeygen();
    expect(k.publicKey.length).toBe(KEM_SIZES.publicKey);
    const { cipherText, sharedSecret } = kemEncapsulate(k.publicKey);
    expect(cipherText.length).toBe(KEM_SIZES.cipherText);
    expect(toHex(kemDecapsulate(cipherText, k.secretKey))).toBe(toHex(sharedSecret));
  });
  it('ML-DSA-65 signs and verifies', () => {
    const k = dsaKeygen();
    const msg = utf8('cypherpunks write code');
    const sig = dsaSign(msg, k.secretKey);
    expect(dsaVerify(sig, msg, k.publicKey)).toBe(true);
    expect(dsaVerify(sig, utf8('cypherpunks write cod3'), k.publicKey)).toBe(false);
  });
});
