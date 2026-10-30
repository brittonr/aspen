#[path = "cli/ops/reliability/mod.rs"]
mod reliability_port;
mod cli_reliability {
    pub(crate) use super::reliability_port::*;
}
#[path = "cli/workflow/federation.rs"]
mod federation_port;
mod cli_federation {
    pub(crate) use super::federation_port::*;
}
#[cfg(any(feature = "spacewasm-pilot", feature = "component-rpc"))]
#[path = "cli/runtime/pilots.rs"]
mod runtime_pilot_port;
#[cfg(any(feature = "spacewasm-pilot", feature = "component-rpc"))]
mod cli_pilots {
    pub(crate) use super::runtime_pilot_port::*;
}
