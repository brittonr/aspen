//! Cluster management commands.
//!
//! Commands for cluster initialization, status, health checks,
//! membership management, deployment, and maintenance operations.

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::ClusterStateOutput;
use crate::output::DeployInitOutput;
use crate::output::DeployNodeStatusEntry;
use crate::output::DeployStatusOutput;
use crate::output::DeployWaitFinalOutput;
use crate::output::DeployWaitOutput;
use crate::output::HealthOutput;
use crate::output::NodeInfo;
use crate::output::RaftMetricsOutput;
use crate::output::RollbackOutput;
use crate::output::UpdatePeerOutput;
use crate::output::print_output;
use crate::output::print_success;

/// Cluster management commands.
#[derive(Subcommand)]
pub enum ClusterCommand {
    /// Initialize a new cluster.
    Init(InitArgs),

    /// Show cluster status and node information.
    Status,

    /// Check node health.
    Health,

    /// Show detailed Raft metrics.
    Metrics,

    /// Get Prometheus-format metrics.
    Prometheus,

    /// Add a learner node to the cluster.
    AddLearner(AddLearnerArgs),

    /// Promote a learner to voter.
    Promote(PromoteArgs),

    /// Change cluster membership.
    ChangeMembership(ChangeMembershipArgs),

    /// Trigger a Raft snapshot.
    Snapshot,

    /// Checkpoint SQLite WAL.
    CheckpointWal,

    /// Get cluster connection ticket.
    Ticket,

    /// Start a rolling deployment of a new binary.
    Deploy(DeployArgs),

    /// Show deployment status with per-node breakdown.
    DeployStatus,

    /// Roll back the current or last deployment.
    Rollback,

    /// Update a peer's address in the local network factory (no Raft consensus).
    UpdatePeer(UpdatePeerArgs),

    /// Show network metrics (connection pool, snapshot transfers).
    Network,

    /// Permanently expunge a node from the cluster.
    ///
    /// Removes from Raft membership, triggers trust reconfiguration, and sends
    /// expungement notification. The target node will need a factory reset to rejoin.
    Expunge(ExpungeArgs),
}

#[derive(Args)]
pub struct UpdatePeerArgs {
    /// Node ID of the peer to update.
    #[arg(long)]
    pub node_id: u64,

    /// JSON endpoint address: {"id":"<hex>","addrs":[{"Ip":"host:port"}]}.
    #[arg(long)]
    pub addr: String,
}

#[derive(Args)]
pub struct ExpungeArgs {
    /// Node ID of the node to expunge.
    pub node_id: u64,

    /// Required: confirm that you understand the node will need a factory reset.
    #[arg(long)]
    pub confirm: bool,
}

#[derive(Args)]
pub struct AddLearnerArgs {
    /// Node ID of the learner to add.
    #[arg(long)]
    pub node_id: u64,

    /// JSON endpoint address: {"id":"<hex>","addrs":[{"Ip":"host:port"}]}.
    #[arg(long)]
    pub addr: String,
}

#[derive(Args)]
pub struct PromoteArgs {
    /// Node ID of the learner to promote.
    #[arg(long)]
    pub learner_id: u64,

    /// Optional voter to replace.
    #[arg(long)]
    pub replace: Option<u64>,

    /// Skip safety checks.
    #[arg(long = "force")]
    pub is_force: bool,
}

#[derive(Args)]
pub struct ChangeMembershipArgs {
    /// New set of voter node IDs.
    #[arg(required = true)]
    pub members: Vec<u64>,
}

#[derive(Args)]
pub struct DeployArgs {
    /// Artifact to deploy: a Nix store path or blob hash.
    pub artifact: String,

    /// Deployment strategy.
    #[arg(long, default_value = "rolling")]
    pub strategy: String,

    /// Maximum nodes to upgrade concurrently.
    #[arg(long, default_value_t = 1)]
    pub max_concurrent: u32,

    /// Seconds to wait for a node to become healthy after upgrade.
    #[arg(long = "health-timeout", default_value_t = 120)]
    pub health_timeout_secs: u64,

    /// Block until the deployment reaches a terminal state (completed/failed/rolled_back).
    #[arg(long)]
    pub wait: bool,

    /// Maximum seconds to wait for deployment completion (requires --wait).
    #[arg(long = "deploy-timeout", default_value_t = 3600)]
    pub deploy_timeout_secs: u64,
}

