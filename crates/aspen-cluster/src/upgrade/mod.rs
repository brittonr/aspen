//! Node-level upgrade executor for rolling deployments.
//!
//! Handles the lifecycle of upgrading a single node's binary:
//! 1. Drain in-flight operations (graceful shutdown of client RPCs)
//! 2. Replace the binary (Nix profile switch or blob download)
//! 3. Restart the process (systemd or execve)
//! 4. Report status transitions to cluster KV
//!
//! # Tiger Style
//!
//! - Bounded drain timeout (`DRAIN_TIMEOUT_SECS`)
//! - Explicit status reporting at each state transition
//! - Atomic binary replacement (Nix generations or rename)
//! - Preserved rollback path (Nix rollback or `.bak` file)

mod drain;
pub mod executor;
mod restart;
pub mod rollback;
pub mod status;
mod types;

pub use executor::NodeUpgradeExecutor;
pub use types::NodeUpgradeConfig;
pub use types::NodeUpgradeError;
pub use types::RestartMethod;
pub use types::UpgradeMethod;
