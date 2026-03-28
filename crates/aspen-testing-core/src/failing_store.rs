//! Error injection wrapper for `KeyValueStore`.
//!
//! Wraps any `KeyValueStore` implementation with configurable failure injection.
//! Useful for testing error handling paths without needing madsim or real network faults.
//!
//! # Usage
//!
//! ```ignore
//! let store = DeterministicKeyValueStore::new();
//! let failing = FailingKeyValueStore::new(store);
//!
//! // Fail every 3rd write
//! failing.fail_writes_every(3);
//!
//! // Fail all reads
//! failing.fail_all_reads();
//!
//! // Fail reads for specific keys
//! failing.fail_reads_for_key("_sys:leader");
//!
//! // Clear all injected faults
//! failing.clear_faults();
//! ```

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use aspen_kv_types::{
    DeleteRequest, DeleteResult, KeyValueStoreError, ReadRequest, ReadResult, ScanRequest,
    ScanResult, WriteRequest, WriteResult,
};
use aspen_traits::KeyValueStore;
use tokio::sync::RwLock;

/// Configurable failure injection modes.
#[derive(Debug, Clone)]
enum FaultMode {
    /// No faults injected.
    None,
    /// Fail every Nth operation.
    EveryN(u64),
    /// Fail all operations.
    All,
}

/// Fault configuration for a specific operation type.
#[derive(Debug)]
struct FaultConfig {
    mode: FaultMode,
    counter: AtomicU64,
    /// Specific keys to fail on (if empty, applies to all keys).
    fail_keys: HashSet<String>,
    /// Error to return on failure.
    error_message: String,
}

impl FaultConfig {
    fn new() -> Self {
        Self {
            mode: FaultMode::None,
            counter: AtomicU64::new(0),
            fail_keys: HashSet::new(),
            error_message: "injected fault".to_string(),
        }
    }

    fn should_fail(&self, key: Option<&str>) -> bool {
        // Check key filter first
        if !self.fail_keys.is_empty() {
            if let Some(k) = key {
                if !self.fail_keys.contains(k) {
                    return false;
                }
            }
        }

        match &self.mode {
            FaultMode::None => false,
            FaultMode::All => true,
            FaultMode::EveryN(n) => {
                let count = self.counter.fetch_add(1, Ordering::Relaxed) + 1;
                count % n == 0
            }
        }
    }

    fn make_error(&self) -> KeyValueStoreError {
        KeyValueStoreError::Failed {
            reason: self.error_message.clone(),
        }
    }
}

/// A `KeyValueStore` wrapper that injects configurable failures.
///
/// All operations delegate to the inner store unless a fault is configured
/// for that operation type. Faults can be changed at any time via the
/// configuration methods.
pub struct FailingKeyValueStore<S: KeyValueStore + ?Sized> {
    inner: Arc<S>,
    read_faults: RwLock<FaultConfig>,
    write_faults: RwLock<FaultConfig>,
    delete_faults: RwLock<FaultConfig>,
    scan_faults: RwLock<FaultConfig>,
}

impl<S: KeyValueStore + ?Sized> FailingKeyValueStore<S> {
    /// Create a new failing store wrapping the given store.
    pub fn new(inner: Arc<S>) -> Arc<Self> {
        Arc::new(Self {
            inner,
            read_faults: RwLock::new(FaultConfig::new()),
            write_faults: RwLock::new(FaultConfig::new()),
            delete_faults: RwLock::new(FaultConfig::new()),
            scan_faults: RwLock::new(FaultConfig::new()),
        })
    }

    // -----------------------------------------------------------------------
    // Read fault configuration
    // -----------------------------------------------------------------------

    /// Fail all read operations.
    pub async fn fail_all_reads(&self) {
        let mut cfg = self.read_faults.write().await;
        cfg.mode = FaultMode::All;
    }

    /// Fail every Nth read operation.
    pub async fn fail_reads_every(&self, n: u64) {
        let mut cfg = self.read_faults.write().await;
        cfg.mode = FaultMode::EveryN(n);
        cfg.counter = AtomicU64::new(0);
    }

