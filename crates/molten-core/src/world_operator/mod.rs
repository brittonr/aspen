//! Pure planning and admission for the composed Molten world operator.
//!
//! The core orders typed component operations, reports the first stable
//! blocker, binds preview identity, and rechecks apply facts. It performs no
//! filesystem, process, network, clock, credential, storage, or component
//! operation.

#![allow(
    tigerstyle::non_trait_imports,
    reason = "the world operator core composes closed typed protocol records"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "world operator protocol names remain explicit at the product boundary"
)]

mod identity;
mod model;
mod planning;
mod receipt;

pub use identity::*;
pub use model::*;
pub use planning::*;
pub use receipt::*;

#[cfg(test)]
mod tests;
