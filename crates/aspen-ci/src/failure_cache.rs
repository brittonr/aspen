//! Derivation failure cache for CI builds.
//!
//! Caches failed derivation output paths in the KV store to prevent
//! identical rebuilds. Entries expire after a configurable TTL.
//!
//! ## TODO: Retry Integration
//!
//! Task 4.8 requires clearing failure cache entries when jobs are retried.
//! This needs integration with the aspen-jobs retry system to call
//! `clear_failure()` when a job with retry_count > 0 is being retried.
//! The hook point would be in the job manager when processing retries.

use aspen_core::DeleteRequest;
use aspen_core::KeyValueStore;
use aspen_core::ReadRequest;
use aspen_core::WriteCommand;
use aspen_core::WriteRequest;
use serde::Deserialize;
use serde::Serialize;
use snafu::ResultExt;
use snafu::Snafu;
use tracing::debug;
use tracing::warn;

/// Default TTL for failure cache entries: 24 hours.
pub const DEFAULT_FAILURE_CACHE_TTL_MS: u64 = 24 * 60 * 60 * 1000;

/// KV prefix for cached build failure paths.
pub const KV_PREFIX_CI_FAILED_PATHS: &str = "_ci:failed-paths:";

/// Errors for failure cache operations.
#[derive(Debug, Snafu)]
#[allow(missing_docs)]
pub enum FailureCacheError {
    /// Failed to serialize cache entry to JSON.
    #[snafu(display("failed to serialize failure cache entry: {source}"))]
    Serialization { source: serde_json::Error },

    /// Failed to deserialize cache entry from JSON.
    #[snafu(display("failed to deserialize failure cache entry: {source}"))]
    Deserialization { source: serde_json::Error },

    /// Failed to write to the KV store.
    #[snafu(display("failed to write to KV store: {source}"))]
    KvWrite { source: aspen_core::KeyValueStoreError },

    /// Failed to read from the KV store.
    #[snafu(display("failed to read from KV store: {source}"))]
    KvRead { source: aspen_core::KeyValueStoreError },

    /// Failed to delete from the KV store.
    #[snafu(display("failed to delete from KV store: {source}"))]
    KvDelete { source: aspen_core::KeyValueStoreError },
}

type Result<T> = std::result::Result<T, FailureCacheError>;

/// A cached failure entry stored in the KV store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureCacheEntry {
    /// The flake reference that failed (e.g., ".#packages.x86_64-linux.default").
    pub flake_ref: String,
    /// When this entry was created (Unix ms).
    pub created_at_ms: u64,
    /// TTL in milliseconds.
    pub ttl_ms: u64,
}

impl FailureCacheEntry {
    /// Check if this entry has expired.
    pub fn is_expired(&self, now_ms: u64) -> bool {
        now_ms > self.created_at_ms.saturating_add(self.ttl_ms)
    }
}

/// Record a failed flake reference in the failure cache.
pub async fn record_failure<S: KeyValueStore + ?Sized>(store: &S, flake_ref: &str, ttl_ms: u64) -> Result<()> {
    let now_ms = current_time_ms();

    let key = failure_cache_key(flake_ref);
    let entry = FailureCacheEntry {
        flake_ref: flake_ref.to_string(),
        created_at_ms: now_ms,
        ttl_ms,
    };

    let value = serde_json::to_string(&entry).context(SerializationSnafu)?;
    let request = WriteRequest::from_command(WriteCommand::Set { key, value });

    store.write(request).await.context(KvWriteSnafu)?;
    debug!(flake_ref = %flake_ref, ttl_ms, "Cached build failure");

    Ok(())
}

/// Check if a flake reference has a cached failure. Returns true on cache hit.
pub async fn check_failure<S: KeyValueStore + ?Sized>(store: &S, flake_ref: &str) -> Result<bool> {
    let now_ms = current_time_ms();

    let key = failure_cache_key(flake_ref);
    let request = ReadRequest::new(&key);

    let result = store.read(request).await.context(KvReadSnafu)?;
    if let Some(kv) = result.kv {
        let entry: FailureCacheEntry = serde_json::from_str(&kv.value).context(DeserializationSnafu)?;

        if entry.is_expired(now_ms) {
            // Expired — clean up and skip
            debug!(flake_ref = %flake_ref, "Removing expired failure cache entry");
            let delete_request = DeleteRequest::new(&key);
            if let Err(e) = store.delete(delete_request).await {
                warn!(error = %e, key = %key, "Failed to clean up expired failure cache entry");
            }
            return Ok(false);
        }

        debug!(flake_ref = %flake_ref, "Found cached build failure");
        return Ok(true);
    }

    Ok(false)
}

