//! Job monitoring and observability system.
//!
//! This module provides comprehensive monitoring, tracing, and metrics collection
//! for distributed job execution across the cluster.
//!
//! ## Features
//!
//! - Distributed tracing with OpenTelemetry compatibility
//! - Metrics aggregation with Prometheus export
//! - Job execution history and audit logs
//! - Performance profiling and bottleneck detection
//! - Real-time monitoring dashboards
//! - Alerting and anomaly detection
//!
//! ## Tiger Style
//!
//! - Bounded metrics storage (MAX_METRICS_PER_NODE = 10000)
//! - Fixed retention periods for historical data
//! - Fail-fast on monitoring errors (don't block job execution)
//! - Efficient sampling for high-throughput scenarios

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::error::{JobError, Result};
use crate::job::{Job, JobId, JobStatus};

/// Maximum metrics retained per node.
const MAX_METRICS_PER_NODE: usize = 10_000;

/// Maximum trace spans retained.
const MAX_TRACE_SPANS: usize = 100_000;

/// Maximum audit log entries.
const MAX_AUDIT_LOG_ENTRIES: usize = 50_000;

/// Default metrics window.
const DEFAULT_METRICS_WINDOW: Duration = Duration::from_secs(300); // 5 minutes

/// Trace context for distributed tracing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceContext {
    /// Trace ID (unique across cluster).
    pub trace_id: String,
    /// Parent span ID.
    pub parent_span_id: Option<String>,
    /// Current span ID.
    pub span_id: String,
    /// Trace flags.
    pub flags: u8,
    /// Baggage items.
    pub baggage: HashMap<String, String>,
}

impl TraceContext {
    /// Create a new root trace.
    pub fn new_root() -> Self {
        Self {
            trace_id: uuid::Uuid::new_v4().to_string(),
            parent_span_id: None,
            span_id: uuid::Uuid::new_v4().to_string(),
            flags: 1, // Sampled
            baggage: HashMap::new(),
        }
    }

    /// Create a child span.
    pub fn child(&self) -> Self {
        Self {
            trace_id: self.trace_id.clone(),
            parent_span_id: Some(self.span_id.clone()),
            span_id: uuid::Uuid::new_v4().to_string(),
            flags: self.flags,
            baggage: self.baggage.clone(),
        }
    }
}

/// Span for distributed tracing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSpan {
    /// Span ID.
    pub span_id: String,
    /// Trace ID.
    pub trace_id: String,
    /// Parent span ID.
    pub parent_span_id: Option<String>,
    /// Operation name.
    pub operation: String,
    /// Service name.
    pub service: String,
    /// Start time.
    pub start_time: DateTime<Utc>,
    /// End time.
    pub end_time: Option<DateTime<Utc>>,
    /// Duration in microseconds.
    pub duration_us: Option<u64>,
    /// Span status.
    pub status: SpanStatus,
    /// Attributes.
    pub attributes: HashMap<String, String>,
    /// Events.
    pub events: Vec<SpanEvent>,
    /// Links to other spans.
    pub links: Vec<SpanLink>,
}

/// Span status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpanStatus {
    /// Unset status.
    Unset,
    /// Operation succeeded.
    Ok,
    /// Operation failed.
    Error,
}

/// Event within a span.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanEvent {
    /// Event name.
    pub name: String,
    /// Event timestamp.
    pub timestamp: DateTime<Utc>,
    /// Event attributes.
    pub attributes: HashMap<String, String>,
}

/// Link to another span.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanLink {
    /// Linked trace ID.
    pub trace_id: String,
    /// Linked span ID.
    pub span_id: String,
    /// Link attributes.
    pub attributes: HashMap<String, String>,
}

