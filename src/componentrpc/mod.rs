//! Experimental sync-only component RPC; no existing protocol is rerouted.
pub mod authority;
pub mod bridge;
#[path = "shell/channels.rs"]
mod channels;
mod codec;
pub mod diagnostics;
#[path = "fixture/run.rs"]
pub mod fixture;
#[path = "fixture/authority.rs"]
mod fixtureauth;
#[path = "fixture/fabric.rs"]
mod fixturefabric;
#[path = "shell/journal.rs"]
pub mod journal;
#[path = "shell/materialization.rs"]
pub mod materialization;
#[path = "fixture/negative.rs"]
mod negative;
#[path = "shell/operator.rs"]
pub mod operator;
#[path = "shell/package.rs"]
mod package;
pub mod profile;
#[path = "shell/runtime.rs"]
mod runtime;
#[cfg(feature = "component-rpc-telemetry")]
pub mod telemetry;
pub mod transcript;
#[path = "shell/transport.rs"]
pub mod transport;

// r[impl molten.component_rpc.profile]
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
