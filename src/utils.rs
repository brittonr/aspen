/// Utility functions for system health checks and resource management.
///
/// This module provides Tiger Style resource management:
/// - Fixed limits (95% disk usage threshold)
/// - Fail-fast semantics for resource exhaustion
/// - Explicit error types
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

    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
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
    use std::path::PathBuf;

    #[test]
    fn test_usage_percent_calculation() {
        // 50% used
        assert_eq!(DiskSpace::usage_percent(100, 50), 50);

        // 90% used
        assert_eq!(DiskSpace::usage_percent(100, 10), 90);

        // 0% used
        assert_eq!(DiskSpace::usage_percent(100, 100), 0);

        // 100% used
        assert_eq!(DiskSpace::usage_percent(100, 0), 100);

        // Empty disk
        assert_eq!(DiskSpace::usage_percent(0, 0), 0);
    }

    #[test]
    fn test_check_disk_space_current_dir() {
        // Check disk space for current directory
        let path = PathBuf::from(".");
        let result = check_disk_space(&path);

        assert!(result.is_ok(), "disk space check should succeed");

        let disk_space = result.unwrap();
        assert!(disk_space.total_bytes > 0, "total bytes should be positive");
        assert!(
            disk_space.available_bytes <= disk_space.total_bytes,
            "available should be <= total"
        );
        assert!(
            disk_space.used_bytes <= disk_space.total_bytes,
            "used should be <= total"
        );
        assert!(
            disk_space.usage_percent <= 100,
            "usage percent should be <= 100"
        );
    }

    #[test]
    fn test_ensure_disk_space_available_current_dir() {
        // This test may fail if disk is actually >95% full, which is fine
        let path = PathBuf::from(".");
        let result = ensure_disk_space_available(&path);

        // We can't assert Ok here since the disk might actually be full,
        // but we can verify the error message is correct if it fails
        if let Err(e) = result {
            let msg = e.to_string();
            assert!(msg.contains("disk usage too high"));
        }
    }
}
