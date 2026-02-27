//! Trace query commands.
//!
//! Commands for querying distributed traces stored by the observability
//! pipeline. Traces are ingested via `TraceIngest` and stored as
//! `_sys:traces:{trace_id}:{span_id}` KV entries.

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::SpanStatusWire;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::Outputable;
use crate::output::print_output;

/// Distributed trace query operations.
#[derive(Subcommand)]
pub enum TraceCommand {
    /// List recent traces (newest first).
    List(TraceListArgs),

    /// Get all spans for a specific trace.
    Get(TraceGetArgs),

    /// Search spans by criteria.
    Search(TraceSearchArgs),
}

#[derive(Args)]
pub struct TraceListArgs {
    /// Start time filter (Unix microseconds, inclusive).
    #[arg(long)]
    pub start: Option<u64>,

    /// End time filter (Unix microseconds, exclusive).
    #[arg(long)]
    pub end: Option<u64>,

    /// Maximum traces to return.
    #[arg(long, default_value = "100")]
    pub limit: u32,
}

#[derive(Args)]
pub struct TraceGetArgs {
    /// Trace ID (32 hex chars).
    pub trace_id: String,
}

#[derive(Args)]
pub struct TraceSearchArgs {
    /// Filter by operation name (case-insensitive substring).
    #[arg(long)]
    pub operation: Option<String>,

    /// Minimum span duration in microseconds.
    #[arg(long)]
    pub min_duration_us: Option<u64>,

    /// Maximum span duration in microseconds.
    #[arg(long)]
    pub max_duration_us: Option<u64>,

    /// Filter by status: ok, error, unset.
    #[arg(long)]
    pub status: Option<String>,

    /// Maximum spans to return.
    #[arg(long, default_value = "100")]
    pub limit: u32,
}

// =============================================================================
// Output types
// =============================================================================

struct TraceListOutput {
    traces: Vec<TraceSummaryRow>,
    count: u32,
    is_truncated: bool,
}

struct TraceSummaryRow {
    trace_id: String,
    span_count: u32,
    root_operation: Option<String>,
    start_time_us: u64,
    total_duration_us: u64,
    has_error: bool,
}

impl Outputable for TraceListOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "traces": self.traces.iter().map(|t| serde_json::json!({
                "trace_id": t.trace_id,
                "span_count": t.span_count,
                "root_operation": t.root_operation,
                "start_time_us": t.start_time_us,
                "total_duration_us": t.total_duration_us,
                "has_error": t.has_error,
            })).collect::<Vec<_>>(),
            "count": self.count,
            "is_truncated": self.is_truncated,
        })
    }

    fn to_human(&self) -> String {
        if self.traces.is_empty() {
            return "No traces found".to_string();
        }
        let mut out = format!("Traces ({}{}):\n\n", self.count, if self.is_truncated { "+" } else { "" });
        out.push_str("TRACE ID                          | SPANS | OPERATION            | DURATION    | ERR\n");
        out.push_str("----------------------------------+-------+----------------------+-------------+----\n");
        for t in &self.traces {
            let op = t.root_operation.as_deref().unwrap_or("-");
            let op_display = if op.len() > 20 { &op[..20] } else { op };
            let dur = format_duration_us(t.total_duration_us);
            let err = if t.has_error { "!" } else { " " };
            out.push_str(&format!(
                "{} | {:>5} | {:20} | {:>11} | {}\n",
                t.trace_id, t.span_count, op_display, dur, err
            ));
        }
        out
    }
}

struct TraceGetOutput {
    trace_id: String,
    spans: Vec<SpanRow>,
    span_count: u32,
}

struct SpanRow {
    span_id: String,
    parent_id: String,
    operation: String,
    start_time_us: u64,
    duration_us: u64,
    status: String,
}

impl Outputable for TraceGetOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "trace_id": self.trace_id,
            "span_count": self.span_count,
            "spans": self.spans.iter().map(|s| serde_json::json!({
                "span_id": s.span_id,
                "parent_id": s.parent_id,
                "operation": s.operation,
                "start_time_us": s.start_time_us,
                "duration_us": s.duration_us,
                "status": s.status,
            })).collect::<Vec<_>>(),
        })
    }

    fn to_human(&self) -> String {
        if self.spans.is_empty() {
            return format!("Trace {} not found (0 spans)", self.trace_id);
        }
        let mut out = format!("Trace {} ({} spans):\n\n", self.trace_id, self.span_count);
        out.push_str("SPAN ID          | PARENT           | OPERATION            | DURATION    | STATUS\n");
        out.push_str("-----------------+------------------+----------------------+-------------+-------\n");
        for s in &self.spans {
            let op = if s.operation.len() > 20 {
                &s.operation[..20]
            } else {
                &s.operation
            };
            let dur = format_duration_us(s.duration_us);
            out.push_str(&format!("{} | {} | {:20} | {:>11} | {}\n", s.span_id, s.parent_id, op, dur, s.status));
        }
        out
    }
}

struct TraceSearchOutput {
    spans: Vec<SpanRow>,
    count: u32,
    is_truncated: bool,
}

