//! Alert rule management commands.
//!
//! Commands for creating, listing, deleting, and evaluating alert rules.
//! Alert rules define threshold conditions on server-side metrics and
//! track state transitions (Ok → Pending → Firing → Ok).

use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use anyhow::Result;
use aspen_client_api::AlertComparison;
use aspen_client_api::AlertRuleWire;
use aspen_client_api::AlertSeverity;
use aspen_client_api::AlertStatus;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::Outputable;
use crate::output::print_output;

/// Alert rule management.
#[derive(Subcommand)]
pub enum AlertCommand {
    /// Create or update an alert rule.
    Create(AlertCreateArgs),

    /// Delete an alert rule.
    Delete(AlertDeleteArgs),

    /// List all alert rules with current state.
    List,

    /// Get alert rule details with state and history.
    Get(AlertGetArgs),

    /// Evaluate an alert rule against current metrics.
    Evaluate(AlertEvaluateArgs),
}

#[derive(Args)]
pub struct AlertCreateArgs {
    /// Rule name.
    pub name: String,

    /// Metric name to monitor.
    #[arg(long)]
    pub metric: String,

    /// Comparison: gt, gte, lt, lte, eq, ne.
    #[arg(long)]
    pub comparison: String,

    /// Threshold value.
    #[arg(long)]
    pub threshold: f64,

    /// Aggregation: avg, max, min, sum, last, count.
    #[arg(long, default_value = "avg")]
    pub aggregation: String,

    /// Evaluation window in seconds.
    #[arg(long, default_value = "300")]
    pub window_seconds: u64,

    /// Duration condition must hold before firing (seconds, 0 = immediate).
    #[arg(long, default_value = "0")]
    pub for_seconds: u64,

    /// Severity: info, warning, critical.
    #[arg(long, default_value = "warning")]
    pub severity: String,

    /// Description.
    #[arg(long, default_value = "")]
    pub description: String,

    /// Label filter (key=value), repeatable.
    #[arg(long = "label", value_parser = parse_label_filter)]
    pub labels: Vec<(String, String)>,

    /// Disable the rule (default: enabled).
    #[arg(long)]
    pub disabled: bool,
}

#[derive(Args)]
pub struct AlertDeleteArgs {
    /// Rule name.
    pub name: String,
}

#[derive(Args)]
pub struct AlertGetArgs {
    /// Rule name.
    pub name: String,
}

#[derive(Args)]
pub struct AlertEvaluateArgs {
    /// Rule name.
    pub name: String,
}

fn parse_label_filter(s: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = s.splitn(2, '=').collect();
    if parts.len() != 2 {
        return Err("label must be key=value".to_string());
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

fn parse_comparison(s: &str) -> Result<AlertComparison> {
    match s {
        "gt" => Ok(AlertComparison::GreaterThan),
        "gte" => Ok(AlertComparison::GreaterThanOrEqual),
        "lt" => Ok(AlertComparison::LessThan),
        "lte" => Ok(AlertComparison::LessThanOrEqual),
        "eq" => Ok(AlertComparison::Equal),
        "ne" => Ok(AlertComparison::NotEqual),
        _ => anyhow::bail!("invalid comparison: '{}'. Use: gt, gte, lt, lte, eq, ne", s),
    }
}

fn parse_severity(s: &str) -> Result<AlertSeverity> {
    match s {
        "info" => Ok(AlertSeverity::Info),
        "warning" => Ok(AlertSeverity::Warning),
        "critical" => Ok(AlertSeverity::Critical),
        _ => anyhow::bail!("invalid severity: '{}'. Use: info, warning, critical", s),
    }
}

fn format_status(status: &AlertStatus) -> &'static str {
    match status {
        AlertStatus::Ok => "OK",
        AlertStatus::Pending => "PENDING",
        AlertStatus::Firing => "FIRING",
    }
}

fn format_comparison(cmp: &AlertComparison) -> &'static str {
    match cmp {
        AlertComparison::GreaterThan => ">",
        AlertComparison::GreaterThanOrEqual => ">=",
        AlertComparison::LessThan => "<",
        AlertComparison::LessThanOrEqual => "<=",
        AlertComparison::Equal => "==",
        AlertComparison::NotEqual => "!=",
    }
}

