//! Aspen library entry point.
//!
//! The previous iterations of the crate had a much richer surface area that
//! bound `openraft`, Iroh, and `ractor`. Those modules were wiped so we could
//! rebuild the stack deliberately. For now we expose lightweight scaffolding
//! that mirrors the control-plane and transport traits used throughout the
//! integration tests and examples. Each module intentionally keeps the API
//! narrow so we can iterate quickly while wiring the real implementations back
//! in.

pub mod api;
pub mod cluster;
pub mod node;
pub mod protocol_handlers;
pub mod raft;
pub mod simulation;
pub mod tui_rpc;
pub mod tui_rpc_server;

/// Testing infrastructure for deterministic multi-node Raft tests.
///
/// Provides `AspenRouter` for managing in-memory Raft clusters with simulated
/// networking. Used by integration tests in tests/ directory.
pub mod testing;

/// System utility functions for resource management and health checks.
///
/// Provides Tiger Style resource management including disk space checking
/// with fixed thresholds and fail-fast semantics.
pub mod utils;

pub use node::NodeBuilder;
// Note: NodeClient removed - use RaftNode directly from bootstrap_simple
// Note: RaftControlClient removed - use RaftNode directly from bootstrap_simple

/// Fuzzing-specific module that exposes internal types for fuzz testing.
///
/// Only available when compiled with the `fuzzing` feature.
/// Exposes serialization types that are normally internal to allow
/// fuzzing of deserialization code paths.
#[cfg(feature = "fuzzing")]
pub mod fuzz_helpers {
    //! Fuzzing helpers that expose internal types for security testing.
    //!
    //! This module re-exports types used in serialization boundaries that
    //! need to be fuzz-tested. These types handle untrusted input from:
    //! - Network peers (Raft RPC)
    //! - HTTP clients (API requests)
    //! - Gossip messages (peer discovery)
    //! - Cluster tickets (bootstrap)

    // Raft RPC types (network attack surface)
    pub use crate::raft::rpc::{
        RaftAppendEntriesRequest, RaftRpcProtocol, RaftRpcResponse, RaftSnapshotRequest,
        RaftVoteRequest,
    };
    pub use crate::raft::types::{AppRequest, AppResponse, AppTypeConfig, NodeId, RaftMemberInfo};

    // API request types (HTTP attack surface)
    pub use crate::api::{
        AddLearnerRequest, ChangeMembershipRequest, DeleteRequest, InitRequest, ReadRequest,
        ScanRequest, WriteRequest,
    };

    // Cluster ticket type (user input attack surface)
    pub use crate::cluster::ticket::AspenClusterTicket;

    // TUI RPC types
    pub use crate::tui_rpc::{TuiRpcRequest, TuiRpcResponse};
}
