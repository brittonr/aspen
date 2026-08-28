//! Imperative crash, restart, durable-read-back, schedule, and receipt shell.

#![allow(
    tigerstyle::non_trait_imports,
    reason = "the shell composes explicit core DTO, node-state, Preserves, and error boundaries"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "public world-fault names preserve their protocol domain for embedding consumers"
)]

mod ports;
mod records;
mod service;

pub use ports::*;
pub use records::*;
pub use service::*;

#[cfg(test)]
mod tests;
