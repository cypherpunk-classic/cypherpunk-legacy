import { ml_kem768 } from '@noble/post-quantum/ml-kem.js';
import { ml_dsa65 } from '@noble/post-quantum/ml-dsa.js';
import { randomBytes } from '@noble/hashes/utils.js';
import type { Bytes } from './bytes';

/**
 * Post-quantum primitives standardised by NIST in August 2024:
 *   ML-KEM (FIPS 203, from CRYSTALS-Kyber): key encapsulation from Module-LWE.
 *   ML-DSA (FIPS 204, from CRYSTALS-Dilithium): signatures from Module-LWE/SIS.
 * Implementations: @noble/post-quantum (pure TypeScript).
 */

export interface KemKeys {
  publicKey: Bytes;
  secretKey: Bytes;
}

export const kemKeygen = (seed?: Bytes): KemKeys => ml_kem768.keygen(seed);

export function kemEncapsulate(publicKey: Bytes): { cipherText: Bytes; sharedSecret: Bytes } {
  return ml_kem768.encapsulate(publicKey);
}

export const kemDecapsulate = (cipherText: Bytes, secretKey: Bytes): Bytes => ml_kem768.decapsulate(cipherText, secretKey);

export const KEM_SIZES = { publicKey: 1184, secretKey: 2400, cipherText: 1088, sharedSecret: 32 } as const;

export interface DsaKeys {
  publicKey: Bytes;
  secretKey: Bytes;
}

export const dsaKeygen = (seed: Bytes = randomBytes(32)): DsaKeys => ml_dsa65.keygen(seed);
export const dsaSign = (msg: Bytes, secretKey: Bytes): Bytes => ml_dsa65.sign(msg, secretKey);
export const dsaVerify = (sig: Bytes, msg: Bytes, publicKey: Bytes): boolean => {
  try {
    return ml_dsa65.verify(sig, msg, publicKey);
  } catch {
    return false;
  }
};

export const DSA_SIZES = { publicKey: 1952, signature: 3309 } as const;
