//! Worker wrapper with automatic distributed tracing.
//!
//! This module provides a worker wrapper that automatically instruments
//! job execution with distributed tracing spans.

use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use tracing::error;
use tracing::info;

use crate::error::Result;
use crate::job::Job;
use crate::job::JobResult;
use crate::monitoring::JobMetrics;
use crate::monitoring::JobMonitoringService;
use crate::monitoring::SpanEvent;
use crate::monitoring::SpanStatus;
use crate::tracing::AttributeValue;
use crate::tracing::DistributedTracingService;
use crate::tracing::SpanKind;
use crate::worker::Worker;

/// Worker wrapper that adds distributed tracing.
pub struct TracedWorker<W: Worker> {
    /// Inner worker.
    inner: W,
    /// Tracing service.
    tracing_service: Arc<DistributedTracingService>,
    /// Monitoring service.
    monitoring_service: Arc<JobMonitoringService<dyn aspen_core::KeyValueStore>>,
    /// Worker ID.
    worker_id: String,
    /// Node ID.
    node_id: String,
}

impl<W: Worker> TracedWorker<W> {
    /// Create a new traced worker.
    pub fn new(
        inner: W,
        tracing_service: Arc<DistributedTracingService>,
        monitoring_service: Arc<JobMonitoringService<dyn aspen_core::KeyValueStore>>,
        worker_id: String,
        node_id: String,
    ) -> Self {
        Self {
            inner,
            tracing_service,
            monitoring_service,
            worker_id,
            node_id,
        }
    }

    /// Build initial job attributes for tracing.
    fn execute_build_initial_attributes(
        &self,
        job: &Job,
        job_type: &str,
    ) -> std::collections::HashMap<String, AttributeValue> {
        let mut attributes = std::collections::HashMap::new();
        attributes.insert("job.type".to_string(), AttributeValue::String(job_type.to_string()));
        attributes
            .insert("job.priority".to_string(), AttributeValue::String(format!("{:?}", job.spec.config.priority)));
        attributes.insert("job.retry_count".to_string(), AttributeValue::Int(job.attempts as i64));
        attributes.insert("worker.id".to_string(), AttributeValue::String(self.worker_id.clone()));
        attributes.insert("node.id".to_string(), AttributeValue::String(self.node_id.clone()));
        attributes
    }

    /// Build metrics attributes after job execution.
    fn execute_build_metrics_attributes(
        &self,
        result: &JobResult,
        execution_time: std::time::Duration,
    ) -> std::collections::HashMap<String, AttributeValue> {
        let mut attributes = std::collections::HashMap::new();
        attributes.insert("job.execution_time_ms".to_string(), AttributeValue::Int(execution_time.as_millis() as i64));
        attributes.insert("job.success".to_string(), AttributeValue::Bool(result.is_success()));

        if let JobResult::Failure(failure) = result {
            attributes.insert("job.error".to_string(), AttributeValue::String(failure.reason.clone()));
        }
        attributes
    }

    /// Build job metrics for monitoring service.
    fn execute_build_job_metrics(
        &self,
        job_id: &crate::job::JobId,
        job_type: &str,
        execution_time: std::time::Duration,
        result: &JobResult,
    ) -> JobMetrics {
        JobMetrics {
            job_id: job_id.clone(),
            job_type: job_type.to_string(),
            worker_id: self.worker_id.clone(),
            node_id: self.node_id.clone(),
            queue_time_ms: 0, // Would need queue time tracking
            execution_time_ms: execution_time.as_millis() as u64,
            total_time_ms: execution_time.as_millis() as u64,
            cpu_usage: 0.0,  // Would need resource monitoring
            memory_bytes: 0, // Would need resource monitoring
            network_sent_bytes: 0,
            network_recv_bytes: 0,
            retry_count: 0,
            is_success: result.is_success(),
            error_message: if let JobResult::Failure(f) = result {
                Some(f.reason.clone())
            } else {
                None
            },
            custom: std::collections::HashMap::new(),
        }
    }
}

