//! Blob storage commands.
//!
//! Commands for content-addressed blob storage with BLAKE3 hashing.
//! Supports adding, retrieving, and managing blobs with protection tags.

use std::io::Read;
use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::Outputable;
use crate::output::print_output;
use aspen::client_rpc::ClientRpcRequest;
use aspen::client_rpc::ClientRpcResponse;

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

        if self.has_more && self.continuation_token.is_some() {
            output.push_str(&format!(
                "\nMore results available. Use --token {}",
                self.continuation_token.as_ref().unwrap()
            ));
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
            if args.output.is_some() && result.found && result.data.is_some() {
                let output_path = args.output.as_ref().unwrap();
                let data = result.data.as_ref().unwrap();
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