/// Job execution metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobMetrics {
    /// Job ID.
    pub job_id: JobId,
    /// Job type.
    pub job_type: String,
    /// Worker ID.
    pub worker_id: String,
    /// Node ID.
    pub node_id: String,
    /// Queue time (ms).
    pub queue_time_ms: u64,
    /// Execution time (ms).
    pub execution_time_ms: u64,
    /// Total time (ms).
    pub total_time_ms: u64,
    /// CPU usage (percentage).
    pub cpu_usage: f32,
    /// Memory usage (bytes).
    pub memory_bytes: u64,
    /// Network bytes sent.
    pub network_sent_bytes: u64,
    /// Network bytes received.
    pub network_recv_bytes: u64,
    /// Retry count.
    pub retry_count: u32,
    /// Success flag.
    pub success: bool,
    /// Error message if failed.
    pub error_message: Option<String>,
    /// Custom metrics.
    pub custom: HashMap<String, f64>,
}

/// Aggregated metrics over a time window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedMetrics {
    /// Time window start.
    pub window_start: DateTime<Utc>,
    /// Time window end.
    pub window_end: DateTime<Utc>,
    /// Total jobs processed.
    pub total_jobs: u64,
    /// Successful jobs.
    pub successful_jobs: u64,
    /// Failed jobs.
    pub failed_jobs: u64,
    /// Average queue time (ms).
    pub avg_queue_time_ms: f64,
    /// Average execution time (ms).
    pub avg_execution_time_ms: f64,
    /// P50 execution time (ms).
    pub p50_execution_time_ms: u64,
    /// P95 execution time (ms).
    pub p95_execution_time_ms: u64,
    /// P99 execution time (ms).
    pub p99_execution_time_ms: u64,
    /// Jobs per second.
    pub jobs_per_second: f64,
    /// Success rate.
    pub success_rate: f64,
    /// Top job types by count.
    pub top_job_types: Vec<(String, u64)>,
    /// Top error reasons.
    pub top_errors: Vec<(String, u64)>,
}

/// Audit log entry for job operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// Entry ID.
    pub id: String,
    /// Timestamp.
    pub timestamp: DateTime<Utc>,
    /// Job ID.
    pub job_id: Option<JobId>,
    /// User/system that triggered the action.
    pub actor: String,
    /// Action performed.
    pub action: AuditAction,
    /// Resource affected.
    pub resource: String,
    /// Old value (for updates).
    pub old_value: Option<String>,
    /// New value (for updates).
    pub new_value: Option<String>,
    /// Result of the action.
    pub result: AuditResult,
    /// Additional metadata.
    pub metadata: HashMap<String, String>,
}

/// Audit action types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditAction {
    /// Job created.
    JobCreated,
    /// Job updated.
    JobUpdated,
    /// Job deleted.
    JobDeleted,
    /// Job started.
    JobStarted,
    /// Job completed.
    JobCompleted,
    /// Job failed.
    JobFailed,
    /// Job retried.
    JobRetried,
    /// Job cancelled.
    JobCancelled,
    /// Worker registered.
    WorkerRegistered,
    /// Worker deregistered.
    WorkerDeregistered,
    /// Configuration changed.
    ConfigChanged,
}

/// Result of an audited action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditResult {
    /// Action succeeded.
    Success,
    /// Action failed.
    Failure,
    /// Action partially succeeded.
    Partial,
}

/// Performance profile for a job execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobProfile {
    /// Job ID.
    pub job_id: JobId,
    /// Total execution time.
    pub total_time: Duration,
    /// Time breakdown by phase.
    pub phases: Vec<ProfilePhase>,
    /// Resource usage samples.
    pub resource_samples: Vec<ResourceSample>,
    /// Bottlenecks detected.
    pub bottlenecks: Vec<Bottleneck>,
    /// Recommendations.
    pub recommendations: Vec<String>,
}

/// Phase in job execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilePhase {
    /// Phase name.
    pub name: String,
    /// Start time (relative to job start).
    pub start_offset: Duration,
    /// Duration.
    pub duration: Duration,
    /// CPU time.
    pub cpu_time: Duration,
    /// Wait time.
    pub wait_time: Duration,
    /// I/O time.
    pub io_time: Duration,
}

