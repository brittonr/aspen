//! Pure functions for CI pipeline logic.
//!
//! This module re-exports verified pure functions from `aspen-ci-core::verified`.
//! These functions encapsulate deterministic computation logic that can be
//! tested independently of the async orchestration code. Time-dependent
//! calculations accept explicit time parameters.
//!
//! # Tiger Style
//!
//! - Pure functions with no side effects
//! - Deterministic: same inputs always produce same outputs
//! - Time passed as explicit parameter
//! - Saturating arithmetic for overflow safety

// Re-export everything from aspen-ci-core::verified
pub use aspen_ci_core::verified::*;
