//! Experimental sync-only component RPC; no existing protocol is rerouted.
pub mod authority;
pub mod bridge;
mod channels;
mod codec;
pub mod diagnostics;
pub mod fixture;
mod fixtureauth;
mod fixturefabric;
pub mod journal;
pub mod materialization;
mod negative;
pub mod operator;
mod package;
pub mod profile;
mod runtime;
#[cfg(feature = "component-rpc-telemetry")]
pub mod telemetry;
pub mod transcript;
pub mod transport;

pub const PROFILE: &str = "molten.component-rpc.hello.v1";
pub const REVISION: &str = "aa01409ed38bf7a978570ff567c4475376925df6";
pub const INTERFACE: &str = "wrpc-examples:hello/handler";
pub const FUNCTION: &str = "hello";
pub const MAX_BYTES: usize = 4096;
pub type Result<T, E = String> = std::result::Result<T, E>;
pub(crate) fn failure(error: impl std::fmt::Display) -> String {
    error.to_string()
}
pub(crate) fn reference(bytes: &[u8]) -> String {
    format!("blake3:{}", blake3::hash(bytes))
}
pub(crate) fn hash(value: &preserves::IOValue) -> Result<String> {
    crate::codec::canonical_hash(value).map_err(failure)
}
