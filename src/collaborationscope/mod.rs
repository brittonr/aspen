//! Collaboration scope shell: canonical metadata, real authority, committed application adapter.
#[path = "admission/authority.rs"]
mod authority;
#[path = "admission/binding.rs"]
mod binding;
#[path = "codec/canonical.rs"]
mod canonical;
#[path = "codec/command.rs"]
mod command;
#[path = "application/commit.rs"]
mod commit;
#[path = "codec/decode.rs"]
mod decode;
#[path = "admission/manifest.rs"]
mod manifest;
#[path = "native/callback.rs"]
mod native;
#[path = "native/runtime.rs"]
pub mod native_runtime;
#[path = "codec/projection.rs"]
mod projection;
#[path = "application/replay.rs"]
mod replay;
#[path = "application/runtime.rs"]
mod runtime;
#[path = "application/storage.rs"]
mod storage;
#[path = "application/store.rs"]
mod store;

#[cfg(test)]
pub(crate) mod fixtures;
#[cfg(test)]
mod simulation;
#[cfg(test)]
mod tests;

pub use authority::*;
pub use binding::*;
pub use canonical::*;
pub use command::*;
pub use decode::*;
pub use manifest::*;
pub use molten_core::collaboration_scope::*;
pub use native::*;
pub use projection::*;
pub use replay::*;
pub use runtime::*;
pub use store::*;
