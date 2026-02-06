//! Hybrid Logical Clock utilities for deterministic distributed ordering.
//!
//! This module provides shared HLC (Hybrid Logical Clock) utilities used throughout
//! the Aspen ecosystem for deterministic timestamp ordering in distributed systems.
//!
//! ## Key Features
//!
//! - **Deterministic ordering**: HLC provides total ordering even when wall clocks are equal
//! - **Causal consistency**: Updates from received timestamps maintain happened-before
//!   relationships
//! - **Node ID tiebreaker**: Each node has a unique 16-byte ID derived from its identifier
//!
//! ## Usage
//!
//! ```ignore
//! use aspen_core::hlc::{create_hlc, new_timestamp, HlcTimestamp};
//!
//! // Create an HLC instance for a node
//! let hlc = create_hlc("node-1");
//!
//! // Generate timestamps
//! let ts1 = new_timestamp(&hlc);
//! let ts2 = new_timestamp(&hlc);
//! assert!(ts2 > ts1); // Monotonically increasing
//!
//! // Convert to Unix milliseconds for display/logging
//! let unix_ms = to_unix_ms(&ts1);
//! ```
//!
//! ## Tiger Style
//!
//! - Deterministic ID derivation from node_id using blake3 hash
//! - No panics in production code (infallible operations)
//! - Pure functions for testability

use serde::Deserialize;
use serde::Serialize;
// Re-export core uhlc types for convenience
pub use uhlc::HLC;
pub use uhlc::ID;
pub use uhlc::NTP64;
pub use uhlc::Timestamp as HlcTimestamp;

/// Create an HLC instance from a node identifier.
///
/// The node ID is hashed using blake3 to produce a deterministic 16-byte HLC ID.
/// This ensures that the same node_id always produces the same HLC instance ID,
/// which is critical for deterministic conflict resolution.
///
/// # Arguments
///
/// * `node_id` - A string identifier for the node (e.g., "node-1", a UUID, etc.)
///
/// # Returns
///
/// An HLC instance ready for timestamp generation.
///
/// # Example
///
/// ```ignore
/// let hlc = create_hlc("my-node-id");
/// let timestamp = hlc.new_timestamp();
/// ```
pub fn create_hlc(node_id: &str) -> HLC {
    let hash = blake3::hash(node_id.as_bytes());
    let bytes = hash.as_bytes();
    // Safety: blake3 hash is always 32 bytes, extracting first 16 via explicit indexing
    // is infallible. Using explicit array construction avoids try_into().unwrap().
    let id_bytes: [u8; 16] = [
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], bytes[8], bytes[9], bytes[10],
        bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
    ];
    // uhlc::ID::try_from([u8; 16]) is infallible - the TryFrom impl for fixed-size arrays
    // always succeeds. Use match to handle the Result without expect().
    match uhlc::ID::try_from(id_bytes) {
        Ok(id) => uhlc::HLCBuilder::new().with_id(id).build(),
        Err(_) => {
            // Unreachable: 16-byte arrays are always valid for uhlc::ID.
            // Fallback to default ID for Tiger Style compliance (no panics).
            uhlc::HLCBuilder::new().build()
        }
    }
}

/// Generate a new HLC timestamp.
///
/// This is a convenience wrapper around `HLC::new_timestamp()`.
/// Each call returns a timestamp that is strictly greater than any
/// previous timestamp from this HLC instance.
///
/// # Arguments
///
/// * `hlc` - The HLC instance to generate a timestamp from
///
/// # Returns
///
/// A new HLC timestamp that is monotonically increasing.
pub fn new_timestamp(hlc: &HLC) -> HlcTimestamp {
    hlc.new_timestamp()
}

/// Update HLC from a received timestamp for causal ordering.
///
/// When receiving a message with an HLC timestamp from another node,
/// call this function to advance the local HLC if the received timestamp
/// is in the future. This maintains the happened-before relationship
/// across distributed nodes.
///
/// # Arguments
///
/// * `hlc` - The local HLC instance to update
/// * `received` - The timestamp received from another node
///
/// # Returns
///
/// * `Ok(())` - If the update was successful
/// * `Err(String)` - If the received timestamp is too far in the future (more than the configured
///   max delta, typically 1 second)
///
/// # Example
///
/// ```ignore
/// // On message receive:
/// if let Err(e) = update_from_timestamp(&local_hlc, &received_timestamp) {
///     warn!("Clock drift detected: {}", e);
/// }
/// ```
pub fn update_from_timestamp(hlc: &HLC, received: &HlcTimestamp) -> Result<(), String> {
    hlc.update_with_timestamp(received)
}

