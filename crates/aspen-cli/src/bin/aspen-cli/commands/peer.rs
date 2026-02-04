//! Peer cluster federation commands.
//!
//! Commands for managing multi-cluster federation with priority-based
//! conflict resolution and key filtering.

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use clap::Args;
use clap::Subcommand;
use clap::ValueEnum;

use crate::client::AspenClient;
use crate::output::Outputable;
use crate::output::print_output;

/// Peer cluster federation operations.
#[derive(Subcommand)]
pub enum PeerCommand {
    /// Add a peer cluster to sync with.
    Add(AddArgs),

    /// Remove a peer cluster subscription.
    Remove(RemoveArgs),

    /// List all peer cluster subscriptions.
    List,

    /// Get sync status for a specific peer cluster.
    Status(StatusArgs),

    /// Update the subscription filter for a peer cluster.
    Filter(FilterArgs),

    /// Update the priority for a peer cluster.
    Priority(PriorityArgs),

    /// Enable or disable a peer cluster subscription.
    Enable(EnableArgs),

    /// Disable a peer cluster subscription.
    Disable(DisableArgs),

    /// Get the origin metadata for a key.
    Origin(OriginArgs),
}

#[derive(Args)]
pub struct AddArgs {
    /// Serialized AspenDocsTicket from the peer cluster.
    pub ticket: String,
}

#[derive(Args)]
pub struct RemoveArgs {
    /// Cluster ID of the peer to remove.
    pub cluster_id: String,
}

#[derive(Args)]
pub struct StatusArgs {
    /// Cluster ID of the peer.
    pub cluster_id: String,
}

/// Filter type values.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum FilterType {
    /// Sync all keys.
    Full,
    /// Only sync keys matching prefixes.
    Include,
    /// Sync all keys except those matching prefixes.
    Exclude,
}

impl FilterType {
    fn as_str(&self) -> &'static str {
        match self {
            FilterType::Full => "full",
            FilterType::Include => "include",
            FilterType::Exclude => "exclude",
        }
    }
}

#[derive(Args)]
pub struct FilterArgs {
    /// Cluster ID of the peer.
    pub cluster_id: String,

    /// Filter type.
    #[arg(long, value_enum, name = "type")]
    pub filter_type: FilterType,

    /// Prefixes for include/exclude filters (comma-separated).
    #[arg(long)]
    pub prefixes: Option<String>,
}

#[derive(Args)]
pub struct PriorityArgs {
    /// Cluster ID of the peer.
    pub cluster_id: String,

    /// New priority (0 = highest, lower wins conflicts).
    pub priority: u32,
}

#[derive(Args)]
pub struct EnableArgs {
    /// Cluster ID of the peer.
    pub cluster_id: String,
}

#[derive(Args)]
pub struct DisableArgs {
    /// Cluster ID of the peer.
    pub cluster_id: String,
}

#[derive(Args)]
pub struct OriginArgs {
    /// The key to look up origin for.
    pub key: String,
}

/// Add peer cluster output.
pub struct AddPeerOutput {
    pub success: bool,
    pub cluster_id: Option<String>,
    pub priority: Option<u32>,
    pub error: Option<String>,
}

impl Outputable for AddPeerOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "success": self.success,
            "cluster_id": self.cluster_id,
            "priority": self.priority,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.success {
            format!(
                "Added peer cluster: {} (priority: {})",
                self.cluster_id.as_deref().unwrap_or("unknown"),
                self.priority.unwrap_or(0)
            )
        } else {
            format!("Add failed: {}", self.error.as_deref().unwrap_or("unknown error"))
        }
    }
}

/// Remove peer cluster output.
pub struct RemovePeerOutput {
    pub success: bool,
    pub cluster_id: String,
    pub error: Option<String>,
}

impl Outputable for RemovePeerOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "success": self.success,
            "cluster_id": self.cluster_id,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.success {
            format!("Removed peer cluster: {}", self.cluster_id)
        } else {
            format!("Remove failed: {}", self.error.as_deref().unwrap_or("unknown error"))
        }
    }
}

/// List peer clusters output.
pub struct ListPeersOutput {
    pub peers: Vec<PeerInfo>,
    pub count: u32,
    pub error: Option<String>,
}

pub struct PeerInfo {
    pub cluster_id: String,
    pub name: String,
    pub state: String,
    pub priority: u32,
    pub enabled: bool,
    pub sync_count: u64,
    pub failure_count: u64,
}

