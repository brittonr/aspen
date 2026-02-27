//! Observability features for the Aspen client SDK.
//!
//! This module provides distributed tracing, metrics collection, and
//! performance monitoring capabilities for client operations.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use anyhow::Result;
use aspen_client_api::AlertEvaluateResultResponse;
use aspen_client_api::AlertGetResultResponse;
use aspen_client_api::AlertListResultResponse;
use aspen_client_api::AlertRuleResultResponse;
use aspen_client_api::AlertRuleWire;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::IngestSpan;
use aspen_client_api::MetricDataPoint;
use aspen_client_api::MetricIngestResultResponse;
use aspen_client_api::MetricListResultResponse;
use aspen_client_api::MetricQueryResultResponse;
use aspen_client_api::MetricTypeWire;
use aspen_client_api::SpanEventWire;
use aspen_client_api::SpanStatusWire;
use aspen_client_api::TraceGetResultResponse;
use aspen_client_api::TraceIngestResultResponse;
use aspen_client_api::TraceListResultResponse;
use aspen_client_api::TraceSearchResultResponse;
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
    /// Start time (monotonic, for duration).
    pub start_time: Instant,
    /// Start time (wall clock, for wire format).
    pub start_system_time: SystemTime,
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
            start_system_time: SystemTime::now(),
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

    /// Snapshot all in-memory metrics as data points for server-side ingest.
    pub async fn to_data_points(&self, timestamp_us: u64) -> Vec<MetricDataPoint> {
        let labels: Vec<(String, String)> = self.labels.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        let mut points = Vec::new();

        // Counters
        for (name, value) in self.counters.read().await.iter() {
            points.push(MetricDataPoint {
                name: name.clone(),
                metric_type: MetricTypeWire::Counter,
                timestamp_us,
                value: *value as f64,
                labels: labels.clone(),
                histogram_buckets: None,
                histogram_sum: None,
                histogram_count: None,
            });
        }

        // Gauges
        for (name, value) in self.gauges.read().await.iter() {
            points.push(MetricDataPoint {
                name: name.clone(),
                metric_type: MetricTypeWire::Gauge,
                timestamp_us,
                value: *value,
                labels: labels.clone(),
                histogram_buckets: None,
                histogram_sum: None,
                histogram_count: None,
            });
        }

        points
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

/// Convert a Span to the wire-format IngestSpan.
///
/// Pure function: converts monotonic Instant to absolute SystemTime for transport.
fn span_to_ingest(span: &Span) -> IngestSpan {
    let start_time_us = span.start_system_time.duration_since(UNIX_EPOCH).unwrap_or_default().as_micros() as u64;

    let duration_us = span.duration().unwrap_or_default().as_micros() as u64;

    let status = match &span.status {
        SpanStatus::Unset => SpanStatusWire::Unset,
        SpanStatus::Ok => SpanStatusWire::Ok,
        SpanStatus::Error(msg) => SpanStatusWire::Error(msg.clone()),
    };

    let max_attrs = aspen_constants::MAX_SPAN_ATTRIBUTES as usize;
    let max_events = aspen_constants::MAX_SPAN_EVENTS as usize;

    let attributes: Vec<(String, String)> =
        span.attributes.iter().take(max_attrs).map(|(k, v)| (k.clone(), v.clone())).collect();

    let events: Vec<SpanEventWire> = span
        .events
        .iter()
        .take(max_events)
        .map(|e| {
            // Approximate event timestamp: start_system_time + (event.timestamp - start_time)
            let offset = e.timestamp.duration_since(span.start_time);
            let event_us = start_time_us.saturating_add(offset.as_micros() as u64);
            SpanEventWire {
                name: e.name.clone(),
                timestamp_us: event_us,
                attributes: e.attributes.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            }
        })
        .collect();

    IngestSpan {
        trace_id: span.context.trace_id.clone(),
        span_id: span.context.span_id.clone(),
        parent_id: span.context.parent_id.clone(),
        operation: span.operation.clone(),
        start_time_us,
        duration_us,
        status,
        attributes,
        events,
    }
}

/// Observability client for distributed tracing and metrics.
pub struct ObservabilityClient<'a> {
    /// Active spans.
    spans: Arc<RwLock<Vec<Span>>>,
    /// Pending spans awaiting flush to server.
    pending_spans: Arc<RwLock<Vec<IngestSpan>>>,
    /// Metrics collector.
    metrics: MetricsCollector,
    /// Client reference for sending traces.
    client: &'a AspenClient,
}

