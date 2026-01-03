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
