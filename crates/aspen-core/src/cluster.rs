//! Cluster management types.
//!
//! This module re-exports the cluster types from `aspen-cluster-types` for backward compatibility.
//! New code should prefer importing from `aspen_cluster_types` directly for lighter dependencies.
//!
//! Types for managing cluster membership and topology.

// Re-export everything from aspen-cluster-types
pub use aspen_cluster_types::AddLearnerRequest;
pub use aspen_cluster_types::ChangeMembershipRequest;
pub use aspen_cluster_types::ClusterNode;
pub use aspen_cluster_types::ClusterState;
pub use aspen_cluster_types::InitRequest;
