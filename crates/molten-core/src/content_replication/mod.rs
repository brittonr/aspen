//! Deterministic content-replication policy and transition plans.

#![allow(
    tigerstyle::concrete_construction_in_core,
    reason = "the pure core constructs bounded domain plans and local collections, never infrastructure adapters"
)]

mod diagnostic;
mod eligibility;
mod identity;
mod model;
mod planner;
mod projection;
pub mod repair;
mod validation;

pub use diagnostic::*;
pub use eligibility::*;
pub use identity::*;
pub use model::*;
pub use planner::*;
pub use projection::*;
pub use repair::*;
pub use validation::*;

#[cfg(test)]
mod tests;