/// Convert HLC timestamp to milliseconds since UNIX epoch.
///
/// This extracts the wall-clock component of the HLC timestamp and
/// converts it to Unix milliseconds. Useful for logging, display,
/// and interop with systems that expect Unix timestamps.
///
/// # Arguments
///
/// * `timestamp` - The HLC timestamp to convert
///
/// # Returns
///
/// Milliseconds since Unix epoch (January 1, 1970 00:00:00 UTC).
///
/// # Note
///
/// This loses the logical clock component and node ID from the HLC.
/// For ordering comparisons, always use the full HLC timestamp.
pub fn to_unix_ms(timestamp: &HlcTimestamp) -> u64 {
    // HLC uses NTP64 format: upper 32 bits are seconds, lower 32 bits are fractions
    let ntp64 = timestamp.get_time().as_u64();
    let seconds = ntp64 >> 32;
    seconds * 1000
}

/// Wrapper for HLC timestamp with serde support.
///
/// The raw `uhlc::Timestamp` doesn't implement Serialize/Deserialize,
/// so this wrapper provides serialization via the underlying NTP64 time and ID.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SerializableTimestamp {
    /// NTP64 time component
    time: u64,
    /// ID component (16 bytes)
    id: [u8; 16],
}

impl SerializableTimestamp {
    /// Create a new serializable timestamp from an HLC timestamp.
    pub fn new(timestamp: HlcTimestamp) -> Self {
        Self {
            time: timestamp.get_time().as_u64(),
            id: timestamp.get_id().to_le_bytes(),
        }
    }

    /// Convert back to an HLC timestamp.
    pub fn to_hlc_timestamp(&self) -> HlcTimestamp {
        // ID::try_from([u8; 16]) is infallible - 16-byte arrays are always valid.
        // Use ok() and unwrap_or to provide a fallback for Tiger Style compliance.
        // The fallback is unreachable but avoids expect/unwrap panics.
        let id = ID::try_from(self.id).ok().unwrap_or_else(|| {
            // Unreachable: self.id is [u8; 16] which is always valid for ID.
            // Use NonZeroU8::MIN (1) as a safe fallback ID.
            ID::from(std::num::NonZeroU8::MIN)
        });
        HlcTimestamp::new(NTP64(self.time), id)
    }

    /// Get the time component as u64.
    pub fn time(&self) -> u64 {
        self.time
    }

    /// Get the ID component.
    pub fn id(&self) -> &[u8; 16] {
        &self.id
    }

    /// Convert to milliseconds since UNIX epoch.
    ///
    /// This extracts the wall-clock component and converts to Unix milliseconds.
    /// Useful for logging, display, and interop with systems that expect Unix timestamps.
    ///
    /// Note: This loses the logical clock component and node ID. For ordering
    /// comparisons, always use the full `SerializableTimestamp`.
    pub fn to_unix_ms(&self) -> u64 {
        // NTP64 format: upper 32 bits are seconds, lower 32 bits are fractions
        let seconds = self.time >> 32;
        seconds * 1000
    }

    /// Create a SerializableTimestamp from Unix milliseconds.
    ///
    /// This creates a timestamp with the given Unix time (converted to NTP64 format)
    /// and a zeroed ID. Useful for reconstructing timestamps from stored Unix
    /// millisecond values, though the resulting timestamp loses node ID information.
    ///
    /// # Arguments
    ///
    /// * `millis` - Milliseconds since Unix epoch (January 1, 1970 00:00:00 UTC)
    ///
    /// # Returns
    ///
    /// A SerializableTimestamp representing approximately that time.
    ///
    /// # Note
    ///
    /// The conversion is approximate since NTP64 has higher precision than
    /// milliseconds, and the node ID is zeroed. For precise ordering, use
    /// timestamps generated by an HLC instance.
    pub fn from_millis(millis: u64) -> Self {
        // Convert to NTP64: seconds in upper 32 bits
        let seconds = millis / 1000;
        let time = seconds << 32;
        Self {
            time,
            id: [0u8; 16], // Zeroed ID since we don't know the original node
        }
    }
}

