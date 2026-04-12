//! Lazy key reconstruction and epoch rotation manager.
//!
//! `KeyManager` coordinates the lifecycle of at-rest encryption keys:
//!
//! 1. **Lazy reconstruction**: On first secrets access, collects shares from peers, reconstructs
//!    the cluster secret, derives the at-rest key, and creates a `SecretsEncryption` context.
//!
//! 2. **Epoch rotation**: When a new epoch is committed (membership change), derives a new key from
//!    the new secret, swaps it into the encryption context, and signals re-encryption.
//!
//! The `KeyManager` is the bridge between the trust protocol layer (share
//! collection) and the secrets engine (encryption/decryption).

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::encryption::SecretsEncryption;
use crate::encryption::SecretsUnavailableError;
use crate::kdf;
use crate::shamir;

/// Callback trait for share collection from peers.
///
/// Implementors provide the mechanism for requesting shares from other
/// cluster members (typically via Iroh QUIC + trust ALPN).
#[async_trait]
pub trait ShareCollector: Send + Sync {
    /// Collect shares from peers for the given epoch.
    ///
    /// Must return at least `threshold` valid shares for reconstruction.
    /// Returns the collected shares and the threshold used.
    async fn collect_shares(&self, epoch: u64) -> Result<Vec<shamir::Share>, ShareCollectionError>;
}

/// Errors during share collection.
#[derive(Debug, snafu::Snafu)]
pub enum ShareCollectionError {
    /// Not enough peers responded with valid shares.
    #[snafu(display("below quorum: got {collected} shares, need {threshold}"))]
    BelowQuorum { collected: u32, threshold: u32 },

    /// Network or protocol error during collection.
    #[snafu(display("share collection failed: {reason}"))]
    CollectionFailed { reason: String },
}

/// Configuration for the key manager.
pub struct KeyManagerConfig {
    /// Cluster identifier (used in HKDF derivation).
    pub cluster_id: Vec<u8>,
    /// Local node ID (used for nonce prefix).
    pub node_id: u32,
    /// Initial nonce counter (loaded from redb on startup).
    pub initial_nonce_counter: u64,
    /// Current trust epoch.
    pub epoch: u64,
}

/// Manages the lifecycle of at-rest encryption keys.
///
/// Thread-safe via `tokio::sync::RwLock`. The encryption context is
/// lazily initialized on first access and rotated on epoch changes.
pub struct KeyManager {
    /// Current encryption context (None until first reconstruction).
    encryption: RwLock<Option<Arc<SecretsEncryption>>>,
    /// Configuration.
    config: KeyManagerConfig,
    /// Whether reconstruction has been attempted.
    reconstruction_attempted: RwLock<bool>,
}

impl KeyManager {
    /// Create a new key manager.
    ///
    /// The encryption context is NOT initialized at creation time.
    /// Call `ensure_initialized` to trigger lazy reconstruction.
    pub fn new(config: KeyManagerConfig) -> Self {
        Self {
            encryption: RwLock::new(None),
            config,
            reconstruction_attempted: RwLock::new(false),
        }
    }

    /// Create a key manager with a pre-initialized encryption context.
    ///
    /// Used when the cluster secret is already known (e.g., leader that
    /// just performed initialization or reconfiguration).
    pub fn with_secret(config: KeyManagerConfig, cluster_secret: &[u8; 32]) -> Self {
        let enc = SecretsEncryption::new(
            cluster_secret,
            &config.cluster_id,
            config.epoch,
            config.node_id,
            config.initial_nonce_counter,
        );
        Self {
            encryption: RwLock::new(Some(Arc::new(enc))),
            config,
            reconstruction_attempted: RwLock::new(true),
        }
    }