/// Resource usage sample.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSample {
    /// Sample time (relative to job start).
    pub time_offset: Duration,
    /// CPU usage (percentage).
    pub cpu_percent: f32,
    /// Memory usage (bytes).
    pub memory_bytes: u64,
    /// Disk I/O (bytes/sec).
    pub disk_io_rate: u64,
    /// Network I/O (bytes/sec).
    pub network_io_rate: u64,
}

/// Detected performance bottleneck.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bottleneck {
    /// Bottleneck type.
    pub bottleneck_type: BottleneckType,
    /// Severity (0-100).
    pub severity: u8,
    /// Description.
    pub description: String,
    /// Affected phase.
    pub phase: Option<String>,
    /// Duration of bottleneck.
    pub duration: Duration,
}

/// Types of performance bottlenecks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BottleneckType {
    /// CPU-bound.
    CpuBound,
    /// Memory pressure.
    MemoryPressure,
    /// I/O wait.
    IoWait,
    /// Network latency.
    NetworkLatency,
    /// Lock contention.
    LockContention,
    /// Queue backup.
    QueueBackup,
}

/// Job monitoring service.
pub struct JobMonitoringService<S: aspen_core::KeyValueStore + ?Sized> {
    /// Key-value store for persistence.
    store: Arc<S>,
    /// Active trace spans.
    spans: Arc<RwLock<HashMap<String, TraceSpan>>>,
    /// Completed spans (bounded).
    completed_spans: Arc<RwLock<VecDeque<TraceSpan>>>,
    /// Job metrics (bounded).
    metrics: Arc<RwLock<VecDeque<JobMetrics>>>,
    /// Audit log (bounded).
    audit_log: Arc<RwLock<VecDeque<AuditLogEntry>>>,
    /// Active job profiles.
    profiles: Arc<RwLock<HashMap<JobId, JobProfileBuilder>>>,
    /// Metrics aggregator.
    aggregator: Arc<RwLock<MetricsAggregator>>,
}

