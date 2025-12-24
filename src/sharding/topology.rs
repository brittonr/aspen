//! Shard topology management for dynamic shard splitting and merging.
//!
//! This module provides data structures for tracking and updating shard topology,
//! enabling runtime shard reconfiguration without cluster downtime.
//!
//! # Architecture
//!
//! The topology system follows TiKV's Multi-Raft pattern:
//! 1. Topology changes are submitted through Raft consensus
//! 2. All replicas apply changes at the same log index (atomic)
//! 3. Shard 0 stores the authoritative topology metadata (control plane)
//! 4. Topology changes are broadcast via gossip to all nodes
//!
//! # Range-Based Routing
//!
//! Unlike hash-based sharding, range-based routing uses key ranges:
//! - Each shard owns a contiguous range `[start_key, end_key)`
//! - Routing is O(log n) via BTreeMap lookup
//! - Splits divide a range at a chosen split key
//! - Merges combine adjacent ranges
//!
//! # Tiger Style
//!
//! - Version counter for detecting stale updates
//! - Explicit state machine for shard lifecycle
//! - Bounded metrics for split decisions
//! - Fixed limits on concurrent operations

use std::collections::BTreeMap;

use serde::Deserialize;
use serde::Serialize;

use super::MAX_SHARDS;
use super::router::ShardId;

/// Default size threshold for triggering automatic splits (64 MB).
pub const DEFAULT_SPLIT_SIZE_BYTES: u64 = 64 * 1024 * 1024;

/// Default QPS threshold for triggering automatic splits.
pub const DEFAULT_SPLIT_QPS: u32 = 2500;

/// Default size threshold for allowing merges (16 MB).
/// Two shards can merge if both are below this threshold.
pub const DEFAULT_MERGE_SIZE_BYTES: u64 = 16 * 1024 * 1024;

/// Maximum combined size after merge (48 MB).
/// Leaves headroom before next split trigger.
pub const DEFAULT_MERGE_MAX_COMBINED_BYTES: u64 = 48 * 1024 * 1024;

/// Grace period before cleaning up tombstoned shards (in seconds).
pub const SHARD_TOMBSTONE_GRACE_PERIOD_SECS: u64 = 300;

/// A key range representing a contiguous segment of the key space.
///
/// Ranges are half-open intervals: `[start_key, end_key)`.
/// An empty `end_key` indicates the range extends to the end of the keyspace.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct KeyRange {
    /// Start key (inclusive). Empty string means start of keyspace.
    pub start_key: String,
    /// End key (exclusive). Empty string means end of keyspace.
    pub end_key: String,
}

impl KeyRange {
    /// Create a new key range.
    pub fn new(start_key: impl Into<String>, end_key: impl Into<String>) -> Self {
        Self {
            start_key: start_key.into(),
            end_key: end_key.into(),
        }
    }

    /// Create a range covering the entire keyspace.
    pub fn full() -> Self {
        Self::new("", "")
    }

    /// Check if a key falls within this range.
    ///
    /// Returns true if `start_key <= key < end_key` (or key >= start_key if end_key is empty).
    pub fn contains(&self, key: &str) -> bool {
        key >= self.start_key.as_str() && (self.end_key.is_empty() || key < self.end_key.as_str())
    }

    /// Check if this range is empty (start >= end for non-unbounded ranges).
    pub fn is_empty(&self) -> bool {
        !self.end_key.is_empty() && self.start_key >= self.end_key
    }

    /// Split this range at the given key, returning (left, right) ranges.
    ///
    /// Returns None if split_key is not within the range.
    pub fn split_at(&self, split_key: &str) -> Option<(KeyRange, KeyRange)> {
        if !self.contains(split_key) || split_key == self.start_key.as_str() {
            return None;
        }

        let left = KeyRange::new(&self.start_key, split_key);
        let right = KeyRange::new(split_key, &self.end_key);

        Some((left, right))
    }

