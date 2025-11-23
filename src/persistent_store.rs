//! Persistent Store Abstraction
//!
//! This module defines the `PersistentStore` trait that abstracts persistence operations
//! for workflow data. By operating at the domain level (Job types) rather than
//! exposing raw SQL, this trait enables:
//! - Testing with mock implementations
//! - Swapping storage backends without changing business logic
//! - Clear separation between domain and infrastructure concerns

use anyhow::Result;
use async_trait::async_trait;
use crate::domain::types::{Job, JobStatus};

/// Persistent storage abstraction for workflow data
///
/// This trait defines high-level operations on workflow data, abstracting away
/// the underlying storage mechanism (SQL, NoSQL, in-memory, etc.).
#[async_trait]
pub trait PersistentStore: Send + Sync {
    /// Load all workflows from persistent storage
    ///
    /// Returns a vector of all Jobs currently in storage, regardless of status.
    async fn load_all_workflows(&self) -> Result<Vec<Job>>;

    /// Insert or replace a workflow in persistent storage
    ///
    /// If a workflow with the same job_id exists, it will be replaced.
    /// This operation should be atomic.
    async fn upsert_workflow(&self, job: &Job) -> Result<()>;

    /// Atomically claim a pending workflow
    ///
    /// Attempts to update a workflow from 'pending' status to 'claimed' status,
    /// setting the claimed_by field. This operation uses optimistic locking via
    /// the WHERE clause to prevent race conditions.
    ///
    /// Returns the number of rows affected (0 if already claimed, 1 if successful).
    async fn claim_workflow(
        &self,
        job_id: &str,
        claimed_by: &str,
        updated_at: i64,
    ) -> Result<usize>;

    /// Update workflow status with state machine guards
    ///
    /// Updates a workflow's status, optionally setting completed_by for terminal states.
    /// Implements state machine invariants:
    /// - Cannot transition from terminal states (completed/failed) unless idempotent
    /// - Terminal transitions must set completed_by
    ///
    /// Returns the number of rows affected (0 if update rejected by guards).
    async fn update_workflow_status(
        &self,
        job_id: &str,
        status: &JobStatus,
        completed_by: Option<&str>,
        updated_at: i64,
    ) -> Result<usize>;
}
