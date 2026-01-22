//! Horizontal sharding and consistent hash routing for Aspen distributed systems.
//!
//! This crate provides key-based sharding for distributing data across multiple
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
//! ShardedKeyValueStore.shards[2].write(request)
//!        ↓
//! Individual RaftNode handles the operation
//! ```
//!
//! # Components
//!
//! - [`ShardRouter`]: Routes keys to shards using Jump consistent hash
//! - [`ShardedKeyValueStore`]: Wraps multiple `KeyValueStore` implementations
//! - [`ShardTopology`]: Manages shard state and range assignments
//! - [`ShardMetricsCollector`]: Collects per-shard metrics for automation
//! - [`ShardAutomationManager`]: Background split/merge automation
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
//! use aspen_sharding::{ShardRouter, ShardConfig, ShardedKeyValueStore};
//! use aspen_core::KeyValueStore;
//!
//! // Create a router for 4 shards
//! let config = ShardConfig::new(4);
//! let router = ShardRouter::new(config.clone());
//!
//! // Route a key to its shard
//! let shard_id = router.get_shard_for_key("user:123");
//! assert!(shard_id < 4);
//!
//! // Create a sharded store wrapper
//! let store: ShardedKeyValueStore<MyKVStore> = ShardedKeyValueStore::new(config);
//! ```
//!
//! ## Federation Integration
//!
//! The sharding layer integrates with federation through the `FederationResourceResolver`
//! trait (defined in `aspen-cluster`). This abstraction allows federation to work
//! transparently whether the underlying storage is sharded or not.
//!
//! ```text
//! Federation Query
//!        |
//!        v
//! +------------------+
//! | ResourceResolver |  <- Trait abstraction
//! +------------------+
//!        |
//!   +----+----+
//!   |         |
//!   v         v
//! Direct   Sharded
//! (single  (routes to
//!  node)   appropriate
//!          shard)
//! ```
//!
//! ### Resolver Implementations
//!
//! - **`DirectResourceResolver`**: For non-sharded deployments. Routes all queries to the single
//!   `KeyValueStore` instance.
//!
//! - **`ShardedResourceResolver`**: For sharded deployments. Uses `ShardRouter` to determine which
//!   shard owns a key, then routes the query appropriately. Handles `ShardMoved` errors with
//!   automatic retry (up to 3 attempts).
//!
//! ### Design Principle
//!
//! Federation operates at the **cluster level**, not the shard level. Clients and
//! federated peers never see shards directly - the resolver abstracts this away.
//! This means:
//!
//! - Shard splits/merges are invisible to federation
//! - Federation sync continues working during topology changes
//! - Error handling is centralized in the resolver layer
//!
//! ### Node ID Encoding
//!
//! Shard-aware node IDs encode the shard number in the upper 16 bits:
//!
//! ```text
//! |<-- 16 bits -->|<------ 48 bits ------>|
//! |   shard_id    |     physical_node_id   |
//! ```
//!
//! - `encode_shard_node_id(shard_id, physical_id)` creates composite ID
//! - `decode_shard_node_id(node_id)` extracts (shard_id, physical_id)
//! - Shard 0 produces unchanged physical IDs (backwards compatible)

pub mod automation;
pub mod consistent_hash;
pub mod metrics;
pub mod router;
pub mod sharded;
pub mod topology;

pub use automation::AutomationConfig;
pub use automation::ShardAutomationManager;
pub use consistent_hash::JumpHash;
pub use metrics::METRICS_CHECK_INTERVAL;
pub use metrics::METRICS_WINDOW_DURATION;
pub use metrics::MetricsSnapshot;
pub use metrics::ShardMetricsAtomic;
pub use metrics::ShardMetricsCollector;
pub use router::ShardConfig;
pub use router::ShardId;
pub use router::ShardRange;
pub use router::ShardRouter;
pub use sharded::ShardedKeyValueStore;
pub use topology::DEFAULT_MERGE_MAX_COMBINED_BYTES;
pub use topology::DEFAULT_MERGE_SIZE_BYTES;
pub use topology::DEFAULT_SPLIT_QPS;
pub use topology::DEFAULT_SPLIT_SIZE_BYTES;
pub use topology::KeyRange;
pub use topology::SHARD_TOMBSTONE_GRACE_PERIOD_SECS;
pub use topology::ShardInfo;
pub use topology::ShardMetrics;
pub use topology::ShardState;
pub use topology::ShardTopology;
pub use topology::TopologyAnnouncement;
pub use topology::TopologyError;

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

