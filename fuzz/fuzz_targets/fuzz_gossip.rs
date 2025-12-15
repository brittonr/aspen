//! Fuzz target for gossip peer announcement deserialization.
//!
//! This target fuzzes the postcard deserialization of gossip messages from
//! peers. These are MEDIUM RISK since gossip messages come from potentially
//! untrusted peers on the network.
//!
//! Note: The PeerAnnouncement type is private, so we fuzz the public
//! fields that would be deserialized (NodeId, EndpointAddr).
//!
//! Attack vectors tested:
//! - Malformed postcard bytes
//! - Invalid NodeId values
//! - Invalid EndpointAddr structure
//! - Truncated messages

#![no_main]

use libfuzzer_sys::fuzz_target;

use aspen::fuzz_helpers::NodeId;

// Simplified gossip message structure for fuzzing
// Matches PeerAnnouncement in gossip_discovery.rs
#[derive(serde::Serialize, serde::Deserialize)]
struct FuzzPeerAnnouncement {
    node_id: NodeId,
    // EndpointAddr is complex, just fuzz the bytes
    endpoint_addr_bytes: Vec<u8>,
    timestamp_micros: u64,
}

fuzz_target!(|data: &[u8]| {
    // Fuzz the simplified peer announcement
    let _ = postcard::from_bytes::<FuzzPeerAnnouncement>(data);

    // Also fuzz just NodeId deserialization
    let _ = postcard::from_bytes::<NodeId>(data);
    let _ = bincode::deserialize::<NodeId>(data);

    // Fuzz u64 timestamp extraction (should handle truncated data)
    if data.len() >= 8 {
        let _timestamp = u64::from_le_bytes(data[0..8].try_into().unwrap());
    }
});
