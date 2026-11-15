//! Imperative shell for the addressable-actor composition profile.

pub mod command;
pub mod counter;
pub mod delivery;
pub mod native;
mod ports;
mod records;
#[path = "records/result.rs"]
mod result;
mod service;
mod simulation;
mod store;

pub use ports::*;
pub use records::*;
pub use result::*;
pub use service::*;
pub use simulation::*;
pub use store::*;

#[cfg(test)]
mod tests;
