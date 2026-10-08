import { sha256 } from '@noble/hashes/sha2.js';
import { randomBytes } from '@noble/hashes/utils.js';
import { bytesToBigInt, concat, utf8, type Bytes } from './bytes';

/**
 * Deterministic random bit generator for *puzzle content*.
 *
 * Every save file carries a random 32-byte seed. Each puzzle derives its own
 * stream with a label ("ch1/pad", "ch4/warden-key", ...) so that a puzzle looks
 * the same every time the same save is loaded, while two players see different
 * keys, pads and shares. Output block i is SHA-256(seed || label || i).
 *
 * This is a content generator, not a key generator: the only real secrets in
 * the game (the key you create in the epilogue) come from crypto.getRandomValues.
 */
export class Drbg {
  private counter = 0;
  private buf: Bytes = new Uint8Array(0);
  private readonly prefix: Bytes;

  constructor(seed: Bytes, label: string) {
    this.prefix = concat(seed, utf8(`|${label}|`));
  }

  static fresh(label = 'fresh'): Drbg {
    return new Drbg(randomBytes(32), label);
  }

  bytes(n: number): Bytes {
    const out = new Uint8Array(n);
    let o = 0;
    while (o < n) {
      if (this.buf.length === 0) {
        const c = new Uint8Array(4);
        new DataView(c.buffer).setUint32(0, this.counter++);
        this.buf = sha256(concat(this.prefix, c));
      }
      const take = Math.min(n - o, this.buf.length);
      out.set(this.buf.subarray(0, take), o);
      this.buf = this.buf.subarray(take);
      o += take;
    }
    return out;
  }

  /** Uniform bigint in [0, n) by rejection sampling. */
  below(n: bigint): bigint {
    if (n <= 0n) throw new Error('bound must be positive');
    const bits = n.toString(2).length;
    const bytes = Math.ceil(bits / 8);
    const mask = (1n << BigInt(bits)) - 1n;
    for (;;) {
      const x = bytesToBigInt(this.bytes(bytes)) & mask;
      if (x < n) return x;
    }
  }

  /** Uniform bigint in [lo, hi]. */
  range(lo: bigint, hi: bigint): bigint {
    return lo + this.below(hi - lo + 1n);
  }

  /** Uniform integer in [0, n) for small n. */
  int(n: number): number {
    return Number(this.below(BigInt(n)));
  }

  pick<T>(arr: readonly T[]): T {
    return arr[this.int(arr.length)]!;
  }

  shuffle<T>(arr: T[]): T[] {
    for (let i = arr.length - 1; i > 0; i--) {
      const j = this.int(i + 1);
      [arr[i], arr[j]] = [arr[j]!, arr[i]!];
    }
    return arr;
  }

  /** Non-zero scalar modulo n (secp256k1 order by default via caller). */
  scalar(n: bigint): bigint {
    return 1n + this.below(n - 1n);
  }
}