    /// Get the encryption context, triggering lazy reconstruction if needed.
    ///
    /// On first call, collects shares from peers via the `collector`,
    /// reconstructs the cluster secret, and derives the at-rest key.
    ///
    /// Returns `SecretsUnavailable` if reconstruction fails (cluster below quorum).
    pub async fn ensure_initialized(
        &self,
        collector: &dyn ShareCollector,
    ) -> Result<Arc<SecretsEncryption>, SecretsUnavailableError> {
        // Fast path: already initialized
        {
            let enc = self.encryption.read().await;
            if let Some(ref e) = *enc {
                return Ok(Arc::clone(e));
            }
        }

        // Slow path: attempt reconstruction (only once)
        let mut attempted = self.reconstruction_attempted.write().await;
        if *attempted {
            // Already attempted and failed — don't retry automatically
            return Err(SecretsUnavailableError::BelowQuorum);
        }
        *attempted = true;
        drop(attempted);

        // Collect shares from peers
        let shares = collector
            .collect_shares(self.config.epoch)
            .await
            .map_err(|_| SecretsUnavailableError::BelowQuorum)?;

        // Reconstruct secret
        let secret_bytes = shamir::reconstruct_secret(&shares).map_err(|_| SecretsUnavailableError::BelowQuorum)?;

        // Create encryption context
        let enc = SecretsEncryption::new(
            &secret_bytes,
            &self.config.cluster_id,
            self.config.epoch,
            self.config.node_id,
            self.config.initial_nonce_counter,
        );
        let enc = Arc::new(enc);

        // Store it
        let mut guard = self.encryption.write().await;
        *guard = Some(Arc::clone(&enc));

        Ok(enc)
    }

    /// Get the encryption context if already initialized, without triggering reconstruction.
    pub async fn get_if_initialized(&self) -> Option<Arc<SecretsEncryption>> {
        let guard: tokio::sync::RwLockReadGuard<'_, Option<Arc<SecretsEncryption>>> = self.encryption.read().await;
        guard.clone()
    }

    /// Rotate to a new epoch with a new cluster secret.
    ///
    /// Derives a fresh key, updates the encryption context, and retains
    /// the old epoch's key for mixed-epoch reads during re-encryption.
    ///
    /// Returns the updated encryption context.
    pub async fn rotate_epoch(&self, new_secret: &[u8; 32], new_epoch: u64) -> Arc<SecretsEncryption> {
        self.rotate_epoch_with_prior_secrets(new_secret, new_epoch, &BTreeMap::new()).await
    }

    /// Rotate to a new epoch and seed keys for prior epochs recovered from the chain.
    pub async fn rotate_epoch_with_prior_secrets(
        &self,
        new_secret: &[u8; 32],
        new_epoch: u64,
        prior_secrets: &BTreeMap<u64, [u8; 32]>,
    ) -> Arc<SecretsEncryption> {
        let mut guard: tokio::sync::RwLockWriteGuard<'_, Option<Arc<SecretsEncryption>>> =
            self.encryption.write().await;

        let initial_counter =
            guard.as_ref().map(|existing| existing.nonce_counter()).unwrap_or(self.config.initial_nonce_counter);
        let mut new_enc = SecretsEncryption::new(
            new_secret,
            &self.config.cluster_id,
            new_epoch,
            self.config.node_id,
            initial_counter,
        );

        if let Some(existing) = guard.as_ref() {
            new_enc.copy_epoch_keys_from(existing.as_ref());
        }

        for (epoch, secret) in prior_secrets {
            if *epoch == new_epoch {
                continue;
            }
            let key = kdf::derive_key(secret, kdf::CONTEXT_SECRETS_AT_REST, &self.config.cluster_id, *epoch);
            new_enc.add_epoch_key(*epoch, key);
        }

        let result = Arc::new(new_enc);
        *guard = Some(Arc::clone(&result));

        let mut attempted = self.reconstruction_attempted.write().await;
        *attempted = true;

        result
    }

    /// Allow retry of reconstruction (e.g., after more nodes come online).
    pub async fn reset_reconstruction_attempt(&self) {
        let mut attempted = self.reconstruction_attempted.write().await;
        *attempted = false;
    }

    /// Current epoch.
    pub fn epoch(&self) -> u64 {
        self.config.epoch
    }

