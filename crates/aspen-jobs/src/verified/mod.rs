//! Pure functions for job system logic.
//!
//! These functions encapsulate deterministic computation logic that can be
//! tested independently of the async job management code. All time-dependent
//! calculations accept explicit time parameters rather than calling system
//! clocks.
//!
//! # Tiger Style
//!
//! - Pure functions with no side effects
//! - Deterministic: same inputs always produce same outputs
//! - Time passed as explicit parameter (no `Utc::now()` calls)
//! - Saturating arithmetic for overflow safety

mod dependency;
mod retry;
mod saga;
mod schedule;
mod workflow;

pub use dependency::*;
pub use retry::*;
pub use saga::*;
pub use schedule::*;
pub use workflow::*;
