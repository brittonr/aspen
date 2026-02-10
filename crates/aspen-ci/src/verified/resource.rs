//! Pure resource limit calculation functions.
//!
//! These functions compute resource limits for CI jobs without
//! accessing system resources.
//!
//! # Tiger Style
//!
//! - Pure calculation functions
//! - Explicit bounds checking
//! - No I/O or system calls

/// Compute effective memory limit for a job.
///
/// Takes the requested limit and caps it at the maximum allowed.
///
/// # Arguments
///
/// * `requested_bytes` - Requested memory limit (if any)
/// * `max_bytes` - Maximum allowed memory limit
/// * `default_bytes` - Default memory limit if not specified
///
/// # Returns
///
/// The effective memory limit in bytes.
///
/// # Example
///
/// ```
/// use aspen_ci::verified::compute_effective_memory_limit;
///
/// const GB: u64 = 1024 * 1024 * 1024;
///
/// // Requested within limits
/// assert_eq!(compute_effective_memory_limit(Some(2 * GB), 4 * GB, GB), 2 * GB);
///
/// // Requested exceeds max, capped
/// assert_eq!(compute_effective_memory_limit(Some(8 * GB), 4 * GB, GB), 4 * GB);
///
/// // Not specified, use default
/// assert_eq!(compute_effective_memory_limit(None, 4 * GB, GB), GB);
/// ```
#[inline]
pub const fn compute_effective_memory_limit(requested_bytes: Option<u64>, max_bytes: u64, default_bytes: u64) -> u64 {
    match requested_bytes {
        Some(requested) => {
            if requested > max_bytes {
                max_bytes
            } else {
                requested
            }
        }
        None => default_bytes,
    }
}

/// Compute effective CPU weight for a job.
///
/// CPU weight determines relative CPU priority (higher = more CPU time).
/// Normal system processes typically have weight 100.
///
/// # Arguments
///
/// * `requested_weight` - Requested CPU weight (if any)
/// * `max_weight` - Maximum allowed weight
/// * `default_weight` - Default weight (usually 50 for CI jobs)
///
/// # Returns
///
/// The effective CPU weight.
///
/// # Example
///
/// ```
/// use aspen_ci::verified::compute_effective_cpu_weight;
///
/// // Requested within limits
/// assert_eq!(compute_effective_cpu_weight(Some(75), 100, 50), 75);
///
/// // Requested exceeds max, capped
/// assert_eq!(compute_effective_cpu_weight(Some(150), 100, 50), 100);
///
/// // Not specified, use default
/// assert_eq!(compute_effective_cpu_weight(None, 100, 50), 50);
/// ```
#[inline]
pub const fn compute_effective_cpu_weight(requested_weight: Option<u32>, max_weight: u32, default_weight: u32) -> u32 {
    match requested_weight {
        Some(requested) => {
            if requested > max_weight {
                max_weight
            } else {
                requested
            }
        }
        None => default_weight,
    }
}

/// Compute effective process (PID) limit for a job.
///
/// # Arguments
///
/// * `requested_pids` - Requested PID limit (if any)
/// * `max_pids` - Maximum allowed PIDs
/// * `default_pids` - Default PID limit
///
/// # Returns
///
/// The effective PID limit.
///
/// # Example
///
/// ```
/// use aspen_ci::verified::compute_effective_pid_limit;
///
/// // Requested within limits
/// assert_eq!(compute_effective_pid_limit(Some(2048), 4096, 1024), 2048);
///
/// // Requested exceeds max, capped
/// assert_eq!(compute_effective_pid_limit(Some(8192), 4096, 1024), 4096);
///
/// // Not specified, use default
/// assert_eq!(compute_effective_pid_limit(None, 4096, 1024), 1024);
/// ```
#[inline]
pub const fn compute_effective_pid_limit(requested_pids: Option<u32>, max_pids: u32, default_pids: u32) -> u32 {
    match requested_pids {
        Some(requested) => {
            if requested > max_pids {
                max_pids
            } else {
                requested
            }
        }
        None => default_pids,
    }
}

/// Compute memory high watermark for throttling.
///
/// The "high" watermark is a soft limit that triggers throttling before
/// the hard limit is reached. Typically set to a percentage of the max.
///
/// # Arguments
///
/// * `max_bytes` - Hard memory limit in bytes
/// * `high_percentage` - Percentage for high watermark (0-100)
///
/// # Returns
///
/// The high watermark in bytes.
///
/// # Example
///
/// ```
/// use aspen_ci::verified::compute_memory_high_watermark;
///
/// const GB: u64 = 1024 * 1024 * 1024;
///
/// // 75% of 4 GB = 3 GB
/// assert_eq!(compute_memory_high_watermark(4 * GB, 75), 3 * GB);
///
/// // 100% equals max
/// assert_eq!(compute_memory_high_watermark(4 * GB, 100), 4 * GB);
/// ```
#[inline]
pub const fn compute_memory_high_watermark(max_bytes: u64, high_percentage: u32) -> u64 {
    let percentage = if high_percentage > 100 { 100 } else { high_percentage };
    (max_bytes / 100) * percentage as u64
}

