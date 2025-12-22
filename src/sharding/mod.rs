//! Sharding module for horizontal scaling.
//!
//! This module provides key-based sharding for distributing data across multiple
//! Raft clusters. Each shard is an independent Raft cluster that owns a subset
//! of the key space.
//!
//! # Architecture
//!
//! ```text
//! Client Request (key: "user:123")
//!        ↓
//! ShardRouter.get_shard_for_key("user:123")
//!        ↓
//! Returns ShardId = 2 (via consistent hashing)
//!        ↓
//! ShardedRaftNode.shards[2].write(request)
//!        ↓
//! Individual RaftNode handles the operation
//! ```
//!
//! # Tiger Style
//!
//! - Fixed limits: MAX_SHARDS = 256 to prevent unbounded growth
//! - Consistent hashing: Jump hash for uniform distribution
//! - Explicit error types with actionable context
//!
//! # Example
//!
//! ```ignore
//! use aspen::sharding::{ShardRouter, ShardConfig};
//!
//! let config = ShardConfig::new(4); // 4 shards
//! let router = ShardRouter::new(config);
//!
//! let shard_id = router.get_shard_for_key("user:123");
//! assert!(shard_id < 4);
//! ```

pub mod consistent_hash;
pub mod router;
pub mod sharded;

pub use consistent_hash::JumpHash;
pub use router::{ShardConfig, ShardId, ShardRange, ShardRouter};
pub use sharded::ShardedKeyValueStore;

/// Maximum number of shards supported.
///
/// Tiger Style: Fixed limit prevents unbounded shard growth.
/// 256 shards provides 256x scaling capacity which is sufficient
/// for most use cases while keeping metadata manageable.
pub const MAX_SHARDS: u32 = 256;

/// Minimum number of shards.
///
/// At least 1 shard is required for the system to function.
pub const MIN_SHARDS: u32 = 1;

/// Default number of shards for new clusters.
///
/// Starting with 4 shards provides a good balance between
/// initial overhead and future scaling headroom.
pub const DEFAULT_SHARDS: u32 = 4;