    /// Fail reads for a specific key.
    pub async fn fail_reads_for_key(&self, key: impl Into<String>) {
        let mut cfg = self.read_faults.write().await;
        cfg.mode = FaultMode::All;
        cfg.fail_keys.insert(key.into());
    }

    /// Set custom error message for read failures.
    pub async fn set_read_error(&self, msg: impl Into<String>) {
        let mut cfg = self.read_faults.write().await;
        cfg.error_message = msg.into();
    }

    // -----------------------------------------------------------------------
    // Write fault configuration
    // -----------------------------------------------------------------------

    /// Fail all write operations.
    pub async fn fail_all_writes(&self) {
        let mut cfg = self.write_faults.write().await;
        cfg.mode = FaultMode::All;
    }

    /// Fail every Nth write operation.
    pub async fn fail_writes_every(&self, n: u64) {
        let mut cfg = self.write_faults.write().await;
        cfg.mode = FaultMode::EveryN(n);
        cfg.counter = AtomicU64::new(0);
    }

    /// Fail writes for a specific key.
    pub async fn fail_writes_for_key(&self, key: impl Into<String>) {
        let mut cfg = self.write_faults.write().await;
        cfg.mode = FaultMode::All;
        cfg.fail_keys.insert(key.into());
    }

    /// Set custom error message for write failures.
    pub async fn set_write_error(&self, msg: impl Into<String>) {
        let mut cfg = self.write_faults.write().await;
        cfg.error_message = msg.into();
    }

    // -----------------------------------------------------------------------
    // Delete fault configuration
    // -----------------------------------------------------------------------

    /// Fail all delete operations.
    pub async fn fail_all_deletes(&self) {
        let mut cfg = self.delete_faults.write().await;
        cfg.mode = FaultMode::All;
    }

    // -----------------------------------------------------------------------
    // Scan fault configuration
    // -----------------------------------------------------------------------

    /// Fail all scan operations.
    pub async fn fail_all_scans(&self) {
        let mut cfg = self.scan_faults.write().await;
        cfg.mode = FaultMode::All;
    }

    // -----------------------------------------------------------------------
    // Clear faults
    // -----------------------------------------------------------------------

    /// Remove all injected faults, returning to pass-through mode.
    pub async fn clear_faults(&self) {
        *self.read_faults.write().await = FaultConfig::new();
        *self.write_faults.write().await = FaultConfig::new();
        *self.delete_faults.write().await = FaultConfig::new();
        *self.scan_faults.write().await = FaultConfig::new();
    }
}

#[async_trait]
impl<S: KeyValueStore + ?Sized> KeyValueStore for FailingKeyValueStore<S> {
    async fn write(&self, request: WriteRequest) -> Result<WriteResult, KeyValueStoreError> {
        // Write faults apply to all writes (key extraction is complex for multi-key ops)
        let cfg = self.write_faults.read().await;
        if cfg.should_fail(None) {
            return Err(cfg.make_error());
        }
        drop(cfg);
        self.inner.write(request).await
    }

    async fn read(&self, request: ReadRequest) -> Result<ReadResult, KeyValueStoreError> {
        let key = request.key.clone();
        let cfg = self.read_faults.read().await;
        if cfg.should_fail(Some(&key)) {
            return Err(cfg.make_error());
        }
        drop(cfg);
        self.inner.read(request).await
    }

    async fn delete(&self, request: DeleteRequest) -> Result<DeleteResult, KeyValueStoreError> {
        let key = request.key.clone();
        let cfg = self.delete_faults.read().await;
        if cfg.should_fail(Some(&key)) {
            return Err(cfg.make_error());
        }
        drop(cfg);
        self.inner.delete(request).await
    }

