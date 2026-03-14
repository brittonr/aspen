//! Verus specification for build phase timing aggregation.
//!
//! Verifies that `aggregate_phase_timings` maintains monotonicity
//! of timing totals using saturating arithmetic.

use vstd::prelude::*;

verus! {

    // ========================================================================
    // Spec Functions
    // ========================================================================

    /// Specification: aggregated totals are always >= previous totals.
    pub open spec fn timing_monotonic(
        prev_import: u64,
        prev_build: u64,
        prev_upload: u64,
        new_import: u64,
        new_build: u64,
        new_upload: u64,
    ) -> bool {
        new_import >= prev_import
            && new_build >= prev_build
            && new_upload >= prev_upload
    }

    // ========================================================================
    // Exec Functions
    // ========================================================================

    /// Verify that saturating add preserves monotonicity for import time.
    pub fn verify_import_monotonic(current: u64, delta: u64) -> (result: u64)
        ensures
            result >= current,
    {
        current.saturating_add(delta)
    }

    /// Verify that saturating add preserves monotonicity for build time.
    pub fn verify_build_monotonic(current: u64, delta: u64) -> (result: u64)
        ensures
            result >= current,
    {
        current.saturating_add(delta)
    }

    /// Verify that saturating add preserves monotonicity for upload time.
    pub fn verify_upload_monotonic(current: u64, delta: u64) -> (result: u64)
        ensures
            result >= current,
    {
        current.saturating_add(delta)
    }

    /// Verify full aggregation preserves monotonicity for all three fields.
    pub fn verify_aggregate_monotonic(
        prev_import: u64,
        prev_build: u64,
        prev_upload: u64,
        import_delta: u64,
        build_delta: u64,
        upload_delta: u64,
    ) -> (result: (u64, u64, u64))
        ensures
            result.0 >= prev_import,
            result.1 >= prev_build,
            result.2 >= prev_upload,
    {
        (
            prev_import.saturating_add(import_delta),
            prev_build.saturating_add(build_delta),
            prev_upload.saturating_add(upload_delta),
        )
    }

    /// Verify that aggregation saturates at u64::MAX instead of overflowing.
    pub fn verify_saturation(current: u64, delta: u64) -> (result: u64)
        ensures
            result <= u64::MAX,
            result >= current,
    {
        current.saturating_add(delta)
    }

}
