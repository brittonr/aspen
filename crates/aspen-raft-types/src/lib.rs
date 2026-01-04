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
//! # Note on AppTypeConfig
//!
//! The OpenRaft type configuration (`AppTypeConfig`) is NOT included in this crate.
//! Due to Rust's orphan rules, `AppTypeConfig` must be declared in the main crate
//! where openraft traits are implemented. The component types (AppRequest, AppResponse,
//! NodeId, RaftMemberInfo) are exported here for reuse.
//!
//! Similarly, RPC types that depend on `AppTypeConfig` (like `RaftRpcProtocol`,
//! `RaftVoteRequest`, etc.) remain in the main crate since they use `AppTypeConfig`
//! in their type parameters.
//! # Tiger Style
//!
//! All types follow Tiger Style principles:
//! - Explicit bounds on all collections and operations
//! - Serializable for wire protocol compatibility
//! - Clear documentation of invariants and constraints

pub mod constants;
pub mod member;
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
pub use request::AppRequest;
pub use request::AppResponse;
