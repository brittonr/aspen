//! Core traits for Aspen operations.
//!
//! Defines the primary interfaces for cluster control and key-value storage.

use async_trait::async_trait;

use crate::cluster::AddLearnerRequest;
use crate::cluster::ChangeMembershipRequest;
use crate::cluster::ClusterState;
use crate::cluster::InitRequest;
use crate::error::ControlPlaneError;
use crate::error::KeyValueStoreError;
use crate::kv::DeleteRequest;
use crate::kv::DeleteResult;
use crate::kv::ReadRequest;
use crate::kv::ReadResult;
use crate::kv::ScanRequest;
use crate::kv::ScanResult;
use crate::kv::WriteRequest;
use crate::kv::WriteResult;
use crate::types::ClusterMetrics;
use crate::types::SnapshotLogId;

/// Backend trait for coordination primitives to abstract away Raft dependency.
///
/// This trait provides a unified interface for coordination primitives (queues,
/// rate limiters, service registry, etc.) to interact with the underlying
/// distributed system without directly depending on Raft implementation details.
#[async_trait]
pub trait CoordinationBackend: Send + Sync + 'static {
    /// Get a unique timestamp in milliseconds since Unix epoch.
    async fn now_unix_ms(&self) -> u64;

    /// Get the current node ID.
    async fn node_id(&self) -> u64;

    /// Check if this node is the current leader.
    async fn is_leader(&self) -> bool;

    /// Get the key-value store implementation.
    fn kv_store(&self) -> std::sync::Arc<dyn KeyValueStore>;

    /// Get the cluster controller implementation.
    fn cluster_controller(&self) -> std::sync::Arc<dyn ClusterController>;
}

/// Manages cluster membership and Raft consensus operations.
///
/// This trait provides the control plane interface for initializing clusters,
/// managing node membership, and monitoring cluster health.
#[async_trait]
pub trait ClusterController: Send + Sync {
    /// Initialize a new Raft cluster with founding members.
    async fn init(&self, request: InitRequest) -> Result<ClusterState, ControlPlaneError>;

    /// Add a non-voting learner node to the cluster.
    async fn add_learner(&self, request: AddLearnerRequest) -> Result<ClusterState, ControlPlaneError>;

    /// Change the set of voting members in the cluster.
    async fn change_membership(&self, request: ChangeMembershipRequest) -> Result<ClusterState, ControlPlaneError>;

    /// Get the current cluster topology and membership state.
    async fn current_state(&self) -> Result<ClusterState, ControlPlaneError>;

    /// Get the current Raft metrics for observability.
    async fn get_metrics(&self) -> Result<ClusterMetrics, ControlPlaneError>;

    /// Trigger a snapshot to be taken immediately.
    async fn trigger_snapshot(&self) -> Result<Option<SnapshotLogId>, ControlPlaneError>;

    /// Get the current leader ID, if known.
    async fn get_leader(&self) -> Result<Option<u64>, ControlPlaneError> {
        Ok(self.get_metrics().await?.current_leader)
    }

    /// Check if the cluster has been initialized.
    fn is_initialized(&self) -> bool;
}

// Blanket implementation for Arc<T>
#[async_trait]
impl<T: ClusterController> ClusterController for std::sync::Arc<T> {
    async fn init(&self, request: InitRequest) -> Result<ClusterState, ControlPlaneError> {
        (**self).init(request).await
    }

    async fn add_learner(&self, request: AddLearnerRequest) -> Result<ClusterState, ControlPlaneError> {
        (**self).add_learner(request).await
    }

    async fn change_membership(&self, request: ChangeMembershipRequest) -> Result<ClusterState, ControlPlaneError> {
        (**self).change_membership(request).await
    }

    async fn current_state(&self) -> Result<ClusterState, ControlPlaneError> {
        (**self).current_state().await
    }

    async fn get_metrics(&self) -> Result<ClusterMetrics, ControlPlaneError> {
        (**self).get_metrics().await
    }

    async fn trigger_snapshot(&self) -> Result<Option<SnapshotLogId>, ControlPlaneError> {
        (**self).trigger_snapshot().await
    }

    async fn get_leader(&self) -> Result<Option<u64>, ControlPlaneError> {
        (**self).get_leader().await
    }

    fn is_initialized(&self) -> bool {
        (**self).is_initialized()
    }
}

