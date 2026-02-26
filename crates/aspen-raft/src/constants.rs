//! Centralized constants for the Raft module.
//!
//! This module re-exports constants from the aspen-constants crate for backward
//! compatibility within the raft module and its dependents.
//!
//! Tiger Style: Constants are fixed and immutable, enforced at compile time.
//! Each constant has explicit bounds to prevent unbounded resource allocation.

// ============================================================================
// Re-exports from aspen-constants (all constants centralized)
// ============================================================================
// These constants are re-exported for backward compatibility within the raft
// module and its dependents. New code should import from aspen_core directly.

pub use aspen_core::*;