fn now_us() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_micros() as u64
}

// =============================================================================
// Output types
// =============================================================================

struct AlertListOutput {
    rules: Vec<AlertListRow>,
    count: u32,
}

struct AlertListRow {
    name: String,
    metric: String,
    status: String,
    severity: String,
    condition: String,
}

impl Outputable for AlertListOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "rules": self.rules.iter().map(|r| serde_json::json!({
                "name": r.name,
                "metric": r.metric,
                "status": r.status,
                "severity": r.severity,
                "condition": r.condition,
            })).collect::<Vec<_>>(),
            "count": self.count,
        })
    }

    fn to_human(&self) -> String {
        if self.rules.is_empty() {
            return "No alert rules defined".to_string();
        }
        let mut out = format!("Alert Rules ({}):\n\n", self.count);
        out.push_str("NAME                 | METRIC               | STATUS  | SEVERITY | CONDITION\n");
        out.push_str("---------------------+----------------------+---------+----------+-------------------\n");
        for r in &self.rules {
            let name = if r.name.len() > 20 { &r.name[..20] } else { &r.name };
            let metric = if r.metric.len() > 20 {
                &r.metric[..20]
            } else {
                &r.metric
            };
            out.push_str(&format!(
                "{:20} | {:20} | {:7} | {:8} | {}\n",
                name, metric, r.status, r.severity, r.condition
            ));
        }
        out
    }
}

struct AlertGetOutput {
    detail: serde_json::Value,
}

impl Outputable for AlertGetOutput {
    fn to_json(&self) -> serde_json::Value {
        self.detail.clone()
    }

    fn to_human(&self) -> String {
        // Pretty-print the JSON for human output (sufficient for v1)
        serde_json::to_string_pretty(&self.detail).unwrap_or_else(|_| "error formatting output".to_string())
    }
}

struct AlertResultOutput {
    message: String,
}

impl Outputable for AlertResultOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({ "message": self.message })
    }

    fn to_human(&self) -> String {
        self.message.clone()
    }
}

struct AlertEvaluateOutput {
    detail: serde_json::Value,
}

impl Outputable for AlertEvaluateOutput {
    fn to_json(&self) -> serde_json::Value {
        self.detail.clone()
    }

    fn to_human(&self) -> String {
        serde_json::to_string_pretty(&self.detail).unwrap_or_else(|_| "error formatting output".to_string())
    }
}

// =============================================================================
// Command execution
// =============================================================================

impl AlertCommand {
    pub async fn run(&self, client: &AspenClient, is_json: bool) -> Result<()> {
        match self {
            AlertCommand::Create(args) => run_alert_create(client, args, is_json).await,
            AlertCommand::Delete(args) => run_alert_delete(client, args, is_json).await,
            AlertCommand::List => run_alert_list(client, is_json).await,
            AlertCommand::Get(args) => run_alert_get(client, args, is_json).await,
            AlertCommand::Evaluate(args) => run_alert_evaluate(client, args, is_json).await,
        }
    }
}