/// Distributed key-value store interface.
///
/// Provides linearizable read/write access to a distributed key-value store
/// backed by Raft consensus.
#[async_trait]
pub trait KeyValueStore: Send + Sync {
    /// Write one or more key-value pairs to the store.
    async fn write(&self, request: WriteRequest) -> Result<WriteResult, KeyValueStoreError>;

    /// Read a value by key with revision metadata.
    async fn read(&self, request: ReadRequest) -> Result<ReadResult, KeyValueStoreError>;

    /// Delete a key from the store.
    async fn delete(&self, request: DeleteRequest) -> Result<DeleteResult, KeyValueStoreError>;

    /// Scan keys matching a prefix with pagination support.
    async fn scan(&self, request: ScanRequest) -> Result<ScanResult, KeyValueStoreError>;
}

// Blanket implementation for Arc<T>
#[async_trait]
impl<T: KeyValueStore + ?Sized> KeyValueStore for std::sync::Arc<T> {
    async fn write(&self, request: WriteRequest) -> Result<WriteResult, KeyValueStoreError> {
        (**self).write(request).await
    }

    async fn read(&self, request: ReadRequest) -> Result<ReadResult, KeyValueStoreError> {
        (**self).read(request).await
    }

    async fn delete(&self, request: DeleteRequest) -> Result<DeleteResult, KeyValueStoreError> {
        (**self).delete(request).await
    }

