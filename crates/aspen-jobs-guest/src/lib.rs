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

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{vec::Vec, string::String, format};

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
/// This calls the host-provided `hl_println` function.
pub fn println(msg: &str) {
    // This would call the actual Hyperlight host function
    // For now, it's a placeholder
    #[cfg(feature = "std")]
    eprintln!("[Guest] {}", msg);
}

/// Get the current Unix timestamp from the host.
pub fn get_timestamp() -> u64 {
    // This would call the host-provided `hl_get_time` function
    // For now, return a placeholder
    #[cfg(feature = "std")]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
    #[cfg(not(feature = "std"))]
    {
        0
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
        #[no_mangle]
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
        #[no_mangle]
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