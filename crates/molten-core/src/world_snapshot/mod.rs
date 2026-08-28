#![allow(
    tigerstyle::borrowed_argument_types,
    tigerstyle::non_trait_imports,
    tigerstyle::path_segment_repetition,
    tigerstyle::unbounded_collection_growth,
    reason = "the bounded pure snapshot protocol keeps closed DTO names, validators, and diagnostic accumulation together for review"
)]

//! Pure execution-snapshot profile, compatibility, restore, and clone planning.

mod identity;
mod model;
mod operation;
mod validation;

pub use identity::*;
pub use model::*;
pub use operation::*;
pub use validation::*;

#[cfg(test)]
mod tests;
