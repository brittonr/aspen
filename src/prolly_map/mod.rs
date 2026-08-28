//! Capability-rooted Prolly semantic-state map shell.

#![allow(
    tigerstyle::non_trait_imports,
    tigerstyle::path_segment_repetition,
    reason = "the shell keeps application ports, Redb adapter types, and canonical record names explicit"
)]

mod ports;
mod records;
mod service;
mod store;

pub use ports::*;
pub use records::*;
pub use service::*;
pub use store::*;

#[cfg(test)]
mod tests;
