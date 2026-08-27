#![allow(
    tigerstyle::path_segment_repetition,
    reason = "native-host names keep system-extension service facts distinct from generic process facts"
)]
#![allow(
    tigerstyle::non_trait_imports,
    reason = "the native-host core composes one closed set of system-extension contracts"
)]
#![allow(
    tigerstyle::borrowed_argument_types,
    reason = "bounded validators append typed issues to caller-owned diagnostic vectors"
)]
// r[impl molten.system_extension.native_host.neutrality]
//! Pure native system-extension host admission and recovery decisions.

mod admission;
mod model;
mod recovery;

#[cfg(test)]
mod tests;

pub use admission::*;
pub use model::*;
pub use recovery::*;