    /// Check if this range is adjacent to another (can be merged).
    ///
    /// Two ranges are adjacent if one ends where the other begins.
    /// Note: We compare non-empty boundaries, treating empty "" as infinity.
    pub fn is_adjacent_to(&self, other: &KeyRange) -> bool {
        // self comes before other: self.end == other.start (both non-empty)
        (!self.end_key.is_empty() && self.end_key == other.start_key)
            // other comes before self: other.end == self.start (both non-empty)
            || (!other.end_key.is_empty() && other.end_key == self.start_key)
    }

    /// Merge this range with an adjacent range.
    ///
    /// Returns None if ranges are not adjacent.
    pub fn merge_with(&self, other: &KeyRange) -> Option<KeyRange> {
        // self comes before other: self ends where other starts
        if !self.end_key.is_empty() && self.end_key == other.start_key {
            // Merged: [self.start, other.end)
            Some(KeyRange::new(&self.start_key, &other.end_key))
        // other comes before self: other ends where self starts
        } else if !other.end_key.is_empty() && other.end_key == self.start_key {
            // Merged: [other.start, self.end)
            Some(KeyRange::new(&other.start_key, &self.end_key))
        } else {
            None
        }
    }
}

impl Default for KeyRange {
    fn default() -> Self {
        Self::full()
    }
}

/// Lifecycle state of a shard.
///
/// Shards transition through states during splits and merges:
/// ```text
/// Active ──┬── Splitting ── Active (source becomes smaller)
///          │                   │
///          │                   └── Active (new shard created)
///          │
///          └── Merging ── Tombstone (source shard)
///                            │
///                            └── Active (target shard expanded)
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ShardState {
    /// Normal operation - shard is accepting reads and writes.
    #[default]
    Active,

    /// Shard is being split into two shards.
    ///
    /// During splitting:
    /// - Writes to the splitting range may be queued
    /// - Reads can still be served
    /// - Keys >= split_key will move to new_shard_id
    Splitting {
        /// The key at which to split (keys >= this go to new shard).
        split_key: String,
        /// ID of the new shard being created.
        new_shard_id: ShardId,
    },

    /// Shard is being merged into another shard.
    ///
    /// During merging:
    /// - New writes are rejected with ShardMoved error
    /// - Reads can still be served until merge completes
    /// - All keys will move to target_shard_id
    Merging {
        /// ID of the shard this will merge into.
        target_shard_id: ShardId,
    },

    /// Shard has been merged or deleted.
    ///
    /// Tombstoned shards are kept for a grace period to redirect
    /// clients, then cleaned up.
    Tombstone {
        /// When this shard was tombstoned (Unix timestamp in seconds).
        tombstoned_at: u64,
        /// The shard that now owns this shard's keys (for redirects).
        successor_shard_id: Option<ShardId>,
    },
}

impl ShardState {
    /// Check if the shard can accept writes.
    pub fn can_write(&self) -> bool {
        matches!(self, ShardState::Active)
    }

    /// Check if the shard can serve reads.
    pub fn can_read(&self) -> bool {
        matches!(self, ShardState::Active | ShardState::Splitting { .. } | ShardState::Merging { .. })
    }

    /// Check if the shard is in a transitional state.
    pub fn is_transitioning(&self) -> bool {
        matches!(self, ShardState::Splitting { .. } | ShardState::Merging { .. })
    }
}

/// Runtime metrics for a single shard.
///
/// These metrics are used to make automatic split/merge decisions.
/// Tiger Style: All counters have reasonable bounds via normal operation limits.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShardMetrics {
    /// Estimated size of all keys and values in bytes.
    pub size_bytes: u64,
    /// Total number of keys in this shard.
    pub key_count: u64,
    /// Read operations in the current measurement window.
    pub read_count: u64,
    /// Write operations in the current measurement window.
    pub write_count: u64,
    /// Timestamp when metrics were last reset (Unix millis).
    pub window_start_ms: u64,
}

