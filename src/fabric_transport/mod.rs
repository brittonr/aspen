//! Canonical transport artifacts plus live Iroh and deterministic adapter shells.
//!
//! Pure protocol-registration, session, stream, framing, flow-control,
//! cancellation, and failure laws are owned by `molten-core`.

mod adapters;
mod canonical;
pub(crate) mod cross_process;

#[cfg(test)]
mod tests;

pub use adapters::*;
pub use canonical::*;
pub use cross_process::*;
pub use molten_core::fabric_transport::*;