/// Bits reserved for shard ID in the encoded NodeId.
/// Upper 16 bits for shard, lower 48 bits for physical node ID.
const SHARD_ID_BITS: u32 = 16;

/// Mask for extracting physical node ID from encoded NodeId.
const PHYSICAL_NODE_MASK: u64 = (1u64 << (64 - SHARD_ID_BITS)) - 1;

/// Encode a physical node ID and shard ID into a combined NodeId.
///
/// The shard ID is stored in the upper 16 bits, allowing up to 65535 shards
/// (though MAX_SHARDS limits this to 256). The physical node ID uses the
/// lower 48 bits, supporting up to 281 trillion physical nodes.
///
/// # Example
///
/// ```
/// use aspen_sharding::{encode_shard_node_id, decode_shard_node_id};
///
/// let node_id = encode_shard_node_id(42, 3);
/// let (physical, shard) = decode_shard_node_id(node_id);
/// assert_eq!(physical, 42);
/// assert_eq!(shard, 3);
/// ```
#[inline]
pub fn encode_shard_node_id(physical_node_id: u64, shard_id: ShardId) -> u64 {
    debug_assert!(physical_node_id <= PHYSICAL_NODE_MASK, "Physical node ID exceeds 48-bit limit");
    debug_assert!(shard_id <= MAX_SHARDS, "Shard ID exceeds MAX_SHARDS limit");
    physical_node_id | ((shard_id as u64) << (64 - SHARD_ID_BITS))
}

/// Decode an encoded NodeId into its physical node ID and shard ID components.
///
/// # Example
///
/// ```
/// use aspen_sharding::{encode_shard_node_id, decode_shard_node_id};
///
/// let node_id = encode_shard_node_id(100, 5);
/// let (physical, shard) = decode_shard_node_id(node_id);
/// assert_eq!(physical, 100);
/// assert_eq!(shard, 5);
/// ```
#[inline]
pub fn decode_shard_node_id(encoded_node_id: u64) -> (u64, ShardId) {
    let physical_node_id = encoded_node_id & PHYSICAL_NODE_MASK;
    let shard_id = (encoded_node_id >> (64 - SHARD_ID_BITS)) as ShardId;
    (physical_node_id, shard_id)
}

/// Check if an encoded NodeId belongs to a specific shard.
#[inline]
pub fn is_shard_node(encoded_node_id: u64, shard_id: ShardId) -> bool {
    let (_, node_shard) = decode_shard_node_id(encoded_node_id);
    node_shard == shard_id
}

/// Extract just the shard ID from an encoded NodeId.
#[inline]
pub fn get_shard_from_node_id(encoded_node_id: u64) -> ShardId {
    (encoded_node_id >> (64 - SHARD_ID_BITS)) as ShardId
}

/// Extract just the physical node ID from an encoded NodeId.
#[inline]
pub fn get_physical_node_id(encoded_node_id: u64) -> u64 {
    encoded_node_id & PHYSICAL_NODE_MASK
}

/// Storage path configuration for a single shard.
///
/// Contains paths for both Raft log storage (redb) and state machine
/// storage. Each shard gets isolated storage to enable:
/// - Independent growth management per shard
/// - Simpler shard migration/rebalancing
/// - Better fault isolation
#[derive(Debug, Clone)]
pub struct ShardStoragePaths {
    /// Shard identifier.
    pub shard_id: ShardId,
    /// Directory containing all shard data.
    pub shard_dir: std::path::PathBuf,
    /// Path for Raft log database (redb).
    pub log_path: std::path::PathBuf,
    /// Path for state machine database.
    pub state_machine_path: std::path::PathBuf,
}

impl ShardStoragePaths {
    /// Create storage paths for a shard.
    ///
    /// Directory structure:
    /// ```text
    /// data_dir/
    ///   shard-{shard_id}/
    ///     raft-log.db
    ///     state-machine.db
    /// ```
    ///
    /// # Arguments
    ///
    /// * `data_dir` - Base data directory for the node
    /// * `shard_id` - Shard identifier
    ///
    /// # Example
    ///
    /// ```
    /// use aspen_sharding::ShardStoragePaths;
    /// use std::path::PathBuf;
    ///
    /// let paths = ShardStoragePaths::new("/data/node-1", 3);
    /// assert_eq!(paths.shard_dir, PathBuf::from("/data/node-1/shard-3"));
    /// assert_eq!(paths.log_path, PathBuf::from("/data/node-1/shard-3/raft-log.db"));
    /// ```
    pub fn new(data_dir: impl AsRef<std::path::Path>, shard_id: ShardId) -> Self {
        let shard_dir = data_dir.as_ref().join(format!("shard-{}", shard_id));
        Self {
            shard_id,
            log_path: shard_dir.join("raft-log.db"),
            state_machine_path: shard_dir.join("state-machine.db"),
            shard_dir,
        }
    }

