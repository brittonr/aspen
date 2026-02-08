//! Constants for pure function limits.
//!
//! These are duplicated from aspen-core to avoid circular dependencies.
//! Tiger Style: Fixed limits prevent unbounded resource allocation.

/// Maximum number of keys that can be returned in a single scan.
pub const MAX_SCAN_RESULTS: u32 = 10_000;

/// Default number of keys returned in a scan if limit is not specified.
pub const DEFAULT_SCAN_LIMIT: u32 = 1_000;
