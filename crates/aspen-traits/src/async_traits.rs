use alloc::boxed::Box;
use alloc::sync::Arc;
use async_trait::async_trait;

use crate::AddLearnerRequest;
use crate::ChangeMembershipRequest;
use crate::ClusterMetrics;
use crate::ClusterState;
use crate::ControlPlaneError;
use crate::DeleteRequest;
use crate::DeleteResult;
use crate::InitRequest;
use crate::KeyValueStoreError;
use crate::ReadRequest;
use crate::ReadResult;
use crate::ScanRequest;
use crate::ScanResult;
use crate::SnapshotLogId;
use crate::WriteRequest;
use crate::WriteResult;

// ============================================================================
// KV Capability Traits
// ============================================================================

/// Read a single key from the store.
#[async_trait]
pub trait KvRead: Send + Sync {
    /// Read a value by key with revision metadata.
    async fn read(&self, request: ReadRequest) -> Result<ReadResult, KeyValueStoreError>;
}

/// Write one or more key-value pairs to the store.
#[async_trait]
pub trait KvWrite: Send + Sync {
    /// Write one or more key-value pairs to the store.
    async fn write(&self, request: WriteRequest) -> Result<WriteResult, KeyValueStoreError>;
}

/// Delete a key from the store.
#[async_trait]
pub trait KvDelete: Send + Sync {
    /// Delete a key from the store.
    async fn delete(&self, request: DeleteRequest) -> Result<DeleteResult, KeyValueStoreError>;
}

/// Scan keys matching a prefix with linearizable consistency.
#[async_trait]
pub trait KvScan: Send + Sync {
    /// Scan keys matching a prefix with pagination support.
    async fn scan(&self, request: ScanRequest) -> Result<ScanResult, KeyValueStoreError>;
}

/// Scan keys from the local state machine without linearizability guarantees.
///
/// Unlike [`KvScan::scan`], this reads directly from the local state machine
/// without confirming leadership or contacting the Raft leader. The data may
/// be slightly stale, but it is safe for use cases where eventual consistency
/// is acceptable:
///
/// - Plugin manifest discovery at startup
/// - Cache warming on followers
/// - Background index rebuilding
#[async_trait]
pub trait KvLocalScan: Send + Sync {
    /// Scan keys from the local state machine.
    async fn scan_local(&self, request: ScanRequest) -> Result<ScanResult, KeyValueStoreError>;
}

// ============================================================================
// Arc blanket impls for KV capability traits
// ============================================================================

#[async_trait]
impl<T: KvRead + ?Sized> KvRead for Arc<T> {
    async fn read(&self, request: ReadRequest) -> Result<ReadResult, KeyValueStoreError> {
        (**self).read(request).await
    }
}

#[async_trait]
impl<T: KvWrite + ?Sized> KvWrite for Arc<T> {
    async fn write(&self, request: WriteRequest) -> Result<WriteResult, KeyValueStoreError> {
        (**self).write(request).await
    }
}

#[async_trait]
impl<T: KvDelete + ?Sized> KvDelete for Arc<T> {
    async fn delete(&self, request: DeleteRequest) -> Result<DeleteResult, KeyValueStoreError> {
        (**self).delete(request).await
    }
}

#[async_trait]
impl<T: KvScan + ?Sized> KvScan for Arc<T> {
    async fn scan(&self, request: ScanRequest) -> Result<ScanResult, KeyValueStoreError> {
        (**self).scan(request).await
    }
}

#[async_trait]
impl<T: KvLocalScan + ?Sized> KvLocalScan for Arc<T> {
    async fn scan_local(&self, request: ScanRequest) -> Result<ScanResult, KeyValueStoreError> {
        (**self).scan_local(request).await
    }
}

// ============================================================================
// CoordinationBackend
// ============================================================================

/// Backend trait for coordination primitives to abstract away Raft dependency.
#[async_trait]
pub trait CoordinationBackend: Send + Sync + 'static {
    /// Get a unique timestamp in milliseconds since Unix epoch.
    async fn now_unix_ms(&self) -> u64;

    /// Get the current node ID.
    async fn node_id(&self) -> u64;

    /// Check if this node is the current leader.
    async fn is_leader(&self) -> bool;

    /// Get the key-value store implementation.
    fn kv_store(&self) -> Arc<dyn KeyValueStore>;

    /// Get the cluster controller implementation.
    fn cluster_controller(&self) -> Arc<dyn ClusterController>;
}

// ============================================================================
// ClusterController
// ============================================================================

/// Manages cluster membership and Raft consensus operations.
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

    /// Trigger Raft leadership transfer to the specified node.
    async fn transfer_leader(&self, target: u64) -> Result<(), ControlPlaneError>;

    /// Check if the cluster has been initialized.
    fn is_initialized(&self) -> bool;
}

#[async_trait]
impl<T: ClusterController> ClusterController for Arc<T> {
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

    async fn transfer_leader(&self, target: u64) -> Result<(), ControlPlaneError> {
        (**self).transfer_leader(target).await
    }

    fn is_initialized(&self) -> bool {
        (**self).is_initialized()
    }
}

// ============================================================================
// KeyValueStore (composite compatibility trait)
// ============================================================================

/// Distributed key-value store interface.
///
/// This is a composite trait combining all KV capabilities for consumers that
/// need the full store surface. New code should prefer narrow capability traits
/// ([`KvRead`], [`KvWrite`], [`KvDelete`], [`KvScan`]) where possible.
#[async_trait]
pub trait KeyValueStore: KvRead + KvWrite + KvDelete + KvScan + Send + Sync {
    /// Scan keys from the local state machine without linearizability guarantees.
    async fn scan_local(&self, request: ScanRequest) -> Result<ScanResult, KeyValueStoreError> {
        self.scan(request).await
    }
}

#[async_trait]
impl<T: KeyValueStore + ?Sized> KeyValueStore for Arc<T> {
    async fn scan_local(&self, request: ScanRequest) -> Result<ScanResult, KeyValueStoreError> {
        (**self).scan_local(request).await
    }
}
