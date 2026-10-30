//! Independently admitted optional WASI OTel tracing export; never application evidence.
mod authority;
mod bundle;
mod fixtureauth;
mod host;
pub mod operator;
mod projection;

pub use authority::Lease;
pub use authority::Scope;
pub use authority::admit;
pub use authority::binding;
pub use authority::resource;
pub use bundle::Bundle;
pub use host::ExportInput;
pub use host::export;

pub const REVISION: &str = "e5262a84044aff064c9599eebc05013bb3432795";
pub const INTERFACE: &str = "wasi:otel/tracing@0.2.0-rc.2";
pub const ABILITY: &str = "component-rpc/diagnostics-export";
pub const CONTRACT: &str = "component-rpc-telemetry-export-v1";
pub const MAX_SPANS: usize = 32;
pub type Result<T, E = String> = std::result::Result<T, E>;