impl<S: aspen_core::KeyValueStore + ?Sized + 'static> JobMonitoringService<S> {
    /// Create a new monitoring service.
    pub fn new(store: Arc<S>) -> Self {
        Self {
            store,
            spans: Arc::new(RwLock::new(HashMap::new())),
            completed_spans: Arc::new(RwLock::new(VecDeque::new())),
            metrics: Arc::new(RwLock::new(VecDeque::new())),
            audit_log: Arc::new(RwLock::new(VecDeque::new())),
            profiles: Arc::new(RwLock::new(HashMap::new())),
            aggregator: Arc::new(RwLock::new(MetricsAggregator::new())),
        }
    }

    /// Start a new trace span.
    pub async fn start_span(
        &self,
        trace_ctx: &TraceContext,
        operation: &str,
        service: &str,
    ) -> String {
        let span = TraceSpan {
            span_id: trace_ctx.span_id.clone(),
            trace_id: trace_ctx.trace_id.clone(),
            parent_span_id: trace_ctx.parent_span_id.clone(),
            operation: operation.to_string(),
            service: service.to_string(),
            start_time: Utc::now(),
            end_time: None,
            duration_us: None,
            status: SpanStatus::Unset,
            attributes: HashMap::new(),
            events: Vec::new(),
            links: Vec::new(),
        };

        let span_id = span.span_id.clone();

        let mut spans = self.spans.write().await;
        spans.insert(span_id.clone(), span);

        debug!(
            span_id = %span_id,
            operation,
            service,
            "started trace span"
        );

        span_id
    }

    /// End a trace span.
    pub async fn end_span(&self, span_id: &str, status: SpanStatus) -> Result<()> {
        let mut spans = self.spans.write().await;

        if let Some(mut span) = spans.remove(span_id) {
            let end_time = Utc::now();
            let duration = end_time.timestamp_micros() - span.start_time.timestamp_micros();

            span.end_time = Some(end_time);
            span.duration_us = Some(duration as u64);
            span.status = status;

            // Move to completed spans
            let mut completed = self.completed_spans.write().await;

            // Enforce bounded storage
            while completed.len() >= MAX_TRACE_SPANS {
                completed.pop_front();
            }

            completed.push_back(span.clone());

            debug!(
                span_id,
                duration_us = duration,
                status = ?status,
                "ended trace span"
            );

            // Persist important spans
            if duration > 1_000_000 { // > 1 second
                self.persist_span(&span).await?;
            }
        }

        Ok(())
    }

    /// Add an event to a span.
    pub async fn add_span_event(&self, span_id: &str, event: SpanEvent) -> Result<()> {
        let mut spans = self.spans.write().await;

        if let Some(span) = spans.get_mut(span_id) {
            span.events.push(event);
        }

        Ok(())
    }

    /// Record job metrics.
    pub async fn record_metrics(&self, metrics: JobMetrics) -> Result<()> {
        debug!(
            job_id = %metrics.job_id,
            job_type = %metrics.job_type,
            success = metrics.success,
            "recording job metrics"
        );

        // Add to aggregator
        self.aggregator.write().await.add_metrics(&metrics);

        // Store in bounded queue
        let mut metrics_queue = self.metrics.write().await;

        while metrics_queue.len() >= MAX_METRICS_PER_NODE {
            metrics_queue.pop_front();
        }

        metrics_queue.push_back(metrics.clone());

        // Persist metrics
        self.persist_metrics(&metrics).await?;

        Ok(())
    }

    /// Get aggregated metrics for a time window.
    pub async fn get_aggregated_metrics(
        &self,
        window: Duration,
    ) -> Result<AggregatedMetrics> {
        self.aggregator.read().await.get_aggregated(window)
    }

    /// Add an audit log entry.
    pub async fn audit_log(
        &self,
        action: AuditAction,
        actor: &str,
        resource: &str,
        result: AuditResult,
        metadata: HashMap<String, String>,
    ) -> Result<()> {
        let entry = AuditLogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            job_id: metadata.get("job_id").and_then(|id| JobId::parse(id).ok()),
            actor: actor.to_string(),
            action,
            resource: resource.to_string(),
            old_value: metadata.get("old_value").cloned(),
            new_value: metadata.get("new_value").cloned(),
            result,
            metadata,
        };

        debug!(
            action = ?action,
            actor,
            resource,
            result = ?result,
            "audit log entry"
        );

        // Store in bounded log
        let mut audit_log = self.audit_log.write().await;

        while audit_log.len() >= MAX_AUDIT_LOG_ENTRIES {
            audit_log.pop_front();
        }

        audit_log.push_back(entry.clone());

        // Persist audit entry
        self.persist_audit_entry(&entry).await?;

        Ok(())
    }

    /// Start profiling a job.
    pub async fn start_profiling(&self, job_id: JobId) -> Result<()> {
        let mut profiles = self.profiles.write().await;

        let builder = JobProfileBuilder::new(job_id.clone());
        profiles.insert(job_id, builder);

        Ok(())
    }

    /// Record a profile phase.
    pub async fn record_profile_phase(
        &self,
        job_id: &JobId,
        phase: ProfilePhase,
    ) -> Result<()> {
        let mut profiles = self.profiles.write().await;

        if let Some(builder) = profiles.get_mut(job_id) {
            builder.add_phase(phase);
        }

        Ok(())
    }

    /// Record a resource sample.
    pub async fn record_resource_sample(
        &self,
        job_id: &JobId,
        sample: ResourceSample,
    ) -> Result<()> {
        let mut profiles = self.profiles.write().await;

        if let Some(builder) = profiles.get_mut(job_id) {
            builder.add_sample(sample);
        }

        Ok(())
    }

    /// Finish profiling and analyze.
    pub async fn finish_profiling(&self, job_id: &JobId) -> Result<JobProfile> {
        let mut profiles = self.profiles.write().await;

        if let Some(builder) = profiles.remove(job_id) {
            let profile = builder.build();

            // Persist profile
            self.persist_profile(&profile).await?;

            Ok(profile)
        } else {
            Err(JobError::JobNotFound {
                id: job_id.to_string(),
            })
        }
    }

    /// Query historical traces.
    pub async fn query_traces(
        &self,
        filter: TraceFilter,
    ) -> Result<Vec<TraceSpan>> {
        let completed = self.completed_spans.read().await;

        let filtered: Vec<_> = completed
            .iter()
            .filter(|span| filter.matches(span))
            .cloned()
            .collect();

        Ok(filtered)
    }

    /// Query audit log.
    pub async fn query_audit_log(
        &self,
        filter: AuditFilter,
    ) -> Result<Vec<AuditLogEntry>> {
        let audit_log = self.audit_log.read().await;

        let filtered: Vec<_> = audit_log
            .iter()
            .filter(|entry| filter.matches(entry))
            .cloned()
            .collect();

        Ok(filtered)
    }

    /// Get Prometheus metrics.
    pub async fn export_prometheus(&self) -> String {
        let metrics = self.metrics.read().await;
        let aggregated = self.aggregator.read().await;

        let mut output = String::new();

        // Job metrics
        output.push_str("# HELP job_execution_duration_seconds Job execution duration\n");
        output.push_str("# TYPE job_execution_duration_seconds histogram\n");

        for m in metrics.iter().take(100) {
            output.push_str(&format!(
                "job_execution_duration_seconds{{job_type=\"{}\",status=\"{}\"}} {}\n",
                m.job_type,
                if m.success { "success" } else { "failure" },
                m.execution_time_ms as f64 / 1000.0
            ));
        }

        // Aggregated metrics
        output.push_str("\n# HELP job_total Total number of jobs\n");
        output.push_str("# TYPE job_total counter\n");
        output.push_str(&format!("job_total {}\n", aggregated.total_count));

        output.push_str("\n# HELP job_success_rate Job success rate\n");
        output.push_str("# TYPE job_success_rate gauge\n");
        output.push_str(&format!("job_success_rate {}\n", aggregated.success_rate()));

        output
    }

    // Internal persistence methods

    async fn persist_span(&self, span: &TraceSpan) -> Result<()> {
        let key = format!("trace:span:{}", span.span_id);
        let value = serde_json::to_string(span)?;

        self.store.write(aspen_core::WriteRequest {
            command: aspen_core::WriteCommand::Set { key, value },
        }).await?;

        Ok(())
    }

    async fn persist_metrics(&self, metrics: &JobMetrics) -> Result<()> {
        let key = format!("metrics:job:{}:{}", metrics.job_id, Utc::now().timestamp());
        let value = serde_json::to_string(metrics)?;

        self.store.write(aspen_core::WriteRequest {
            command: aspen_core::WriteCommand::SetWithTTL {
                key,
                value,
                ttl_seconds: 86400, // 24 hours
            },
        }).await?;

        Ok(())
    }

    async fn persist_audit_entry(&self, entry: &AuditLogEntry) -> Result<()> {
        let key = format!("audit:{}", entry.id);
        let value = serde_json::to_string(entry)?;

        self.store.write(aspen_core::WriteRequest {
            command: aspen_core::WriteCommand::Set { key, value },
        }).await?;

        Ok(())
    }

    async fn persist_profile(&self, profile: &JobProfile) -> Result<()> {
        let key = format!("profile:job:{}", profile.job_id);
        let value = serde_json::to_string(profile)?;

        self.store.write(aspen_core::WriteRequest {
            command: aspen_core::WriteCommand::SetWithTTL {
                key,
                value,
                ttl_seconds: 604800, // 7 days
            },
        }).await?;

        Ok(())
    }
}

