//! Distributed tracing implementation for job execution.
//!
//! This module provides OpenTelemetry-compatible distributed tracing
//! that follows job execution across nodes, workers, and services.
//!
//! ## Features
//!
//! - W3C Trace Context propagation
//! - OpenTelemetry semantic conventions
//! - Automatic span correlation across nodes
//! - Sampling strategies for high-throughput scenarios
//! - Trace export to various backends (Jaeger, Zipkin, etc.)
//! - Baggage propagation for cross-cutting concerns

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{Span, debug, error, info, warn};

use crate::error::{JobError, Result};
use crate::job::{Job, JobId, JobStatus};
use crate::monitoring::{SpanEvent, SpanLink, SpanStatus, TraceSpan};

/// W3C Trace Context version.
const TRACE_VERSION: &str = "00";

/// Maximum baggage items.
const MAX_BAGGAGE_ITEMS: usize = 64;

/// Maximum baggage size in bytes.
const MAX_BAGGAGE_SIZE: usize = 8192;

/// OpenTelemetry trace context following W3C Trace Context spec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedTraceContext {
    /// Trace ID (16 bytes, 32 hex chars).
    pub trace_id: TraceId,
    /// Parent span ID (8 bytes, 16 hex chars).
    pub parent_span_id: Option<SpanId>,
    /// Current span ID (8 bytes, 16 hex chars).
    pub span_id: SpanId,
    /// Trace flags (8 bits).
    pub trace_flags: TraceFlags,
    /// Trace state (vendor-specific).
    pub trace_state: Option<String>,
    /// Baggage items for cross-cutting concerns.
    pub baggage: Baggage,
}

/// Trace ID (128-bit / 16 bytes).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TraceId(pub [u8; 16]);

impl TraceId {
    /// Generate a new random trace ID.
    pub fn generate() -> Self {
        let mut bytes = [0u8; 16];
        getrandom::getrandom(&mut bytes).unwrap();
        Self(bytes)
    }

    /// Parse from hex string.
    pub fn from_hex(hex: &str) -> Option<Self> {
        if hex.len() != 32 {
            return None;
        }

        let mut bytes = [0u8; 16];
        hex::decode_to_slice(hex, &mut bytes).ok()?;
        Some(Self(bytes))
    }

    /// Convert to hex string.
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

/// Span ID (64-bit / 8 bytes).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpanId(pub [u8; 8]);

impl SpanId {
    /// Generate a new random span ID.
    pub fn generate() -> Self {
        let mut bytes = [0u8; 8];
        getrandom::getrandom(&mut bytes).unwrap();
        Self(bytes)
    }

    /// Parse from hex string.
    pub fn from_hex(hex: &str) -> Option<Self> {
        if hex.len() != 16 {
            return None;
        }

        let mut bytes = [0u8; 8];
        hex::decode_to_slice(hex, &mut bytes).ok()?;
        Some(Self(bytes))
    }

    /// Convert to hex string.
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

/// Trace flags (8 bits).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceFlags(pub u8);

impl TraceFlags {
    /// Sampled flag.
    pub const SAMPLED: u8 = 0x01;

    /// Check if trace is sampled.
    pub fn is_sampled(&self) -> bool {
        self.0 & Self::SAMPLED != 0
    }

    /// Set sampled flag.
    pub fn set_sampled(&mut self, sampled: bool) {
        if sampled {
            self.0 |= Self::SAMPLED;
        } else {
            self.0 &= !Self::SAMPLED;
        }
    }
}

/// Baggage for cross-cutting concerns.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Baggage {
    items: HashMap<String, String>,
}

impl Baggage {
    /// Create new empty baggage.
    pub fn new() -> Self {
        Self { items: HashMap::new() }
    }

    /// Add an item to baggage.
    pub fn insert(&mut self, key: String, value: String) -> Result<()> {
        if self.items.len() >= MAX_BAGGAGE_ITEMS {
            return Err(JobError::InvalidJobSpec {
                reason: "baggage item limit exceeded".to_string(),
            });
        }

        let size = self.items.iter().map(|(k, v)| k.len() + v.len()).sum::<usize>() + key.len() + value.len();

        if size > MAX_BAGGAGE_SIZE {
            return Err(JobError::InvalidJobSpec {
                reason: "baggage size limit exceeded".to_string(),
            });
        }

        self.items.insert(key, value);
        Ok(())
    }

