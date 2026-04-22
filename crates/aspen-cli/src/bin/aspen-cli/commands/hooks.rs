//! Hook system commands.
//!
//! Commands for managing and monitoring the event-driven hook system,
//! including listing handlers, viewing metrics, manual triggering, and
//! generating shareable hook trigger URLs.

use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use aspen_client_api::CLIENT_ALPN;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::HookHandlerInfo;
use aspen_client_api::HookHandlerMetrics;
use aspen_client_api::MAX_CLIENT_MESSAGE_SIZE;
use aspen_hooks_ticket::AspenHookTicket;
use clap::Args;
use clap::Subcommand;
use iroh::Endpoint;
use iroh::EndpointAddr;
use iroh::endpoint::VarInt;
use serde_json::json;
use tokio::time::timeout;

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

    /// Create a shareable hook trigger URL.
    ///
    /// Generates an Iroh-based URL that external programs can use to trigger
    /// hooks on this cluster without needing the full cluster ticket.
    CreateUrl(CreateUrlArgs),

    /// Trigger a hook using a hook URL.
    ///
    /// Connect to a cluster using a hook trigger URL and fire the configured event.
    TriggerUrl(TriggerUrlArgs),
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

#[derive(Args)]
pub struct CreateUrlArgs {
    /// Event type to trigger.
    ///
    /// Valid types: write_committed, delete_committed, membership_changed,
    /// leader_elected, snapshot_created, snapshot_installed, health_changed,
    /// node_added, node_removed, ttl_expired
    #[arg(long)]
    pub event_type: String,

    /// Default JSON payload template (optional).
    ///
    /// External programs can override this when triggering.
    #[arg(long)]
    pub payload: Option<String>,

    /// Expiration time in hours (default: 24).
    ///
    /// Set to 0 for no expiration.
    #[arg(long, default_value = "24")]
    pub expires: u64,

    /// Relay URL for NAT traversal (optional).
    #[arg(long)]
    pub relay_url: Option<String>,
}

#[derive(Args)]
pub struct TriggerUrlArgs {
    /// Hook trigger URL (aspenhook...).
    pub url: String,

    /// Override the default payload with custom JSON.
    #[arg(long)]
    pub payload: Option<String>,

    /// RPC timeout in milliseconds.
    #[arg(long = "timeout", default_value = "5000")]
    pub timeout_ms: u64,
}

/// Hook list output.
pub struct HookListOutput {
    pub is_enabled: bool,
    pub handlers: Vec<HookHandlerInfo>,
}

impl Outputable for HookListOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "is_enabled": self.is_enabled,
            "handlers": self.handlers
        })
    }

    fn to_human(&self) -> String {
        let mut output = format!("Hook System: {}\n\n", if self.is_enabled { "enabled" } else { "disabled" });

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
                if h.is_enabled { "yes" } else { "no" },
                h.timeout_ms,
            ));
        }

        output
    }
}

/// Hook metrics output.
pub struct HookMetricsOutput {
    pub is_enabled: bool,
    pub total_events_processed: u64,
    pub handlers: Vec<HookHandlerMetrics>,
}

fn format_avg_latency_us(avg_duration_us: u64) -> String {
    debug_assert!(avg_duration_us <= u64::MAX / 1000);
    if avg_duration_us == 0 {
        return "n/a".to_string();
    }
    if avg_duration_us >= 1000 {
        return format!("{:.1}ms", avg_duration_us as f64 / 1000.0);
    }
    format!("{}us", avg_duration_us)
}

impl Outputable for HookMetricsOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "is_enabled": self.is_enabled,
            "total_events_processed": self.total_events_processed,
            "handlers": self.handlers
        })
    }

    fn to_human(&self) -> String {
        let mut output = format!("Hook System: {}\n", if self.is_enabled { "enabled" } else { "disabled" });
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

            output.push_str(&format!(
                "{:<20}  {:>7}  {:>6}  {:>7}  {:>7}  {:>11}\n",
                name_short,
                m.success_count,
                m.failure_count,
                m.dropped_count,
                m.jobs_submitted,
                format_avg_latency_us(m.avg_duration_us),
            ));
        }

        output
    }
}

/// Hook trigger output.
pub struct HookTriggerOutput {
    pub is_success: bool,
    pub dispatched_count: u32,
    pub error: Option<String>,
    pub handler_failures: Vec<(String, String)>,
}

impl Outputable for HookTriggerOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "is_success": self.is_success,
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

/// Hook URL creation output.
pub struct HookCreateUrlOutput {
    pub url: String,
    pub cluster_id: String,
    pub event_type: String,
    pub expires: String,
    pub peer_count: u32,
}

