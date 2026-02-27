//! Observability wire types for trace ingest.
//!
//! These types are used to serialize completed trace spans for transport
//! over the client RPC protocol. The client batches spans and sends them
//! to the server for storage and aggregation.
//!
//! # Tiger Style
//!
//! - All collections are bounded (MAX_TRACE_BATCH_SIZE, MAX_SPAN_ATTRIBUTES, MAX_SPAN_EVENTS)
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
