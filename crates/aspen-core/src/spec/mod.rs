//! Formal verification specifications for Aspen Core
//!
//! This module contains Verus specifications that prove correctness
//! of core primitives including HLC, tuple layer, directory layer,
//! and allocator.
//!
//! # Overview
//!
//! The specs verify key invariants:
//!
//! 1. **HLC**: Monotonicity, causality, total order, bounded drift
//! 2. **Tuple Layer**: Order preservation, roundtrip, prefix property
//! 3. **Directory Layer**: Namespace isolation, prefix uniqueness, hierarchy
//! 4. **Allocator**: Uniqueness, monotonicity, counter bounded
//!
//! # Module Structure
//!
//! - `verus_shim`: Zero-cost ghost code macros for production use
//!
//! # Usage
//!
//! These specifications are compiled when the `verus` feature is enabled:
//!
//! ```bash
//! # Verify specifications
//! nix run .#verify-verus-core
//!
//! # Check ghost code compiles
//! cargo check -p aspen-core --features verus
//! ```
//!
//! # Design Rationale
//!
//! The specifications model each component as a state machine with:
//! - Abstract state representation
//! - Operation preconditions and postconditions
//! - Invariant preservation proofs

pub mod verus_shim;

// Re-export verus shims for ghost code in production
pub use verus_shim::*;