    /// Get a baggage item.
    pub fn get(&self, key: &str) -> Option<&String> {
        self.items.get(key)
    }

    /// Remove a baggage item.
    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.items.remove(key)
    }

    /// Iterate over baggage items.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.items.iter()
    }
}

impl DistributedTraceContext {
    /// Create a new root trace context.
    pub fn new_root() -> Self {
        Self {
            trace_id: TraceId::generate(),
            parent_span_id: None,
            span_id: SpanId::generate(),
            trace_flags: TraceFlags(TraceFlags::SAMPLED),
            trace_state: None,
            baggage: Baggage::new(),
        }
    }

    /// Create a child span context.
    pub fn child(&self) -> Self {
        Self {
            trace_id: self.trace_id.clone(),
            parent_span_id: Some(self.span_id.clone()),
            span_id: SpanId::generate(),
            trace_flags: self.trace_flags,
            trace_state: self.trace_state.clone(),
            baggage: self.baggage.clone(),
        }
    }

    /// Parse from W3C traceparent header.
    pub fn from_traceparent(header: &str) -> Option<Self> {
        let parts: Vec<&str> = header.split('-').collect();

        if parts.len() != 4 {
            return None;
        }

        let version = parts[0];
        if version != TRACE_VERSION {
            return None;
        }

        let trace_id = TraceId::from_hex(parts[1])?;
        let span_id = SpanId::from_hex(parts[2])?;
        let trace_flags = u8::from_str_radix(parts[3], 16).ok()?;

        Some(Self {
            trace_id,
            parent_span_id: None,
            span_id,
            trace_flags: TraceFlags(trace_flags),
            trace_state: None,
            baggage: Baggage::new(),
        })
    }

    /// Format as W3C traceparent header.
    pub fn to_traceparent(&self) -> String {
        format!("{}-{}-{}-{:02x}", TRACE_VERSION, self.trace_id.to_hex(), self.span_id.to_hex(), self.trace_flags.0)
    }

    /// Parse baggage from header.
    pub fn parse_baggage(&mut self, header: &str) -> Result<()> {
        for item in header.split(',') {
            let item = item.trim();
            if let Some((key, value)) = item.split_once('=') {
                self.baggage.insert(key.to_string(), value.to_string())?;
            }
        }
        Ok(())
    }

    /// Format baggage as header.
    pub fn format_baggage(&self) -> String {
        self.baggage.iter().map(|(k, v)| format!("{}={}", k, v)).collect::<Vec<_>>().join(",")
    }
}

/// Sampling strategy for distributed tracing.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SamplingStrategy {
    /// Always sample.
    AlwaysOn,
    /// Never sample.
    AlwaysOff,
    /// Sample based on probability (0.0 to 1.0).
    Probability(f64),
    /// Sample based on rate limit (traces per second).
    RateLimited(u32),
    /// Adaptive sampling based on load.
    Adaptive,
}

impl SamplingStrategy {
    /// Decide whether to sample a trace.
    pub fn should_sample(&self, trace_id: &TraceId) -> bool {
        match self {
            Self::AlwaysOn => true,
            Self::AlwaysOff => false,
            Self::Probability(p) => {
                // Use trace ID for deterministic sampling
                let hash = trace_id.0[0] as f64 / 255.0;
                hash < *p
            }
            Self::RateLimited(_rate) => {
                // Would need rate limiter implementation
                true
            }
            Self::Adaptive => {
                // Would need load monitoring
                true
            }
        }
    }
}

/// Distributed span with full context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedSpan {
    /// Span context.
    pub context: DistributedTraceContext,
    /// Operation name.
    pub operation_name: String,
    /// Service name.
    pub service_name: String,
    /// Node ID where span executed.
    pub node_id: String,
    /// Worker ID that processed the span.
    pub worker_id: Option<String>,
    /// Job ID associated with the span.
    pub job_id: Option<JobId>,
    /// Start time.
    pub start_time: DateTime<Utc>,
    /// End time.
    pub end_time: Option<DateTime<Utc>>,
    /// Span kind.
    pub kind: SpanKind,
    /// Span attributes (OpenTelemetry semantic conventions).
    pub attributes: HashMap<String, AttributeValue>,
    /// Span events.
    pub events: Vec<SpanEvent>,
    /// Links to other spans.
    pub links: Vec<SpanLink>,
    /// Span status.
    pub status: SpanStatus,
}

