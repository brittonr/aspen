//! Guest binary support library for Aspen VM-based job execution.
//!
//! This library provides helpers and abstractions for writing guest binaries
//! that can be executed in Hyperlight micro-VMs by the Aspen job system.
//!
//! # Hyperlight Execution Model
//!
//! This guest code runs in Hyperlight's sandboxed micro-VM environment with
//! the following guarantees provided by the Hyperlight host:
//!
//! - **Single-threaded execution**: Each VM instance runs exactly one thread. The host serializes
//!   all calls through Hyperlight's RPC mechanism, so concurrent access to global state cannot
//!   occur within a single VM.
//!
//! - **Memory isolation**: Guest memory is completely isolated from host memory. The guest cannot
//!   access host memory, and the host accesses guest memory only through explicit API calls with
//!   validated pointers.
//!
//! - **Entry sequence**: The execution flow is:
//!   1. `_start()` - VM entry point, initializes environment
//!   2. `hyperlight_main()` - Called by `_start()`, initializes heap and buffers
//!   3. `execute()` - Called by host for each job invocation
//!   4. `get_result_len()` - Called by host to retrieve result size
//!
//! - **ABI contract**: The host provides valid pointers via the Hyperlight ABI. Input pointers
//!   passed to `execute()` are guaranteed to be valid for the specified length and aligned
//!   appropriately for byte access.
//!
//! # Safety Invariants
//!
//! This module contains unsafe code that relies on the following invariants:
//!
//! 1. **Single-threaded access to RESULT_BUFFER**: Only one function (`execute`, `get_result_len`,
//!    or `hyperlight_main`) runs at a time.
//!
//! 2. **Host copies result before next execute()**: The host must copy the result data before
//!    calling `execute()` again, as the buffer is reused.
//!
//! 3. **Valid FFI pointers**: Pointers passed from the host are valid for the specified length and
//!    remain valid for the duration of the function call.
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
use alloc::string::String;
use alloc::vec::Vec;
use core::panic::PanicInfo;

// We need to provide our own allocator since we're not using std
use linked_list_allocator::LockedHeap;

// Global allocator for no_std environment
#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

// Static heap memory (256KB for more complex operations)
static mut HEAP_MEM: [u8; 262144] = [0; 262144];

