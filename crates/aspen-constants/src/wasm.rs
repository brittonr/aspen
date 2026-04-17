//! WASM Component execution resource bounds.
//!
//! Tiger Style: Fixed limits for WASM sandbox execution to prevent
//! resource exhaustion. All WASM components run under these constraints.

/// Maximum WASM component size in bytes (50 MB).
pub const MAX_WASM_COMPONENT_SIZE: u64 = 52_428_800;

/// Default fuel limit for WASM execution (~10-100s of compute).
pub const DEFAULT_WASM_FUEL_LIMIT: u64 = 100_000_000;

/// Maximum allowed fuel limit.
pub const MAX_WASM_FUEL_LIMIT: u64 = 10_000_000_000;

/// Fuel yield interval for cooperative scheduling (Lunatic pattern).
pub const WASM_FUEL_YIELD_INTERVAL: u64 = 100_000;

/// Default memory limit for WASM execution (256 MB).
pub const DEFAULT_WASM_MEMORY_LIMIT: u64 = 268_435_456;

/// Maximum memory limit for WASM execution (1 GB).
pub const MAX_WASM_MEMORY_LIMIT: u64 = 1_073_741_824;

/// Default wall-clock execution timeout for a single WASM guest call (30 seconds).
///
/// hyperlight-wasm 0.12 does not expose fuel metering, so we enforce execution
/// limits via wall-clock timeout around the `spawn_blocking` call. This prevents
/// infinite loops or pathological compute from blocking the handler indefinitely.
pub const DEFAULT_WASM_EXECUTION_TIMEOUT_SECS: u64 = 30;

/// Maximum allowed wall-clock execution timeout (5 minutes).
///
/// Manifests may override the default up to this cap. Values above this are
/// clamped silently.
pub const MAX_WASM_EXECUTION_TIMEOUT_SECS: u64 = 300;

/// Default guest output buffer size for WASM sandbox responses (256 KB).
///
/// hyperlight-wasm defaults to 16 KB (`0x4000`) which is too small for
/// responses containing large JSON payloads (e.g., CI job results, KV scan
/// results, multi-step pipeline outputs). 256 KB covers realistic use cases
/// while keeping per-sandbox memory overhead reasonable.
///
/// If a response exceeds this limit, the guest call fails with a buffer
/// overflow error. Extremely large data should use blob storage instead.
pub const DEFAULT_WASM_GUEST_OUTPUT_BUFFER_SIZE: usize = 256 * 1024;

// Compile-time: output buffer must be at least 32KB to handle typical
// CI job results (~20KB) and KV scan payloads with headroom.
const _: () = assert!(DEFAULT_WASM_GUEST_OUTPUT_BUFFER_SIZE >= 32 * 1024, "WASM output buffer must be at least 32KB");
