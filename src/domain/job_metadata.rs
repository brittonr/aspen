//! Job metadata for timing and audit information
//!
//! This module defines domain types for job metadata that track timing,
//! lifecycle events, and audit information.

use serde::{Deserialize, Serialize};

/// Job metadata tracks timing and audit information
///
/// This is separate from core job data and requirements, focusing on
/// when things happened rather than what the job is.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JobMetadata {
    /// Timestamp when job was created (Unix epoch seconds)
    pub created_at: i64,
    /// Timestamp when job was last updated (Unix epoch seconds)
    pub updated_at: i64,
    /// Timestamp when job execution started (Unix epoch seconds)
    pub started_at: Option<i64>,
    /// Timestamp when job was completed or failed (Unix epoch seconds)
    pub completed_at: Option<i64>,
    /// Number of times this job has been retried
    pub retry_count: u32,
}

impl JobMetadata {
    /// Create new metadata with current timestamp
    pub fn new() -> Self {
        let now = current_timestamp();
        Self {
            created_at: now,
            updated_at: now,
            started_at: None,
            completed_at: None,
            retry_count: 0,
        }
    }

    /// Create metadata with specific creation time
    pub fn with_creation_time(created_at: i64) -> Self {
        Self {
            created_at,
            updated_at: created_at,
            started_at: None,
            completed_at: None,
            retry_count: 0,
        }
    }

    /// Mark the job as updated now
    pub fn mark_updated(&mut self) {
        self.updated_at = current_timestamp();
    }

    /// Mark the job as started now
    pub fn mark_started(&mut self) {
        let now = current_timestamp();
        self.started_at = Some(now);
        self.updated_at = now;
    }

    /// Mark the job as completed now
    pub fn mark_completed(&mut self) {
        let now = current_timestamp();
        self.completed_at = Some(now);
        self.updated_at = now;
    }

    /// Increment retry count
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
        self.mark_updated();
    }

    /// Calculate duration of job execution in seconds
    ///
    /// Returns the time from when the job started executing to when it was completed,
    /// or to the last update if still running. Returns 0 if job hasn't started yet.
    pub fn duration_seconds(&self) -> i64 {
        match self.started_at {
            Some(start) => {
                let end = self.completed_at.unwrap_or(self.updated_at);
                end - start
            }
            None => 0,
        }
    }

    /// Calculate duration of job execution in milliseconds
    pub fn duration_ms(&self) -> i64 {
        self.duration_seconds() * 1000
    }

    /// Calculate time since last update in seconds
    pub fn time_since_update_seconds(&self) -> i64 {
        let now = current_timestamp();
        now - self.updated_at
    }

    /// Check if the job has started execution
    pub fn has_started(&self) -> bool {
        self.started_at.is_some()
    }

    /// Check if the job has completed
    pub fn has_completed(&self) -> bool {
        self.completed_at.is_some()
    }
}

impl Default for JobMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Get current Unix timestamp in seconds
fn current_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("System time is before UNIX epoch")
        .as_secs() as i64
}
