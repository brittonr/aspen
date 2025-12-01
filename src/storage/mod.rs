//! Storage adapters for Aspen.
//!
//! # Responsibilities
//! - Provide explicit interfaces for Raft log/state-machine implementations
//!   so we can swap between the production `redb` backend and deterministic
//!   in-memory engines under `madsim`.
//! - Surface property-test seam points in `storage::log` and
//!   `storage::state_machine` so future contributors can drop in deterministic
//!   harnesses quickly.
//! - Keep implementations statically allocated where possible per Tiger Style.
//!
//! # Current layout
//! - [`log`] exposes the log-IO contract plus an in-memory implementation that
//!   already enforces ordering/truncation invariants.
//! - [`state_machine`] handles state-application snapshots and similarly ships
//!   with an in-memory engine.
//! - [`StorageSurface`] pipes both handles together so the Raft actor can depend
//!   on a single struct while still swapping backends independently.

use std::sync::Arc;

pub mod log;
pub mod state_machine;

use log::{InMemoryLog, LogHandle};
use state_machine::{InMemoryStateMachine, StateMachineHandle};

/// Storage surface that bundles both log + state machine handles.
pub struct StorageSurface {
    log: Arc<dyn LogHandle>,
    state_machine: Arc<dyn StateMachineHandle>,
}

impl std::fmt::Debug for StorageSurface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StorageSurface")
            .field("log", &"dyn LogHandle")
            .field("state_machine", &"dyn StateMachineHandle")
            .finish()
    }
}

impl Clone for StorageSurface {
    fn clone(&self) -> Self {
        Self {
            log: Arc::clone(&self.log),
            state_machine: Arc::clone(&self.state_machine),
        }
    }
}

impl StorageSurface {
    /// Builds an in-memory surface suitable for deterministic tests.
    pub fn in_memory() -> Self {
        Self {
            log: Arc::new(InMemoryLog::default()),
            state_machine: Arc::new(InMemoryStateMachine::default()),
        }
    }

    /// Construct a surface from explicit handles (e.g., redb, forthcoming).
    pub fn from_parts(log: Arc<dyn LogHandle>, state_machine: Arc<dyn StateMachineHandle>) -> Self {
        Self { log, state_machine }
    }

    /// Access the log handle.
    pub fn log(&self) -> Arc<dyn LogHandle> {
        Arc::clone(&self.log)
    }

    /// Access the state machine handle.
    pub fn state_machine(&self) -> Arc<dyn StateMachineHandle> {
        Arc::clone(&self.state_machine)
    }
}

/// Storage plan describing how to construct deterministic backends.
#[derive(Debug, Clone)]
pub struct StoragePlan {
    /// Node identifier for namespacing deterministic data directories.
    pub node_id: String,
    /// Optional deterministic seed.
    pub seed: Option<u64>,
    /// Path on disk where persistent storage will land during Phase 2 (redb).
    pub data_dir: Option<std::path::PathBuf>,
}

impl StoragePlan {
    /// Validate invariants before the plan is executed.
    pub fn validate(&self) {
        assert!(
            !self.node_id.is_empty(),
            "storage plans require stable node identifiers"
        );
    }

    /// For now the plan materializes an in-memory surface; Phase 2 will route
    /// `data_dir` to the redb backend.
    pub fn materialize(&self) -> StorageSurface {
        self.validate();
        let mut log_impl = InMemoryLog::default();
        if let Some(seed) = self.seed {
            log_impl.seed(seed);
        }
        let log: Arc<dyn LogHandle> = Arc::new(log_impl);
        let sm: Arc<dyn StateMachineHandle> = Arc::new(InMemoryStateMachine::default());
        StorageSurface::from_parts(log, sm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_surface_in_memory_isolated() {
        let plan = StoragePlan {
            node_id: "node-a".into(),
            seed: Some(42),
            data_dir: None,
        };
        let surface = plan.materialize();
        surface.log().append(1, b"one").unwrap();
        assert_eq!(surface.log().next_index(), 2);
        let bytes = surface.state_machine().snapshot().unwrap();
        assert!(bytes.is_empty());
    }
}
