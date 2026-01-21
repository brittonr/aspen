//! Nix binary cache commands.
//!
//! Commands for querying and downloading from the distributed
//! Nix binary cache built on top of iroh-blobs storage.

use anyhow::Result;
use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;
use clap::Args;
use clap::Subcommand;
use serde_json::json;

use crate::client::AspenClient;
use crate::output::Outputable;
use crate::output::print_output;

/// Nix binary cache operations.
#[derive(Subcommand)]
pub enum CacheCommand {
    /// Query the cache for a store path.
    Query(QueryArgs),

    /// Get cache statistics.
    Stats,

    /// Get download ticket for a store path.
    Download(DownloadArgs),
}

#[derive(Args)]
pub struct QueryArgs {
    /// Store path or store hash to query.
    ///
    /// Can be a full store path (e.g., /nix/store/abc...-name)
    /// or just the hash part (e.g., abc...).
    pub store_path: String,
}

#[derive(Args)]
pub struct DownloadArgs {
    /// Store path or store hash to download.
    pub store_path: String,

    /// Output the blob ticket only (for scripting).
    #[arg(long)]
    pub ticket_only: bool,
}

/// Cache query output.
pub struct CacheQueryOutput {
    pub found: bool,
    pub store_path: Option<String>,
    pub store_hash: Option<String>,
    pub blob_hash: Option<String>,
    pub nar_size: Option<u64>,
    pub nar_hash: Option<String>,
    pub references: Option<Vec<String>>,
    pub deriver: Option<String>,
    pub error: Option<String>,
}

impl Outputable for CacheQueryOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "found": self.found,
            "store_path": self.store_path,
            "store_hash": self.store_hash,
            "blob_hash": self.blob_hash,
            "nar_size": self.nar_size,
            "nar_hash": self.nar_hash,
            "references": self.references,
            "deriver": self.deriver,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if !self.found {
            return format!(
                "Store path not found in cache{}",
                self.error.as_ref().map(|e| format!(": {}", e)).unwrap_or_default()
            );
        }

        let mut output = String::new();
        if let Some(ref path) = self.store_path {
            output.push_str(&format!("StorePath: {}\n", path));
        }
        if let Some(ref hash) = self.store_hash {
            output.push_str(&format!("StoreHash: {}\n", hash));
        }
        if let Some(ref hash) = self.blob_hash {
            output.push_str(&format!("BlobHash: {}\n", hash));
        }
        if let Some(size) = self.nar_size {
            output.push_str(&format!("NarSize: {}\n", format_size(size)));
        }
        if let Some(ref hash) = self.nar_hash {
            output.push_str(&format!("NarHash: {}\n", hash));
        }
        if let Some(ref refs) = self.references {
            if !refs.is_empty() {
                output.push_str("References:\n");
                for r in refs {
                    output.push_str(&format!("  {}\n", r));
                }
            }
        }
        if let Some(ref d) = self.deriver {
            output.push_str(&format!("Deriver: {}\n", d));
        }

        output
    }
}

/// Cache statistics output.
pub struct CacheStatsOutput {
    pub total_entries: u64,
    pub total_nar_bytes: u64,
    pub query_hits: u64,
    pub query_misses: u64,
    pub node_id: u64,
    pub error: Option<String>,
}

impl Outputable for CacheStatsOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "total_entries": self.total_entries,
            "total_nar_bytes": self.total_nar_bytes,
            "query_hits": self.query_hits,
            "query_misses": self.query_misses,
            "node_id": self.node_id,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if let Some(ref e) = self.error {
            return format!("Error: {}", e);
        }

        let hit_rate = if self.query_hits + self.query_misses > 0 {
            (self.query_hits as f64 / (self.query_hits + self.query_misses) as f64) * 100.0
        } else {
            0.0
        };

        format!(
            "Cache Statistics (Node {})\n\
             ────────────────────────────────\n\
             Total entries: {}\n\
             Total NAR size: {}\n\
             Query hits: {}\n\
             Query misses: {}\n\
             Hit rate: {:.1}%\n",
            self.node_id,
            self.total_entries,
            format_size(self.total_nar_bytes),
            self.query_hits,
            self.query_misses,
            hit_rate
        )
    }
}

/// Cache download output.
pub struct CacheDownloadOutput {
    pub found: bool,
    pub blob_ticket: Option<String>,
    pub blob_hash: Option<String>,
    pub nar_size: Option<u64>,
    pub error: Option<String>,
}

