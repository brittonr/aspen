//! Distributed job queue system for Aspen.
//!
//! This crate provides a high-level job queue system built on top of Aspen's
//! distributed coordination primitives. It supports job scheduling, worker
//! management, priority queues, retries, and monitoring.
//!
//! # Features
//!
//! - **Job Scheduling**: Immediate, delayed, and cron-based scheduling
//! - **Priority Queues**: High, normal, and low priority job execution
//! - **Worker Management**: Dynamic worker pools with health checks
//! - **Retry Policies**: Exponential backoff and custom retry strategies
//! - **Dead Letter Queue**: Automatic handling of failed jobs
//! - **Job Dependencies**: Complex workflows with dependency tracking
//! - **Monitoring**: Rich metrics and job history
//!
//! # Example
//!
//! ```ignore
//! use aspen_jobs::{JobManager, JobSpec, Worker, WorkerPool};
//!
//! // Define a worker
//! struct EmailWorker;
//!
//! #[async_trait]
//! impl Worker for EmailWorker {
//!     async fn execute(&self, job: Job) -> JobResult {
//!         // Send email...
//!         Ok(JobOutput::success("Email sent"))
//!     }
//! }
//!
//! // Submit a job
//! let job_manager = JobManager::new(store);
//! let job = JobSpec::new("send_email")
//!     .payload(serde_json::json!({ "to": "user@example.com" }))
//!     .priority(Priority::High);
//!
//! let job_id = job_manager.submit(job).await?;
//!
//! // Start worker pool
//! let pool = WorkerPool::new(store)
//!     .register_handler("send_email", EmailWorker)
//!     .start(4).await?;
//! ```

#![warn(missing_docs)]

mod affinity;
mod analytics;
mod blob_storage;
mod error;
mod job;
mod manager;
mod progress;
mod types;
mod worker;
mod workflow;

pub use affinity::{AffinityJobManager, AffinityStrategy, JobAffinity, WorkerMetadata};
pub use analytics::{JobAnalytics, AnalyticsQuery, AnalyticsResult, TimeWindow, GroupBy, AnalyticsDashboard, ExportFormat};
pub use blob_storage::{BlobJobManager, JobBlobStorage, BlobPayload, PayloadFormat, BlobCollection, BlobHash, BlobStats};
pub use error::{JobError, JobErrorKind};
pub use job::{Job, JobConfig, JobId, JobOutput, JobResult, JobSpec, JobStatus};
pub use manager::{JobManager, JobManagerConfig};
pub use progress::{CrdtProgressTracker, ProgressUpdate, JobProgress, ProgressCrdt, ProgressSyncManager};
pub use types::{Priority, RetryPolicy, Schedule};
pub use worker::{Worker, WorkerConfig, WorkerPool, WorkerStatus};
pub use workflow::{WorkflowManager, WorkflowDefinition, WorkflowStep, WorkflowTransition, TransitionCondition, WorkflowBuilder};