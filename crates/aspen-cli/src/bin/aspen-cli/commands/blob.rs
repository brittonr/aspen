//! Blob storage commands.
//!
//! Commands for content-addressed blob storage with BLAKE3 hashing.
//! Supports adding, retrieving, and managing blobs with protection tags.

use std::io::Read;
use std::path::PathBuf;

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::Outputable;
use crate::output::print_output;

/// Blob storage operations.
#[derive(Subcommand)]
pub enum BlobCommand {
    /// Add a blob from file or data.
    Add(AddArgs),

    /// Get a blob by hash.
    Get(GetArgs),

    /// Check if a blob exists.
    Has(HasArgs),

    /// Get a ticket for sharing a blob.
    Ticket(TicketArgs),

    /// List blobs in the store.
    List(ListArgs),

    /// Protect a blob from garbage collection.
    Protect(ProtectArgs),

    /// Remove protection from a blob.
    Unprotect(UnprotectArgs),

    /// Delete a blob from the store.
    Delete(DeleteArgs),

    /// Download a blob from a remote peer using a ticket.
    Download(DownloadArgs),

    /// Download a blob by hash using DHT discovery.
    ///
    /// Queries the BitTorrent Mainline DHT for providers of the given hash,
    /// then fetches the blob from the first available provider.
    /// Requires the `global-discovery` feature on the server.
    DownloadByHash(DownloadByHashArgs),

    /// Download a blob from a specific provider using DHT discovery.
    ///
    /// Uses the provider's public key to look up their node address in the DHT
    /// via BEP-44 mutable items, then fetches the blob from that provider.
    /// Requires the `global-discovery` feature on the server.
    DownloadByProvider(DownloadByProviderArgs),

    /// Get detailed status information about a blob.
    Status(StatusArgs),

    /// Get replication status for a blob across cluster nodes.
    ReplicationStatus(ReplicationStatusArgs),

    /// Trigger blob replication/repair to ensure sufficient replicas.
    ///
    /// Replicates the specified blob to additional nodes based on the
    /// replication factor. If no target nodes are specified, automatic
    /// placement selects optimal targets.
    Repair(RepairArgs),

    /// Run a full blob repair cycle across the cluster.
    ///
    /// Scans for under-replicated blobs and repairs them in priority order:
    /// 1. Critical (0 replicas)
    /// 2. UnderReplicated (below min_replicas)
    /// 3. Degraded (below replication_factor)
    ///
    /// Returns immediately - repairs happen asynchronously in the background.
    RepairCycle,
}

#[derive(Args)]
pub struct AddArgs {
    /// File path to add (use - for stdin).
    #[arg(conflicts_with = "data")]
    pub file: Option<PathBuf>,

    /// Raw data to add (use --file for files).
    #[arg(long, conflicts_with = "file")]
    pub data: Option<String>,

    /// Optional tag to protect the blob from GC.
    #[arg(long)]
    pub tag: Option<String>,
}

#[derive(Args)]
pub struct GetArgs {
    /// BLAKE3 hash of the blob (hex-encoded).
    pub hash: String,

    /// Output file path (default: stdout).
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Args)]
pub struct HasArgs {
    /// BLAKE3 hash of the blob (hex-encoded).
    pub hash: String,
}

#[derive(Args)]
pub struct TicketArgs {
    /// BLAKE3 hash of the blob (hex-encoded).
    pub hash: String,
}

#[derive(Args)]
pub struct ListArgs {
    /// Maximum number of blobs to return.
    #[arg(long, default_value = "100")]
    pub limit: u32,

    /// Continuation token from previous list.
    #[arg(long)]
    pub token: Option<String>,
}

#[derive(Args)]
pub struct ProtectArgs {
    /// BLAKE3 hash of the blob (hex-encoded).
    pub hash: String,

    /// Tag name for the protection.
    #[arg(long)]
    pub tag: String,
}

#[derive(Args)]
pub struct UnprotectArgs {
    /// Tag name to remove.
    pub tag: String,
}