impl Outputable for TraceSearchOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "count": self.count,
            "is_truncated": self.is_truncated,
            "spans": self.spans.iter().map(|s| serde_json::json!({
                "span_id": s.span_id,
                "parent_id": s.parent_id,
                "operation": s.operation,
                "start_time_us": s.start_time_us,
                "duration_us": s.duration_us,
                "status": s.status,
            })).collect::<Vec<_>>(),
        })
    }

    fn to_human(&self) -> String {
        if self.spans.is_empty() {
            return "No matching spans found".to_string();
        }
        let mut out = format!("Search results ({}{}):\n\n", self.count, if self.is_truncated { "+" } else { "" });
        out.push_str("SPAN ID          | OPERATION            | DURATION    | STATUS\n");
        out.push_str("-----------------+----------------------+-------------+-------\n");
        for s in &self.spans {
            let op = if s.operation.len() > 20 {
                &s.operation[..20]
            } else {
                &s.operation
            };
            let dur = format_duration_us(s.duration_us);
            out.push_str(&format!("{} | {:20} | {:>11} | {}\n", s.span_id, op, dur, s.status));
        }
        out
    }
}

/// Format microseconds into a human-readable duration string.
fn format_duration_us(us: u64) -> String {
    if us < 1_000 {
        format!("{}µs", us)
    } else if us < 1_000_000 {
        format!("{:.1}ms", us as f64 / 1_000.0)
    } else {
        format!("{:.2}s", us as f64 / 1_000_000.0)
    }
}

fn status_to_string(status: &SpanStatusWire) -> String {
    match status {
        SpanStatusWire::Unset => "unset".to_string(),
        SpanStatusWire::Ok => "ok".to_string(),
        SpanStatusWire::Error(msg) => format!("error: {}", msg),
    }
}

// =============================================================================
// Command dispatch
// =============================================================================

impl TraceCommand {
    pub async fn run(self, client: &AspenClient, is_json: bool) -> Result<()> {
        match self {
            Self::List(args) => run_trace_list(client, args, is_json).await,
            Self::Get(args) => run_trace_get(client, args, is_json).await,
            Self::Search(args) => run_trace_search(client, args, is_json).await,
        }
    }
}

async fn run_trace_list(client: &AspenClient, args: TraceListArgs, is_json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::TraceList {
            start_time_us: args.start,
            end_time_us: args.end,
            limit: Some(args.limit),
            continuation_token: None,
        })
        .await?;

    match response {
        ClientRpcResponse::TraceListResult(result) => {
            if let Some(err) = result.error {
                anyhow::bail!("trace list failed: {}", err);
            }
            let output = TraceListOutput {
                traces: result
                    .traces
                    .into_iter()
                    .map(|t| TraceSummaryRow {
                        trace_id: t.trace_id,
                        span_count: t.span_count,
                        root_operation: t.root_operation,
                        start_time_us: t.start_time_us,
                        total_duration_us: t.total_duration_us,
                        has_error: t.has_error,
                    })
                    .collect(),
                count: result.count,
                is_truncated: result.is_truncated,
            };
            print_output(&output, is_json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("trace list failed: {}", e.message),
        other => anyhow::bail!("unexpected response: {:?}", other),
    }
}

async fn run_trace_get(client: &AspenClient, args: TraceGetArgs, is_json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::TraceGet {
            trace_id: args.trace_id.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::TraceGetResult(result) => {
            if let Some(err) = result.error {
                anyhow::bail!("trace get failed: {}", err);
            }
            let output = TraceGetOutput {
                trace_id: result.trace_id,
                spans: result
                    .spans
                    .iter()
                    .map(|s| SpanRow {
                        span_id: s.span_id.clone(),
                        parent_id: s.parent_id.clone(),
                        operation: s.operation.clone(),
                        start_time_us: s.start_time_us,
                        duration_us: s.duration_us,
                        status: status_to_string(&s.status),
                    })
                    .collect(),
                span_count: result.span_count,
            };
            print_output(&output, is_json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("trace get failed: {}", e.message),
        other => anyhow::bail!("unexpected response: {:?}", other),
    }
}

async fn run_trace_search(client: &AspenClient, args: TraceSearchArgs, is_json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::TraceSearch {
            operation: args.operation,
            min_duration_us: args.min_duration_us,
            max_duration_us: args.max_duration_us,
            status: args.status,
            limit: Some(args.limit),
        })
        .await?;

    match response {
        ClientRpcResponse::TraceSearchResult(result) => {
            if let Some(err) = result.error {
                anyhow::bail!("trace search failed: {}", err);
            }
            let output = TraceSearchOutput {
                spans: result
                    .spans
                    .iter()
                    .map(|s| SpanRow {
                        span_id: s.span_id.clone(),
                        parent_id: s.parent_id.clone(),
                        operation: s.operation.clone(),
                        start_time_us: s.start_time_us,
                        duration_us: s.duration_us,
                        status: status_to_string(&s.status),
                    })
                    .collect(),
                count: result.count,
                is_truncated: result.is_truncated,
            };
            print_output(&output, is_json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("trace search failed: {}", e.message),
        other => anyhow::bail!("unexpected response: {:?}", other),
    }
}
