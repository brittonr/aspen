//! Bounded public registry over the existing scoped live replica.
#[path = "operator/access.rs"]
mod access;
#[path = "setup/consensus.rs"]
mod consensus;
#[path = "operator/control.rs"]
mod control;
#[path = "setup/decoder.rs"]
mod decoder;
#[path = "host/diagnostics.rs"]
mod diagnostics;
#[path = "host/forwarding.rs"]
mod forwarding;
#[path = "setup/launch.rs"]
mod launch;
#[path = "host/leader.rs"]
mod leader;
#[path = "operator/lifecycle.rs"]
mod lifecycle;
#[path = "setup/network.rs"]
mod network;
#[path = "host/probe.rs"]
mod probe;
pub mod profiles;
pub mod recipe;
#[path = "host/recovery.rs"]
mod recovery;
#[path = "setup/registry.rs"]
mod registry;
#[path = "host/runtime.rs"]
mod runtime;
#[path = "host/session.rs"]
mod session;

pub use access::Authorization;
pub use consensus::Group;
pub use consensus::Membership;
pub use consensus::Replica;
pub use decoder::Configuration;
pub use decoder::Durability;
pub use decoder::Time;
pub use diagnostics::HostObservation;
pub use launch::PortBinding;
pub use launch::Startup;
pub use launch::start;
pub use leader::PeerProbe;
pub use lifecycle::command;
pub use network::Peer;
pub use network::Transport;
pub use registry::Registry;
pub use registry::RegistryAdmission;
pub use runtime::Host;
pub use runtime::HostAdmission;

#[cfg(test)]
mod tests;
