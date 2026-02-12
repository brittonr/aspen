//! Hybrid Logical Clock utilities for deterministic distributed ordering.
//!
//! This crate provides shared HLC (Hybrid Logical Clock) utilities used throughout
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
//! use aspen_hlc::{create_hlc, new_timestamp, HlcTimestamp};
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
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], bytes[8],
        bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
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
        assert!(
            receiver_ts >= sender_ts,
            "Receiver should advance past sender's timestamp"
        );
    }

    #[test]
    fn test_to_unix_ms_conversion() {
        let hlc = create_hlc("unix-test");
        let ts = new_timestamp(&hlc);
        let unix_ms = to_unix_ms(&ts);

        // Should be a reasonable Unix timestamp (after year 2020)
        let year_2020_ms: u64 = 1577836800000; // Jan 1, 2020 00:00:00 UTC
        assert!(
            unix_ms > year_2020_ms,
            "Timestamp should be after year 2020: {} vs {}",
            unix_ms,
            year_2020_ms
        );

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
        assert_ne!(
            ts_a, ts_b,
            "Different nodes should produce different timestamps"
        );

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
    fn test_serializable_timestamp_postcard_serialization() {
        let hlc = create_hlc("postcard-test");
        let ts = new_timestamp(&hlc);
        let ser = SerializableTimestamp::new(ts);

        // Test postcard (binary) serialization
        let bytes = postcard::to_allocvec(&ser).expect("postcard serialize");
        let deser: SerializableTimestamp = postcard::from_bytes(&bytes).expect("postcard deserialize");

        assert_eq!(ser, deser);
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
}
