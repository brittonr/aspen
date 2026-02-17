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
// Allow nested ifs for readability - many patterns in job state machines are clearer with nesting
#![allow(clippy::collapsible_if)]

mod affinity;
mod analytics;
mod blob_storage;
mod dependency_tracker;
mod distributed_pool;
mod dlq_inspector;
mod durable_timer;
mod error;
mod event_store;
mod job;
mod manager;
mod monitoring;
mod parse;
mod progress;
mod replay;
mod saga;
mod scheduler;
mod traced_worker;
mod tracing;
mod types;
/// Verified pure functions for job system logic.
pub mod verified;
mod worker;
pub mod workers;
mod workflow;

#[cfg(any(feature = "vm-executor", feature = "wasm-component", feature = "nanvix-executor"))]
pub mod vm_executor;

pub use affinity::AffinityJobManager;
pub use affinity::AffinityStrategy;
pub use affinity::JobAffinity;
pub use affinity::WorkerMetadata;
pub use analytics::AnalyticsDashboard;
pub use analytics::AnalyticsQuery;
pub use analytics::AnalyticsResult;
pub use analytics::ExportFormat;
pub use analytics::GroupBy;
pub use analytics::JobAnalytics;
pub use analytics::TimeWindow;
pub use blob_storage::BlobCollection;
pub use blob_storage::BlobHash;
pub use blob_storage::BlobJobManager;
pub use blob_storage::BlobPayload;
pub use blob_storage::BlobStats;
pub use blob_storage::JobBlobStorage;
pub use blob_storage::PayloadFormat;
pub use dependency_tracker::DependencyFailurePolicy;
pub use dependency_tracker::DependencyGraph;
pub use dependency_tracker::DependencyState;
pub use dependency_tracker::JobDependencyInfo;
pub use distributed_pool::ClusterJobStats;
pub use distributed_pool::DistributedJobExt;
pub use distributed_pool::DistributedJobRouter;
pub use distributed_pool::DistributedPoolConfig;
pub use distributed_pool::DistributedWorkerPool;
pub use distributed_pool::GroupMessage;
pub use distributed_pool::WorkerGroupHandle;
pub use dlq_inspector::DLQAnalysis;
pub use dlq_inspector::DLQExport;
pub use dlq_inspector::DLQExportEntry;
pub use dlq_inspector::DLQInspector;
pub use dlq_inspector::DLQRecommendation;
pub use dlq_inspector::RecommendationSeverity;
// Durable timers
pub use durable_timer::DurableTimer;
pub use durable_timer::DurableTimerManager;
pub use durable_timer::TimerFiredEvent;
pub use durable_timer::TimerId;
pub use durable_timer::TimerService;
pub use error::JobError;
pub use error::JobErrorKind;
// Event sourcing
pub use event_store::ActivityState;
pub use event_store::CachedActivityResult;
pub use event_store::CompensationEntry;
pub use event_store::TimerState;
pub use event_store::WorkflowEvent;
pub use event_store::WorkflowEventStore;
pub use event_store::WorkflowEventType;
pub use event_store::WorkflowExecutionId;
pub use event_store::WorkflowReplayEngine;
pub use event_store::WorkflowSnapshot;
pub use job::DLQMetadata;
pub use job::DLQReason;
pub use job::Job;
pub use job::JobConfig;
pub use job::JobFailure;
pub use job::JobId;
pub use job::JobOutput;
pub use job::JobResult;
pub use job::JobSpec;
pub use job::JobStatus;
pub use manager::JobCompletionCallback;
pub use manager::JobManager;
pub use manager::JobManagerConfig;
pub use monitoring::AggregatedMetrics;
pub use monitoring::AuditAction;
pub use monitoring::AuditFilter;
pub use monitoring::AuditLogEntry;
pub use monitoring::AuditResult;
pub use monitoring::Bottleneck;
pub use monitoring::BottleneckType;
pub use monitoring::JobMetrics;
pub use monitoring::JobMonitoringService;
pub use monitoring::JobProfile;
pub use monitoring::SpanEvent;
pub use monitoring::SpanLink;
pub use monitoring::SpanStatus;
pub use monitoring::TraceContext;
pub use monitoring::TraceFilter;
pub use monitoring::TraceSpan;
pub use parse::parse_schedule;
pub use progress::CrdtProgressTracker;
pub use progress::JobProgress;
pub use progress::ProgressCrdt;
pub use progress::ProgressSyncManager;
pub use progress::ProgressUpdate;
pub use replay::DeterministicJobExecutor;
pub use replay::ExecutionRecord;
pub use replay::JobEvent;
pub use replay::JobReplaySystem;
pub use replay::ReplayConfig;
pub use replay::ReplayRunner;
pub use replay::ReplayStats;
// Saga pattern
pub use saga::CompensationResult;
pub use saga::SagaBuilder;
pub use saga::SagaDefinition;
pub use saga::SagaExecutor;
pub use saga::SagaState;
pub use saga::SagaStep;
pub use scheduler::CatchUpPolicy;
pub use scheduler::ConflictPolicy;
pub use scheduler::ScheduledJob;
pub use scheduler::SchedulerConfig;
pub use scheduler::SchedulerService;
pub use traced_worker::TracedWorker;
pub use traced_worker::WorkerTracingExt;
pub use tracing::AttributeValue;
pub use tracing::Baggage;
pub use tracing::ConsoleExporter;
pub use tracing::DistributedSpan;
pub use tracing::DistributedTraceContext;
pub use tracing::DistributedTracingService;
pub use tracing::OtlpExporter;
pub use tracing::SamplingStrategy;
pub use tracing::SpanId;
pub use tracing::SpanKind;
pub use tracing::TraceExporter;
pub use tracing::TraceFlags;
pub use tracing::TraceId;
pub use tracing::TracedJobOperation;
pub use types::DLQStats;
pub use types::Priority;
pub use types::QueueStats;
pub use types::RetryPolicy;
pub use types::Schedule;
#[cfg(feature = "vm-executor")]
pub use vm_executor::HyperlightWorker;
#[cfg(any(feature = "vm-executor", feature = "wasm-component", feature = "nanvix-executor"))]
pub use vm_executor::JobPayload as VmJobPayload;
#[cfg(feature = "nanvix-executor")]
pub use vm_executor::NanvixWorker;
#[cfg(feature = "wasm-component")]
pub use vm_executor::WasmComponentWorker;
pub use worker::Worker;
pub use worker::WorkerConfig;
pub use worker::WorkerInfo;
pub use worker::WorkerPool;
pub use worker::WorkerPoolStats;
pub use worker::WorkerStatus;
pub use workflow::TransitionCondition;
pub use workflow::WorkflowBuilder;
pub use workflow::WorkflowDefinition;
pub use workflow::WorkflowManager;
pub use workflow::WorkflowStep;
pub use workflow::WorkflowTransition;
