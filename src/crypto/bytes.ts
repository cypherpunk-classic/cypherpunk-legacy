/**
 * Byte, bit and modular-arithmetic helpers shared by every cryptographic module.
 *
 * Everything in src/crypto is pure (no DOM, no Phaser) so it can be unit-tested
 * in Node and read on its own as reference material.
 */

export type Bytes = Uint8Array;

const HEX = '0123456789abcdef';

export function toHex(b: Bytes): string {
  let s = '';
  for (let i = 0; i < b.length; i++) s += HEX[b[i]! >> 4]! + HEX[b[i]! & 15]!;
  return s;
}

export function fromHex(h: string): Bytes {
  const clean = h.trim().toLowerCase().replace(/^0x/, '').replace(/\s+/g, '');
  if (clean.length % 2 !== 0 || /[^0-9a-f]/.test(clean)) throw new Error(`invalid hex: ${h.slice(0, 16)}`);
  const out = new Uint8Array(clean.length / 2);
  for (let i = 0; i < out.length; i++) out[i] = parseInt(clean.slice(2 * i, 2 * i + 2), 16);
  return out;
}

export function isHex(h: string): boolean {
  const clean = h.trim().toLowerCase().replace(/^0x/, '');
  return clean.length % 2 === 0 && /^[0-9a-f]*$/.test(clean);
}

export const utf8 = (s: string): Bytes => new TextEncoder().encode(s);
export const fromUtf8 = (b: Bytes): string => new TextDecoder().decode(b);

/** Latin-1 style view: one character per byte, used for XOR puzzles over ASCII. */
export function bytesToAscii(b: Bytes, unprintable = '·'): string {
  let s = '';
  for (const x of b) s += x >= 0x20 && x < 0x7f ? String.fromCharCode(x) : unprintable;
  return s;
}

export function asciiToBytes(s: string): Bytes {
  const out = new Uint8Array(s.length);
  for (let i = 0; i < s.length; i++) out[i] = s.charCodeAt(i) & 0xff;
  return out;
}

export function concat(...parts: Bytes[]): Bytes {
  const len = parts.reduce((a, p) => a + p.length, 0);
  const out = new Uint8Array(len);
  let o = 0;
  for (const p of parts) {
    out.set(p, o);
    o += p.length;
  }
  return out;
}

export function xor(a: Bytes, b: Bytes): Bytes {
  const n = Math.min(a.length, b.length);
  const out = new Uint8Array(n);
  for (let i = 0; i < n; i++) out[i] = a[i]! ^ b[i]!;
  return out;
}

export function equalBytes(a: Bytes, b: Bytes): boolean {
  if (a.length !== b.length) return false;
  let d = 0;
  for (let i = 0; i < a.length; i++) d |= a[i]! ^ b[i]!;
  return d === 0;
}

/** Lexicographic comparison of byte strings (used by BIP341 TapBranch). */
export function compareBytes(a: Bytes, b: Bytes): number {
  const n = Math.min(a.length, b.length);
  for (let i = 0; i < n; i++) if (a[i] !== b[i]) return a[i]! - b[i]!;
  return a.length - b.length;
}

export function bytesToBits(b: Bytes): string {
  let s = '';
  for (const x of b) s += x.toString(2).padStart(8, '0');
  return s;
}

export function bitsToBytes(bits: string): Bytes {
  if (bits.length % 8 !== 0) throw new Error('bit length must be a multiple of 8');
  const out = new Uint8Array(bits.length / 8);
  for (let i = 0; i < out.length; i++) out[i] = parseInt(bits.slice(8 * i, 8 * i + 8), 2);
  return out;
}

// ---------------------------------------------------------------------------
// bigint <-> bytes
// ---------------------------------------------------------------------------

export function bytesToBigInt(b: Bytes): bigint {
  return b.length === 0 ? 0n : BigInt('0x' + toHex(b));
}

