//! Encrypted secret chain for epoch-based secret rotation.
//!
//! Each epoch's secret is encrypted with the next epoch's key, forming
//! a chain. Any node that holds the current epoch's secret can decrypt
//! all prior secrets by walking backwards through the chain.
//!
//! This is the Oxide trust-quorum pattern: on reconfiguration, the new
//! secret encrypts all prior secrets into a chain. Removed nodes' old
//! shares become useless for the current epoch.

use std::collections::BTreeMap;

use chacha20poly1305::aead::Aead;
use chacha20poly1305::aead::KeyInit;
use chacha20poly1305::ChaCha20Poly1305;
use chacha20poly1305::Nonce;
use serde::Deserialize;
use serde::Serialize;
use snafu::Snafu;

use crate::kdf;

/// Context for deriving chain encryption keys.
const CONTEXT_CHAIN_ENCRYPTION: &[u8] = b"aspen-v1-chain-encryption";

/// An encrypted chain of prior epoch secrets.
///
/// At epoch N, the chain contains secrets for epochs 1..N-1, encrypted
/// under a key derived from epoch N's secret.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncryptedSecretChain {
    /// Random salt for key derivation (unique per chain).
    pub salt: [u8; 32],
    /// Encrypted data: serialized `BTreeMap<u64, [u8; 32]>` (epoch → secret bytes).
    pub data: Vec<u8>,
    /// The epoch this chain was created at (encrypting key epoch).
    pub epoch: u64,
    /// Number of prior secrets in the chain.
    pub prior_count: u32,
}

/// Errors from chain operations.
#[derive(Debug, Snafu)]
pub enum ChainError {
    /// AEAD encryption failed.
    #[snafu(display("chain encryption failed"))]
    EncryptFailed,

    /// AEAD decryption failed (wrong key or tampered data).
    #[snafu(display("chain decryption failed: wrong key or tampered data"))]
    DecryptFailed,

    /// Serialization error.
    #[snafu(display("chain serialization error: {reason}"))]
    Serialize { reason: String },

    /// Deserialization error.
    #[snafu(display("chain deserialization error: {reason}"))]
    Deserialize { reason: String },
}

/// Encrypt a map of prior secrets into a chain using the new epoch's secret.
///
/// The chain is encrypted with a key derived from `new_secret` via HKDF,
/// bound to the cluster ID and epoch. A random salt ensures the derived key
/// is unique even if the same secret is reused (which shouldn't happen, but
/// defense in depth).
pub fn encrypt_chain(
    prior_secrets: &BTreeMap<u64, [u8; 32]>,
    new_secret: &[u8; 32],
    cluster_id: &[u8],
    epoch: u64,
) -> Result<EncryptedSecretChain, ChainError> {
    // Generate random salt
    let mut salt = [0u8; 32];
    rand::fill(&mut salt);

    // Derive chain encryption key
    let mut info = Vec::with_capacity(CONTEXT_CHAIN_ENCRYPTION.len() + salt.len() + cluster_id.len() + 8);
    info.extend_from_slice(CONTEXT_CHAIN_ENCRYPTION);
    info.extend_from_slice(&salt);
    info.extend_from_slice(cluster_id);
    info.extend_from_slice(&epoch.to_be_bytes());

    let chain_key = kdf::derive_key(new_secret, &info, &[], 0);

    // Serialize the prior secrets map
    let plaintext =
        serde_json::to_vec(prior_secrets).map_err(|e| ChainError::Serialize { reason: e.to_string() })?;

    // Encrypt with ChaCha20-Poly1305
    let cipher = ChaCha20Poly1305::new((&chain_key).into());
    let nonce = Nonce::from_slice(&[0u8; 12]); // Fixed nonce OK since key is unique per chain
    let ciphertext = cipher.encrypt(nonce, plaintext.as_ref()).map_err(|_| ChainError::EncryptFailed)?;

    Ok(EncryptedSecretChain {
        salt,
        data: ciphertext,
        epoch,
        prior_count: prior_secrets.len() as u32,
    })
}