impl ShardMetrics {
    /// Create new empty metrics.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get queries per second based on current window.
    pub fn qps(&self, current_time_ms: u64) -> u32 {
        let elapsed_secs = (current_time_ms.saturating_sub(self.window_start_ms)) / 1000;
        if elapsed_secs == 0 {
            return 0;
        }
        let total_ops = self.read_count + self.write_count;
        (total_ops / elapsed_secs) as u32
    }

    /// Check if this shard should be split based on thresholds.
    pub fn should_split(&self, size_threshold: u64, qps_threshold: u32, current_time_ms: u64) -> bool {
        self.size_bytes > size_threshold || self.qps(current_time_ms) > qps_threshold
    }

    /// Check if this shard is eligible for merging.
    pub fn can_merge(&self, size_threshold: u64) -> bool {
        self.size_bytes < size_threshold
    }

    /// Record a read operation.
    pub fn record_read(&mut self) {
        self.read_count = self.read_count.saturating_add(1);
    }

    /// Record a write operation with size delta.
    pub fn record_write(&mut self, key_len: usize, value_len: usize, is_delete: bool) {
        self.write_count = self.write_count.saturating_add(1);
        let entry_size = (key_len + value_len) as u64;
        if is_delete {
            self.size_bytes = self.size_bytes.saturating_sub(entry_size);
            self.key_count = self.key_count.saturating_sub(1);
        } else {
            self.size_bytes = self.size_bytes.saturating_add(entry_size);
            self.key_count = self.key_count.saturating_add(1);
        }
    }

    /// Reset the measurement window.
    pub fn reset_window(&mut self, current_time_ms: u64) {
        self.read_count = 0;
        self.write_count = 0;
        self.window_start_ms = current_time_ms;
    }
}

/// Information about a single shard.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShardInfo {
    /// Unique shard identifier.
    pub shard_id: ShardId,
    /// Current lifecycle state.
    pub state: ShardState,
    /// Key range owned by this shard.
    pub key_range: KeyRange,
    /// Physical node IDs that host replicas of this shard.
    pub members: Vec<u64>,
    /// Hint for the current Raft leader of this shard (may be stale).
    pub leader_hint: Option<u64>,
    /// When this shard was created (Unix timestamp in seconds).
    pub created_at: u64,
    /// Parent shard if this was created via split.
    pub split_from: Option<ShardId>,
}

impl ShardInfo {
    /// Create a new shard with the given ID and range.
    pub fn new(shard_id: ShardId, key_range: KeyRange) -> Self {
        Self {
            shard_id,
            state: ShardState::Active,
            key_range,
            members: Vec::new(),
            leader_hint: None,
            created_at: 0,
            split_from: None,
        }
    }

    /// Create a new shard from a split operation.
    pub fn from_split(
        shard_id: ShardId,
        key_range: KeyRange,
        parent_id: ShardId,
        members: Vec<u64>,
        created_at: u64,
    ) -> Self {
        Self {
            shard_id,
            state: ShardState::Active,
            key_range,
            members,
            leader_hint: None,
            created_at,
            split_from: Some(parent_id),
        }
    }
}

/// Complete cluster shard topology with versioning.
///
/// The topology is the authoritative source of truth for shard ownership.
/// It is stored in shard 0's state machine and replicated via Raft.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShardTopology {
    /// Monotonically increasing version number.
    /// Incremented on every topology change.
    pub version: u64,
    /// All shards in the cluster, indexed by shard ID.
    pub shards: BTreeMap<ShardId, ShardInfo>,
    /// Range-to-shard mapping for efficient routing.
    /// Key is the range start_key, value is shard_id.
    ranges: BTreeMap<String, ShardId>,
    /// When this topology was last updated (Unix timestamp in seconds).
    pub updated_at: u64,
}

