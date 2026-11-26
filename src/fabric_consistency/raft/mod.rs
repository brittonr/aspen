//! The replicated Raft shell: Tokio timers, Iroh transport, Redb durability, the effect executor,
//! and live services around the pure `molten-raft-core` kernel, which owns the replica state,
//! the transition function, its effect plans, start admission, and envelope shape validation.

#[path = "parts/admission/mod.rs"]
mod admission;
mod application;
mod canonical;
mod control;
mod durability;
mod evidence;
mod executor;
mod iroh;
mod kernel;
mod ports;
#[path = "parts/recovery/mod.rs"]
mod recovery;
mod service;
mod time;

pub use admission::*;
pub use application::*;
pub use canonical::*;
pub use control::*;
pub use durability::*;
pub use evidence::*;
pub use executor::*;
pub use iroh::*;
pub use kernel::*;
pub use molten_raft_core::admission::ReplicaStartPlan;
pub use molten_raft_core::denial::ReplicaError;
pub use molten_raft_core::denial::ReplicaErrorClass;
pub use molten_raft_core::model::*;
pub use molten_raft_core::ports::ReplicaAdapterSet;
pub use molten_raft_core::ports::ReplicaPortBundle;
pub use molten_raft_core::ports::ReplicaRuntimePortIdentity;
pub use molten_raft_core::ports::validate_replica_runtime_identity_for_start;
pub use molten_raft_core::transition::StaleTimerEvent;
pub use molten_raft_core::transition::is_snapshot_due;
pub use molten_raft_core::transition::stale_timer_event;
pub use ports::*;
pub use recovery::*;
pub use service::*;
pub use time::*;

#[cfg(test)]
#[path = "canonical/tests.rs"]
mod canonical_tests;
#[cfg(test)]
#[path = "../../collaborationscope/process.rs"]
mod collaboration_process;
#[cfg(test)]
#[path = "durability/tests.rs"]
mod durability_tests;
#[cfg(test)]
#[path = "evidence/tests.rs"]
mod evidence_tests;
#[cfg(test)]
#[path = "executor/chunks.rs"]
mod executor_chunk_tests;
#[cfg(test)]
#[path = "executor/tests.rs"]
mod executor_tests;
#[cfg(test)]
#[path = "iroh/bundle.rs"]
mod iroh_bundle_tests;
#[cfg(test)]
#[path = "iroh/tests.rs"]
mod iroh_tests;
#[cfg(test)]
#[path = "kernel/tests.rs"]
mod kernel_tests;
#[cfg(test)]
mod live_cluster;
#[cfg(test)]
mod live_process;
#[cfg(test)]
#[path = "ports/tests.rs"]
mod port_tests;
#[cfg(test)]
#[path = "rejoin/crash.rs"]
mod rejoin_restart_tests;
#[cfg(test)]
pub(crate) mod tests;
#[cfg(test)]
#[path = "time/tests.rs"]
mod time_tests;
