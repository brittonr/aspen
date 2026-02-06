//! Distributed lock with fencing tokens.
//!
//! Provides mutual exclusion across distributed nodes with:
//! - Monotonically increasing fencing tokens for split-brain prevention
//! - TTL-based automatic expiration for crash recovery
//! - Exponential backoff with jitter to prevent thundering herd
//!
//! # Formal Verification
//!
//! This module has formal Verus specifications in `crates/aspen-coordination/verus/`
//! that prove three key invariants:
//!
//! 1. **Fencing Token Monotonicity**: Tokens strictly increase on each acquisition
//! 2. **Mutual Exclusion**: At most one holder at any time (via CAS semantics)
//! 3. **TTL Expiration Safety**: Expired locks become reacquirable
//!
//! Ghost code annotations link the implementation to these formal proofs.

use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use aspen_core::KeyValueStore;
use aspen_core::KeyValueStoreError;
use aspen_core::ReadRequest;
use aspen_core::WriteCommand;
use aspen_core::WriteRequest;
use rand::Rng;
use tracing::debug;
use tracing::warn;

use crate::error::CoordinationError;
use crate::error::LockHeldSnafu;
use crate::error::LockLostSnafu;
use crate::error::TimeoutSnafu;
use crate::spec::verus_shim::*;
use crate::types::FencingToken;
use crate::types::LockEntry;

/// Configuration for distributed lock.
#[derive(Debug, Clone)]
pub struct LockConfig {
    /// Time-to-live for the lock in milliseconds.
    pub ttl_ms: u64,
    /// Maximum time to wait for lock acquisition.
    pub acquire_timeout_ms: u64,
    /// Initial backoff for retry in milliseconds.
    pub initial_backoff_ms: u64,
    /// Maximum backoff between retries in milliseconds.
    pub max_backoff_ms: u64,
}

impl Default for LockConfig {
    fn default() -> Self {
        Self {
            ttl_ms: 30_000,             // 30 seconds
            acquire_timeout_ms: 10_000, // 10 seconds
            initial_backoff_ms: 10,     // 10ms initial
            max_backoff_ms: 1_000,      // 1 second max
        }
    }
}

/// A distributed mutex lock.
///
/// Provides mutual exclusion with fencing tokens for correctness
/// and TTL-based expiration for liveness.
pub struct DistributedLock<S: KeyValueStore + ?Sized> {
    store: Arc<S>,
    key: String,
    holder_id: String,
    config: LockConfig,
}