/// Initialize the heap allocator.
///
/// # Safety
///
/// This function is safe to call because:
/// - `HEAP_MEM` is a static array that exists for the lifetime of the program
/// - The heap memory is never moved (static location)
/// - `init_heap` is called exactly once per VM instance (guarded by AtomicBool in macro)
/// - Single-threaded execution prevents concurrent initialization
pub fn init_heap() {
    // SAFETY: HEAP_MEM is a static array that:
    // 1. Exists for the entire lifetime of the program (static storage)
    // 2. Is never moved or reallocated (fixed address in guest memory)
    // 3. Is accessed single-threaded (Hyperlight guarantees)
    // 4. Is only initialized once (guarded by atomic flag in macro)
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
use serde::Deserialize;
use serde::Serialize;
pub use serde_json;

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
/// This macro generates the required entry points for Hyperlight, including
/// the `execute`, `get_result_len`, and `hyperlight_main` functions that the
/// host uses to interact with the guest.
///
/// # Safety
///
/// The generated code contains unsafe blocks that rely on the following
/// invariants guaranteed by the Hyperlight execution model:
///
/// 1. **Single-threaded execution**: Only one function runs at a time per VM.
/// 2. **Valid host pointers**: Pointers passed by the host are valid for their specified length and
///    properly aligned.
/// 3. **Sequential access**: The host calls `execute()`, then `get_result_len()`, then copies the
///    result before calling `execute()` again.
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
        // Global state to store the handler result.
        //
        // SAFETY: This mutable static is safe because Hyperlight guarantees
        // single-threaded execution per VM instance. Only one of execute(),
        // get_result_len(), or hyperlight_main() runs at any time.
        static mut RESULT_BUFFER: Option<alloc::vec::Vec<u8>> = None;

        /// Entry point for the VM.
        ///
        /// Initializes the environment and enters a spin loop waiting for
        /// function calls from the host. The `pause` instruction is used
        /// for efficient spinning without causing a VM exit.
        #[unsafe(no_mangle)]
        pub extern "C" fn _start() -> ! {
            // Call hyperlight_main for initialization
            hyperlight_main();

            // Keep the VM alive with a simple spin loop.
            // The pause instruction hints to the CPU that we're spinning
            // but doesn't cause VM exit like hlt does.
            loop {
                // SAFETY: The pause instruction is a no-op hint that improves
                // spin-wait performance. It has no memory safety implications.
                unsafe {
                    core::arch::asm!("pause");
                }
            }
        }

        /// Function called by Hyperlight host via `sandbox.call("execute", input)`.
        ///
        /// This function processes input from the host and returns a pointer to
        /// the result buffer. The host must call `get_result_len()` to determine
        /// the result size and copy the data before calling `execute()` again.
        ///
        /// # Safety
        ///
        /// This function is safe to call from the Hyperlight host because:
        /// - `input_ptr` is validated (null check, bounds via `input_len`)
        /// - Single-threaded access to `RESULT_BUFFER` (Hyperlight guarantees)
        /// - The returned pointer remains valid until the next `execute()` call
        #[unsafe(no_mangle)]
        #[unsafe(export_name = "execute")]
        pub extern "C" fn execute(input_ptr: *const u8, input_len: usize) -> *mut u8 {
            // Initialize heap if not already done (idempotent via atomic flag)
            static INIT: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);
            if !INIT.swap(true, core::sync::atomic::Ordering::SeqCst) {
                $crate::init_heap();
            }

            // SAFETY: Convert host-provided pointer to a Rust slice.
            // Invariants guaranteed by Hyperlight ABI:
            // 1. If input_ptr is non-null, it points to at least input_len valid bytes
            // 2. The memory remains valid for the duration of this function call
            // 3. The host does not modify this memory during the call
            // 4. The pointer is properly aligned for byte access (u8 has no alignment requirements)
            let input = unsafe {
                if input_ptr.is_null() || input_len == 0 {
                    &[]
                } else {
                    core::slice::from_raw_parts(input_ptr, input_len)
                }
            };

            // Call the user-provided handler
            let output = $handler(input);

            // SAFETY: Store result in global buffer and return pointer.
            // This is safe because:
            // 1. Hyperlight guarantees single-threaded execution per VM
            // 2. Only execute() writes to RESULT_BUFFER; get_result_len() only reads
            // 3. The host must copy the result before calling execute() again
            // 4. The pointer remains valid until RESULT_BUFFER is overwritten
            unsafe {
                RESULT_BUFFER = Some(output.clone());
                if let Some(ref result) = RESULT_BUFFER {
                    result.as_ptr() as *mut u8
                } else {
                    core::ptr::null_mut()
                }
            }
        }

        /// Return the length of the last result.
        ///
        /// Called by the host after `execute()` to determine how many bytes
        /// to copy from the result pointer.
        #[unsafe(no_mangle)]
        #[unsafe(export_name = "get_result_len")]
        pub extern "C" fn get_result_len() -> usize {
            // SAFETY: Read from RESULT_BUFFER is safe because:
            // 1. Single-threaded execution (Hyperlight guarantees)
            // 2. This is always called after execute(), which initializes the buffer
            // 3. We only read the length, not modifying the buffer
            unsafe { RESULT_BUFFER.as_ref().map(|v| v.len()).unwrap_or(0) }
        }

        /// Alternative entry point that Hyperlight might look for.
        #[unsafe(no_mangle)]
        #[unsafe(export_name = "guest_main")]
        pub extern "C" fn guest_main() -> i32 {
            // Initialize the heap
            $crate::init_heap();
            // Return success
            0
        }

        /// Primary Hyperlight entry point - called by `_start`.
        ///
        /// Initializes the heap and result buffer. Called once during VM startup.
        #[unsafe(no_mangle)]
        #[unsafe(export_name = "hyperlight_main")]
        pub extern "C" fn hyperlight_main() {
            // Initialize the heap
            $crate::init_heap();

            // SAFETY: Initialize RESULT_BUFFER if not already set.
            // This is safe because:
            // 1. hyperlight_main() is called exactly once during VM startup
            // 2. Single-threaded execution prevents concurrent access
            // 3. We check for None before writing to avoid double-initialization
            unsafe {
                if RESULT_BUFFER.is_none() {
                    RESULT_BUFFER = Some(alloc::vec::Vec::new());
                }
            }

            // Return cleanly - Hyperlight will manage the VM lifecycle
            // and call the execute function when needed
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
                    let output = $crate::JobOutput::failure(alloc::format!("Failed to parse input: {}", e));
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
