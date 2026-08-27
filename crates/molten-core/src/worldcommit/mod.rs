#![allow(
    tigerstyle::non_trait_imports,
    reason = "the pure world-commit core composes a closed set of typed protocol DTOs and validators"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "public world-commit names distinguish this protocol from other commit and root domains"
)]
#![allow(
    tigerstyle::borrowed_argument_types,
    reason = "bounded validators append typed diagnostics and restore steps, so Vec mutation is part of the contract"
)]

mod capture;
mod model;
mod reference;
mod restore;
mod validation;

pub use capture::*;
pub use model::*;
pub use reference::*;
pub use restore::*;
pub use validation::*;

#[cfg(test)]
mod tests;
