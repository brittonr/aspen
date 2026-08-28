//! Pure bounded Prolly semantic-state map.

#![allow(
    tigerstyle::non_trait_imports,
    tigerstyle::path_segment_repetition,
    reason = "domain-qualified map and evidence types keep profile, node, oracle, and benchmark ownership explicit"
)]
#![allow(
    tigerstyle::unbounded_collection_growth,
    reason = "validated profile limits bound entries, nodes, graph facts, diffs, and every collection-producing loop"
)]

mod differential;
mod model;
mod operations;
mod profile;
mod proof;
mod retention;
mod tree;

pub use differential::*;
pub use model::*;
pub use operations::*;
pub use profile::*;
pub use proof::*;
pub use retention::*;
pub use tree::*;

#[cfg(test)]
mod tests;
