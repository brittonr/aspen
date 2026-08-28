//! Pure world-transition replay and complete capsule planning.
//!
//! This core validates canonical identities, closure, ordering, bounds, and
//! divergence meaning. It performs no storage, restore, execution, import,
//! publication, authority, or effect operations.

#![allow(
    tigerstyle::non_trait_imports,
    reason = "the replay core composes one closed set of typed protocol records"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "world replay names remain explicit at the product boundary"
)]

mod comparison;
mod identity;
mod model;
mod planning;
mod validation;

pub use comparison::*;
pub use identity::*;
pub use model::*;
pub use planning::*;
pub use validation::*;

#[cfg(test)]
mod tests;