impl ClusterCommand {
    /// Execute the cluster command.
    pub async fn run(self, client: &AspenClient, is_json_output: bool) -> Result<()> {
        match self {
            ClusterCommand::Init(args) => init_cluster(client, is_json_output, &args).await,
            ClusterCommand::Status => cluster_status(client, is_json_output).await,
            ClusterCommand::Health => health_check(client, is_json_output).await,
            ClusterCommand::Metrics => raft_metrics(client, is_json_output).await,
            ClusterCommand::Prometheus => prometheus_metrics(client, is_json_output).await,
            ClusterCommand::AddLearner(args) => add_learner(client, args, is_json_output).await,
            ClusterCommand::Promote(args) => promote_learner(client, args, is_json_output).await,
            ClusterCommand::ChangeMembership(args) => change_membership(client, args, is_json_output).await,
            ClusterCommand::Snapshot => trigger_snapshot(client, is_json_output).await,
            ClusterCommand::CheckpointWal => checkpoint_wal(client, is_json_output).await,
            ClusterCommand::Ticket => get_ticket(client, is_json_output).await,
            ClusterCommand::Deploy(args) => deploy(client, args, is_json_output).await,
            ClusterCommand::DeployStatus => deploy_status(client, is_json_output).await,
            ClusterCommand::Rollback => rollback(client, is_json_output).await,
            ClusterCommand::UpdatePeer(args) => update_peer(client, args, is_json_output).await,
            ClusterCommand::Network => network_metrics(client, is_json_output).await,
            ClusterCommand::Expunge(args) => expunge_node(client, args, is_json_output).await,
        }
    }
}

/// Arguments for `cluster init`.
#[derive(Args, Debug, Default)]
pub struct InitArgs {
    /// Enable trust (Shamir cluster secret sharing).
    #[arg(long)]
    pub trust: bool,

    /// Trust reconstruction threshold (default: majority).
    /// Only used when --trust is set.
    #[arg(long)]
    pub trust_threshold: Option<u8>,
}

