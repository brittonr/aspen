//! Aspen vNext crate boundary.
//!
//! The rebuilt crate organizes functionality into explicit top-level modules so
//! we can iterate on transport, Raft wiring, and the eventual API surface in
//! isolation while still composing them through `ractor` actors later on.

#![deny(missing_docs)]

/// External API surfaces (CLI, RPC, embedding hooks) will be declared here.
pub mod api;
/// Cluster orchestration primitives that sit directly on top of
/// `ractor_cluster::NodeServer` plus Aspen-specific helpers.
pub mod cluster;
/// Raft wiring (state machine + network glue) lives here. Phase 1 only defines
/// the traits so Phase 2 can plug the actual OpenRaft actors in.
pub mod raft;
/// Storage adapters (logs, state machines, snapshots) and their property-test
/// seam points.
pub mod storage;

pub use cluster::{DeterministicClusterConfig, NodeServerConfig, NodeServerHandle};
