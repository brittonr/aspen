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
//! - **Node metadata** ([`RaftMemberInfo`]): Information about cluster members
//! - **Constants**: Network, storage, and timing constants
//!
//! # AppTypeConfig
//!
//! Note: `AppTypeConfig` is NOT defined here due to Rust's orphan rules. The type configuration
//! must be declared in the crate where openraft traits are implemented. See
//! `aspen-raft/src/types.rs` for the canonical declaration.
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
pub use constants::READ_INDEX_TIMEOUT;
pub use constants::SNAPSHOT_INSTALL_TIMEOUT_MS;
// Node metadata
pub use member::RaftMemberInfo;
// Network types
pub use network::ConnectionHealth;
pub use network::ConnectionStatus;
pub use network::DriftSeverity;
pub use network::FailureType;
pub use request::AppRequest;
pub use request::AppResponse;

// Note: AppTypeConfig is NOT defined here due to Rust's orphan rules.
// The type configuration must be declared in the crate where openraft traits
// are implemented. See aspen-raft/src/types.rs for the canonical declaration.
//
// aspen-transport has its own identical declaration to avoid circular dependencies
// (aspen-raft depends on aspen-transport). Code that bridges between them uses
// safe transmutes with documented SAFETY comments.