impl Outputable for CacheDownloadOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "found": self.found,
            "blob_ticket": self.blob_ticket,
            "blob_hash": self.blob_hash,
            "nar_size": self.nar_size,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if !self.found {
            return "Store path not found in cache".to_string();
        }

        if let Some(ref e) = self.error {
            return format!("Error: {}", e);
        }

        let mut output = String::new();
        if let Some(ref ticket) = self.blob_ticket {
            output.push_str(&format!("BlobTicket: {}\n", ticket));
        }
        if let Some(ref hash) = self.blob_hash {
            output.push_str(&format!("BlobHash: {}\n", hash));
        }
        if let Some(size) = self.nar_size {
            output.push_str(&format!("NarSize: {}\n", format_size(size)));
        }

        output
    }
}

impl CacheCommand {
    /// Execute the cache command.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            CacheCommand::Query(args) => cache_query(client, args, json).await,
            CacheCommand::Stats => cache_stats(client, json).await,
            CacheCommand::Download(args) => cache_download(client, args, json).await,
        }
    }
}

async fn cache_query(client: &AspenClient, args: QueryArgs, json: bool) -> Result<()> {
    let store_hash = extract_store_hash(&args.store_path);

    let response = client.send(ClientRpcRequest::CacheQuery { store_hash }).await?;

    match response {
        ClientRpcResponse::CacheQueryResult(result) => {
            let output = CacheQueryOutput {
                found: result.found,
                store_path: result.entry.as_ref().map(|e| e.store_path.clone()),
                store_hash: result.entry.as_ref().map(|e| e.store_hash.clone()),
                blob_hash: result.entry.as_ref().map(|e| e.blob_hash.clone()),
                nar_size: result.entry.as_ref().map(|e| e.nar_size),
                nar_hash: result.entry.as_ref().map(|e| e.nar_hash.clone()),
                references: result.entry.as_ref().map(|e| e.references.clone()),
                deriver: result.entry.as_ref().and_then(|e| e.deriver.clone()),
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

async fn cache_stats(client: &AspenClient, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::CacheStats).await?;

    match response {
        ClientRpcResponse::CacheStatsResult(result) => {
            let output = CacheStatsOutput {
                total_entries: result.total_entries,
                total_nar_bytes: result.total_nar_bytes,
                query_hits: result.query_hits,
                query_misses: result.query_misses,
                node_id: result.node_id,
                error: result.error,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn cache_download(client: &AspenClient, args: DownloadArgs, json: bool) -> Result<()> {
    let store_hash = extract_store_hash(&args.store_path);

    let response = client.send(ClientRpcRequest::CacheDownload { store_hash }).await?;

    match response {
        ClientRpcResponse::CacheDownloadResult(result) => {
            // If ticket_only is set and we have a ticket, just print it
            if args.ticket_only {
                if let Some(ref ticket) = result.blob_ticket {
                    println!("{}", ticket);
                    return Ok(());
                } else {
                    anyhow::bail!("no blob ticket available");
                }
            }

            let output = CacheDownloadOutput {
                found: result.found,
                blob_ticket: result.blob_ticket,
                blob_hash: result.blob_hash,
                nar_size: result.nar_size,
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

// ============================================================================
// Helper Functions
// ============================================================================

/// Extract the store hash from a store path or hash string.
///
/// Accepts either:
/// - Full store path: /nix/store/abc...-name -> abc...
/// - Just the hash: abc... -> abc...
fn extract_store_hash(input: &str) -> String {
    const NIX_STORE_PREFIX: &str = "/nix/store/";

    if let Some(path) = input.strip_prefix(NIX_STORE_PREFIX) {
        // Extract the hash from the full path
        // The hash is everything before the first dash
        if let Some(dash_idx) = path.find('-') {
            path[..dash_idx].to_string()
        } else {
            // No dash found, return the whole thing
            path.to_string()
        }
    } else {
        // Assume it's already just the hash
        input.to_string()
    }
}

/// Format a byte size in human-readable form.
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_store_hash_full_path() {
        assert_eq!(extract_store_hash("/nix/store/abc123def456-hello-2.10"), "abc123def456");
    }

    #[test]
    fn test_extract_store_hash_hash_only() {
        assert_eq!(extract_store_hash("abc123def456"), "abc123def456");
    }

    #[test]
    fn test_extract_store_hash_no_dash() {
        assert_eq!(extract_store_hash("/nix/store/abc123def456"), "abc123def456");
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(2048), "2.00 KB");
        assert_eq!(format_size(1048576), "1.00 MB");
        assert_eq!(format_size(1073741824), "1.00 GB");
    }
}
