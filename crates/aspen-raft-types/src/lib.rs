//! Raft consensus type definitions for Aspen distributed systems.
//!
//! This crate contains the core type definitions used by Aspen's Raft
//! implementation, including application requests/responses, node metadata,
//! and network constants.
//!
//! # Overview
//!
//! The types in this crate are organized into several categories:
//!
//! - **Application types** ([`AppRequest`], [`AppResponse`]): The commands and responses replicated
//!   through Raft consensus
//! - **Node metadata** ([`RaftMemberInfo`]): Transport-neutral information about cluster members
//! - **Constants**: Network, storage, and timing constants
//!
//! # AppTypeConfig
//!
//! `AppTypeConfig` lives here as the shared leaf type configuration used by
//! consensus, transport, and router layers.
//!
//! # Tiger Style
//!
//! All types follow Tiger Style principles:
//! - Explicit bounds on all collections and operations
//! - Serializable for wire protocol compatibility
//! - Clear documentation of invariants and constraints

pub mod constants;
pub mod member;
pub mod network;
pub mod request;

// Re-export all public types at crate root for convenience

// Application types
// Re-export NodeId from aspen-core for convenience
pub use aspen_core::NodeId;
// Re-export TxnOpResult for AppResponse
pub use aspen_core::TxnOpResult;
// Constants
pub use constants::FAILURE_DETECTOR_CHANNEL_CAPACITY;
pub use constants::IROH_CONNECT_TIMEOUT;
pub use constants::IROH_READ_TIMEOUT;
pub use constants::IROH_STREAM_OPEN_TIMEOUT;
pub use constants::MAX_CONCURRENT_CONNECTIONS;
pub use constants::MAX_PEERS;
pub use constants::MAX_RPC_MESSAGE_SIZE;
pub use constants::MAX_SNAPSHOT_SIZE;
pub use constants::MAX_STREAMS_PER_CONNECTION;
pub use constants::MEMBERSHIP_OPERATION_TIMEOUT;
pub use constants::MIN_SNAPSHOT_LOG_THRESHOLD;
pub use constants::READ_INDEX_TIMEOUT;
pub use constants::SNAPSHOT_INSTALL_TIMEOUT_MS;
// Node metadata
pub use member::RaftMemberInfo;
// Network types
pub use network::ConnectionHealth;
pub use network::ConnectionStatus;
pub use network::DriftSeverity;
pub use network::FailureType;
pub use network::StreamPriority;
use openraft::declare_raft_types;
pub use request::AppRequest;
pub use request::AppResponse;
pub use request::TrustInitializePayload;
pub use request::TrustReconfigurationPayload;

declare_raft_types!(
    /// Shared OpenRaft type configuration for Aspen.
    ///
    /// Kept in the leaf `aspen-raft-types` crate so consensus, transport, and
    /// router layers all import one source of truth without `unsafe` bridging.
    pub AppTypeConfig:
        D = AppRequest,
        R = AppResponse,
        NodeId = NodeId,
        Node = RaftMemberInfo,
);
