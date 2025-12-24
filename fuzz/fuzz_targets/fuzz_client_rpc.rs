//! Fuzz target for Client RPC message deserialization.
//!
//! This target fuzzes the deserialization of Client RPC messages that arrive
//! over Iroh QUIC connections from clients. These are HIGH RISK entry points
//! since they handle potentially untrusted network input.
//!
//! Attack vectors tested:
//! - Malformed postcard bytes
//! - Truncated messages
//! - Invalid enum variants
//! - Oversized strings and vectors
//! - Invalid UTF-8 in string fields
//! - Boundary conditions for node IDs

// Import the types we're fuzzing
use aspen::fuzz_helpers::ClientRpcRequest;
use aspen::fuzz_helpers::ClientRpcResponse;
use bolero::check;

// Tiger Style: Bounded message size
const MAX_CLIENT_MESSAGE_SIZE: usize = 1024 * 1024;

#[test]
fn fuzz_client_rpc() {
    check!().with_type::<Vec<u8>>().for_each(|data| {
        // Tiger Style: Skip oversized inputs early
        if data.len() > MAX_CLIENT_MESSAGE_SIZE {
            return;
        }

        // Fuzz ClientRpcRequest deserialization (incoming from clients)
        // This is the main entry point for client operations
        let _ = postcard::from_bytes::<ClientRpcRequest>(data);

        // Fuzz ClientRpcResponse deserialization
        // Tests the response path for consistency
        let _ = postcard::from_bytes::<ClientRpcResponse>(data);
    });
}