/// Filter for querying traces.
#[derive(Debug, Clone, Default)]
pub struct TraceFilter {
    /// Filter by trace ID.
    pub trace_id: Option<String>,
    /// Filter by operation.
    pub operation: Option<String>,
    /// Filter by service.
    pub service: Option<String>,
    /// Filter by minimum duration.
    pub min_duration_us: Option<u64>,
    /// Filter by status.
    pub status: Option<SpanStatus>,
}

impl TraceFilter {
    fn matches(&self, span: &TraceSpan) -> bool {
        if let Some(ref trace_id) = self.trace_id {
            if &span.trace_id != trace_id {
                return false;
            }
        }

        if let Some(ref operation) = self.operation {
            if &span.operation != operation {
                return false;
            }
        }

        if let Some(ref service) = self.service {
            if &span.service != service {
                return false;
            }
        }

        if let Some(min_duration) = self.min_duration_us {
            if let Some(duration) = span.duration_us {
                if duration < min_duration {
                    return false;
                }
            }
        }

        if let Some(status) = self.status {
            if span.status != status {
                return false;
            }
        }

        true
    }
}

/// Filter for querying audit log.
#[derive(Debug, Clone, Default)]
pub struct AuditFilter {
    /// Filter by action.
    pub action: Option<AuditAction>,
    /// Filter by actor.
    pub actor: Option<String>,
    /// Filter by resource.
    pub resource: Option<String>,
    /// Filter by result.
    pub result: Option<AuditResult>,
    /// Filter by time range.
    pub since: Option<DateTime<Utc>>,
}

