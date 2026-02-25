//! Fuzz target for protocol handler message parsing.
//!
//! This target fuzzes the combined size-bounded read + postcard parse path
//! used by protocol handlers for both Raft RPC and TUI RPC protocols.
//! Tests the protocol_handlers.rs code paths without requiring network I/O.
//!
//! Attack vectors tested:
//! - Messages at exact size limits
//! - Truncated postcard streams
//! - Valid postcard of wrong variant
//! - Messages just under/at/over size limits
//! - Invalid message types

use aspen::fuzz_helpers::ClientRpcRequest;
use aspen::fuzz_helpers::ClientRpcResponse;
use aspen::fuzz_helpers::RaftRpcProtocol;
use aspen::fuzz_helpers::RaftRpcResponse;
use bolero::check;

// Tiger Style: Constants from protocol_handlers.rs
const MAX_RPC_MESSAGE_SIZE: usize = 10 * 1024 * 1024; // 10 MB

#[test]
fn fuzz_protocol_handler() {
    check!().with_type::<Vec<u8>>().for_each(|data| {
        // Test Raft RPC path - simulates size-bounded stream read
        // Protocol handler reads up to MAX_RPC_MESSAGE_SIZE bytes then deserializes
        if data.len() <= MAX_RPC_MESSAGE_SIZE {
            // Try deserializing as request (incoming from peers)
            let _ = postcard::from_bytes::<RaftRpcProtocol>(data);
            // Try deserializing as response (incoming from peers)
            let _ = postcard::from_bytes::<RaftRpcResponse>(data);
        }

        // Test Client RPC path - uses same size limit as Raft RPC
        if data.len() <= MAX_RPC_MESSAGE_SIZE {
            // Try deserializing as client request (incoming from CLI/apps)
            let _ = postcard::from_bytes::<ClientRpcRequest>(data);
            // Try deserializing as client response
            let _ = postcard::from_bytes::<ClientRpcResponse>(data);
        }
    });
}
