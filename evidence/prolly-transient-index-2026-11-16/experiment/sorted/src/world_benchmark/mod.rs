//! Pure world benchmark profiles, exact metrics, and extraction decisions.
//!
//! The core classifies supplied observations. It performs no measurement, I/O,
//! deletion, dependency selection, or repository creation.

#![allow(
    tigerstyle::non_trait_imports,
    reason = "the benchmark core composes one closed public protocol vocabulary"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "world benchmark names remain explicit at product boundaries"
)]

mod decision;
mod identity;
mod model;
mod validation;

pub use decision::*;
pub use identity::*;
pub use model::*;
pub use validation::*;

#[cfg(test)]
mod tests;
