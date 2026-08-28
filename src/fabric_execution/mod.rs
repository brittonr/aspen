#![allow(
    tigerstyle::module_file_count,
    reason = "execution separates canonical contracts, application ports, live mechanics, simulation, and tests"
)]

//! Canonical bounded-execution fabric port and adapters.

mod canonical;
mod live;
mod mechanics;
mod ports;
mod simulation;

#[cfg(test)]
mod tests;

pub use canonical::*;
pub use live::*;
pub use molten_core::fabric_execution::*;
pub use ports::*;
pub use simulation::*;
