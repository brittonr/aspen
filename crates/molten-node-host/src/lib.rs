//! Capability-rooted node host boundary for Molten.
//!
//! This crate owns local node-state and typed local-store authority. It does
//! not own CLI parsing, operator presentation, release policy, workload
//! semantics, or test harness orchestration.

pub mod error;
pub mod local_store;
#[path = "node/state.rs"]
pub mod node_state;

pub mod core_api {
    pub use molten_core::*;
}

pub mod prelude {
    pub use crate::error::MoltenError;
    pub use crate::error::Result;
    pub use crate::local_store::*;
    pub use crate::node_state::*;
}