impl Outputable for ListPeersOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "peers": self.peers.iter().map(|p| {
                serde_json::json!({
                    "cluster_id": p.cluster_id,
                    "name": p.name,
                    "state": p.state,
                    "priority": p.priority,
                    "enabled": p.enabled,
                    "sync_count": p.sync_count,
                    "failure_count": p.failure_count
                })
            }).collect::<Vec<_>>(),
            "count": self.count,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if let Some(ref err) = self.error {
            return format!("List failed: {}", err);
        }

        if self.peers.is_empty() {
            return "No peer clusters configured".to_string();
        }

        let mut output = format!("Peer Clusters ({})\n", self.count);
        output
            .push_str("Cluster ID               | Name             | State      | Pri | Enabled | Syncs | Failures\n");
        output
            .push_str("-------------------------+------------------+------------+-----+---------+-------+----------\n");

        for peer in &self.peers {
            output.push_str(&format!(
                "{:24} | {:16} | {:10} | {:3} | {:7} | {:>5} | {:>8}\n",
                &peer.cluster_id[..24.min(peer.cluster_id.len())],
                &peer.name[..16.min(peer.name.len())],
                &peer.state,
                peer.priority,
                if peer.enabled { "yes" } else { "no" },
                peer.sync_count,
                peer.failure_count
            ));
        }

        output
    }
}

/// Peer status output.
pub struct PeerStatusOutput {
    pub found: bool,
    pub cluster_id: String,
    pub state: String,
    pub syncing: bool,
    pub entries_received: u64,
    pub entries_imported: u64,
    pub entries_skipped: u64,
    pub entries_filtered: u64,
    pub error: Option<String>,
}

impl Outputable for PeerStatusOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "found": self.found,
            "cluster_id": self.cluster_id,
            "state": self.state,
            "syncing": self.syncing,
            "entries_received": self.entries_received,
            "entries_imported": self.entries_imported,
            "entries_skipped": self.entries_skipped,
            "entries_filtered": self.entries_filtered,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if !self.found {
            return format!("Peer cluster not found: {}", self.cluster_id);
        }

        if let Some(ref err) = self.error {
            return format!("Status failed: {}", err);
        }

        format!(
            "Peer: {}\n  State:           {}\n  Syncing:         {}\n  Entries received: {}\n  Entries imported: {}\n  Entries skipped:  {}\n  Entries filtered: {}",
            self.cluster_id,
            self.state,
            if self.syncing { "yes" } else { "no" },
            self.entries_received,
            self.entries_imported,
            self.entries_skipped,
            self.entries_filtered
        )
    }
}

/// Simple success output.
pub struct PeerSuccessOutput {
    pub operation: String,
    pub cluster_id: String,
    pub success: bool,
    pub error: Option<String>,
}

impl Outputable for PeerSuccessOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "operation": self.operation,
            "cluster_id": self.cluster_id,
            "success": self.success,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.success {
            "OK".to_string()
        } else {
            format!("{} failed: {}", self.operation, self.error.as_deref().unwrap_or("unknown error"))
        }
    }
}

/// Key origin output.
pub struct KeyOriginOutput {
    pub found: bool,
    pub key: String,
    pub cluster_id: Option<String>,
    pub priority: Option<u32>,
    pub timestamp_secs: Option<u64>,
    pub is_local: Option<bool>,
}

impl Outputable for KeyOriginOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "found": self.found,
            "key": self.key,
            "cluster_id": self.cluster_id,
            "priority": self.priority,
            "timestamp_secs": self.timestamp_secs,
            "is_local": self.is_local
        })
    }

    fn to_human(&self) -> String {
        if !self.found {
            return format!("Key '{}' has no origin metadata", self.key);
        }

        let origin = if self.is_local.unwrap_or(false) {
            "local".to_string()
        } else {
            self.cluster_id.clone().unwrap_or_else(|| "unknown".to_string())
        };

        format!(
            "Key: {}\n  Origin: {}\n  Priority: {}\n  Timestamp: {}",
            self.key,
            origin,
            self.priority.unwrap_or(0),
            self.timestamp_secs.unwrap_or(0)
        )
    }
}

impl PeerCommand {
    /// Execute the peer command.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            PeerCommand::Add(args) => peer_add(client, args, json).await,
            PeerCommand::Remove(args) => peer_remove(client, args, json).await,
            PeerCommand::List => peer_list(client, json).await,
            PeerCommand::Status(args) => peer_status(client, args, json).await,
            PeerCommand::Filter(args) => peer_filter(client, args, json).await,
            PeerCommand::Priority(args) => peer_priority(client, args, json).await,
            PeerCommand::Enable(args) => peer_enable(client, args, json).await,
            PeerCommand::Disable(args) => peer_disable(client, args, json).await,
            PeerCommand::Origin(args) => peer_origin(client, args, json).await,
        }
    }
}

