//! Pure addressable-actor composition over admitted Molten fabric facts.
//!
//! This module owns canonical actor identity, the actor lifecycle view, the
//! closed survival matrix, and deterministic effect plans. It performs no
//! storage, clock, process, network, transport, policy, or observability I/O.
//! Existing fabric components retain those capabilities and their authority.
//!
//! r[impl molten.addressable_actor.profile]

#![allow(
    tigerstyle::path_segment_repetition,
    reason = "public actor vocabulary stays explicit at the composition boundary"
)]

mod identity;
mod model;
mod status;
mod transition;
mod validation;

pub use identity::*;
pub use model::*;
pub use status::*;
pub use transition::*;
pub use validation::*;

#[cfg(test)]
mod tests;