async fn run_alert_create(client: &AspenClient, args: &AlertCreateArgs, is_json: bool) -> Result<()> {
    let comparison = parse_comparison(&args.comparison)?;
    let severity = parse_severity(&args.severity)?;
    let ts = now_us();

    let rule = AlertRuleWire {
        name: args.name.clone(),
        metric_name: args.metric.clone(),
        label_filters: args.labels.clone(),
        aggregation: args.aggregation.clone(),
        window_duration_us: args.window_seconds.saturating_mul(1_000_000),
        comparison,
        threshold: args.threshold,
        for_duration_us: args.for_seconds.saturating_mul(1_000_000),
        severity,
        description: args.description.clone(),
        is_enabled: !args.disabled,
        created_at_us: ts,
        updated_at_us: ts,
    };

    let response = client.send(ClientRpcRequest::AlertCreate { rule }).await?;

    match response {
        ClientRpcResponse::AlertCreateResult(result) => {
            if let Some(e) = &result.error {
                anyhow::bail!("alert create failed: {}", e);
            }
            let output = AlertResultOutput {
                message: format!("Alert rule '{}' created", result.rule_name),
            };
            print_output(&output, is_json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("alert create failed: {}", e.message),
        other => anyhow::bail!("unexpected response: {:?}", other),
    }
}

async fn run_alert_delete(client: &AspenClient, args: &AlertDeleteArgs, is_json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::AlertDelete {
            name: args.name.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::AlertDeleteResult(result) => {
            if let Some(e) = &result.error {
                anyhow::bail!("alert delete failed: {}", e);
            }
            let output = AlertResultOutput {
                message: format!("Alert rule '{}' deleted", result.rule_name),
            };
            print_output(&output, is_json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("alert delete failed: {}", e.message),
        other => anyhow::bail!("unexpected response: {:?}", other),
    }
}

async fn run_alert_list(client: &AspenClient, is_json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::AlertList).await?;

    match response {
        ClientRpcResponse::AlertListResult(result) => {
            if let Some(e) = &result.error {
                anyhow::bail!("alert list failed: {}", e);
            }
            let output = AlertListOutput {
                count: result.count,
                rules: result
                    .rules
                    .into_iter()
                    .map(|rws| {
                        let status =
                            rws.state.as_ref().map(|s| format_status(&s.status)).unwrap_or("UNKNOWN").to_string();
                        let condition = format!(
                            "{} {} {}",
                            rws.rule.aggregation,
                            format_comparison(&rws.rule.comparison),
                            rws.rule.threshold
                        );
                        AlertListRow {
                            name: rws.rule.name,
                            metric: rws.rule.metric_name,
                            status,
                            severity: format!("{:?}", rws.rule.severity),
                            condition,
                        }
                    })
                    .collect(),
            };
            print_output(&output, is_json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("alert list failed: {}", e.message),
        other => anyhow::bail!("unexpected response: {:?}", other),
    }
}

async fn run_alert_get(client: &AspenClient, args: &AlertGetArgs, is_json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::AlertGet {
            name: args.name.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::AlertGetResult(result) => {
            if let Some(e) = &result.error {
                anyhow::bail!("alert get failed: {}", e);
            }
            let output = AlertGetOutput {
                detail: serde_json::json!({
                    "rule": result.rule,
                    "state": result.state,
                    "history": result.history,
                }),
            };
            print_output(&output, is_json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("alert get failed: {}", e.message),
        other => anyhow::bail!("unexpected response: {:?}", other),
    }
}

async fn run_alert_evaluate(client: &AspenClient, args: &AlertEvaluateArgs, is_json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::AlertEvaluate {
            name: args.name.clone(),
            now_us: now_us(),
        })
        .await?;

    match response {
        ClientRpcResponse::AlertEvaluateResult(result) => {
            if let Some(e) = &result.error {
                anyhow::bail!("alert evaluate failed: {}", e);
            }
            let output = AlertEvaluateOutput {
                detail: serde_json::json!({
                    "rule_name": result.rule_name,
                    "status": format_status(&result.status),
                    "computed_value": result.computed_value,
                    "threshold": result.threshold,
                    "did_transition": result.did_transition,
                    "previous_status": result.previous_status.as_ref().map(format_status),
                }),
            };
            print_output(&output, is_json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("alert evaluate failed: {}", e.message),
        other => anyhow::bail!("unexpected response: {:?}", other),
    }
}
