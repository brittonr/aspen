//! Verus formal verification specs for aspen-commit-dag.
//!
//! # Invariants Verified
//!
//! ## Commit Hash
//!
//! 1. **COMMIT-1: Determinism**: Same inputs produce same CommitId
//! 2. **COMMIT-2: Chain Continuity**: CommitId depends on parent hash
//! 3. **COMMIT-3: Tamper Detection**: Modified mutations produce different mutations_hash
//! 4. **COMMIT-4: Sort Invariant**: mutations_hash input is always sorted by key
//!
//! ## Diff
//!
//! 1. **DIFF-1: Sorted Output**: diff output is sorted by key
//! 2. **DIFF-2: No Phantom Entries**: every DiffEntry corresponds to a real difference
//!
//! # Trusted Axioms
//!
//! - BLAKE3 is collision resistant (modeled as uninterpreted)
//! - BLAKE3 is deterministic: same input → same output
//! - BLAKE3 output is always 32 bytes

mod commit_hash_spec;
mod diff_spec;
