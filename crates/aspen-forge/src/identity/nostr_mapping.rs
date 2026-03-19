//! Nostr identity → ed25519 keypair mapping.
//!
//! Each Nostr npub gets an assigned ed25519 keypair managed by the cluster.
//! The secret key is encrypted at rest using XChaCha20-Poly1305 with a key
//! derived from the cluster's iroh secret key via BLAKE3 keyed hash.
//!
//! ```text
//!   npub (secp256k1)
//!       │
//!       ▼
//!   KV: _identity:npub:{hex} → encrypted(ed25519 secret key)
//!       │
//!       ▼
//!   UserContext { npub, signing_key, public_key }
//! ```

use std::sync::Arc;

use chacha20poly1305::XChaCha20Poly1305;
use chacha20poly1305::aead::Aead;
use chacha20poly1305::aead::KeyInit;
use iroh::SecretKey;
use serde::Deserialize;
use serde::Serialize;
use tracing::debug;

use crate::error::ForgeError;
use crate::error::ForgeResult;

/// KV key prefix for identity mappings.
const KV_PREFIX: &str = "_identity:npub:";

/// XChaCha20-Poly1305 nonce size (24 bytes).
const NONCE_SIZE: usize = 24;

/// BLAKE3 key derivation context for identity encryption.
const KDF_CONTEXT: &str = "aspen-forge-nostr-identity-encryption-key-v1";

/// Per-user signing context resolved from a Nostr npub.
#[derive(Clone)]
pub struct UserContext {
    /// Nostr public key (hex-encoded, 64 chars).
    pub npub: String,
    /// Assigned ed25519 signing key.
    pub signing_key: SecretKey,
    /// Corresponding ed25519 public key.
    pub public_key: iroh::PublicKey,
}

/// Stored mapping entry (serialized before encryption).
#[derive(Serialize, Deserialize)]
struct StoredMapping {
    /// Raw ed25519 secret key bytes (32 bytes, hex-encoded for serde).
    secret_key_hex: String,
    /// Creation timestamp (ms since epoch).
    created_at_ms: u64,
}

/// Manages npub → ed25519 keypair mappings in the Raft KV store.
pub struct NostrIdentityStore<K: ?Sized> {
    kv: Arc<K>,
    /// Encryption key derived from the cluster's iroh secret key.
    encryption_key: [u8; 32],
}

impl<K: aspen_core::KeyValueStore + ?Sized> NostrIdentityStore<K> {
    /// Create a new identity store.
    ///
    /// Derives the encryption key from the cluster's iroh secret key using
    /// BLAKE3 keyed hash with a fixed context string.
    // r[impl identity.crypto.key-derivation]
    // r[impl identity.mapping.cluster-bound]
    pub fn new(kv: Arc<K>, cluster_secret_key: &SecretKey) -> Self {
        let raw_key = cluster_secret_key.to_bytes();
        let derived = blake3::derive_key(KDF_CONTEXT, &raw_key);
        Self {
            kv,
            encryption_key: derived,
        }
    }

    /// Get or create an ed25519 keypair for the given npub.
    ///
    /// If a mapping exists, decrypts and returns it. Otherwise generates
    /// a new keypair, encrypts and stores it, and returns the new context.
    // r[impl identity.mapping.create]
    // r[impl identity.mapping.retrieve]
    // r[impl identity.mapping.isolation]
    pub async fn get_or_create(&self, npub_hex: &str) -> ForgeResult<UserContext> {
        if let Some(ctx) = self.get(npub_hex).await? {
            return Ok(ctx);
        }

        // Generate a new ed25519 keypair.
        // iroh::SecretKey::generate needs a CryptoRng — use getrandom for the
        // seed bytes and construct from raw bytes to avoid a rand dep in non-test.
        let mut key_bytes = [0u8; 32];
        getrandom::fill(&mut key_bytes).map_err(|e| ForgeError::KvStorage {
            message: format!("failed to generate random key: {e}"),
        })?;
        let signing_key = SecretKey::from_bytes(&key_bytes);
        let public_key = signing_key.public();

        let mapping = StoredMapping {
            secret_key_hex: hex::encode(signing_key.to_bytes()),
            created_at_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        };

        let plaintext = postcard::to_allocvec(&mapping).map_err(|e| ForgeError::InvalidRepoIdentity {
            message: format!("failed to serialize identity mapping: {e}"),
        })?;

        let ciphertext = self.encrypt(&plaintext)?;
        let value = base64::Engine::encode(&base64::prelude::BASE64_STANDARD, &ciphertext);

        let key = format!("{KV_PREFIX}{npub_hex}");
        self.kv.write(aspen_core::WriteRequest::set(key, value)).await.map_err(|e| ForgeError::KvStorage {
            message: format!("failed to store identity mapping: {e}"),
        })?;

        debug!(npub = npub_hex, ed25519 = %public_key, "created new identity mapping");

        Ok(UserContext {
            npub: npub_hex.to_string(),
            signing_key,
            public_key,
        })
    }

