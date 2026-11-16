//! Canonical transport artifacts plus live Iroh and deterministic adapter shells.
//!
//! Pure protocol-registration, session, stream, framing, flow-control,
//! cancellation, and failure laws are owned by `molten-core`.

#![allow(
    tigerstyle::module_file_count,
    reason = "transport separates cross-process mechanisms, ports, shells, projection, mechanisms, and tests"
)]

mod adapters;
mod canonical;
mod consumer;
pub(crate) mod cross_process;
mod ip_hint;
mod ports;
mod shell;

#[cfg(test)]
mod tests;

pub use adapters::*;
pub use canonical::*;
pub use consumer::*;
pub use cross_process::*;
pub use ip_hint::*;
pub use molten_core::fabric_transport::*;
pub use ports::*;
pub use shell::*;
