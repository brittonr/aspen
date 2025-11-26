//! Domain events for job lifecycle
//!
//! Domain events represent significant state changes in the business domain.
//! They enable:
//! - Audit logging
//! - Metrics collection
//! - Notification systems
//! - Event sourcing
//! - Decoupled side effects

use async_trait::async_trait;
use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::domain::types::JobStatus;

/// Domain event representing a significant state change
///
/// Events are immutable records of what happened in the system.
/// They are published by command services after successful state mutations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DomainEvent {
    /// A new job was submitted to the queue
    JobSubmitted {
        job_id: String,
        payload: serde_json::Value,
        timestamp: i64,
    },

    /// A job was claimed by a worker
    JobClaimed {
        job_id: String,
        worker_id: String,
        timestamp: i64,
    },

    /// A job's status changed
    JobStatusChanged {
        job_id: String,
        old_status: JobStatus,
        new_status: JobStatus,
        timestamp: i64,
    },

    /// A job completed successfully
    JobCompleted {
        job_id: String,
        worker_id: Option<String>,
        duration_ms: u64,
        timestamp: i64,
    },

    /// A job failed
    JobFailed {
        job_id: String,
        worker_id: Option<String>,
        error: String,
        timestamp: i64,
    },

    /// A job was cancelled
    JobCancelled {
        job_id: String,
        reason: String,
        timestamp: i64,
    },

    /// A failed job was reset for retry
    JobRetried {
        job_id: String,
        retry_count: usize,
        timestamp: i64,
    },
}

impl DomainEvent {
    /// Get the job ID associated with this event
    pub fn job_id(&self) -> &str {
        match self {
            DomainEvent::JobSubmitted { job_id, .. } => job_id,
            DomainEvent::JobClaimed { job_id, .. } => job_id,
            DomainEvent::JobStatusChanged { job_id, .. } => job_id,
            DomainEvent::JobCompleted { job_id, .. } => job_id,
            DomainEvent::JobFailed { job_id, .. } => job_id,
            DomainEvent::JobCancelled { job_id, .. } => job_id,
            DomainEvent::JobRetried { job_id, .. } => job_id,
        }
    }

    /// Get the timestamp when this event occurred
    pub fn timestamp(&self) -> i64 {
        match self {
            DomainEvent::JobSubmitted { timestamp, .. } => *timestamp,
            DomainEvent::JobClaimed { timestamp, .. } => *timestamp,
            DomainEvent::JobStatusChanged { timestamp, .. } => *timestamp,
            DomainEvent::JobCompleted { timestamp, .. } => *timestamp,
            DomainEvent::JobFailed { timestamp, .. } => *timestamp,
            DomainEvent::JobCancelled { timestamp, .. } => *timestamp,
            DomainEvent::JobRetried { timestamp, .. } => *timestamp,
        }
    }

    /// Get a human-readable description of this event
    pub fn description(&self) -> String {
        match self {
            DomainEvent::JobSubmitted { job_id, payload, .. } => {
                let summary = if let Some(url) = payload.get("url").and_then(|v| v.as_str()) {
                    format!(" for URL {}", url)
                } else {
                    String::new()
                };
                format!("Job {} submitted{}", job_id, summary)
            }
            DomainEvent::JobClaimed { job_id, worker_id, .. } => {
                format!("Job {} claimed by worker {}", job_id, worker_id)
            }
            DomainEvent::JobStatusChanged { job_id, old_status, new_status, .. } => {
                format!("Job {} status changed from {:?} to {:?}", job_id, old_status, new_status)
            }
            DomainEvent::JobCompleted { job_id, duration_ms, .. } => {
                format!("Job {} completed in {}ms", job_id, duration_ms)
            }
            DomainEvent::JobFailed { job_id, error, .. } => {
                format!("Job {} failed: {}", job_id, error)
            }
            DomainEvent::JobCancelled { job_id, reason, .. } => {
                format!("Job {} cancelled: {}", job_id, reason)
            }
            DomainEvent::JobRetried { job_id, retry_count, .. } => {
                format!("Job {} retried (attempt #{})", job_id, retry_count)
            }
        }
    }
}

/// Publisher trait for domain events
///
/// Implement this trait to handle domain events (logging, metrics, notifications, etc.)
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publish a domain event
    ///
    /// # Arguments
    /// * `event` - The domain event to publish
    ///
    /// # Errors
    /// Returns error if publishing fails (e.g., network error, full queue)
    async fn publish(&self, event: DomainEvent) -> Result<()>;

    /// Publish multiple events in a batch
    ///
    /// Default implementation publishes one-by-one, but implementations
    /// can optimize for batch operations.
    async fn publish_batch(&self, events: Vec<DomainEvent>) -> Result<()> {
        for event in events {
            self.publish(event).await?;
        }
        Ok(())
    }
}

/// Helper function to get the current timestamp
pub fn current_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}