//! Complete cluster shard topology with versioning and split/merge operations.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde::Serialize;

use super::KeyRange;
use super::ShardInfo;
use super::ShardState;
use super::TopologyError;
use super::constants::SHARD_TOMBSTONE_GRACE_PERIOD_SECS;
use super::routing::generate_hex_boundaries;
use crate::MAX_SHARDS;
use crate::router::ShardId;

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

        // SAFETY: We verified self.shards.contains_key(&source_shard_id) at the start
        // of this function and returned early if false. No mutations remove this key.
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

        // SAFETY: We verified self.shards.contains_key(&target_shard_id) at the start
        // and borrowed it immutably above. No mutations remove this key.
        let target = self.shards.get_mut(&target_shard_id).unwrap();
        target.key_range = merged_range.clone();

        // SAFETY: We verified self.shards.contains_key(&source_shard_id) at the start.
        // No mutations remove this key between that check and here.
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