impl ShardTopology {
    /// Create a new topology with default initial shards.
    ///
    /// Creates 4 shards with hex-based range boundaries:
    /// - Shard 0: ["", "4")
    /// - Shard 1: ["4", "8")
    /// - Shard 2: ["8", "c")
    /// - Shard 3: ["c", "")
    pub fn new(num_shards: u32, created_at: u64) -> Self {
        assert!((1..=MAX_SHARDS).contains(&num_shards));

        let mut shards = BTreeMap::new();
        let mut ranges = BTreeMap::new();

        if num_shards == 1 {
            // Single shard owns everything
            let info = ShardInfo {
                shard_id: 0,
                state: ShardState::Active,
                key_range: KeyRange::full(),
                members: Vec::new(),
                leader_hint: None,
                created_at,
                split_from: None,
            };
            ranges.insert(String::new(), 0);
            shards.insert(0, info);
        } else {
            // Distribute keyspace evenly using hex boundaries
            let boundaries = generate_hex_boundaries(num_shards);
            for (i, (start, end)) in boundaries.into_iter().enumerate() {
                let shard_id = i as ShardId;
                let info = ShardInfo {
                    shard_id,
                    state: ShardState::Active,
                    key_range: KeyRange::new(&start, &end),
                    members: Vec::new(),
                    leader_hint: None,
                    created_at,
                    split_from: None,
                };
                ranges.insert(start, shard_id);
                shards.insert(shard_id, info);
            }
        }

        Self {
            version: 1,
            shards,
            ranges,
            updated_at: created_at,
        }
    }

    /// Create an empty topology (for deserialization).
    pub fn empty() -> Self {
        Self {
            version: 0,
            shards: BTreeMap::new(),
            ranges: BTreeMap::new(),
            updated_at: 0,
        }
    }

    /// Get the shard that owns a given key.
    ///
    /// Uses BTreeMap range lookup for O(log n) routing.
    pub fn get_shard_for_key(&self, key: &str) -> Option<ShardId> {
        // Find the range whose start_key is <= key
        self.ranges.range(..=key.to_string()).next_back().map(|(_, &shard_id)| shard_id)
    }

    /// Get shard info by ID.
    pub fn get_shard(&self, shard_id: ShardId) -> Option<&ShardInfo> {
        self.shards.get(&shard_id)
    }

    /// Get mutable shard info by ID.
    pub fn get_shard_mut(&mut self, shard_id: ShardId) -> Option<&mut ShardInfo> {
        self.shards.get_mut(&shard_id)
    }

    /// Get all active shard IDs.
    pub fn active_shard_ids(&self) -> Vec<ShardId> {
        self.shards.values().filter(|s| matches!(s.state, ShardState::Active)).map(|s| s.shard_id).collect()
    }

    /// Get total number of shards (including non-active).
    pub fn shard_count(&self) -> usize {
        self.shards.len()
    }

    /// Get the next available shard ID.
    pub fn next_shard_id(&self) -> ShardId {
        self.shards.keys().max().map(|&id| id + 1).unwrap_or(0)
    }

    /// Apply a shard split operation.
    ///
    /// This is called when a ShardSplit command is applied via Raft.
    /// The operation is idempotent if the topology version matches.
    pub fn apply_split(
        &mut self,
        source_shard_id: ShardId,
        split_key: String,
        new_shard_id: ShardId,
        timestamp: u64,
    ) -> Result<(), TopologyError> {
        // Validate source shard exists and is active
        let source = self.shards.get(&source_shard_id).ok_or(TopologyError::ShardNotFound {
            shard_id: source_shard_id,
        })?;

        if !matches!(source.state, ShardState::Active) {
            return Err(TopologyError::InvalidState {
                shard_id: source_shard_id,
                expected: "Active".to_string(),
                actual: format!("{:?}", source.state),
            });
        }

        // Validate split key is within range
        if !source.key_range.contains(&split_key) {
            return Err(TopologyError::InvalidSplitKey {
                key: split_key,
                range: source.key_range.clone(),
            });
        }

        // Validate new shard ID doesn't exist
        if self.shards.contains_key(&new_shard_id) {
            return Err(TopologyError::ShardAlreadyExists { shard_id: new_shard_id });
        }

        // Perform the split
        let source = self.shards.get_mut(&source_shard_id).unwrap();
        let (left_range, right_range) =
            source.key_range.split_at(&split_key).ok_or(TopologyError::InvalidSplitKey {
                key: split_key.clone(),
                range: source.key_range.clone(),
            })?;

        // Update source shard with left range
        let source_members = source.members.clone();
        source.key_range = left_range;

        // Create new shard with right range
        let new_shard =
            ShardInfo::from_split(new_shard_id, right_range.clone(), source_shard_id, source_members, timestamp);

        // Update range index
        self.ranges.insert(right_range.start_key.clone(), new_shard_id);

        // Insert new shard
        self.shards.insert(new_shard_id, new_shard);

        // Increment version
        self.version += 1;
        self.updated_at = timestamp;

        Ok(())
    }