#[derive(Args)]
pub struct DeleteArgs {
    /// BLAKE3 hash of the blob (hex-encoded).
    pub hash: String,

    /// Force deletion even if protected.
    #[arg(long)]
    pub force: bool,
}

#[derive(Args)]
pub struct DownloadArgs {
    /// Blob ticket from remote peer.
    #[arg(value_name = "BLOB_TICKET")]
    pub blob_ticket: String,

    /// Optional tag to protect the downloaded blob from GC.
    #[arg(long)]
    pub tag: Option<String>,
}

#[derive(Args)]
pub struct DownloadByHashArgs {
    /// BLAKE3 hash of the blob to download (hex-encoded).
    pub hash: String,

    /// Optional tag to protect the downloaded blob from GC.
    #[arg(long)]
    pub tag: Option<String>,
}

#[derive(Args)]
pub struct DownloadByProviderArgs {
    /// BLAKE3 hash of the blob to download (hex-encoded).
    pub hash: String,

    /// Public key of the provider node (hex-encoded or zbase32).
    pub provider: String,

    /// Optional tag to protect the downloaded blob from GC.
    #[arg(long)]
    pub tag: Option<String>,
}

#[derive(Args)]
pub struct StatusArgs {
    /// BLAKE3 hash of the blob (hex-encoded).
    pub hash: String,
}

#[derive(Args)]
pub struct ReplicationStatusArgs {
    /// BLAKE3 hash of the blob (hex-encoded).
    pub hash: String,
}

#[derive(Args)]
pub struct RepairArgs {
    /// BLAKE3 hash of the blob to repair (hex-encoded).
    pub hash: String,

    /// Target node IDs for replication (comma-separated).
    ///
    /// If not specified, automatic placement selects optimal targets
    /// based on node health, weights, and failure domains.
    #[arg(long)]
    pub targets: Option<String>,

    /// Replication factor override (0 = use default policy).
    #[arg(long, default_value = "0")]
    pub replication_factor: u32,
}

/// Add blob output.
pub struct AddBlobOutput {
    pub success: bool,
    pub hash: Option<String>,
    pub size: Option<u64>,
    pub was_new: Option<bool>,
    pub error: Option<String>,
}

impl Outputable for AddBlobOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "success": self.success,
            "hash": self.hash,
            "size": self.size,
            "was_new": self.was_new,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.success {
            let hash = self.hash.as_deref().unwrap_or("unknown");
            let size = self.size.unwrap_or(0);
            let status = if self.was_new.unwrap_or(false) {
                "added"
            } else {
                "exists"
            };
            format!("{} ({} bytes, {})", hash, size, status)
        } else {
            format!("Add failed: {}", self.error.as_deref().unwrap_or("unknown error"))
        }
    }
}

/// Get blob output.
pub struct GetBlobOutput {
    pub found: bool,
    pub data: Option<Vec<u8>>,
    pub error: Option<String>,
}

impl Outputable for GetBlobOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "found": self.found,
            "size": self.data.as_ref().map(|d| d.len()),
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.found {
            match &self.data {
                Some(data) => {
                    // Try to display as UTF-8
                    match String::from_utf8(data.clone()) {
                        Ok(s) => s,
                        Err(_) => format!("<binary: {} bytes>", data.len()),
                    }
                }
                None => "Blob found but no data".to_string(),
            }
        } else {
            format!("Blob not found: {}", self.error.as_deref().unwrap_or(""))
        }
    }
}

/// Has blob output.
pub struct HasBlobOutput {
    pub hash: String,
    pub exists: bool,
    pub error: Option<String>,
}

impl Outputable for HasBlobOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "hash": self.hash,
            "exists": self.exists,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.exists { "exists" } else { "not found" }.to_string()
    }
}

/// Blob ticket output.
pub struct BlobTicketOutput {
    pub success: bool,
    pub ticket: Option<String>,
    pub error: Option<String>,
}

