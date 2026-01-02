//! Simple echo worker for VM job execution.
//!
//! This worker demonstrates the simplest possible guest binary that
//! echoes back its input with a prefix.

#![no_std]
#![no_main]

extern crate alloc;
use alloc::{format, string::String, vec::Vec};
use aspen_jobs_guest::*;

/// Simple echo handler that prefixes the input.
fn echo_handler(input: &[u8]) -> Vec<u8> {
    // Convert input to string (or use lossy conversion for invalid UTF-8)
    let input_str = String::from_utf8_lossy(input);

    // Echo back with prefix
    let response = format!("Echo: {}", input_str);

    // Log to host (for debugging)
    println(&response);

    response.into_bytes()
}

// Define the entry point
define_job_handler!(echo_handler);

// Required for no_std
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    // In a real implementation, we might want to communicate the panic to the host
    loop {}
}

// Entry point for the binary (required for no_main)
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // The actual entry is through the execute function
    // This is just to satisfy the linker
    loop {}
}