async fn init_cluster(client: &AspenClient, is_json_output: bool, args: &InitArgs) -> Result<()> {
    debug_assert!(args.trust || args.trust_threshold.is_none());
    debug_assert!(args.trust_threshold.is_none_or(|threshold| threshold > 0));
    let request = if args.trust {
        ClientRpcRequest::InitClusterWithTrust {
            threshold: args.trust_threshold,
        }
    } else {
        ClientRpcRequest::InitCluster
    };
    let response = client.send(request).await?;

    match response {
        ClientRpcResponse::InitResult(result) => {
            if is_json_output {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "success",
                        "message": "Cluster initialized",
                        "result": format!("{:?}", result)
                    })
                );
            } else {
                println!("Cluster initialized successfully");
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn cluster_status(client: &AspenClient, is_json_output: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::GetClusterState).await?;

    match response {
        ClientRpcResponse::ClusterState(state) => {
            let output = ClusterStateOutput {
                nodes: state
                    .nodes
                    .iter()
                    .map(|n| NodeInfo {
                        node_id: n.node_id,
                        endpoint_id: n.endpoint_addr.clone(),
                        is_leader: n.is_leader,
                        is_voter: n.is_voter,
                    })
                    .collect(),
            };
            debug_assert_eq!(output.nodes.len(), state.nodes.len());
            debug_assert!(output.nodes.iter().all(|node| node.node_id > 0));
            print_output(&output, is_json_output);
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn health_check(client: &AspenClient, is_json_output: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::GetHealth).await?;

    match response {
        ClientRpcResponse::Health(health) => {
            let output = HealthOutput {
                status: health.status,
                node_id: health.node_id,
                raft_node_id: health.raft_node_id,
                uptime_seconds: health.uptime_seconds,
                iroh_node_id: health.iroh_node_id,
            };
            debug_assert_eq!(output.node_id, health.node_id);
            debug_assert_eq!(output.raft_node_id, health.raft_node_id);
            print_output(&output, is_json_output);
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn raft_metrics(client: &AspenClient, is_json_output: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::GetRaftMetrics).await?;

    match response {
        ClientRpcResponse::RaftMetrics(metrics) => {
            let output = RaftMetricsOutput {
                state: metrics.state,
                current_leader: metrics.current_leader,
                current_term: metrics.current_term,
                last_log_index: metrics.last_log_index.unwrap_or(0),
                last_applied: metrics.last_applied_index.unwrap_or(0),
                snapshot_index: metrics.snapshot_index.unwrap_or(0),
            };
            debug_assert_eq!(output.last_log_index, metrics.last_log_index.unwrap_or(0));
            debug_assert_eq!(output.last_applied, metrics.last_applied_index.unwrap_or(0));
            print_output(&output, is_json_output);
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn prometheus_metrics(client: &AspenClient, is_json_output: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::GetMetrics).await?;

    match response {
        ClientRpcResponse::Metrics(metrics) => {
            debug_assert!(!metrics.prometheus_text.trim().is_empty());
            debug_assert!(metrics.prometheus_text.contains('\n') || metrics.prometheus_text.contains(' '));
            if is_json_output {
                println!(
                    "{}",
                    serde_json::json!({
                        "format": "prometheus",
                        "metrics": metrics.prometheus_text
                    })
                );
            } else {
                // Prometheus format is already human-readable
                println!("{}", metrics.prometheus_text);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn network_metrics(client: &AspenClient, is_json_output: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::GetNetworkMetrics).await?;

    match response {
        ClientRpcResponse::NetworkMetrics(m) => {
            let unhealthy_connections = m.degraded_connections.saturating_add(m.failed_connections);
            debug_assert!(m.healthy_connections <= m.total_connections);
            debug_assert!(unhealthy_connections <= m.total_connections);
            if is_json_output {
                println!("{}", serde_json::to_string_pretty(&m)?);
            } else {
                println!("Connection Pool");
                println!("  total:    {}", m.total_connections);
                println!("  healthy:  {}", m.healthy_connections);
                println!("  degraded: {}", m.degraded_connections);
                println!("  failed:   {}", m.failed_connections);
                println!("Streams");
                println!("  active:       {}", m.total_active_streams);
                println!("  raft opened:  {}", m.raft_streams_opened);
                println!("  bulk opened:  {}", m.bulk_streams_opened);
                println!("ReadIndex Retries");
                println!("  attempts:  {}", m.read_index_retry_count);
                println!("  successes: {}", m.read_index_retry_success_count);
                if !m.recent_snapshots.is_empty() {
                    println!("Recent Snapshots ({})", m.recent_snapshots.len());
                    for s in &m.recent_snapshots {
                        println!(
                            "  peer={} dir={} size={} dur={}ms outcome={}",
                            s.peer_id, s.direction, s.size_bytes, s.duration_ms, s.outcome
                        );
                    }
                }
                if let Some(err) = &m.error {
                    println!("Note: {}", err);
                }
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn expunge_node(client: &AspenClient, args: ExpungeArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(args.node_id > 0);
    if !args.confirm {
        anyhow::bail!(
            "This will permanently remove node {} from the cluster. \
             The node will need a factory reset to rejoin. \
             Pass --confirm to proceed.",
            args.node_id
        );
    }

    eprintln!(
        "WARNING: Permanently expunging node {} from the cluster. \
         The node will need a factory reset to rejoin.",
        args.node_id
    );

    let response = client.send(ClientRpcRequest::ExpungeNode { node_id: args.node_id }).await?;

    match response {
        ClientRpcResponse::ExpungeNodeResult(result) => {
            debug_assert_eq!(result.node_id, args.node_id);
            debug_assert!(result.is_success || result.error.is_some());
            if is_json_output {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else if result.is_success {
                println!("Node {} has been permanently expunged from the cluster.", result.node_id);
            } else {
                anyhow::bail!("Failed to expunge node {}: {}", result.node_id, result.error.unwrap_or_default());
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn add_learner(client: &AspenClient, args: AddLearnerArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(args.node_id > 0);
    debug_assert!(!args.addr.is_empty());
    let response = client
        .send(ClientRpcRequest::AddLearner {
            node_id: args.node_id,
            addr: args.addr.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::AddLearnerResult(result) => {
            if !result.is_success {
                anyhow::bail!("add-learner failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()));
            }
            print_success(&format!("Learner {} added at {}", args.node_id, args.addr), is_json_output);
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn update_peer(client: &AspenClient, args: UpdatePeerArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(args.node_id > 0);
    debug_assert!(!args.addr.is_empty());
    let response = client
        .send(ClientRpcRequest::AddPeer {
            node_id: args.node_id,
            endpoint_addr: args.addr.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::AddPeerResult(result) => {
            let output = UpdatePeerOutput {
                is_success: result.is_success,
                error: result.error,
            };
            print_output(&output, is_json_output);
            if !output.is_success {
                anyhow::bail!("update-peer failed: {}", output.error.as_deref().unwrap_or("unknown error"));
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn promote_learner(client: &AspenClient, args: PromoteArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(args.learner_id > 0);
    debug_assert!(args.replace.is_none_or(|node_id| node_id > 0));
    let response = client
        .send(ClientRpcRequest::PromoteLearner {
            learner_id: args.learner_id,
            replace_node: args.replace,
            is_force: args.is_force,
        })
        .await?;

    match response {
        ClientRpcResponse::PromoteLearnerResult(result) => {
            if is_json_output {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "success",
                        "learner_id": args.learner_id,
                        "promoted": result.is_success,
                        "message": result.message
                    })
                );
            } else if result.is_success {
                println!("Learner {} promoted to voter", args.learner_id);
            } else {
                anyhow::bail!("promote failed: {}", result.message);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn change_membership(client: &AspenClient, args: ChangeMembershipArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.members.is_empty());
    debug_assert!(args.members.iter().all(|member| *member > 0));
    let response = client
        .send(ClientRpcRequest::ChangeMembership {
            members: args.members.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::ChangeMembershipResult(result) => {
            if !result.is_success {
                anyhow::bail!(
                    "change-membership failed: {}",
                    result.error.unwrap_or_else(|| "unknown error".to_string())
                );
            }
            print_success(&format!("Membership changed to voters: {:?}", args.members), is_json_output);
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn trigger_snapshot(client: &AspenClient, is_json_output: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::TriggerSnapshot).await?;

    match response {
        ClientRpcResponse::SnapshotResult(_) => {
            print_success("Snapshot triggered successfully", is_json_output);
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn checkpoint_wal(client: &AspenClient, is_json_output: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::CheckpointWal).await?;

    match response {
        ClientRpcResponse::CheckpointWalResult(result) => {
            debug_assert!(result.is_success || result.error.is_some());
            debug_assert!(result.pages_checkpointed.is_some() || !result.is_success);
            if is_json_output {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": if result.is_success { "success" } else { "failed" },
                        "pages_checkpointed": result.pages_checkpointed,
                        "wal_size_before_bytes": result.wal_size_before_bytes,
                        "wal_size_after_bytes": result.wal_size_after_bytes
                    })
                );
            } else if result.is_success {
                let pages = result.pages_checkpointed.unwrap_or(0);
                println!("WAL checkpoint complete: {} pages checkpointed", pages);
            } else {
                println!("WAL checkpoint failed: {}", result.error.unwrap_or_else(|| "unknown error".to_string()));
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn get_ticket(client: &AspenClient, is_json_output: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::GetClusterTicket).await?;

    match response {
        ClientRpcResponse::ClusterTicket(ticket_response) => {
            debug_assert!(!ticket_response.ticket.is_empty());
            debug_assert_eq!(ticket_response.ticket, ticket_response.ticket.trim());
            if is_json_output {
                println!(
                    "{}",
                    serde_json::json!({
                        "ticket": ticket_response.ticket
                    })
                );
            } else {
                println!("{}", ticket_response.ticket);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn deploy(client: &AspenClient, args: DeployArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.artifact.is_empty());
    debug_assert!(args.max_concurrent > 0);
    let should_wait = args.wait;
    let timeout_secs = args.deploy_timeout_secs;

    let response = client
        .send(ClientRpcRequest::ClusterDeploy {
            artifact: args.artifact,
            strategy: args.strategy,
            max_concurrent: args.max_concurrent,
            health_timeout_secs: args.health_timeout_secs,
            expected_binary: None,
        })
        .await?;

    match response {
        ClientRpcResponse::ClusterDeployResult(result) => {
            let output = DeployInitOutput {
                is_accepted: result.is_accepted,
                deploy_id: result.deploy_id.clone(),
                error: result.error.clone(),
            };

            if !result.is_accepted {
                print_output(&output, is_json_output);
                anyhow::bail!("deployment rejected: {}", result.error.as_deref().unwrap_or("unknown error"));
            }

            print_output(&output, is_json_output);

            if should_wait {
                deploy_wait(client, result.deploy_id.as_deref(), timeout_secs, is_json_output).await
            } else {
                Ok(())
            }
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn deploy_status(client: &AspenClient, is_json_output: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::ClusterDeployStatus).await?;

    match response {
        ClientRpcResponse::ClusterDeployStatusResult(result) => {
            let output = DeployStatusOutput {
                is_found: result.is_found,
                deploy_id: result.deploy_id,
                status: result.status,
                artifact: result.artifact,
                nodes: result
                    .nodes
                    .iter()
                    .map(|n| DeployNodeStatusEntry {
                        node_id: n.node_id,
                        status: n.status.clone(),
                        error: n.error.clone(),
                    })
                    .collect(),
                started_at_ms: result.started_at_ms,
                elapsed_ms: result.elapsed_ms,
                error: result.error,
            };
            debug_assert_eq!(output.nodes.len(), result.nodes.len());
            debug_assert!(output.nodes.iter().all(|node| node.node_id > 0));
            print_output(&output, is_json_output);
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn rollback(client: &AspenClient, is_json_output: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::ClusterRollback).await?;

    match response {
        ClientRpcResponse::ClusterRollbackResult(result) => {
            let output = RollbackOutput {
                is_accepted: result.is_accepted,
                deploy_id: result.deploy_id,
                error: result.error.clone(),
            };
            debug_assert!(output.is_accepted || output.error.is_some());
            debug_assert_eq!(output.is_accepted, result.is_accepted);
            print_output(&output, is_json_output);
            if !result.is_accepted {
                anyhow::bail!("rollback rejected: {}", result.error.as_deref().unwrap_or("unknown error"));
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

fn deploy_wait_max_attempts(timeout_secs: u64) -> u64 {
    timeout_secs.saturating_div(5).saturating_add(2)
}

fn deploy_wait_timeout_output(
    deploy_id: Option<&str>,
    timeout_secs: u64,
    prev_statuses: &std::collections::HashMap<u64, String>,
) -> DeployWaitFinalOutput {
    let final_nodes: Vec<DeployNodeStatusEntry> = prev_statuses
        .iter()
        .map(|(id, status)| DeployNodeStatusEntry {
            node_id: *id,
            status: status.clone(),
            error: None,
        })
        .collect();
    DeployWaitFinalOutput {
        deploy_id: deploy_id.map(String::from),
        status: "timeout".to_string(),
        elapsed_secs: timeout_secs,
        nodes: final_nodes,
        error: Some(format!("timed out after {}s", timeout_secs)),
    }
}

fn deploy_wait_final_output(
    deploy_id: Option<&str>,
    status: &str,
    elapsed_secs: u64,
    nodes: &[aspen_client_api::messages::NodeDeployStatusEntry],
    error: Option<String>,
) -> DeployWaitFinalOutput {
    DeployWaitFinalOutput {
        deploy_id: deploy_id.map(String::from),
        status: status.to_string(),
        elapsed_secs,
        nodes: nodes
            .iter()
            .map(|node| DeployNodeStatusEntry {
                node_id: node.node_id,
                status: node.status.clone(),
                error: node.error.clone(),
            })
            .collect(),
        error,
    }
}

fn print_deploy_wait_transitions(
    prev_statuses: &mut std::collections::HashMap<u64, String>,
    nodes: &[aspen_client_api::messages::NodeDeployStatusEntry],
    elapsed_secs: u64,
    is_json_output: bool,
) {
    const MAX_DEPLOY_STATUS_ENTRIES: usize = 1_000;

    debug_assert!(nodes.len() <= MAX_DEPLOY_STATUS_ENTRIES);
    for node in nodes {
        let previous_status = prev_statuses.get(&node.node_id).cloned().unwrap_or_default();
        if previous_status != node.status {
            let transition = DeployWaitOutput {
                node_id: node.node_id,
                old_status: previous_status,
                new_status: node.status.clone(),
                error: node.error.clone(),
                elapsed_secs,
            };
            print_output(&transition, is_json_output);
            prev_statuses.insert(node.node_id, node.status.clone());
        }
    }
}

struct DeployWaitRpcError<'a> {
    code: &'a str,
    message: &'a str,
}

fn report_deploy_wait_rpc_error(is_json_output: bool, consecutive_errors: u32, error: DeployWaitRpcError<'_>) {
    const WARN_EVERY_N_ERRORS: u32 = 5;

    if !is_json_output && consecutive_errors % WARN_EVERY_N_ERRORS == 1 {
        eprintln!(
            "warning: status poll RPC error ({} consecutive): {}: {}",
            consecutive_errors, error.code, error.message
        );
    }
}

fn handle_deploy_wait_status_response(
    deploy_id: Option<&str>,
    result: aspen_client_api::messages::ClusterDeployStatusResultResponse,
    prev_statuses: &mut std::collections::HashMap<u64, String>,
    is_json_output: bool,
) -> Result<bool> {
    debug_assert!(result.is_found || result.nodes.is_empty());
    debug_assert!(result.is_found || result.error.is_none());
    if !result.is_found {
        let output = deploy_wait_final_output(deploy_id, "completed", 0, &[], None);
        print_output(&output, is_json_output);
        return Ok(true);
    }
    if let (Some(expected), Some(actual)) = (deploy_id, &result.deploy_id)
        && expected != actual
    {
        anyhow::bail!("deployment ID mismatch: expected {}, found {}", expected, actual);
    }

    let elapsed_secs = result.elapsed_ms.unwrap_or(0) / 1000;
    print_deploy_wait_transitions(prev_statuses, &result.nodes, elapsed_secs, is_json_output);
    let status = result.status.as_deref().unwrap_or("unknown");
    if status == "completed" {
        let output = deploy_wait_final_output(deploy_id, status, elapsed_secs, &result.nodes, None);
        print_output(&output, is_json_output);
        return Ok(true);
    }
    if status == "failed" || status == "rolled_back" {
        let output = deploy_wait_final_output(deploy_id, status, elapsed_secs, &result.nodes, result.error.clone());
        print_output(&output, is_json_output);
        anyhow::bail!("deployment {}: {}", status, result.error.as_deref().unwrap_or("unknown error"));
    }

    Ok(false)
}

#[allow(
    ambient_clock,
    reason = "CLI deploy wait needs a monotonic start instant while polling restart progress"
)]
fn deploy_wait_start() -> tokio::time::Instant {
    tokio::time::Instant::now()
}

#[allow(
    ambient_clock,
    reason = "CLI deploy wait compares elapsed monotonic time against a timeout deadline"
)]
fn has_deploy_wait_timed_out(deadline: tokio::time::Instant) -> bool {
    tokio::time::Instant::now() >= deadline
}

/// Poll deployment status until a terminal state or timeout.
///
/// Diffs per-node statuses against a snapshot and prints transitions.
/// Returns `Ok(())` on completed, `Err` on failed/rolled_back/timeout.
/// Exit codes (handled by caller or process): 0=completed, 1=failed, 2=timeout.
///
/// During a rolling deploy, nodes restart — connection failures are expected.
/// This function tolerates errors for the entire deploy timeout duration,
/// using backoff to reduce QUIC pressure when nodes are temporarily down.
async fn deploy_wait(
    client: &AspenClient,
    deploy_id: Option<&str>,
    timeout_secs: u64,
    is_json_output: bool,
) -> Result<()> {
    use std::collections::HashMap;

    const BASE_POLL_INTERVAL_SECS: u64 = 5;
    const ERROR_POLL_INTERVAL_SECS: u64 = 10;
    const WARN_EVERY_N_ERRORS: u32 = 5;
    const MAX_DEPLOY_STATUS_ENTRIES: usize = 1_000;

    let base_sleep = std::time::Duration::from_secs(BASE_POLL_INTERVAL_SECS);
    let error_sleep = std::time::Duration::from_secs(ERROR_POLL_INTERVAL_SECS);
    let deadline = deploy_wait_start() + std::time::Duration::from_secs(timeout_secs);
    let mut prev_statuses: HashMap<u64, String> = HashMap::with_capacity(MAX_DEPLOY_STATUS_ENTRIES);
    let mut consecutive_errors: u32 = 0;

    debug_assert!(BASE_POLL_INTERVAL_SECS >= 1);
    debug_assert!(ERROR_POLL_INTERVAL_SECS >= BASE_POLL_INTERVAL_SECS);
    for _poll_attempt in 0..deploy_wait_max_attempts(timeout_secs) {
        tokio::time::sleep(if consecutive_errors > 0 {
            error_sleep
        } else {
            base_sleep
        })
        .await;
        if has_deploy_wait_timed_out(deadline) {
            let output = deploy_wait_timeout_output(deploy_id, timeout_secs, &prev_statuses);
            print_output(&output, is_json_output);
            std::process::exit(2);
        }

        let response = match client.send(ClientRpcRequest::ClusterDeployStatus).await {
            Ok(response) => {
                if consecutive_errors > 0 && !is_json_output {
                    eprintln!("info: reconnected after {} consecutive errors", consecutive_errors);
                }
                consecutive_errors = 0;
                response
            }
            Err(error) => {
                consecutive_errors = consecutive_errors.saturating_add(1);
                if !is_json_output && consecutive_errors % WARN_EVERY_N_ERRORS == 1 {
                    eprintln!(
                        "warning: status poll error ({} consecutive, will retry until deploy timeout): {}",
                        consecutive_errors, error
                    );
                }
                continue;
            }
        };

        match response {
            ClientRpcResponse::ClusterDeployStatusResult(result) => {
                if handle_deploy_wait_status_response(deploy_id, result, &mut prev_statuses, is_json_output)? {
                    return Ok(());
                }
            }
            ClientRpcResponse::Error(error) => {
                if error.code.contains("DEPLOY_UNAVAILABLE") {
                    anyhow::bail!("deploy feature not enabled on server");
                }
                consecutive_errors = consecutive_errors.saturating_add(1);
                report_deploy_wait_rpc_error(is_json_output, consecutive_errors, DeployWaitRpcError {
                    code: &error.code,
                    message: &error.message,
                });
            }
            _ => {
                if !is_json_output {
                    eprintln!("warning: unexpected response during status poll");
                }
            }
        }
    }

    anyhow::bail!("deployment polling exceeded bounded attempts after {}s", timeout_secs)
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;
    // Use Cli::try_parse_from to validate clap parsing for deploy subcommands.
    use crate::cli::Cli;

    #[test]
    fn test_cluster_deploy_parse_minimal() {
        let result = Cli::try_parse_from(["aspen-cli", "cluster", "deploy", "/nix/store/abc123-aspen-node"]);
        assert!(result.is_ok(), "parse failed: {:?}", result.err());
    }

    #[test]
    fn test_cluster_deploy_parse_all_flags() {
        let result = Cli::try_parse_from([
            "aspen-cli",
            "cluster",
            "deploy",
            "/nix/store/abc123-aspen-node",
            "--strategy",
            "rolling",
            "--max-concurrent",
            "3",
            "--health-timeout",
            "60",
        ]);
        assert!(result.is_ok(), "parse failed: {:?}", result.err());

        let cli = result.unwrap();
        match cli.command {
            crate::cli::Commands::Cluster(cmd) => match cmd {
                ClusterCommand::Deploy(args) => {
                    assert_eq!(args.artifact, "/nix/store/abc123-aspen-node");
                    assert_eq!(args.strategy, "rolling");
                    assert_eq!(args.max_concurrent, 3);
                    assert_eq!(args.health_timeout_secs, 60);
                }
                other => panic!("expected Deploy, got {:?}", std::mem::discriminant(&other)),
            },
            other => panic!("expected Cluster, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn test_cluster_deploy_defaults() {
        let result = Cli::try_parse_from(["aspen-cli", "cluster", "deploy", "blobhash123abc"]);
        assert!(result.is_ok());

        let cli = result.unwrap();
        match cli.command {
            crate::cli::Commands::Cluster(cmd) => match cmd {
                ClusterCommand::Deploy(args) => {
                    assert_eq!(args.strategy, "rolling");
                    assert_eq!(args.max_concurrent, 1);
                    assert_eq!(args.health_timeout_secs, 120);
                }
                other => panic!("expected Deploy, got {:?}", std::mem::discriminant(&other)),
            },
            other => panic!("expected Cluster, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn test_cluster_deploy_requires_artifact() {
        let result = Cli::try_parse_from(["aspen-cli", "cluster", "deploy"]);
        assert!(result.is_err(), "should fail without artifact");
    }

    #[test]
    fn test_cluster_deploy_status_parse() {
        let result = Cli::try_parse_from(["aspen-cli", "cluster", "deploy-status"]);
        assert!(result.is_ok(), "parse failed: {:?}", result.err());

        let cli = result.unwrap();
        match cli.command {
            crate::cli::Commands::Cluster(cmd) => {
                assert!(matches!(cmd, ClusterCommand::DeployStatus));
            }
            other => panic!("expected Cluster, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn test_cluster_rollback_parse() {
        let result = Cli::try_parse_from(["aspen-cli", "cluster", "rollback"]);
        assert!(result.is_ok(), "parse failed: {:?}", result.err());

        let cli = result.unwrap();
        match cli.command {
            crate::cli::Commands::Cluster(cmd) => {
                assert!(matches!(cmd, ClusterCommand::Rollback));
            }
            other => panic!("expected Cluster, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn test_cluster_deploy_rejects_unknown_flag() {
        let result = Cli::try_parse_from([
            "aspen-cli",
            "cluster",
            "deploy",
            "/nix/store/abc",
            "--unknown-flag",
            "val",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cluster_deploy_wait_flag() {
        let result = Cli::try_parse_from([
            "aspen-cli",
            "cluster",
            "deploy",
            "/nix/store/abc123-aspen-node",
            "--wait",
        ]);
        assert!(result.is_ok(), "parse failed: {:?}", result.err());

        let cli = result.unwrap();
        match cli.command {
            crate::cli::Commands::Cluster(cmd) => match cmd {
                ClusterCommand::Deploy(args) => {
                    assert!(args.wait);
                    assert_eq!(args.deploy_timeout_secs, 3600); // default
                }
                other => panic!("expected Deploy, got {:?}", std::mem::discriminant(&other)),
            },
            other => panic!("expected Cluster, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn test_cluster_deploy_wait_with_timeout() {
        let result = Cli::try_parse_from([
            "aspen-cli",
            "cluster",
            "deploy",
            "/nix/store/abc123-aspen-node",
            "--wait",
            "--deploy-timeout",
            "120",
        ]);
        assert!(result.is_ok(), "parse failed: {:?}", result.err());

        let cli = result.unwrap();
        match cli.command {
            crate::cli::Commands::Cluster(cmd) => match cmd {
                ClusterCommand::Deploy(args) => {
                    assert!(args.wait);
                    assert_eq!(args.deploy_timeout_secs, 120);
                }
                other => panic!("expected Deploy, got {:?}", std::mem::discriminant(&other)),
            },
            other => panic!("expected Cluster, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn test_cluster_deploy_timeout_without_wait() {
        // --timeout without --wait should parse fine (ignored at runtime)
        let result = Cli::try_parse_from([
            "aspen-cli",
            "cluster",
            "deploy",
            "/nix/store/abc123-aspen-node",
            "--deploy-timeout",
            "60",
        ]);
        assert!(result.is_ok(), "parse failed: {:?}", result.err());

        let cli = result.unwrap();
        match cli.command {
            crate::cli::Commands::Cluster(cmd) => match cmd {
                ClusterCommand::Deploy(args) => {
                    assert!(!args.wait);
                    assert_eq!(args.deploy_timeout_secs, 60);
                }
                other => panic!("expected Deploy, got {:?}", std::mem::discriminant(&other)),
            },
            other => panic!("expected Cluster, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn test_cluster_deploy_wait_defaults_no_wait() {
        let result = Cli::try_parse_from(["aspen-cli", "cluster", "deploy", "blobhash123"]);
        assert!(result.is_ok());

        let cli = result.unwrap();
        match cli.command {
            crate::cli::Commands::Cluster(cmd) => match cmd {
                ClusterCommand::Deploy(args) => {
                    assert!(!args.wait);
                    assert_eq!(args.deploy_timeout_secs, 3600);
                }
                other => panic!("expected Deploy, got {:?}", std::mem::discriminant(&other)),
            },
            other => panic!("expected Cluster, got {:?}", std::mem::discriminant(&other)),
        }
    }

    #[test]
    fn test_cluster_deploy_all_flags_with_wait() {
        let result = Cli::try_parse_from([
            "aspen-cli",
            "cluster",
            "deploy",
            "/nix/store/abc",
            "--strategy",
            "rolling",
            "--max-concurrent",
            "2",
            "--health-timeout",
            "60",
            "--wait",
            "--deploy-timeout",
            "900",
        ]);
        assert!(result.is_ok(), "parse failed: {:?}", result.err());

        let cli = result.unwrap();
        match cli.command {
            crate::cli::Commands::Cluster(cmd) => match cmd {
                ClusterCommand::Deploy(args) => {
                    assert_eq!(args.artifact, "/nix/store/abc");
                    assert_eq!(args.strategy, "rolling");
                    assert_eq!(args.max_concurrent, 2);
                    assert_eq!(args.health_timeout_secs, 60);
                    assert!(args.wait);
                    assert_eq!(args.deploy_timeout_secs, 900);
                }
                other => panic!("expected Deploy, got {:?}", std::mem::discriminant(&other)),
            },
            other => panic!("expected Cluster, got {:?}", std::mem::discriminant(&other)),
        }
    }
}
