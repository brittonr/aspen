//! Index management commands.
//!
//! Commands for listing and inspecting secondary indexes.
//! Indexes are used by the SQL query engine for efficient filtering
//! on non-key columns like mod_revision, create_revision, etc.
//!
//! `list` and `show` work offline (builtin index metadata only).
//! `create`, `drop`, and `scan` require a cluster connection.

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_core::layer::IndexDefinition;
use aspen_core::layer::IndexFieldType;
use clap::Args;
use clap::Subcommand;
use serde::Serialize;

use crate::client::AspenClient;
use crate::output::Outputable;
use crate::output::print_output;

/// Secondary index operations.
#[derive(Subcommand)]
pub enum IndexCommand {
    /// List all indexes (local builtin metadata).
    List(ListArgs),

    /// Show details of a specific index (local builtin metadata).
    Show(ShowArgs),

    /// Create a custom secondary index.
    Create(CreateArgs),

    /// Drop a custom secondary index.
    Drop(DropArgs),

    /// Scan an index for matching entries.
    Scan(ScanArgs),

    /// List all indexes from the server (includes custom indexes).
    ServerList(ServerListArgs),
}

#[derive(Args)]
pub struct ListArgs {
    /// Show only builtin indexes.
    #[arg(long)]
    pub builtin: bool,
}

#[derive(Args)]
pub struct ShowArgs {
    /// Name of the index to show.
    pub name: String,
}

#[derive(Args)]
pub struct CreateArgs {
    /// Index name (must be unique).
    pub name: String,
    /// Field to index.
    #[arg(long)]
    pub field: String,
    /// Field type: integer, unsignedinteger, or string.
    #[arg(long, default_value = "integer")]
    pub field_type: String,
    /// Enforce uniqueness.
    #[arg(long)]
    pub unique: bool,
    /// Index null values.
    #[arg(long)]
    pub index_nulls: bool,
}

#[derive(Args)]
pub struct DropArgs {
    /// Name of the index to drop.
    pub name: String,
}

#[derive(Args)]
pub struct ScanArgs {
    /// Index name to scan.
    pub index_name: String,
    /// Value to match (hex-encoded bytes, or plain text for string indexes).
    pub value: String,
    /// Scan mode: exact, range, or lt.
    #[arg(long, default_value = "exact")]
    pub mode: String,
    /// End value for range mode (hex-encoded bytes).
    #[arg(long)]
    pub end_value: Option<String>,
    /// Maximum results.
    #[arg(long, default_value = "1000")]
    pub limit: u32,
}

#[derive(Args)]
pub struct ServerListArgs;

impl IndexCommand {
    /// Returns true if this command can run without a cluster connection.
    pub fn is_local(&self) -> bool {
        matches!(self, IndexCommand::List(_) | IndexCommand::Show(_))
    }

    /// Execute a local-only index command (no client needed).
    pub fn run_local(self, json: bool) -> Result<()> {
        match self {
            IndexCommand::List(args) => index_list(args, json),
            IndexCommand::Show(args) => index_show(args, json),
            _ => anyhow::bail!("this command requires a cluster connection"),
        }
    }

    /// Execute an index command that requires a cluster connection.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            IndexCommand::List(args) => index_list(args, json),
            IndexCommand::Show(args) => index_show(args, json),
            IndexCommand::Create(args) => index_create(client, args, json).await,
            IndexCommand::Drop(args) => index_drop(client, args, json).await,
            IndexCommand::Scan(args) => index_scan(client, args, json).await,
            IndexCommand::ServerList(_) => index_server_list(client, json).await,
        }
    }
}

// =============================================================================
// Output types
// =============================================================================

/// Output for index list.
#[derive(Debug, Serialize)]
pub struct IndexListOutput {
    pub count: usize,
    pub indexes: Vec<IndexInfo>,
}

/// Summary info for an index.
#[derive(Debug, Serialize)]
pub struct IndexInfo {
    pub name: String,
    pub field: String,
    pub field_type: String,
    pub builtin: bool,
}