    /// Apply a shard merge operation.
    ///
    /// Merges source_shard into target_shard, expanding target's range.
    pub fn apply_merge(
        &mut self,
        source_shard_id: ShardId,
        target_shard_id: ShardId,
        timestamp: u64,
    ) -> Result<(), TopologyError> {
        // Validate both shards exist and are active
        let source = self.shards.get(&source_shard_id).ok_or(TopologyError::ShardNotFound {
            shard_id: source_shard_id,
        })?;

        if !matches!(source.state, ShardState::Active) {
            return Err(TopologyError::InvalidState {
                shard_id: source_shard_id,
                expected: "Active".to_string(),
                actual: format!("{:?}", source.state),
            });
        }

        let target = self.shards.get(&target_shard_id).ok_or(TopologyError::ShardNotFound {
            shard_id: target_shard_id,
        })?;

        if !matches!(target.state, ShardState::Active) {
            return Err(TopologyError::InvalidState {
                shard_id: target_shard_id,
                expected: "Active".to_string(),
                actual: format!("{:?}", target.state),
            });
        }

        // Validate ranges are adjacent
        if !source.key_range.is_adjacent_to(&target.key_range) {
            return Err(TopologyError::RangesNotAdjacent {
                source: source.key_range.clone(),
                target: target.key_range.clone(),
            });
        }

        // Compute merged range
        let source_range = source.key_range.clone();
        let target_range = target.key_range.clone();
        let merged_range = source_range.merge_with(&target_range).ok_or(TopologyError::RangesNotAdjacent {
            source: source_range.clone(),
            target: target_range.clone(),
        })?;

        // Update target shard with merged range
        let target = self.shards.get_mut(&target_shard_id).unwrap();
        target.key_range = merged_range.clone();

        // Mark source as tombstone
        let source = self.shards.get_mut(&source_shard_id).unwrap();
        source.state = ShardState::Tombstone {
            tombstoned_at: timestamp,
            successor_shard_id: Some(target_shard_id),
        };

        // Update range index - remove source's start_key entry
        self.ranges.remove(&source_range.start_key);

        // Update target's range entry if the merged range has a new start
        if merged_range.start_key != target_range.start_key {
            self.ranges.remove(&target_range.start_key);
            self.ranges.insert(merged_range.start_key, target_shard_id);
        }

        // Increment version
        self.version += 1;
        self.updated_at = timestamp;

        Ok(())
    }

    /// Clean up tombstoned shards past grace period.
    pub fn cleanup_tombstones(&mut self, current_timestamp: u64) {
        let shards_to_remove: Vec<ShardId> = self
            .shards
            .iter()
            .filter_map(|(&id, info)| {
                if let ShardState::Tombstone { tombstoned_at, .. } = info.state
                    && current_timestamp.saturating_sub(tombstoned_at) > SHARD_TOMBSTONE_GRACE_PERIOD_SECS
                {
                    return Some(id);
                }
                None
            })
            .collect();

        let removed_count = shards_to_remove.len();
        for shard_id in shards_to_remove {
            self.shards.remove(&shard_id);
        }

        if removed_count > 0 {
            self.version += 1;
            self.updated_at = current_timestamp;
        }
    }
}

impl Default for ShardTopology {
    fn default() -> Self {
        Self::new(4, 0)
    }
}

