//! In-memory storage implementations for testing and development.
//!
//! This module provides non-persistent storage backends for Raft log and state machine,
//! suitable for unit tests, madsim simulations, and development environments.

mod log_store;
mod state_machine;

pub use log_store::InMemoryLogStore;
pub use state_machine::InMemoryStateMachine;
pub use state_machine::InMemoryStateMachineStore;

// Re-export StoredSnapshot for use in state_machine submodule
pub(crate) use super::StoredSnapshot;
