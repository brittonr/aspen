//! Pure snapshot decision functions.
//!
//! Formally verified — see `verus/snapshot_spec.rs` for proofs.

/// Memory pressure levels matching `MemoryWatcher` in `aspen-cluster`.
///
/// 0 = Normal (usage < 80%)
/// 1 = Warning (usage >= 80%)
/// 2 = Critical (usage >= 90%)
pub const PRESSURE_NORMAL: u8 = 0;
pub const PRESSURE_WARNING: u8 = 1;
pub const PRESSURE_CRITICAL: u8 = 2;

/// Check if the golden snapshot should be invalidated after consecutive
/// restore failures.
///
/// Returns `true` when the failure count reaches the configured threshold.
#[inline]
pub fn should_invalidate_snapshot(restore_failures_consecutive: u32, max_restore_failures: u32) -> bool {
    restore_failures_consecutive >= max_restore_failures
}

/// Check if a snapshot restore should be allowed given memory pressure.
///
/// At Critical pressure, no new restores are allowed.
/// At Warning or Normal, restores proceed.
#[inline]
pub fn should_allow_restore(pressure_level: u8, _active_fork_count: u32) -> bool {
    pressure_level < PRESSURE_CRITICAL
}

/// Compute the effective fork count for speculative execution, adjusted
/// for memory pressure.
///
/// - Normal: use requested count (capped at max)
/// - Warning: halve the requested count (minimum 1)
/// - Critical: reduce to 1 (no speculation)
#[inline]
pub fn compute_adaptive_fork_count(requested_count: u32, max_count: u32, pressure_level: u8) -> u32 {
    let count = match pressure_level {
        PRESSURE_CRITICAL => 1,
        PRESSURE_WARNING => (requested_count / 2).max(1),
        _ => requested_count,
    };
    count.min(max_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- should_invalidate_snapshot ---

    #[test]
    fn test_no_failures_does_not_invalidate() {
        assert!(!should_invalidate_snapshot(0, 3));
    }

    #[test]
    fn test_below_threshold_does_not_invalidate() {
        assert!(!should_invalidate_snapshot(2, 3));
    }

    #[test]
    fn test_at_threshold_invalidates() {
        assert!(should_invalidate_snapshot(3, 3));
    }

    #[test]
    fn test_above_threshold_invalidates() {
        assert!(should_invalidate_snapshot(5, 3));
    }

    #[test]
    fn test_threshold_one_invalidates_on_first_failure() {
        assert!(should_invalidate_snapshot(1, 1));
    }

    #[test]
    fn test_threshold_zero_always_invalidates() {
        assert!(should_invalidate_snapshot(0, 0));
    }

    // --- should_allow_restore ---

    #[test]
    fn test_allow_restore_at_normal() {
        assert!(should_allow_restore(PRESSURE_NORMAL, 0));
        assert!(should_allow_restore(PRESSURE_NORMAL, 5));
    }

    #[test]
    fn test_allow_restore_at_warning() {
        assert!(should_allow_restore(PRESSURE_WARNING, 0));
        assert!(should_allow_restore(PRESSURE_WARNING, 5));
    }

    #[test]
    fn test_block_restore_at_critical() {
        assert!(!should_allow_restore(PRESSURE_CRITICAL, 0));
        assert!(!should_allow_restore(PRESSURE_CRITICAL, 5));
    }

    // --- compute_adaptive_fork_count ---

    #[test]
    fn test_fork_count_normal_pressure() {
        assert_eq!(compute_adaptive_fork_count(4, 8, PRESSURE_NORMAL), 4);
    }

    #[test]
    fn test_fork_count_normal_capped_by_max() {
        assert_eq!(compute_adaptive_fork_count(10, 8, PRESSURE_NORMAL), 8);
    }

    #[test]
    fn test_fork_count_warning_halved() {
        assert_eq!(compute_adaptive_fork_count(4, 8, PRESSURE_WARNING), 2);
    }

    #[test]
    fn test_fork_count_warning_minimum_one() {
        assert_eq!(compute_adaptive_fork_count(1, 8, PRESSURE_WARNING), 1);
    }

    #[test]
    fn test_fork_count_critical_always_one() {
        assert_eq!(compute_adaptive_fork_count(4, 8, PRESSURE_CRITICAL), 1);
        assert_eq!(compute_adaptive_fork_count(1, 8, PRESSURE_CRITICAL), 1);
    }

    #[test]
    fn test_fork_count_zero_requested() {
        // Zero requested at normal = 0
        assert_eq!(compute_adaptive_fork_count(0, 8, PRESSURE_NORMAL), 0);
        // Zero requested at warning = max(0/2, 1) = 1
        assert_eq!(compute_adaptive_fork_count(0, 8, PRESSURE_WARNING), 1);
        // Zero requested at critical = 1
        assert_eq!(compute_adaptive_fork_count(0, 8, PRESSURE_CRITICAL), 1);
    }

    #[test]
    fn test_fork_count_max_zero() {
        // Max 0 caps everything to 0
        assert_eq!(compute_adaptive_fork_count(4, 0, PRESSURE_NORMAL), 0);
    }
}