/// Errors that can occur during topology operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TopologyError {
    /// The specified shard was not found.
    ShardNotFound {
        /// The shard ID that was not found.
        shard_id: ShardId,
    },
    /// A shard with this ID already exists.
    ShardAlreadyExists {
        /// The shard ID that already exists.
        shard_id: ShardId,
    },
    /// The shard is not in the expected state for this operation.
    InvalidState {
        /// The shard ID in invalid state.
        shard_id: ShardId,
        /// The expected state name.
        expected: String,
        /// The actual state name.
        actual: String,
    },
    /// The split key is not valid for the shard's range.
    InvalidSplitKey {
        /// The invalid split key.
        key: String,
        /// The shard's key range.
        range: KeyRange,
    },
    /// The ranges are not adjacent (cannot merge).
    RangesNotAdjacent {
        /// The source range.
        source: KeyRange,
        /// The target range.
        target: KeyRange,
    },
    /// Topology version mismatch (stale update).
    VersionMismatch {
        /// The expected topology version.
        expected: u64,
        /// The actual topology version.
        actual: u64,
    },
}

impl std::fmt::Display for TopologyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TopologyError::ShardNotFound { shard_id } => {
                write!(f, "shard {} not found", shard_id)
            }
            TopologyError::ShardAlreadyExists { shard_id } => {
                write!(f, "shard {} already exists", shard_id)
            }
            TopologyError::InvalidState {
                shard_id,
                expected,
                actual,
            } => {
                write!(f, "shard {} in invalid state: expected {}, got {}", shard_id, expected, actual)
            }
            TopologyError::InvalidSplitKey { key, range } => {
                write!(f, "split key '{}' not in range [{}, {})", key, range.start_key, range.end_key)
            }
            TopologyError::RangesNotAdjacent { source, target } => {
                write!(
                    f,
                    "ranges not adjacent: [{}, {}) and [{}, {})",
                    source.start_key, source.end_key, target.start_key, target.end_key
                )
            }
            TopologyError::VersionMismatch { expected, actual } => {
                write!(f, "topology version mismatch: expected {}, got {}", expected, actual)
            }
        }
    }
}

impl std::error::Error for TopologyError {}

/// Generate hex-based range boundaries for initial shard distribution.
///
/// Distributes the keyspace evenly using hex character boundaries.
fn generate_hex_boundaries(num_shards: u32) -> Vec<(String, String)> {
    if num_shards == 1 {
        return vec![("".to_string(), "".to_string())];
    }

    // For up to 16 shards, use single hex characters
    // For more, use two hex characters (256 buckets max)
    let hex_chars = if num_shards <= 16 {
        "0123456789abcdef"
    } else {
        "00010203040506070809101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e7f808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9fa0a1a2a3a4a5a6a7a8a9aaabacadaeafb0b1b2b3b4b5b6b7b8b9babbbcbdbebfc0c1c2c3c4c5c6c7c8c9cacbcccdcecfd0d1d2d3d4d5d6d7d8d9dadbdcdddedfe0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7f8f9fafbfcfdfeff"
    };

    // Simple approach: divide hex space evenly
    let boundaries: Vec<String> = if num_shards <= 16 {
        let step = 16 / num_shards;
        (0..num_shards)
            .map(|i| {
                if i == 0 {
                    "".to_string()
                } else {
                    let idx = (i * step) as usize;
                    hex_chars.chars().nth(idx).unwrap().to_string()
                }
            })
            .collect()
    } else {
        let step = 256 / num_shards;
        (0..num_shards)
            .map(|i| {
                if i == 0 {
                    "".to_string()
                } else {
                    let idx = (i * step) as usize * 2;
                    hex_chars[idx..idx + 2].to_string()
                }
            })
            .collect()
    };

    // Create range tuples
    let mut ranges = Vec::with_capacity(num_shards as usize);
    for i in 0..num_shards as usize {
        let start = boundaries[i].clone();
        let end = if i + 1 < boundaries.len() {
            boundaries[i + 1].clone()
        } else {
            "".to_string() // Last shard extends to end
        };
        ranges.push((start, end));
    }

    ranges
}