/// Clear failure cache entry for the given flake reference.
pub async fn clear_failure<S: KeyValueStore + ?Sized>(store: &S, flake_ref: &str) -> Result<()> {
    let key = failure_cache_key(flake_ref);
    let request = DeleteRequest::new(&key);

    if let Err(e) = store.delete(request).await.context(KvDeleteSnafu) {
        warn!(error = %e, flake_ref = %flake_ref, "Failed to clear failure cache entry");
    } else {
        debug!(flake_ref = %flake_ref, "Cleared failure cache entry");
    }

    Ok(())
}

/// Generate cache key from a flake reference using BLAKE3 hash.
fn failure_cache_key(flake_ref: &str) -> String {
    let hash = blake3::hash(flake_ref.as_bytes());
    format!("{}{}", KV_PREFIX_CI_FAILED_PATHS, hash.to_hex())
}

/// Get current unix timestamp in milliseconds.
fn current_time_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_failure_cache_key_deterministic() {
        let key1 = failure_cache_key(".#packages.x86_64-linux.default");
        let key2 = failure_cache_key(".#packages.x86_64-linux.default");
        assert_eq!(key1, key2);
        assert!(key1.starts_with(KV_PREFIX_CI_FAILED_PATHS));
    }

    #[test]
    fn test_failure_cache_key_different_refs() {
        let key1 = failure_cache_key(".#packages.x86_64-linux.default");
        let key2 = failure_cache_key(".#packages.x86_64-linux.dev");
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_failure_entry_not_expired() {
        let entry = FailureCacheEntry {
            flake_ref: ".#packages.x86_64-linux.default".to_string(),
            created_at_ms: 1000,
            ttl_ms: 5000,
        };
        assert!(!entry.is_expired(3000)); // now = 3000, expires at 6000
    }

    #[test]
    fn test_failure_entry_expired() {
        let entry = FailureCacheEntry {
            flake_ref: ".#packages.x86_64-linux.default".to_string(),
            created_at_ms: 1000,
            ttl_ms: 5000,
        };
        assert!(entry.is_expired(7000)); // now = 7000, expires at 6000
    }

    #[test]
    fn test_failure_entry_at_boundary() {
        let entry = FailureCacheEntry {
            flake_ref: ".#packages.x86_64-linux.default".to_string(),
            created_at_ms: 1000,
            ttl_ms: 5000,
        };
        // At exactly created_at + ttl, not expired yet
        assert!(!entry.is_expired(6000));
        // One ms after, expired
        assert!(entry.is_expired(6001));
    }

    #[test]
    fn test_failure_entry_overflow_protection() {
        let entry = FailureCacheEntry {
            flake_ref: ".#packages.x86_64-linux.default".to_string(),
            created_at_ms: u64::MAX - 100,
            ttl_ms: 200,
        };
        // Saturating add prevents overflow
        assert!(!entry.is_expired(u64::MAX - 50));
    }

    #[test]
    fn test_current_time_ms_is_positive() {
        let time = current_time_ms();
        assert!(time > 0);
    }

    #[test]
    fn test_entry_serialization() {
        let entry = FailureCacheEntry {
            flake_ref: ".#packages.x86_64-linux.default".to_string(),
            created_at_ms: 1_000_000,
            ttl_ms: 86_400_000,
        };

        let serialized = serde_json::to_string(&entry).unwrap();
        let deserialized: FailureCacheEntry = serde_json::from_str(&serialized).unwrap();

        assert_eq!(entry.flake_ref, deserialized.flake_ref);
        assert_eq!(entry.created_at_ms, deserialized.created_at_ms);
        assert_eq!(entry.ttl_ms, deserialized.ttl_ms);
    }
}
