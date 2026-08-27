#![allow(
    tigerstyle::non_trait_imports,
    reason = "the world-head core composes explicit claim, Choregraph, policy, and conflict DTOs"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "public world-head names retain their protocol domain inside the larger runtime core"
)]

mod admission;
mod choregraph;
mod conflict;
mod domain;
mod validation;

pub use conflict::*;
pub use domain::*;
pub use validation::*;

#[cfg(test)]
mod tests;
