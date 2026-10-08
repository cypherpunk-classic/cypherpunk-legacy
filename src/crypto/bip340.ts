import { bigIntToBytes, bytesToBigInt, concat, toHex, type Bytes } from './bytes';
import { G, N, hasEvenY, invN, liftX, modN, mul, schnorr, taggedHash, xOnly, type Pt } from './secp';

/**
 * BIP-340 Schnorr signatures over secp256k1.
 *
 *   sign:   R = k*G (k negated so R has even y), e = H_challenge(R.x || P.x || m) mod n,
 *           s = k + e*d mod n                    signature = R.x || s
 *   verify: s*G == R + e*P
 *
 * Real signing goes through @noble/curves (deterministic nonce derived from the
 * key, aux randomness and the message). `signWithNonce` exists only so the game
 * can reproduce a broken device that reuses k, the bug that leaks the key.
 */

export interface Sig {
  r: Bytes; // R.x, 32 bytes
  s: bigint;
}

export const sigBytes = (sig: Sig): Bytes => concat(sig.r, bigIntToBytes(sig.s, 32));
export const sigHex = (sig: Sig): string => toHex(sigBytes(sig));

export function parseSig(b: Bytes): Sig {
  if (b.length !== 64) throw new Error('signature must be 64 bytes');
  return { r: b.slice(0, 32), s: bytesToBigInt(b.slice(32)) };
}

/** BIP-340 secret key normalisation: the key whose public point has even y. */
export function evenKey(d: bigint): bigint {
  const P = mul(d);
  return hasEvenY(P) ? modN(d) : N - modN(d);
}

/** e = int(hash_BIP0340/challenge(R.x || P.x || m)) mod n */
export function challenge(rx: Bytes, px: Bytes, m: Bytes): bigint {
  return modN(bytesToBigInt(taggedHash('BIP0340/challenge', rx, px, m)));
}

export function sign(m: Bytes, secretKey: bigint, auxRand?: Bytes): Sig {
  return parseSig(schnorr.sign(m, bigIntToBytes(modN(secretKey), 32), auxRand));
}

export function verify(sig: Sig | Bytes, m: Bytes, pubX: Bytes): boolean {
  try {
    return schnorr.verify(sig instanceof Uint8Array ? sig : sigBytes(sig), m, pubX);
  } catch {
    return false;
  }
}

export function publicKey(secretKey: bigint): Bytes {
  return xOnly(mul(secretKey));
}

/** Sign with a caller-chosen nonce k. Never do this outside a museum of broken devices. */
export function signWithNonce(m: Bytes, secretKey: bigint, k0: bigint): Sig {
  const d = evenKey(secretKey);
  const P = mul(d);
  let k = modN(k0);
  const R = mul(k);
  if (!hasEvenY(R)) k = N - k;
  const rx = xOnly(R);
  const e = challenge(rx, xOnly(P), m);
  return { r: rx, s: modN(k + e * d) };
}

/** The pieces of the verification equation, for showing on screen. */
export function explainVerify(sig: Sig, m: Bytes, pubX: Bytes): { e: bigint; lhs: Pt; rhs: Pt; ok: boolean } {
  const P = liftX(pubX);
  const R = liftX(sig.r);
  const e = challenge(sig.r, pubX, m);
  const lhs = mul(sig.s, G);
  const rhs = R.add(mul(e, P));
  return { e, lhs, rhs, ok: lhs.equals(rhs) };
}

export interface NonceReuseSolution {
  e1: bigint;
  e2: bigint;
  sDiff: bigint;
  eDiff: bigint;
  eDiffInv: bigint;
  d: bigint;
}

/**
 * Two signatures with the same R under the same key:
 *   s1 = k + e1*d,  s2 = k + e2*d   =>   d = (s1 - s2) / (e1 - e2)  mod n
 */
export function recoverFromNonceReuse(pubX: Bytes, m1: Bytes, sig1: Sig, m2: Bytes, sig2: Sig): NonceReuseSolution {
  if (toHex(sig1.r) !== toHex(sig2.r)) throw new Error('signatures do not share a nonce');
  const e1 = challenge(sig1.r, pubX, m1);
  const e2 = challenge(sig2.r, pubX, m2);
  if (e1 === e2) throw new Error('identical challenges');
  const sDiff = modN(sig1.s - sig2.s);
  const eDiff = modN(e1 - e2);
  const eDiffInv = invN(eDiff);
  const d = modN(sDiff * eDiffInv);
  return { e1, e2, sDiff, eDiff, eDiffInv, d };
}
