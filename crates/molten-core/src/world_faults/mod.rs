//! Pure bounded crash, restart, uncertainty, and concurrency conformance policy.
//!
//! The core owns the closed mutation inventory, semantic fault phases,
//! deterministic schedules, conservative comparisons, and bounded receipt
//! meaning. Shell adapters own interruption, restart, durable observation, and
//! receipt persistence. Subsystem cores retain operation and recovery policy.

#![allow(
    tigerstyle::non_trait_imports,
    reason = "world fault conformance composes closed protocol and reconciliation DTOs"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "public world-fault names preserve the protocol domain when consumers import them"
)]
#![allow(
    tigerstyle::borrowed_argument_types,
    reason = "bounded encoders and validators mutate preallocated byte and diagnostic vectors"
)]
#![allow(
    tigerstyle::unbounded_collection_growth,
    reason = "closed inventory and validated profile limits bound every map, set, byte vector, diagnostic, and result collection"
)]

mod conformance;
mod identity;
mod inventory;
mod model;
mod profile;
mod schedule;
mod validation;

pub use conformance::*;
pub use identity::*;
pub use inventory::*;
pub use model::*;
pub use profile::*;
pub use schedule::*;
pub use validation::*;

#[cfg(test)]
mod tests;