impl<'a> ObservabilityClient<'a> {
    /// Create a new observability client.
    pub fn new(client: &'a AspenClient) -> Self {
        Self {
            spans: Arc::new(RwLock::new(Vec::new())),
            pending_spans: Arc::new(RwLock::new(Vec::new())),
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
    ///
    /// The span is converted to wire format and added to the pending buffer.
    /// When the buffer reaches MAX_TRACE_BATCH_SIZE, it auto-flushes.
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

        // Convert to wire format and buffer
        let ingest_span = span_to_ingest(&span);
        let should_flush = {
            let mut pending = self.pending_spans.write().await;
            pending.push(ingest_span);
            pending.len() >= aspen_constants::MAX_TRACE_BATCH_SIZE as usize
        };

        // Auto-flush when batch is full
        if should_flush && let Err(e) = self.flush().await {
            tracing::warn!("auto-flush trace spans failed: {}", e);
        }
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

    /// Flush pending spans to the server.
    ///
    /// Drains the pending buffer and sends a TraceIngest RPC to the cluster.
    /// Returns the server's response with accepted/dropped counts.
    pub async fn flush(&self) -> Result<TraceIngestResultResponse> {
        let spans = {
            let mut pending = self.pending_spans.write().await;
            std::mem::take(&mut *pending)
        };

        if spans.is_empty() {
            return Ok(TraceIngestResultResponse {
                is_success: true,
                accepted_count: 0,
                dropped_count: 0,
                error: None,
            });
        }

        let response = self.client.send(ClientRpcRequest::TraceIngest { spans }).await?;

        match response {
            ClientRpcResponse::TraceIngestResult(result) => Ok(result),
            ClientRpcResponse::Error(e) => Err(anyhow::anyhow!("trace ingest failed: {}", e.message)),
            other => Err(anyhow::anyhow!("unexpected response: {:?}", other)),
        }
    }

    /// Get the number of pending spans awaiting flush.
    pub async fn pending_count(&self) -> usize {
        self.pending_spans.read().await.len()
    }

    /// List recent traces with optional time range filter.
    ///
    /// Returns trace summaries sorted newest-first, bounded by `limit`.
    pub async fn list_traces(
        &self,
        start_time_us: Option<u64>,
        end_time_us: Option<u64>,
        limit: Option<u32>,
    ) -> Result<TraceListResultResponse> {
        let response = self
            .client
            .send(ClientRpcRequest::TraceList {
                start_time_us,
                end_time_us,
                limit,
                continuation_token: None,
            })
            .await?;

        match response {
            ClientRpcResponse::TraceListResult(result) => Ok(result),
            ClientRpcResponse::Error(e) => Err(anyhow::anyhow!("trace list failed: {}", e.message)),
            other => Err(anyhow::anyhow!("unexpected response: {:?}", other)),
        }
    }

    /// Get all spans for a specific trace.
    pub async fn get_trace(&self, trace_id: &str) -> Result<TraceGetResultResponse> {
        let response = self
            .client
            .send(ClientRpcRequest::TraceGet {
                trace_id: trace_id.to_string(),
            })
            .await?;

        match response {
            ClientRpcResponse::TraceGetResult(result) => Ok(result),
            ClientRpcResponse::Error(e) => Err(anyhow::anyhow!("trace get failed: {}", e.message)),
            other => Err(anyhow::anyhow!("unexpected response: {:?}", other)),
        }
    }

    /// Search spans by criteria (case-insensitive operation match).
    pub async fn search_traces(
        &self,
        operation: Option<&str>,
        min_duration_us: Option<u64>,
        max_duration_us: Option<u64>,
        status: Option<&str>,
        limit: Option<u32>,
    ) -> Result<TraceSearchResultResponse> {
        let response = self
            .client
            .send(ClientRpcRequest::TraceSearch {
                operation: operation.map(String::from),
                min_duration_us,
                max_duration_us,
                status: status.map(String::from),
                limit,
            })
            .await?;

        match response {
            ClientRpcResponse::TraceSearchResult(result) => Ok(result),
            ClientRpcResponse::Error(e) => Err(anyhow::anyhow!("trace search failed: {}", e.message)),
            other => Err(anyhow::anyhow!("unexpected response: {:?}", other)),
        }
    }

    /// Export all spans in OpenTelemetry format.
    pub async fn export_spans(&self) -> Vec<Span> {
        self.spans.read().await.clone()
    }

    /// Clear all collected spans and pending buffer.
    pub async fn clear_spans(&self) {
        self.spans.write().await.clear();
        self.pending_spans.write().await.clear();
    }

    // =========================================================================
    // Metrics — server-side storage
    // =========================================================================

    /// Ingest a batch of metric data points to the server.
    pub async fn ingest_metrics(
        &self,
        data_points: Vec<MetricDataPoint>,
        ttl_seconds: Option<u32>,
    ) -> Result<MetricIngestResultResponse> {
        let response = self
            .client
            .send(ClientRpcRequest::MetricIngest {
                data_points,
                ttl_seconds,
            })
            .await?;

        match response {
            ClientRpcResponse::MetricIngestResult(result) => Ok(result),
            ClientRpcResponse::Error(e) => Err(anyhow::anyhow!("metric ingest failed: {}", e.message)),
            other => Err(anyhow::anyhow!("unexpected response: {:?}", other)),
        }
    }

    /// List available metric names.
    pub async fn list_metrics(&self, prefix: Option<&str>, limit: Option<u32>) -> Result<MetricListResultResponse> {
        let response = self
            .client
            .send(ClientRpcRequest::MetricList {
                prefix: prefix.map(String::from),
                limit,
            })
            .await?;

        match response {
            ClientRpcResponse::MetricListResult(result) => Ok(result),
            ClientRpcResponse::Error(e) => Err(anyhow::anyhow!("metric list failed: {}", e.message)),
            other => Err(anyhow::anyhow!("unexpected response: {:?}", other)),
        }
    }

    /// Query metric data points by name and optional filters.
    #[allow(clippy::too_many_arguments)]
    pub async fn query_metrics(
        &self,
        name: &str,
        start_time_us: Option<u64>,
        end_time_us: Option<u64>,
        label_filters: Vec<(String, String)>,
        aggregation: Option<&str>,
        step_us: Option<u64>,
        limit: Option<u32>,
    ) -> Result<MetricQueryResultResponse> {
        let response = self
            .client
            .send(ClientRpcRequest::MetricQuery {
                name: name.to_string(),
                start_time_us,
                end_time_us,
                label_filters,
                aggregation: aggregation.map(String::from),
                step_us,
                limit,
            })
            .await?;

        match response {
            ClientRpcResponse::MetricQueryResult(result) => Ok(result),
            ClientRpcResponse::Error(e) => Err(anyhow::anyhow!("metric query failed: {}", e.message)),
            other => Err(anyhow::anyhow!("unexpected response: {:?}", other)),
        }
    }

    // =========================================================================
    // Alerts
    // =========================================================================

    /// Create or update an alert rule.
    pub async fn create_alert(&self, rule: AlertRuleWire) -> Result<AlertRuleResultResponse> {
        let response = self.client.send(ClientRpcRequest::AlertCreate { rule }).await?;

        match response {
            ClientRpcResponse::AlertCreateResult(result) => Ok(result),
            ClientRpcResponse::Error(e) => Err(anyhow::anyhow!("alert create failed: {}", e.message)),
            other => Err(anyhow::anyhow!("unexpected response: {:?}", other)),
        }
    }

    /// Delete an alert rule.
    pub async fn delete_alert(&self, name: &str) -> Result<AlertRuleResultResponse> {
        let response = self.client.send(ClientRpcRequest::AlertDelete { name: name.to_string() }).await?;

        match response {
            ClientRpcResponse::AlertDeleteResult(result) => Ok(result),
            ClientRpcResponse::Error(e) => Err(anyhow::anyhow!("alert delete failed: {}", e.message)),
            other => Err(anyhow::anyhow!("unexpected response: {:?}", other)),
        }
    }

    /// List all alert rules with current state.
    pub async fn list_alerts(&self) -> Result<AlertListResultResponse> {
        let response = self.client.send(ClientRpcRequest::AlertList).await?;

        match response {
            ClientRpcResponse::AlertListResult(result) => Ok(result),
            ClientRpcResponse::Error(e) => Err(anyhow::anyhow!("alert list failed: {}", e.message)),
            other => Err(anyhow::anyhow!("unexpected response: {:?}", other)),
        }
    }

    /// Get a single alert rule with state and history.
    pub async fn get_alert(&self, name: &str) -> Result<AlertGetResultResponse> {
        let response = self.client.send(ClientRpcRequest::AlertGet { name: name.to_string() }).await?;

        match response {
            ClientRpcResponse::AlertGetResult(result) => Ok(result),
            ClientRpcResponse::Error(e) => Err(anyhow::anyhow!("alert get failed: {}", e.message)),
            other => Err(anyhow::anyhow!("unexpected response: {:?}", other)),
        }
    }

    /// Evaluate an alert rule on-demand.
    pub async fn evaluate_alert(&self, name: &str, now_us: u64) -> Result<AlertEvaluateResultResponse> {
        let response = self
            .client
            .send(ClientRpcRequest::AlertEvaluate {
                name: name.to_string(),
                now_us,
            })
            .await?;

        match response {
            ClientRpcResponse::AlertEvaluateResult(result) => Ok(result),
            ClientRpcResponse::Error(e) => Err(anyhow::anyhow!("alert evaluate failed: {}", e.message)),
            other => Err(anyhow::anyhow!("unexpected response: {:?}", other)),
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // TraceContext tests
    // =========================================================================

    #[test]
    fn test_trace_context_new_root() {
        let ctx = TraceContext::new_root();
        assert_eq!(ctx.trace_id.len(), 32, "trace_id should be 32 hex chars");
        assert_eq!(ctx.span_id.len(), 16, "span_id should be 16 hex chars");
        assert_eq!(ctx.parent_id, "0000000000000000", "root has zero parent");
        assert_eq!(ctx.flags, 0x01, "default is sampled");
        assert!(ctx.state.is_none());
    }

    #[test]
    fn test_trace_context_new_root_unique_ids() {
        let a = TraceContext::new_root();
        let b = TraceContext::new_root();
        assert_ne!(a.trace_id, b.trace_id, "different roots should have different trace IDs");
        assert_ne!(a.span_id, b.span_id, "different roots should have different span IDs");
    }

    #[test]
    fn test_trace_context_child_preserves_trace_id() {
        let root = TraceContext::new_root();
        let child = root.child();

        assert_eq!(child.trace_id, root.trace_id, "child inherits trace_id");
        assert_eq!(child.parent_id, root.span_id, "child's parent is root's span");
        assert_ne!(child.span_id, root.span_id, "child gets new span_id");
        assert_eq!(child.flags, root.flags, "child inherits flags");
    }

    #[test]
    fn test_trace_context_child_chain() {
        let root = TraceContext::new_root();
        let child = root.child();
        let grandchild = child.child();

        assert_eq!(grandchild.trace_id, root.trace_id);
        assert_eq!(grandchild.parent_id, child.span_id);
        assert_ne!(grandchild.span_id, child.span_id);
    }

    #[test]
    fn test_traceparent_roundtrip() {
        let ctx = TraceContext::new_root();
        let header = ctx.to_traceparent();

        // Format: 00-{trace_id}-{span_id}-{flags}
        let parts: Vec<&str> = header.split('-').collect();
        assert_eq!(parts.len(), 4);
        assert_eq!(parts[0], "00", "version");
        assert_eq!(parts[1], ctx.trace_id);
        assert_eq!(parts[2], ctx.span_id);
        assert_eq!(parts[3], "01", "sampled flag");

        let parsed = TraceContext::from_traceparent(&header).unwrap();
        assert_eq!(parsed.trace_id, ctx.trace_id);
        assert_eq!(parsed.span_id, ctx.span_id);
        assert_eq!(parsed.flags, ctx.flags);
    }

    #[test]
    fn test_traceparent_parse_invalid() {
        assert!(TraceContext::from_traceparent("invalid").is_err());
        assert!(TraceContext::from_traceparent("00-abc").is_err());
        assert!(TraceContext::from_traceparent("").is_err());
    }

    // =========================================================================
    // Span tests
    // =========================================================================

    #[test]
    fn test_span_new_root() {
        let span = Span::new("test.operation", None);
        assert_eq!(span.operation, "test.operation");
        assert_eq!(span.context.parent_id, "0000000000000000");
        assert!(span.end_time.is_none());
        assert_eq!(span.status, SpanStatus::Unset);
        assert!(span.attributes.is_empty());
        assert!(span.events.is_empty());
    }

    #[test]
    fn test_span_new_with_parent() {
        let parent_ctx = TraceContext::new_root();
        let span = Span::new("child.op", Some(&parent_ctx));
        assert_eq!(span.context.trace_id, parent_ctx.trace_id);
        assert_eq!(span.context.parent_id, parent_ctx.span_id);
    }

    #[test]
    fn test_span_set_attribute() {
        let mut span = Span::new("op", None);
        span.set_attribute("http.method", "GET");
        span.set_attribute("http.status_code", "200");

        assert_eq!(span.attributes.len(), 2);
        assert_eq!(span.attributes.get("http.method").unwrap(), "GET");
    }

    #[test]
    fn test_span_add_event() {
        let mut span = Span::new("op", None);
        let mut attrs = HashMap::new();
        attrs.insert("key".to_string(), "value".to_string());
        span.add_event("cache.miss", attrs);

        assert_eq!(span.events.len(), 1);
        assert_eq!(span.events[0].name, "cache.miss");
    }

    #[test]
    fn test_span_set_status() {
        let mut span = Span::new("op", None);
        assert_eq!(span.status, SpanStatus::Unset);

        span.set_status(SpanStatus::Ok);
        assert_eq!(span.status, SpanStatus::Ok);

        span.set_status(SpanStatus::Error("fail".to_string()));
        assert_eq!(span.status, SpanStatus::Error("fail".to_string()));
    }

    #[test]
    fn test_span_end_and_duration() {
        let mut span = Span::new("op", None);
        assert!(span.duration().is_none(), "unfinished span has no duration");

        span.end();
        assert!(span.end_time.is_some());
        assert!(span.duration().is_some());
    }

    // =========================================================================
    // span_to_ingest conversion tests
    // =========================================================================

    #[test]
    fn test_span_to_ingest_basic() {
        let mut span = Span::new("test.op", None);
        span.set_status(SpanStatus::Ok);
        span.end();

        let ingest = span_to_ingest(&span);
        assert_eq!(ingest.trace_id, span.context.trace_id);
        assert_eq!(ingest.span_id, span.context.span_id);
        assert_eq!(ingest.parent_id, span.context.parent_id);
        assert_eq!(ingest.operation, "test.op");
        assert!(ingest.start_time_us > 0, "should have a real timestamp");
        assert_eq!(ingest.status, SpanStatusWire::Ok);
    }

    #[test]
    fn test_span_to_ingest_error_status() {
        let mut span = Span::new("fail.op", None);
        span.set_status(SpanStatus::Error("boom".to_string()));
        span.end();

        let ingest = span_to_ingest(&span);
        assert_eq!(ingest.status, SpanStatusWire::Error("boom".to_string()));
    }

    #[test]
    fn test_span_to_ingest_with_attributes() {
        let mut span = Span::new("op", None);
        span.set_attribute("k1", "v1");
        span.set_attribute("k2", "v2");
        span.end();

        let ingest = span_to_ingest(&span);
        assert_eq!(ingest.attributes.len(), 2);
    }

    #[test]
    fn test_span_to_ingest_truncates_attributes() {
        let mut span = Span::new("op", None);
        let max = aspen_constants::MAX_SPAN_ATTRIBUTES as usize;
        for i in 0..max + 10 {
            span.set_attribute(format!("key_{}", i), "val");
        }
        span.end();

        let ingest = span_to_ingest(&span);
        assert!(ingest.attributes.len() <= max, "should be bounded to MAX_SPAN_ATTRIBUTES");
    }

    #[test]
    fn test_span_to_ingest_truncates_events() {
        let mut span = Span::new("op", None);
        let max = aspen_constants::MAX_SPAN_EVENTS as usize;
        for i in 0..max + 5 {
            span.add_event(format!("event_{}", i), HashMap::new());
        }
        span.end();

        let ingest = span_to_ingest(&span);
        assert!(ingest.events.len() <= max, "should be bounded to MAX_SPAN_EVENTS");
    }

    // =========================================================================
    // MetricsCollector tests
    // =========================================================================

    #[tokio::test]
    async fn test_metrics_counter() {
        let metrics = MetricsCollector::new();
        metrics.increment("requests", 1).await;
        metrics.increment("requests", 5).await;

        let counters = metrics.get_counters().await;
        assert_eq!(counters.get("requests"), Some(&6));
    }

    #[tokio::test]
    async fn test_metrics_gauge() {
        let metrics = MetricsCollector::new();
        metrics.gauge("active_connections", 42.0).await;
        metrics.gauge("active_connections", 37.0).await;

        let gauges = metrics.get_gauges().await;
        assert_eq!(gauges.get("active_connections"), Some(&37.0));
    }

    #[tokio::test]
    async fn test_metrics_histogram() {
        let metrics = MetricsCollector::new();
        for v in &[10.0, 20.0, 30.0, 40.0, 50.0] {
            metrics.histogram("latency_ms", *v).await;
        }

        let stats = metrics.get_histogram_stats("latency_ms").await.unwrap();
        assert_eq!(stats.count, 5);
        assert!((stats.sum - 150.0).abs() < f64::EPSILON);
        assert!((stats.mean - 30.0).abs() < f64::EPSILON);
        assert!((stats.min - 10.0).abs() < f64::EPSILON);
        assert!((stats.max - 50.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_metrics_histogram_nonexistent() {
        let metrics = MetricsCollector::new();
        assert!(metrics.get_histogram_stats("nosuch").await.is_none());
    }

    #[tokio::test]
    async fn test_metrics_prometheus_export() {
        let metrics = MetricsCollector::new().with_label("node", "1");
        metrics.increment("my_counter", 10).await;
        metrics.gauge("my_gauge", 3.5).await;

        let output = metrics.export_prometheus().await;
        assert!(output.contains("my_counter"));
        assert!(output.contains("10"));
        assert!(output.contains("my_gauge"));
        assert!(output.contains("3.5"));
        assert!(output.contains("node=\"1\""));
    }

    // =========================================================================
    // percentile helper
    // =========================================================================

    #[test]
    fn test_percentile_empty() {
        assert!((percentile(&[], 0.5) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_percentile_single() {
        assert!((percentile(&[42.0], 0.5) - 42.0).abs() < f64::EPSILON);
        assert!((percentile(&[42.0], 0.99) - 42.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_percentile_sorted() {
        let values: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        let p50 = percentile(&values, 0.50);
        let p99 = percentile(&values, 0.99);
        // p50 of 1..100 should be around 50
        assert!((49.0..=51.0).contains(&p50));
        // p99 should be around 99
        assert!((98.0..=100.0).contains(&p99));
    }

    // =========================================================================
    // ObservabilityBuilder tests
    // =========================================================================

    #[test]
    fn test_builder_defaults() {
        let builder = ObservabilityBuilder::default();
        assert!(builder.tracing_enabled);
        assert!(builder.metrics_enabled);
        assert!((builder.sampling_rate - 1.0).abs() < f64::EPSILON);
        assert_eq!(builder.max_spans, 10000);
    }

    #[test]
    fn test_builder_sampling_rate_clamp() {
        let builder = ObservabilityBuilder::default().with_sampling_rate(2.0);
        assert!((builder.sampling_rate - 1.0).abs() < f64::EPSILON);

        let builder = ObservabilityBuilder::default().with_sampling_rate(-0.5);
        assert!((builder.sampling_rate - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_builder_chaining() {
        let builder = ObservabilityBuilder::default()
            .with_tracing(false)
            .with_metrics(false)
            .with_sampling_rate(0.5)
            .with_max_spans(500)
            .with_export_interval(Duration::from_secs(30));

        assert!(!builder.tracing_enabled);
        assert!(!builder.metrics_enabled);
        assert!((builder.sampling_rate - 0.5).abs() < f64::EPSILON);
        assert_eq!(builder.max_spans, 500);
        assert_eq!(builder.export_interval, Duration::from_secs(30));
    }
}
