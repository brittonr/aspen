//! Observability features for the Aspen client SDK.
//!
//! This module provides distributed tracing, metrics collection, and
//! performance monitoring capabilities for client operations.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::RwLock;

use crate::AspenClient;

/// W3C Trace Context for distributed tracing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceContext {
    /// Trace ID (32 hex chars).
    pub trace_id: String,
    /// Parent span ID (16 hex chars).
    pub parent_id: String,
    /// Current span ID (16 hex chars).
    pub span_id: String,
    /// Trace flags (01 = sampled).
    pub flags: u8,
    /// Vendor-specific trace state.
    pub state: Option<String>,
}

impl TraceContext {
    /// Create a new root trace context.
    pub fn new_root() -> Self {
        Self {
            trace_id: Self::generate_trace_id(),
            parent_id: "0000000000000000".to_string(),
            span_id: Self::generate_span_id(),
            flags: 0x01, // Sampled
            state: None,
        }
    }

    /// Create a child span from this context.
    pub fn child(&self) -> Self {
        Self {
            trace_id: self.trace_id.clone(),
            parent_id: self.span_id.clone(),
            span_id: Self::generate_span_id(),
            flags: self.flags,
            state: self.state.clone(),
        }
    }

    /// Generate a random trace ID.
    fn generate_trace_id() -> String {
        format!("{:032x}", rand::random::<u128>())
    }

    /// Generate a random span ID.
    fn generate_span_id() -> String {
        format!("{:016x}", rand::random::<u64>())
    }

    /// Convert to W3C traceparent header format.
    pub fn to_traceparent(&self) -> String {
        format!("00-{}-{}-{:02x}", self.trace_id, self.span_id, self.flags)
    }

    /// Parse from W3C traceparent header format.
    pub fn from_traceparent(header: &str) -> Result<Self> {
        let parts: Vec<&str> = header.split('-').collect();
        if parts.len() != 4 {
            return Err(anyhow::anyhow!("Invalid traceparent format"));
        }

        Ok(Self {
            trace_id: parts[1].to_string(),
            span_id: parts[2].to_string(),
            parent_id: "0000000000000000".to_string(),
            flags: u8::from_str_radix(parts[3], 16)?,
            state: None,
        })
    }
}

/// Span represents a unit of work in a distributed trace.
#[derive(Debug, Clone)]
pub struct Span {
    /// Span context.
    pub context: TraceContext,
    /// Operation name.
    pub operation: String,
    /// Start time.
    pub start_time: Instant,
    /// End time (if completed).
    pub end_time: Option<Instant>,
    /// Span attributes.
    pub attributes: HashMap<String, String>,
    /// Span events.
    pub events: Vec<SpanEvent>,
    /// Span status.
    pub status: SpanStatus,
}

impl Span {
    /// Create a new span.
    pub fn new(operation: impl Into<String>, parent: Option<&TraceContext>) -> Self {
        let context = match parent {
            Some(ctx) => ctx.child(),
            None => TraceContext::new_root(),
        };

        Self {
            context,
            operation: operation.into(),
            start_time: Instant::now(),
            end_time: None,
            attributes: HashMap::new(),
            events: Vec::new(),
            status: SpanStatus::Unset,
        }
    }

    /// Add an attribute to the span.
    pub fn set_attribute(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.attributes.insert(key.into(), value.into());
    }

    /// Add an event to the span.
    pub fn add_event(&mut self, name: impl Into<String>, attributes: HashMap<String, String>) {
        self.events.push(SpanEvent {
            timestamp: Instant::now(),
            name: name.into(),
            attributes,
        });
    }

    /// Set the span status.
    pub fn set_status(&mut self, status: SpanStatus) {
        self.status = status;
    }

    /// End the span.
    pub fn end(&mut self) {
        self.end_time = Some(Instant::now());
    }

    /// Get the span duration.
    pub fn duration(&self) -> Option<Duration> {
        self.end_time.map(|end| end.duration_since(self.start_time))
    }
}

/// Event that occurred during a span.
#[derive(Debug, Clone)]
pub struct SpanEvent {
    /// Event timestamp.
    pub timestamp: Instant,
    /// Event name.
    pub name: String,
    /// Event attributes.
    pub attributes: HashMap<String, String>,
}

/// Span status.
#[derive(Debug, Clone, PartialEq)]
pub enum SpanStatus {
    /// Status not set.
    Unset,
    /// Operation completed successfully.
    Ok,
    /// Operation failed.
    Error(String),
}

