//! Utility functions for system health checks and resource management.
//!
//! This module re-exports utilities from extracted crates for backward compatibility.
//!
//! # Tiger Style
//!
//! - Fixed limits (95% disk usage threshold)
//! - Fail-fast semantics for resource exhaustion
//! - Explicit error types
//! - Safe time access without panics

// Re-export time utilities from aspen-time
pub use aspen_disk::DISK_USAGE_THRESHOLD_PERCENT;
pub use aspen_disk::DiskSpace;
// Re-export disk utilities from aspen-disk
pub use aspen_disk::check_disk_space;
pub use aspen_disk::ensure_disk_space_available;
pub use aspen_time::current_time_ms;
pub use aspen_time::current_time_secs;