async fn peer_add(client: &AspenClient, args: AddArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::AddPeerCluster { ticket: args.ticket }).await?;

    match response {
        ClientRpcResponse::AddPeerClusterResult(result) => {
            let output = AddPeerOutput {
                success: result.success,
                cluster_id: result.cluster_id,
                priority: result.priority,
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

async fn peer_remove(client: &AspenClient, args: RemoveArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::RemovePeerCluster {
            cluster_id: args.cluster_id.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::RemovePeerClusterResult(result) => {
            let output = RemovePeerOutput {
                success: result.success,
                cluster_id: args.cluster_id,
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

async fn peer_list(client: &AspenClient, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::ListPeerClusters).await?;

    match response {
        ClientRpcResponse::ListPeerClustersResult(result) => {
            let peers = result
                .peers
                .into_iter()
                .map(|p| PeerInfo {
                    cluster_id: p.cluster_id,
                    name: p.name,
                    state: p.state,
                    priority: p.priority,
                    enabled: p.enabled,
                    sync_count: p.sync_count,
                    failure_count: p.failure_count,
                })
                .collect();

            let output = ListPeersOutput {
                peers,
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

async fn peer_status(client: &AspenClient, args: StatusArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::GetPeerClusterStatus {
            cluster_id: args.cluster_id.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::PeerClusterStatus(result) => {
            let output = PeerStatusOutput {
                found: result.found,
                cluster_id: args.cluster_id,
                state: result.state,
                syncing: result.syncing,
                entries_received: result.entries_received,
                entries_imported: result.entries_imported,
                entries_skipped: result.entries_skipped,
                entries_filtered: result.entries_filtered,
                error: result.error,
            };
            print_output(&output, json);
            if !result.found {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn peer_filter(client: &AspenClient, args: FilterArgs, json: bool) -> Result<()> {
    // Convert comma-separated prefixes to JSON array if provided
    let prefixes_json = match args.prefixes {
        Some(ref prefixes) if !prefixes.is_empty() => {
            let prefixes: Vec<&str> = prefixes.split(',').map(|p| p.trim()).collect();
            Some(serde_json::to_string(&prefixes)?)
        }
        _ => None,
    };

    let response = client
        .send(ClientRpcRequest::UpdatePeerClusterFilter {
            cluster_id: args.cluster_id.clone(),
            filter_type: args.filter_type.as_str().to_string(),
            prefixes: prefixes_json,
        })
        .await?;

    match response {
        ClientRpcResponse::UpdatePeerClusterFilterResult(result) => {
            let output = PeerSuccessOutput {
                operation: "filter".to_string(),
                cluster_id: args.cluster_id,
                success: result.success,
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

async fn peer_priority(client: &AspenClient, args: PriorityArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::UpdatePeerClusterPriority {
            cluster_id: args.cluster_id.clone(),
            priority: args.priority,
        })
        .await?;

    match response {
        ClientRpcResponse::UpdatePeerClusterPriorityResult(result) => {
            let output = PeerSuccessOutput {
                operation: "priority".to_string(),
                cluster_id: args.cluster_id,
                success: result.success,
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

async fn peer_enable(client: &AspenClient, args: EnableArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::SetPeerClusterEnabled {
            cluster_id: args.cluster_id.clone(),
            enabled: true,
        })
        .await?;

    match response {
        ClientRpcResponse::SetPeerClusterEnabledResult(result) => {
            let output = PeerSuccessOutput {
                operation: "enable".to_string(),
                cluster_id: args.cluster_id,
                success: result.success,
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

async fn peer_disable(client: &AspenClient, args: DisableArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::SetPeerClusterEnabled {
            cluster_id: args.cluster_id.clone(),
            enabled: false,
        })
        .await?;

    match response {
        ClientRpcResponse::SetPeerClusterEnabledResult(result) => {
            let output = PeerSuccessOutput {
                operation: "disable".to_string(),
                cluster_id: args.cluster_id,
                success: result.success,
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

async fn peer_origin(client: &AspenClient, args: OriginArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::GetKeyOrigin { key: args.key.clone() }).await?;

    match response {
        ClientRpcResponse::KeyOriginResult(result) => {
            let output = KeyOriginOutput {
                found: result.found,
                key: args.key,
                cluster_id: result.cluster_id,
                priority: result.priority,
                timestamp_secs: result.timestamp_secs,
                is_local: result.is_local,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}
