//! Observability wire types for traces, metrics, and alerting.
//!
//! These types are used to serialize completed trace spans, metric data points,
//! and alert rules for transport over the client RPC protocol.
//!
//! # Tiger Style
//!
//! - All collections are bounded (MAX_TRACE_BATCH_SIZE, MAX_METRIC_BATCH_SIZE, etc.)
//! - Timestamps use explicit units: `_us` suffix for microseconds
//! - No unbounded allocations

use serde::Deserialize;
use serde::Serialize;

/// A completed span for trace ingest.
///
/// Wire-format representation of a finished distributed tracing span.
/// Converted from the client-side `Span` type before transmission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestSpan {
    /// Trace ID (32 hex chars).
    pub trace_id: String,
    /// Span ID (16 hex chars).
    pub span_id: String,
    /// Parent span ID (16 hex chars, "0000000000000000" for root).
    pub parent_id: String,
    /// Operation name.
    pub operation: String,
    /// Start time as Unix microseconds.
    pub start_time_us: u64,
    /// Duration in microseconds.
    pub duration_us: u64,
    /// Span status.
    pub status: SpanStatusWire,
    /// Span attributes (bounded by MAX_SPAN_ATTRIBUTES).
    pub attributes: Vec<(String, String)>,
    /// Span events (bounded by MAX_SPAN_EVENTS).
    pub events: Vec<SpanEventWire>,
}

/// Wire-format span status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SpanStatusWire {
    /// Status not set.
    Unset,
    /// Operation completed successfully.
    Ok,
    /// Operation failed with error message.
    Error(String),
}

/// Wire-format span event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanEventWire {
    /// Event name.
    pub name: String,
    /// Event timestamp as Unix microseconds.
    pub timestamp_us: u64,
    /// Event attributes.
    pub attributes: Vec<(String, String)>,
}

/// Response from trace ingest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceIngestResultResponse {
    /// Whether the ingest succeeded.
    pub is_success: bool,
    /// Number of spans accepted.
    pub accepted_count: u32,
    /// Number of spans dropped (exceeded batch limit).
    pub dropped_count: u32,
    /// Error message if ingest failed.
    pub error: Option<String>,
}

// =============================================================================
// Trace query response types
// =============================================================================

/// Summary of a single trace (used in TraceList responses).
///
/// Aggregated from all spans sharing a trace_id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSummary {
    /// Trace ID (32 hex chars).
    pub trace_id: String,
    /// Number of spans in this trace.
    pub span_count: u32,
    /// Operation name of the root span (parent_id == "0000000000000000").
    pub root_operation: Option<String>,
    /// Earliest span start time as Unix microseconds.
    pub start_time_us: u64,
    /// Total trace duration in microseconds (max end - min start).
    pub total_duration_us: u64,
    /// Whether any span in the trace has an error status.
    pub has_error: bool,
}

/// Response from TraceList.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceListResultResponse {
    /// Trace summaries (bounded by query limit).
    pub traces: Vec<TraceSummary>,
    /// Number of traces returned.
    pub count: u32,
    /// Whether results were truncated by the limit.
    pub is_truncated: bool,
    /// Error message if query failed.
    pub error: Option<String>,
}

/// Response from TraceGet (all spans for one trace).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceGetResultResponse {
    /// Trace ID queried.
    pub trace_id: String,
    /// All spans belonging to this trace.
    pub spans: Vec<IngestSpan>,
    /// Number of spans returned.
    pub span_count: u32,
    /// Error message if query failed.
    pub error: Option<String>,
}

/// Response from TraceSearch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSearchResultResponse {
    /// Matching spans (bounded by query limit).
    pub spans: Vec<IngestSpan>,
    /// Number of spans returned.
    pub count: u32,
    /// Whether results were truncated by the limit.
    pub is_truncated: bool,
    /// Error message if query failed.
    pub error: Option<String>,
}

// =============================================================================
// Metric wire types
// =============================================================================

/// Metric data point type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MetricTypeWire {
    /// Monotonically increasing counter (resets to zero on restart).
    Counter,
    /// Point-in-time value that can go up or down.
    Gauge,
    /// Distribution of values (pre-aggregated client-side).
    Histogram,
}

/// A single metric data point for ingest.
///
/// Wire-format representation. Clients batch these and send via `MetricIngest`.
/// Labels are bounded by `MAX_METRIC_LABELS`, name by `MAX_METRIC_NAME_SIZE`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricDataPoint {
    /// Metric name (e.g., "kv.read.latency_us", "raft.commit.count").
    pub name: String,
    /// Metric type (counter, gauge, histogram).
    pub metric_type: MetricTypeWire,
    /// Timestamp as Unix microseconds.
    pub timestamp_us: u64,
    /// Metric value (for counter/gauge: the value; for histogram: ignored).
    pub value: f64,
    /// Labels for dimensional filtering (bounded by `MAX_METRIC_LABELS`).
    pub labels: Vec<(String, String)>,
    /// Histogram bucket counts: `(upper_bound_exclusive, cumulative_count)`.
    /// Only used for `MetricTypeWire::Histogram`.
    pub histogram_buckets: Option<Vec<(f64, u64)>>,
    /// Histogram sum (only for Histogram type).
    pub histogram_sum: Option<f64>,
    /// Histogram sample count (only for Histogram type).
    pub histogram_count: Option<u64>,
}

/// Metadata about a registered metric name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricMetadata {
    /// Metric name.
    pub name: String,
    /// Metric type.
    pub metric_type: MetricTypeWire,
    /// Human-readable description.
    pub description: String,
    /// Unit string (e.g., "us", "bytes", "requests").
    pub unit: String,
    /// Last seen label keys.
    pub label_keys: Vec<String>,
    /// Last update timestamp in Unix microseconds.
    pub last_updated_us: u64,
}

