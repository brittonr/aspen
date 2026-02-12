//! Core wrapper types for Aspen.
//!
//! This module re-exports the core types from `aspen-cluster-types` for backward compatibility.
//! New code should prefer importing from `aspen_cluster_types` directly for lighter dependencies.
//!
//! This module provides type-safe wrappers around external dependencies,
//! hiding implementation details while providing stable public APIs.

// Re-export everything from aspen-cluster-types
pub use aspen_cluster_types::ClusterMetrics;
pub use aspen_cluster_types::NodeAddress;
pub use aspen_cluster_types::NodeId;
pub use aspen_cluster_types::NodeState;
pub use aspen_cluster_types::SnapshotLogId;
