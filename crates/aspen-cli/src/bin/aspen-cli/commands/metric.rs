//! Metric commands for server-side metric operations.
//!
//! Commands for ingesting, listing, and querying metrics stored by the
//! observability pipeline. Metrics are stored as `_sys:metrics:{name}:{ts}`
//! KV entries with TTL-based expiration.

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::MetricDataPoint;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::Outputable;
use crate::output::print_output;

/// Server-side metric operations.
#[derive(Subcommand)]
pub enum MetricCommand {
    /// List available metric names.
    List(MetricListArgs),

    /// Query metric data points.
    Query(MetricQueryArgs),

    /// Ingest metric data points from stdin (JSON array).
    Ingest(MetricIngestArgs),
}

#[derive(Args)]
pub struct MetricListArgs {
    /// Filter by name prefix.
    #[arg(long)]
    pub prefix: Option<String>,

    /// Maximum results.
    #[arg(long, default_value = "100")]
    pub limit: u32,
}

#[derive(Args)]
pub struct MetricQueryArgs {
    /// Metric name to query.
    pub name: String,

    /// Start time (Unix microseconds, inclusive).
    #[arg(long)]
    pub start_us: Option<u64>,

    /// End time (Unix microseconds, exclusive).
    #[arg(long)]
    pub end_us: Option<u64>,

    /// Label filter (key=value), repeatable.
    #[arg(long = "label", value_parser = parse_label_filter)]
    pub labels: Vec<(String, String)>,

    /// Aggregation: none, avg, max, min, sum, count, last.
    #[arg(long, default_value = "none")]
    pub aggregation: String,

    /// Aggregation step size in microseconds.
    #[arg(long)]
    pub step_us: Option<u64>,

    /// Maximum data points to return.
    #[arg(long, default_value = "500")]
    pub limit: u32,
}

#[derive(Args)]
pub struct MetricIngestArgs {
    /// TTL in seconds for data points.
    #[arg(long, default_value = "86400")]
    pub ttl_seconds: u32,
}

fn parse_label_filter(s: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = s.splitn(2, '=').collect();
    if parts.len() != 2 {
        return Err("label must be key=value".to_string());
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

// =============================================================================
// Output types
// =============================================================================

struct MetricListOutput {
    metrics: Vec<MetricMetaRow>,
    count: u32,
}

struct MetricMetaRow {
    name: String,
    metric_type: String,
    unit: String,
    label_keys: Vec<String>,
}

impl Outputable for MetricListOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "metrics": self.metrics.iter().map(|m| serde_json::json!({
                "name": m.name,
                "type": m.metric_type,
                "unit": m.unit,
                "label_keys": m.label_keys,
            })).collect::<Vec<_>>(),
            "count": self.count,
        })
    }

    fn to_human(&self) -> String {
        if self.metrics.is_empty() {
            return "No metrics found".to_string();
        }
        let mut out = format!("Metrics ({}):\n\n", self.count);
        out.push_str("NAME                             | TYPE      | UNIT      | LABELS\n");
        out.push_str("---------------------------------+-----------+-----------+-----------\n");
        for m in &self.metrics {
            let name_display = if m.name.len() > 32 { &m.name[..32] } else { &m.name };
            let labels = if m.label_keys.is_empty() {
                "-".to_string()
            } else {
                m.label_keys.join(", ")
            };
            out.push_str(&format!("{:32} | {:9} | {:9} | {}\n", name_display, m.metric_type, m.unit, labels));
        }
        out
    }
}

struct MetricQueryOutput {
    name: String,
    data_points: Vec<DataPointRow>,
    count: u32,
    is_truncated: bool,
}

struct DataPointRow {
    timestamp_us: u64,
    value: f64,
    labels: Vec<(String, String)>,
}

impl Outputable for MetricQueryOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "data_points": self.data_points.iter().map(|dp| serde_json::json!({
                "timestamp_us": dp.timestamp_us,
                "value": dp.value,
                "labels": dp.labels.iter().map(|(k, v)| serde_json::json!({k: v})).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            "count": self.count,
            "is_truncated": self.is_truncated,
        })
    }

    fn to_human(&self) -> String {
        if self.data_points.is_empty() {
            return format!("No data points for metric '{}'", self.name);
        }
        let mut out =
            format!("Metric: {} ({}{} points)\n\n", self.name, self.count, if self.is_truncated { "+" } else { "" });
        out.push_str("TIMESTAMP (us)       | VALUE          | LABELS\n");
        out.push_str("---------------------+----------------+--------------------\n");
        for dp in &self.data_points {
            let labels = if dp.labels.is_empty() {
                "-".to_string()
            } else {
                dp.labels.iter().map(|(k, v)| format!("{}={}", k, v)).collect::<Vec<_>>().join(", ")
            };
            out.push_str(&format!("{:>20} | {:>14.2} | {}\n", dp.timestamp_us, dp.value, labels));
        }
        out
    }
}

