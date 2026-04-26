//! Core traits for Aspen distributed systems.
//!
//! This crate defines the primary interfaces for cluster control and key-value storage.
//!
//! ## KV Capability Traits
//!
//! Narrow capability traits following the Interface Segregation Principle:
//!
//! - [`KvRead`]: Single-key reads
//! - [`KvWrite`]: Write commands
//! - [`KvDelete`]: Delete commands
//! - [`KvScan`]: Prefix scans (linearizable)
//! - [`KvLocalScan`]: Local state-machine scans (stale/eventual)
//!
//! The composite [`KeyValueStore`] trait preserves compatibility for consumers
//! that need the full KV surface.
//!
//! ## Other Traits
//!
//! - [`ClusterController`]: Manages cluster membership and Raft consensus operations
//! - [`CoordinationBackend`]: Backend trait for coordination primitives
//!
//! ## Blanket Implementations
//!
//! All traits have blanket implementations for `Arc<T>` to support easy sharing
//! across tasks and threads.
//!
//! ## Feature Flags
//!
//! - `async` (default): Enables async trait definitions via `async-trait`. Without
//!   this feature, only the type re-exports are available — suitable for alloc-only
//!   consumers that need the types but not the async runtime.

#![cfg_attr(not(test), no_std)]

extern crate alloc;

// Re-export types from aspen-cluster-types needed for ClusterController
pub use aspen_cluster_types::AddLearnerRequest;
pub use aspen_cluster_types::ChangeMembershipRequest;
pub use aspen_cluster_types::ClusterMetrics;
pub use aspen_cluster_types::ClusterState;
pub use aspen_cluster_types::ControlPlaneError;
pub use aspen_cluster_types::InitRequest;
pub use aspen_cluster_types::SnapshotLogId;
// Re-export types from aspen-kv-types needed for KeyValueStore
pub use aspen_kv_types::DeleteRequest;
pub use aspen_kv_types::DeleteResult;
pub use aspen_kv_types::KeyValueStoreError;
pub use aspen_kv_types::ReadRequest;
pub use aspen_kv_types::ReadResult;
pub use aspen_kv_types::ScanRequest;
pub use aspen_kv_types::ScanResult;
pub use aspen_kv_types::WriteRequest;
pub use aspen_kv_types::WriteResult;

// ============================================================================
// Async trait definitions (gated behind `async` feature)
// ============================================================================

#[cfg(feature = "async")]
mod async_traits;

#[cfg(feature = "async")]
pub use async_traits::*;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::sync::Arc;

    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    #[test]
    fn cluster_controller_is_send_sync() {
        assert_send::<Arc<dyn ClusterController>>();
        assert_sync::<Arc<dyn ClusterController>>();
    }

    #[test]
    fn key_value_store_is_send_sync() {
        assert_send::<Arc<dyn KeyValueStore>>();
        assert_sync::<Arc<dyn KeyValueStore>>();
    }

    #[test]
    fn coordination_backend_is_send_sync() {
        assert_send::<Arc<dyn CoordinationBackend>>();
        assert_sync::<Arc<dyn CoordinationBackend>>();
    }

    #[test]
    fn kv_read_is_send_sync() {
        assert_send::<Arc<dyn KvRead>>();
        assert_sync::<Arc<dyn KvRead>>();
    }

    #[test]
    fn kv_write_is_send_sync() {
        assert_send::<Arc<dyn KvWrite>>();
        assert_sync::<Arc<dyn KvWrite>>();
    }

    #[test]
    fn kv_delete_is_send_sync() {
        assert_send::<Arc<dyn KvDelete>>();
        assert_sync::<Arc<dyn KvDelete>>();
    }

    #[test]
    fn kv_scan_is_send_sync() {
        assert_send::<Arc<dyn KvScan>>();
        assert_sync::<Arc<dyn KvScan>>();
    }

    #[test]
    fn kv_local_scan_is_send_sync() {
        assert_send::<Arc<dyn KvLocalScan>>();
        assert_sync::<Arc<dyn KvLocalScan>>();
    }

    #[test]
    fn key_value_store_implies_capabilities() {
        fn accepts_read<T: KvRead>(_: &T) {}
        fn accepts_write<T: KvWrite>(_: &T) {}
        fn accepts_delete<T: KvDelete>(_: &T) {}
        fn accepts_scan<T: KvScan>(_: &T) {}
        fn from_kv_store<T: KeyValueStore>(store: &T) {
            accepts_read(store);
            accepts_write(store);
            accepts_delete(store);
            accepts_scan(store);
        }
        let _ = from_kv_store::<Arc<dyn KeyValueStore>>;
    }

    /// Fixture: linearizable-only store implementing KvRead+KvWrite+KvDelete+KvScan
    /// but NOT KvLocalScan.
    struct LinearizableOnlyStore;

    #[async_trait::async_trait]
    impl KvRead for LinearizableOnlyStore {
        async fn read(&self, _: ReadRequest) -> Result<ReadResult, KeyValueStoreError> {
            unimplemented!()
        }
    }
    #[async_trait::async_trait]
    impl KvWrite for LinearizableOnlyStore {
        async fn write(&self, _: WriteRequest) -> Result<WriteResult, KeyValueStoreError> {
            unimplemented!()
        }
    }
    #[async_trait::async_trait]
    impl KvDelete for LinearizableOnlyStore {
        async fn delete(&self, _: DeleteRequest) -> Result<DeleteResult, KeyValueStoreError> {
            unimplemented!()
        }
    }
    #[async_trait::async_trait]
    impl KvScan for LinearizableOnlyStore {
        async fn scan(&self, _: ScanRequest) -> Result<ScanResult, KeyValueStoreError> {
            unimplemented!()
        }
    }

    /// Fixture: store implementing both KvScan + KvLocalScan.
    struct BothScansStore;

    #[async_trait::async_trait]
    impl KvScan for BothScansStore {
        async fn scan(&self, _: ScanRequest) -> Result<ScanResult, KeyValueStoreError> {
            unimplemented!()
        }
    }
    #[async_trait::async_trait]
    impl KvLocalScan for BothScansStore {
        async fn scan_local(&self, _: ScanRequest) -> Result<ScanResult, KeyValueStoreError> {
            unimplemented!()
        }
    }

    #[test]
    fn linearizable_only_store_does_not_require_local_scan() {
        fn accepts_read<T: KvRead>(_: &T) {}
        fn accepts_write<T: KvWrite>(_: &T) {}
        fn accepts_delete<T: KvDelete>(_: &T) {}
        fn accepts_scan<T: KvScan>(_: &T) {}

        let store = LinearizableOnlyStore;
        accepts_read(&store);
        accepts_write(&store);
        accepts_delete(&store);
        accepts_scan(&store);
    }

    #[test]
    fn explicit_local_scan_capability_is_additive() {
        fn needs_both_scans<T: KvScan + KvLocalScan>(_: &T) {}
        let store = BothScansStore;
        needs_both_scans(&store);
    }
}
