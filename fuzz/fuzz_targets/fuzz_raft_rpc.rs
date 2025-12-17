//! Fuzz target for Raft RPC message deserialization.
//!
//! This target fuzzes the deserialization of Raft RPC messages that arrive
//! over the network from peers. These are HIGH RISK entry points since they
//! handle untrusted network input.
//!
//! Attack vectors tested:
//! - Malformed postcard bytes
//! - Truncated messages
//! - Invalid enum variants
//! - Oversized payloads
//! - Invalid UTF-8 in string fields

use bolero::check;

// Import the types we're fuzzing
use aspen::fuzz_helpers::{RaftRpcProtocol, RaftRpcResponse};

#[test]
fn fuzz_raft_rpc() {
    check!().with_type::<Vec<u8>>().for_each(|data| {
        // Fuzz RaftRpcProtocol deserialization (incoming requests)
        // This is the main entry point for Raft RPCs from peers
        let _ = postcard::from_bytes::<RaftRpcProtocol>(data);

        // Fuzz RaftRpcResponse deserialization (incoming responses)
        // This is what we receive when we make RPC calls to peers
        let _ = postcard::from_bytes::<RaftRpcResponse>(data);
    });
}