impl<S: KeyValueStore + ?Sized + 'static> DistributedLock<S> {
    /// Create a new distributed lock handle.
    ///
    /// # Arguments
    /// * `store` - The underlying key-value store
    /// * `key` - The lock key (should be unique per resource)
    /// * `holder_id` - Unique identifier for this lock holder
    /// * `config` - Lock configuration
    pub fn new(store: Arc<S>, key: impl Into<String>, holder_id: impl Into<String>, config: LockConfig) -> Self {
        Self {
            store,
            key: key.into(),
            holder_id: holder_id.into(),
            config,
        }
    }

    /// Attempt to acquire the lock with retries.
    ///
    /// Returns the fencing token on success.
    /// Retries with exponential backoff until timeout.
    pub async fn acquire(&self) -> Result<LockGuard<S>, CoordinationError> {
        let deadline = Instant::now() + Duration::from_millis(self.config.acquire_timeout_ms);
        let mut backoff_ms = self.config.initial_backoff_ms;

        loop {
            match self.try_acquire().await {
                Ok(guard) => return Ok(guard),
                Err(CoordinationError::LockHeld { holder, deadline_ms }) => {
                    if Instant::now() >= deadline {
                        return TimeoutSnafu {
                            operation: format!("lock acquisition for '{}'", self.key),
                        }
                        .fail();
                    }

                    // Calculate wait time with jitter
                    // Create rng here to avoid holding non-Send type across await
                    let jitter = rand::rng().random_range(0..backoff_ms / 2 + 1);
                    let sleep_ms = backoff_ms + jitter;

                    debug!(
                        key = %self.key,
                        holder = %holder,
                        deadline_ms,
                        backoff_ms = sleep_ms,
                        "lock held, backing off"
                    );

                    tokio::time::sleep(Duration::from_millis(sleep_ms)).await;

                    // Exponential backoff
                    backoff_ms = (backoff_ms * 2).min(self.config.max_backoff_ms);
                }
                Err(CoordinationError::CasConflict) => {
                    // Immediate retry on CAS conflict
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Try to acquire the lock without blocking.
    ///
    /// Returns immediately with success or failure.
    ///
    /// # Verified Properties (see `verus/acquire_spec.rs`)
    ///
    /// - **INVARIANT 1**: New fencing token > max_fencing_token_issued
    /// - **INVARIANT 2**: New deadline = acquired_at + ttl
    /// - **INVARIANT 3**: Only succeeds if lock is available (None or expired)
    pub async fn try_acquire(&self) -> Result<LockGuard<S>, CoordinationError> {
        // Read current lock state
        let current = self.read_lock_entry().await?;

        // Ghost: Capture pre-state for verification
        ghost! {
            let pre_token = current.as_ref().map(|e| e.fencing_token).unwrap_or(0);
            let _pre_state = LockStateSpec::from_entry(current.as_ref(), crate::types::now_unix_ms());
        }

        // Determine expected value and new token
        let (expected, new_token) = match current {
            Some(ref entry) if !entry.is_expired() => {
                // Lock held by someone else (not expired)
                return LockHeldSnafu {
                    holder: entry.holder_id.clone(),
                    deadline_ms: entry.deadline_ms,
                }
                .fail();
            }
            Some(ref entry) => {
                // Lock expired, we can take it
                debug!(
                    key = %self.key,
                    previous_holder = %entry.holder_id,
                    "taking expired lock"
                );
                (Some(serde_json::to_string(entry)?), entry.fencing_token + 1)
            }
            None => {
                // Lock doesn't exist yet
                (None, 1)
            }
        };

        // Create new lock entry
        let new_entry = LockEntry::new(self.holder_id.clone(), new_token, self.config.ttl_ms);
        let new_json = serde_json::to_string(&new_entry)?;
        // Pre-compute released JSON for Drop
        let released_json = serde_json::to_string(&new_entry.released())?;

        // Atomic CAS
        match self
            .store
            .write(WriteRequest {
                command: WriteCommand::CompareAndSwap {
                    key: self.key.clone(),
                    expected,
                    new_value: new_json.clone(),
                },
            })
            .await
        {
            Ok(_) => {
                debug!(
                    key = %self.key,
                    holder = %self.holder_id,
                    fencing_token = new_token,
                    ttl_ms = self.config.ttl_ms,
                    "lock acquired"
                );

                // Proof: Verify fencing token monotonicity and TTL validity
                proof! {
                    // Link to acquire_spec.rs proofs:
                    // - acquire_increases_fencing_token: new_token > pre_token
                    // - acquire_establishes_ttl_validity: deadline = acquired_at + ttl
                    // - acquire_preserves_lock_invariant: all invariants maintained
                    assert(new_token > pre_token);
                    assert(new_entry.deadline_ms == new_entry.acquired_at_ms + new_entry.ttl_ms);
                }

                Ok(LockGuard {
                    store: self.store.clone(),
                    key: self.key.clone(),
                    holder_id: self.holder_id.clone(),
                    fencing_token: FencingToken(new_token),
                    entry_json: new_json,
                    released_json,
                    deadline_ms: new_entry.deadline_ms,
                })
            }
            Err(KeyValueStoreError::CompareAndSwapFailed { actual, .. }) => {
                // Someone else got it or state changed
                if let Some(json) = actual {
                    match serde_json::from_str::<LockEntry>(&json) {
                        Ok(entry) => LockHeldSnafu {
                            holder: entry.holder_id,
                            deadline_ms: entry.deadline_ms,
                        }
                        .fail(),
                        Err(_) => Err(CoordinationError::CasConflict),
                    }
                } else {
                    // Key was deleted between read and CAS
                    Err(CoordinationError::CasConflict)
                }
            }
            Err(e) => Err(CoordinationError::Storage { source: e }),
        }
    }

    /// Extend the lock's TTL.
    ///
    /// Must be called before the lock expires to prevent release.
    /// Returns error if lock was lost (another holder acquired it).
    ///
    /// # Verified Properties (see `verus/renew_spec.rs`)
    ///
    /// - **CRITICAL**: Fencing token is preserved (unchanged)
    /// - Deadline is extended: new_deadline = new_acquired_at + ttl
    /// - Lock remains held by the same holder
    pub async fn renew(&self, guard: &LockGuard<S>) -> Result<(), CoordinationError> {
        // Read current state
        let current = self.read_lock_entry().await?;

        // Ghost: Capture pre-state token for verification
        ghost! {
            let pre_token = guard.fencing_token.value();
            let _pre_state = LockStateSpec::from_entry(current.as_ref(), crate::types::now_unix_ms());
        }

        match current {
            Some(entry) if entry.fencing_token == guard.fencing_token.value() => {
                // We still hold it, extend TTL
                let renewed = LockEntry::new(self.holder_id.clone(), entry.fencing_token, self.config.ttl_ms);
                let new_json = serde_json::to_string(&renewed)?;

                match self
                    .store
                    .write(WriteRequest {
                        command: WriteCommand::CompareAndSwap {
                            key: self.key.clone(),
                            expected: Some(guard.entry_json.clone()),
                            new_value: new_json,
                        },
                    })
                    .await
                {
                    Ok(_) => {
                        // Proof: Verify fencing token is preserved
                        proof! {
                            // Link to renew_spec.rs proofs:
                            // - renew_preserves_fencing_token: token unchanged
                            // - renew_extends_deadline: new deadline computed correctly
                            // - renew_maintains_holder: still held by same holder
                            assert(renewed.fencing_token == pre_token);
                            assert(renewed.deadline_ms == renewed.acquired_at_ms + renewed.ttl_ms);
                        }

                        debug!(
                            key = %self.key,
                            fencing_token = guard.fencing_token.value(),
                            "lock renewed"
                        );
                        Ok(())
                    }
                    Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => LockLostSnafu {
                        expected_holder: self.holder_id.clone(),
                        current_holder: "unknown".to_string(),
                    }
                    .fail(),
                    Err(e) => Err(CoordinationError::Storage { source: e }),
                }
            }
            Some(entry) => LockLostSnafu {
                expected_holder: self.holder_id.clone(),
                current_holder: entry.holder_id,
            }
            .fail(),
            None => LockLostSnafu {
                expected_holder: self.holder_id.clone(),
                current_holder: "none (deleted)".to_string(),
            }
            .fail(),
        }
    }

    /// Read the current lock entry from storage.
    async fn read_lock_entry(&self) -> Result<Option<LockEntry>, CoordinationError> {
        match self.store.read(ReadRequest::new(self.key.clone())).await {
            Ok(result) => {
                let value = result.kv.map(|kv| kv.value).unwrap_or_default();
                let entry: LockEntry = serde_json::from_str(&value).map_err(|_| CoordinationError::CorruptedData {
                    key: self.key.clone(),
                    reason: "invalid JSON".to_string(),
                })?;
                Ok(Some(entry))
            }
            Err(KeyValueStoreError::NotFound { .. }) => Ok(None),
            Err(e) => Err(CoordinationError::Storage { source: e }),
        }
    }
}

/// RAII guard that releases the lock on drop.
///
/// The lock is released when this guard is dropped. The fencing token
/// should be passed to any external services that need to validate
/// the lock is still held.
pub struct LockGuard<S: KeyValueStore + ?Sized + 'static> {
    store: Arc<S>,
    key: String,
    holder_id: String,
    fencing_token: FencingToken,
    entry_json: String,
    /// Pre-computed released entry JSON for use in Drop.
    released_json: String,
    /// Lock expiration deadline in Unix milliseconds.
    deadline_ms: u64,
}

impl<S: KeyValueStore + ?Sized> LockGuard<S> {
    /// Get the fencing token.
    ///
    /// Include this token in all operations protected by the lock.
    /// External services should reject operations with stale tokens.
    pub fn fencing_token(&self) -> FencingToken {
        self.fencing_token
    }

    /// Get the lock key.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Get the holder ID.
    pub fn holder_id(&self) -> &str {
        &self.holder_id
    }

    /// Get the lock deadline in Unix milliseconds.
    ///
    /// The lock expires at this time if not renewed.
    pub fn deadline_ms(&self) -> u64 {
        self.deadline_ms
    }

    /// Explicitly release the lock.
    ///
    /// This is called automatically on drop, but can be called explicitly
    /// if you need to handle release errors.
    pub async fn release(self) -> Result<(), CoordinationError> {
        self.release_impl().await
    }

    /// # Verified Properties (see `verus/release_spec.rs`)
    ///
    /// - Max fencing token is preserved (not decreased)
    /// - Entry deadline_ms is set to 0 (released state)
    /// - Fencing token in entry is preserved for history
    async fn release_impl(&self) -> Result<(), CoordinationError> {
        // Ghost: Capture pre-state for verification
        ghost! {
            let pre_token = self.fencing_token.value();
        }

        match self
            .store
            .write(WriteRequest {
                command: WriteCommand::CompareAndSwap {
                    key: self.key.clone(),
                    expected: Some(self.entry_json.clone()),
                    new_value: self.released_json.clone(),
                },
            })
            .await
        {
            Ok(_) => {
                // Proof: Verify release properties
                proof! {
                    // Link to release_spec.rs proofs:
                    // - release_preserves_max_token: max token unchanged
                    // - release_clears_deadline: deadline_ms = 0
                    // - release_preserves_entry_token: entry token preserved
                    // - release_makes_available: lock now available
                    // The released_json has deadline_ms = 0 (see LockEntry::released())
                }

                debug!(
                    key = %self.key,
                    fencing_token = self.fencing_token.value(),
                    "lock released"
                );
                Ok(())
            }
            Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => {
                // Lock was already released or taken by someone else
                warn!(
                    key = %self.key,
                    fencing_token = self.fencing_token.value(),
                    "lock release failed: already released or taken"
                );
                Ok(()) // Not an error, just means someone else has it
            }
            Err(e) => Err(CoordinationError::Storage { source: e }),
        }
    }
}

impl<S: KeyValueStore + ?Sized + 'static> Drop for LockGuard<S> {
    fn drop(&mut self) {
        // Best-effort release - lock will expire anyway via TTL
        let store = self.store.clone();
        let key = self.key.clone();
        let expected = self.entry_json.clone();
        let released = self.released_json.clone();
        let token = self.fencing_token.value();

        tokio::spawn(async move {
            let result = store
                .write(WriteRequest {
                    command: WriteCommand::CompareAndSwap {
                        key: key.clone(),
                        expected: Some(expected),
                        new_value: released,
                    },
                })
                .await;

            match result {
                Ok(_) => debug!(key = %key, fencing_token = token, "lock released on drop"),
                Err(e) => {
                    debug!(key = %key, error = %e, "lock release on drop failed (will expire via TTL)")
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use aspen_core::inmemory::DeterministicKeyValueStore;

    use super::*;

    #[tokio::test]
    async fn test_lock_acquire_release() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let lock = DistributedLock::new(store, "test_lock", "holder_1", LockConfig::default());

        let guard = lock.try_acquire().await.unwrap();
        assert!(guard.fencing_token().value() > 0);
        // Guard dropped here, lock released
    }

    #[tokio::test]
    async fn test_lock_contention() {
        let store = Arc::new(DeterministicKeyValueStore::new());

        let lock1 = DistributedLock::new(store.clone(), "test_lock", "holder_1", LockConfig::default());
        let lock2 = DistributedLock::new(store, "test_lock", "holder_2", LockConfig::default());

        let _guard1 = lock1.try_acquire().await.unwrap();

        // Second acquire should fail
        let result = lock2.try_acquire().await;
        assert!(matches!(result, Err(CoordinationError::LockHeld { .. })));
    }

    #[tokio::test]
    async fn test_lock_fencing_token_increases() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let lock = DistributedLock::new(store, "test_lock", "holder_1", LockConfig::default());

        let guard1 = lock.try_acquire().await.unwrap();
        let token1 = guard1.fencing_token();
        guard1.release().await.unwrap();

        let guard2 = lock.try_acquire().await.unwrap();
        let token2 = guard2.fencing_token();

        assert!(token2.value() > token1.value());
    }

    #[tokio::test]
    async fn test_lock_expiry() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let config = LockConfig {
            ttl_ms: 50, // Very short TTL for testing
            ..Default::default()
        };

        let lock1 = DistributedLock::new(store.clone(), "test_lock", "holder_1", config.clone());
        let lock2 = DistributedLock::new(store, "test_lock", "holder_2", config);

        let _guard1 = lock1.try_acquire().await.unwrap();

        // Wait for expiry
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Should be able to acquire now
        let guard2 = lock2.try_acquire().await.unwrap();
        assert!(guard2.fencing_token().value() > 1);
    }
}
