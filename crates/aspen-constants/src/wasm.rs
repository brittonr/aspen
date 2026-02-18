//! WASM Component execution resource bounds.
//!
//! Tiger Style: Fixed limits for WASM sandbox execution to prevent
//! resource exhaustion. All WASM components run under these constraints.

/// Maximum WASM component size in bytes (50 MB).
pub const MAX_WASM_COMPONENT_SIZE: u64 = 50 * 1024 * 1024;

/// Default fuel limit for WASM execution (~10-100s of compute).
pub const DEFAULT_WASM_FUEL_LIMIT: u64 = 100_000_000;

/// Maximum allowed fuel limit.
pub const MAX_WASM_FUEL_LIMIT: u64 = 10_000_000_000;

/// Fuel yield interval for cooperative scheduling (Lunatic pattern).
pub const WASM_FUEL_YIELD_INTERVAL: u64 = 100_000;

/// Default memory limit for WASM execution (256 MB).
pub const DEFAULT_WASM_MEMORY_LIMIT: u64 = 256 * 1024 * 1024;

/// Maximum memory limit for WASM execution (1 GB).
pub const MAX_WASM_MEMORY_LIMIT: u64 = 1024 * 1024 * 1024;

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
