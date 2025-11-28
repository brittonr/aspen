//! Production-ready timestamp handling utilities for Blixard.
//!
//! This module provides robust handling of SystemTime and monotonic timestamps
//! for distributed systems, based on patterns used by tokio-console and other
//! production Rust systems.
//!
//! Key concepts:
//! - Use `Instant` for measuring elapsed time (monotonic, never goes backwards)
//! - Use `SystemTime` anchored to `Instant` for serializable timestamps (clock-skew resistant)
//! - Use `HybridLogicalClock` for ordering events in distributed systems

use std::time::{SystemTime, UNIX_EPOCH};

pub mod anchor;
pub mod validator;
pub mod hlc;

pub use anchor::TimeAnchor;
pub use validator::{TimestampValidator, SequenceValidator};
pub use hlc::HybridLogicalClock;

/// Get the current Unix timestamp in seconds.
///
/// This uses the system clock and may fail if clock goes backwards.
/// For production code, consider using `TimeAnchor::now()` instead.
pub fn unix_timestamp_secs() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|e| format!("Clock error: {}", e))
}

/// Get the current Unix timestamp in milliseconds.
///
/// This uses the system clock and may fail if clock goes backwards.
pub fn unix_timestamp_millis() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .map_err(|e| format!("Clock error: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_timestamp_increases() {
        let t1 = unix_timestamp_secs().unwrap();
        std::thread::sleep(Duration::from_millis(10));
        let t2 = unix_timestamp_secs().unwrap();
        assert!(t2 >= t1);
    }
}
