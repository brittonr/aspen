//! Federation commands for cross-cluster discovery.
//!
//! Commands for managing federation with other Aspen clusters,
//! including cluster identity, discovery, and federated resources.

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::Outputable;
use crate::output::print_output;

/// Federation operations for cross-cluster discovery.
#[derive(Subcommand)]
pub enum FederationCommand {
    /// Show federation status and cluster identity.
    Status,

    /// List discovered federated clusters.
    Peers,

    /// Show details of a specific federated cluster.
    Peer(PeerArgs),

    /// Trust a cluster (add to trusted list).
    Trust(TrustArgs),

    /// Untrust a cluster (remove from trusted list).
    Untrust(UntrustArgs),

    /// Federate a repository (make it discoverable).
    Federate(FederateArgs),

    /// List federated repositories.
    ListFederated,
}

#[derive(Args)]
pub struct PeerArgs {
    /// Cluster public key (base32 or hex).
    pub cluster_key: String,
}

#[derive(Args)]
pub struct TrustArgs {
    /// Cluster public key to trust (base32 or hex).
    pub cluster_key: String,
}

#[derive(Args)]
pub struct UntrustArgs {
    /// Cluster public key to untrust (base32 or hex).
    pub cluster_key: String,
}

#[derive(Args)]
pub struct FederateArgs {
    /// Repository ID to federate.
    pub repo_id: String,

    /// Federation mode (public or allowlist).
    #[arg(long, default_value = "public")]
    pub mode: String,
}

/// Federation status output.
pub struct FederationStatusOutput {
    pub is_enabled: bool,
    pub cluster_name: String,
    pub cluster_key: String,
    pub dht_enabled: bool,
    pub gossip_enabled: bool,
    pub discovered_clusters: u32,
    pub federated_repos: u32,
    pub error: Option<String>,
}

impl Outputable for FederationStatusOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "enabled": self.is_enabled,
            "cluster_name": self.cluster_name,
            "cluster_key": self.cluster_key,
            "dht_enabled": self.dht_enabled,
            "gossip_enabled": self.gossip_enabled,
            "discovered_clusters": self.discovered_clusters,
            "federated_repos": self.federated_repos,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if let Some(ref err) = self.error {
            return format!("Error: {}", err);
        }

        if !self.is_enabled {
            return "Federation is disabled".to_string();
        }

        format!(
            "Federation Status\n\
             Cluster Name:        {}\n\
             Cluster Key:         {}\n\
             DHT Discovery:       {}\n\
             Gossip Enabled:      {}\n\
             Discovered Clusters: {}\n\
             Federated Repos:     {}",
            self.cluster_name,
            self.cluster_key,
            if self.dht_enabled { "enabled" } else { "disabled" },
            if self.gossip_enabled { "enabled" } else { "disabled" },
            self.discovered_clusters,
            self.federated_repos
        )
    }
}

/// Discovered clusters list output.
pub struct DiscoveredClustersOutput {
    pub clusters: Vec<DiscoveredClusterInfo>,
    pub count: u32,
    pub error: Option<String>,
}

pub struct DiscoveredClusterInfo {
    pub cluster_key: String,
    pub name: String,
    pub node_count: u32,
    pub capabilities: Vec<String>,
    pub discovered_at: String,
}

impl Outputable for DiscoveredClustersOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "clusters": self.clusters.iter().map(|c| {
                serde_json::json!({
                    "cluster_key": c.cluster_key,
                    "name": c.name,
                    "node_count": c.node_count,
                    "capabilities": c.capabilities,
                    "discovered_at": c.discovered_at
                })
            }).collect::<Vec<_>>(),
            "count": self.count,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if let Some(ref err) = self.error {
            return format!("Error: {}", err);
        }

        if self.clusters.is_empty() {
            return "No federated clusters discovered".to_string();
        }

        let mut output = format!("Discovered Clusters ({})\n", self.count);
        output.push_str("Cluster Key (short)  | Name             | Nodes | Capabilities\n");
        output.push_str("---------------------+------------------+-------+-------------\n");

        for cluster in &self.clusters {
            let short_key = if cluster.cluster_key.len() > 16 {
                format!("{}...", &cluster.cluster_key[..16])
            } else {
                cluster.cluster_key.clone()
            };
            let caps = cluster.capabilities.join(", ");
            output.push_str(&format!(
                "{:20} | {:16} | {:>5} | {}\n",
                short_key,
                &cluster.name[..16.min(cluster.name.len())],
                cluster.node_count,
                caps
            ));
        }

        output
    }
}

/// Simple success output.
pub struct FederationSuccessOutput {
    pub operation: String,
    pub success: bool,
    pub message: Option<String>,
    pub error: Option<String>,
}

impl Outputable for FederationSuccessOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "operation": self.operation,
            "success": self.success,
            "message": self.message,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.success {
            self.message.clone().unwrap_or_else(|| "OK".to_string())
        } else {
            format!("{} failed: {}", self.operation, self.error.as_deref().unwrap_or("unknown error"))
        }
    }
}

