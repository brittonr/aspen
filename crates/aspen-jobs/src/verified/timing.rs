//! Pure functions for build phase timing aggregation.
//!
//! Formally verified — see `verus/timing_spec.rs` for proofs.
//! Uses saturating arithmetic to prevent overflow.

/// Aggregated phase timing totals for a worker.
#[derive(Debug, Clone, Copy, Default)]
pub struct PhaseTimingTotals {
    /// Total import time across all builds.
    pub total_import_ms: u64,
    /// Total build time across all builds.
    pub total_build_ms: u64,
    /// Total upload time across all builds.
    pub total_upload_ms: u64,
}

/// Aggregate a new build's phase timings into running totals.
///
/// Uses saturating addition to prevent overflow.
#[inline]
pub fn aggregate_phase_timings(
    current: PhaseTimingTotals,
    import_ms: u64,
    build_ms: u64,
    upload_ms: u64,
) -> PhaseTimingTotals {
    PhaseTimingTotals {
        total_import_ms: current.total_import_ms.saturating_add(import_ms),
        total_build_ms: current.total_build_ms.saturating_add(build_ms),
        total_upload_ms: current.total_upload_ms.saturating_add(upload_ms),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aggregate_zero_base() {
        let result = aggregate_phase_timings(PhaseTimingTotals::default(), 100, 200, 50);
        assert_eq!(result.total_import_ms, 100);
        assert_eq!(result.total_build_ms, 200);
        assert_eq!(result.total_upload_ms, 50);
    }

    #[test]
    fn test_aggregate_accumulates() {
        let first = aggregate_phase_timings(PhaseTimingTotals::default(), 100, 200, 50);
        let second = aggregate_phase_timings(first, 150, 300, 75);
        assert_eq!(second.total_import_ms, 250);
        assert_eq!(second.total_build_ms, 500);
        assert_eq!(second.total_upload_ms, 125);
    }

    #[test]
    fn test_aggregate_saturates_on_overflow() {
        let near_max = PhaseTimingTotals {
            total_import_ms: u64::MAX - 10,
            total_build_ms: u64::MAX - 5,
            total_upload_ms: u64::MAX,
        };
        let result = aggregate_phase_timings(near_max, 100, 100, 100);
        assert_eq!(result.total_import_ms, u64::MAX);
        assert_eq!(result.total_build_ms, u64::MAX);
        assert_eq!(result.total_upload_ms, u64::MAX);
    }
}