struct MetricIngestOutput {
    accepted: u32,
    dropped: u32,
}

impl Outputable for MetricIngestOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "accepted": self.accepted,
            "dropped": self.dropped,
        })
    }

    fn to_human(&self) -> String {
        format!("Ingested: {} accepted, {} dropped", self.accepted, self.dropped)
    }
}

// =============================================================================
// Command execution
// =============================================================================

impl MetricCommand {
    pub async fn run(&self, client: &AspenClient, is_json: bool) -> Result<()> {
        match self {
            MetricCommand::List(args) => run_metric_list(client, args, is_json).await,
            MetricCommand::Query(args) => run_metric_query(client, args, is_json).await,
            MetricCommand::Ingest(args) => run_metric_ingest(client, args, is_json).await,
        }
    }
}

async fn run_metric_list(client: &AspenClient, args: &MetricListArgs, is_json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::MetricList {
            prefix: args.prefix.clone(),
            limit: Some(args.limit),
        })
        .await?;

    match response {
        ClientRpcResponse::MetricListResult(result) => {
            if let Some(e) = &result.error {
                anyhow::bail!("metric list failed: {}", e);
            }
            let output = MetricListOutput {
                count: result.count,
                metrics: result
                    .metrics
                    .into_iter()
                    .map(|m| MetricMetaRow {
                        name: m.name,
                        metric_type: format!("{:?}", m.metric_type),
                        unit: if m.unit.is_empty() { "-".to_string() } else { m.unit },
                        label_keys: m.label_keys,
                    })
                    .collect(),
            };
            print_output(&output, is_json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("metric list failed: {}", e.message),
        other => anyhow::bail!("unexpected response: {:?}", other),
    }
}

async fn run_metric_query(client: &AspenClient, args: &MetricQueryArgs, is_json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::MetricQuery {
            name: args.name.clone(),
            start_time_us: args.start_us,
            end_time_us: args.end_us,
            label_filters: args.labels.clone(),
            aggregation: Some(args.aggregation.clone()),
            step_us: args.step_us,
            limit: Some(args.limit),
        })
        .await?;

    match response {
        ClientRpcResponse::MetricQueryResult(result) => {
            if let Some(e) = &result.error {
                anyhow::bail!("metric query failed: {}", e);
            }
            let output = MetricQueryOutput {
                name: result.name,
                count: result.count,
                is_truncated: result.is_truncated,
                data_points: result
                    .data_points
                    .into_iter()
                    .map(|dp| DataPointRow {
                        timestamp_us: dp.timestamp_us,
                        value: dp.value,
                        labels: dp.labels,
                    })
                    .collect(),
            };
            print_output(&output, is_json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("metric query failed: {}", e.message),
        other => anyhow::bail!("unexpected response: {:?}", other),
    }
}

async fn run_metric_ingest(client: &AspenClient, args: &MetricIngestArgs, is_json: bool) -> Result<()> {
    // Read JSON array of data points from stdin
    let mut input = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut input)?;
    let data_points: Vec<MetricDataPoint> = serde_json::from_str(&input)
        .map_err(|e| anyhow::anyhow!("invalid JSON input: {}. Expected array of MetricDataPoint", e))?;

    let response = client
        .send(ClientRpcRequest::MetricIngest {
            data_points,
            ttl_seconds: Some(args.ttl_seconds),
        })
        .await?;

    match response {
        ClientRpcResponse::MetricIngestResult(result) => {
            if let Some(e) = &result.error {
                anyhow::bail!("metric ingest failed: {}", e);
            }
            let output = MetricIngestOutput {
                accepted: result.accepted_count,
                dropped: result.dropped_count,
            };
            print_output(&output, is_json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("metric ingest failed: {}", e.message),
        other => anyhow::bail!("unexpected response: {:?}", other),
    }
}
