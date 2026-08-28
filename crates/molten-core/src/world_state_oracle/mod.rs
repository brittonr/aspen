//! Pure authority-neutral semantic-state oracle contracts.

#![allow(
    tigerstyle::non_trait_imports,
    tigerstyle::path_segment_repetition,
    reason = "oracle-qualified public names keep third-party evidence distinct from Molten product state"
)]
#![allow(
    tigerstyle::borrowed_argument_types,
    tigerstyle::unbounded_collection_growth,
    reason = "validated source, row, and diagnostic bounds constrain every mutable collection"
)]

mod admission;
mod comparison;
mod compatibility;
mod identity;
mod model;
mod projection;

pub use admission::*;
pub use comparison::*;
pub use compatibility::*;
pub use identity::*;
pub use model::*;
pub use projection::*;

#[cfg(test)]
mod tests;
