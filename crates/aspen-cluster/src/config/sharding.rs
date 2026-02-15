//! Horizontal sharding configuration.
//!
//! Enables distributing data across multiple independent Raft clusters (shards)
//! for horizontal scaling.

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// Horizontal sharding configuration.
///
/// Enables distributing data across multiple independent Raft clusters (shards)
/// for horizontal scaling. Each shard handles a subset of the key space
/// determined by consistent hashing.
///
/// # Architecture
///
/// ```text
/// ShardedKeyValueStore
///     |
///     +-- ShardRouter (consistent hashing)
///     |
///     +-- shards[0] -> RaftNode (shard-0/ directory)
///     +-- shards[1] -> RaftNode (shard-1/ directory)
///     +-- shards[2] -> RaftNode (shard-2/ directory)
///     ...
/// ```
///
/// # TOML Example
///
/// ```toml
/// [sharding]
/// enabled = true
/// num_shards = 4
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ShardingConfig {
    /// Enable horizontal sharding.
    ///
    /// When enabled, the node will create multiple RaftNode instances (one per shard)
    /// and route operations using consistent hashing.
    ///
    /// Default: false (single-node mode).
    #[serde(default, rename = "enabled")]
    pub is_enabled: bool,

    /// Number of shards to create.
    ///
    /// Must be between 1 and 256 (MAX_SHARDS).
    /// Each shard gets its own storage directory and Raft consensus group.
    ///
    /// Default: 4 shards.
    #[serde(default = "default_num_shards")]
    pub num_shards: u32,

    /// List of shard IDs this node should host.
    ///
    /// If empty, the node hosts all shards (0..num_shards).
    /// This allows selective shard placement for multi-node deployments.
    ///
    /// Default: empty (host all shards).
    #[serde(default)]
    pub local_shards: Vec<u32>,
}

impl Default for ShardingConfig {
    fn default() -> Self {
        Self {
            is_enabled: false,
            num_shards: default_num_shards(),
            local_shards: vec![],
        }
    }
}

pub(crate) fn default_num_shards() -> u32 {
    aspen_sharding::DEFAULT_SHARDS
}
