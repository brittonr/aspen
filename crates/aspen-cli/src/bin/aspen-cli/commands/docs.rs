//! Docs operations.
//!
//! Commands for iroh-docs CRDT-replicated namespace.
//! Supports set, get, delete operations on the docs namespace.

use anyhow::Result;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::Outputable;
use crate::output::print_output;
use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;

/// Docs namespace operations.
#[derive(Subcommand)]
pub enum DocsCommand {
    /// Set a key-value pair in the docs namespace.
    Set(SetArgs),

    /// Get a value from the docs namespace.
    Get(GetArgs),

    /// Delete a key from the docs namespace.
    Delete(DeleteArgs),

    /// List entries in the docs namespace.
    List(ListArgs),

    /// Get docs namespace status.
    Status,

    /// Get a ticket for sharing the docs namespace.
    Ticket(TicketArgs),
}

#[derive(Args)]
pub struct SetArgs {
    /// The key to set.
    pub key: String,

    /// The value to set.
    pub value: String,
}

#[derive(Args)]
pub struct GetArgs {
    /// The key to get.
    pub key: String,
}

#[derive(Args)]
pub struct DeleteArgs {
    /// The key to delete.
    pub key: String,
}

#[derive(Args)]
pub struct ListArgs {
    /// Prefix filter for keys.
    #[arg(long)]
    pub prefix: Option<String>,

    /// Maximum entries to return.
    #[arg(long, default_value = "100")]
    pub limit: u32,
}

#[derive(Args)]
pub struct TicketArgs {
    /// Whether to request read-write access (default: read-only).
    #[arg(long)]
    pub read_write: bool,

    /// Priority level (0 = highest).
    #[arg(long, default_value = "0")]
    pub priority: u8,
}

// =============================================================================
// Output types
// =============================================================================

/// Docs set output.
pub struct DocsSetOutput {
    pub success: bool,
    pub key: Option<String>,
    pub size: Option<u64>,
    pub error: Option<String>,
}

impl Outputable for DocsSetOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "success": self.success,
            "key": self.key,
            "size": self.size,
            "error": self.error,
        })
    }

    fn to_human(&self) -> String {
        if let Some(ref e) = self.error {
            format!("error: {e}")
        } else if self.success {
            let key = self.key.as_deref().unwrap_or("?");
            let size = self.size.unwrap_or(0);
            format!("{key} ({size} bytes)")
        } else {
            "failed".to_string()
        }
    }
}

/// Docs get output.
pub struct DocsGetOutput {
    pub found: bool,
    pub value: Option<Vec<u8>>,
    pub size: Option<u64>,
    pub error: Option<String>,
}

impl Outputable for DocsGetOutput {
    fn to_json(&self) -> serde_json::Value {
        let value_str = self.value.as_ref().map(|v| String::from_utf8_lossy(v).to_string());
        serde_json::json!({
            "found": self.found,
            "value": value_str,
            "size": self.size,
            "error": self.error,
        })
    }

    fn to_human(&self) -> String {
        if let Some(ref e) = self.error {
            format!("error: {e}")
        } else if self.found {
            if let Some(ref v) = self.value {
                String::from_utf8_lossy(v).to_string()
            } else {
                let size = self.size.unwrap_or(0);
                format!("(found, {size} bytes, content not available)")
            }
        } else {
            "(not found)".to_string()
        }
    }
}

/// Docs delete output.
pub struct DocsDeleteOutput {
    pub success: bool,
    pub error: Option<String>,
}

impl Outputable for DocsDeleteOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "success": self.success,
            "error": self.error,
        })
    }

    fn to_human(&self) -> String {
        if let Some(ref e) = self.error {
            format!("error: {e}")
        } else if self.success {
            "deleted".to_string()
        } else {
            "failed".to_string()
        }
    }
}

/// Docs list output.
pub struct DocsListOutput {
    pub entries: Vec<DocsListEntry>,
    pub count: u32,
    pub has_more: bool,
    pub error: Option<String>,
}

/// Single docs list entry.
pub struct DocsListEntry {
    pub key: String,
    pub size: u64,
    pub hash: String,
}

