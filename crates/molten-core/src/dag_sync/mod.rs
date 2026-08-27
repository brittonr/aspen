//! Domain-neutral deterministic DAG synchronization planning.

#![allow(
    tigerstyle::path_segment_repetition,
    reason = "public DAG-prefixed protocol types remain unambiguous when consumers import them outside this module"
)]
#![allow(
    tigerstyle::concrete_construction_in_core,
    reason = "the pure core constructs bounded domain decisions and local collections, never infrastructure adapters"
)]

mod identity;
mod issue;
mod model;
mod planner;
mod reference;
mod validation;

use identity::*;
pub use issue::*;
pub use model::*;
pub use planner::*;
pub use reference::*;
use validation::*;

#[cfg(test)]
mod tests;