impl Outputable for IndexListOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "count": self.count,
            "indexes": self.indexes.iter().map(|idx| serde_json::json!({
                "name": idx.name,
                "field": idx.field,
                "field_type": idx.field_type,
                "builtin": idx.builtin
            })).collect::<Vec<_>>()
        })
    }

    fn to_human(&self) -> String {
        if self.count == 0 {
            return "No indexes found.".to_string();
        }

        let mut output = format!("Indexes ({}):\n\n", self.count);
        for idx in &self.indexes {
            let builtin_marker = if idx.builtin { " (builtin)" } else { "" };
            output.push_str(&format!("  {} -> {}: {}{}\n", idx.name, idx.field, idx.field_type, builtin_marker));
        }
        output
    }
}

/// Output for index show.
#[derive(Debug, Serialize)]
pub struct IndexShowOutput {
    pub name: String,
    pub field: Option<String>,
    pub field_type: String,
    pub builtin: bool,
    pub unique: bool,
    pub index_nulls: bool,
    pub system_key: String,
    pub description: String,
}

impl Outputable for IndexShowOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "field": self.field,
            "field_type": self.field_type,
            "builtin": self.builtin,
            "unique": self.unique,
            "index_nulls": self.index_nulls,
            "system_key": self.system_key,
            "description": self.description
        })
    }

    fn to_human(&self) -> String {
        format!(
            "Index: {}\n\n\
             \x20 Field:        {}\n\
             \x20 Field Type:   {}\n\
             \x20 Builtin:      {}\n\
             \x20 Unique:       {}\n\
             \x20 Index Nulls:  {}\n\
             \x20 System Key:   {}\n\n\
             Description:\n\
             \x20 {}",
            self.name,
            self.field.as_deref().unwrap_or("N/A"),
            self.field_type,
            self.builtin,
            self.unique,
            self.index_nulls,
            self.system_key,
            self.description
        )
    }
}

/// Output for index create.
#[derive(Debug, Serialize)]
pub struct IndexCreateOutput {
    pub name: String,
    pub is_success: bool,
    pub error: Option<String>,
}

impl Outputable for IndexCreateOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "success": self.is_success,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.is_success {
            format!("Index '{}' created.", self.name)
        } else {
            format!("Failed to create index '{}': {}", self.name, self.error.as_deref().unwrap_or("unknown error"))
        }
    }
}

/// Output for index drop.
#[derive(Debug, Serialize)]
pub struct IndexDropOutput {
    pub name: String,
    pub was_dropped: bool,
    pub error: Option<String>,
}

impl Outputable for IndexDropOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "dropped": self.was_dropped,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if self.was_dropped {
            format!("Index '{}' dropped.", self.name)
        } else if let Some(err) = &self.error {
            format!("Failed to drop index '{}': {}", self.name, err)
        } else {
            format!("Index '{}' not found.", self.name)
        }
    }
}

/// Output for index scan.
#[derive(Debug, Serialize)]
pub struct IndexScanOutput {
    pub index_name: String,
    pub count: u32,
    pub has_more: bool,
    pub primary_keys: Vec<String>,
    pub error: Option<String>,
}

impl Outputable for IndexScanOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "index_name": self.index_name,
            "count": self.count,
            "has_more": self.has_more,
            "primary_keys": self.primary_keys,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if let Some(err) = &self.error {
            return format!("Index scan failed: {}", err);
        }
        let mut output = format!(
            "Index '{}': {} result(s){}:\n",
            self.index_name,
            self.count,
            if self.has_more { " (more available)" } else { "" }
        );
        for key in &self.primary_keys {
            output.push_str(&format!("  {}\n", key));
        }
        output
    }
}

// =============================================================================
// Local command handlers (no client)
// =============================================================================

fn index_list(_args: ListArgs, json: bool) -> Result<()> {
    let definitions = IndexDefinition::builtins();

    let indexes: Vec<IndexInfo> = definitions
        .iter()
        .map(|def| IndexInfo {
            name: def.name.clone(),
            field: def.field.clone().unwrap_or_default(),
            field_type: format_field_type(def.field_type),
            builtin: def.builtin,
        })
        .collect();

    let output = IndexListOutput {
        count: indexes.len(),
        indexes,
    };

    print_output(&output, json);
    Ok(())
}