impl Outputable for DocsListOutput {
    fn to_json(&self) -> serde_json::Value {
        let entries: Vec<serde_json::Value> = self
            .entries
            .iter()
            .map(|e| {
                serde_json::json!({
                    "key": e.key,
                    "size": e.size,
                    "hash": e.hash,
                })
            })
            .collect();
        serde_json::json!({
            "entries": entries,
            "count": self.count,
            "has_more": self.has_more,
            "error": self.error,
        })
    }

    fn to_human(&self) -> String {
        if let Some(ref e) = self.error {
            return format!("error: {e}");
        }

        if self.entries.is_empty() {
            return "(no entries)".to_string();
        }

        let mut lines: Vec<String> = Vec::new();
        for entry in &self.entries {
            lines.push(format!("{} ({} bytes, {})", entry.key, entry.size, &entry.hash[..16]));
        }
        if self.has_more {
            lines.push(format!("... ({} shown, more available)", self.count));
        } else {
            lines.push(format!("({} total)", self.count));
        }
        lines.join("\n")
    }
}

/// Docs status output.
pub struct DocsStatusOutput {
    pub enabled: bool,
    pub namespace_id: Option<String>,
    pub author_id: Option<String>,
    pub entry_count: Option<u64>,
    pub replica_open: Option<bool>,
    pub error: Option<String>,
}

impl Outputable for DocsStatusOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "enabled": self.enabled,
            "namespace_id": self.namespace_id,
            "author_id": self.author_id,
            "entry_count": self.entry_count,
            "replica_open": self.replica_open,
            "error": self.error,
        })
    }

    fn to_human(&self) -> String {
        if let Some(ref e) = self.error {
            return format!("error: {e}");
        }

        if !self.enabled {
            return "docs not enabled".to_string();
        }

        let mut lines = vec!["Docs enabled".to_string()];
        if let Some(ref ns) = self.namespace_id {
            lines.push(format!("  Namespace: {}", &ns[..32.min(ns.len())]));
        }
        if let Some(ref author) = self.author_id {
            lines.push(format!("  Author: {}", &author[..32.min(author.len())]));
        }
        if let Some(count) = self.entry_count {
            lines.push(format!("  Entries: {count}"));
        }
        if let Some(open) = self.replica_open {
            lines.push(format!("  Replica: {}", if open { "open" } else { "closed" }));
        }
        lines.join("\n")
    }
}

/// Docs ticket output.
pub struct DocsTicketOutput {
    pub ticket: Option<String>,
    pub cluster_id: Option<String>,
    pub namespace_id: Option<String>,
    pub read_write: bool,
    pub priority: u8,
    pub error: Option<String>,
}

impl Outputable for DocsTicketOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "ticket": self.ticket,
            "cluster_id": self.cluster_id,
            "namespace_id": self.namespace_id,
            "read_write": self.read_write,
            "priority": self.priority,
            "error": self.error,
        })
    }

    fn to_human(&self) -> String {
        if let Some(ref e) = self.error {
            return format!("error: {e}");
        }

        if let Some(ref t) = self.ticket {
            let mode = if self.read_write { "read-write" } else { "read-only" };
            format!("{t}\n({mode}, priority={})", self.priority)
        } else {
            "(no ticket)".to_string()
        }
    }
}

// =============================================================================
// Command handlers
// =============================================================================

impl DocsCommand {
    /// Execute the docs command.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            DocsCommand::Set(args) => docs_set(client, args, json).await,
            DocsCommand::Get(args) => docs_get(client, args, json).await,
            DocsCommand::Delete(args) => docs_delete(client, args, json).await,
            DocsCommand::List(args) => docs_list(client, args, json).await,
            DocsCommand::Status => docs_status(client, json).await,
            DocsCommand::Ticket(args) => docs_ticket(client, args, json).await,
        }
    }
}