    /// Ensure the shard directory exists.
    ///
    /// Creates the directory and all parent directories if they don't exist.
    ///
    /// # Errors
    ///
    /// Returns an error if directory creation fails.
    pub fn ensure_dir_exists(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.shard_dir)
    }
}

/// Generate storage paths for all shards in a cluster.
///
/// # Arguments
///
/// * `data_dir` - Base data directory for the node
/// * `num_shards` - Number of shards to generate paths for
///
/// # Returns
///
/// Vector of `ShardStoragePaths` for shards 0..num_shards
pub fn generate_shard_storage_paths(data_dir: impl AsRef<std::path::Path>, num_shards: u32) -> Vec<ShardStoragePaths> {
    (0..num_shards).map(|shard_id| ShardStoragePaths::new(data_dir.as_ref(), shard_id)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        for physical in [0, 1, 100, 1000, PHYSICAL_NODE_MASK] {
            for shard in [0, 1, 4, 255] {
                let encoded = encode_shard_node_id(physical, shard);
                let (decoded_physical, decoded_shard) = decode_shard_node_id(encoded);
                assert_eq!(decoded_physical, physical, "Physical node mismatch");
                assert_eq!(decoded_shard, shard, "Shard ID mismatch");
            }
        }
    }

    #[test]
    fn test_is_shard_node() {
        let node = encode_shard_node_id(42, 5);
        assert!(is_shard_node(node, 5));
        assert!(!is_shard_node(node, 4));
        assert!(!is_shard_node(node, 6));
    }

    #[test]
    fn test_get_shard_from_node_id() {
        let node = encode_shard_node_id(999, 7);
        assert_eq!(get_shard_from_node_id(node), 7);
    }

    #[test]
    fn test_get_physical_node_id() {
        let node = encode_shard_node_id(12345, 3);
        assert_eq!(get_physical_node_id(node), 12345);
    }

    #[test]
    fn test_shard_zero_is_backward_compatible() {
        // When shard_id=0, the encoded ID should equal the physical ID
        // This ensures backward compatibility with non-sharded clusters
        let physical = 42u64;
        let encoded = encode_shard_node_id(physical, 0);
        assert_eq!(encoded, physical);
    }

    #[test]
    fn test_shard_storage_paths_new() {
        let paths = ShardStoragePaths::new("/data/node-1", 3);
        assert_eq!(paths.shard_id, 3);
        assert_eq!(paths.shard_dir, std::path::PathBuf::from("/data/node-1/shard-3"));
        assert_eq!(paths.log_path, std::path::PathBuf::from("/data/node-1/shard-3/raft-log.db"));
        assert_eq!(paths.state_machine_path, std::path::PathBuf::from("/data/node-1/shard-3/state-machine.db"));
    }

    #[test]
    fn test_shard_storage_paths_shard_zero() {
        let paths = ShardStoragePaths::new("/data/node-1", 0);
        assert_eq!(paths.shard_id, 0);
        assert_eq!(paths.shard_dir, std::path::PathBuf::from("/data/node-1/shard-0"));
    }

    #[test]
    fn test_generate_shard_storage_paths() {
        let paths = generate_shard_storage_paths("/data/node-1", 4);
        assert_eq!(paths.len(), 4);

        for (i, p) in paths.iter().enumerate() {
            assert_eq!(p.shard_id, i as u32);
            assert!(p.shard_dir.to_string_lossy().contains(&format!("shard-{}", i)));
            assert!(p.log_path.to_string_lossy().ends_with("raft-log.db"));
            assert!(p.state_machine_path.to_string_lossy().ends_with("state-machine.db"));
        }
    }

    #[test]
    fn test_generate_shard_storage_paths_empty() {
        let paths = generate_shard_storage_paths("/data/node-1", 0);
        assert!(paths.is_empty());
    }

    #[test]
    fn test_shard_storage_paths_relative_path() {
        let paths = ShardStoragePaths::new("./data", 2);
        assert_eq!(paths.shard_dir, std::path::PathBuf::from("./data/shard-2"));
    }
}
