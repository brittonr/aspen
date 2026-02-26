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

mod announcement;
pub mod constants;
mod error;
mod key_range;
mod routing;
mod shard;
mod topology_map;

pub use announcement::TopologyAnnouncement;
pub use constants::DEFAULT_MERGE_MAX_COMBINED_BYTES;
pub use constants::DEFAULT_MERGE_SIZE_BYTES;
pub use constants::DEFAULT_SPLIT_QPS;
pub use constants::DEFAULT_SPLIT_SIZE_BYTES;
pub use constants::SHARD_TOMBSTONE_GRACE_PERIOD_SECS;
pub use error::TopologyError;
pub use key_range::KeyRange;
pub use shard::ShardInfo;
pub use shard::ShardMetrics;
pub use shard::ShardState;
pub use topology_map::ShardTopology;

#[cfg(test)]
mod tests {
    use routing::generate_hex_boundaries;

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
        assert!(matches!(shard1.state, ShardState::Tombstone {
            successor_shard_id: Some(0),
            ..
        }));

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
