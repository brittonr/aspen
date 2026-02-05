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
///
/// Re-exported from the `aspen-api` crate.
pub use aspen_api as api;
/// Capability-based authorization system.
pub mod auth;
/// Content-addressed blob storage using iroh-blobs.
///
/// Re-exported from the `aspen-blob` crate.
pub use aspen_blob as blob;
/// Client libraries for connecting to Aspen clusters.
///
/// Re-exported from the `aspen-client` crate.
pub use aspen_client as client;
/// Client RPC protocol definitions over Iroh.
///
/// Re-exported from the `aspen-client-api` crate.
pub use aspen_client_api as client_rpc;
/// Cluster coordination and bootstrap logic.
///
/// Re-exported from the `aspen-cluster` crate.
pub use aspen_cluster as cluster;
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
/// Re-exported from the `aspen-dns` crate.
#[cfg(feature = "dns")]
pub use aspen_dns as dns;
/// CRDT-based document synchronization using iroh-docs.
///
/// Re-exported from the `aspen-docs` crate.
pub use aspen_docs as docs;
/// FoundationDB-style layer abstractions (tuple encoding, subspaces).
/// Internal implementation detail - not part of public API.
pub(crate) mod layer;
/// Node builder pattern for programmatic configuration.
pub mod node;
/// Protocol adapters for bridging internal types with trait interfaces.
pub mod protocol_adapters;
/// Forge: Git on Aspen - decentralized code collaboration.
///
/// Only available with the `forge` feature enabled. Provides Git object
/// storage, collaborative objects (issues, patches), and ref management
/// built on Aspen's distributed primitives.
///
/// Re-exported from the `aspen-forge` crate.
#[cfg(feature = "forge")]
pub use aspen_forge as forge;
/// Pijul: Native patch-based version control.
///
/// Only available with the `pijul` feature enabled. Provides native Pijul
/// integration with libpijul, using Pijul's patch-based model with commutative
/// merges. Changes are stored in iroh-blobs for P2P distribution.
///
/// Re-exported from the `aspen-pijul` crate.
#[cfg(feature = "pijul")]
pub use aspen_pijul as pijul;
/// Raft consensus implementation with direct async APIs.
///
/// Re-exported from the `aspen-raft` crate.
pub use aspen_raft as raft;
/// Sharding module for horizontal scaling via key-based partitioning.
///
/// Re-exported from the `aspen-sharding` crate.
pub use aspen_sharding as sharding;
/// DataFusion SQL integration for Redb storage backend.
///
/// Only available with the `sql` feature enabled. Provides read-only SQL
/// query execution over Redb KV data using Apache DataFusion.
#[cfg(feature = "sql")]
pub mod sql;

/// Hybrid Logical Clock utilities for deterministic distributed ordering.
///
/// Re-exported from the `aspen-core` crate.
pub use aspen_core::hlc;
/// Testing infrastructure for deterministic multi-node Raft tests.
///
/// Provides `AspenRouter` for managing in-memory Raft clusters with simulated
/// networking. Used by integration tests in tests/ directory.
///
/// Re-exported from the `aspen-testing` crate.
#[cfg(any(test, feature = "testing"))]
pub use aspen_testing as testing;

/// System utility functions for resource management and health checks.
///
/// Provides Tiger Style resource management including disk space checking
/// with fixed thresholds and fail-fast semantics.
pub mod utils;

// Re-export NodeAddress wrapper type for public API
pub use api::NodeAddress;
// Note: NodeClient removed - use RaftNode directly from bootstrap_simple
// Note: RaftControlClient removed - use RaftNode directly from bootstrap_simple

// Re-export key protocol handler types at crate root
pub use aspen_client_api::CLIENT_ALPN;
// Re-export git-bridge ALPN (requires forge + git-bridge features)
#[cfg(all(feature = "forge", feature = "git-bridge"))]
pub use aspen_forge::GIT_BRIDGE_ALPN;
pub use aspen_rpc_handlers::ClientProtocolContext;
pub use aspen_rpc_handlers::ClientProtocolHandler;
pub use aspen_transport::AuthenticatedRaftProtocolHandler;
pub use aspen_transport::LOG_SUBSCRIBER_ALPN;
pub use aspen_transport::LogSubscriberProtocolHandler;
#[allow(deprecated)] // Re-export for backward compatibility
pub use aspen_transport::RAFT_ALPN;
pub use aspen_transport::RAFT_AUTH_ALPN;
pub use aspen_transport::RAFT_SHARDED_ALPN;
pub use aspen_transport::RaftProtocolHandler;
pub use aspen_transport::ShardedRaftProtocolHandler;
pub use aspen_transport::TrustedPeersRegistry;
// Re-export federation ALPN at crate root for convenient access
pub use cluster::federation::FEDERATION_ALPN;
pub use node::NodeBuilder;
// Re-export authentication types
pub use raft::auth::{AuthChallenge, AuthContext, AuthResponse, AuthResult};
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
///
/// This module re-exports types used in serialization boundaries that
/// need to be fuzz-tested. These types handle untrusted input from:
/// - Network peers (Raft RPC)
/// - Iroh Client RPC (API requests)
/// - Gossip messages (peer discovery)
/// - Cluster tickets (bootstrap)
#[cfg(feature = "fuzzing")]
pub mod fuzz_helpers {

    // Raft RPC types (network attack surface)
    // API request types (Iroh Client RPC attack surface)
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