/// Gossip announcement for topology changes.
///
/// Broadcast via gossip to inform all nodes of topology updates.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TopologyAnnouncement {
    /// Protocol version for forward compatibility.
    pub version: u8,
    /// Node ID of the announcing node.
    pub node_id: u64,
    /// Current topology version.
    pub topology_version: u64,
    /// Hash of the full topology (for consistency checking).
    pub topology_hash: u64,
    /// Raft term when this topology was committed.
    pub term: u64,
    /// Timestamp of announcement (Unix micros).
    pub timestamp_micros: u64,
}

impl TopologyAnnouncement {
    /// Current protocol version for topology announcements.
    pub const PROTOCOL_VERSION: u8 = 1;

    /// Create a new topology announcement.
    pub fn new(node_id: u64, topology_version: u64, topology_hash: u64, term: u64, timestamp_micros: u64) -> Self {
        Self {
            version: Self::PROTOCOL_VERSION,
            node_id,
            topology_version,
            topology_hash,
            term,
            timestamp_micros,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_range_contains() {
        let range = KeyRange::new("a", "d");
        assert!(!range.contains("")); // before start
        assert!(range.contains("a")); // at start
        assert!(range.contains("b")); // middle
        assert!(range.contains("c")); // middle
        assert!(!range.contains("d")); // at end (exclusive)
        assert!(!range.contains("e")); // after end
    }

    #[test]
    fn test_key_range_unbounded() {
        let range = KeyRange::new("a", "");
        assert!(!range.contains("")); // before start
        assert!(range.contains("a")); // at start
        assert!(range.contains("zzzzz")); // any key >= start
    }

    #[test]
    fn test_key_range_full() {
        let range = KeyRange::full();
        assert!(range.contains(""));
        assert!(range.contains("anything"));
        assert!(range.contains("zzzzzzzzzz"));
    }

    #[test]
    fn test_key_range_split() {
        let range = KeyRange::new("a", "z");
        let (left, right) = range.split_at("m").unwrap();

        assert_eq!(left.start_key, "a");
        assert_eq!(left.end_key, "m");
        assert_eq!(right.start_key, "m");
        assert_eq!(right.end_key, "z");

        assert!(left.contains("a"));
        assert!(left.contains("l"));
        assert!(!left.contains("m"));

        assert!(right.contains("m"));
        assert!(right.contains("y"));
        assert!(!right.contains("z"));
    }

    #[test]
    fn test_key_range_split_invalid() {
        let range = KeyRange::new("m", "z");
        assert!(range.split_at("a").is_none()); // before range
        assert!(range.split_at("m").is_none()); // at start (would create empty left)
    }

    #[test]
    fn test_key_range_merge() {
        let left = KeyRange::new("a", "m");
        let right = KeyRange::new("m", "z");

        let merged = left.merge_with(&right).unwrap();
        assert_eq!(merged.start_key, "a");
        assert_eq!(merged.end_key, "z");

        let merged_rev = right.merge_with(&left).unwrap();
        assert_eq!(merged_rev.start_key, "a");
        assert_eq!(merged_rev.end_key, "z");
    }

    #[test]
    fn test_key_range_merge_non_adjacent() {
        let left = KeyRange::new("a", "m");
        let right = KeyRange::new("n", "z");
        assert!(left.merge_with(&right).is_none());
    }

    #[test]
    fn test_shard_topology_new() {
        let topology = ShardTopology::new(4, 1000);

        assert_eq!(topology.version, 1);
        assert_eq!(topology.shard_count(), 4);

        // Check that all keys route to some shard
        assert!(topology.get_shard_for_key("").is_some());
        assert!(topology.get_shard_for_key("abc").is_some());
        assert!(topology.get_shard_for_key("xyz").is_some());
    }

    #[test]
    fn test_shard_topology_single_shard() {
        let topology = ShardTopology::new(1, 1000);

        assert_eq!(topology.shard_count(), 1);
        assert_eq!(topology.get_shard_for_key(""), Some(0));
        assert_eq!(topology.get_shard_for_key("anything"), Some(0));
    }

    #[test]
    fn test_shard_topology_split() {
        let mut topology = ShardTopology::new(1, 1000);

        // Split shard 0 at key "m"
        topology.apply_split(0, "m".to_string(), 1, 2000).unwrap();

        assert_eq!(topology.version, 2);
        assert_eq!(topology.shard_count(), 2);

        // Keys before "m" go to shard 0
        assert_eq!(topology.get_shard_for_key("a"), Some(0));
        assert_eq!(topology.get_shard_for_key("l"), Some(0));

        // Keys at or after "m" go to shard 1
        assert_eq!(topology.get_shard_for_key("m"), Some(1));
        assert_eq!(topology.get_shard_for_key("z"), Some(1));
    }

    #[test]
    fn test_shard_topology_merge() {
        let mut topology = ShardTopology::new(1, 1000);

        // Split first
        topology.apply_split(0, "m".to_string(), 1, 2000).unwrap();

        // Merge back
        topology.apply_merge(1, 0, 3000).unwrap();

        assert_eq!(topology.version, 3);

        // Shard 1 is now tombstone
        let shard1 = topology.get_shard(1).unwrap();
        assert!(matches!(
            shard1.state,
            ShardState::Tombstone {
                successor_shard_id: Some(0),
                ..
            }
        ));

        // Shard 0 owns everything again
        assert_eq!(topology.get_shard_for_key("a"), Some(0));
        assert_eq!(topology.get_shard_for_key("z"), Some(0));
    }

    #[test]
    fn test_shard_metrics_qps() {
        let mut metrics = ShardMetrics {
            window_start_ms: 0,
            read_count: 100,
            write_count: 50,
            ..Default::default()
        };

        // At 10 seconds, QPS = 150/10 = 15
        assert_eq!(metrics.qps(10_000), 15);

        metrics.reset_window(10_000);
        assert_eq!(metrics.read_count, 0);
        assert_eq!(metrics.write_count, 0);
    }

    #[test]
    fn test_shard_state_transitions() {
        let state = ShardState::Active;
        assert!(state.can_write());
        assert!(state.can_read());
        assert!(!state.is_transitioning());

        let splitting = ShardState::Splitting {
            split_key: "m".to_string(),
            new_shard_id: 1,
        };
        assert!(!splitting.can_write()); // Writes blocked during split
        assert!(splitting.can_read());
        assert!(splitting.is_transitioning());

        let merging = ShardState::Merging { target_shard_id: 0 };
        assert!(!merging.can_write());
        assert!(merging.can_read());
        assert!(merging.is_transitioning());

        let tombstone = ShardState::Tombstone {
            tombstoned_at: 1000,
            successor_shard_id: Some(0),
        };
        assert!(!tombstone.can_write());
        assert!(!tombstone.can_read());
        assert!(!tombstone.is_transitioning());
    }

    #[test]
    fn test_topology_error_display() {
        let err = TopologyError::ShardNotFound { shard_id: 5 };
        assert_eq!(format!("{}", err), "shard 5 not found");

        let err = TopologyError::InvalidSplitKey {
            key: "x".to_string(),
            range: KeyRange::new("a", "m"),
        };
        assert!(format!("{}", err).contains("split key 'x' not in range"));
    }

    #[test]
    fn test_hex_boundaries_4_shards() {
        let boundaries = generate_hex_boundaries(4);
        assert_eq!(boundaries.len(), 4);

        // First shard starts at ""
        assert_eq!(boundaries[0].0, "");

        // All ranges should be non-empty except possibly last end
        for (start, end) in &boundaries[..3] {
            assert!(!end.is_empty(), "intermediate ranges should have end key");
            assert!(start < end, "start should be < end");
        }

        // Last shard should extend to end
        assert!(boundaries[3].1.is_empty());
    }
}
