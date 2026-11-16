//! Optional compute-capacity service composing pure decisions with admitted effects.

pub mod canonical;
pub mod persistence;
mod ports;
mod shell;

pub use molten_core::compute_capacity::*;
pub use ports::*;
pub use shell::*;

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "fixture/tests.rs"]
mod fixture;
