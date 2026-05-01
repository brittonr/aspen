# I1 Jobs public surface inventory

Generated from `crates/aspen-jobs/src/lib.rs` public re-exports. This first slice treats only pure job model/policy items as immediate `aspen-jobs-core` candidates; service implementations remain in `aspen-jobs` until the core fixture and compatibility re-export checks pass.

| Export | Classification |
| --- | --- |
| `pub use affinity::AffinityJobManager` | runtime shell / worker orchestration |
| `pub use affinity::AffinityStrategy` | runtime shell / worker orchestration |
| `pub use affinity::JobAffinity` | runtime shell / worker orchestration |
| `pub use affinity::WorkerMetadata` | runtime shell / worker orchestration |
| `pub use analytics::AnalyticsDashboard` | runtime shell / deferred service subsystem |
| `pub use analytics::AnalyticsQuery` | runtime shell / deferred service subsystem |
| `pub use analytics::AnalyticsResult` | runtime shell / deferred service subsystem |
| `pub use analytics::ExportFormat` | runtime shell / deferred service subsystem |
| `pub use analytics::GroupBy` | runtime shell / deferred service subsystem |
| `pub use analytics::JobAnalytics` | runtime shell / deferred service subsystem |
| `pub use analytics::TimeWindow` | runtime shell / deferred service subsystem |
| `pub use blob_storage::BlobCollection` | worker/executor adapter |
| `pub use blob_storage::BlobHash` | worker/executor adapter |
| `pub use blob_storage::BlobJobManager` | worker/executor adapter |
| `pub use blob_storage::BlobPayload` | worker/executor adapter |
| `pub use blob_storage::BlobStats` | worker/executor adapter |
| `pub use blob_storage::JobBlobStorage` | worker/executor adapter |
| `pub use blob_storage::PayloadFormat` | worker/executor adapter |
| `pub use dependency_tracker::DependencyFailurePolicy` | deferred orchestration / possible core helper after storage audit |
| `pub use dependency_tracker::DependencyGraph` | deferred orchestration / possible core helper after storage audit |
| `pub use dependency_tracker::DependencyState` | deferred orchestration / possible core helper after storage audit |
| `pub use dependency_tracker::JobDependencyInfo` | deferred orchestration / possible core helper after storage audit |
| `pub use distributed_pool::ClusterJobStats` | runtime shell / worker orchestration |
| `pub use distributed_pool::DistributedJobExt` | runtime shell / worker orchestration |
| `pub use distributed_pool::DistributedJobRouter` | runtime shell / worker orchestration |
| `pub use distributed_pool::DistributedPoolConfig` | runtime shell / worker orchestration |
| `pub use distributed_pool::DistributedWorkerPool` | runtime shell / worker orchestration |
| `pub use distributed_pool::GroupMessage` | runtime shell / worker orchestration |
| `pub use distributed_pool::WorkerGroupHandle` | runtime shell / worker orchestration |
| `pub use dlq_inspector::DlqAnalysis` | runtime shell / deferred service subsystem |
| `pub use dlq_inspector::DlqExport` | runtime shell / deferred service subsystem |
| `pub use dlq_inspector::DlqExportEntry` | runtime shell / deferred service subsystem |
| `pub use dlq_inspector::DlqInspector` | runtime shell / deferred service subsystem |
| `pub use dlq_inspector::DlqRecommendation` | runtime shell / deferred service subsystem |
| `pub use dlq_inspector::RecommendationSeverity` | runtime shell / deferred service subsystem |
| `pub use durable_executor::DurableWorkflowExecutor` | runtime shell / deferred service subsystem |
| `pub use durable_executor::DurableWorkflowStatus` | runtime shell / deferred service subsystem |
| `pub use durable_executor::WorkflowHandle` | runtime shell / deferred service subsystem |
| `pub use durable_timer::DurableTimer` | runtime shell / deferred service subsystem |
| `pub use durable_timer::DurableTimerManager` | runtime shell / deferred service subsystem |
| `pub use durable_timer::TimerFiredEvent` | runtime shell / deferred service subsystem |
| `pub use durable_timer::TimerId` | runtime shell / deferred service subsystem |
| `pub use durable_timer::TimerService` | runtime shell / deferred service subsystem |
| `pub use error::JobError` | compatibility shell error surface; possible later core split |
| `pub use error::JobErrorKind` | compatibility shell error surface; possible later core split |
| `pub use event_store::ActivityState` | runtime shell / deferred service subsystem |
| `pub use event_store::CachedActivityResult` | runtime shell / deferred service subsystem |
| `pub use event_store::CompensationEntry` | runtime shell / deferred service subsystem |
| `pub use event_store::TimerState` | runtime shell / deferred service subsystem |
| `pub use event_store::WorkflowEvent` | runtime shell / deferred service subsystem |
| `pub use event_store::WorkflowEventStore` | runtime shell / deferred service subsystem |
| `pub use event_store::WorkflowEventType` | runtime shell / deferred service subsystem |
| `pub use event_store::WorkflowExecutionId` | runtime shell / deferred service subsystem |
| `pub use event_store::WorkflowReplayEngine` | runtime shell / deferred service subsystem |
| `pub use event_store::WorkflowSnapshot` | runtime shell / deferred service subsystem |
| `pub use job::DlqMetadata` | reusable core candidate |
| `pub use job::DlqReason` | reusable core candidate |
| `pub use job::Job` | reusable core candidate |
| `pub use job::JobConfig` | reusable core candidate |
| `pub use job::JobFailure` | reusable core candidate |
| `pub use job::JobId` | reusable core candidate |
| `pub use job::JobOutput` | reusable core candidate |
| `pub use job::JobResult` | reusable core candidate |
| `pub use job::JobSpec` | reusable core candidate |
| `pub use job::JobStatus` | reusable core candidate |
| `pub use manager::JobCompletionCallback` | runtime shell / worker orchestration |
| `pub use manager::JobManager` | runtime shell / worker orchestration |
| `pub use manager::JobManagerConfig` | runtime shell / worker orchestration |
| `pub use monitoring::AggregatedMetrics` | runtime shell / deferred service subsystem |
| `pub use monitoring::AuditAction` | runtime shell / deferred service subsystem |
| `pub use monitoring::AuditFilter` | runtime shell / deferred service subsystem |
| `pub use monitoring::AuditLogEntry` | runtime shell / deferred service subsystem |
| `pub use monitoring::AuditResult` | runtime shell / deferred service subsystem |
| `pub use monitoring::Bottleneck` | runtime shell / deferred service subsystem |
| `pub use monitoring::BottleneckType` | runtime shell / deferred service subsystem |
| `pub use monitoring::JobMetrics` | runtime shell / deferred service subsystem |
| `pub use monitoring::JobMonitoringService` | runtime shell / deferred service subsystem |
| `pub use monitoring::JobProfile` | runtime shell / deferred service subsystem |
| `pub use monitoring::SpanEvent` | runtime shell / deferred service subsystem |
| `pub use monitoring::SpanLink` | runtime shell / deferred service subsystem |
| `pub use monitoring::SpanStatus` | runtime shell / deferred service subsystem |
| `pub use monitoring::TraceContext` | runtime shell / deferred service subsystem |
| `pub use monitoring::TraceFilter` | runtime shell / deferred service subsystem |
| `pub use monitoring::TraceSpan` | runtime shell / deferred service subsystem |
| `pub use parse::parse_schedule` | reusable core candidate |
| `pub use progress::CrdtProgressTracker` | runtime shell / deferred service subsystem |
| `pub use progress::JobProgress` | runtime shell / deferred service subsystem |
| `pub use progress::ProgressCrdt` | runtime shell / deferred service subsystem |
| `pub use progress::ProgressSyncManager` | runtime shell / deferred service subsystem |
| `pub use progress::ProgressUpdate` | runtime shell / deferred service subsystem |
| `pub use replay::DeterministicJobExecutor` | runtime shell / deferred service subsystem |
| `pub use replay::ExecutionRecord` | runtime shell / deferred service subsystem |
| `pub use replay::JobEvent` | runtime shell / deferred service subsystem |
| `pub use replay::JobReplaySystem` | runtime shell / deferred service subsystem |
| `pub use replay::ReplayConfig` | runtime shell / deferred service subsystem |
| `pub use replay::ReplayRunner` | runtime shell / deferred service subsystem |
| `pub use replay::ReplayStats` | runtime shell / deferred service subsystem |
| `pub use saga::CompensationResult` | runtime shell / deferred service subsystem |
| `pub use saga::SagaBuilder` | runtime shell / deferred service subsystem |
| `pub use saga::SagaDefinition` | runtime shell / deferred service subsystem |
| `pub use saga::SagaExecutor` | runtime shell / deferred service subsystem |
| `pub use saga::SagaState` | runtime shell / deferred service subsystem |
| `pub use saga::SagaStep` | runtime shell / deferred service subsystem |
| `pub use scheduler::CatchUpPolicy` | runtime shell / worker orchestration |
| `pub use scheduler::ConflictPolicy` | runtime shell / worker orchestration |
| `pub use scheduler::ScheduledJob` | reusable core candidate |
| `pub use scheduler::SchedulerConfig` | reusable core candidate |
| `pub use scheduler::SchedulerService` | reusable core candidate |
| `pub use system_info::DiskFreeReadings` | runtime shell / deferred service subsystem |
| `pub use system_info::PressureReadings` | runtime shell / deferred service subsystem |
| `pub use system_info::read_disk_free` | runtime shell / deferred service subsystem |
| `pub use system_info::read_psi_pressure` | runtime shell / deferred service subsystem |
| `pub use traced_worker::TracedWorker` | runtime shell / worker orchestration |
| `pub use traced_worker::WorkerTracingExt` | runtime shell / worker orchestration |
| `pub use tracing::AttributeValue` | runtime shell / deferred service subsystem |
| `pub use tracing::Baggage` | runtime shell / deferred service subsystem |
| `pub use tracing::ConsoleExporter` | runtime shell / deferred service subsystem |
| `pub use tracing::DistributedSpan` | runtime shell / deferred service subsystem |
| `pub use tracing::DistributedTraceContext` | runtime shell / deferred service subsystem |
| `pub use tracing::DistributedTracingService` | runtime shell / deferred service subsystem |
| `pub use tracing::OtlpExporter` | runtime shell / deferred service subsystem |
| `pub use tracing::SamplingStrategy` | runtime shell / deferred service subsystem |
| `pub use tracing::SpanId` | runtime shell / deferred service subsystem |
| `pub use tracing::SpanKind` | runtime shell / deferred service subsystem |
| `pub use tracing::TraceExporter` | runtime shell / deferred service subsystem |
| `pub use tracing::TraceFlags` | runtime shell / deferred service subsystem |
| `pub use tracing::TraceId` | runtime shell / deferred service subsystem |
| `pub use tracing::TracedJobOperation` | runtime shell / deferred service subsystem |
| `pub use types::DLQStats` | reusable core candidate |
| `pub use types::Priority` | reusable core candidate |
| `pub use types::QueueStats` | reusable core candidate |
| `pub use types::RetryPolicy` | reusable core candidate |
| `pub use types::Schedule` | reusable core candidate |
| `pub use vm_executor::HyperlightWorker` | worker/executor adapter |
| `pub use vm_executor::JobPayload as VmJobPayload` | worker/executor adapter |
| `pub use vm_executor::WasmComponentWorker` | worker/executor adapter |
| `pub use worker::Worker` | runtime shell / worker orchestration |
| `pub use worker::WorkerConfig` | runtime shell / worker orchestration |
| `pub use worker::WorkerInfo` | runtime shell / worker orchestration |
| `pub use worker::WorkerPool` | runtime shell / worker orchestration |
| `pub use worker::WorkerPoolStats` | runtime shell / worker orchestration |
| `pub use worker::WorkerStatus` | runtime shell / worker orchestration |
| `pub use workflow::TransitionCondition` | needs manual review |
| `pub use workflow::WorkflowBuilder` | needs manual review |
| `pub use workflow::WorkflowDefinition` | needs manual review |
| `pub use workflow::WorkflowManager` | needs manual review |
| `pub use workflow::WorkflowStep` | needs manual review |
| `pub use workflow::WorkflowTransition` | needs manual review |

## First-slice core candidates

- `job::{JobId, JobStatus, JobResult, JobOutput, JobFailure, JobConfig, JobSpec, DlqMetadata, DlqReason}`
- `types::{Priority, RetryPolicy, Schedule, QueueStats, DLQStats}`
- Pure parse/schedule descriptors only if they do not require ambient time or cron runtime evaluation.

## Explicitly deferred

- `JobManager`, `SchedulerService`, worker pools, distributed routing, Redb/storage-backed state, monitoring/tracing, durable workflow execution, saga executor, blob and VM/plugin adapters.
- `dependency_tracker` remains deferred until its storage/runtime coupling is audited; pure dependency-state enums may move earlier if they have no manager coupling.
