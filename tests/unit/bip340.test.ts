import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { bytesToBigInt, fromHex, toHex, utf8 } from '../../src/crypto/bytes';
import { evenKey, explainVerify, publicKey, recoverFromNonceReuse, sign, signWithNonce, sigHex, verify } from '../../src/crypto/bip340';

const rows = readFileSync(new URL('../vectors/test-vectors.csv', import.meta.url), 'utf8').trim().split('\n').slice(1).map((l: string) => l.split(','));

describe('BIP-340', () => {
  it('passes the official signing and verification vectors', () => {
    for (const [, sk, pk, aux, msg, sig, result] of rows) {
      if (sk) expect(sigHex(sign(fromHex(msg!), bytesToBigInt(fromHex(sk)), fromHex(aux!))).toUpperCase()).toBe(sig);
      expect(verify(fromHex(sig!), fromHex(msg!), fromHex(pk!))).toBe(result === 'TRUE');
    }
  });

  it('a reused nonce gives the key away', () => {
    const d = 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdefn;
    const P = publicKey(d);
    const k = 0xdeadbeefcafebabedeadbeefcafebabedeadbeefcafebabedeadbeefcafebaben;
    const m1 = utf8('ORDER 1: SEIZE THE ARCHIVE');
    const m2 = utf8('ORDER 2: MONITOR THE WHARF');
    const s1 = signWithNonce(m1, d, k);
    const s2 = signWithNonce(m2, d, k);
    expect(verify(s1, m1, P)).toBe(true);
    expect(verify(s2, m2, P)).toBe(true);
    expect(toHex(s1.r)).toBe(toHex(s2.r));
    const sol = recoverFromNonceReuse(P, m1, s1, m2, s2);
    expect(sol.d).toBe(evenKey(d));
    const forged = utf8('RELEASE THE ARCHIVE');
    expect(verify(sign(forged, sol.d), forged, P)).toBe(true);
    expect(explainVerify(s1, m1, P).ok).toBe(true);
  });
});