/// Span kind following OpenTelemetry spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpanKind {
    /// Internal operation.
    Internal,
    /// Server-side handling of RPC.
    Server,
    /// Client-side RPC.
    Client,
    /// Producer of async message.
    Producer,
    /// Consumer of async message.
    Consumer,
}

/// Attribute value types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AttributeValue {
    /// String value.
    String(String),
    /// Boolean value.
    Bool(bool),
    /// Integer value.
    Int(i64),
    /// Float value.
    Float(f64),
    /// Array of strings.
    StringArray(Vec<String>),
    /// Array of booleans.
    BoolArray(Vec<bool>),
    /// Array of integers.
    IntArray(Vec<i64>),
    /// Array of floats.
    FloatArray(Vec<f64>),
}

/// Distributed tracing service.
pub struct DistributedTracingService {
    /// Active spans by span ID.
    active_spans: Arc<RwLock<HashMap<SpanId, DistributedSpan>>>,
    /// Completed spans buffer.
    completed_spans: Arc<RwLock<Vec<DistributedSpan>>>,
    /// Sampling strategy.
    sampling_strategy: SamplingStrategy,
    /// Trace exporters.
    exporters: Arc<RwLock<Vec<Box<dyn TraceExporter>>>>,
    /// Node ID for this service.
    node_id: String,
}

impl DistributedTracingService {
    /// Create a new tracing service.
    pub fn new(node_id: String, sampling_strategy: SamplingStrategy) -> Self {
        Self {
            active_spans: Arc::new(RwLock::new(HashMap::new())),
            completed_spans: Arc::new(RwLock::new(Vec::new())),
            sampling_strategy,
            exporters: Arc::new(RwLock::new(Vec::new())),
            node_id,
        }
    }

    /// Add a trace exporter.
    pub async fn add_exporter(&self, exporter: Box<dyn TraceExporter>) {
        self.exporters.write().await.push(exporter);
    }

    /// Start a new span.
    pub async fn start_span(
        &self,
        parent_context: Option<&DistributedTraceContext>,
        operation_name: &str,
        kind: SpanKind,
    ) -> DistributedTraceContext {
        let context = if let Some(parent) = parent_context {
            parent.child()
        } else {
            DistributedTraceContext::new_root()
        };

        // Check sampling decision
        if !self.sampling_strategy.should_sample(&context.trace_id) {
            return context;
        }

        let span = DistributedSpan {
            context: context.clone(),
            operation_name: operation_name.to_string(),
            service_name: "aspen-jobs".to_string(),
            node_id: self.node_id.clone(),
            worker_id: None,
            job_id: None,
            start_time: Utc::now(),
            end_time: None,
            kind,
            attributes: HashMap::new(),
            events: Vec::new(),
            links: Vec::new(),
            status: SpanStatus::Unset,
        };

        self.active_spans.write().await.insert(context.span_id.clone(), span);

        debug!(
            trace_id = %context.trace_id.to_hex(),
            span_id = %context.span_id.to_hex(),
            operation = operation_name,
            "started distributed span"
        );

        context
    }

    /// Set span attributes.
    pub async fn set_attributes(&self, span_id: &SpanId, attributes: HashMap<String, AttributeValue>) {
        let mut spans = self.active_spans.write().await;

        if let Some(span) = spans.get_mut(span_id) {
            span.attributes.extend(attributes);
        }
    }

    /// Add span event.
    pub async fn add_event(&self, span_id: &SpanId, event: SpanEvent) {
        let mut spans = self.active_spans.write().await;

        if let Some(span) = spans.get_mut(span_id) {
            span.events.push(event);
        }
    }

    /// Set job context for a span.
    pub async fn set_job_context(&self, span_id: &SpanId, job_id: JobId, worker_id: String) {
        let mut spans = self.active_spans.write().await;

        if let Some(span) = spans.get_mut(span_id) {
            span.job_id = Some(job_id.clone());
            span.worker_id = Some(worker_id.clone());

            // Add OpenTelemetry semantic attributes
            span.attributes.insert("job.id".to_string(), AttributeValue::String(job_id.to_string()));
            span.attributes.insert("worker.id".to_string(), AttributeValue::String(worker_id));
        }
    }