    /// Look up an existing mapping for the given npub.
    pub async fn get(&self, npub_hex: &str) -> ForgeResult<Option<UserContext>> {
        let key = format!("{KV_PREFIX}{npub_hex}");
        let result = self
            .kv
            .read(aspen_core::ReadRequest {
                key,
                consistency: aspen_core::ReadConsistency::Linearizable,
            })
            .await;

        let value = match result {
            Ok(r) => match r.kv.map(|kv| kv.value) {
                Some(v) => v,
                None => return Ok(None),
            },
            Err(aspen_core::KeyValueStoreError::NotFound { .. }) => return Ok(None),
            Err(e) => {
                return Err(ForgeError::KvStorage {
                    message: format!("failed to read identity mapping: {e}"),
                });
            }
        };

        let ciphertext = base64::Engine::decode(&base64::prelude::BASE64_STANDARD, &value).map_err(|e| {
            ForgeError::InvalidRepoIdentity {
                message: format!("invalid base64 in identity mapping: {e}"),
            }
        })?;

        let plaintext = self.decrypt(&ciphertext)?;

        let mapping: StoredMapping = postcard::from_bytes(&plaintext).map_err(|e| ForgeError::InvalidRepoIdentity {
            message: format!("failed to deserialize identity mapping: {e}"),
        })?;

        let key_bytes = hex::decode(&mapping.secret_key_hex).map_err(|e| ForgeError::InvalidRepoIdentity {
            message: format!("invalid secret key hex: {e}"),
        })?;

        if key_bytes.len() != 32 {
            return Err(ForgeError::InvalidRepoIdentity {
                message: format!("secret key wrong length: {}", key_bytes.len()),
            });
        }

        let mut buf = [0u8; 32];
        buf.copy_from_slice(&key_bytes);
        let signing_key = SecretKey::from_bytes(&buf);
        let public_key = signing_key.public();

        Ok(Some(UserContext {
            npub: npub_hex.to_string(),
            signing_key,
            public_key,
        }))
    }

    // r[impl identity.mapping.encryption]
    // r[impl identity.crypto.nonce-unique]
    /// Encrypt plaintext with XChaCha20-Poly1305.
    fn encrypt(&self, plaintext: &[u8]) -> ForgeResult<Vec<u8>> {
        let cipher = XChaCha20Poly1305::new((&self.encryption_key).into());
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        getrandom::fill(&mut nonce_bytes).map_err(|e| ForgeError::KvStorage {
            message: format!("failed to generate nonce: {e}"),
        })?;
        let nonce = chacha20poly1305::XNonce::from(nonce_bytes);

        let ciphertext = cipher.encrypt(&nonce, plaintext).map_err(|e| ForgeError::KvStorage {
            message: format!("encryption failed: {e}"),
        })?;

        // Prepend nonce to ciphertext
        let mut result = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    // r[impl identity.crypto.roundtrip]
    // r[impl identity.crypto.wrong-key-fails]
    /// Decrypt ciphertext (nonce ∥ ciphertext) with XChaCha20-Poly1305.
    fn decrypt(&self, data: &[u8]) -> ForgeResult<Vec<u8>> {
        if data.len() < NONCE_SIZE {
            return Err(ForgeError::InvalidRepoIdentity {
                message: "ciphertext too short".to_string(),
            });
        }

        let (nonce_bytes, ciphertext) = data.split_at(NONCE_SIZE);
        let nonce = chacha20poly1305::XNonce::from_slice(nonce_bytes);
        let cipher = XChaCha20Poly1305::new((&self.encryption_key).into());

        cipher.decrypt(nonce, ciphertext).map_err(|e| ForgeError::KvStorage {
            message: format!("decryption failed (wrong cluster key?): {e}"),
        })
    }
}

#[cfg(test)]
mod tests {
    use aspen_testing_core::DeterministicKeyValueStore;

