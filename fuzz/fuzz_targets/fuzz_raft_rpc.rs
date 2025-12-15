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

#![no_main]

use libfuzzer_sys::fuzz_target;

// Import the types we're fuzzing
use aspen::fuzz_helpers::{RaftRpcProtocol, RaftRpcResponse};

fuzz_target!(|data: &[u8]| {
    // Fuzz RaftRpcProtocol deserialization (incoming requests)
    // This is the main entry point for Raft RPCs from peers
    let _ = postcard::from_bytes::<RaftRpcProtocol>(data);

    // Fuzz RaftRpcResponse deserialization (incoming responses)
    // This is what we receive when we make RPC calls to peers
    let _ = postcard::from_bytes::<RaftRpcResponse>(data);
});