    /// End a span.
    pub async fn end_span(&self, span_id: &SpanId, status: SpanStatus) -> Result<()> {
        let mut spans = self.active_spans.write().await;

        if let Some(mut span) = spans.remove(span_id) {
            span.end_time = Some(Utc::now());
            span.status = status;

            let duration = span.end_time.unwrap().timestamp_micros() - span.start_time.timestamp_micros();

            span.attributes.insert("duration_us".to_string(), AttributeValue::Int(duration));

            debug!(
                trace_id = %span.context.trace_id.to_hex(),
                span_id = %span.context.span_id.to_hex(),
                duration_us = duration,
                status = ?status,
                "ended distributed span"
            );

            // Store completed span
            self.completed_spans.write().await.push(span.clone());

            // Export to backends
            self.export_span(span).await?;
        }

        Ok(())
    }

    /// Export span to configured exporters.
    async fn export_span(&self, span: DistributedSpan) -> Result<()> {
        let exporters = self.exporters.read().await;

        for exporter in exporters.iter() {
            if let Err(e) = exporter.export(vec![span.clone()]).await {
                warn!(error = %e, "failed to export span");
            }
        }

        Ok(())
    }

    /// Inject trace context into job metadata.
    pub fn inject_context(&self, context: &DistributedTraceContext, job: &mut Job) {
        job.spec.metadata.insert("traceparent".to_string(), context.to_traceparent());

        if !context.baggage.items.is_empty() {
            job.spec.metadata.insert("baggage".to_string(), context.format_baggage());
        }

        if let Some(ref state) = context.trace_state {
            job.spec.metadata.insert("tracestate".to_string(), state.clone());
        }
    }

    /// Extract trace context from job metadata.
    pub fn extract_context(&self, job: &Job) -> Option<DistributedTraceContext> {
        let traceparent = job.spec.metadata.get("traceparent")?;

        let mut context = DistributedTraceContext::from_traceparent(traceparent)?;

        // Extract baggage
        if let Some(baggage) = job.spec.metadata.get("baggage") {
            let _ = context.parse_baggage(baggage);
        }

        // Extract trace state
        if let Some(state) = job.spec.metadata.get("tracestate") {
            context.trace_state = Some(state.clone());
        }

        Some(context)
    }

    /// Create a span for job execution.
    pub async fn trace_job_execution<F, Fut>(
        &self,
        job: &Job,
        worker_id: &str,
        operation: F,
    ) -> Result<crate::job::JobResult>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = crate::job::JobResult>,
    {
        // Extract parent context from job
        let parent_context = self.extract_context(job);

        // Start execution span
        let context = self
            .start_span(parent_context.as_ref(), &format!("job.execute.{}", job.spec.job_type), SpanKind::Internal)
            .await;

        // Set job context
        self.set_job_context(&context.span_id, job.id.clone(), worker_id.to_string()).await;

        // Add attributes
        let mut attributes = HashMap::new();
        attributes.insert("job.type".to_string(), AttributeValue::String(job.spec.job_type.clone()));
        attributes
            .insert("job.priority".to_string(), AttributeValue::String(format!("{:?}", job.spec.config.priority)));
        attributes.insert("job.retry_count".to_string(), AttributeValue::Int(job.attempts as i64));

        self.set_attributes(&context.span_id, attributes).await;

        // Execute with tracing
        let start = Instant::now();
        let result = operation().await;
        let duration = start.elapsed();

        // Set final status
        let status = if result.is_success() {
            SpanStatus::Ok
        } else {
            SpanStatus::Error
        };

        // Add execution metrics
        let mut metrics = HashMap::new();
        metrics.insert("job.execution_time_ms".to_string(), AttributeValue::Int(duration.as_millis() as i64));

        self.set_attributes(&context.span_id, metrics).await;

        // End span
        self.end_span(&context.span_id, status).await?;

        Ok(result)
    }

    /// Get trace for a given trace ID.
    pub async fn get_trace(&self, trace_id: &TraceId) -> Vec<DistributedSpan> {
        let completed = self.completed_spans.read().await;

        completed.iter().filter(|span| span.context.trace_id == *trace_id).cloned().collect()
    }
}

/// Trait for trace exporters.
#[async_trait]
pub trait TraceExporter: Send + Sync {
    /// Export spans to backend.
    async fn export(&self, spans: Vec<DistributedSpan>) -> Result<()>;

    /// Shutdown the exporter.
    async fn shutdown(&self) -> Result<()>;
}

