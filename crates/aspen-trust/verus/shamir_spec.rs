//! Verus formal specifications for Shamir threshold bounds.
//!
//! # Invariants Verified
//!
//! 1. **SHAMIR-1: Threshold Lower Bound**: K >= 1 (at least one share required)
//! 2. **SHAMIR-2: Threshold Upper Bound**: K <= N (can't require more shares than exist)
//! 3. **SHAMIR-3: Total Upper Bound**: N <= 255 (GF(2^8) field constraint)
//! 4. **SHAMIR-4: Default Threshold Majority**: default_threshold(n) == (n/2) + 1
//! 5. **SHAMIR-5: Default Threshold Valid**: default_threshold(n) is always valid for n

#[allow(unused_imports)]
use vstd::prelude::*;

verus! {

// ========================================================================
// Spec Functions
// ========================================================================

/// Specification: valid threshold parameters for Shamir secret sharing.
pub open spec fn valid_threshold_spec(threshold: u8, total: u8) -> bool {
    threshold >= 1 && threshold <= total && total >= 1
}

/// Specification: default majority threshold for cluster size n.
pub open spec fn default_threshold_spec(n: u32) -> u8 {
    if (n / 2) + 1 > 255 {
        255u8
    } else {
        ((n / 2) + 1) as u8
    }
}

// ========================================================================
// Exec Functions (verified implementations)
// ========================================================================

/// Check if threshold/total pair is valid.
pub fn is_valid_threshold(threshold: u8, total: u8) -> (result: bool)
    ensures result == valid_threshold_spec(threshold, total)
{
    threshold >= 1 && threshold <= total && total >= 1
}

/// Compute default majority threshold.
pub fn default_threshold_for_size(n: u32) -> (result: u8)
    ensures
        result == default_threshold_spec(n),
        result >= 1u8,
        n >= 1 ==> valid_threshold_spec(result, if n <= 255 { n as u8 } else { 255u8 }),
{
    let majority: u32 = (n / 2) + 1;
    if majority > 255 {
        255u8
    } else {
        majority as u8
    }
}

// ========================================================================
// Proofs
// ========================================================================

/// Proof: default threshold is always >= 1.
pub proof fn default_threshold_at_least_one(n: u32)
    ensures default_threshold_spec(n) >= 1u8
{
    // (n/2) + 1 >= 1 for all n >= 0
}

/// Proof: default threshold never exceeds n (when n >= 1 and n <= 255).
pub proof fn default_threshold_at_most_n(n: u32)
    requires 1 <= n && n <= 255
    ensures default_threshold_spec(n) <= n as u8
{
    // (n/2) + 1 <= n  iff  1 <= n - n/2 = ceil(n/2), true for n >= 1
}

/// Proof: valid_threshold is symmetric in its constraints.
pub proof fn threshold_validity_complete(threshold: u8, total: u8)
    ensures
        valid_threshold_spec(threshold, total) ==> threshold >= 1,
        valid_threshold_spec(threshold, total) ==> threshold <= total,
        valid_threshold_spec(threshold, total) ==> total >= 1,
{
    // Direct from definition
}

}