/// Response from metric ingest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricIngestResultResponse {
    /// Whether the ingest succeeded.
    pub is_success: bool,
    /// Number of data points accepted.
    pub accepted_count: u32,
    /// Number of data points dropped (exceeded batch limit).
    pub dropped_count: u32,
    /// Error message if ingest failed.
    pub error: Option<String>,
}

/// Response from metric query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricQueryResultResponse {
    /// Metric name queried.
    pub name: String,
    /// Matching data points (chronologically ordered).
    pub data_points: Vec<MetricDataPoint>,
    /// Number of results returned.
    pub count: u32,
    /// Whether results were truncated.
    pub is_truncated: bool,
    /// Error message if query failed.
    pub error: Option<String>,
}

/// Response from metric list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricListResultResponse {
    /// Available metric metadata entries.
    pub metrics: Vec<MetricMetadata>,
    /// Number of metric names returned.
    pub count: u32,
    /// Error message if list failed.
    pub error: Option<String>,
}

// =============================================================================
// Alert wire types
// =============================================================================

/// Comparison operator for alert thresholds.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertComparison {
    /// Value > threshold.
    GreaterThan,
    /// Value >= threshold.
    GreaterThanOrEqual,
    /// Value < threshold.
    LessThan,
    /// Value <= threshold.
    LessThanOrEqual,
    /// Value == threshold.
    Equal,
    /// Value != threshold.
    NotEqual,
}

/// Alert severity level.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertSeverity {
    /// Informational — no action required.
    Info,
    /// Warning — investigate soon.
    Warning,
    /// Critical — immediate action required.
    Critical,
}

/// Alert rule state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertStatus {
    /// Rule defined but threshold not breached.
    Ok,
    /// Threshold breached, waiting for `for_duration_us` before firing.
    Pending,
    /// Threshold breached for longer than `for_duration_us`.
    Firing,
}

/// Wire-format alert rule definition.
///
/// Stored under `_sys:alerts:rule:{name}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRuleWire {
    /// Unique rule name.
    pub name: String,
    /// Metric name to monitor.
    pub metric_name: String,
    /// Optional label filters. Only data points matching ALL labels are evaluated.
    pub label_filters: Vec<(String, String)>,
    /// Aggregation over the evaluation window: "avg", "max", "min", "sum", "last", "count".
    pub aggregation: String,
    /// Evaluation window in microseconds (how far back to aggregate).
    pub window_duration_us: u64,
    /// Comparison operator.
    pub comparison: AlertComparison,
    /// Threshold value.
    pub threshold: f64,
    /// Duration the condition must hold before firing (microseconds, 0 = immediate).
    pub for_duration_us: u64,
    /// Alert severity.
    pub severity: AlertSeverity,
    /// Human-readable description.
    pub description: String,
    /// Whether the rule is enabled.
    pub is_enabled: bool,
    /// Creation timestamp in Unix microseconds.
    pub created_at_us: u64,
    /// Last modification timestamp in Unix microseconds.
    pub updated_at_us: u64,
}

/// Current state of an alert rule, stored under `_sys:alerts:state:{name}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertStateWire {
    /// Rule name.
    pub rule_name: String,
    /// Current status.
    pub status: AlertStatus,
    /// Aggregated metric value from last evaluation.
    pub last_value: Option<f64>,
    /// Timestamp of last evaluation in Unix microseconds.
    pub last_evaluated_us: u64,
    /// When the condition first became true (for pending→firing transition).
    pub condition_since_us: Option<u64>,
    /// When the alert last fired (transitioned to Firing).
    pub last_fired_us: Option<u64>,
    /// When the alert last resolved (transitioned from Firing to Ok).
    pub last_resolved_us: Option<u64>,
}

/// A single history entry for an alert state transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertHistoryEntry {
    /// Rule name.
    pub rule_name: String,
    /// Transition: previous status.
    pub from_status: AlertStatus,
    /// Transition: new status.
    pub to_status: AlertStatus,
    /// Aggregated value at transition time.
    pub value: f64,
    /// Threshold at transition time.
    pub threshold: f64,
    /// Transition timestamp in Unix microseconds.
    pub timestamp_us: u64,
}

/// Response from alert create/delete.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRuleResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Rule name affected.
    pub rule_name: String,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Response from alert list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertListResultResponse {
    /// Alert rules with their current states.
    pub rules: Vec<AlertRuleWithState>,
    /// Number of rules returned.
    pub count: u32,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Alert rule paired with its current state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRuleWithState {
    /// The rule definition.
    pub rule: AlertRuleWire,
    /// Current state (if evaluated at least once).
    pub state: Option<AlertStateWire>,
}

/// Response from alert get (single rule + state + history).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertGetResultResponse {
    /// The rule definition.
    pub rule: Option<AlertRuleWire>,
    /// Current state.
    pub state: Option<AlertStateWire>,
    /// Recent history entries (bounded by `MAX_ALERT_HISTORY_PER_RULE`).
    pub history: Vec<AlertHistoryEntry>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Response from alert evaluate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertEvaluateResultResponse {
    /// Rule name evaluated.
    pub rule_name: String,
    /// Status after evaluation.
    pub status: AlertStatus,
    /// Computed metric value.
    pub computed_value: Option<f64>,
    /// Threshold from rule.
    pub threshold: f64,
    /// Whether a state transition occurred.
    pub did_transition: bool,
    /// Previous status (if transition occurred).
    pub previous_status: Option<AlertStatus>,
    /// Error message if evaluation failed.
    pub error: Option<String>,
}
