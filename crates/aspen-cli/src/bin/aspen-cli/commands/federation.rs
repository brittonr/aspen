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

    /// Perform a one-shot federation sync pull from a remote cluster.
    Sync(SyncArgs),

    /// Fetch ref objects from a remote federated repository.
    Fetch(FetchArgs),

    /// Pull updates for a previously mirrored federation repo.
    Pull(PullArgs),

    /// Push a local repo's objects and refs to a remote cluster.
    Push(PushArgs),

    /// Issue a federation capability token to a remote cluster.
    Grant(GrantArgs),

    /// Manage federation tokens.
    #[command(subcommand)]
    Token(TokenCommand),
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

/// Arguments for federation sync command.
#[derive(Args)]
pub struct SyncArgs {
    /// Remote peer's iroh node ID (base32-encoded public key).
    #[arg(long)]
    pub peer: String,

    /// Direct socket address hint for the remote peer (e.g., "192.168.1.1:54866").
    /// Helps iroh establish a QUIC connection without relay or discovery.
    #[arg(long)]
    pub addr: Option<String>,

    /// After discovering refs, also fetch ref objects and persist locally.
    #[arg(long = "fetch")]
    pub is_fetch: bool,

    /// Repo ID (hex-encoded) for bidirectional sync. When set, compares
    /// local and remote ref heads and transfers objects in both directions.
    #[arg(long)]
    pub repo: Option<String>,

    /// On divergent refs, local wins (push to remote). Default: remote wins (pull).
    #[arg(long = "push-wins")]
    pub is_push_wins: bool,
}

/// Arguments for federation fetch command.
#[derive(Args)]
pub struct FetchArgs {
    /// Remote peer's iroh node ID (base32-encoded public key).
    #[arg(long)]
    pub peer: String,

    /// Direct socket address hint for the remote peer.
    #[arg(long)]
    pub addr: Option<String>,

    /// Federated resource ID (origin:local_id format).
    #[arg(long)]
    pub fed_id: String,
}

/// Arguments for federation push command.
#[derive(Args)]
pub struct PushArgs {
    /// Remote peer's iroh node ID (base32-encoded public key).
    #[arg(long)]
    pub peer: String,

    /// Direct socket address hint for the remote peer.
    #[arg(long)]
    pub addr: Option<String>,

    /// Local repo ID to push (hex-encoded).
    #[arg(long)]
    pub repo: String,
}

/// Arguments for federation pull command.
///
/// Two modes:
/// - Cold pull: `--peer <node-id> --repo <remote-repo-hex>` creates a mirror.
/// - Mirror pull: `--repo <mirror-id>` updates an existing mirror.
#[derive(Args)]
pub struct PullArgs {
    /// Repo ID (hex-encoded). Interpretation depends on --peer:
    /// with --peer: remote repo ID on the peer cluster.
    /// without --peer: local mirror repo ID.
    #[arg(long)]
    pub repo: String,

    /// Remote peer's iroh node ID (base32-encoded). Enables cold-pull mode.
    #[arg(long)]
    pub peer: Option<String>,

    /// Direct socket address hint for the remote peer (e.g., "192.168.1.5:12345").
    #[arg(long)]
    pub addr: Option<String>,
}

/// Arguments for federation grant command.
#[derive(Args)]
pub struct GrantArgs {
    /// Remote cluster's public key (audience).
    #[arg(long)]
    pub audience: String,

    /// Capabilities: comma-separated list of "pull", "push", or "pull,push".
    /// Each can have an optional `:prefix` suffix (e.g., "pull:forge:org-a/").
    #[arg(long, default_value = "pull")]
    pub caps: String,

    /// Token lifetime in seconds (default: 86400 = 24 hours).
    #[arg(long, default_value = "86400")]
    pub lifetime: u64,
}

/// Token management subcommands.
#[derive(Subcommand)]
pub enum TokenCommand {
    /// List active federation tokens issued by this cluster.
    List,

    /// Inspect a base64-encoded federation token (decode without verifying).
    Inspect(InspectArgs),
}

/// Arguments for token inspect command.
#[derive(Args)]
pub struct InspectArgs {
    /// Base64-encoded token string to inspect.
    pub token_b64: String,
}