impl Outputable for HookCreateUrlOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "url": self.url,
            "cluster_id": self.cluster_id,
            "event_type": self.event_type,
            "expires": self.expires,
            "peer_count": self.peer_count
        })
    }

    fn to_human(&self) -> String {
        let mut output = String::new();
        output.push_str("Hook Trigger URL Created\n\n");
        output.push_str(&format!("Cluster:    {}\n", self.cluster_id));
        output.push_str(&format!("Event Type: {}\n", self.event_type));
        output.push_str(&format!("Expires:    {}\n", self.expires));
        output.push_str(&format!("Peers:      {}\n\n", self.peer_count));
        output.push_str("URL:\n");
        output.push_str(&self.url);
        output.push_str("\n\nUsage:\n");
        output.push_str(&format!("  aspen-cli hooks trigger-url \"{}\"\n", self.url));
        output
    }
}

/// Hook URL trigger output.
pub struct HookTriggerUrlOutput {
    pub is_success: bool,
    pub cluster_id: String,
    pub event_type: String,
    pub dispatched_count: u32,
    pub error: Option<String>,
    pub handler_failures: Vec<(String, String)>,
}

impl Outputable for HookTriggerUrlOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "is_success": self.is_success,
            "cluster_id": self.cluster_id,
            "event_type": self.event_type,
            "dispatched_count": self.dispatched_count,
            "error": self.error,
            "handler_failures": self.handler_failures
        })
    }

    fn to_human(&self) -> String {
        if let Some(err) = &self.error {
            return format!("Trigger failed: {}", err);
        }

        let mut output = format!("Hook triggered on cluster '{}'\n", self.cluster_id);
        output.push_str(&format!("Event type: {}\n", self.event_type));
        output.push_str(&format!("Dispatched to {} handler(s)\n", self.dispatched_count));

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
    pub async fn run(self, client: &AspenClient, is_json_output: bool) -> Result<()> {
        match self {
            HookCommand::List => hook_list(client, is_json_output).await,
            HookCommand::Metrics(args) => hook_metrics(client, args, is_json_output).await,
            HookCommand::Trigger(args) => hook_trigger(client, args, is_json_output).await,
            HookCommand::CreateUrl(args) => hook_create_url(client, args, is_json_output).await,
            HookCommand::TriggerUrl(args) => hook_trigger_url(args, is_json_output).await,
        }
    }
}

