//! Hook system commands.
//!
//! Commands for managing and monitoring the event-driven hook system,
//! including listing handlers, viewing metrics, and manual triggering.

use anyhow::Result;
use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;
use aspen_client_rpc::HookHandlerInfo;
use aspen_client_rpc::HookHandlerMetrics;
use clap::Args;
use clap::Subcommand;
use serde_json::json;

use crate::client::AspenClient;
use crate::output::Outputable;
use crate::output::print_output;

/// Hook system operations.
#[derive(Subcommand)]
pub enum HookCommand {
    /// List configured hook handlers.
    List,

    /// Get hook execution metrics.
    Metrics(MetricsArgs),

    /// Manually trigger a hook event for testing.
    Trigger(TriggerArgs),
}

#[derive(Args)]
pub struct MetricsArgs {
    /// Filter by handler name (optional).
    #[arg(long)]
    pub handler: Option<String>,
}

#[derive(Args)]
pub struct TriggerArgs {
    /// Event type to trigger.
    ///
    /// Valid types: write_committed, delete_committed, membership_changed,
    /// leader_elected, snapshot_created
    pub event_type: String,

    /// JSON payload for the event (optional, defaults to {}).
    #[arg(long, default_value = "{}")]
    pub payload: String,
}

/// Hook list output.
pub struct HookListOutput {
    pub enabled: bool,
    pub handlers: Vec<HookHandlerInfo>,
}

impl Outputable for HookListOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "enabled": self.enabled,
            "handlers": self.handlers
        })
    }

    fn to_human(&self) -> String {
        let mut output = format!("Hook System: {}\n\n", if self.enabled { "enabled" } else { "disabled" });

        if self.handlers.is_empty() {
            output.push_str("No handlers configured");
            return output;
        }

        output.push_str(&format!("Handlers ({}):\n\n", self.handlers.len()));

        // Table header
        output.push_str("NAME                  PATTERN                           TYPE      MODE    ENABLED  TIMEOUT\n");
        output.push_str("────────────────────  ────────────────────────────────  ────────  ──────  ───────  ───────\n");

        for h in &self.handlers {
            let name_short = if h.name.len() > 20 {
                format!("{}...", &h.name[..17])
            } else {
                h.name.clone()
            };

            let pattern_short = if h.pattern.len() > 32 {
                format!("{}...", &h.pattern[..29])
            } else {
                h.pattern.clone()
            };

            let type_short = if h.handler_type.len() > 8 {
                format!("{}...", &h.handler_type[..5])
            } else {
                h.handler_type.clone()
            };

            output.push_str(&format!(
                "{:<20}  {:<32}  {:<8}  {:<6}  {:<7}  {:>5}ms\n",
                name_short,
                pattern_short,
                type_short,
                h.execution_mode,
                if h.enabled { "yes" } else { "no" },
                h.timeout_ms,
            ));
        }

        output
    }
}

/// Hook metrics output.
pub struct HookMetricsOutput {
    pub enabled: bool,
    pub total_events_processed: u64,
    pub handlers: Vec<HookHandlerMetrics>,
}

impl Outputable for HookMetricsOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "enabled": self.enabled,
            "total_events_processed": self.total_events_processed,
            "handlers": self.handlers
        })
    }

    fn to_human(&self) -> String {
        let mut output = format!("Hook System: {}\n", if self.enabled { "enabled" } else { "disabled" });
        output.push_str(&format!("Total Events Processed: {}\n\n", self.total_events_processed));

        if self.handlers.is_empty() {
            output.push_str("No handler metrics available");
            return output;
        }

        output.push_str(&format!("Handler Metrics ({}):\n\n", self.handlers.len()));

        // Table header
        output.push_str("NAME                  SUCCESS  FAILED  DROPPED  JOBS     AVG LATENCY\n");
        output.push_str("────────────────────  ───────  ──────  ───────  ───────  ───────────\n");

        for m in &self.handlers {
            let name_short = if m.name.len() > 20 {
                format!("{}...", &m.name[..17])
            } else {
                m.name.clone()
            };

            let latency_str = if m.avg_duration_us > 0 {
                if m.avg_duration_us >= 1000 {
                    format!("{:.1}ms", m.avg_duration_us as f64 / 1000.0)
                } else {
                    format!("{}us", m.avg_duration_us)
                }
            } else {
                "n/a".to_string()
            };

            output.push_str(&format!(
                "{:<20}  {:>7}  {:>6}  {:>7}  {:>7}  {:>11}\n",
                name_short, m.success_count, m.failure_count, m.dropped_count, m.jobs_submitted, latency_str,
            ));
        }

        output
    }
}

/// Hook trigger output.
pub struct HookTriggerOutput {
    pub success: bool,
    pub dispatched_count: usize,
    pub error: Option<String>,
    pub handler_failures: Vec<(String, String)>,
}

impl Outputable for HookTriggerOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "success": self.success,
            "dispatched_count": self.dispatched_count,
            "error": self.error,
            "handler_failures": self.handler_failures
        })
    }

    fn to_human(&self) -> String {
        if let Some(err) = &self.error {
            return format!("Trigger failed: {}", err);
        }

        let mut output = format!("Event dispatched to {} handler(s)\n", self.dispatched_count);

        if self.handler_failures.is_empty() {
            output.push_str("All handlers executed successfully");
        } else {
            output.push_str(&format!("\nHandler failures ({}):\n", self.handler_failures.len()));
            for (name, error) in &self.handler_failures {
                output.push_str(&format!("  - {}: {}\n", name, error));
            }
        }

        output
    }
}

impl HookCommand {
    /// Execute the hook command.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            HookCommand::List => hook_list(client, json).await,
            HookCommand::Metrics(args) => hook_metrics(client, args, json).await,
            HookCommand::Trigger(args) => hook_trigger(client, args, json).await,
        }
    }
}

async fn hook_list(client: &AspenClient, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::HookList).await?;

    match response {
        ClientRpcResponse::HookListResult(result) => {
            let output = HookListOutput {
                enabled: result.enabled,
                handlers: result.handlers,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn hook_metrics(client: &AspenClient, args: MetricsArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::HookGetMetrics {
            handler_name: args.handler,
        })
        .await?;

    match response {
        ClientRpcResponse::HookMetricsResult(result) => {
            let output = HookMetricsOutput {
                enabled: result.enabled,
                total_events_processed: result.total_events_processed,
                handlers: result.handlers,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn hook_trigger(client: &AspenClient, args: TriggerArgs, json: bool) -> Result<()> {
    // Validate payload is valid JSON
    let payload: serde_json::Value =
        serde_json::from_str(&args.payload).map_err(|e| anyhow::anyhow!("Invalid payload JSON: {}", e))?;

    let response = client
        .send(ClientRpcRequest::HookTrigger {
            event_type: args.event_type,
            payload,
        })
        .await?;

    match response {
        ClientRpcResponse::HookTriggerResult(result) => {
            let output = HookTriggerOutput {
                success: result.success,
                dispatched_count: result.dispatched_count,
                error: result.error,
                handler_failures: result.handler_failures,
            };
            print_output(&output, json);
            if !result.success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}
