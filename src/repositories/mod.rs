//! Repository traits for infrastructure abstraction
//!
//! This module defines trait-based abstractions over infrastructure services,
//! enabling dependency injection and testability in the domain layer.

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value as JsonValue;

use crate::domain::types::{HealthStatus, Job, JobStatus, QueueStats};

pub mod hiqlite_repository;
pub mod work_queue_repository;

// Export mocks module for testing (both unit tests and integration tests)
#[cfg(any(test, feature = "test-utils"))]
pub mod mocks;

// Re-export concrete implementations
pub use hiqlite_repository::HiqliteStateRepository;
pub use work_queue_repository::WorkQueueWorkRepository;

/// Repository abstraction for cluster state and database operations
///
/// Wraps HiqliteService to provide a testable interface for domain services.
/// Returns domain types to maintain proper layer separation.
#[async_trait]
pub trait StateRepository: Send + Sync {
    /// Get database cluster health information
    ///
    /// Returns health status using domain types, independent of infrastructure.
    async fn health_check(&self) -> Result<HealthStatus>;
}

/// Repository abstraction for work queue operations
///
/// Wraps WorkQueue to provide a testable interface for job lifecycle operations.
/// All methods use domain types (Job, JobStatus, QueueStats) instead of
/// infrastructure types, maintaining proper layer separation.
#[async_trait]
pub trait WorkRepository: Send + Sync {
    /// Publish a new work item to the queue
    async fn publish_work(&self, job_id: String, payload: JsonValue) -> Result<()>;

    /// Claim the next available work item
    async fn claim_work(&self) -> Result<Option<Job>>;

    /// Update the status of a work item
    async fn update_status(&self, job_id: &str, status: JobStatus) -> Result<()>;

    /// List all work items in the queue
    async fn list_work(&self) -> Result<Vec<Job>>;

    /// Get aggregate statistics for the work queue
    async fn stats(&self) -> QueueStats;
}