export function bigIntToBytes(x: bigint, len = 32): Bytes {
  if (x < 0n) throw new Error('negative');
  const h = x.toString(16).padStart(len * 2, '0');
  if (h.length > len * 2) throw new Error('integer too large for length');
  return fromHex(h);
}

export const bigToHex = (x: bigint, len = 32): string => toHex(bigIntToBytes(x, len));

export function bitLength(x: bigint): number {
  if (x < 0n) x = -x;
  return x === 0n ? 0 : x.toString(2).length;
}

/** log2 of a positive bigint with ~50 bits of precision. */
export function log2Big(x: bigint): number {
  if (x <= 0n) return -Infinity;
  const bl = bitLength(x);
  if (bl <= 52) return Math.log2(Number(x));
  const shift = BigInt(bl - 52);
  return Math.log2(Number(x >> shift)) + Number(shift);
}

// ---------------------------------------------------------------------------
// Modular arithmetic on bigint
// ---------------------------------------------------------------------------

export function mod(a: bigint, m: bigint): bigint {
  const r = a % m;
  return r < 0n ? r + m : r;
}

/** Floor division that is correct for negative numerators. */
export function floorDiv(a: bigint, b: bigint): bigint {
  const q = a / b;
  return (a % b !== 0n) && ((a < 0n) !== (b < 0n)) ? q - 1n : q;
}

/** Nearest integer to a/b (ties round up), exact for bigint. */
export function roundDiv(a: bigint, b: bigint): bigint {
  if (b < 0n) {
    a = -a;
    b = -b;
  }
  return floorDiv(2n * a + b, 2n * b);
}

export function egcd(a: bigint, b: bigint): [bigint, bigint, bigint] {
  let [oldR, r] = [a, b];
  let [oldS, s] = [1n, 0n];
  let [oldT, t] = [0n, 1n];
  while (r !== 0n) {
    const q = floorDiv(oldR, r);
    [oldR, r] = [r, oldR - q * r];
    [oldS, s] = [s, oldS - q * s];
    [oldT, t] = [t, oldT - q * t];
  }
  return [oldR, oldS, oldT];
}

export function modInv(a: bigint, m: bigint): bigint {
  const [g, x] = egcd(mod(a, m), m);
  if (g !== 1n && g !== -1n) throw new Error('not invertible');
  return mod(x * g, m);
}

export function modPow(base: bigint, exp: bigint, m: bigint): bigint {
  if (m === 1n) return 0n;
  let result = 1n;
  let b = mod(base, m);
  let e = exp;
  if (e < 0n) {
    b = modInv(b, m);
    e = -e;
  }
  while (e > 0n) {
    if (e & 1n) result = (result * b) % m;
    b = (b * b) % m;
    e >>= 1n;
  }
  return result;
}

export function gcd(a: bigint, b: bigint): bigint {
  a = a < 0n ? -a : a;
  b = b < 0n ? -b : b;
  while (b) [a, b] = [b, a % b];
  return a;
}

/** Deterministic Miller–Rabin, adequate for the small parameters used in puzzles. */
export function isProbablePrime(n: bigint): boolean {
  if (n < 2n) return false;
  const small = [2n, 3n, 5n, 7n, 11n, 13n, 17n, 19n, 23n, 29n, 31n, 37n];
  for (const p of small) {
    if (n === p) return true;
    if (n % p === 0n) return false;
  }
  let d = n - 1n;
  let r = 0;
  while ((d & 1n) === 0n) {
    d >>= 1n;
    r++;
  }
  outer: for (const a of small) {
    let x = modPow(a, d, n);
    if (x === 1n || x === n - 1n) continue;
    for (let i = 1; i < r; i++) {
      x = (x * x) % n;
      if (x === n - 1n) continue outer;
    }
    return false;
  }
  return true;
}

/** Abbreviate long hex for display: 0a1b2c…7e8f */
export function shortHex(h: string, head = 8, tail = 6): string {
  return h.length <= head + tail + 1 ? h : `${h.slice(0, head)}…${h.slice(-tail)}`;
}
