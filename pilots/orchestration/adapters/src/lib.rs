// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]
//! Capability-rooted infrastructure adapters and the explicit composition boundary.
pub mod identity;
pub mod policy;
pub mod protocol;
pub mod root;
pub mod service;
pub mod storage;
pub mod transport;
pub mod wasm;
pub use identity::*;
use molten_batch_application::Error;
pub use policy::*;
pub use protocol::*;
pub use root::*;
pub use service::*;
pub use storage::*;
pub use transport::*;
pub use wasm::*;
fn invalid(e: impl std::fmt::Display) -> Error {
    Error::Invalid(e.to_string())
}