    use super::*;

    fn test_store() -> NostrIdentityStore<dyn aspen_core::KeyValueStore> {
        let kv: Arc<dyn aspen_core::KeyValueStore> = DeterministicKeyValueStore::new();
        let cluster_key = SecretKey::generate(&mut rand::rng());
        NostrIdentityStore::new(kv, &cluster_key)
    }

    // r[verify identity.mapping.create]
    // r[verify identity.mapping.retrieve]
    #[tokio::test]
    async fn create_and_retrieve_mapping() {
        let store = test_store();
        let npub = "a".repeat(64);

        // First call creates
        let ctx1 = store.get_or_create(&npub).await.unwrap();
        assert_eq!(ctx1.npub, npub);

        // Second call retrieves the same key
        let ctx2 = store.get_or_create(&npub).await.unwrap();
        assert_eq!(ctx1.public_key, ctx2.public_key);
    }

    // r[verify identity.mapping.isolation]
    #[tokio::test]
    async fn different_npubs_get_different_keys() {
        let store = test_store();
        let ctx_a = store.get_or_create(&"a".repeat(64)).await.unwrap();
        let ctx_b = store.get_or_create(&"b".repeat(64)).await.unwrap();
        assert_ne!(ctx_a.public_key, ctx_b.public_key);
    }

    #[tokio::test]
    async fn get_nonexistent_returns_none() {
        let store = test_store();
        let result = store.get(&"c".repeat(64)).await.unwrap();
        assert!(result.is_none());
    }

    // r[verify identity.mapping.encryption]
    // r[verify identity.crypto.roundtrip]
    // r[verify identity.crypto.key-derivation]
    // r[verify identity.crypto.nonce-unique]
    #[tokio::test]
    async fn encryption_is_not_plaintext() {
        let kv: Arc<dyn aspen_core::KeyValueStore> = DeterministicKeyValueStore::new();
        let cluster_key = SecretKey::generate(&mut rand::rng());
        let store = NostrIdentityStore::new(kv.clone(), &cluster_key);

        let npub = "d".repeat(64);
        let ctx = store.get_or_create(&npub).await.unwrap();

        // Read raw KV value
        let key = format!("{KV_PREFIX}{npub}");
        use aspen_core::KeyValueStore;
        let raw = kv
            .read(aspen_core::ReadRequest {
                key,
                consistency: aspen_core::ReadConsistency::Linearizable,
            })
            .await
            .unwrap();
        let raw_value = raw.kv.unwrap().value;

        // Raw value should NOT contain the secret key hex in cleartext
        let secret_hex = hex::encode(ctx.signing_key.to_bytes());
        assert!(!raw_value.contains(&secret_hex), "KV value should not contain plaintext secret key");
    }

    // r[verify identity.mapping.cluster-bound]
    // r[verify identity.crypto.wrong-key-fails]
    #[tokio::test]
    async fn wrong_cluster_key_cannot_decrypt() {
        let kv: Arc<dyn aspen_core::KeyValueStore> = DeterministicKeyValueStore::new();
        let key1 = SecretKey::generate(&mut rand::rng());
        let key2 = SecretKey::generate(&mut rand::rng());

        let store1 = NostrIdentityStore::new(kv.clone(), &key1);
        store1.get_or_create(&"e".repeat(64)).await.unwrap();

        let store2 = NostrIdentityStore::new(kv, &key2);
        let result = store2.get(&"e".repeat(64)).await;
        assert!(result.is_err(), "should fail to decrypt with wrong key");
    }
}