async fn docs_set(client: &AspenClient, args: SetArgs, json: bool) -> Result<()> {
    let request = ClientRpcRequest::DocsSet {
        key: args.key,
        value: args.value.into_bytes(),
    };

    let response = client.send(request).await?;

    let output = match response {
        ClientRpcResponse::DocsSetResult(r) => DocsSetOutput {
            success: r.success,
            key: r.key,
            size: r.size,
            error: r.error,
        },
        other => DocsSetOutput {
            success: false,
            key: None,
            size: None,
            error: Some(format!("unexpected response: {other:?}")),
        },
    };

    print_output(&output, json);
    Ok(())
}

async fn docs_get(client: &AspenClient, args: GetArgs, json: bool) -> Result<()> {
    let request = ClientRpcRequest::DocsGet { key: args.key };

    let response = client.send(request).await?;

    let output = match response {
        ClientRpcResponse::DocsGetResult(r) => DocsGetOutput {
            found: r.found,
            value: r.value,
            size: r.size,
            error: r.error,
        },
        other => DocsGetOutput {
            found: false,
            value: None,
            size: None,
            error: Some(format!("unexpected response: {other:?}")),
        },
    };

    print_output(&output, json);
    Ok(())
}

async fn docs_delete(client: &AspenClient, args: DeleteArgs, json: bool) -> Result<()> {
    let request = ClientRpcRequest::DocsDelete { key: args.key };

    let response = client.send(request).await?;

    let output = match response {
        ClientRpcResponse::DocsDeleteResult(r) => DocsDeleteOutput {
            success: r.success,
            error: r.error,
        },
        other => DocsDeleteOutput {
            success: false,
            error: Some(format!("unexpected response: {other:?}")),
        },
    };

    print_output(&output, json);
    Ok(())
}

async fn docs_list(client: &AspenClient, args: ListArgs, json: bool) -> Result<()> {
    let request = ClientRpcRequest::DocsList {
        prefix: args.prefix,
        limit: Some(args.limit),
    };

    let response = client.send(request).await?;

    let output = match response {
        ClientRpcResponse::DocsListResult(r) => {
            let entries = r
                .entries
                .into_iter()
                .map(|e| DocsListEntry {
                    key: e.key,
                    size: e.size,
                    hash: e.hash,
                })
                .collect();
            DocsListOutput {
                entries,
                count: r.count,
                has_more: r.has_more,
                error: r.error,
            }
        }
        other => DocsListOutput {
            entries: vec![],
            count: 0,
            has_more: false,
            error: Some(format!("unexpected response: {other:?}")),
        },
    };

    print_output(&output, json);
    Ok(())
}

async fn docs_status(client: &AspenClient, json: bool) -> Result<()> {
    let request = ClientRpcRequest::DocsStatus;

    let response = client.send(request).await?;

    let output = match response {
        ClientRpcResponse::DocsStatusResult(r) => DocsStatusOutput {
            enabled: r.enabled,
            namespace_id: r.namespace_id,
            author_id: r.author_id,
            entry_count: r.entry_count,
            replica_open: r.replica_open,
            error: r.error,
        },
        other => DocsStatusOutput {
            enabled: false,
            namespace_id: None,
            author_id: None,
            entry_count: None,
            replica_open: None,
            error: Some(format!("unexpected response: {other:?}")),
        },
    };

    print_output(&output, json);
    Ok(())
}

async fn docs_ticket(client: &AspenClient, args: TicketArgs, json: bool) -> Result<()> {
    let request = ClientRpcRequest::GetDocsTicket {
        read_write: args.read_write,
        priority: args.priority,
    };

    let response = client.send(request).await?;

    let output = match response {
        ClientRpcResponse::DocsTicket(r) => DocsTicketOutput {
            ticket: Some(r.ticket),
            cluster_id: Some(r.cluster_id),
            namespace_id: Some(r.namespace_id),
            read_write: r.read_write,
            priority: r.priority,
            error: r.error,
        },
        other => DocsTicketOutput {
            ticket: None,
            cluster_id: None,
            namespace_id: None,
            read_write: false,
            priority: 0,
            error: Some(format!("unexpected response: {other:?}")),
        },
    };

    print_output(&output, json);
    Ok(())
}
