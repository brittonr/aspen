//! Index management commands.
//!
//! Commands for listing and inspecting secondary indexes.
//! Indexes are used by the SQL query engine for efficient filtering
//! on non-key columns like mod_revision, create_revision, etc.

use anyhow::Result;
use aspen_core::layer::IndexDefinition;
use aspen_core::layer::IndexFieldType;
use clap::Args;
use clap::Subcommand;
use serde::Serialize;

use crate::output::Outputable;
use crate::output::print_output;

/// Secondary index operations.
#[derive(Subcommand)]
pub enum IndexCommand {
    /// List all indexes.
    List(ListArgs),

    /// Show details of a specific index.
    Show(ShowArgs),
    // TODO: Future commands when RPC support is added:
    // /// Create a new custom index.
    // Create(CreateArgs),
    // /// Drop an index.
    // Drop(DropArgs),
    // /// Scan an index directly (debugging).
    // Scan(ScanArgs),
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

impl IndexCommand {
    /// Execute the index command.
    pub fn run(self, json: bool) -> Result<()> {
        match self {
            IndexCommand::List(args) => index_list(args, json),
            IndexCommand::Show(args) => index_show(args, json),
        }
    }
}

/// Output for index list.
#[derive(Debug, Serialize)]
pub struct IndexListOutput {
    /// Number of indexes found.
    pub count: usize,
    /// Index definitions.
    pub indexes: Vec<IndexInfo>,
}

/// Summary info for an index.
#[derive(Debug, Serialize)]
pub struct IndexInfo {
    /// Index name.
    pub name: String,
    /// Field being indexed.
    pub field: String,
    /// Field type (integer, string, etc.).
    pub field_type: String,
    /// Whether this is a built-in index.
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
    /// Index name.
    pub name: String,
    /// Field being indexed.
    pub field: Option<String>,
    /// Field type.
    pub field_type: String,
    /// Whether this is a built-in index.
    pub builtin: bool,
    /// Whether the index enforces uniqueness.
    pub unique: bool,
    /// Whether null values are indexed.
    pub index_nulls: bool,
    /// System key where this index metadata is stored.
    pub system_key: String,
    /// Description of what this index is useful for.
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

fn format_field_type(ft: IndexFieldType) -> String {
    match ft {
        IndexFieldType::Integer => "i64".to_string(),
        IndexFieldType::UnsignedInteger => "u64".to_string(),
        IndexFieldType::String => "string".to_string(),
    }
}
