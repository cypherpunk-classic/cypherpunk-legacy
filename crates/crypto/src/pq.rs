//! Post-quantum primitives standardised by NIST in August 2024:
//!   ML-KEM (FIPS 203, from CRYSTALS-Kyber): key encapsulation from Module-LWE.
//!   ML-DSA (FIPS 204, from CRYSTALS-Dilithium): signatures from Module-LWE/SIS.
//! Implementations: RustCrypto's `ml-kem` and `ml-dsa` (pure Rust).

use ml_dsa::{Keypair, MlDsa65, SigningKey as DsaSigningKey, VerifyingKey as DsaVerifyingKey};
use ml_kem::{Decapsulate, DecapsulationKey, EncapsulationKey, KeyExport, MlKem768};

use crate::drbg::random_bytes;

pub struct KemSizes {
    pub public_key: usize,
    pub secret_key: usize,
    pub cipher_text: usize,
    pub shared_secret: usize,
}

/// FIPS 203 sizes for ML-KEM-768 (secret key in its expanded 2400-byte form).
pub const KEM_SIZES: KemSizes = KemSizes { public_key: 1184, secret_key: 2400, cipher_text: 1088, shared_secret: 32 };

pub struct DsaSizes {
    pub public_key: usize,
    pub signature: usize,
}

/// FIPS 204 sizes for ML-DSA-65.
pub const DSA_SIZES: DsaSizes = DsaSizes { public_key: 1952, signature: 3309 };

/// An ML-KEM-768 key pair, kept as its 64-byte seed (d || z).
pub struct KemKeys {
    pub seed: [u8; 64],
    dk: DecapsulationKey<MlKem768>,
}

impl KemKeys {
    pub fn from_seed(seed: [u8; 64]) -> Self {
        let dk = DecapsulationKey::<MlKem768>::from_seed(seed.into());
        Self { seed, dk }
    }

    pub fn generate() -> Self {
        Self::from_seed(random_bytes::<64>())
    }

    pub fn public_key(&self) -> Vec<u8> {
        self.dk.encapsulation_key().to_bytes().to_vec()
    }

    pub fn decapsulate(&self, ct: &[u8]) -> Result<[u8; 32], String> {
        let ct = ct.try_into().map_err(|_| format!("ciphertext must be {} bytes", KEM_SIZES.cipher_text))?;
        Ok(self.dk.decapsulate(&ct).into())
    }
}

/// Encapsulate to a public key with explicit randomness m (deterministic, for reproducible scenes).
pub fn kem_encapsulate_with(public_key: &[u8], m: [u8; 32]) -> Result<(Vec<u8>, [u8; 32]), String> {
    let key = public_key.try_into().map_err(|_| format!("public key must be {} bytes", KEM_SIZES.public_key))?;
    let ek = EncapsulationKey::<MlKem768>::new(&key).map_err(|_| "invalid encapsulation key".to_string())?;
    let (ct, ss) = ek.encapsulate_deterministic(&m.into());
    Ok((ct.to_vec(), ss.into()))
}

pub fn kem_encapsulate(public_key: &[u8]) -> Result<(Vec<u8>, [u8; 32]), String> {
    kem_encapsulate_with(public_key, random_bytes::<32>())
}

/// An ML-DSA-65 key pair from a 32-byte seed (FIPS 204 KeyGen_internal).
pub struct DsaKeys {
    pub seed: [u8; 32],
    sk: DsaSigningKey<MlDsa65>,
}

impl DsaKeys {
    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self { seed, sk: DsaSigningKey::<MlDsa65>::from_seed(&seed.into()) }
    }

    pub fn generate() -> Self {
        Self::from_seed(random_bytes::<32>())
    }

    pub fn public_key(&self) -> Vec<u8> {
        self.sk.verifying_key().encode().to_vec()
    }

    /// Deterministic ML-DSA signature with an empty context string.
    pub fn sign(&self, msg: &[u8]) -> Vec<u8> {
        self.sk.expanded_key().sign_deterministic(msg, b"").expect("empty context").encode().to_vec()
    }
}

pub fn dsa_verify(sig: &[u8], msg: &[u8], public_key: &[u8]) -> bool {
    let Ok(pk) = public_key.try_into() else { return false };
    let vk = DsaVerifyingKey::<MlDsa65>::decode(&pk);
    let Ok(enc) = sig.try_into() else { return false };
    let Some(sig) = ml_dsa::Signature::<MlDsa65>::decode(&enc) else { return false };
    vk.verify_with_context(msg, b"", &sig)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mlkem_and_mldsa_roundtrip() {
        let keys = KemKeys::from_seed([7u8; 64]);
        let pk = keys.public_key();
        assert_eq!(pk.len(), KEM_SIZES.public_key);
        let (ct, ss) = kem_encapsulate_with(&pk, [9u8; 32]).unwrap();
        assert_eq!(ct.len(), KEM_SIZES.cipher_text);
        assert_eq!(keys.decapsulate(&ct).unwrap(), ss);
        let mut bad = ct.clone();
        bad[0] ^= 1;
        assert_ne!(keys.decapsulate(&bad).unwrap(), ss); // implicit rejection

        let dsa = DsaKeys::from_seed([3u8; 32]);
        let pk = dsa.public_key();
        assert_eq!(pk.len(), DSA_SIZES.public_key);
        let sig = dsa.sign(b"harvest now, decrypt later");
        assert_eq!(sig.len(), DSA_SIZES.signature);
        assert!(dsa_verify(&sig, b"harvest now, decrypt later", &pk));
        assert!(!dsa_verify(&sig, b"harvest later", &pk));
    }
}