/// Metrics collector for client operations.
pub struct MetricsCollector {
    /// Counter metrics.
    counters: Arc<RwLock<HashMap<String, u64>>>,
    /// Gauge metrics.
    gauges: Arc<RwLock<HashMap<String, f64>>>,
    /// Histogram metrics.
    histograms: Arc<RwLock<HashMap<String, Vec<f64>>>>,
    /// Labels for metrics.
    labels: HashMap<String, String>,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsCollector {
    /// Create a new metrics collector.
    pub fn new() -> Self {
        Self {
            counters: Arc::new(RwLock::new(HashMap::new())),
            gauges: Arc::new(RwLock::new(HashMap::new())),
            histograms: Arc::new(RwLock::new(HashMap::new())),
            labels: HashMap::new(),
        }
    }

    /// Add a label to all metrics.
    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    /// Increment a counter.
    pub async fn increment(&self, name: impl Into<String>, value: u64) {
        let mut counters = self.counters.write().await;
        let counter = counters.entry(name.into()).or_insert(0);
        *counter += value;
    }

    /// Set a gauge value.
    pub async fn gauge(&self, name: impl Into<String>, value: f64) {
        let mut gauges = self.gauges.write().await;
        gauges.insert(name.into(), value);
    }

    /// Record a histogram value.
    pub async fn histogram(&self, name: impl Into<String>, value: f64) {
        let mut histograms = self.histograms.write().await;
        let histogram = histograms.entry(name.into()).or_insert_with(Vec::new);
        histogram.push(value);
    }

    /// Get all counter values.
    pub async fn get_counters(&self) -> HashMap<String, u64> {
        self.counters.read().await.clone()
    }

    /// Get all gauge values.
    pub async fn get_gauges(&self) -> HashMap<String, f64> {
        self.gauges.read().await.clone()
    }

    /// Get histogram statistics.
    pub async fn get_histogram_stats(&self, name: &str) -> Option<HistogramStats> {
        let histograms = self.histograms.read().await;
        histograms.get(name).map(|values| {
            let mut sorted = values.clone();
            // f64 comparison: use total_cmp for NaN-safe ordering (NaN sorts as greater than all values)
            sorted.sort_by(|a, b| a.total_cmp(b));

            HistogramStats {
                count: sorted.len() as u64,
                sum: sorted.iter().sum(),
                mean: sorted.iter().sum::<f64>() / sorted.len() as f64,
                min: sorted.first().copied().unwrap_or(0.0),
                max: sorted.last().copied().unwrap_or(0.0),
                p50: percentile(&sorted, 0.50),
                p95: percentile(&sorted, 0.95),
                p99: percentile(&sorted, 0.99),
            }
        })
    }

    /// Export metrics in Prometheus format.
    pub async fn export_prometheus(&self) -> String {
        let mut output = String::new();

        // Export counters
        for (name, value) in self.counters.read().await.iter() {
            output.push_str(&format!("# TYPE {} counter\n", name));
            output.push_str(&format!("{}{} {}\n", name, self.format_labels(), value));
        }

        // Export gauges
        for (name, value) in self.gauges.read().await.iter() {
            output.push_str(&format!("# TYPE {} gauge\n", name));
            output.push_str(&format!("{}{} {}\n", name, self.format_labels(), value));
        }

        // Export histograms
        for (name, _values) in self.histograms.read().await.iter() {
            if let Some(stats) = self.get_histogram_stats(name).await {
                output.push_str(&format!("# TYPE {} histogram\n", name));
                output.push_str(&format!("{}_count{} {}\n", name, self.format_labels(), stats.count));
                output.push_str(&format!("{}_sum{} {}\n", name, self.format_labels(), stats.sum));
                output.push_str(&format!("{}_p50{} {}\n", name, self.format_labels(), stats.p50));
                output.push_str(&format!("{}_p95{} {}\n", name, self.format_labels(), stats.p95));
                output.push_str(&format!("{}_p99{} {}\n", name, self.format_labels(), stats.p99));
            }
        }

        output
    }

    /// Format labels for Prometheus export.
    fn format_labels(&self) -> String {
        if self.labels.is_empty() {
            String::new()
        } else {
            let labels: Vec<String> = self.labels.iter().map(|(k, v)| format!("{}=\"{}\"", k, v)).collect();
            format!("{{{}}}", labels.join(","))
        }
    }
}

/// Statistics for histogram metrics.
#[derive(Debug, Clone)]
pub struct HistogramStats {
    /// Number of samples.
    pub count: u64,
    /// Sum of all samples.
    pub sum: f64,
    /// Mean value.
    pub mean: f64,
    /// Minimum value.
    pub min: f64,
    /// Maximum value.
    pub max: f64,
    /// 50th percentile.
    pub p50: f64,
    /// 95th percentile.
    pub p95: f64,
    /// 99th percentile.
    pub p99: f64,
}

/// Calculate percentile from sorted values.
fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() - 1) as f64 * p) as usize;
    sorted[idx]
}

