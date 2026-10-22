//! Outside-in scenarios over real node processes.
//!
//! A scenario drives ordinary node processes through public entry points over
//! the real transport and real local stores, applies one declared fault at a
//! declared boundary, and then reads externally observed histories through
//! public interfaces. This module is the pure part: it validates scenario
//! declarations and judges observed histories. The shell that starts
//! processes, injects faults, and reads histories lives with the tests.
//!
//! The judgment uses only observed outcomes. No observation carries a time, so
//! physical timing cannot enter an agreement decision. Every result carries
//! the track's non-claims: no production admission, no replacement of
//! deterministic simulation, and no performance characteristic.

#![allow(
    tigerstyle::concrete_construction_in_core,
    reason = "the pure core constructs bounded scenario decisions and local collections, never infrastructure adapters"
)]

mod judge;
mod model;
mod observation;
mod validate;

pub use judge::*;
pub use model::*;
pub use observation::*;
pub use validate::*;

#[cfg(test)]
mod tests;
