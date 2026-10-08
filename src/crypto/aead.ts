import { ctr, gcm } from '@noble/ciphers/aes.js';
import { sha256 } from '@noble/hashes/sha2.js';
import { hkdf } from '@noble/hashes/hkdf.js';
import { asciiToBytes, bytesToAscii, concat, utf8, fromUtf8, type Bytes } from './bytes';

/**
 * Encryption alone vs. authenticated encryption.
 *
 * AES-CTR turns AES into a stream cipher: C = P xor AES_K(nonce || counter...).
 * Flip bit i of the ciphertext and bit i of the plaintext flips, silently.
 * AES-GCM is CTR plus a GHASH authentication tag over the ciphertext; any
 * change makes decryption fail instead of producing an attacker-chosen message.
 */

export interface CtrTicket {
  nonce: Bytes; // 16-byte initial counter block
  ct: Bytes;
}

export function ctrEncrypt(key: Bytes, nonce: Bytes, plaintext: string): CtrTicket {
  return { nonce, ct: ctr(key, nonce).encrypt(asciiToBytes(plaintext)) };
}

export function ctrDecrypt(key: Bytes, t: CtrTicket): string {
  return bytesToAscii(ctr(key, t.nonce).decrypt(t.ct), '?');
}

export interface GcmBox {
  nonce: Bytes; // 12 bytes
  ct: Bytes; // ciphertext || 16-byte tag
}

export function gcmEncrypt(key: Bytes, nonce: Bytes, plaintext: Bytes, aad?: Bytes): GcmBox {
  return { nonce, ct: gcm(key, nonce, aad).encrypt(plaintext) };
}

export type Opened = { ok: true; plaintext: Bytes } | { ok: false; error: string };

export function gcmDecrypt(key: Bytes, box: GcmBox, aad?: Bytes): Opened {
  try {
    return { ok: true, plaintext: gcm(key, box.nonce, aad).decrypt(box.ct) };
  } catch {
    return { ok: false, error: 'authentication tag mismatch' };
  }
}

export const gcmEncryptText = (key: Bytes, nonce: Bytes, text: string, aad?: Bytes): GcmBox =>
  gcmEncrypt(key, nonce, utf8(text), aad);

export function gcmDecryptText(key: Bytes, box: GcmBox, aad?: Bytes): { ok: true; text: string } | { ok: false; error: string } {
  const r = gcmDecrypt(key, box, aad);
  return r.ok ? { ok: true, text: fromUtf8(r.plaintext) } : r;
}

/** Flip one bit of a byte array (returns a copy). */
export function flipBit(b: Bytes, bitIndex: number): Bytes {
  const out = b.slice();
  out[bitIndex >> 3]! ^= 0x80 >> (bitIndex & 7);
  return out;
}

/** The malleability attack in one line: C' = C xor (P xor P'). */
export function forgeCtr(t: CtrTicket, known: string, wanted: string): CtrTicket {
  if (known.length !== wanted.length) throw new Error('same length only');
  const delta = asciiToBytes(known).map((b, i) => b ^ wanted.charCodeAt(i));
  return { nonce: t.nonce, ct: t.ct.map((b, i) => b ^ (delta[i] ?? 0)) };
}

/** Derive a 32-byte symmetric key from arbitrary secret material. */
export function kdf(secret: Bytes, info: string, salt = 'cypherpunk-legacy'): Bytes {
  return hkdf(sha256, secret, utf8(salt), utf8(info), 32);
}

export function sha256Hex(b: Bytes): string {
  return Array.from(sha256(b), (x) => x.toString(16).padStart(2, '0')).join('');
}

export { concat };
