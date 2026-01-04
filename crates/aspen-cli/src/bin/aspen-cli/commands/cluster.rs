//! Cluster management commands.
//!
//! Commands for cluster initialization, status, health checks,
//! membership management, and maintenance operations.

use anyhow::Result;
use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::ClusterStateOutput;
use crate::output::HealthOutput;
use crate::output::NodeInfo;
use crate::output::RaftMetricsOutput;
use crate::output::print_output;
use crate::output::print_success;

/// Cluster management commands.
#[derive(Subcommand)]
pub enum ClusterCommand {
    /// Initialize a new cluster.
    Init,

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
}

#[derive(Args)]
pub struct AddLearnerArgs {
    /// Node ID of the learner to add.
    #[arg(long)]
    pub node_id: u64,

    /// Network address of the learner (endpoint_id format).
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
    #[arg(long)]
    pub force: bool,
}

#[derive(Args)]
pub struct ChangeMembershipArgs {
    /// New set of voter node IDs.
    #[arg(required = true)]
    pub members: Vec<u64>,
}

impl ClusterCommand {
    /// Execute the cluster command.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            ClusterCommand::Init => init_cluster(client, json).await,
            ClusterCommand::Status => cluster_status(client, json).await,
            ClusterCommand::Health => health_check(client, json).await,
            ClusterCommand::Metrics => raft_metrics(client, json).await,
            ClusterCommand::Prometheus => prometheus_metrics(client, json).await,
            ClusterCommand::AddLearner(args) => add_learner(client, args, json).await,
            ClusterCommand::Promote(args) => promote_learner(client, args, json).await,
            ClusterCommand::ChangeMembership(args) => change_membership(client, args, json).await,
            ClusterCommand::Snapshot => trigger_snapshot(client, json).await,
            ClusterCommand::CheckpointWal => checkpoint_wal(client, json).await,
            ClusterCommand::Ticket => get_ticket(client, json).await,
        }
    }
}

async fn init_cluster(client: &AspenClient, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::InitCluster).await?;

    match response {
        ClientRpcResponse::InitResult(result) => {
            if json {
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

async fn cluster_status(client: &AspenClient, json: bool) -> Result<()> {
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
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn health_check(client: &AspenClient, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::GetHealth).await?;

    match response {
        ClientRpcResponse::Health(health) => {
            let output = HealthOutput {
                status: health.status,
                node_id: health.node_id,
                raft_node_id: health.raft_node_id,
                uptime_seconds: health.uptime_seconds,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn raft_metrics(client: &AspenClient, json: bool) -> Result<()> {
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
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn prometheus_metrics(client: &AspenClient, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::GetMetrics).await?;

    match response {
        ClientRpcResponse::Metrics(metrics) => {
            if json {
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

async fn add_learner(client: &AspenClient, args: AddLearnerArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::AddLearner {
            node_id: args.node_id,
            addr: args.addr.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::AddLearnerResult(_) => {
            print_success(&format!("Learner {} added at {}", args.node_id, args.addr), json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn promote_learner(client: &AspenClient, args: PromoteArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::PromoteLearner {
            learner_id: args.learner_id,
            replace_node: args.replace,
            force: args.force,
        })
        .await?;

    match response {
        ClientRpcResponse::PromoteLearnerResult(result) => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "success",
                        "learner_id": args.learner_id,
                        "promoted": result.success,
                        "message": result.message
                    })
                );
            } else if result.success {
                println!("Learner {} promoted to voter", args.learner_id);
            } else {
                println!("Promotion failed: {}", result.message);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn change_membership(client: &AspenClient, args: ChangeMembershipArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::ChangeMembership {
            members: args.members.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::ChangeMembershipResult(_) => {
            print_success(&format!("Membership changed to voters: {:?}", args.members), json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn trigger_snapshot(client: &AspenClient, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::TriggerSnapshot).await?;

    match response {
        ClientRpcResponse::SnapshotResult(_) => {
            print_success("Snapshot triggered successfully", json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn checkpoint_wal(client: &AspenClient, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::CheckpointWal).await?;

    match response {
        ClientRpcResponse::CheckpointWalResult(result) => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": if result.success { "success" } else { "failed" },
                        "pages_checkpointed": result.pages_checkpointed,
                        "wal_size_before_bytes": result.wal_size_before_bytes,
                        "wal_size_after_bytes": result.wal_size_after_bytes
                    })
                );
            } else if result.success {
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

async fn get_ticket(client: &AspenClient, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::GetClusterTicket).await?;

    match response {
        ClientRpcResponse::ClusterTicket(ticket_response) => {
            if json {
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
