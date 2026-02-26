//! Shard state, info, and metrics types.

use serde::Deserialize;
use serde::Serialize;

use super::KeyRange;
use crate::router::ShardId;

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
