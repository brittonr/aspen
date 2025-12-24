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

use aspen::fuzz_helpers::RaftRpcProtocol;
use aspen::fuzz_helpers::RaftRpcResponse;
use aspen::fuzz_helpers::TuiRpcRequest;
use aspen::fuzz_helpers::TuiRpcResponse;
use bolero::check;

// Tiger Style: Constants from protocol_handlers.rs
const MAX_RPC_MESSAGE_SIZE: usize = 10 * 1024 * 1024; // 10 MB
const MAX_TUI_MESSAGE_SIZE: usize = 1024 * 1024; // 1 MB

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

        // Test TUI RPC path - smaller size limit for TUI clients
        // Protocol handler reads up to MAX_TUI_MESSAGE_SIZE bytes then deserializes
        if data.len() <= MAX_TUI_MESSAGE_SIZE {
            // Try deserializing as TUI request (incoming from TUI clients)
            let _ = postcard::from_bytes::<TuiRpcRequest>(data);
            // Try deserializing as TUI response
            let _ = postcard::from_bytes::<TuiRpcResponse>(data);
        }

        // Test boundary cases: messages exactly at size limits
        // These edge cases are particularly interesting for fuzzing
        if data.len() == MAX_TUI_MESSAGE_SIZE || data.len() == MAX_RPC_MESSAGE_SIZE {
            // Ensure no panics at exact boundaries
            let _ = postcard::from_bytes::<RaftRpcProtocol>(data);
            let _ = postcard::from_bytes::<TuiRpcRequest>(data);
        }
    });
}