/// Check if job resource limits are valid.
///
/// # Arguments
///
/// * `memory_bytes` - Requested memory in bytes
/// * `pids` - Requested PID limit
/// * `min_memory_bytes` - Minimum allowed memory
/// * `max_memory_bytes` - Maximum allowed memory
/// * `max_pids` - Maximum allowed PIDs
///
/// # Returns
///
/// `true` if the limits are valid.
///
/// # Example
///
/// ```
/// use aspen_ci::verified::are_resource_limits_valid;
///
/// const GB: u64 = 1024 * 1024 * 1024;
/// const MB: u64 = 1024 * 1024;
///
/// // Valid limits
/// assert!(are_resource_limits_valid(2 * GB, 2048, 256 * MB, 4 * GB, 4096));
///
/// // Memory too low
/// assert!(!are_resource_limits_valid(128 * MB, 2048, 256 * MB, 4 * GB, 4096));
///
/// // PIDs too high
/// assert!(!are_resource_limits_valid(2 * GB, 8192, 256 * MB, 4 * GB, 4096));
/// ```
#[inline]
pub const fn are_resource_limits_valid(
    memory_bytes: u64,
    pids: u32,
    min_memory_bytes: u64,
    max_memory_bytes: u64,
    max_pids: u32,
) -> bool {
    memory_bytes >= min_memory_bytes && memory_bytes <= max_memory_bytes && pids <= max_pids
}

/// Format bytes as a human-readable string for cgroup configuration.
///
/// Returns the value in bytes as a string (cgroups use raw bytes).
///
/// # Arguments
///
/// * `bytes` - Memory size in bytes
///
/// # Returns
///
/// String representation of bytes.
///
/// # Example
///
/// ```
/// use aspen_ci::verified::format_bytes_for_cgroup;
///
/// assert_eq!(format_bytes_for_cgroup(4294967296), "4294967296");
/// ```
#[inline]
pub fn format_bytes_for_cgroup(bytes: u64) -> String {
    bytes.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const GB: u64 = 1024 * 1024 * 1024;
    const MB: u64 = 1024 * 1024;

    // ========================================================================
    // compute_effective_memory_limit tests
    // ========================================================================

    #[test]
    fn test_memory_within_limits() {
        assert_eq!(compute_effective_memory_limit(Some(2 * GB), 4 * GB, GB), 2 * GB);
    }

    #[test]
    fn test_memory_exceeds_max() {
        assert_eq!(compute_effective_memory_limit(Some(8 * GB), 4 * GB, GB), 4 * GB);
    }

    #[test]
    fn test_memory_default() {
        assert_eq!(compute_effective_memory_limit(None, 4 * GB, GB), GB);
    }

    // ========================================================================
    // compute_effective_cpu_weight tests
    // ========================================================================

    #[test]
    fn test_cpu_within_limits() {
        assert_eq!(compute_effective_cpu_weight(Some(75), 100, 50), 75);
    }

    #[test]
    fn test_cpu_exceeds_max() {
        assert_eq!(compute_effective_cpu_weight(Some(150), 100, 50), 100);
    }

    #[test]
    fn test_cpu_default() {
        assert_eq!(compute_effective_cpu_weight(None, 100, 50), 50);
    }

    // ========================================================================
    // compute_effective_pid_limit tests
    // ========================================================================

    #[test]
    fn test_pids_within_limits() {
        assert_eq!(compute_effective_pid_limit(Some(2048), 4096, 1024), 2048);
    }

    #[test]
    fn test_pids_exceeds_max() {
        assert_eq!(compute_effective_pid_limit(Some(8192), 4096, 1024), 4096);
    }

    #[test]
    fn test_pids_default() {
        assert_eq!(compute_effective_pid_limit(None, 4096, 1024), 1024);
    }

    // ========================================================================
    // compute_memory_high_watermark tests
    // ========================================================================

    #[test]
    fn test_watermark_75_percent() {
        assert_eq!(compute_memory_high_watermark(4 * GB, 75), 3 * GB);
    }

    #[test]
    fn test_watermark_100_percent() {
        assert_eq!(compute_memory_high_watermark(4 * GB, 100), 4 * GB);
    }

    #[test]
    fn test_watermark_over_100() {
        // Capped at 100%
        assert_eq!(compute_memory_high_watermark(4 * GB, 150), 4 * GB);
    }

    // ========================================================================
    // are_resource_limits_valid tests
    // ========================================================================

    #[test]
    fn test_valid_limits() {
        assert!(are_resource_limits_valid(2 * GB, 2048, 256 * MB, 4 * GB, 4096));
    }

    #[test]
    fn test_memory_too_low() {
        assert!(!are_resource_limits_valid(128 * MB, 2048, 256 * MB, 4 * GB, 4096));
    }

    #[test]
    fn test_memory_too_high() {
        assert!(!are_resource_limits_valid(8 * GB, 2048, 256 * MB, 4 * GB, 4096));
    }

    #[test]
    fn test_pids_too_high() {
        assert!(!are_resource_limits_valid(2 * GB, 8192, 256 * MB, 4 * GB, 4096));
    }

    // ========================================================================
    // format_bytes_for_cgroup tests
    // ========================================================================

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes_for_cgroup(4 * GB), "4294967296");
    }
}