/// Observability client for distributed tracing and metrics.
pub struct ObservabilityClient<'a> {
    /// Active spans.
    spans: Arc<RwLock<Vec<Span>>>,
    /// Metrics collector.
    metrics: MetricsCollector,
    /// Client reference for sending traces.
    #[allow(dead_code)]
    client: &'a AspenClient,
}

impl<'a> ObservabilityClient<'a> {
    /// Create a new observability client.
    pub fn new(client: &'a AspenClient) -> Self {
        Self {
            spans: Arc::new(RwLock::new(Vec::new())),
            metrics: MetricsCollector::new(),
            client,
        }
    }

    /// Start a new span.
    pub async fn start_span(&self, operation: impl Into<String>, parent: Option<&TraceContext>) -> Span {
        let span = Span::new(operation, parent);
        self.spans.write().await.push(span.clone());

        // Increment span counter
        self.metrics.increment("spans_started", 1).await;

        span
    }

    /// End a span and record it.
    pub async fn end_span(&self, mut span: Span) {
        span.end();

        // Record span duration
        if let Some(duration) = span.duration() {
            self.metrics.histogram("span_duration_ms", duration.as_millis() as f64).await;
        }

        // Update span status metrics
        match &span.status {
            SpanStatus::Ok => self.metrics.increment("spans_succeeded", 1).await,
            SpanStatus::Error(_) => self.metrics.increment("spans_failed", 1).await,
            SpanStatus::Unset => {}
        }

        // DEFERRED: Send span to server for aggregation. Requires a TraceIngest
        // RPC endpoint on the server side and a batching/flush strategy here.
        // self.client.send_trace(span).await;
    }

    /// Record an operation with automatic span creation.
    pub async fn record_operation<F, T>(
        &self,
        operation: impl Into<String>,
        parent: Option<&TraceContext>,
        f: F,
    ) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        let mut span = self.start_span(operation, parent).await;

        match f.await {
            Ok(result) => {
                span.set_status(SpanStatus::Ok);
                self.end_span(span).await;
                Ok(result)
            }
            Err(e) => {
                span.set_status(SpanStatus::Error(e.to_string()));
                self.end_span(span).await;
                Err(e)
            }
        }
    }

    /// Get the metrics collector.
    pub fn metrics(&self) -> &MetricsCollector {
        &self.metrics
    }

    /// Export all spans in OpenTelemetry format.
    pub async fn export_spans(&self) -> Vec<Span> {
        self.spans.read().await.clone()
    }

    /// Clear all collected spans.
    pub async fn clear_spans(&self) {
        self.spans.write().await.clear();
    }
}

/// Extension trait to add observability to AspenClient.
pub trait AspenClientObservabilityExt {
    /// Get an observability client.
    fn observability(&self) -> ObservabilityClient<'_>;
}

impl AspenClientObservabilityExt for AspenClient {
    fn observability(&self) -> ObservabilityClient<'_> {
        ObservabilityClient::new(self)
    }
}

/// Builder for configuring observability.
pub struct ObservabilityBuilder {
    /// Enable tracing.
    pub tracing_enabled: bool,
    /// Enable metrics.
    pub metrics_enabled: bool,
    /// Sampling rate (0.0 to 1.0).
    pub sampling_rate: f64,
    /// Export interval.
    pub export_interval: Duration,
    /// Maximum spans to keep in memory.
    pub max_spans: usize,
}

impl Default for ObservabilityBuilder {
    fn default() -> Self {
        Self {
            tracing_enabled: true,
            metrics_enabled: true,
            sampling_rate: 1.0,
            export_interval: Duration::from_secs(60),
            max_spans: 10000,
        }
    }
}

impl ObservabilityBuilder {
    /// Enable or disable tracing.
    pub fn with_tracing(mut self, enabled: bool) -> Self {
        self.tracing_enabled = enabled;
        self
    }

    /// Enable or disable metrics.
    pub fn with_metrics(mut self, enabled: bool) -> Self {
        self.metrics_enabled = enabled;
        self
    }

    /// Set the sampling rate.
    pub fn with_sampling_rate(mut self, rate: f64) -> Self {
        self.sampling_rate = rate.clamp(0.0, 1.0);
        self
    }

    /// Set the export interval.
    pub fn with_export_interval(mut self, interval: Duration) -> Self {
        self.export_interval = interval;
        self
    }

    /// Set the maximum number of spans to keep.
    pub fn with_max_spans(mut self, max: usize) -> Self {
        self.max_spans = max;
        self
    }
}