impl AuditFilter {
    fn matches(&self, entry: &AuditLogEntry) -> bool {
        if let Some(action) = self.action {
            if entry.action != action {
                return false;
            }
        }

        if let Some(ref actor) = self.actor {
            if &entry.actor != actor {
                return false;
            }
        }

        if let Some(ref resource) = self.resource {
            if &entry.resource != resource {
                return false;
            }
        }

        if let Some(result) = self.result {
            if entry.result != result {
                return false;
            }
        }

        if let Some(since) = self.since {
            if entry.timestamp < since {
                return false;
            }
        }

        true
    }
}

/// Builder for job profiles.
struct JobProfileBuilder {
    job_id: JobId,
    start_time: Instant,
    phases: Vec<ProfilePhase>,
    samples: Vec<ResourceSample>,
}

impl JobProfileBuilder {
    fn new(job_id: JobId) -> Self {
        Self {
            job_id,
            start_time: Instant::now(),
            phases: Vec::new(),
            samples: Vec::new(),
        }
    }

    fn add_phase(&mut self, phase: ProfilePhase) {
        self.phases.push(phase);
    }

    fn add_sample(&mut self, sample: ResourceSample) {
        self.samples.push(sample);
    }

    fn build(self) -> JobProfile {
        let total_time = self.start_time.elapsed();

        // Analyze for bottlenecks
        let bottlenecks = self.detect_bottlenecks();

        // Generate recommendations
        let recommendations = self.generate_recommendations(&bottlenecks);

        JobProfile {
            job_id: self.job_id,
            total_time,
            phases: self.phases,
            resource_samples: self.samples,
            bottlenecks,
            recommendations,
        }
    }

    fn detect_bottlenecks(&self) -> Vec<Bottleneck> {
        let mut bottlenecks = Vec::new();

        // Check for CPU bottlenecks
        let high_cpu_samples = self.samples.iter()
            .filter(|s| s.cpu_percent > 90.0)
            .count();

        if high_cpu_samples > self.samples.len() / 2 {
            bottlenecks.push(Bottleneck {
                bottleneck_type: BottleneckType::CpuBound,
                severity: 80,
                description: "High CPU usage detected".to_string(),
                phase: None,
                duration: Duration::from_secs(high_cpu_samples as u64),
            });
        }

        // Check for I/O wait
        for phase in &self.phases {
            if phase.io_time > phase.duration / 2 {
                bottlenecks.push(Bottleneck {
                    bottleneck_type: BottleneckType::IoWait,
                    severity: 70,
                    description: format!("High I/O wait in phase {}", phase.name),
                    phase: Some(phase.name.clone()),
                    duration: phase.io_time,
                });
            }
        }

        bottlenecks
    }