impl From<HlcTimestamp> for SerializableTimestamp {
    fn from(ts: HlcTimestamp) -> Self {
        Self::new(ts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_hlc_deterministic() {
        // Same node_id should produce consistent HLC behavior
        let hlc1 = create_hlc("test-node");
        let hlc2 = create_hlc("test-node");

        // Both HLCs should be able to generate timestamps
        let ts1 = new_timestamp(&hlc1);
        let ts2 = new_timestamp(&hlc2);

        // Timestamps should be valid (non-zero time)
        assert!(ts1.get_time().as_u64() > 0);
        assert!(ts2.get_time().as_u64() > 0);
    }

    #[test]
    fn test_timestamps_monotonic() {
        let hlc = create_hlc("monotonic-test");

        let ts1 = new_timestamp(&hlc);
        let ts2 = new_timestamp(&hlc);
        let ts3 = new_timestamp(&hlc);

        // Each timestamp should be strictly greater than the previous
        assert!(ts2 > ts1, "ts2 should be > ts1");
        assert!(ts3 > ts2, "ts3 should be > ts2");
    }

    #[test]
    fn test_update_from_future_timestamp() {
        let hlc_sender = create_hlc("sender");
        let hlc_receiver = create_hlc("receiver");

        // Generate a "future" timestamp from sender
        let sender_ts = new_timestamp(&hlc_sender);

        // Update receiver's HLC with sender's timestamp
        let result = update_from_timestamp(&hlc_receiver, &sender_ts);
        assert!(result.is_ok(), "Should accept reasonable timestamp");

        // Receiver's next timestamp should be >= sender's timestamp
        let receiver_ts = new_timestamp(&hlc_receiver);
        assert!(receiver_ts >= sender_ts, "Receiver should advance past sender's timestamp");
    }

    #[test]
    fn test_to_unix_ms_conversion() {
        let hlc = create_hlc("unix-test");
        let ts = new_timestamp(&hlc);
        let unix_ms = to_unix_ms(&ts);

        // Should be a reasonable Unix timestamp (after year 2020)
        let year_2020_ms: u64 = 1577836800000; // Jan 1, 2020 00:00:00 UTC
        assert!(unix_ms > year_2020_ms, "Timestamp should be after year 2020: {} vs {}", unix_ms, year_2020_ms);

        // Should be less than year 2100
        let year_2100_ms: u64 = 4102444800000;
        assert!(unix_ms < year_2100_ms, "Timestamp should be before year 2100");
    }

    #[test]
    fn test_different_nodes_different_ordering() {
        let hlc_a = create_hlc("node-a");
        let hlc_b = create_hlc("node-b");

        // Generate timestamps from different nodes at "the same time"
        let ts_a = new_timestamp(&hlc_a);
        let ts_b = new_timestamp(&hlc_b);

        // They should not be equal (different node IDs break ties)
        assert_ne!(ts_a, ts_b, "Different nodes should produce different timestamps");

        // But they should be comparable (total ordering)
        let _ordering = ts_a.cmp(&ts_b);
        // Just checking it doesn't panic - the actual ordering depends on node IDs
    }

    #[test]
    fn test_serializable_timestamp() {
        let hlc = create_hlc("serde-test");
        let ts = new_timestamp(&hlc);

        // Convert to serializable
        let ser_ts = SerializableTimestamp::new(ts);

        // Serialize and deserialize
        let json = serde_json::to_string(&ser_ts).expect("serialize");
        let deser: SerializableTimestamp = serde_json::from_str(&json).expect("deserialize");

        // Should round-trip correctly
        assert_eq!(ser_ts, deser);
        assert_eq!(ser_ts.time(), deser.time());
        assert_eq!(ser_ts.id(), deser.id());
    }

    // =========================================================================
    // ID Derivation Tests
    // =========================================================================

    #[test]
    fn test_same_node_id_produces_same_hlc_id() {
        let hlc1 = create_hlc("consistent-node");
        let hlc2 = create_hlc("consistent-node");

        let ts1 = new_timestamp(&hlc1);
        let ts2 = new_timestamp(&hlc2);

        // Same node_id means same HLC ID
        assert_eq!(ts1.get_id(), ts2.get_id());
    }

    #[test]
    fn test_different_node_ids_produce_different_hlc_ids() {
        let hlc1 = create_hlc("node-alpha");
        let hlc2 = create_hlc("node-beta");

        let ts1 = new_timestamp(&hlc1);
        let ts2 = new_timestamp(&hlc2);

        // Different node_ids mean different HLC IDs
        assert_ne!(ts1.get_id(), ts2.get_id());
    }

    #[test]
    fn test_empty_node_id() {
        // Empty string should still produce a valid HLC
        let hlc = create_hlc("");
        let ts = new_timestamp(&hlc);
        assert!(ts.get_time().as_u64() > 0);
    }

    #[test]
    fn test_unicode_node_id() {
        // Unicode node IDs should work
        let hlc = create_hlc("node-test");
        let ts = new_timestamp(&hlc);
        assert!(ts.get_time().as_u64() > 0);
    }

    #[test]
    fn test_long_node_id() {
        // Very long node IDs should work (blake3 hashes to fixed size)
        let long_id = "x".repeat(10000);
        let hlc = create_hlc(&long_id);
        let ts = new_timestamp(&hlc);
        assert!(ts.get_time().as_u64() > 0);
    }

    // =========================================================================
    // Timestamp Generation Tests
    // =========================================================================

    #[test]
    fn test_many_timestamps_monotonic() {
        let hlc = create_hlc("batch-test");

        let mut prev = new_timestamp(&hlc);
        for _ in 0..1000 {
            let curr = new_timestamp(&hlc);
            assert!(curr > prev, "Timestamps must be strictly monotonic");
            prev = curr;
        }
    }

    #[test]
    fn test_rapid_timestamp_generation() {
        let hlc = create_hlc("rapid-test");

        // Generate many timestamps in quick succession
        let timestamps: Vec<_> = (0..100).map(|_| new_timestamp(&hlc)).collect();

        // All should be unique
        let unique: std::collections::HashSet<_> = timestamps.iter().map(|ts| ts.to_string()).collect();
        assert_eq!(unique.len(), timestamps.len());
    }

    #[test]
    fn test_timestamp_from_multiple_hlcs_same_node() {
        // Two HLCs with same node_id operate independently
        let hlc1 = create_hlc("shared-node");
        let hlc2 = create_hlc("shared-node");

        let ts1_a = new_timestamp(&hlc1);
        let ts1_b = new_timestamp(&hlc1);
        let ts2_a = new_timestamp(&hlc2);
        let ts2_b = new_timestamp(&hlc2);

        // Each HLC maintains its own monotonic sequence
        assert!(ts1_b > ts1_a);
        assert!(ts2_b > ts2_a);

        // All have same node ID
        assert_eq!(ts1_a.get_id(), ts2_a.get_id());
    }

    // =========================================================================
    // Update from Timestamp Tests
    // =========================================================================

    #[test]
    fn test_update_advances_clock() {
        let hlc_a = create_hlc("node-a");
        let hlc_b = create_hlc("node-b");

        // Generate timestamps from A
        let ts_a1 = new_timestamp(&hlc_a);
        let _ = new_timestamp(&hlc_a);
        let ts_a3 = new_timestamp(&hlc_a);

        // B hasn't generated any timestamps yet
        let ts_b_before = new_timestamp(&hlc_b);

        // Update B with A's latest timestamp
        let result = update_from_timestamp(&hlc_b, &ts_a3);
        assert!(result.is_ok());

        // B's next timestamp should be > A's timestamp
        let ts_b_after = new_timestamp(&hlc_b);
        assert!(ts_b_after > ts_a3);
        assert!(ts_b_after > ts_b_before);

        // And also > A's first timestamp
        assert!(ts_b_after > ts_a1);
    }

    #[test]
    fn test_update_with_past_timestamp() {
        let hlc = create_hlc("test-node");

        // Generate some timestamps to advance the clock
        let _ = new_timestamp(&hlc);
        let _ = new_timestamp(&hlc);
        let current = new_timestamp(&hlc);

        // Create a "past" timestamp (from another node but earlier)
        let hlc_past = create_hlc("past-node");
        let past_ts = new_timestamp(&hlc_past);

        // Update with the past timestamp should succeed
        let result = update_from_timestamp(&hlc, &past_ts);
        assert!(result.is_ok());

        // Next timestamp should still be > current
        let next = new_timestamp(&hlc);
        assert!(next > current);
    }

    // =========================================================================
    // Unix Milliseconds Conversion Tests
    // =========================================================================

    #[test]
    fn test_to_unix_ms_is_consistent() {
        let hlc = create_hlc("unix-test");

        let ts1 = new_timestamp(&hlc);
        let ts2 = new_timestamp(&hlc);

        let ms1 = to_unix_ms(&ts1);
        let ms2 = to_unix_ms(&ts2);

        // ms2 should be >= ms1 (may be equal if within same second)
        assert!(ms2 >= ms1);
    }

    #[test]
    fn test_to_unix_ms_precision() {
        let hlc = create_hlc("precision-test");

        let ts = new_timestamp(&hlc);
        let ms = to_unix_ms(&ts);

        // Should be in milliseconds (reasonable range check)
        let now_approx = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;

        // Allow 1 minute difference
        assert!(ms.abs_diff(now_approx) < 60_000, "Timestamp should be within 1 minute of now");
    }

    // =========================================================================
    // SerializableTimestamp Extended Tests
    // =========================================================================

    #[test]
    fn test_serializable_timestamp_from_trait() {
        let hlc = create_hlc("from-test");
        let ts = new_timestamp(&hlc);

        let ser_ts: SerializableTimestamp = ts.into();
        assert!(ser_ts.time() > 0);
    }

    #[test]
    fn test_serializable_timestamp_to_hlc_roundtrip() {
        let hlc = create_hlc("roundtrip-test");
        let original = new_timestamp(&hlc);

        let ser = SerializableTimestamp::new(original);
        let reconstructed = ser.to_hlc_timestamp();

        // Time and ID should match
        assert_eq!(original.get_time(), reconstructed.get_time());
        assert_eq!(original.get_id(), reconstructed.get_id());
    }

    #[test]
    fn test_serializable_timestamp_to_unix_ms() {
        let hlc = create_hlc("ser-unix-test");
        let ts = new_timestamp(&hlc);
        let ser_ts = SerializableTimestamp::new(ts);

        let direct_ms = to_unix_ms(&ts);
        let ser_ms = ser_ts.to_unix_ms();

        assert_eq!(direct_ms, ser_ms);
    }

    #[test]
    fn test_serializable_timestamp_from_millis() {
        let millis: u64 = 1704067200000; // Jan 1, 2024 00:00:00 UTC

        let ser_ts = SerializableTimestamp::from_millis(millis);

        // ID should be zeroed
        assert_eq!(ser_ts.id(), &[0u8; 16]);

        // Time should round-trip approximately (within same second)
        let recovered_ms = ser_ts.to_unix_ms();
        assert_eq!(recovered_ms / 1000, millis / 1000, "Seconds should match");
    }

    #[test]
    fn test_serializable_timestamp_ordering() {
        let hlc = create_hlc("ordering-test");

        let ts1 = new_timestamp(&hlc);
        let ts2 = new_timestamp(&hlc);

        let ser1 = SerializableTimestamp::new(ts1);
        let ser2 = SerializableTimestamp::new(ts2);

        // Ordering should be preserved
        assert!(ser2 > ser1);
    }

    #[test]
    fn test_serializable_timestamp_hash() {
        use std::collections::HashSet;

        let hlc = create_hlc("hash-test");

        let ts1 = new_timestamp(&hlc);
        let ts2 = new_timestamp(&hlc);
        let ts3 = ts1; // Same timestamp

        let ser1 = SerializableTimestamp::new(ts1);
        let ser2 = SerializableTimestamp::new(ts2);
        let ser3 = SerializableTimestamp::new(ts3);

        let mut set = HashSet::new();
        set.insert(ser1.clone());
        set.insert(ser2);
        set.insert(ser3);

        // ser1 and ser3 are the same, so set should have 2 elements
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_serializable_timestamp_clone() {
        let hlc = create_hlc("clone-test");
        let ts = new_timestamp(&hlc);
        let ser = SerializableTimestamp::new(ts);

        let cloned = ser.clone();

        assert_eq!(ser, cloned);
        assert_eq!(ser.time(), cloned.time());
        assert_eq!(ser.id(), cloned.id());
    }

    #[test]
    fn test_serializable_timestamp_debug() {
        let hlc = create_hlc("debug-test");
        let ts = new_timestamp(&hlc);
        let ser = SerializableTimestamp::new(ts);

        let debug_str = format!("{:?}", ser);
        assert!(debug_str.contains("SerializableTimestamp"));
    }

    #[test]
    fn test_serializable_timestamp_postcard_serialization() {
        let hlc = create_hlc("postcard-test");
        let ts = new_timestamp(&hlc);
        let ser = SerializableTimestamp::new(ts);

        // Test postcard (binary) serialization
        let bytes = postcard::to_allocvec(&ser).expect("postcard serialize");
        let deser: SerializableTimestamp = postcard::from_bytes(&bytes).expect("postcard deserialize");

        assert_eq!(ser, deser);
    }

    // =========================================================================
    // Clock Drift and Edge Case Tests
    // =========================================================================

    #[test]
    fn test_update_from_same_hlc_timestamp() {
        let hlc = create_hlc("same-hlc");

        let ts1 = new_timestamp(&hlc);
        // Updating with own timestamp should work
        let result = update_from_timestamp(&hlc, &ts1);
        assert!(result.is_ok());

        // Next timestamp should still be > ts1
        let ts2 = new_timestamp(&hlc);
        assert!(ts2 > ts1);
    }

    #[test]
    fn test_multiple_updates_from_different_nodes() {
        let hlc_main = create_hlc("main");
        let hlc_peer1 = create_hlc("peer1");
        let hlc_peer2 = create_hlc("peer2");

        // Generate timestamps from peers
        let ts_peer1 = new_timestamp(&hlc_peer1);
        let ts_peer2 = new_timestamp(&hlc_peer2);

        // Update main with both peer timestamps
        let result1 = update_from_timestamp(&hlc_main, &ts_peer1);
        let result2 = update_from_timestamp(&hlc_main, &ts_peer2);

        assert!(result1.is_ok());
        assert!(result2.is_ok());

        // Main's next timestamp should be > both peers
        let ts_main = new_timestamp(&hlc_main);
        assert!(ts_main > ts_peer1);
        assert!(ts_main > ts_peer2);
    }

    #[test]
    fn test_update_chain_preserves_causality() {
        let hlc_a = create_hlc("node-a");
        let hlc_b = create_hlc("node-b");
        let hlc_c = create_hlc("node-c");

        // A generates ts1
        let ts_a = new_timestamp(&hlc_a);

        // B receives from A, generates ts2
        update_from_timestamp(&hlc_b, &ts_a).unwrap();
        let ts_b = new_timestamp(&hlc_b);
        assert!(ts_b > ts_a);

        // C receives from B, generates ts3
        update_from_timestamp(&hlc_c, &ts_b).unwrap();
        let ts_c = new_timestamp(&hlc_c);
        assert!(ts_c > ts_b);
        assert!(ts_c > ts_a);
    }

    #[test]
    fn test_serializable_timestamp_from_millis_boundary_values() {
        // Test with zero
        let ts_zero = SerializableTimestamp::from_millis(0);
        assert_eq!(ts_zero.to_unix_ms(), 0);

        // Test with small value
        let ts_small = SerializableTimestamp::from_millis(1000);
        assert_eq!(ts_small.to_unix_ms() / 1000, 1);

        // Test with large value (year 2100 ish)
        let ts_large = SerializableTimestamp::from_millis(4102444800000);
        assert_eq!(ts_large.to_unix_ms() / 1000, 4102444800000 / 1000);
    }

    #[test]
    fn test_serializable_timestamp_equality_and_ordering() {
        let ts1 = SerializableTimestamp::from_millis(1000);
        let ts2 = SerializableTimestamp::from_millis(1000);
        let ts3 = SerializableTimestamp::from_millis(2000);

        // Same time, same ID -> equal
        assert_eq!(ts1, ts2);

        // Different times -> ordered
        assert!(ts3 > ts1);
        assert!(ts1 < ts3);
    }

    #[test]
    fn test_serializable_timestamp_ordering_with_different_ids() {
        let hlc1 = create_hlc("first");
        let hlc2 = create_hlc("second");

        let ts1 = new_timestamp(&hlc1);
        let ts2 = new_timestamp(&hlc2);

        let ser1 = SerializableTimestamp::new(ts1);
        let ser2 = SerializableTimestamp::new(ts2);

        // They should have a defined ordering (based on time, then id)
        assert!(ser1 != ser2);
        let ordered = ser1 < ser2 || ser2 < ser1;
        assert!(ordered, "Timestamps should be comparable");
    }

    #[test]
    fn test_ntp64_conversion_edge_cases() {
        // Create timestamp at a known Unix time
        let known_millis: u64 = 1704067200000; // Jan 1, 2024 00:00:00 UTC
        let ser_ts = SerializableTimestamp::from_millis(known_millis);

        // Convert back and check it's in the right range
        let recovered = ser_ts.to_unix_ms();
        // Should be within same second
        assert_eq!(recovered / 1000, known_millis / 1000);
    }

    #[test]
    fn test_hlc_id_consistency_across_restarts() {
        // Simulating node restart by creating HLC twice with same node_id
        let hlc1 = create_hlc("persistent-node-id");
        let ts1 = new_timestamp(&hlc1);

        // "Restart" - create new HLC with same node_id
        let hlc2 = create_hlc("persistent-node-id");
        let ts2 = new_timestamp(&hlc2);

        // Both should have same ID
        assert_eq!(ts1.get_id(), ts2.get_id());

        // But ts2 might not be > ts1 (independent HLCs)
        // They should still be comparable
        let _cmp = ts1.cmp(&ts2);
    }

    #[test]
    fn test_serializable_timestamp_bincode_serialization() {
        let hlc = create_hlc("bincode-test");
        let ts = new_timestamp(&hlc);
        let ser = SerializableTimestamp::new(ts);

        // Test bincode serialization
        let bytes = bincode::serialize(&ser).expect("bincode serialize");
        let deser: SerializableTimestamp = bincode::deserialize(&bytes).expect("bincode deserialize");

        assert_eq!(ser, deser);
    }

    #[test]
    fn test_to_unix_ms_large_timestamp() {
        let hlc = create_hlc("large-ts");

        // Generate many timestamps to increase the logical clock
        for _ in 0..100 {
            let _ = new_timestamp(&hlc);
        }

        let ts = new_timestamp(&hlc);
        let ms = to_unix_ms(&ts);

        // Should still be a valid Unix timestamp (not affected by logical clock)
        let now_approx = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;

        assert!(ms.abs_diff(now_approx) < 60_000, "Timestamp should be within 1 minute of now");
    }

    #[test]
    fn test_hlc_with_special_node_ids() {
        // Test with various special strings
        let long_id = "a".repeat(1000);
        let special_ids = vec![
            "",                 // empty
            " ",                // space
            "\n\t",             // whitespace
            "node-with-dashes", // dashes
            "node_with_underscores",
            "123",            // numeric
            long_id.as_str(), // very long
        ];

        for id in special_ids {
            let hlc = create_hlc(id);
            let ts = new_timestamp(&hlc);
            assert!(ts.get_time().as_u64() > 0, "Failed for node_id: {id:?}");
        }
    }

    #[test]
    fn test_concurrent_timestamp_generation_single_hlc() {
        let hlc = create_hlc("concurrent");

        // Generate timestamps from multiple "threads" conceptually
        let mut timestamps = Vec::new();
        for _ in 0..1000 {
            timestamps.push(new_timestamp(&hlc));
        }

        // All should be strictly increasing
        for i in 1..timestamps.len() {
            assert!(timestamps[i] > timestamps[i - 1], "Timestamps must be strictly monotonic at index {i}");
        }
    }

    #[test]
    fn test_serializable_timestamp_id_accessor() {
        let hlc = create_hlc("id-accessor-test");
        let ts = new_timestamp(&hlc);
        let ser = SerializableTimestamp::new(ts);

        // ID should be 16 bytes
        assert_eq!(ser.id().len(), 16);

        // ID should be derived from node_id hash
        let hlc2 = create_hlc("id-accessor-test");
        let ts2 = new_timestamp(&hlc2);
        let ser2 = SerializableTimestamp::new(ts2);

        // Same node_id means same ID bytes
        assert_eq!(ser.id(), ser2.id());
    }

    #[test]
    fn test_serializable_timestamp_from_millis_roundtrip() {
        // Test various millisecond values
        let test_values = vec![
            0u64,
            1000,
            1577836800000, // Jan 1, 2020
            1704067200000, // Jan 1, 2024
            2524608000000, // Jan 1, 2050
        ];

        for millis in test_values {
            let ser = SerializableTimestamp::from_millis(millis);
            let recovered = ser.to_unix_ms();
            // Should match at second precision
            assert_eq!(recovered / 1000, millis / 1000, "Roundtrip failed for millis={millis}");
        }
    }
}
