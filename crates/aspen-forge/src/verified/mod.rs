//! Pure functions for Forge logic.
//!
//! These functions encapsulate deterministic computation logic that can be
//! tested independently of the async Forge code. Time-dependent calculations
//! accept explicit time parameters rather than calling system clocks.
//!
//! # Tiger Style
//!
//! - Pure functions with no side effects
//! - Deterministic: same inputs always produce same outputs
//! - Time passed as explicit parameter (no `Instant::now()` calls)
//! - Saturating arithmetic for overflow safety

mod cob_state;
mod rate_limiter;
mod ref_validation;

pub use cob_state::*;
pub use rate_limiter::*;
pub use ref_validation::*;
