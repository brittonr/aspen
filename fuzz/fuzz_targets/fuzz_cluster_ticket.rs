//! Fuzz target for cluster ticket deserialization.
//!
//! This target fuzzes the postcard deserialization of cluster tickets that
//! users provide via CLI. These are MEDIUM RISK since they come from user
//! input (though typically trusted sources sharing tickets).
//!
//! Attack vectors tested:
//! - Malformed base32 encoding
//! - Invalid postcard payload
//! - Oversized bootstrap sets
//! - Invalid TopicId
//! - Invalid EndpointId values

use bolero::check;

use aspen::fuzz_helpers::AspenClusterTicket;

#[test]
fn fuzz_cluster_ticket() {
    check!().with_type::<Vec<u8>>().for_each(|data| {
        // Fuzz raw postcard deserialization (bypassing base32)
        // This tests the core deserialization path
        let _ = postcard::from_bytes::<AspenClusterTicket>(data);

        // Also fuzz as a string (tests base32 decoding path)
        if let Ok(s) = std::str::from_utf8(data) {
            let _ = AspenClusterTicket::deserialize(s);
        }

        // Fuzz TopicId alone (32-byte fixed size)
        if data.len() >= 32 {
            let topic_bytes: [u8; 32] = data[0..32].try_into().unwrap();
            let _ = iroh_gossip::proto::TopicId::from_bytes(topic_bytes);
        }
    });
}
