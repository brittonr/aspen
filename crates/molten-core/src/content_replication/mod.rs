//! Deterministic content-replication policy and transition plans.

#![allow(
    tigerstyle::concrete_construction_in_core,
    reason = "the pure core constructs bounded domain plans and local collections, never infrastructure adapters"
)]

mod action;
mod diagnostic;
mod identity;
mod model;
mod planner;
mod validation;

pub use action::*;
pub use diagnostic::*;
pub use identity::*;
pub use model::*;
pub use planner::*;
pub use validation::*;

#[cfg(test)]
mod tests;
