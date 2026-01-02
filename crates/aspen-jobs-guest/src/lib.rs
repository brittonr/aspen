//! Guest binary support library for Aspen VM-based job execution.
//!
//! This library provides helpers and abstractions for writing guest binaries
//! that can be executed in Hyperlight micro-VMs by the Aspen job system.
//!
//! # Example
//!
//! ```no_run
//! use aspen_jobs_guest::*;
//!
//! fn process_job(input: &[u8]) -> Vec<u8> {
//!     // Your job processing logic here
//!     format!("Processed: {}", String::from_utf8_lossy(input)).into_bytes()
//! }
//!
//! // Define the guest entry point
//! define_job_handler!(process_job);
//! ```

#![no_std]

extern crate alloc;
use alloc::{vec::Vec, string::String};
use linked_list_allocator::LockedHeap;

// For panic handling in no_std
use core::panic::PanicInfo;

#[panic_handler]
fn panic(_panic: &PanicInfo) -> ! {
    // In a VM guest, we can't really do much on panic
    // Just loop forever
    loop {}
}

// Hyperlight guest API imports
unsafe extern "C" {
    // Print function provided by Hyperlight host
    fn hl_print(msg: *const u8, len: usize);

    // Get time function provided by Hyperlight host
    fn hl_get_time() -> u64;
}

// Global allocator for no_std environment
#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

// Static heap memory (64KB)
static mut HEAP_MEM: [u8; 65536] = [0; 65536];
const HEAP_SIZE: usize = 65536;

/// Initialize the heap allocator
/// This must be called before any allocations
pub fn init_heap() {
    unsafe {
        let heap_start = core::ptr::addr_of_mut!(HEAP_MEM) as *mut u8;
        ALLOCATOR.lock().init(heap_start, HEAP_SIZE);
    }
}

// Memory allocation functions required for guest binaries
#[unsafe(no_mangle)]
pub extern "C" fn malloc(_size: usize) -> *mut u8 {
    // Simple bump allocator for guest
    // In production, use a proper allocator
    core::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub extern "C" fn free(_ptr: *mut u8) {
    // No-op for simple allocator
}

use serde::{Deserialize, Serialize};

/// Result type for guest operations.
pub type Result<T> = core::result::Result<T, GuestError>;

/// Error type for guest operations.
#[derive(Debug)]
pub enum GuestError {
    /// Failed to deserialize input.
    DeserializationFailed,
    /// Failed to serialize output.
    SerializationFailed,
    /// Custom error with message.
    Custom(&'static str),
}

/// Standard job input structure.
#[derive(Debug, Deserialize)]
pub struct JobInput {
    /// Job payload data.
    pub payload: serde_json::Value,
    /// Job configuration.
    pub config: serde_json::Value,
}

/// Standard job output structure.
#[derive(Debug, Serialize)]
pub struct JobOutput {
    /// Indicates if the job succeeded.
    pub success: bool,
    /// Result data.
    pub data: serde_json::Value,
    /// Optional error message.
    pub error: Option<String>,
}

impl JobOutput {
    /// Create a successful output.
    pub fn success(data: serde_json::Value) -> Self {
        Self {
            success: true,
            data,
            error: None,
        }
    }

    /// Create a failure output.
    pub fn failure(error: String) -> Self {
        Self {
            success: false,
            data: serde_json::Value::Null,
            error: Some(error),
        }
    }
}

/// Helper function to parse job input.
pub fn parse_input(input: &[u8]) -> Result<JobInput> {
    serde_json::from_slice(input).map_err(|_| GuestError::DeserializationFailed)
}

/// Helper function to serialize job output.
pub fn serialize_output(output: &JobOutput) -> Result<Vec<u8>> {
    serde_json::to_vec(output).map_err(|_| GuestError::SerializationFailed)
}

/// Print a message to the host (for debugging).
///
/// This calls the host-provided `hl_print` function.
pub fn println(msg: &str) {
    unsafe {
        hl_print(msg.as_ptr(), msg.len());
    }
}

/// Get the current Unix timestamp from the host.
pub fn get_timestamp() -> u64 {
    unsafe {
        hl_get_time()
    }
}

/// Macro to define a job handler function as the guest entry point.
///
/// This macro generates the required `extern "C"` function that Hyperlight
/// expects, handling the low-level details of input/output.
///
/// # Example
///
/// ```no_run
/// use aspen_jobs_guest::*;
///
/// fn my_job_handler(input: &[u8]) -> Vec<u8> {
///     // Process the input
///     format!("Result: {}", input.len()).into_bytes()
/// }
///
/// define_job_handler!(my_job_handler);
/// ```
#[macro_export]
macro_rules! define_job_handler {
    ($handler:ident) => {
        /// Guest entry point called by Hyperlight.
        /// Hyperlight expects a guest_main function that returns i32
        #[unsafe(no_mangle)]
        pub extern "C" fn guest_main() -> i32 {
            // Initialize the heap allocator
            $crate::init_heap();

            // For now, just return success
            // In a real implementation, we'd read input from shared memory
            0
        }

        /// Legacy execute function for compatibility
        #[unsafe(no_mangle)]
        pub extern "C" fn execute(input_ptr: *const u8, input_len: usize) -> *mut u8 {
            // Safety: We trust the host to provide valid input
            let input = unsafe {
                if input_ptr.is_null() || input_len == 0 {
                    &[]
                } else {
                    core::slice::from_raw_parts(input_ptr, input_len)
                }
            };

            // Call the handler
            let output = $handler(input);

            // Allocate memory for output
            let output_len = output.len();
            let output_ptr = if output_len > 0 {
                let mut output_vec = output;
                let ptr = output_vec.as_mut_ptr();
                core::mem::forget(output_vec); // Prevent deallocation
                ptr
            } else {
                core::ptr::null_mut()
            };

            output_ptr
        }

        /// Get the length of the last output.
        #[unsafe(no_mangle)]
        pub extern "C" fn get_output_len() -> usize {
            // This would be implemented with proper state management
            0
        }
    };
}

/// Helper macro for simple job handlers that work with JSON.
///
/// This macro handles JSON deserialization/serialization automatically.
///
/// # Example
///
/// ```no_run
/// use aspen_jobs_guest::*;
/// use serde_json::json;
///
/// fn process(input: JobInput) -> JobOutput {
///     JobOutput::success(json!({
///         "processed": true,
///         "input_size": input.payload.to_string().len()
///     }))
/// }
///
/// define_json_handler!(process);
/// ```
#[macro_export]
macro_rules! define_json_handler {
    ($handler:ident) => {
        fn __json_handler_wrapper(input: &[u8]) -> Vec<u8> {
            // Parse input
            let job_input = match $crate::parse_input(input) {
                Ok(i) => i,
                Err(_) => {
                    let output = $crate::JobOutput::failure("Failed to parse input".into());
                    return $crate::serialize_output(&output).unwrap_or_default();
                }
            };

            // Call the handler
            let output = $handler(job_input);

            // Serialize output
            $crate::serialize_output(&output).unwrap_or_default()
        }

        $crate::define_job_handler!(__json_handler_wrapper);
    };
}