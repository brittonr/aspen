#![allow(
    tigerstyle::module_file_count,
    reason = "time separates capacity, core projection, ports, shells, mechanisms, fixtures, and tests"
)]

pub mod capacity;

mod adapters;
mod canonical;
mod fixture;
mod history;
mod ports;
mod shell;

#[cfg(test)]
pub(crate) mod tests;

pub use adapters::*;
pub use canonical::*;
pub use fixture::*;
pub use history::*;
pub use molten_core::fabric_time::*;
pub use ports::*;
pub use shell::*;
