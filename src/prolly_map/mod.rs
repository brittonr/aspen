//! Capability-rooted Prolly semantic-state map shell.

#![allow(
    tigerstyle::non_trait_imports,
    tigerstyle::path_segment_repetition,
    reason = "the shell keeps application ports, Redb adapter types, and canonical record names explicit"
)]

#[cfg(feature = "marble-object-store")]
mod marble;
mod ports;
mod records;
mod selection;
mod service;
mod store;
mod storeretention;
mod storesession;

#[cfg(feature = "marble-object-store")]
pub use marble::*;
pub use ports::*;
pub use records::*;
pub use selection::*;
pub use service::*;
pub use store::*;

#[cfg(test)]
mod tests;