    /// Node ID.
    pub fn node_id(&self) -> u32 {
        self.config.node_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockCollector {
        shares: Vec<shamir::Share>,
    }

    #[async_trait]
    impl ShareCollector for MockCollector {
        async fn collect_shares(&self, _epoch: u64) -> Result<Vec<shamir::Share>, ShareCollectionError> {
            if self.shares.is_empty() {
                return Err(ShareCollectionError::BelowQuorum {
                    collected: 0,
                    threshold: 2,
                });
            }
            Ok(self.shares.clone())
        }
    }

    fn make_shares() -> ([u8; 32], Vec<shamir::Share>) {
        let secret = [0xABu8; 32];
        let mut rng = rand::rng();
        let shares = shamir::split_secret(&secret, 2, 3, &mut rng).unwrap();
        (secret, shares)
    }

    #[tokio::test]
    async fn test_lazy_reconstruction_success() {
        let (_secret, shares) = make_shares();
        let collector = MockCollector {
            shares: shares[..2].to_vec(),
        };
        let config = KeyManagerConfig {
            cluster_id: b"test-cluster".to_vec(),
            node_id: 1,
            initial_nonce_counter: 0,
            epoch: 1,
        };
        let km = KeyManager::new(config);

        // First call triggers reconstruction
        let enc = km.ensure_initialized(&collector).await.unwrap();
        assert_eq!(enc.epoch(), 1);

        // Second call returns cached
        let enc2 = km.ensure_initialized(&collector).await.unwrap();
        assert!(Arc::ptr_eq(&enc, &enc2));
    }

    #[tokio::test]
    async fn test_lazy_reconstruction_below_quorum() {
        let collector = MockCollector { shares: vec![] };
        let config = KeyManagerConfig {
            cluster_id: b"test-cluster".to_vec(),
            node_id: 1,
            initial_nonce_counter: 0,
            epoch: 1,
        };
        let km = KeyManager::new(config);

        let result = km.ensure_initialized(&collector).await;
        assert!(matches!(result, Err(SecretsUnavailableError::BelowQuorum)));

        // Second attempt also fails (no automatic retry)
        let result = km.ensure_initialized(&collector).await;
        assert!(matches!(result, Err(SecretsUnavailableError::BelowQuorum)));
    }

    #[tokio::test]
    async fn test_retry_after_reset() {
        let (_secret, shares) = make_shares();
        let collector = MockCollector {
            shares: shares[..2].to_vec(),
        };
        let config = KeyManagerConfig {
            cluster_id: b"test-cluster".to_vec(),
            node_id: 1,
            initial_nonce_counter: 0,
            epoch: 1,
        };
        let km = KeyManager::new(config);

        // First attempt with empty collector fails
        let empty_collector = MockCollector { shares: vec![] };
        let _ = km.ensure_initialized(&empty_collector).await;

        // Reset and retry with good collector
        km.reset_reconstruction_attempt().await;
        let enc = km.ensure_initialized(&collector).await.unwrap();
        assert_eq!(enc.epoch(), 1);
    }

    #[tokio::test]
    async fn test_with_secret_skips_reconstruction() {
        let (secret, _shares) = make_shares();
        let config = KeyManagerConfig {
            cluster_id: b"test-cluster".to_vec(),
            node_id: 1,
            initial_nonce_counter: 0,
            epoch: 1,
        };
        let km = KeyManager::with_secret(config, &secret);

        // Already initialized, no collector needed
        let enc = km.get_if_initialized().await.unwrap();
        assert_eq!(enc.epoch(), 1);
    }

    #[tokio::test]
    async fn test_rotate_epoch() {
        let (secret, _shares) = make_shares();
        let config = KeyManagerConfig {
            cluster_id: b"test-cluster".to_vec(),
            node_id: 1,
            initial_nonce_counter: 0,
            epoch: 1,
        };
        let km = KeyManager::with_secret(config, &secret);

        let old_enc = km.get_if_initialized().await.unwrap();
        let (old_ciphertext, _) = old_enc.wrap_write(b"legacy-secret").unwrap();

        let new_secret = [0xCDu8; 32];
        let new_enc = km.rotate_epoch(&new_secret, 2).await;
        assert_eq!(new_enc.epoch(), 2);
        assert_eq!(new_enc.epoch_key_count(), 2);
        assert_eq!(new_enc.unwrap_read(&old_ciphertext).unwrap(), b"legacy-secret");
    }

    #[tokio::test]
    async fn test_rotate_from_uninitialized() {
        let config = KeyManagerConfig {
            cluster_id: b"test-cluster".to_vec(),
            node_id: 1,
            initial_nonce_counter: 0,
            epoch: 1,
        };
        let km = KeyManager::new(config);

        // Rotate before initialization (leader creating first epoch)
        let secret = [0xABu8; 32];
        let enc = km.rotate_epoch(&secret, 1).await;
        assert_eq!(enc.epoch(), 1);

        // Now initialized
        let enc2 = km.get_if_initialized().await.unwrap();
        assert!(Arc::ptr_eq(&enc, &enc2));
    }
}
