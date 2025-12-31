//! Aspen library entry point.
//!
//! The previous iterations of the crate had a much richer surface area that
//! bound `openraft`, Iroh, and `ractor`. Those modules were wiped so we could
//! rebuild the stack deliberately. For now we expose lightweight scaffolding
//! that mirrors the control-plane and transport traits used throughout the
//! integration tests and examples. Each module intentionally keeps the API
//! narrow so we can iterate quickly while wiring the real implementations back
//! in.

#![warn(missing_docs)]

/// Public API constants for size limits and timeouts.
pub mod constants;
/// Trait definitions for cluster management and key-value operations.
pub mod api;
/// Capability-based authorization system.
pub mod auth;
/// Content-addressed blob storage using iroh-blobs.
///
/// Re-exported from the `aspen-blob` crate.
pub use aspen_blob as blob;
/// Client libraries for connecting to Aspen clusters.
pub mod client;
/// Client RPC protocol definitions over Iroh.
pub mod client_rpc;
/// Cluster coordination and bootstrap logic.
pub mod cluster;
/// Distributed coordination primitives (barriers, elections, locks).
///
/// Re-exported from the `aspen-coordination` crate.
pub use aspen_coordination as coordination;
/// DNS record management layer built on iroh-docs.
///
/// Only available with the `dns` feature enabled. Provides DNS record
/// management with CRUD operations, wildcard resolution, zone management,
/// and a hickory-based DNS protocol server.
///
/// Core types are provided by the `aspen-dns` crate, with integration
/// functions added by this module.
#[cfg(feature = "dns")]
pub mod dns;
/// CRDT-based document synchronization using iroh-docs.
pub mod docs;
/// FoundationDB-style layer abstractions (tuple encoding, subspaces).
/// Internal implementation detail - not part of public API.
pub(crate) mod layer;
/// Node builder pattern for programmatic configuration.
pub mod node;
/// Protocol handlers for Iroh Router-based ALPN dispatching.
/// Internal implementation detail - key types re-exported at crate root.
pub(crate) mod protocol_handlers;
pub mod raft;
/// Sharding module for horizontal scaling via key-based partitioning.
pub mod sharding;
pub mod simulation;
/// Forge: Git on Aspen - decentralized code collaboration.
///
/// Only available with the `forge` feature enabled. Provides Git object
/// storage, collaborative objects (issues, patches), and ref management
/// built on Aspen's distributed primitives.
#[cfg(feature = "forge")]
pub mod forge;
/// Pijul: Native patch-based version control.
///
/// Only available with the `pijul` feature enabled. Provides native Pijul
/// integration with libpijul, using Pijul's patch-based model with commutative
/// merges. Changes are stored in iroh-blobs for P2P distribution.
#[cfg(feature = "pijul")]
pub mod pijul;
/// DataFusion SQL integration for Redb storage backend.
///
/// Only available with the `sql` feature enabled. Provides read-only SQL
/// query execution over Redb KV data using Apache DataFusion.
#[cfg(feature = "sql")]
pub mod sql;

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

// Re-export key protocol handler types at crate root
pub use protocol_handlers::{
    AuthenticatedRaftProtocolHandler, CLIENT_ALPN, ClientProtocolContext, ClientProtocolHandler, LOG_SUBSCRIBER_ALPN,
    LogSubscriberProtocolHandler, RAFT_ALPN, RAFT_AUTH_ALPN, RAFT_SHARDED_ALPN, RaftProtocolHandler,
};
// Re-export federation ALPN at crate root for convenient access
pub use cluster::federation::FEDERATION_ALPN;
// Re-export git-bridge ALPN (requires forge + git-bridge features)
#[cfg(all(feature = "forge", feature = "git-bridge"))]
pub use forge::GIT_BRIDGE_ALPN;
// Re-export authentication types
pub use raft::auth::{AuthChallenge, AuthContext, AuthResponse, AuthResult};
// Re-export NodeAddress wrapper type for public API
pub use api::NodeAddress;
// Re-export log subscriber types
pub use raft::log_subscriber::{
    EndOfStreamReason, HistoricalLogReader, KvOperation, LogEntryMessage, LogEntryPayload, SubscribeRejectReason,
    SubscribeRequest, SubscribeResponse,
};

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
    // API request types (HTTP attack surface)
    pub use crate::api::AddLearnerRequest;
    pub use crate::api::ChangeMembershipRequest;
    pub use crate::api::DeleteRequest;
    pub use crate::api::InitRequest;
    pub use crate::api::ReadRequest;
    pub use crate::api::ScanRequest;
    pub use crate::api::WriteCommand;
    pub use crate::api::WriteRequest;
    // KV store traits and implementations for differential fuzzing
    pub use crate::api::{DeterministicKeyValueStore, KeyValueStore, KeyValueStoreError};
    // Client RPC types (Iroh protocol attack surface)
    pub use crate::client_rpc::{ClientRpcRequest, ClientRpcResponse};
    // Cluster ticket type (user input attack surface)
    pub use crate::cluster::ticket::AspenClusterTicket;
    pub use crate::raft::rpc::RaftAppendEntriesRequest;
    pub use crate::raft::rpc::RaftRpcProtocol;
    pub use crate::raft::rpc::RaftRpcResponse;
    pub use crate::raft::rpc::RaftSnapshotRequest;
    pub use crate::raft::rpc::RaftVoteRequest;
    pub use crate::raft::types::AppRequest;
    pub use crate::raft::types::AppResponse;
    pub use crate::raft::types::AppTypeConfig;
    pub use crate::raft::types::NodeId;
    pub use crate::raft::types::RaftMemberInfo;
}