    async fn scan(&self, request: ScanRequest) -> Result<ScanResult, KeyValueStoreError> {
        (**self).scan(request).await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::ClusterNode;
    use crate::inmemory::DeterministicClusterController;
    use crate::inmemory::DeterministicKeyValueStore;

    // ============================================================================
    // Arc<T> blanket implementation tests for ClusterController
    // ============================================================================

    #[tokio::test]
    async fn arc_cluster_controller_init_delegates_correctly() {
        let controller = DeterministicClusterController::new();
        let arc_controller: Arc<dyn ClusterController> = controller;

        let result = arc_controller
            .init(InitRequest {
                initial_members: vec![ClusterNode::new(1, "node1", None)],
            })
            .await;

        assert!(result.is_ok());
        let state = result.unwrap();
        assert_eq!(state.members, vec![1]);
    }

    #[tokio::test]
    async fn arc_cluster_controller_add_learner_delegates() {
        let controller = DeterministicClusterController::new();
        let arc_controller: Arc<dyn ClusterController> = controller;

        arc_controller
            .init(InitRequest {
                initial_members: vec![ClusterNode::new(1, "node1", None)],
            })
            .await
            .unwrap();

        let result = arc_controller
            .add_learner(AddLearnerRequest {
                learner: ClusterNode::new(2, "learner", None),
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().learners.len(), 1);
    }

    #[tokio::test]
    async fn arc_cluster_controller_change_membership_delegates() {
        let controller = DeterministicClusterController::new();
        let arc_controller: Arc<dyn ClusterController> = controller;

        arc_controller
            .init(InitRequest {
                initial_members: vec![ClusterNode::new(1, "node1", None)],
            })
            .await
            .unwrap();

        let result = arc_controller.change_membership(ChangeMembershipRequest { members: vec![1, 2] }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().members, vec![1, 2]);
    }

    #[tokio::test]
    async fn arc_cluster_controller_current_state_delegates() {
        let controller = DeterministicClusterController::new();
        let arc_controller: Arc<dyn ClusterController> = controller;

        arc_controller
            .init(InitRequest {
                initial_members: vec![ClusterNode::new(1, "node1", None)],
            })
            .await
            .unwrap();

        let state = arc_controller.current_state().await.unwrap();
        assert_eq!(state.nodes.len(), 1);
    }

    #[tokio::test]
    async fn arc_cluster_controller_get_metrics_delegates() {
        let controller = DeterministicClusterController::new();
        let arc_controller: Arc<dyn ClusterController> = controller;

        // get_metrics returns Unsupported for deterministic backend
        let result = arc_controller.get_metrics().await;
        assert!(matches!(result, Err(ControlPlaneError::Unsupported { .. })));
    }

    #[tokio::test]
    async fn arc_cluster_controller_trigger_snapshot_delegates() {
        let controller = DeterministicClusterController::new();
        let arc_controller: Arc<dyn ClusterController> = controller;

        let result = arc_controller.trigger_snapshot().await;
        assert!(matches!(result, Err(ControlPlaneError::Unsupported { .. })));
    }

    #[tokio::test]
    async fn arc_cluster_controller_get_leader_delegates() {
        let controller = DeterministicClusterController::new();
        let arc_controller: Arc<dyn ClusterController> = controller;

        // DeterministicClusterController overrides get_leader() to return Ok(None)
        // This tests that the Arc wrapper properly delegates
        let result = arc_controller.get_leader().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none()); // Deterministic backend has no leader
    }

    #[tokio::test]
    async fn arc_cluster_controller_is_initialized_delegates() {
        let controller = DeterministicClusterController::new();
        let arc_controller: Arc<dyn ClusterController> = controller;

        // Deterministic controller is always initialized
        assert!(arc_controller.is_initialized());
    }

    // ============================================================================
    // Arc<T> blanket implementation tests for KeyValueStore
    // ============================================================================

    #[tokio::test]
    async fn arc_kv_store_write_delegates_correctly() {
        let store = DeterministicKeyValueStore::new();
        let arc_store: Arc<dyn KeyValueStore> = store;

        let result = arc_store.write(WriteRequest::set("key", "value")).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn arc_kv_store_read_delegates_correctly() {
        let store = DeterministicKeyValueStore::new();
        let arc_store: Arc<dyn KeyValueStore> = store;

        arc_store.write(WriteRequest::set("key", "value")).await.unwrap();

        let result = arc_store.read(ReadRequest::new("key")).await;
        assert!(result.is_ok());
        let kv = result.unwrap().kv.unwrap();
        assert_eq!(kv.key, "key");
        assert_eq!(kv.value, "value");
    }

    #[tokio::test]
    async fn arc_kv_store_delete_delegates_correctly() {
        let store = DeterministicKeyValueStore::new();
        let arc_store: Arc<dyn KeyValueStore> = store;

        arc_store.write(WriteRequest::set("key", "value")).await.unwrap();
        let result = arc_store.delete(DeleteRequest::new("key")).await;

        assert!(result.is_ok());
        assert!(result.unwrap().deleted);
    }

    #[tokio::test]
    async fn arc_kv_store_scan_delegates_correctly() {
        let store = DeterministicKeyValueStore::new();
        let arc_store: Arc<dyn KeyValueStore> = store;

        arc_store.write(WriteRequest::set("prefix:a", "1")).await.unwrap();
        arc_store.write(WriteRequest::set("prefix:b", "2")).await.unwrap();

        let result = arc_store
            .scan(ScanRequest {
                prefix: "prefix:".to_string(),
                limit: None,
                continuation_token: None,
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().count, 2);
    }

    // ============================================================================
    // Trait object (dyn) tests - verifying dynamic dispatch works
    // ============================================================================

    #[tokio::test]
    async fn dyn_cluster_controller_can_be_stored_and_used() {
        // Test that we can store as Box<dyn ClusterController>
        let controller: Box<dyn ClusterController> =
            Box::new(Arc::new(crate::inmemory::DeterministicClusterController::default()));

        let result = controller
            .init(InitRequest {
                initial_members: vec![ClusterNode::new(1, "node", None)],
            })
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn dyn_kv_store_can_be_stored_and_used() {
        // Test that we can store as Box<dyn KeyValueStore>
        let store: Box<dyn KeyValueStore> = Box::new(Arc::new(crate::inmemory::DeterministicKeyValueStore::default()));

        store.write(WriteRequest::set("test", "value")).await.unwrap();
        let result = store.read(ReadRequest::new("test")).await.unwrap();
        assert_eq!(result.kv.unwrap().value, "value");
    }

    // ============================================================================
    // Multiple Arc wrappers (Arc<Arc<T>>) - edge case
    // ============================================================================

    #[tokio::test]
    async fn nested_arc_kv_store_works() {
        let inner = DeterministicKeyValueStore::new();
        let outer: Arc<Arc<crate::inmemory::DeterministicKeyValueStore>> = Arc::new(inner);

        // This tests that Arc<Arc<T>> where T: KeyValueStore also works
        outer.write(WriteRequest::set("nested", "works")).await.unwrap();
        let result = outer.read(ReadRequest::new("nested")).await.unwrap();
        assert_eq!(result.kv.unwrap().value, "works");
    }

    // ============================================================================
    // Send + Sync bounds verification
    // ============================================================================

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
}