fn index_show(args: ShowArgs, json: bool) -> Result<()> {
    let definitions = IndexDefinition::builtins();

    let def = definitions.iter().find(|d| d.name == args.name).ok_or_else(|| {
        anyhow::anyhow!("Index '{}' not found. Use 'aspen-cli index list' to see available indexes.", args.name)
    })?;

    let description = match def.name.as_str() {
        "idx_mod_revision" => {
            "Query entries by their modification revision (Raft log index). \
            Useful for watching for changes since a specific revision."
        }
        "idx_create_revision" => {
            "Query entries by their creation revision. \
            Useful for finding all entries created in a specific time window."
        }
        "idx_expires_at" => {
            "Query entries by expiration timestamp. \
            Used internally for TTL cleanup to find expired entries efficiently."
        }
        "idx_lease_id" => {
            "Query entries attached to a specific lease. \
            Used to find all keys that will be deleted when a lease expires."
        }
        _ => "Custom index for efficient filtering.",
    };

    let output = IndexShowOutput {
        name: def.name.clone(),
        field: def.field.clone(),
        field_type: format_field_type(def.field_type),
        builtin: def.builtin,
        unique: def.options.unique,
        index_nulls: def.options.index_nulls,
        system_key: def.system_key(),
        description: description.to_string(),
    };

    print_output(&output, json);
    Ok(())
}

// =============================================================================
// Remote command handlers (require client)
// =============================================================================

async fn index_create(client: &AspenClient, args: CreateArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::IndexCreate {
            name: args.name.clone(),
            field: args.field,
            field_type: args.field_type,
            is_unique: args.unique,
            should_index_nulls: args.index_nulls,
        })
        .await?;

    match response {
        ClientRpcResponse::IndexCreateResult(result) => {
            let output = IndexCreateOutput {
                name: result.name,
                is_success: result.is_success,
                error: result.error,
            };
            print_output(&output, json);
            if !output.is_success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        other => anyhow::bail!("unexpected response: {:?}", other),
    }
}

async fn index_drop(client: &AspenClient, args: DropArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::IndexDrop {
            name: args.name.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::IndexDropResult(result) => {
            let output = IndexDropOutput {
                name: result.name,
                was_dropped: result.was_dropped,
                error: result.error,
            };
            print_output(&output, json);
            if !result.is_success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        other => anyhow::bail!("unexpected response: {:?}", other),
    }
}

async fn index_scan(client: &AspenClient, args: ScanArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::IndexScan {
            index_name: args.index_name.clone(),
            mode: args.mode,
            value: args.value,
            end_value: args.end_value,
            limit: Some(args.limit),
        })
        .await?;

    match response {
        ClientRpcResponse::IndexScanResult(result) => {
            let output = IndexScanOutput {
                index_name: args.index_name,
                count: result.count,
                has_more: result.has_more,
                primary_keys: result.primary_keys,
                error: result.error,
            };
            print_output(&output, json);
            if !result.is_success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        other => anyhow::bail!("unexpected response: {:?}", other),
    }
}

async fn index_server_list(client: &AspenClient, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::IndexList).await?;

    match response {
        ClientRpcResponse::IndexListResult(result) => {
            let indexes: Vec<IndexInfo> = result
                .indexes
                .iter()
                .map(|def| IndexInfo {
                    name: def.name.clone(),
                    field: def.field.clone().unwrap_or_default(),
                    field_type: def.field_type.clone(),
                    builtin: def.builtin,
                })
                .collect();

            let output = IndexListOutput {
                count: indexes.len(),
                indexes,
            };
            print_output(&output, json);
            if !result.is_success {
                std::process::exit(1);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        other => anyhow::bail!("unexpected response: {:?}", other),
    }
}

fn format_field_type(ft: IndexFieldType) -> String {
    match ft {
        IndexFieldType::Integer => "i64".to_string(),
        IndexFieldType::UnsignedInteger => "u64".to_string(),
        IndexFieldType::String => "string".to_string(),
    }
}
