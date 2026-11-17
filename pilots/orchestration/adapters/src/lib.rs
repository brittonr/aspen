// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]
#![feature(register_tool)]
#![register_tool(tigerstyle)]
//! Capability-rooted infrastructure adapters and the explicit composition boundary.
pub mod fault;
pub mod identity;
pub mod policy;
pub mod progress;
pub mod protocol;
pub mod pure_net;
pub mod root;
pub mod service;
pub mod storage;
pub mod transport;
pub mod wasm;
pub use fault::ProcessFault;
// Reuse the authoritative core implementation, including cursor-retention fixes.
#[path = "../../../../crates/molten-core/src/committed_stream/mod.rs"]
pub mod committed_stream;
pub use identity::*;
use molten_batch_application::Error;
pub use policy::*;
pub use protocol::*;
pub use pure_net::*;
pub use root::*;
pub use service::*;
pub use storage::*;
pub use transport::*;
pub use wasm::*;
fn invalid(e: impl std::fmt::Display) -> Error {
    Error::Invalid(e.to_string())
}