    async fn scan(&self, request: ScanRequest) -> Result<ScanResult, KeyValueStoreError> {
        let prefix = request.prefix.clone();
        let cfg = self.scan_faults.read().await;
        if cfg.should_fail(Some(&prefix)) {
            return Err(cfg.make_error());
        }
        drop(cfg);
        self.inner.scan(request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DeterministicKeyValueStore;

    #[tokio::test]
    async fn test_no_faults_passes_through() {
        let inner = DeterministicKeyValueStore::new();
        let store = FailingKeyValueStore::new(inner);

        let write_result = store
            .write(WriteRequest::set("key1", "value1"))
            .await;
        assert!(write_result.is_ok());

        let read_result = store.read(ReadRequest::new("key1")).await;
        assert!(read_result.is_ok());
        assert_eq!(read_result.unwrap().kv.unwrap().value, "value1");
    }

    #[tokio::test]
    async fn test_fail_all_reads() {
        let inner = DeterministicKeyValueStore::new();
        let store = FailingKeyValueStore::new(inner);

        // Write succeeds
        store.write(WriteRequest::set("key1", "v")).await.unwrap();

        // Enable read faults
        store.fail_all_reads().await;

        let result = store.read(ReadRequest::new("key1")).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("injected fault"));
    }

    #[tokio::test]
    async fn test_fail_all_writes() {
        let inner = DeterministicKeyValueStore::new();
        let store = FailingKeyValueStore::new(inner);

        store.fail_all_writes().await;

        let result = store.write(WriteRequest::set("key1", "v")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fail_every_nth_write() {
        let inner = DeterministicKeyValueStore::new();
        let store = FailingKeyValueStore::new(inner);

        store.fail_writes_every(3).await;

        // Writes 1, 2 succeed; write 3 fails; write 4, 5 succeed; write 6 fails
        assert!(store.write(WriteRequest::set("k1", "v")).await.is_ok());
        assert!(store.write(WriteRequest::set("k2", "v")).await.is_ok());
        assert!(store.write(WriteRequest::set("k3", "v")).await.is_err()); // 3rd
        assert!(store.write(WriteRequest::set("k4", "v")).await.is_ok());
        assert!(store.write(WriteRequest::set("k5", "v")).await.is_ok());
        assert!(store.write(WriteRequest::set("k6", "v")).await.is_err()); // 6th
    }

    #[tokio::test]
    async fn test_fail_specific_key() {
        let inner = DeterministicKeyValueStore::new();
        let store = FailingKeyValueStore::new(inner);

        store.fail_reads_for_key("secret_key").await;

        // Other keys work fine
        store.write(WriteRequest::set("ok_key", "v")).await.unwrap();
        assert!(store.read(ReadRequest::new("ok_key")).await.is_ok());

        // The targeted key fails
        store.write(WriteRequest::set("secret_key", "v")).await.unwrap();
        assert!(store.read(ReadRequest::new("secret_key")).await.is_err());
    }

    #[tokio::test]
    async fn test_clear_faults() {
        let inner = DeterministicKeyValueStore::new();
        let store = FailingKeyValueStore::new(inner);

        store.fail_all_reads().await;
        assert!(store.read(ReadRequest::new("k")).await.is_err());

        store.clear_faults().await;
        // NotFound is OK — the point is it's not an injected error
        let result = store.read(ReadRequest::new("k")).await;
        assert!(matches!(result, Err(KeyValueStoreError::NotFound { .. })));
    }

    #[tokio::test]
    async fn test_custom_error_message() {
        let inner = DeterministicKeyValueStore::new();
        let store = FailingKeyValueStore::new(inner);

        store.fail_all_writes().await;
        store.set_write_error("storage unavailable").await;

        let result = store.write(WriteRequest::set("k", "v")).await;
        assert!(result.unwrap_err().to_string().contains("storage unavailable"));
    }

    #[tokio::test]
    async fn test_fail_all_scans() {
        let inner = DeterministicKeyValueStore::new();
        let store = FailingKeyValueStore::new(inner);

        store.fail_all_scans().await;

        let result = store
            .scan(ScanRequest {
                prefix: String::new(),
                limit_results: Some(10),
                continuation_token: None,
            })
            .await;
        assert!(result.is_err());
    }
}
