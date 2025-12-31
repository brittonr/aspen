//! Public API constants for Aspen operations.
//!
//! This module re-exports constants from the aspen-constants crate for
//! backward compatibility and provides a public API for external consumers.
//!
//! Tiger Style: Constants are fixed and immutable, enforced at compile time.
//! Each constant has explicit bounds to prevent unbounded resource allocation.

// ============================================================================
// Re-exports from aspen-constants
// ============================================================================
// All constants have been centralized in the aspen-constants crate.
// This module provides backward compatibility re-exports.

pub use aspen_constants::*;
