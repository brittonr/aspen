//! Verified pure functions for Raft network operations.
//!
//! This module contains pure functions extracted from the network layer for
//! improved testability. All functions are deterministic and side-effect free.
//!
//! # Module Organization
//!
//! - [`encoding`]: Binary encoding/decoding for shard prefixes
//! - [`network`]: RPC error classification, sharded message handling, response health
//! - [`heuristics`]: Connection health, clock drift, node failure detection

pub mod encoding;
pub mod heuristics;
pub mod network;

// ============================================================================
// Re-exports: Encoding
// ============================================================================
pub use encoding::SHARD_PREFIX_SIZE;
pub use encoding::decode_shard_prefix;
pub use encoding::encode_shard_prefix;
pub use encoding::try_decode_shard_prefix;
// ============================================================================
// Re-exports: Heuristics
// ============================================================================
pub use heuristics::calculate_connection_retry_backoff;
pub use heuristics::calculate_ntp_clock_offset;
pub use heuristics::classify_drift_severity;
pub use heuristics::classify_node_failure;
pub use heuristics::compute_ewma;
pub use heuristics::should_evict_oldest_unreachable;
pub use heuristics::transition_connection_health;
// ============================================================================
// Re-exports: Network
// ============================================================================
pub use network::classify_response_health;
pub use network::classify_rpc_error;
pub use network::deserialize_rpc_response;
pub use network::extract_sharded_response;
pub use network::maybe_prefix_shard_id;
