//! Round-trip fuzz target for Client RPC messages.
//!
//! Tests that serialize → deserialize → serialize produces identical bytes.
//! This catches:
//! - Postcard encoding inconsistencies
//! - Fields with non-canonical serializations
//! - Panics during serialization of valid types
//!
//! Also tests that deserialized-then-reserialized data is a fixed point,
//! which is required for Raft log replay correctness.

use aspen::fuzz_helpers::ClientRpcRequest;
use aspen::fuzz_helpers::ClientRpcResponse;
use bolero::check;

/// Maximum message size (matches production limit).
const MAX_CLIENT_MESSAGE_SIZE: usize = 4 * 1024 * 1024;

#[test]
fn fuzz_client_rpc_roundtrip() {
    check!().with_type::<Vec<u8>>().for_each(|data| {
        if data.len() > MAX_CLIENT_MESSAGE_SIZE {
            return;
        }

        // Request round-trip: if deserialization succeeds, re-serialization must succeed
        // and produce bytes that deserialize to the same discriminant.
        if let Ok(request) = postcard::from_bytes::<ClientRpcRequest>(data) {
            let reserialized = postcard::to_stdvec(&request).expect("re-serialization of valid request must succeed");

            // The re-serialized form must also deserialize successfully
            let request2 = postcard::from_bytes::<ClientRpcRequest>(&reserialized)
                .expect("deserialization of re-serialized request must succeed");

            // Variant must be preserved
            assert_eq!(request.variant_name(), request2.variant_name(), "Round-trip must preserve request variant");

            // Second serialization must be identical (fixed-point)
            let reserialized2 = postcard::to_stdvec(&request2).expect("second re-serialization");
            assert_eq!(reserialized, reserialized2, "Serialization must be a fixed point (idempotent)");
        }

        // Response round-trip
        if let Ok(response) = postcard::from_bytes::<ClientRpcResponse>(data) {
            let reserialized = postcard::to_stdvec(&response).expect("re-serialization of valid response must succeed");

            let _response2 = postcard::from_bytes::<ClientRpcResponse>(&reserialized)
                .expect("deserialization of re-serialized response must succeed");
        }
    });
}