impl Outputable for BlobTicketOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "success": self.success,
            "ticket": self.ticket,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.success {
            self.ticket.clone().unwrap_or_else(|| "no ticket".to_string())
        } else {
            format!("Ticket failed: {}", self.error.as_deref().unwrap_or("unknown error"))
        }
    }
}

/// List blobs output.
pub struct ListBlobsOutput {
    pub blobs: Vec<BlobEntry>,
    pub count: u32,
    pub has_more: bool,
    pub continuation_token: Option<String>,
    pub error: Option<String>,
}

pub struct BlobEntry {
    pub hash: String,
    pub size: u64,
}

impl Outputable for ListBlobsOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "blobs": self.blobs.iter().map(|b| {
                serde_json::json!({
                    "hash": b.hash,
                    "size": b.size
                })
            }).collect::<Vec<_>>(),
            "count": self.count,
            "has_more": self.has_more,
            "continuation_token": self.continuation_token,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if let Some(ref err) = self.error {
            return format!("List failed: {}", err);
        }

        if self.blobs.is_empty() {
            return "No blobs found".to_string();
        }

        let mut output = format!("Blobs ({})\n", self.count);
        output.push_str("Hash                                                             | Size\n");
        output.push_str("-----------------------------------------------------------------+----------\n");

        for blob in &self.blobs {
            output.push_str(&format!("{} | {:>8}\n", &blob.hash[..64.min(blob.hash.len())], blob.size));
        }

        if self.has_more {
            if let Some(token) = &self.continuation_token {
                output.push_str(&format!("\nMore results available. Use --token {}", token));
            }
        }

        output
    }
}

/// Protect/Unprotect blob output.
pub struct ProtectBlobOutput {
    pub operation: String,
    pub success: bool,
    pub error: Option<String>,
}

impl Outputable for ProtectBlobOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "operation": self.operation,
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

/// Delete blob output.
pub struct DeleteBlobOutput {
    pub success: bool,
    pub error: Option<String>,
}

impl Outputable for DeleteBlobOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "success": self.success,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.success {
            "deleted".to_string()
        } else {
            format!("Delete failed: {}", self.error.as_deref().unwrap_or("unknown error"))
        }
    }
}

/// Download blob output.
pub struct DownloadBlobOutput {
    pub success: bool,
    pub hash: Option<String>,
    pub size: Option<u64>,
    pub error: Option<String>,
}

impl Outputable for DownloadBlobOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "success": self.success,
            "hash": self.hash,
            "size": self.size,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.success {
            let hash = self.hash.as_deref().unwrap_or("unknown");
            let size = self.size.unwrap_or(0);
            format!("Downloaded {} ({} bytes)", hash, size)
        } else {
            format!("Download failed: {}", self.error.as_deref().unwrap_or("unknown error"))
        }
    }
}

/// Blob status output.
pub struct BlobStatusOutput {
    pub found: bool,
    pub hash: Option<String>,
    pub size: Option<u64>,
    pub complete: Option<bool>,
    pub tags: Option<Vec<String>>,
    pub error: Option<String>,
}

impl Outputable for BlobStatusOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "found": self.found,
            "hash": self.hash,
            "size": self.size,
            "complete": self.complete,
            "tags": self.tags,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if !self.found {
            return format!(
                "Blob not found: {}",
                self.error.as_deref().unwrap_or(self.hash.as_deref().unwrap_or("unknown"))
            );
        }

        let hash = self.hash.as_deref().unwrap_or("unknown");
        let size = self.size.map(|s| format!("{} bytes", s)).unwrap_or_else(|| "unknown size".to_string());
        let complete = if self.complete.unwrap_or(false) {
            "complete"
        } else {
            "incomplete"
        };
        let tags = self
            .tags
            .as_ref()
            .map(|t| if t.is_empty() { "none".to_string() } else { t.join(", ") })
            .unwrap_or_else(|| "unknown".to_string());

        format!("Hash: {}\nSize: {}\nStatus: {}\nTags: {}", hash, size, complete, tags)
    }
}