/// Decrypt a chain using the epoch's secret, recovering all prior secrets.
pub fn decrypt_chain(
    chain: &EncryptedSecretChain,
    secret: &[u8; 32],
    cluster_id: &[u8],
) -> Result<BTreeMap<u64, [u8; 32]>, ChainError> {
    // Derive the same chain encryption key
    let mut info = Vec::with_capacity(CONTEXT_CHAIN_ENCRYPTION.len() + chain.salt.len() + cluster_id.len() + 8);
    info.extend_from_slice(CONTEXT_CHAIN_ENCRYPTION);
    info.extend_from_slice(&chain.salt);
    info.extend_from_slice(cluster_id);
    info.extend_from_slice(&chain.epoch.to_be_bytes());

    let chain_key = kdf::derive_key(secret, &info, &[], 0);

    // Decrypt
    let cipher = ChaCha20Poly1305::new((&chain_key).into());
    let nonce = Nonce::from_slice(&[0u8; 12]);
    let plaintext = cipher.decrypt(nonce, chain.data.as_ref()).map_err(|_| ChainError::DecryptFailed)?;

    // Deserialize
    let secrets: BTreeMap<u64, [u8; 32]> =
        serde_json::from_slice(&plaintext).map_err(|e| ChainError::Deserialize { reason: e.to_string() })?;

    Ok(secrets)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_secret(seed: u8) -> [u8; 32] {
        let mut s = [0u8; 32];
        s[0] = seed;
        s[31] = seed.wrapping_mul(7);
        s
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let mut prior = BTreeMap::new();
        prior.insert(1, make_secret(0xAA));
        prior.insert(2, make_secret(0xBB));

        let new_secret = make_secret(0xCC);
        let cluster_id = b"test-cluster";

        let chain = encrypt_chain(&prior, &new_secret, cluster_id, 3).unwrap();
        assert_eq!(chain.epoch, 3);
        assert_eq!(chain.prior_count, 2);

        let recovered = decrypt_chain(&chain, &new_secret, cluster_id).unwrap();
        assert_eq!(recovered, prior);
    }

    #[test]
    fn test_tampered_ciphertext_fails() {
        let mut prior = BTreeMap::new();
        prior.insert(1, make_secret(0xAA));

        let new_secret = make_secret(0xCC);
        let mut chain = encrypt_chain(&prior, &new_secret, b"cluster", 2).unwrap();

        // Tamper
        if let Some(byte) = chain.data.first_mut() {
            *byte ^= 0xFF;
        }

        let result = decrypt_chain(&chain, &new_secret, b"cluster");
        assert!(matches!(result, Err(ChainError::DecryptFailed)));
    }

    #[test]
    fn test_wrong_secret_fails() {
        let mut prior = BTreeMap::new();
        prior.insert(1, make_secret(0xAA));

        let new_secret = make_secret(0xCC);
        let chain = encrypt_chain(&prior, &new_secret, b"cluster", 2).unwrap();

        let wrong = make_secret(0xDD);
        let result = decrypt_chain(&chain, &wrong, b"cluster");
        assert!(matches!(result, Err(ChainError::DecryptFailed)));
    }

    #[test]
    fn test_empty_chain() {
        let prior = BTreeMap::new();
        let new_secret = make_secret(0xCC);

        let chain = encrypt_chain(&prior, &new_secret, b"cluster", 1).unwrap();
        assert_eq!(chain.prior_count, 0);

        let recovered = decrypt_chain(&chain, &new_secret, b"cluster").unwrap();
        assert!(recovered.is_empty());
    }

    #[test]
    fn test_multi_epoch_chain() {
        // Simulate 5 epochs of rotation
        let mut all_secrets = BTreeMap::new();
        let cluster_id = b"multi-epoch";

        for epoch in 1u64..=5 {
            let secret = make_secret(epoch as u8);
            all_secrets.insert(epoch, secret);
        }

        // Build chain at epoch 5 containing secrets 1-4
        let prior: BTreeMap<u64, [u8; 32]> =
            all_secrets.iter().filter(|(k, _)| **k < 5).map(|(&k, &v)| (k, v)).collect();

        let chain = encrypt_chain(&prior, &all_secrets[&5], cluster_id, 5).unwrap();
        assert_eq!(chain.prior_count, 4);

        let recovered = decrypt_chain(&chain, &all_secrets[&5], cluster_id).unwrap();
        assert_eq!(recovered.len(), 4);
        for epoch in 1..5 {
            assert_eq!(&recovered[&epoch], &all_secrets[&epoch]);
        }
    }

    #[test]
    fn test_wrong_cluster_id_fails() {
        let mut prior = BTreeMap::new();
        prior.insert(1, make_secret(0xAA));

        let secret = make_secret(0xCC);
        let chain = encrypt_chain(&prior, &secret, b"cluster-a", 2).unwrap();

        let result = decrypt_chain(&chain, &secret, b"cluster-b");
        assert!(matches!(result, Err(ChainError::DecryptFailed)));
    }
}