#[async_trait]
impl<W: Worker> Worker for TracedWorker<W> {
    async fn execute(&self, job: Job) -> JobResult {
        let start_time = Instant::now();
        let job_id = job.id.clone();
        let job_type = job.spec.job_type.clone();

        // Extract parent trace context and start execution span
        let parent_context = self.tracing_service.extract_context(&job);
        let span_context = self
            .tracing_service
            .start_span(parent_context.as_ref(), &format!("job.execute.{}", job_type), SpanKind::Internal)
            .await;

        // Set job context and initial attributes
        self.tracing_service
            .set_job_context(&span_context.span_id, job_id.clone(), self.worker_id.clone())
            .await;
        let attributes = self.execute_build_initial_attributes(&job, &job_type);
        self.tracing_service.set_attributes(&span_context.span_id, attributes).await;

        // Record job start event
        let start_event = SpanEvent {
            name: "job.started".to_string(),
            timestamp: chrono::Utc::now(),
            attributes: std::collections::HashMap::new(),
        };
        self.tracing_service.add_event(&span_context.span_id, start_event).await;

        info!(
            job_id = %job_id,
            job_type = %job_type,
            worker_id = %self.worker_id,
            trace_id = %span_context.trace_id.to_hex(),
            span_id = %span_context.span_id.to_hex(),
            "starting traced job execution"
        );

        // Execute the actual job with profiling
        let _ = self.monitoring_service.start_profiling(job_id.clone()).await;
        let result = self.inner.execute(job).await;
        let execution_time = start_time.elapsed();

        // Record execution results
        let span_status = if result.is_success() {
            SpanStatus::Ok
        } else {
            SpanStatus::Error
        };
        let metrics_attributes = self.execute_build_metrics_attributes(&result, execution_time);
        self.tracing_service.set_attributes(&span_context.span_id, metrics_attributes).await;

        // Record completion event
        let completion_event = SpanEvent {
            name: if result.is_success() {
                "job.completed"
            } else {
                "job.failed"
            }
            .to_string(),
            timestamp: chrono::Utc::now(),
            attributes: std::collections::HashMap::new(),
        };
        self.tracing_service.add_event(&span_context.span_id, completion_event).await;

        // End span and record metrics
        self.tracing_service.end_span(&span_context.span_id, span_status).await.unwrap_or_else(|e| {
            error!(error = %e, "failed to end trace span");
        });

        let metrics = self.execute_build_job_metrics(&job_id, &job_type, execution_time, &result);
        let _ = self.monitoring_service.record_metrics(metrics).await;
        let _ = self.monitoring_service.finish_profiling(&job_id).await;

        info!(
            job_id = %job_id,
            job_type = %job_type,
            execution_time_ms = execution_time.as_millis(),
            success = result.is_success(),
            trace_id = %span_context.trace_id.to_hex(),
            span_id = %span_context.span_id.to_hex(),
            "completed traced job execution"
        );

        result
    }

    async fn on_start(&self) -> Result<()> {
        // Start a span for worker startup
        let span_context = self.tracing_service.start_span(None, "worker.start", SpanKind::Internal).await;

        let mut attributes = std::collections::HashMap::new();
        attributes.insert("worker.id".to_string(), AttributeValue::String(self.worker_id.clone()));
        attributes.insert("node.id".to_string(), AttributeValue::String(self.node_id.clone()));

        self.tracing_service.set_attributes(&span_context.span_id, attributes).await;

        let result = self.inner.on_start().await;

        let status = if result.is_ok() {
            SpanStatus::Ok
        } else {
            SpanStatus::Error
        };

        self.tracing_service.end_span(&span_context.span_id, status).await?;

        result
    }

    async fn on_shutdown(&self) -> Result<()> {
        // Start a span for worker shutdown
        let span_context = self.tracing_service.start_span(None, "worker.shutdown", SpanKind::Internal).await;

        let mut attributes = std::collections::HashMap::new();
        attributes.insert("worker.id".to_string(), AttributeValue::String(self.worker_id.clone()));

        self.tracing_service.set_attributes(&span_context.span_id, attributes).await;

        let result = self.inner.on_shutdown().await;

        let status = if result.is_ok() {
            SpanStatus::Ok
        } else {
            SpanStatus::Error
        };

        self.tracing_service.end_span(&span_context.span_id, status).await?;

        result
    }

    fn job_types(&self) -> Vec<String> {
        self.inner.job_types()
    }

    fn can_handle(&self, job_type: &str) -> bool {
        self.inner.can_handle(job_type)
    }
}

/// Extension trait for adding tracing to workers.
pub trait WorkerTracingExt: Worker + Sized {
    /// Wrap this worker with tracing.
    fn with_tracing(
        self,
        tracing_service: Arc<DistributedTracingService>,
        monitoring_service: Arc<JobMonitoringService<dyn aspen_core::KeyValueStore>>,
        worker_id: String,
        node_id: String,
    ) -> TracedWorker<Self> {
        TracedWorker::new(self, tracing_service, monitoring_service, worker_id, node_id)
    }
}

impl<W: Worker> WorkerTracingExt for W {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::JobSpec;
    use crate::tracing::SamplingStrategy;

    struct TestWorker;

    #[async_trait]
    impl Worker for TestWorker {
        async fn execute(&self, _job: Job) -> JobResult {
            JobResult::success(serde_json::json!({"result": "ok"}))
        }
    }

    #[tokio::test]
    async fn test_traced_worker() {
        let store: Arc<dyn aspen_core::KeyValueStore> = Arc::new(aspen_testing::DeterministicKeyValueStore::new());
        let tracing_service =
            Arc::new(DistributedTracingService::new("test-node".to_string(), SamplingStrategy::AlwaysOn));
        let monitoring_service = Arc::new(JobMonitoringService::new(store));

        let worker = TestWorker;
        let traced = worker.with_tracing(
            tracing_service.clone(),
            monitoring_service,
            "worker-1".to_string(),
            "node-1".to_string(),
        );

        let job = Job::from_spec(JobSpec::new("test_job"));

        let result = traced.execute(job).await;
        assert!(result.is_success());
    }
}