impl FederationCommand {
    /// Execute the federation command.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            FederationCommand::Status => federation_status(client, json).await,
            FederationCommand::Peers => federation_peers(client, json).await,
            FederationCommand::Peer(args) => federation_peer(client, args, json).await,
            FederationCommand::Trust(args) => federation_trust(client, args, json).await,
            FederationCommand::Untrust(args) => federation_untrust(client, args, json).await,
            FederationCommand::Federate(args) => federation_federate(client, args, json).await,
            FederationCommand::ListFederated => federation_list_federated(client, json).await,
        }
    }
}

async fn federation_status(client: &AspenClient, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::GetFederationStatus).await?;

    match response {
        ClientRpcResponse::FederationStatus(status) => {
            let output = FederationStatusOutput {
                is_enabled: status.is_enabled,
                cluster_name: status.cluster_name,
                cluster_key: status.cluster_key,
                dht_enabled: status.dht_enabled,
                gossip_enabled: status.gossip_enabled,
                discovered_clusters: status.discovered_clusters,
                federated_repos: status.federated_repos,
                error: status.error,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn federation_peers(client: &AspenClient, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::ListDiscoveredClusters).await?;

    match response {
        ClientRpcResponse::DiscoveredClusters(result) => {
            let clusters = result
                .clusters
                .into_iter()
                .map(|c| DiscoveredClusterInfo {
                    cluster_key: c.cluster_key,
                    name: c.name,
                    node_count: c.node_count,
                    capabilities: c.capabilities,
                    discovered_at: c.discovered_at,
                })
                .collect();

            let output = DiscoveredClustersOutput {
                clusters,
                count: result.count,
                error: result.error,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn federation_peer(client: &AspenClient, args: PeerArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::GetDiscoveredCluster {
            cluster_key: args.cluster_key.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::DiscoveredCluster(result) => {
            if !result.found {
                let output = FederationSuccessOutput {
                    operation: "get_peer".to_string(),
                    success: false,
                    message: None,
                    error: Some(format!("Cluster not found: {}", args.cluster_key)),
                };
                print_output(&output, json);
                std::process::exit(1);
            }

            // For now, just show the raw JSON
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "found": result.found,
                        "cluster_key": result.cluster_key,
                        "name": result.name,
                        "node_count": result.node_count,
                        "capabilities": result.capabilities,
                        "relay_urls": result.relay_urls,
                        "discovered_at": result.discovered_at
                    })
                );
            } else {
                println!("Cluster Key:   {}", result.cluster_key.unwrap_or_default());
                println!("Name:          {}", result.name.unwrap_or_default());
                println!("Nodes:         {}", result.node_count.unwrap_or(0));
                println!("Capabilities:  {}", result.capabilities.unwrap_or_default().join(", "));
                println!("Relay URLs:    {}", result.relay_urls.unwrap_or_default().join(", "));
                println!("Discovered At: {}", result.discovered_at.unwrap_or_default());
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn federation_trust(client: &AspenClient, args: TrustArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::TrustCluster {
            cluster_key: args.cluster_key.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::TrustClusterResult(result) => {
            let output = FederationSuccessOutput {
                operation: "trust".to_string(),
                success: result.success,
                message: if result.success {
                    Some(format!("Trusted cluster: {}", args.cluster_key))
                } else {
                    None
                },
                error: result.error,
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

async fn federation_untrust(client: &AspenClient, args: UntrustArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::UntrustCluster {
            cluster_key: args.cluster_key.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::UntrustClusterResult(result) => {
            let output = FederationSuccessOutput {
                operation: "untrust".to_string(),
                success: result.success,
                message: if result.success {
                    Some(format!("Untrusted cluster: {}", args.cluster_key))
                } else {
                    None
                },
                error: result.error,
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

async fn federation_federate(client: &AspenClient, args: FederateArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::FederateRepository {
            repo_id: args.repo_id.clone(),
            mode: args.mode.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::FederateRepositoryResult(result) => {
            let output = FederationSuccessOutput {
                operation: "federate".to_string(),
                success: result.success,
                message: if result.success {
                    Some(format!("Federated repository {} (mode: {})", args.repo_id, args.mode))
                } else {
                    None
                },
                error: result.error,
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

async fn federation_list_federated(client: &AspenClient, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::ListFederatedRepositories).await?;

    match response {
        ClientRpcResponse::FederatedRepositories(result) => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "repositories": result.repositories,
                        "count": result.count,
                        "error": result.error
                    })
                );
            } else if result.repositories.is_empty() {
                println!("No repositories are federated");
            } else {
                println!("Federated Repositories ({})", result.count);
                println!("Repo ID                          | Mode      | Fed ID");
                println!("---------------------------------+-----------+--------");
                for repo in &result.repositories {
                    println!("{:32} | {:9} | {}", &repo.repo_id[..32.min(repo.repo_id.len())], repo.mode, repo.fed_id);
                }
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}
