#![allow(
    tigerstyle::non_trait_imports,
    reason = "the world-merge core composes explicit diff, schema, handler, conflict, and result DTOs"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "world-merge protocol names remain explicit inside the larger runtime core"
)]
#![allow(
    tigerstyle::borrowed_argument_types,
    reason = "bounded reducers append typed outputs, conflicts, and diagnostics through explicit Vec mutation"
)]

mod admission;
mod diff;
mod handlers;
mod model;
mod planning;

pub use admission::*;
pub use diff::*;
pub use handlers::*;
pub use model::*;

#[cfg(test)]
mod tests;
