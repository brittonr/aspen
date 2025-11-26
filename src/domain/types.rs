//! Domain types - Compatibility re-exports
//!
//! This module re-exports all domain types from their new organized locations.
//! It maintains backward compatibility while the codebase transitions to the
//! new module structure.
//!
//! New code should import directly from:
//! - `crate::domain::job::*` for job types
//! - `crate::domain::worker::*` for worker types
//! - `crate::domain::queue::*` for queue types

// Job types
pub use super::job::{Job, JobStatus};

// Worker types
pub use super::worker::{
    Worker, WorkerHeartbeat, WorkerRegistration, WorkerStats, WorkerStatus, WorkerType,
};

// Queue types
pub use super::queue::{HealthStatus, QueueStats};
