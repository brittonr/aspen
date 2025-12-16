//! Fuzz target for TUI RPC message deserialization.
//!
//! This target fuzzes the deserialization of TUI RPC messages that arrive
//! over Iroh QUIC connections from aspen-tui clients. These are HIGH RISK
//! entry points since they handle potentially untrusted network input.
//!
//! Attack vectors tested:
//! - Malformed postcard bytes
//! - Truncated messages
//! - Invalid enum variants
//! - Oversized strings and vectors
//! - Invalid UTF-8 in string fields
//! - Boundary conditions for node IDs

#![no_main]

use libfuzzer_sys::fuzz_target;

// Import the types we're fuzzing
use aspen::fuzz_helpers::{TuiRpcRequest, TuiRpcResponse};

// Tiger Style: Bounded message size
const MAX_TUI_MESSAGE_SIZE: usize = 1024 * 1024;

fuzz_target!(|data: &[u8]| {
    // Tiger Style: Skip oversized inputs early
    if data.len() > MAX_TUI_MESSAGE_SIZE {
        return;
    }

    // Fuzz TuiRpcRequest deserialization (incoming from TUI clients)
    // This is the main entry point for TUI operations
    let _ = postcard::from_bytes::<TuiRpcRequest>(data);

    // Fuzz TuiRpcResponse deserialization
    // Tests the response path for consistency
    let _ = postcard::from_bytes::<TuiRpcResponse>(data);
});