async fn hook_list(client: &AspenClient, is_json_output: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::HookList).await?;

    match response {
        ClientRpcResponse::HookListResult(result) => {
            let handler_count = result.handlers.len();
            let output = HookListOutput {
                is_enabled: result.is_enabled,
                handlers: result.handlers,
            };
            debug_assert_eq!(output.handlers.len(), handler_count);
            debug_assert_eq!(output.is_enabled, result.is_enabled);
            print_output(&output, is_json_output);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn hook_metrics(client: &AspenClient, args: MetricsArgs, is_json_output: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::HookGetMetrics {
            handler_name: args.handler,
        })
        .await?;

    match response {
        ClientRpcResponse::HookMetricsResult(result) => {
            let handler_count = result.handlers.len();
            let output = HookMetricsOutput {
                is_enabled: result.is_enabled,
                total_events_processed: result.total_events_processed,
                handlers: result.handlers,
            };
            debug_assert_eq!(output.handlers.len(), handler_count);
            debug_assert_eq!(output.total_events_processed, result.total_events_processed);
            print_output(&output, is_json_output);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn hook_trigger(client: &AspenClient, args: TriggerArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.event_type.is_empty());
    debug_assert!(!args.payload.is_empty());
    // Validate payload is valid JSON (but send as string for PostCard compatibility)
    let _: serde_json::Value =
        serde_json::from_str(&args.payload).map_err(|e| anyhow::anyhow!("Invalid payload JSON: {}", e))?;

    let response = client
        .send(ClientRpcRequest::HookTrigger {
            event_type: args.event_type,
            payload_json: args.payload,
        })
        .await?;

    match response {
        ClientRpcResponse::HookTriggerResult(result) => {
            let output = HookTriggerOutput {
                is_success: result.is_success,
                dispatched_count: result.dispatched_count,
                error: result.error,
                handler_failures: result.handler_failures,
            };
            debug_assert!(output.is_success || output.error.is_some() || !output.handler_failures.is_empty());
            debug_assert_eq!(output.dispatched_count, result.dispatched_count);
            print_output(&output, is_json_output);
            if !result.is_success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn hook_create_url(client: &AspenClient, args: CreateUrlArgs, is_json_output: bool) -> Result<()> {
    // Validate inputs
    hook_create_url_validate_inputs(&args)?;

    // Get cluster ticket and extract bootstrap peers
    let (parsed_ticket, bootstrap_peers) = hook_create_url_get_cluster_info(client).await?;

    // Build the hook ticket with optional fields
    let hook_ticket = hook_create_url_build_ticket(&parsed_ticket.cluster_id, bootstrap_peers.clone(), &args)?;

    let url = hook_ticket.serialize();

    let output = HookCreateUrlOutput {
        url,
        cluster_id: parsed_ticket.cluster_id,
        event_type: args.event_type,
        expires: hook_ticket.expiry_string(),
        peer_count: bootstrap_peers.len() as u32,
    };

    debug_assert!(!output.url.is_empty());
    debug_assert_eq!(usize::try_from(output.peer_count).unwrap_or(0), bootstrap_peers.len());
    print_output(&output, is_json_output);
    Ok(())
}

/// Validate the event type and optional payload JSON for hook URL creation.
fn hook_create_url_validate_inputs(args: &CreateUrlArgs) -> Result<()> {
    debug_assert!(!args.event_type.is_empty());
    debug_assert!(args.payload.as_deref().is_none_or(|payload| !payload.is_empty()));
    let valid_types = [
        "write_committed",
        "delete_committed",
        "membership_changed",
        "leader_elected",
        "snapshot_created",
        "snapshot_installed",
        "health_changed",
        "node_added",
        "node_removed",
        "ttl_expired",
    ];

    if !valid_types.contains(&args.event_type.as_str()) {
        anyhow::bail!("Invalid event type: '{}'. Valid types: {}", args.event_type, valid_types.join(", "));
    }

    if let Some(ref payload) = args.payload {
        let _: serde_json::Value =
            serde_json::from_str(payload).map_err(|e| anyhow::anyhow!("Invalid payload JSON: {}", e))?;
    }

    debug_assert!(args.expires == 0 || args.expires <= u64::from(u32::MAX));
    debug_assert!(args.relay_url.as_deref().is_none_or(|relay_url| !relay_url.is_empty()));
    Ok(())
}

/// Fetch the cluster ticket and extract bootstrap peers.
async fn hook_create_url_get_cluster_info(
    client: &AspenClient,
) -> Result<(aspen_cluster::ticket::AspenClusterTicket, Vec<EndpointAddr>)> {
    let response = client.send(ClientRpcRequest::GetClusterTicket).await?;

    let ticket_response = match response {
        ClientRpcResponse::ClusterTicket(ticket) => ticket,
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    };

    let parsed_ticket = aspen_cluster::ticket::AspenClusterTicket::deserialize(&ticket_response.ticket)
        .context("failed to parse cluster ticket")?;

    let bootstrap_peers: Vec<EndpointAddr> = parsed_ticket.bootstrap.iter().map(|p| p.to_endpoint_addr()).collect();

    if bootstrap_peers.is_empty() {
        anyhow::bail!("no bootstrap peers available in cluster ticket");
    }

    Ok((parsed_ticket, bootstrap_peers))
}

/// Build an AspenHookTicket with the given parameters.
fn hook_create_url_build_ticket(
    cluster_id: &str,
    bootstrap_peers: Vec<EndpointAddr>,
    args: &CreateUrlArgs,
) -> Result<AspenHookTicket> {
    let mut hook_ticket = AspenHookTicket::new(cluster_id, bootstrap_peers).with_event_type(&args.event_type);

    if let Some(ref payload) = args.payload {
        hook_ticket = hook_ticket.with_default_payload(payload.clone());
    }

    if args.expires > 0 {
        hook_ticket = hook_ticket.with_expiry_hours(args.expires);
    }

    if let Some(ref relay_url) = args.relay_url {
        hook_ticket = hook_ticket.with_relay_url(relay_url.clone());
    }

    hook_ticket.validate()?;

    Ok(hook_ticket)
}

async fn hook_trigger_url(args: TriggerUrlArgs, is_json_output: bool) -> Result<()> {
    // Parse the hook ticket
    let ticket = AspenHookTicket::deserialize(&args.url).context("failed to parse hook trigger URL")?;

    // Determine the payload to use
    let payload = args.payload.unwrap_or_else(|| ticket.default_payload.clone().unwrap_or_else(|| "{}".to_string()));

    // Validate the payload JSON
    let _: serde_json::Value =
        serde_json::from_str(&payload).map_err(|e| anyhow::anyhow!("Invalid payload JSON: {}", e))?;

    // Create an Iroh endpoint for connecting
    let secret_key = iroh::SecretKey::generate(&mut rand::rng());
    let endpoint = Endpoint::builder(iroh::endpoint::presets::N0)
        .secret_key(secret_key)
        .alpns(vec![CLIENT_ALPN.to_vec()])
        .bind()
        .await
        .context("failed to create Iroh endpoint")?;

    let rpc_budget = Duration::from_millis(args.timeout_ms);
    debug_assert!(!ticket.bootstrap_peers.is_empty());
    debug_assert!(!payload.is_empty());

    // Try each bootstrap peer
    let mut last_error = None;
    for peer_addr in &ticket.bootstrap_peers {
        let request = RemoteHookTriggerRequest {
            event_type: &ticket.event_type,
            payload_json: &payload,
        };
        match send_hook_trigger(&endpoint, peer_addr, &request, rpc_budget).await {
            Ok(result) => {
                let output = HookTriggerUrlOutput {
                    is_success: result.is_success,
                    cluster_id: ticket.cluster_id.clone(),
                    event_type: ticket.event_type.clone(),
                    dispatched_count: result.dispatched_count,
                    error: result.error,
                    handler_failures: result.handler_failures,
                };
                debug_assert!(output.is_success || output.error.is_some() || !output.handler_failures.is_empty());
                print_output(&output, is_json_output);
                if !result.is_success {
                    std::process::exit(1);
                }
                return Ok(());
            }
            Err(e) => {
                tracing::debug!(peer = ?peer_addr, error = %e, "failed to connect to peer");
                last_error = Some(e);
            }
        }
    }

    // All peers failed
    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("no bootstrap peers available")))
}

struct RemoteHookTriggerRequest<'a> {
    event_type: &'a str,
    payload_json: &'a str,
}

#[allow(unknown_lints)]
#[allow(
    ambient_clock,
    reason = "hook URL trigger tracks a wall-clock deadline across connection and stream phases"
)]
fn hook_deadline_now() -> std::time::Instant {
    std::time::Instant::now()
}

/// Send a hook trigger request to a specific peer.
async fn send_hook_trigger(
    endpoint: &Endpoint,
    peer_addr: &EndpointAddr,
    request: &RemoteHookTriggerRequest<'_>,
    rpc_timeout: Duration,
) -> Result<HookTriggerResult> {
    debug_assert!(!request.event_type.is_empty());
    debug_assert!(!request.payload_json.is_empty());
    // Connect to the peer
    let connection = timeout(rpc_timeout, async {
        endpoint.connect(peer_addr.clone(), CLIENT_ALPN).await.context("failed to connect to peer")
    })
    .await
    .context("connection timeout")??;

    // Build and serialize the request before entering the timed exchange
    let rpc_request = ClientRpcRequest::HookTrigger {
        event_type: request.event_type.to_string(),
        payload_json: request.payload_json.to_string(),
    };
    let request_bytes = postcard::to_stdvec(&rpc_request).context("failed to serialize request")?;

    // Bound the full post-connect exchange with one overall deadline while
    // preserving stage-specific timeout errors.
    let deadline = hook_deadline_now() + rpc_timeout;

    // Open bidirectional stream
    let (mut send, mut recv) = timeout(remaining_timeout_hooks(deadline)?, connection.open_bi())
        .await
        .context("stream open timeout")?
        .context("failed to open stream")?;

    // Send request
    timeout(remaining_timeout_hooks(deadline)?, send.write_all(&request_bytes))
        .await
        .context("request write timeout")?
        .context("failed to send request")?;
    timeout(remaining_timeout_hooks(deadline)?, async { send.finish().context("failed to finish send stream") })
        .await
        .context("stream finish timeout")??;

    // Read response with deadline
    let response_bytes = timeout(remaining_timeout_hooks(deadline)?, recv.read_to_end(MAX_CLIENT_MESSAGE_SIZE))
        .await
        .context("response timeout")?
        .context("failed to read response")?;

    // Deserialize response
    let response: ClientRpcResponse =
        postcard::from_bytes(&response_bytes).context("failed to deserialize response")?;

    // Close connection gracefully
    connection.close(VarInt::from_u32(0), b"done");

    // Handle response
    match response {
        ClientRpcResponse::HookTriggerResult(result) => Ok(HookTriggerResult {
            is_success: result.is_success,
            dispatched_count: result.dispatched_count,
            error: result.error,
            handler_failures: result.handler_failures,
        }),
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

/// Compute remaining time until a deadline, returning an error if already past.
fn remaining_timeout_hooks(deadline: std::time::Instant) -> anyhow::Result<std::time::Duration> {
    match deadline.checked_duration_since(hook_deadline_now()) {
        Some(remaining) if !remaining.is_zero() => Ok(remaining),
        Some(_) => Err(anyhow::anyhow!("deadline exceeded")),
        None => Err(anyhow::anyhow!("deadline exceeded")),
    }
}

/// Result from a hook trigger operation.
struct HookTriggerResult {
    is_success: bool,
    dispatched_count: u32,
    error: Option<String>,
    handler_failures: Vec<(String, String)>,
}
