//! Core traits for Aspen operations.
//!
//! This module re-exports the core traits from `aspen-traits` for backward compatibility.
//! New code should prefer importing from `aspen_traits` directly for lighter dependencies.
//!
//! Defines the primary interfaces for cluster control and key-value storage.

// Re-export all traits from aspen-traits
pub use aspen_traits::ClusterController;
pub use aspen_traits::CoordinationBackend;
pub use aspen_traits::KeyValueStore;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::ClusterNode;
    use crate::cluster::AddLearnerRequest;
    use crate::cluster::ChangeMembershipRequest;
    use crate::cluster::ClusterState;
    use crate::cluster::InitRequest;
    use crate::error::ControlPlaneError;
    use crate::kv::DeleteRequest;
    use crate::kv::ReadRequest;
    use crate::kv::ScanRequest;
    use crate::kv::WriteRequest;
    use crate::test_support::DeterministicClusterController;
    use crate::test_support::DeterministicKeyValueStore;
    use crate::types::ClusterMetrics;

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

        // get_metrics returns ClusterMetrics for deterministic backend
        let result = arc_controller.get_metrics().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn arc_cluster_controller_trigger_snapshot_delegates() {
        let controller = DeterministicClusterController::new();
        let arc_controller: Arc<dyn ClusterController> = controller;

        // trigger_snapshot returns Ok(None) for deterministic backend
        let result = arc_controller.trigger_snapshot().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
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

        // Not initialized until init() is called
        assert!(!arc_controller.is_initialized());

        // Initialize the cluster
        arc_controller
            .init(InitRequest {
                initial_members: vec![ClusterNode::new(1, "node1", None)],
            })
            .await
            .unwrap();

        // Now should be initialized
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
        assert!(result.unwrap().is_deleted);
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
            Box::new(Arc::new(crate::test_support::DeterministicClusterController::default()));

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
        let store: Box<dyn KeyValueStore> =
            Box::new(Arc::new(crate::test_support::DeterministicKeyValueStore::default()));

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
        let outer: Arc<Arc<crate::test_support::DeterministicKeyValueStore>> = Arc::new(inner);

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

    // ============================================================================
    // Default trait method tests
    // ============================================================================

    /// A minimal ClusterController that doesn't override get_leader(),
    /// allowing us to test the default implementation.
    struct MinimalClusterController {
        leader: Option<u64>,
    }

    #[async_trait::async_trait]
    impl ClusterController for MinimalClusterController {
        async fn init(&self, _request: InitRequest) -> Result<ClusterState, ControlPlaneError> {
            Ok(ClusterState::default())
        }

        async fn add_learner(&self, _request: AddLearnerRequest) -> Result<ClusterState, ControlPlaneError> {
            Ok(ClusterState::default())
        }

        async fn change_membership(
            &self,
            _request: ChangeMembershipRequest,
        ) -> Result<ClusterState, ControlPlaneError> {
            Ok(ClusterState::default())
        }

        async fn current_state(&self) -> Result<ClusterState, ControlPlaneError> {
            Ok(ClusterState::default())
        }

        async fn get_metrics(&self) -> Result<ClusterMetrics, ControlPlaneError> {
            Ok(ClusterMetrics {
                id: 1,
                state: crate::types::NodeState::Leader,
                current_leader: self.leader,
                current_term: 1,
                last_log_index: Some(10),
                last_applied_index: Some(10),
                snapshot_index: None,
                replication: None,
                voters: vec![1],
                learners: vec![],
            })
        }

        async fn trigger_snapshot(&self) -> Result<Option<crate::types::SnapshotLogId>, ControlPlaneError> {
            Ok(None)
        }

        fn is_initialized(&self) -> bool {
            true
        }
        // NOTE: We intentionally don't override get_leader() here
        // to test the default implementation
    }

    #[tokio::test]
    async fn default_get_leader_uses_metrics() {
        // Test that the default get_leader() implementation calls get_metrics()
        let controller = MinimalClusterController { leader: Some(42) };

        // This calls the default implementation at lines 72-74
        let result = controller.get_leader().await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(42));
    }

    #[tokio::test]
    async fn default_get_leader_returns_none_when_no_leader() {
        let controller = MinimalClusterController { leader: None };

        let result = controller.get_leader().await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[tokio::test]
    async fn minimal_controller_stub_methods_work() {
        // Exercise all the stub methods to get coverage
        let controller = MinimalClusterController { leader: None };

        // init stub
        let result = controller
            .init(InitRequest {
                initial_members: vec![ClusterNode::new(1, "node1", None)],
            })
            .await;
        assert!(result.is_ok());

        // add_learner stub
        let result = controller
            .add_learner(AddLearnerRequest {
                learner: ClusterNode::new(2, "node2", None),
            })
            .await;
        assert!(result.is_ok());

        // change_membership stub
        let result = controller.change_membership(ChangeMembershipRequest { members: vec![1] }).await;
        assert!(result.is_ok());

        // current_state stub
        let result = controller.current_state().await;
        assert!(result.is_ok());

        // get_metrics (not a stub)
        let result = controller.get_metrics().await;
        assert!(result.is_ok());

        // trigger_snapshot stub
        let result = controller.trigger_snapshot().await;
        assert!(result.is_ok());

        // is_initialized stub
        assert!(controller.is_initialized());
    }
}
