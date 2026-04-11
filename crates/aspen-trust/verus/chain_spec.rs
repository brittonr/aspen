//! Verus formal specifications for encrypted secret chain cardinality.
//!
//! # Invariants Verified
//!
//! 1. **CHAIN-1: Prior Secret Count**: a chain at epoch N contains exactly N-1 prior secrets

#[allow(unused_imports)]
use vstd::prelude::*;

verus! {

/// Specification: the expected number of prior secrets stored in a chain.
pub open spec fn expected_prior_secret_count(epoch: u64) -> int {
    if epoch == 0 {
        0int
    } else {
        epoch as int - 1
    }
}

/// Specification: chain cardinality matches the epoch-derived count.
pub open spec fn chain_length_invariant(epoch: u64, prior_count: u32) -> bool {
    epoch >= 1 && expected_prior_secret_count(epoch) <= u32::MAX as int ==> prior_count as int == expected_prior_secret_count(epoch)
}

/// Compute the expected prior secret count for an epoch.
pub fn compute_prior_secret_count(epoch: u64) -> (result: u32)
    requires
        epoch >= 1,
        expected_prior_secret_count(epoch) <= u32::MAX as int,
    ensures
        result as int == expected_prior_secret_count(epoch),
        result as int == epoch as int - 1,
{
    (epoch - 1) as u32
}

/// Check whether a chain's prior-count field matches the epoch.
pub fn chain_prior_count_matches_epoch(epoch: u64, prior_count: u32) -> (result: bool)
    requires
        epoch >= 1,
        expected_prior_secret_count(epoch) <= u32::MAX as int,
    ensures result == (prior_count as int == expected_prior_secret_count(epoch))
{
    prior_count == (epoch - 1) as u32
}

/// Proof: the computed prior count is always one less than the epoch.
pub proof fn prior_secret_count_is_epoch_minus_one(epoch: u64)
    requires epoch >= 1
    ensures expected_prior_secret_count(epoch) == epoch as int - 1
{
}

/// Proof: every representable epoch yields a chain length satisfying the invariant.
pub proof fn computed_count_satisfies_invariant(epoch: u64)
    requires
        epoch >= 1,
        expected_prior_secret_count(epoch) <= u32::MAX as int,
    ensures chain_length_invariant(epoch, (epoch - 1) as u32)
{
}

}
