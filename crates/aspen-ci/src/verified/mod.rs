//! Pure functions for CI pipeline logic.
//!
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

mod pipeline;
mod resource;
mod timeout;
mod trigger;

pub use pipeline::*;
pub use resource::*;
pub use timeout::*;
pub use trigger::*;
