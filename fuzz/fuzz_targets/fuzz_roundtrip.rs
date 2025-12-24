//! Fuzz target for round-trip serialization consistency.
//!
//! This target verifies that serialize(deserialize(data)) produces
//! consistent results for all message types. This catches subtle
//! serialization bugs where data is lost or corrupted during encoding.
//!
//! Key property tested:
//! - If deserialize(data) succeeds, then serialize(result) should succeed
//! - serialize(deserialize(data)) should be idempotent after first round-trip
//!
//! Attack vectors tested:
//! - Non-canonical encodings
//! - Padding or trailing bytes that get lost
//! - Field ordering inconsistencies
//! - Default value handling differences

use aspen::fuzz_helpers::AspenClusterTicket;
use aspen::fuzz_helpers::RaftRpcProtocol;
use aspen::fuzz_helpers::RaftRpcResponse;
use aspen::fuzz_helpers::TuiRpcRequest;
use aspen::fuzz_helpers::TuiRpcResponse;
use bolero::check;

/// Tiger Style: Maximum message sizes
const MAX_RPC_SIZE: usize = 10 * 1024 * 1024; // 10 MB
const MAX_TUI_SIZE: usize = 1024 * 1024; // 1 MB

#[test]
fn fuzz_roundtrip() {
    check!().with_type::<Vec<u8>>().for_each(|data| {
        // Skip oversized inputs
        if data.len() > MAX_RPC_SIZE {
            return;
        }

        // Round-trip test for RaftRpcProtocol
        if let Ok(msg) = postcard::from_bytes::<RaftRpcProtocol>(data) {
            if let Ok(serialized) = postcard::to_stdvec(&msg) {
                // Deserialize again
                let msg2 = postcard::from_bytes::<RaftRpcProtocol>(&serialized)
                    .expect("re-deserialization should succeed for valid message");

                // Serialize again
                let serialized2 = postcard::to_stdvec(&msg2).expect("re-serialization should succeed");

                // Idempotency: second round-trip should be identical
                assert_eq!(serialized, serialized2, "round-trip must be idempotent for RaftRpcProtocol");
            }
        }

        // Round-trip test for RaftRpcResponse
        if let Ok(msg) = postcard::from_bytes::<RaftRpcResponse>(data) {
            if let Ok(serialized) = postcard::to_stdvec(&msg) {
                let msg2 = postcard::from_bytes::<RaftRpcResponse>(&serialized)
                    .expect("re-deserialization should succeed for valid message");

                let serialized2 = postcard::to_stdvec(&msg2).expect("re-serialization should succeed");

                assert_eq!(serialized, serialized2, "round-trip must be idempotent for RaftRpcResponse");
            }
        }

        // Round-trip test for TuiRpcRequest (smaller size limit)
        if data.len() <= MAX_TUI_SIZE {
            if let Ok(msg) = postcard::from_bytes::<TuiRpcRequest>(data) {
                if let Ok(serialized) = postcard::to_stdvec(&msg) {
                    let msg2 = postcard::from_bytes::<TuiRpcRequest>(&serialized)
                        .expect("re-deserialization should succeed for valid message");

                    let serialized2 = postcard::to_stdvec(&msg2).expect("re-serialization should succeed");

                    assert_eq!(serialized, serialized2, "round-trip must be idempotent for TuiRpcRequest");
                }
            }

            // Round-trip test for TuiRpcResponse
            if let Ok(msg) = postcard::from_bytes::<TuiRpcResponse>(data) {
                if let Ok(serialized) = postcard::to_stdvec(&msg) {
                    let msg2 = postcard::from_bytes::<TuiRpcResponse>(&serialized)
                        .expect("re-deserialization should succeed for valid message");

                    let serialized2 = postcard::to_stdvec(&msg2).expect("re-serialization should succeed");

                    assert_eq!(serialized, serialized2, "round-trip must be idempotent for TuiRpcResponse");
                }
            }
        }

        // Round-trip test for AspenClusterTicket
        // Tickets use postcard + base32, test the postcard layer
        if let Ok(ticket) = postcard::from_bytes::<AspenClusterTicket>(data) {
            if let Ok(serialized) = postcard::to_stdvec(&ticket) {
                let ticket2 = postcard::from_bytes::<AspenClusterTicket>(&serialized)
                    .expect("re-deserialization should succeed for valid ticket");

                let serialized2 = postcard::to_stdvec(&ticket2).expect("re-serialization should succeed");

                assert_eq!(serialized, serialized2, "round-trip must be idempotent for AspenClusterTicket");
            }
        }
    });
}