/// Blob replication status output.
pub struct BlobReplicationStatusOutput {
    pub found: bool,
    pub hash: Option<String>,
    pub size: Option<u64>,
    pub replica_nodes: Option<Vec<u64>>,
    pub replication_factor: Option<u32>,
    pub min_replicas: Option<u32>,
    pub status: Option<String>,
    pub replicas_needed: Option<u32>,
    pub updated_at: Option<String>,
    pub error: Option<String>,
}

impl Outputable for BlobReplicationStatusOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "found": self.found,
            "hash": self.hash,
            "size": self.size,
            "replica_nodes": self.replica_nodes,
            "replication_factor": self.replication_factor,
            "min_replicas": self.min_replicas,
            "status": self.status,
            "replicas_needed": self.replicas_needed,
            "updated_at": self.updated_at,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if !self.found {
            return format!(
                "Replication metadata not found: {}",
                self.error.as_deref().unwrap_or(self.hash.as_deref().unwrap_or("unknown"))
            );
        }

        let hash = self.hash.as_deref().unwrap_or("unknown");
        let size = self.size.map(|s| format!("{} bytes", s)).unwrap_or_else(|| "unknown".to_string());
        let status = self.status.as_deref().unwrap_or("unknown");
        let factor = self.replication_factor.unwrap_or(0);
        let min = self.min_replicas.unwrap_or(0);
        let replicas = self
            .replica_nodes
            .as_ref()
            .map(|nodes| {
                if nodes.is_empty() {
                    "none".to_string()
                } else {
                    nodes.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(", ")
                }
            })
            .unwrap_or_else(|| "unknown".to_string());
        let updated = self.updated_at.as_deref().unwrap_or("unknown");

        let mut output = format!("Hash: {}\nSize: {}\n", hash, size);
        output.push_str(&format!(
            "Replication Factor: {}/{}\n",
            self.replica_nodes.as_ref().map(|n| n.len()).unwrap_or(0),
            factor
        ));
        output.push_str(&format!("Min Replicas: {}\n", min));
        output.push_str(&format!("Status: {}\n", status));
        if let Some(needed) = self.replicas_needed {
            if needed > 0 {
                output.push_str(&format!("Replicas Needed: {}\n", needed));
            }
        }
        output.push_str(&format!("Replica Nodes: [{}]\n", replicas));
        output.push_str(&format!("Last Updated: {}", updated));

        output
    }
}

/// Blob repair output.
pub struct BlobRepairOutput {
    pub success: bool,
    pub hash: Option<String>,
    pub successful_nodes: Option<Vec<u64>>,
    pub failed_nodes: Option<Vec<(u64, String)>>,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
}

impl Outputable for BlobRepairOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "success": self.success,
            "hash": self.hash,
            "successful_nodes": self.successful_nodes,
            "failed_nodes": self.failed_nodes,
            "duration_ms": self.duration_ms,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.success {
            let hash = self.hash.as_deref().unwrap_or("unknown");
            let successful = self.successful_nodes.as_ref().map(|n| n.len()).unwrap_or(0);
            let failed = self.failed_nodes.as_ref().map(|n| n.len()).unwrap_or(0);
            let duration = self.duration_ms.map(|d| format!("{}ms", d)).unwrap_or_else(|| "unknown".to_string());

            let mut output = format!("Repair successful: {} ({})\n", hash, duration);
            output.push_str(&format!("Successful nodes: {}\n", successful));
            if failed > 0 {
                output.push_str(&format!("Failed nodes: {}", failed));
            }
            output
        } else {
            format!("Repair failed: {}", self.error.as_deref().unwrap_or("unknown error"))
        }
    }
}

/// Blob repair cycle output.
pub struct BlobRepairCycleOutput {
    pub success: bool,
    pub error: Option<String>,
}

impl Outputable for BlobRepairCycleOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "success": self.success,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.success {
            "Repair cycle initiated successfully. Repairs running in background.".to_string()
        } else {
            format!("Repair cycle failed: {}", self.error.as_deref().unwrap_or("unknown error"))
        }
    }
}

