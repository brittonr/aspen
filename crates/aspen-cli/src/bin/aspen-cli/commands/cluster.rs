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

    /// Start a rolling deployment of a new binary.
    Deploy(DeployArgs),

    /// Show deployment status with per-node breakdown.
    DeployStatus,

    /// Roll back the current or last deployment.
    Rollback,

    /// Update a peer's address in the local network factory (no Raft consensus).
    UpdatePeer(UpdatePeerArgs),
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
    #[arg(long, default_value_t = 120)]
    pub health_timeout: u64,
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
            ClusterCommand::Deploy(args) => deploy(client, args, json).await,
            ClusterCommand::DeployStatus => deploy_status(client, json).await,
            ClusterCommand::Rollback => rollback(client, json).await,
            ClusterCommand::UpdatePeer(args) => update_peer(client, args, json).await,
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

async fn update_peer(client: &AspenClient, args: UpdatePeerArgs, json: bool) -> Result<()> {
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
            print_output(&output, json);
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

async fn promote_learner(client: &AspenClient, args: PromoteArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::PromoteLearner {
            learner_id: args.learner_id,
            replace_node: args.replace,
            is_force: args.is_force,
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
                        "promoted": result.is_success,
                        "message": result.message
                    })
                );
            } else if result.is_success {
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

async fn deploy(client: &AspenClient, args: DeployArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::ClusterDeploy {
            artifact: args.artifact,
            strategy: args.strategy,
            max_concurrent: args.max_concurrent,
            health_timeout_secs: args.health_timeout,
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
                print_output(&output, json);
                anyhow::bail!("deployment rejected: {}", result.error.as_deref().unwrap_or("unknown error"));
            }

            print_output(&output, json);

            // Poll for progress until terminal state
            let deploy_id = result.deploy_id;
            poll_deploy_progress(client, deploy_id.as_deref(), json).await
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn deploy_status(client: &AspenClient, json: bool) -> Result<()> {
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
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => {
            anyhow::bail!("{}: {}", e.code, e.message)
        }
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn rollback(client: &AspenClient, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::ClusterRollback).await?;

    match response {
        ClientRpcResponse::ClusterRollbackResult(result) => {
            let output = RollbackOutput {
                is_accepted: result.is_accepted,
                deploy_id: result.deploy_id,
                error: result.error.clone(),
            };
            print_output(&output, json);
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

/// Poll deployment status until a terminal state is reached.
async fn poll_deploy_progress(client: &AspenClient, deploy_id: Option<&str>, json: bool) -> Result<()> {
    // Matches aspen_constants::DEPLOY_STATUS_POLL_INTERVAL_SECS
    const POLL_INTERVAL_SECS: u64 = 5;

    let interval = std::time::Duration::from_secs(POLL_INTERVAL_SECS);

    loop {
        tokio::time::sleep(interval).await;

        let response = client.send(ClientRpcRequest::ClusterDeployStatus).await?;

        match response {
            ClientRpcResponse::ClusterDeployStatusResult(result) => {
                if !result.is_found {
                    // Deployment disappeared (completed and archived)
                    if !json {
                        println!("Deployment completed");
                    }
                    return Ok(());
                }

                // Only show progress for the deployment we initiated
                if let (Some(expected), Some(actual)) = (deploy_id, &result.deploy_id) {
                    if expected != actual {
                        anyhow::bail!("deployment ID mismatch: expected {}, found {}", expected, actual);
                    }
                }

                let status = result.status.as_deref().unwrap_or("unknown");
                let elapsed_secs = result.elapsed_ms.unwrap_or(0) / 1000;

                if !json {
                    // Print per-node progress
                    let node_summary: String = result
                        .nodes
                        .iter()
                        .map(|n| format!("  node {}: {}", n.node_id, n.status))
                        .collect::<Vec<_>>()
                        .join("\n");
                    println!("[{}s] status: {}\n{}", elapsed_secs, status, node_summary);
                }

                // Check for terminal states
                match status {
                    "completed" => {
                        if !json {
                            println!("Deployment completed successfully");
                        }
                        return Ok(());
                    }
                    "failed" => {
                        let err_msg = result.error.as_deref().unwrap_or("unknown failure");
                        anyhow::bail!("deployment failed: {}", err_msg);
                    }
                    "rolled_back" => {
                        anyhow::bail!("deployment was rolled back");
                    }
                    _ => {
                        // Still in progress (pending, deploying, rolling_back)
                    }
                }
            }
            ClientRpcResponse::Error(e) => {
                if e.code.contains("DEPLOY_UNAVAILABLE") {
                    anyhow::bail!("deploy feature not enabled on server");
                }
                // Transient errors — keep polling
                if !json {
                    eprintln!("warning: status poll error: {}: {}", e.code, e.message);
                }
            }
            _ => {
                if !json {
                    eprintln!("warning: unexpected response during status poll");
                }
            }
        }
    }
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
                    assert_eq!(args.health_timeout, 60);
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
                    assert_eq!(args.health_timeout, 120);
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
}
