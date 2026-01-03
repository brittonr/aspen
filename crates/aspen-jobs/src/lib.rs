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
mod dependency_tracker;
mod distributed_pool;
mod dlq_inspector;
mod error;
mod job;
mod manager;
mod monitoring;
mod parse;
mod progress;
mod replay;
mod scheduler;
mod traced_worker;
mod tracing;
mod types;
mod worker;
pub mod workers;
mod workflow;

#[cfg(feature = "vm-executor")]
pub mod vm_executor;

pub use affinity::{AffinityJobManager, AffinityStrategy, JobAffinity, WorkerMetadata};
pub use analytics::{
    AnalyticsDashboard, AnalyticsQuery, AnalyticsResult, ExportFormat, GroupBy, JobAnalytics, TimeWindow,
};
pub use blob_storage::{
    BlobCollection, BlobHash, BlobJobManager, BlobPayload, BlobStats, JobBlobStorage, PayloadFormat,
};
pub use dependency_tracker::{DependencyFailurePolicy, DependencyGraph, DependencyState, JobDependencyInfo};
pub use distributed_pool::{
    ClusterJobStats, DistributedJobExt, DistributedJobRouter, DistributedPoolConfig, DistributedWorkerPool,
    GroupMessage, WorkerGroupHandle,
};
pub use dlq_inspector::{
    DLQAnalysis, DLQExport, DLQExportEntry, DLQInspector, DLQRecommendation, RecommendationSeverity,
};
pub use error::{JobError, JobErrorKind};
pub use job::{DLQMetadata, DLQReason, Job, JobConfig, JobId, JobOutput, JobResult, JobSpec, JobStatus};
pub use manager::{JobManager, JobManagerConfig};
pub use monitoring::{
    AggregatedMetrics, AuditAction, AuditFilter, AuditLogEntry, AuditResult, Bottleneck, BottleneckType, JobMetrics,
    JobMonitoringService, JobProfile, SpanEvent, SpanLink, SpanStatus, TraceContext, TraceFilter, TraceSpan,
};
pub use parse::parse_schedule;
pub use progress::{CrdtProgressTracker, JobProgress, ProgressCrdt, ProgressSyncManager, ProgressUpdate};
pub use replay::{
    DeterministicJobExecutor, ExecutionRecord, JobEvent, JobReplaySystem, ReplayConfig, ReplayRunner, ReplayStats,
};
pub use scheduler::{CatchUpPolicy, ConflictPolicy, ScheduledJob, SchedulerConfig, SchedulerService};
pub use traced_worker::{TracedWorker, WorkerTracingExt};
pub use tracing::{
    AttributeValue, Baggage, ConsoleExporter, DistributedSpan, DistributedTraceContext, DistributedTracingService,
    OtlpExporter, SamplingStrategy, SpanId, SpanKind, TraceExporter, TraceFlags, TraceId, TracedJobOperation,
};
pub use types::{DLQStats, Priority, QueueStats, RetryPolicy, Schedule};
pub use worker::{Worker, WorkerConfig, WorkerInfo, WorkerPool, WorkerPoolStats, WorkerStatus};
pub use workflow::{
    TransitionCondition, WorkflowBuilder, WorkflowDefinition, WorkflowManager, WorkflowStep, WorkflowTransition,
};

#[cfg(feature = "vm-executor")]
pub use vm_executor::{HyperlightWorker, JobPayload as VmJobPayload};
