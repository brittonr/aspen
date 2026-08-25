//! Application-owned transport command capability.

#![allow(
    tigerstyle::non_trait_imports,
    reason = "the transport port exposes explicit domain command and transition types"
)]

use super::CanonicalTransportTransition;
use super::TransportCommand;
use crate::fabric::FabricPortResult;

// r[impl molten.modularity.fabric_boundary.ports]

pub trait TransportCommandShell {
    fn profile_id(&self) -> &str;
    fn execute_command(&mut self, command: &TransportCommand) -> FabricPortResult<CanonicalTransportTransition>;
}
