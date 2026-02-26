//! Distributed tracing types and filters.

use std::collections::HashMap;

use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;

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
    pub(crate) fn matches(&self, span: &TraceSpan) -> bool {
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
