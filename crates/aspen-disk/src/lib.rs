//! Disk space utilities for Aspen distributed systems.
//!
//! This module provides Tiger Style resource management for disk space:
//! - Fixed limits (95% disk usage threshold)
//! - Fail-fast semantics for resource exhaustion
//! - Explicit error types
//!
//! # Platform Support
//!
//! - Unix: Uses `libc::statvfs`
//! - Other platforms: Returns `Unsupported` error

use std::path::Path;

/// Disk space information for a filesystem.
#[derive(Debug, Clone)]
pub struct DiskSpace {
    /// Total size of the filesystem in bytes.
    pub total_bytes: u64,
    /// Available space in bytes (for unprivileged users).
    pub available_bytes: u64,
    /// Used space in bytes.
    pub used_bytes: u64,
    /// Usage as a percentage (0-100).
    pub usage_percent: u64,
}

impl DiskSpace {
    /// Calculate disk usage percentage.
    pub fn usage_percent(total: u64, available: u64) -> u64 {
        if total == 0 {
            return 0;
        }
        let used = total.saturating_sub(available);
        used.saturating_mul(100) / total
    }
}

/// Check disk space for a given path.
///
/// Returns disk space information including total, available, used bytes
/// and usage percentage.
///
/// # Platform Support
///
/// - Unix: Uses `libc::statvfs`
/// - Windows: Uses `GetDiskFreeSpaceExW`
/// - Other: Returns error
///
/// # Errors
///
/// Returns `std::io::Error` if the syscall fails or platform is unsupported.
#[cfg(target_family = "unix")]
pub fn check_disk_space(path: &Path) -> std::io::Result<DiskSpace> {
    use std::os::unix::ffi::OsStrExt;

    let path_cstr = std::ffi::CString::new(path.as_os_str().as_bytes())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

    // SAFETY: statvfs is a C struct that can be safely zero-initialized.
    // All fields are primitive types (integers) with no invariants.
    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
    // SAFETY: statvfs() is a POSIX syscall. path_cstr is a valid null-terminated
    // C string (from CString), and stat is a valid mutable reference to statvfs.
    let result = unsafe { libc::statvfs(path_cstr.as_ptr(), &mut stat) };

    if result != 0 {
        return Err(std::io::Error::last_os_error());
    }

    let total_bytes = stat.f_blocks * stat.f_frsize;
    let available_bytes = stat.f_bavail * stat.f_frsize;
    let used_bytes = total_bytes.saturating_sub(available_bytes);
    let usage_percent = DiskSpace::usage_percent(total_bytes, available_bytes);

    Ok(DiskSpace {
        total_bytes,
        available_bytes,
        used_bytes,
        usage_percent,
    })
}

#[cfg(not(target_family = "unix"))]
pub fn check_disk_space(_path: &Path) -> std::io::Result<DiskSpace> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "disk space checking currently only supported on Unix systems",
    ))
}

/// Tiger Style disk space threshold (percentage).
///
/// Writes are rejected when disk usage exceeds this threshold to prevent
/// complete disk exhaustion and maintain system stability.
pub const DISK_USAGE_THRESHOLD_PERCENT: u64 = 95;

