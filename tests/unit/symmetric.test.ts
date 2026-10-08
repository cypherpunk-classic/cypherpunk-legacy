import { describe, expect, it } from 'vitest';
import { asciiToBytes, bytesToAscii, toHex, utf8, xor } from '../../src/crypto/bytes';
import { Drbg } from '../../src/crypto/drbg';
import { cribDrag, makeTwoTimePad, partner } from '../../src/crypto/otp';
import { ctrDecrypt, ctrEncrypt, flipBit, forgeCtr, gcmDecryptText, gcmEncryptText, kdf } from '../../src/crypto/aead';

const rng = () => new Drbg(new Uint8Array(32).fill(7), 'test');

describe('one-time pad reuse', () => {
  const P1 = 'MEET ME AT THE WHARF AT MIDNIGHT';
  const P2 = 'THE VAULT KEY IS SPLIT FIVE WAYS';
  it('the pad cancels out', () => {
    const t = makeTwoTimePad(P1, P2, rng());
    expect(toHex(t.x)).toBe(toHex(xor(asciiToBytes(P1), asciiToBytes(P2))));
    expect(bytesToAscii(xor(t.c1, t.pad))).toBe(P1);
  });
  it('a correct crib reveals the partner text', () => {
    const t = makeTwoTimePad(P1, P2, rng());
    expect(partner(t.x, 'WHARF', 15)).toBe(P2.slice(15, 20));
    const hits = cribDrag(t.x, ' THE ');
    const best = hits.reduce((a, b) => (b.score > a.score ? b : a));
    expect(best.score).toBe(1);
  });
});

describe('malleability vs authenticated encryption', () => {
  const key = new Drbg(new Uint8Array(32), 'k').bytes(32);
  const nonce = new Drbg(new Uint8Array(32), 'n').bytes(16);
  it('CTR: flipping ciphertext bits flips plaintext bits', () => {
    const ticket = ctrEncrypt(key, nonce, 'ROLE=GUEST;NAME=ANON');
    const forged = forgeCtr(ticket, 'ROLE=GUEST;NAME=ANON', 'ROLE=ADMIN;NAME=ANON');
    expect(ctrDecrypt(key, forged)).toBe('ROLE=ADMIN;NAME=ANON');
  });
  it('GCM: any flipped bit is rejected', () => {
    const n12 = nonce.slice(0, 12);
    const box = gcmEncryptText(key, n12, 'ROLE=GUEST;NAME=ANON');
    expect(gcmDecryptText(key, box)).toEqual({ ok: true, text: 'ROLE=GUEST;NAME=ANON' });
    for (const bit of [0, 7, 45, 100, box.ct.length * 8 - 1]) {
      expect(gcmDecryptText(key, { nonce: n12, ct: flipBit(box.ct, bit) }).ok).toBe(false);
    }
  });
  it('kdf is deterministic and domain separated', () => {
    expect(toHex(kdf(utf8('x'), 'a'))).toBe(toHex(kdf(utf8('x'), 'a')));
    expect(toHex(kdf(utf8('x'), 'a'))).not.toBe(toHex(kdf(utf8('x'), 'b')));
  });
});
