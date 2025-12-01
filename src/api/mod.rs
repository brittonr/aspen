//! Public API scaffolding.
//!
//! # Responsibilities
//! - Describe how client-facing APIs (HTTP, CLI, SDKs) will call into the
//!   underlying Raft actor graph.
//! - Keep the seams deterministic so CLI smoke-tests can run under `madsim`
//!   while replaying captured seeds.
//! - Document where property-based request validation tests should land.
//!
//! # Composition
//! The API layer depends on [`crate::cluster`] for node handles, on
//! [`crate::raft`] for logical endpoints (client write, read-only queries), and
//! on [`crate::storage`] for debugging/test hooks.

use crate::cluster::NodeServerHandle;
use crate::raft::RaftNodeSpec;
use crate::storage::StorageSurface;

pub mod control;
pub use control::{
    AddLearnerRequest, ChangeMembershipRequest, ClusterController, ClusterNode, ClusterState,
    ControlPlaneError, ExternalControlPlane, InMemoryControlPlane, InitRequest, KeyValueStore,
    KeyValueStoreError, NodeId, ReadRequest, ReadResult, WriteCommand, WriteRequest, WriteResult,
};

/// High-level API surface describing how future handlers gain access to cluster state.
#[derive(Debug, Clone)]
pub struct ApiSurface {
    cluster: String,
}

impl ApiSurface {
    /// Build the API descriptor for the provided cluster namespace.
    pub fn new(cluster: impl Into<String>) -> Self {
        let cluster = cluster.into();
        assert!(
            !cluster.is_empty(),
            "cluster namespace helps us pin API harnesses to deterministic seeds"
        );
        Self { cluster }
    }

    /// Build a request context; later phases will use this to feed Raft RPCs.
    pub fn context<'a>(
        &'a self,
        node: &'a NodeServerHandle,
        storage: &'a StorageSurface,
        spec: &'a RaftNodeSpec,
    ) -> ApiContext<'a> {
        ApiContext {
            cluster: &self.cluster,
            node,
            storage,
            spec,
        }
    }
}

/// Request context plumbed into actual API handlers.
#[derive(Debug)]
pub struct ApiContext<'a> {
    /// Cluster namespace (mirrors `RaftNodeSpec::cluster`).
    pub cluster: &'a str,
    /// Node handle for registering temporary actors per request.
    pub node: &'a NodeServerHandle,
    /// Storage surface for diagnostics (read-only).
    pub storage: &'a StorageSurface,
    /// Raft node metadata for routing.
    pub spec: &'a RaftNodeSpec,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_surface_requires_namespace() {
        let surface = ApiSurface::new("aspen::primary");
        assert_eq!(surface.cluster, "aspen::primary");
    }
}
