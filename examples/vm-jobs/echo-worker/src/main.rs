//! Simple echo worker for VM job execution.
//!
//! This worker demonstrates the simplest possible guest binary that
//! echoes back its input with a prefix.

#![no_std]
#![no_main]

extern crate alloc;
use alloc::{format, vec::Vec};
use aspen_jobs_guest::*;

/// Simple echo handler that prefixes the input.
fn echo_handler(input: &[u8]) -> Vec<u8> {
    // Convert input to string (or use lossy conversion for invalid UTF-8)
    let input_str = core::str::from_utf8(input)
        .unwrap_or("[invalid UTF-8]");

    // Echo back with prefix
    let response = format!("Echo: {}", input_str);

    // Log to host (for debugging)
    println(&response);

    response.into_bytes()
}

// Define the entry point with all required Hyperlight functions
define_job_handler!(echo_handler);