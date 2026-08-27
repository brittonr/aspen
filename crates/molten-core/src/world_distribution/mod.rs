//! World-specific closure, claim, replication, and retention policy.
//!
//! Generic DAG synchronization, content replication, and Artifact Binding keep
//! their mechanism ownership. This module adds only Molten world meaning.

#![allow(
    tigerstyle::non_trait_imports,
    reason = "world distribution composes closed generic core DTOs through explicit pure adapters"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "world distribution protocol names remain explicit at repository boundaries"
)]

mod claims;
mod closure;
mod model;
mod replication;
mod retention;

pub use claims::*;
pub use closure::*;
pub use model::*;
pub use replication::*;
pub use retention::*;

#[cfg(test)]
mod tests;
