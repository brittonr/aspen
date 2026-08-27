#![allow(
    tigerstyle::non_trait_imports,
    reason = "the pure execution core composes one closed set of typed fabric and execution contracts"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "execution-qualified public names keep process observations distinct from application outcomes"
)]
#![allow(
    tigerstyle::borrowed_argument_types,
    reason = "bounded validators append typed issues to caller-owned diagnostic vectors"
)]
//! Pure bounded-execution fabric contracts.
//!
//! This module owns deterministic profile, request, authority, resource,
//! lifecycle, completion-linkage, and reconciliation decisions. It performs no
//! filesystem, process, environment, clock, network, or persistence effects.

mod admission;
mod lifecycle;
mod model;

#[cfg(test)]
mod tests;

pub use admission::*;
pub use lifecycle::*;
pub use model::*;