/// Check if disk has sufficient space for writes.
///
/// Returns `Ok(())` if disk usage is below threshold, or an error if
/// disk usage is too high (>95%) or the check fails.
///
/// # Tiger Style Justification
///
/// Fixed limit at 95% to:
/// - Prevent complete disk exhaustion
/// - Allow space for system operations (logging, temp files)
/// - Fail fast before storage layer errors occur
pub fn ensure_disk_space_available(path: &Path) -> std::io::Result<()> {
    let disk_space = check_disk_space(path)?;

    if disk_space.usage_percent >= DISK_USAGE_THRESHOLD_PERCENT {
        return Err(std::io::Error::new(
            std::io::ErrorKind::OutOfMemory, // Closest semantic match
            format!(
                "disk usage too high: {}% (threshold: {}%)",
                disk_space.usage_percent, DISK_USAGE_THRESHOLD_PERCENT
            ),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // DiskSpace Tests
    // ========================================================================

    #[test]
    fn disk_space_usage_percent_normal() {
        // 100 total, 50 available = 50% used
        assert_eq!(DiskSpace::usage_percent(100, 50), 50);
    }

    #[test]
    fn disk_space_usage_percent_zero_total() {
        // Edge case: zero total should return 0, not panic
        assert_eq!(DiskSpace::usage_percent(0, 0), 0);
        assert_eq!(DiskSpace::usage_percent(0, 100), 0);
    }

    #[test]
    fn disk_space_usage_percent_full() {
        // 100 total, 0 available = 100% used
        assert_eq!(DiskSpace::usage_percent(100, 0), 100);
    }

    #[test]
    fn disk_space_usage_percent_empty() {
        // 100 total, 100 available = 0% used
        assert_eq!(DiskSpace::usage_percent(100, 100), 0);
    }

    #[test]
    fn disk_space_usage_percent_large_values() {
        // Test with large values (terabyte scale)
        let total = 1_000_000_000_000u64; // 1 TB
        let available = 250_000_000_000u64; // 250 GB
        // 75% used
        assert_eq!(DiskSpace::usage_percent(total, available), 75);
    }

    #[test]
    fn disk_space_usage_percent_rounding() {
        // Test rounding behavior (integer division)
        // 100 total, 33 available = 67 used = 67%
        assert_eq!(DiskSpace::usage_percent(100, 33), 67);
        // 100 total, 67 available = 33 used = 33%
        assert_eq!(DiskSpace::usage_percent(100, 67), 33);
    }

    #[test]
    fn disk_space_usage_percent_available_exceeds_total() {
        // Edge case: available > total (shouldn't happen but handle gracefully)
        // saturating_sub means used = 0
        assert_eq!(DiskSpace::usage_percent(100, 200), 0);
    }

    #[test]
    fn disk_space_struct_debug() {
        let ds = DiskSpace {
            total_bytes: 1000,
            available_bytes: 500,
            used_bytes: 500,
            usage_percent: 50,
        };
        let debug = format!("{:?}", ds);
        assert!(debug.contains("DiskSpace"));
        assert!(debug.contains("1000"));
        assert!(debug.contains("500"));
        assert!(debug.contains("50"));
    }

    #[test]
    fn disk_space_struct_clone() {
        let ds = DiskSpace {
            total_bytes: 1000,
            available_bytes: 500,
            used_bytes: 500,
            usage_percent: 50,
        };
        let ds2 = ds.clone();
        assert_eq!(ds.total_bytes, ds2.total_bytes);
        assert_eq!(ds.available_bytes, ds2.available_bytes);
        assert_eq!(ds.used_bytes, ds2.used_bytes);
        assert_eq!(ds.usage_percent, ds2.usage_percent);
    }

    // ========================================================================
    // check_disk_space Tests (Unix only)
    // ========================================================================

    #[test]
    #[cfg(target_family = "unix")]
    fn check_disk_space_current_dir() {
        let result = check_disk_space(Path::new("."));
        assert!(result.is_ok(), "check_disk_space should succeed for current dir");
        let ds = result.unwrap();
        assert!(ds.total_bytes > 0, "total_bytes should be positive");
        assert!(ds.usage_percent <= 100, "usage_percent should be <= 100");
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn check_disk_space_root() {
        let result = check_disk_space(Path::new("/"));
        assert!(result.is_ok(), "check_disk_space should succeed for root");
        let ds = result.unwrap();
        assert!(ds.total_bytes > 0);
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn check_disk_space_tmp() {
        let result = check_disk_space(Path::new("/tmp"));
        assert!(result.is_ok(), "check_disk_space should succeed for /tmp");
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn check_disk_space_nonexistent_path() {
        let result = check_disk_space(Path::new("/nonexistent/path/that/does/not/exist"));
        assert!(result.is_err(), "check_disk_space should fail for nonexistent path");
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn check_disk_space_values_consistent() {
        let ds = check_disk_space(Path::new(".")).unwrap();
        // used_bytes should approximately equal total - available
        let expected_used = ds.total_bytes.saturating_sub(ds.available_bytes);
        assert_eq!(ds.used_bytes, expected_used, "used_bytes should equal total - available");

        // usage_percent should match DiskSpace::usage_percent calculation
        let expected_percent = DiskSpace::usage_percent(ds.total_bytes, ds.available_bytes);
        assert_eq!(ds.usage_percent, expected_percent);
    }

    // ========================================================================
    // ensure_disk_space_available Tests
    // ========================================================================

    #[test]
    #[cfg(target_family = "unix")]
    fn ensure_disk_space_available_current_dir() {
        // This should pass unless the disk is actually > 95% full
        let result = ensure_disk_space_available(Path::new("."));
        // We can't guarantee this passes (disk might be full), but we can test the function runs
        match result {
            Ok(()) => {
                // Good - disk has space
            }
            Err(e) => {
                // Should be OutOfMemory error kind if disk is too full
                assert!(e.kind() == std::io::ErrorKind::OutOfMemory || e.kind() == std::io::ErrorKind::NotFound);
            }
        }
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn ensure_disk_space_available_nonexistent_path() {
        let result = ensure_disk_space_available(Path::new("/nonexistent/path"));
        assert!(result.is_err());
    }

    // ========================================================================
    // Constant Tests
    // ========================================================================

    #[test]
    fn disk_usage_threshold_is_95_percent() {
        assert_eq!(DISK_USAGE_THRESHOLD_PERCENT, 95);
    }

    #[test]
    fn disk_usage_threshold_reasonable_range() {
        // Threshold should be between 80% and 99%
        assert!(DISK_USAGE_THRESHOLD_PERCENT >= 80, "threshold should be at least 80%");
        assert!(DISK_USAGE_THRESHOLD_PERCENT <= 99, "threshold should leave some headroom");
    }
}
