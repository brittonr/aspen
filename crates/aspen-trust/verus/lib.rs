//! Verus formal specifications for Aspen trust primitives.
//!
//! # Invariants Verified
//!
//! 1. **SHAMIR-1: Threshold Lower Bound**: K >= 1 (at least one share required)
//! 2. **SHAMIR-2: Threshold Upper Bound**: K <= N (can't require more shares than exist)
//! 3. **SHAMIR-3: Total Upper Bound**: N <= 255 (GF(2^8) field constraint)
//! 4. **SHAMIR-4: Default Threshold Majority**: default_threshold(n) == (n/2) + 1
//! 5. **SHAMIR-5: Default Threshold Valid**: default_threshold(n) is always valid for n
//! 6. **CHAIN-1: Prior Secret Count**: a chain at epoch N contains exactly N-1 prior secrets

mod shamir_spec;
mod chain_spec;