impl BlobCommand {
    /// Execute the blob command.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            BlobCommand::Add(args) => blob_add(client, args, json).await,
            BlobCommand::Get(args) => blob_get(client, args, json).await,
            BlobCommand::Has(args) => blob_has(client, args, json).await,
            BlobCommand::Ticket(args) => blob_ticket(client, args, json).await,
            BlobCommand::List(args) => blob_list(client, args, json).await,
            BlobCommand::Protect(args) => blob_protect(client, args, json).await,
            BlobCommand::Unprotect(args) => blob_unprotect(client, args, json).await,
            BlobCommand::Delete(args) => blob_delete(client, args, json).await,
            BlobCommand::Download(args) => blob_download(client, args, json).await,
            BlobCommand::DownloadByHash(args) => blob_download_by_hash(client, args, json).await,
            BlobCommand::DownloadByProvider(args) => blob_download_by_provider(client, args, json).await,
            BlobCommand::Status(args) => blob_status(client, args, json).await,
            BlobCommand::ReplicationStatus(args) => blob_replication_status(client, args, json).await,
            BlobCommand::Repair(args) => blob_repair(client, args, json).await,
            BlobCommand::RepairCycle => blob_repair_cycle(client, json).await,
        }
    }
}

async fn blob_add(client: &AspenClient, args: AddArgs, json: bool) -> Result<()> {
    // Get data from file, stdin, or --data argument
    let data = if let Some(ref path) = args.file {
        if path.as_os_str() == "-" {
            let mut buf = Vec::new();
            std::io::stdin().read_to_end(&mut buf)?;
            buf
        } else {
            std::fs::read(path)?
        }
    } else if let Some(ref data_str) = args.data {
        data_str.as_bytes().to_vec()
    } else {
        anyhow::bail!("must specify --file or --data");
    };

    let response = client.send(ClientRpcRequest::AddBlob { data, tag: args.tag }).await?;

    match response {
        ClientRpcResponse::AddBlobResult(result) => {
            let output = AddBlobOutput {
                success: result.success,
                hash: result.hash,
                size: result.size,
                was_new: result.was_new,
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

async fn blob_get(client: &AspenClient, args: GetArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::GetBlob { hash: args.hash }).await?;

    match response {
        ClientRpcResponse::GetBlobResult(result) => {
            // If output file specified and blob found with data, write directly
            if let (Some(output_path), true, Some(data)) = (&args.output, result.found, &result.data) {
                std::fs::write(output_path, data)?;
                if !json {
                    println!("Wrote {} bytes to {}", data.len(), output_path.display());
                }
                return Ok(());
            }

            let output = GetBlobOutput {
                found: result.found,
                data: result.data,
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

async fn blob_has(client: &AspenClient, args: HasArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::HasBlob {
            hash: args.hash.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::HasBlobResult(result) => {
            let output = HasBlobOutput {
                hash: args.hash,
                exists: result.exists,
                error: result.error,
            };
            print_output(&output, json);
            if !result.exists {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn blob_ticket(client: &AspenClient, args: TicketArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::GetBlobTicket { hash: args.hash }).await?;

    match response {
        ClientRpcResponse::GetBlobTicketResult(result) => {
            let output = BlobTicketOutput {
                success: result.success,
                ticket: result.ticket,
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

async fn blob_list(client: &AspenClient, args: ListArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::ListBlobs {
            limit: args.limit,
            continuation_token: args.token,
        })
        .await?;

    match response {
        ClientRpcResponse::ListBlobsResult(result) => {
            let blobs = result
                .blobs
                .into_iter()
                .map(|b| BlobEntry {
                    hash: b.hash,
                    size: b.size,
                })
                .collect();

            let output = ListBlobsOutput {
                blobs,
                count: result.count,
                has_more: result.has_more,
                continuation_token: result.continuation_token,
                error: result.error,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn blob_protect(client: &AspenClient, args: ProtectArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::ProtectBlob {
            hash: args.hash,
            tag: args.tag,
        })
        .await?;

    match response {
        ClientRpcResponse::ProtectBlobResult(result) => {
            let output = ProtectBlobOutput {
                operation: "protect".to_string(),
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

async fn blob_unprotect(client: &AspenClient, args: UnprotectArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::UnprotectBlob { tag: args.tag }).await?;

    match response {
        ClientRpcResponse::UnprotectBlobResult(result) => {
            let output = ProtectBlobOutput {
                operation: "unprotect".to_string(),
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

async fn blob_delete(client: &AspenClient, args: DeleteArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::DeleteBlob {
            hash: args.hash,
            force: args.force,
        })
        .await?;

    match response {
        ClientRpcResponse::DeleteBlobResult(result) => {
            let output = DeleteBlobOutput {
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

async fn blob_download(client: &AspenClient, args: DownloadArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::DownloadBlob {
            ticket: args.blob_ticket,
            tag: args.tag,
        })
        .await?;

    match response {
        ClientRpcResponse::DownloadBlobResult(result) => {
            let output = DownloadBlobOutput {
                success: result.success,
                hash: result.hash,
                size: result.size,
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

async fn blob_download_by_hash(client: &AspenClient, args: DownloadByHashArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::DownloadBlobByHash {
            hash: args.hash,
            tag: args.tag,
        })
        .await?;

    match response {
        ClientRpcResponse::DownloadBlobByHashResult(result) => {
            let output = DownloadBlobOutput {
                success: result.success,
                hash: result.hash,
                size: result.size,
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

async fn blob_download_by_provider(client: &AspenClient, args: DownloadByProviderArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::DownloadBlobByProvider {
            hash: args.hash,
            provider: args.provider,
            tag: args.tag,
        })
        .await?;

    match response {
        ClientRpcResponse::DownloadBlobByProviderResult(result) => {
            let output = DownloadBlobOutput {
                success: result.success,
                hash: result.hash,
                size: result.size,
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

async fn blob_status(client: &AspenClient, args: StatusArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::GetBlobStatus { hash: args.hash }).await?;

    match response {
        ClientRpcResponse::GetBlobStatusResult(result) => {
            let output = BlobStatusOutput {
                found: result.found,
                hash: result.hash,
                size: result.size,
                complete: result.complete,
                tags: result.tags,
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

async fn blob_replication_status(client: &AspenClient, args: ReplicationStatusArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::GetBlobReplicationStatus { hash: args.hash }).await?;

    match response {
        ClientRpcResponse::GetBlobReplicationStatusResult(result) => {
            let output = BlobReplicationStatusOutput {
                found: result.found,
                hash: result.hash,
                size: result.size,
                replica_nodes: result.replica_nodes,
                replication_factor: result.replication_factor,
                min_replicas: result.min_replicas,
                status: result.status,
                replicas_needed: result.replicas_needed,
                updated_at: result.updated_at,
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

async fn blob_repair(client: &AspenClient, args: RepairArgs, json: bool) -> Result<()> {
    // Parse target nodes if provided
    let target_nodes: Vec<u64> = if let Some(targets_str) = args.targets {
        targets_str
            .split(',')
            .map(|s| s.trim().parse::<u64>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| anyhow::anyhow!("invalid target node ID: {}", e))?
    } else {
        Vec::new() // Automatic placement
    };

    let response = client
        .send(ClientRpcRequest::TriggerBlobReplication {
            hash: args.hash.clone(),
            target_nodes,
            replication_factor: args.replication_factor,
        })
        .await?;

    match response {
        ClientRpcResponse::TriggerBlobReplicationResult(result) => {
            let output = BlobRepairOutput {
                success: result.success,
                hash: result.hash,
                successful_nodes: result.successful_nodes,
                failed_nodes: result.failed_nodes,
                duration_ms: result.duration_ms,
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

async fn blob_repair_cycle(client: &AspenClient, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::RunBlobRepairCycle).await?;

    match response {
        ClientRpcResponse::RunBlobRepairCycleResult(result) => {
            let output = BlobRepairCycleOutput {
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