/// Federation status output.
pub struct FederationStatusOutput {
    pub is_enabled: bool,
    pub cluster_name: String,
    pub cluster_key: String,
    pub is_dht_enabled: bool,
    pub is_gossip_enabled: bool,
    pub discovered_clusters: u32,
    pub federated_repos: u32,
    pub error: Option<String>,
}

impl Outputable for FederationStatusOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "is_enabled": self.is_enabled,
            "cluster_name": self.cluster_name,
            "cluster_key": self.cluster_key,
            "is_dht_enabled": self.is_dht_enabled,
            "is_gossip_enabled": self.is_gossip_enabled,
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
             Gossip:              {}\n\
             Discovered Clusters: {}\n\
             Federated Repos:     {}",
            self.cluster_name,
            self.cluster_key,
            if self.is_dht_enabled { "enabled" } else { "disabled" },
            if self.is_gossip_enabled { "enabled" } else { "disabled" },
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
    pub is_success: bool,
    pub message: Option<String>,
    pub error: Option<String>,
}

impl Outputable for FederationSuccessOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "operation": self.operation,
            "is_success": self.is_success,
            "message": self.message,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.is_success {
            self.message.clone().unwrap_or_else(|| "OK".to_string())
        } else {
            format!("{} failed: {}", self.operation, self.error.as_deref().unwrap_or("unknown error"))
        }
    }
}

impl FederationCommand {
    /// Execute the federation command.
    pub async fn run(self, client: &AspenClient, is_json_output: bool) -> Result<()> {
        match self {
            FederationCommand::Status => federation_status(client, is_json_output).await,
            FederationCommand::Peers => federation_peers(client, is_json_output).await,
            FederationCommand::Peer(args) => federation_peer(client, args, is_json_output).await,
            FederationCommand::Trust(args) => federation_trust(client, args, is_json_output).await,
            FederationCommand::Untrust(args) => federation_untrust(client, args, is_json_output).await,
            FederationCommand::Federate(args) => federation_federate(client, args, is_json_output).await,
            FederationCommand::ListFederated => federation_list_federated(client, is_json_output).await,
            FederationCommand::Sync(args) => federation_sync(client, args, is_json_output).await,
            FederationCommand::Fetch(args) => federation_fetch(client, args, is_json_output).await,
            FederationCommand::Pull(args) => federation_pull(client, args, is_json_output).await,
            FederationCommand::Push(args) => federation_push(client, args, is_json_output).await,
            FederationCommand::Grant(args) => federation_grant(client, args, is_json_output).await,
            FederationCommand::Token(cmd) => match cmd {
                TokenCommand::List => federation_token_list(client, is_json_output).await,
                TokenCommand::Inspect(args) => federation_token_inspect(args, is_json_output).await,
            },
        }
    }
}

