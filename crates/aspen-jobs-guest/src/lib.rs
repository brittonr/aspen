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
#![no_main]

extern crate alloc;
use alloc::{vec::Vec, string::String, format};
use core::panic::PanicInfo;

// We need to provide our own allocator since we're not using std
use linked_list_allocator::LockedHeap;

// Global allocator for no_std environment
#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

// Static heap memory (256KB for more complex operations)
static mut HEAP_MEM: [u8; 262144] = [0; 262144];

/// Initialize the heap allocator
pub fn init_heap() {
    unsafe {
        let heap_start = core::ptr::addr_of_mut!(HEAP_MEM) as *mut u8;
        const HEAP_SIZE: usize = 262144;
        ALLOCATOR.lock().init(heap_start, HEAP_SIZE);
    }
}

// Panic handler for no_std
#[panic_handler]
fn panic(_panic: &PanicInfo) -> ! {
    // In a VM guest, we can't do much on panic
    // Just halt the VM
    unsafe {
        core::arch::asm!("hlt");
    }
    loop {}
}

// Re-export serde for convenience
pub use serde;
pub use serde_json;

use serde::{Deserialize, Serialize};

/// Standard job input structure.
#[derive(Debug, Deserialize)]
pub struct JobInput {
    /// Raw input data
    pub data: Vec<u8>,
    /// Optional configuration
    pub config: Option<serde_json::Value>,
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

/// Helper function to log messages to the host (for debugging).
pub fn println(msg: &str) {
    // For now, this is a no-op since we don't have host function access
    // In a real implementation, this would call a host-provided function
    let _ = msg;
}

/// Macro to define a job handler function as the guest entry point.
///
/// This macro generates the required entry points for Hyperlight.
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

        // Global state to store the handler result
        static mut RESULT_BUFFER: Option<alloc::vec::Vec<u8>> = None;

        /// Entry point for the VM
        /// This is what prevents immediate shutdown
        #[unsafe(no_mangle)]
        pub extern "C" fn _start() -> ! {
            // Initialize the heap
            $crate::init_heap();

            // Initialize any other resources
            unsafe {
                RESULT_BUFFER = Some(alloc::vec::Vec::new());
            }

            // Main loop - wait for work
            loop {
                // In a real implementation, we'd wait for host signals
                // For now, just keep the VM alive
                unsafe {
                    // Use HLT instruction to pause CPU until next interrupt
                    core::arch::asm!("hlt");
                }
            }
        }

        /// Function called by Hyperlight host via sandbox.call("execute", input)
        /// This needs to match the ABI that Hyperlight expects
        #[unsafe(no_mangle)]
        pub extern "C" fn execute(input_ptr: *const u8, input_len: usize) -> *mut u8 {
            // Initialize heap if not already done
            static INIT: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);
            if !INIT.swap(true, core::sync::atomic::Ordering::SeqCst) {
                $crate::init_heap();
            }

            // Get the input data
            let input = unsafe {
                if input_ptr.is_null() || input_len == 0 {
                    &[]
                } else {
                    core::slice::from_raw_parts(input_ptr, input_len)
                }
            };

            // Call the handler
            let output = $handler(input);

            // Store the result
            unsafe {
                RESULT_BUFFER = Some(output.clone());
                if let Some(ref result) = RESULT_BUFFER {
                    result.as_ptr() as *mut u8
                } else {
                    core::ptr::null_mut()
                }
            }
        }

        /// Return the length of the last result
        #[unsafe(no_mangle)]
        pub extern "C" fn get_result_len() -> usize {
            unsafe {
                RESULT_BUFFER.as_ref().map(|v| v.len()).unwrap_or(0)
            }
        }

        /// Alternative entry point names that Hyperlight might look for
        #[unsafe(no_mangle)]
        pub extern "C" fn guest_main() -> i32 {
            // Initialize the heap
            $crate::init_heap();
            // Return success
            0
        }

        /// Another possible entry point
        #[unsafe(no_mangle)]
        pub extern "C" fn hyperlight_main() {
            // Initialize the heap
            $crate::init_heap();
        }
    };
}

/// Macro for JSON-based job handlers.
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
///         "input_size": input.data.len()
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
            let job_input = match serde_json::from_slice::<$crate::JobInput>(input) {
                Ok(i) => i,
                Err(e) => {
                    let output = $crate::JobOutput::failure(
                        alloc::format!("Failed to parse input: {}", e)
                    );
                    return serde_json::to_vec(&output).unwrap_or_default();
                }
            };

            // Call the handler
            let output = $handler(job_input);

            // Serialize output
            serde_json::to_vec(&output).unwrap_or_default()
        }

        $crate::define_job_handler!(__json_handler_wrapper);
    };
}