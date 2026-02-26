//! Raft network layer for Aspen.
//!
//! This crate provides the network infrastructure for Raft consensus communication,
//! including connection pooling, failure detection, clock drift monitoring, and RPC
//! transport over Iroh QUIC.
//!
//! # Module Organization
//!
//! - [`rpc`]: RPC protocol definitions (requests, responses, wire format)
//! - [`types`]: OpenRaft type configuration (AppTypeConfig)
//! - [`verified`]: Pure, deterministic functions for network operations
//!
//! # Features
//!
//! - `testing`: Enable madsim deterministic simulation support
//! - `sharding`: Enable sharded Raft deployment support

pub mod rpc;
pub mod types;
pub mod verified;

// ============================================================================
// Re-exports: RPC Types
// ============================================================================
// ============================================================================
// Re-exports: Network Types from aspen-raft-types
// ============================================================================
pub use aspen_raft_types::network::ConnectionHealth;
pub use aspen_raft_types::network::ConnectionStatus;
pub use aspen_raft_types::network::DriftSeverity;
pub use aspen_raft_types::network::FailureType;
pub use rpc::RaftAppendEntriesRequest;
pub use rpc::RaftFatalErrorKind;
pub use rpc::RaftRpcMessage;
pub use rpc::RaftRpcProtocol;
pub use rpc::RaftRpcResponse;
pub use rpc::RaftRpcResponseWithTimestamps;
pub use rpc::RaftSnapshotRequest;
pub use rpc::RaftVoteRequest;
pub use rpc::ShardedRaftRpcRequest;
pub use rpc::ShardedRaftRpcResponse;
pub use rpc::TimestampInfo;
// ============================================================================
// Re-exports: Types
// ============================================================================
pub use types::AppTypeConfig;
// ============================================================================
// Re-exports: Verified Functions (for convenience)
// ============================================================================
pub use verified::SHARD_PREFIX_SIZE;
pub use verified::calculate_connection_retry_backoff;
pub use verified::calculate_ntp_clock_offset;
pub use verified::classify_drift_severity;
pub use verified::classify_node_failure;
pub use verified::classify_response_health;
pub use verified::classify_rpc_error;
pub use verified::compute_ewma;
pub use verified::decode_shard_prefix;
pub use verified::deserialize_rpc_response;
pub use verified::encode_shard_prefix;
pub use verified::extract_sharded_response;
pub use verified::maybe_prefix_shard_id;
pub use verified::should_evict_oldest_unreachable;
pub use verified::transition_connection_health;
pub use verified::try_decode_shard_prefix;
