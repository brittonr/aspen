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
pub mod worker;

// Export mocks module for testing (both unit tests and integration tests)
pub mod mocks;

// Re-export concrete implementations
pub use hiqlite_repository::HiqliteStateRepository;
pub use work_queue_repository::WorkQueueWorkRepository;
pub use worker::{HiqliteWorkerRepository, WorkerRepository};

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
    ///
    /// If worker_id and worker_type are provided, only jobs compatible with that
    /// worker type will be considered, and the job will be assigned to that worker.
    async fn claim_work(&self, worker_id: Option<&str>, worker_type: Option<crate::domain::types::WorkerType>) -> Result<Option<Job>>;

    /// Update the status of a work item
    async fn update_status(&self, job_id: &str, status: JobStatus) -> Result<()>;

    /// Find a specific job by ID
    ///
    /// This is a **query** that performs data access filtering at the repository layer.
    /// Prefer this over `list_work()` + filter for single job lookups.
    ///
    /// # Arguments
    /// * `job_id` - The job identifier to look up
    ///
    /// # Returns
    /// The job if found, None otherwise
    async fn find_by_id(&self, job_id: &str) -> Result<Option<Job>>;

    /// Find all jobs with a specific status
    ///
    /// This is a **query** that performs data access filtering at the repository layer.
    /// Prefer this over `list_work()` + filter for status-based queries.
    ///
    /// # Arguments
    /// * `status` - The status to filter by
    ///
    /// # Returns
    /// All jobs with the specified status
    async fn find_by_status(&self, status: JobStatus) -> Result<Vec<Job>>;

    /// Find all jobs claimed by a specific worker
    ///
    /// This is a **query** that performs data access filtering at the repository layer.
    /// Prefer this over `list_work()` + filter for worker-based queries.
    ///
    /// # Arguments
    /// * `worker_id` - The worker node ID to filter by
    ///
    /// # Returns
    /// All jobs claimed by the specified worker
    async fn find_by_worker(&self, worker_id: &str) -> Result<Vec<Job>>;

    /// Find jobs with pagination support
    ///
    /// This is a **query** that performs data access filtering and pagination at the repository layer.
    /// Use this for large result sets to avoid loading all data into memory.
    ///
    /// # Arguments
    /// * `offset` - Number of items to skip (0-based)
    /// * `limit` - Maximum number of items to return
    ///
    /// # Returns
    /// Paginated slice of jobs (ordered by update time, most recent first)
    async fn find_paginated(&self, offset: usize, limit: usize) -> Result<Vec<Job>>;

    /// Find all jobs with a specific status claimed by a specific worker
    ///
    /// This is a **composite query** that performs multiple filters at the repository layer.
    /// Useful for complex queries that combine multiple criteria.
    ///
    /// # Arguments
    /// * `status` - The status to filter by
    /// * `worker_id` - The worker node ID to filter by
    ///
    /// # Returns
    /// All jobs matching both criteria
    async fn find_by_status_and_worker(&self, status: JobStatus, worker_id: &str) -> Result<Vec<Job>>;

    /// List all work items in the queue
    async fn list_work(&self) -> Result<Vec<Job>>;

    /// Get aggregate statistics for the work queue
    async fn stats(&self) -> QueueStats;
}