async fn federation_status(client: &AspenClient, is_json_output: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::GetFederationStatus).await?;

    debug_assert!(!is_json_output || is_json_output);

    match response {
        ClientRpcResponse::FederationStatus(status) => {
            let output = FederationStatusOutput {
                is_enabled: status.is_enabled,
                cluster_name: status.cluster_name,
                cluster_key: status.cluster_key,
                is_dht_enabled: status.dht_enabled,
                is_gossip_enabled: status.gossip_enabled,
                discovered_clusters: status.discovered_clusters,
                federated_repos: status.federated_repos,
                error: status.error,
            };
            debug_assert!(output.error.is_none() || !output.is_enabled);
            debug_assert!(output.discovered_clusters == 0 || output.is_enabled || output.error.is_some());
            print_output(&output, is_json_output);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn federation_peers(client: &AspenClient, is_json_output: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::ListDiscoveredClusters).await?;

    debug_assert!(!is_json_output || is_json_output);

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
            debug_assert!(usize::try_from(output.count).unwrap_or(usize::MAX) >= output.clusters.len());
            debug_assert!(output.clusters.iter().all(|cluster| !cluster.cluster_key.is_empty()));
            print_output(&output, is_json_output);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn federation_peer(client: &AspenClient, args: PeerArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.cluster_key.is_empty(), "cluster key must not be empty");
    let response = client
        .send(ClientRpcRequest::GetDiscoveredCluster {
            cluster_key: args.cluster_key.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::DiscoveredCluster(result) => {
            debug_assert!(result.cluster_key.is_some() || !result.was_found);
            debug_assert!(result.name.is_some() || !result.was_found);
            if !result.was_found {
                let output = FederationSuccessOutput {
                    operation: "get_peer".to_string(),
                    is_success: false,
                    message: None,
                    error: Some(format!("Cluster not found: {}", args.cluster_key)),
                };
                print_output(&output, is_json_output);
                std::process::exit(1);
            }

            // For now, just show the raw JSON
            if is_json_output {
                println!(
                    "{}",
                    serde_json::json!({
                        "was_found": result.was_found,
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

async fn federation_trust(client: &AspenClient, args: TrustArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.cluster_key.is_empty(), "cluster key must not be empty");
    let response = client
        .send(ClientRpcRequest::TrustCluster {
            cluster_key: args.cluster_key.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::TrustClusterResult(result) => {
            let output = FederationSuccessOutput {
                operation: "trust".to_string(),
                is_success: result.is_success,
                message: if result.is_success {
                    Some(format!("Trusted cluster: {}", args.cluster_key))
                } else {
                    None
                },
                error: result.error,
            };
            debug_assert!(output.message.is_some() == output.is_success);
            debug_assert!(!output.operation.is_empty());
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

async fn federation_untrust(client: &AspenClient, args: UntrustArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.cluster_key.is_empty(), "cluster key must not be empty");
    let response = client
        .send(ClientRpcRequest::UntrustCluster {
            cluster_key: args.cluster_key.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::UntrustClusterResult(result) => {
            let output = FederationSuccessOutput {
                operation: "untrust".to_string(),
                is_success: result.is_success,
                message: if result.is_success {
                    Some(format!("Untrusted cluster: {}", args.cluster_key))
                } else {
                    None
                },
                error: result.error,
            };
            debug_assert!(output.message.is_some() == output.is_success);
            debug_assert!(!output.operation.is_empty());
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

async fn federation_federate(client: &AspenClient, args: FederateArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.repo_id.is_empty(), "repo id must not be empty");
    debug_assert!(!args.mode.is_empty(), "mode must not be empty");
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
                is_success: result.is_success,
                message: if result.is_success {
                    Some(format!("Federated repository {} (mode: {})", args.repo_id, args.mode))
                } else {
                    None
                },
                error: result.error,
            };
            debug_assert!(output.message.is_some() == output.is_success);
            debug_assert!(!output.operation.is_empty());
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

async fn federation_sync(client: &AspenClient, args: SyncArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.peer.is_empty(), "peer must not be empty");
    // If --repo is set, dispatch to bidirectional sync
    if let Some(ref repo) = args.repo {
        return federation_bidi_sync(
            client,
            BidiSyncTarget {
                peer: &args.peer,
                addr: args.addr.as_deref(),
                repo,
                is_push_wins: args.is_push_wins,
            },
            is_json_output,
        )
        .await;
    }

    // Validate: --push-wins only makes sense with --repo
    if args.is_push_wins {
        anyhow::bail!("--push-wins requires --repo (bidirectional sync mode)");
    }

    let should_fetch_refs = args.is_fetch;
    let peer = args.peer.clone();
    let addr = args.addr.clone();

    let response = client
        .send(ClientRpcRequest::FederationSyncPeer {
            peer_node_id: peer.clone(),
            peer_addr: addr.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::FederationSyncPeerResult(result) => {
            debug_assert!(result.resources.iter().all(|resource| !resource.resource_type.is_empty()));
            debug_assert!(result.remote_cluster_key.is_some() || !result.is_success);
            if is_json_output && !should_fetch_refs {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else if result.is_success {
                println!("Federation sync successful!");
                println!(
                    "  Remote cluster: {} ({})",
                    result.remote_cluster_name.as_deref().unwrap_or("unknown"),
                    result.remote_cluster_key.as_deref().unwrap_or("?"),
                );
                println!("  Trusted: {}", trusted_status_label(result.trusted));
                print_sync_resources(&result.resources);

                fetch_discovered_refs(client, FetchDiscoveredRefsInput {
                    should_fetch_refs,
                    peer: &peer,
                    addr: addr.as_deref(),
                    resources: &result.resources,
                    is_json_output,
                })
                .await?;
            } else {
                eprintln!("Federation sync failed: {}", result.error.as_deref().unwrap_or("unknown error"));
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

struct BidiSyncTarget<'a> {
    peer: &'a str,
    addr: Option<&'a str>,
    repo: &'a str,
    is_push_wins: bool,
}

async fn federation_bidi_sync(client: &AspenClient, target: BidiSyncTarget<'_>, is_json_output: bool) -> Result<()> {
    debug_assert!(!target.peer.is_empty(), "peer must not be empty");
    debug_assert!(!target.repo.is_empty(), "repo must not be empty");
    let response = client
        .send(ClientRpcRequest::FederationBidiSync {
            peer_node_id: target.peer.to_string(),
            peer_addr: target.addr.map(str::to_string),
            repo_id: target.repo.to_string(),
            push_wins: target.is_push_wins,
        })
        .await?;

    match response {
        ClientRpcResponse::FederationBidiSyncResult(result) => {
            debug_assert!(result.conflicts.iter().all(|conflict| !conflict.is_empty()));
            debug_assert!(result.errors.iter().all(|error| !error.is_empty()));
            if is_json_output {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else if result.is_success || result.error.is_none() {
                println!("Bidirectional sync complete for repo {}", target.repo);
                println!("  Pulled:             {} objects", result.pulled);
                println!("  Pushed:             {} objects", result.pushed);
                println!("  Pull refs updated:  {}", result.pull_refs_updated);
                println!("  Push refs updated:  {}", result.push_refs_updated);
                print_sync_conflicts(&result.conflicts, target.is_push_wins);
                print_string_list_if_any("Warnings", &result.errors);
            } else {
                eprintln!("Sync failed: {}", result.error.as_deref().unwrap_or("unknown error"));
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn federation_fetch(client: &AspenClient, args: FetchArgs, is_json_output: bool) -> Result<()> {
    print_fetch_result(client, &args.peer, args.addr.as_deref(), &args.fed_id, is_json_output).await
}

async fn print_fetch_result(
    client: &AspenClient,
    peer: &str,
    addr: Option<&str>,
    fed_id: &str,
    is_json_output: bool,
) -> Result<()> {
    debug_assert!(!peer.is_empty(), "peer must not be empty");
    debug_assert!(!fed_id.is_empty(), "federated id must not be empty");
    let response = client
        .send(ClientRpcRequest::FederationFetchRefs {
            peer_node_id: peer.to_string(),
            peer_addr: addr.map(|s| s.to_string()),
            fed_id: fed_id.to_string(),
        })
        .await?;

    match response {
        ClientRpcResponse::FederationFetchRefsResult(result) => {
            debug_assert!(result.errors.iter().all(|error| !error.is_empty()));
            debug_assert!(result.is_success || result.error.is_some());
            if is_json_output {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else if result.is_success || result.error.is_none() {
                println!("Fetch {}:", shorten_federated_id(fed_id));
                println!("  Fetched:         {}", result.fetched);
                println!("  Already present: {}", result.already_present);
                print_string_list_if_any("Errors", &result.errors);
            } else {
                eprintln!("Fetch failed: {}", result.error.as_deref().unwrap_or("unknown error"));
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn federation_list_federated(client: &AspenClient, is_json_output: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::ListFederatedRepositories).await?;

    debug_assert!(!is_json_output || is_json_output);

    match response {
        ClientRpcResponse::FederatedRepositories(result) => {
            debug_assert!(usize::try_from(result.count).unwrap_or(usize::MAX) >= result.repositories.len());
            debug_assert!(result.repositories.iter().all(|repo| !repo.repo_id.is_empty() && !repo.fed_id.is_empty()));
            if is_json_output {
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

async fn federation_pull(client: &AspenClient, args: PullArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.repo.is_empty(), "repo must not be empty");
    // Validate: --addr only makes sense with --peer
    if args.addr.is_some() && args.peer.is_none() {
        anyhow::bail!("--addr requires --peer (cold-pull mode)");
    }

    // Build request based on mode: cold-pull (--peer set) vs mirror-pull
    let (mirror_repo_id, peer_node_id, peer_addr, repo_id) = if let Some(ref peer) = args.peer {
        // Cold pull: --repo is the remote repo ID
        (None, Some(peer.clone()), args.addr.clone(), Some(args.repo.clone()))
    } else {
        // Mirror pull: --repo is the local mirror ID
        (Some(args.repo.clone()), None, None, None)
    };

    let response = client
        .send(ClientRpcRequest::FederationPull {
            mirror_repo_id,
            peer_node_id,
            peer_addr,
            repo_id,
        })
        .await?;

    let label = if args.peer.is_some() {
        "remote repo"
    } else {
        "mirror repo"
    };

    match response {
        ClientRpcResponse::FederationPullResult(result) => {
            debug_assert!(result.errors.iter().all(|error| !error.is_empty()));
            debug_assert!(result.is_success || result.error.is_some());
            if is_json_output {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else if result.is_success || result.error.is_none() {
                println!("Pull complete for {} {}", label, args.repo);
                println!("  Fetched:         {}", result.fetched);
                println!("  Already present: {}", result.already_present);
                print_string_list_if_any("Errors", &result.errors);
            } else {
                eprintln!("Pull failed: {}", result.error.as_deref().unwrap_or("unknown error"));
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn federation_push(client: &AspenClient, args: PushArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.peer.is_empty(), "peer must not be empty");
    debug_assert!(!args.repo.is_empty(), "repo must not be empty");
    let response = client
        .send(ClientRpcRequest::FederationPush {
            peer_node_id: args.peer.clone(),
            peer_addr: args.addr.clone(),
            repo_id: args.repo.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::FederationPushResult(result) => {
            debug_assert!(result.errors.iter().all(|error| !error.is_empty()));
            debug_assert!(result.is_success || result.error.is_some());
            if is_json_output {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else if result.is_success {
                println!("Push complete!");
                println!("  Imported:     {}", result.imported);
                println!("  Skipped:      {}", result.skipped);
                println!("  Refs updated: {}", result.refs_updated);
                print_string_list_if_any("Warnings", &result.errors);
            } else {
                eprintln!("Push failed: {}", result.error.as_deref().unwrap_or("unknown error"));
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn federation_grant(client: &AspenClient, args: GrantArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.audience.is_empty(), "audience must not be empty");
    debug_assert!(!args.caps.is_empty(), "capabilities must not be empty");
    let response = client
        .send(ClientRpcRequest::FederationGrant {
            audience: args.audience.clone(),
            capabilities: args.caps.clone(),
            lifetime_secs: args.lifetime,
        })
        .await?;

    match response {
        ClientRpcResponse::FederationGrantResult(result) => {
            debug_assert!(result.token_b64.is_some() == result.is_success || result.token_b64.is_none());
            debug_assert!(result.token_hash.is_some() == result.is_success || result.token_hash.is_none());
            if is_json_output {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else if result.is_success {
                println!("Token issued successfully");
                print_optional_field("Token", result.token_b64.as_deref());
                print_optional_field("Hash", result.token_hash.as_deref());
                println!("\nShare this token with the remote cluster operator.");
                println!("They can import it with: aspen-cli federation token import <token>");
            } else {
                eprintln!("Grant failed: {}", result.error.as_deref().unwrap_or("unknown error"));
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

struct FetchDiscoveredRefsInput<'a> {
    should_fetch_refs: bool,
    peer: &'a str,
    addr: Option<&'a str>,
    resources: &'a [aspen_client_api::SyncPeerResourceInfo],
    is_json_output: bool,
}

async fn fetch_discovered_refs(client: &AspenClient, input: FetchDiscoveredRefsInput<'_>) -> Result<()> {
    if !input.should_fetch_refs {
        return Ok(());
    }
    println!();
    for resource in input.resources {
        if let Some(ref fed_id) = resource.fed_id {
            print_fetch_result(client, input.peer, input.addr, fed_id, input.is_json_output).await?;
        }
    }
    Ok(())
}

fn trusted_status_label(is_trusted: Option<bool>) -> &'static str {
    match is_trusted {
        Some(true) => "yes",
        Some(false) => "no",
        None => "unknown",
    }
}

fn print_sync_resources(resources: &[aspen_client_api::SyncPeerResourceInfo]) {
    if resources.is_empty() {
        println!("  No resources discovered");
        return;
    }
    println!("  Resources: {}", resources.len());
    for resource in resources {
        println!("    - {} (refs: {})", resource.resource_type, resource.ref_count);
    }
}

fn sync_conflict_resolution_label(is_push_wins: bool) -> &'static str {
    if is_push_wins { "local wins" } else { "remote wins" }
}

fn shorten_federated_id(fed_id: &str) -> &str {
    if fed_id.len() > 24 { &fed_id[..24] } else { fed_id }
}

fn print_sync_conflicts(conflicts: &[String], is_push_wins: bool) {
    if conflicts.is_empty() {
        return;
    }
    println!("  Conflicts ({}):  {}", sync_conflict_resolution_label(is_push_wins), conflicts.join(", "));
}

fn print_string_list_if_any(label: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }
    println!("  {label}:");
    for value in values {
        println!("    - {}", value);
    }
}

fn print_optional_field(label: &str, value: Option<&str>) {
    if let Some(value) = value {
        println!("  {label}: {}", value);
    }
}

async fn federation_token_list(client: &AspenClient, is_json_output: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::FederationListTokens).await?;

    debug_assert!(!is_json_output || is_json_output);

    match response {
        ClientRpcResponse::FederationListTokensResult(result) => {
            debug_assert!(result.tokens.iter().all(|token| !token.token_hash.is_empty() && !token.audience.is_empty()));
            debug_assert!(result.error.is_none() || result.tokens.is_empty());
            if is_json_output {
                println!("{}", serde_json::to_string_pretty(&result)?);
                return Ok(());
            }
            if let Some(ref err) = result.error {
                eprintln!("Error: {}", err);
                std::process::exit(1);
            }
            if result.tokens.is_empty() {
                println!("No active federation tokens.");
                return Ok(());
            }
            println!("Active Federation Tokens ({}):", result.tokens.len());
            for t in &result.tokens {
                println!();
                println!("  Hash:         {}", t.token_hash);
                println!("  Audience:     {}", t.audience);
                println!("  Capabilities: {}", t.capabilities);
                println!("  Expires:      {}", format_expiry(t.expires_at));
                println!("  Delegation:   depth {}", t.delegation_depth);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

#[allow(
    ambient_clock,
    reason = "CLI federation token inspection needs current wall-clock time for human expiry formatting"
)]
fn current_unix_time_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn format_expiry(expires_at: u64) -> String {
    let now_secs = current_unix_time_secs();
    if expires_at <= now_secs {
        return "EXPIRED".to_string();
    }

    let remaining_secs = expires_at.saturating_sub(now_secs);
    if remaining_secs < 3600 {
        format!("in {} minutes", remaining_secs / 60)
    } else if remaining_secs < 86400 {
        format!("in {} hours", remaining_secs / 3600)
    } else {
        format!("in {} days", remaining_secs / 86400)
    }
}

async fn federation_token_inspect(args: InspectArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.token_b64.is_empty(), "token must not be empty");
    // Decode without verification — show credential fields
    let credential = aspen_auth::Credential::from_base64(&args.token_b64)
        .map_err(|e| anyhow::anyhow!("failed to decode credential: {}", e))?;

    let token = &credential.token;
    debug_assert!(token.expires_at >= token.issued_at, "token expiry should not predate issue time");
    debug_assert!(credential.proofs.len() <= usize::MAX);

    if is_json_output {
        println!("{}", serde_json::to_string_pretty(&credential)?);
    } else {
        println!("Federation Credential (unverified)");
        println!("  Issuer:       {}", hex::encode(token.issuer));
        println!("  Audience:     {:?}", token.audience);
        println!("  Issued at:    {}", token.issued_at);
        println!("  Expires at:   {} ({})", token.expires_at, format_expiry(token.expires_at));
        println!("  Depth:        {}", token.delegation_depth);
        println!("  Proofs:       {}", credential.proofs.len());
        println!("  Capabilities: {}", token.capabilities.len());
        for cap in &token.capabilities {
            println!("    - {:?}", cap);
        }
    }
    Ok(())
}