    fn generate_recommendations(&self, bottlenecks: &[Bottleneck]) -> Vec<String> {
        let mut recommendations = Vec::new();

        for bottleneck in bottlenecks {
            match bottleneck.bottleneck_type {
                BottleneckType::CpuBound => {
                    recommendations.push("Consider parallelizing CPU-intensive operations".to_string());
                }
                BottleneckType::IoWait => {
                    recommendations.push("Consider using async I/O or batching operations".to_string());
                }
                BottleneckType::MemoryPressure => {
                    recommendations.push("Consider streaming data instead of loading into memory".to_string());
                }
                _ => {}
            }
        }

        recommendations
    }
}

/// Metrics aggregator.
struct MetricsAggregator {
    total_count: u64,
    success_count: u64,
    execution_times: VecDeque<u64>,
    queue_times: VecDeque<u64>,
    window_start: DateTime<Utc>,
}

impl MetricsAggregator {
    fn new() -> Self {
        Self {
            total_count: 0,
            success_count: 0,
            execution_times: VecDeque::new(),
            queue_times: VecDeque::new(),
            window_start: Utc::now(),
        }
    }

    fn add_metrics(&mut self, metrics: &JobMetrics) {
        self.total_count += 1;

        if metrics.success {
            self.success_count += 1;
        }

        // Keep bounded history
        while self.execution_times.len() >= 10000 {
            self.execution_times.pop_front();
        }
        self.execution_times.push_back(metrics.execution_time_ms);

        while self.queue_times.len() >= 10000 {
            self.queue_times.pop_front();
        }
        self.queue_times.push_back(metrics.queue_time_ms);
    }

    fn success_rate(&self) -> f64 {
        if self.total_count == 0 {
            0.0
        } else {
            self.success_count as f64 / self.total_count as f64
        }
    }

    fn get_aggregated(&self, window: Duration) -> Result<AggregatedMetrics> {
        let now = Utc::now();
        let window_start = now - chrono::Duration::from_std(window).unwrap();

        // Calculate percentiles
        let mut exec_times: Vec<_> = self.execution_times.iter().cloned().collect();
        exec_times.sort_unstable();

        let p50 = exec_times.get(exec_times.len() / 2).copied().unwrap_or(0);
        let p95 = exec_times.get(exec_times.len() * 95 / 100).copied().unwrap_or(0);
        let p99 = exec_times.get(exec_times.len() * 99 / 100).copied().unwrap_or(0);

        Ok(AggregatedMetrics {
            window_start,
            window_end: now,
            total_jobs: self.total_count,
            successful_jobs: self.success_count,
            failed_jobs: self.total_count - self.success_count,
            avg_queue_time_ms: self.queue_times.iter().sum::<u64>() as f64 / self.queue_times.len().max(1) as f64,
            avg_execution_time_ms: self.execution_times.iter().sum::<u64>() as f64 / self.execution_times.len().max(1) as f64,
            p50_execution_time_ms: p50,
            p95_execution_time_ms: p95,
            p99_execution_time_ms: p99,
            jobs_per_second: self.total_count as f64 / window.as_secs_f64(),
            success_rate: self.success_rate(),
            top_job_types: vec![],  // Would need to track separately
            top_errors: vec![],      // Would need to track separately
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_context() {
        let root = TraceContext::new_root();
        assert!(root.parent_span_id.is_none());

        let child = root.child();
        assert_eq!(child.trace_id, root.trace_id);
        assert_eq!(child.parent_span_id, Some(root.span_id));
    }

    #[tokio::test]
    async fn test_monitoring_service() {
        let store = Arc::new(aspen_core::DeterministicKeyValueStore::new());
        let service = JobMonitoringService::new(store);

        // Start a span
        let trace_ctx = TraceContext::new_root();
        let span_id = service.start_span(&trace_ctx, "test_op", "test_service").await;

        // Add event
        let event = SpanEvent {
            name: "checkpoint".to_string(),
            timestamp: Utc::now(),
            attributes: HashMap::new(),
        };
        service.add_span_event(&span_id, event).await.unwrap();

        // End span
        service.end_span(&span_id, SpanStatus::Ok).await.unwrap();

        // Query traces
        let traces = service.query_traces(TraceFilter::default()).await.unwrap();
        assert_eq!(traces.len(), 1);
    }
}