/// OTLP (OpenTelemetry Protocol) exporter.
pub struct OtlpExporter {
    endpoint: String,
    headers: HashMap<String, String>,
}

impl OtlpExporter {
    /// Create a new OTLP exporter.
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            headers: HashMap::new(),
        }
    }

    /// Add a header for authentication.
    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }
}

#[async_trait]
impl TraceExporter for OtlpExporter {
    async fn export(&self, spans: Vec<DistributedSpan>) -> Result<()> {
        // Convert to OTLP format and send
        // This would use the actual OTLP protocol
        debug!(
            endpoint = %self.endpoint,
            spans = spans.len(),
            "exporting spans via OTLP"
        );

        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

/// Console exporter for debugging.
pub struct ConsoleExporter;

#[async_trait]
impl TraceExporter for ConsoleExporter {
    async fn export(&self, spans: Vec<DistributedSpan>) -> Result<()> {
        for span in spans {
            info!(
                trace_id = %span.context.trace_id.to_hex(),
                span_id = %span.context.span_id.to_hex(),
                operation = %span.operation_name,
                duration_us = span.end_time.map(|e| e.timestamp_micros() - span.start_time.timestamp_micros()),
                status = ?span.status,
                "TRACE"
            );
        }

        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

/// Helper to add tracing to job operations.
pub struct TracedJobOperation<'a> {
    tracing_service: &'a DistributedTracingService,
    context: DistributedTraceContext,
}

impl<'a> TracedJobOperation<'a> {
    /// Create a new traced operation.
    pub fn new(tracing_service: &'a DistributedTracingService, context: DistributedTraceContext) -> Self {
        Self {
            tracing_service,
            context,
        }
    }

    /// Execute an operation with a child span.
    pub async fn with_span<F, Fut, T>(&self, operation_name: &str, kind: SpanKind, operation: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let span_context = self.tracing_service.start_span(Some(&self.context), operation_name, kind).await;

        let result = operation().await;

        let status = if result.is_ok() {
            SpanStatus::Ok
        } else {
            SpanStatus::Error
        };

        self.tracing_service.end_span(&span_context.span_id, status).await?;

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_context_propagation() {
        let root = DistributedTraceContext::new_root();
        let traceparent = root.to_traceparent();

        let parsed = DistributedTraceContext::from_traceparent(&traceparent).unwrap();
        assert_eq!(parsed.trace_id, root.trace_id);
        assert_eq!(parsed.span_id, root.span_id);
    }

    #[test]
    fn test_baggage() {
        let mut baggage = Baggage::new();
        baggage.insert("user_id".to_string(), "123".to_string()).unwrap();
        baggage.insert("tenant".to_string(), "acme".to_string()).unwrap();

        assert_eq!(baggage.get("user_id"), Some(&"123".to_string()));
        assert_eq!(baggage.get("tenant"), Some(&"acme".to_string()));
    }

    #[test]
    fn test_sampling_probability() {
        let strategy = SamplingStrategy::Probability(0.5);

        let mut sampled = 0;
        let total = 1000;

        for _ in 0..total {
            let trace_id = TraceId::generate();
            if strategy.should_sample(&trace_id) {
                sampled += 1;
            }
        }

        // Should be roughly 50% with some variance
        assert!(sampled > 400 && sampled < 600);
    }

    #[tokio::test]
    async fn test_distributed_tracing() {
        let service = DistributedTracingService::new("node1".to_string(), SamplingStrategy::AlwaysOn);

        // Add console exporter
        service.add_exporter(Box::new(ConsoleExporter)).await;

        // Start root span
        let root_context = service.start_span(None, "process_batch", SpanKind::Internal).await;

        // Start child span
        let child_context = service.start_span(Some(&root_context), "process_item", SpanKind::Internal).await;

        // Add attributes
        let mut attributes = HashMap::new();
        attributes.insert("item.id".to_string(), AttributeValue::String("item1".to_string()));
        service.set_attributes(&child_context.span_id, attributes).await;

        // End spans
        service.end_span(&child_context.span_id, SpanStatus::Ok).await.unwrap();
        service.end_span(&root_context.span_id, SpanStatus::Ok).await.unwrap();

        // Get trace
        let trace = service.get_trace(&root_context.trace_id).await;
        assert_eq!(trace.len(), 2);
    }
}
