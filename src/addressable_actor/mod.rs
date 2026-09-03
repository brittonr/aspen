//! Imperative shell for the addressable-actor composition profile.

mod ports;
mod records;
mod service;
mod simulation;
mod store;

pub use ports::*;
pub use records::*;
pub use service::*;
pub use simulation::*;
pub use store::*;

#[cfg(test)]
mod tests;
