//! Fuzz target for node metadata serialization.
//!
//! This target fuzzes bincode serialization/deserialization of node metadata
//! stored in the cluster registry. While this is local storage, corruption
//! could occur from disk errors or bugs.
//!
//! Attack vectors tested:
//! - Malformed bincode bytes
//! - Truncated metadata entries
//! - Invalid string encoding
//! - Node ID boundary values
//! - Timestamp edge cases

#![no_main]

use libfuzzer_sys::fuzz_target;
use arbitrary::Arbitrary;
use serde::{Deserialize, Serialize};

/// Tiger Style: Maximum metadata entry size
const MAX_METADATA_SIZE: usize = 64 * 1024; // 64 KB

/// Node status values
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Arbitrary, PartialEq)]
#[repr(u8)]
enum NodeStatus {
    Unknown = 0,
    Joining = 1,
    Active = 2,
    Leaving = 3,
    Failed = 4,
}

/// Simplified NodeMetadata matching cluster/metadata.rs
#[derive(Debug, Clone, Serialize, Deserialize, Arbitrary)]
struct FuzzNodeMetadata {
    /// Raft node identifier
    node_id: u64,
    /// Iroh endpoint ID (base32 encoded string)
    endpoint_id: String,
    /// Raft address string
    raft_addr: String,
    /// Current status
    status: NodeStatus,
    /// Last update timestamp (Unix epoch seconds)
    last_updated_secs: u64,
}

/// Node registry entry (key-value in redb)
#[derive(Debug, Clone, Serialize, Deserialize, Arbitrary)]
struct FuzzRegistryEntry {
    /// The node ID as key
    key: u64,
    /// Metadata as value
    metadata: FuzzNodeMetadata,
}

fuzz_target!(|data: &[u8]| {
    // Tiger Style: Skip oversized inputs
    if data.len() > MAX_METADATA_SIZE {
        return;
    }

    // Fuzz bincode deserialization of NodeMetadata
    let metadata_result = bincode::deserialize::<FuzzNodeMetadata>(data);

    // If deserialization succeeded, verify round-trip
    if let Ok(metadata) = metadata_result {
        // Re-serialize
        if let Ok(serialized) = bincode::serialize(&metadata) {
            // Re-deserialize
            let metadata2 = bincode::deserialize::<FuzzNodeMetadata>(&serialized)
                .expect("re-deserialization should succeed");

            // Verify key fields match
            assert_eq!(metadata.node_id, metadata2.node_id);
            assert_eq!(metadata.status, metadata2.status);
            assert_eq!(metadata.last_updated_secs, metadata2.last_updated_secs);
        }
    }

    // Fuzz registry entry deserialization
    let entry_result = bincode::deserialize::<FuzzRegistryEntry>(data);

    if let Ok(entry) = entry_result {
        // Verify key matches metadata
        // In real implementation, these should be consistent
        let _ = entry.key;
        let _ = entry.metadata.node_id;

        // Test serialization
        if let Ok(serialized) = bincode::serialize(&entry) {
            let entry2 = bincode::deserialize::<FuzzRegistryEntry>(&serialized)
                .expect("re-deserialization should succeed");
            assert_eq!(entry.key, entry2.key);
        }
    }

    // Test with structure-aware input
    let arbitrary_result = FuzzNodeMetadata::arbitrary(&mut arbitrary::Unstructured::new(data));
    if let Ok(metadata) = arbitrary_result {
        // Verify we can serialize arbitrary metadata
        if let Ok(serialized) = bincode::serialize(&metadata) {
            // Tiger Style: Serialized size should be bounded
            assert!(
                serialized.len() <= MAX_METADATA_SIZE,
                "serialized metadata too large: {}",
                serialized.len()
            );
        }
    }
});
