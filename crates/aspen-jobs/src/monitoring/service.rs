//! Job monitoring service implementation.

use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tokio::sync::RwLock;
use tracing::debug;

use super::audit::AuditAction;
use super::audit::AuditFilter;
use super::audit::AuditLogEntry;
use super::audit::AuditResult;
use super::metrics::AggregatedMetrics;
use super::metrics::JobMetrics;
use super::metrics::MetricsAggregator;
use super::profiling::JobProfile;
use super::profiling::JobProfileBuilder;
use super::profiling::ProfilePhase;
use super::profiling::ResourceSample;
use super::trace::SpanEvent;
use super::trace::SpanStatus;
use super::trace::TraceContext;
use super::trace::TraceFilter;
use super::trace::TraceSpan;
use crate::error::JobError;
use crate::error::Result;
use crate::job::JobId;

/// Maximum metrics retained per node.
const MAX_METRICS_PER_NODE: usize = 10_000;

/// Maximum trace spans retained.
const MAX_TRACE_SPANS: usize = 100_000;

/// Maximum audit log entries.
const MAX_AUDIT_LOG_ENTRIES: usize = 50_000;

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
    pub async fn start_span(&self, trace_ctx: &TraceContext, operation: &str, service: &str) -> String {
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
            if duration > 1_000_000 {
                // > 1 second
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
    pub async fn get_aggregated_metrics(&self, window: Duration) -> Result<AggregatedMetrics> {
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
    pub async fn record_profile_phase(&self, job_id: &JobId, phase: ProfilePhase) -> Result<()> {
        let mut profiles = self.profiles.write().await;

        if let Some(builder) = profiles.get_mut(job_id) {
            builder.add_phase(phase);
        }

        Ok(())
    }

    /// Record a resource sample.
    pub async fn record_resource_sample(&self, job_id: &JobId, sample: ResourceSample) -> Result<()> {
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
            Err(JobError::JobNotFound { id: job_id.to_string() })
        }
    }

    /// Query historical traces.
    pub async fn query_traces(&self, filter: TraceFilter) -> Result<Vec<TraceSpan>> {
        let completed = self.completed_spans.read().await;

        let filtered: Vec<_> = completed.iter().filter(|span| filter.matches(span)).cloned().collect();

        Ok(filtered)
    }

    /// Query audit log.
    pub async fn query_audit_log(&self, filter: AuditFilter) -> Result<Vec<AuditLogEntry>> {
        let audit_log = self.audit_log.read().await;

        let filtered: Vec<_> = audit_log.iter().filter(|entry| filter.matches(entry)).cloned().collect();

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

        self.store
            .write(aspen_core::WriteRequest {
                command: aspen_core::WriteCommand::Set { key, value },
            })
            .await?;

        Ok(())
    }

    async fn persist_metrics(&self, metrics: &JobMetrics) -> Result<()> {
        let key = format!("metrics:job:{}:{}", metrics.job_id, Utc::now().timestamp());
        let value = serde_json::to_string(metrics)?;

        self.store
            .write(aspen_core::WriteRequest {
                command: aspen_core::WriteCommand::SetWithTTL {
                    key,
                    value,
                    ttl_seconds: 86400, // 24 hours
                },
            })
            .await?;

        Ok(())
    }

    async fn persist_audit_entry(&self, entry: &AuditLogEntry) -> Result<()> {
        let key = format!("audit:{}", entry.id);
        let value = serde_json::to_string(entry)?;

        self.store
            .write(aspen_core::WriteRequest {
                command: aspen_core::WriteCommand::Set { key, value },
            })
            .await?;

        Ok(())
    }

    async fn persist_profile(&self, profile: &JobProfile) -> Result<()> {
        let key = format!("profile:job:{}", profile.job_id);
        let value = serde_json::to_string(profile)?;

        self.store
            .write(aspen_core::WriteRequest {
                command: aspen_core::WriteCommand::SetWithTTL {
                    key,
                    value,
                    ttl_seconds: 604800, // 7 days
                },
            })
            .await?;

        Ok(())
    }
}
