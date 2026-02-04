//! Automerge CRDT document operations.
//!
//! Commands for managing Automerge documents with collaborative editing support.
//! Documents are stored in Aspen's distributed KV store with Raft consensus.

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::Outputable;
use crate::output::print_output;

/// Automerge CRDT document operations.
#[derive(Subcommand)]
pub enum AutomergeCommand {
    /// Create a new Automerge document.
    Create(CreateArgs),

    /// Get a document's content as JSON.
    Get(GetArgs),

    /// Delete a document.
    Delete(DeleteArgs),

    /// List documents.
    List(ListArgs),

    /// Get document metadata.
    Meta(MetaArgs),

    /// Check if a document exists.
    Exists(ExistsArgs),

    /// Merge two documents.
    Merge(MergeArgs),
}

#[derive(Args)]
pub struct CreateArgs {
    /// Custom document ID (optional, auto-generated if not provided).
    #[arg(long)]
    pub id: Option<String>,

    /// Namespace for organizing documents.
    #[arg(long)]
    pub namespace: Option<String>,

    /// Human-readable title.
    #[arg(long)]
    pub title: Option<String>,

    /// Document description.
    #[arg(long)]
    pub description: Option<String>,

    /// Tags for categorization (comma-separated).
    #[arg(long, value_delimiter = ',')]
    pub tags: Vec<String>,
}

#[derive(Args)]
pub struct GetArgs {
    /// Document ID.
    pub id: String,
}

#[derive(Args)]
pub struct DeleteArgs {
    /// Document ID to delete.
    pub id: String,
}

#[derive(Args)]
pub struct ListArgs {
    /// Filter by namespace.
    #[arg(long)]
    pub namespace: Option<String>,

    /// Filter by tag.
    #[arg(long)]
    pub tag: Option<String>,

    /// Maximum number of documents to return.
    #[arg(long, default_value = "100")]
    pub limit: u32,

    /// Continuation token for pagination.
    #[arg(long)]
    pub continuation: Option<String>,
}

#[derive(Args)]
pub struct MetaArgs {
    /// Document ID.
    pub id: String,
}

#[derive(Args)]
pub struct ExistsArgs {
    /// Document ID.
    pub id: String,
}

#[derive(Args)]
pub struct MergeArgs {
    /// Target document ID (will be updated).
    pub target: String,

    /// Source document ID (will not be modified).
    pub source: String,
}

// =============================================================================
// Output types
// =============================================================================

/// Create document output.
pub struct CreateOutput {
    pub success: bool,
    pub document_id: Option<String>,
    pub error: Option<String>,
}

impl Outputable for CreateOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "success": self.success,
            "document_id": self.document_id,
            "error": self.error,
        })
    }

    fn to_human(&self) -> String {
        if let Some(ref e) = self.error {
            format!("error: {e}")
        } else if self.success {
            format!("created: {}", self.document_id.as_deref().unwrap_or("?"))
        } else {
            "failed".to_string()
        }
    }
}

/// Get document output.
pub struct GetOutput {
    pub found: bool,
    pub document_id: Option<String>,
    pub content: Option<serde_json::Value>,
    pub size_bytes: Option<u64>,
    pub error: Option<String>,
}

impl Outputable for GetOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "found": self.found,
            "document_id": self.document_id,
            "content": self.content,
            "size_bytes": self.size_bytes,
            "error": self.error,
        })
    }

    fn to_human(&self) -> String {
        if let Some(ref e) = self.error {
            return format!("error: {e}");
        }

        if !self.found {
            return "(not found)".to_string();
        }

        if let Some(ref content) = self.content {
            serde_json::to_string_pretty(content).unwrap_or_else(|_| "(serialization error)".to_string())
        } else {
            let size = self.size_bytes.unwrap_or(0);
            format!("(found, {size} bytes)")
        }
    }
}

/// Delete document output.
pub struct DeleteOutput {
    pub success: bool,
    pub existed: bool,
    pub error: Option<String>,
}

impl Outputable for DeleteOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "success": self.success,
            "existed": self.existed,
            "error": self.error,
        })
    }

    fn to_human(&self) -> String {
        if let Some(ref e) = self.error {
            format!("error: {e}")
        } else if self.success && self.existed {
            "deleted".to_string()
        } else if self.success {
            "not found".to_string()
        } else {
            "failed".to_string()
        }
    }
}

/// List documents output.
pub struct ListOutput {
    pub documents: Vec<DocumentInfo>,
    pub count: u32,
    pub has_more: bool,
    pub continuation_token: Option<String>,
    pub error: Option<String>,
}

/// Document info in list.
pub struct DocumentInfo {
    pub id: String,
    pub title: Option<String>,
    pub namespace: Option<String>,
    pub size_bytes: u64,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

impl Outputable for ListOutput {
    fn to_json(&self) -> serde_json::Value {
        let docs: Vec<serde_json::Value> = self
            .documents
            .iter()
            .map(|d| {
                serde_json::json!({
                    "id": d.id,
                    "title": d.title,
                    "namespace": d.namespace,
                    "size_bytes": d.size_bytes,
                    "created_at": d.created_at,
                    "updated_at": d.updated_at,
                })
            })
            .collect();
        serde_json::json!({
            "documents": docs,
            "count": self.count,
            "has_more": self.has_more,
            "continuation_token": self.continuation_token,
            "error": self.error,
        })
    }

    fn to_human(&self) -> String {
        if let Some(ref e) = self.error {
            return format!("error: {e}");
        }

        if self.documents.is_empty() {
            return "(no documents)".to_string();
        }

        let mut lines: Vec<String> = Vec::new();
        for doc in &self.documents {
            let title = doc.title.as_deref().unwrap_or("(untitled)");
            let ns = doc.namespace.as_deref().unwrap_or("-");
            lines.push(format!("{} {} [{}] ({} bytes)", doc.id, title, ns, doc.size_bytes));
        }
        if self.has_more {
            lines.push(format!("... ({} shown, more available)", self.count));
        } else {
            lines.push(format!("({} total)", self.count));
        }
        lines.join("\n")
    }
}

/// Metadata output.
pub struct MetaOutput {
    pub found: bool,
    pub id: Option<String>,
    pub namespace: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub size_bytes: Option<u64>,
    pub change_count: Option<u64>,
    pub heads: Vec<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub error: Option<String>,
}

impl Outputable for MetaOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "found": self.found,
            "id": self.id,
            "namespace": self.namespace,
            "title": self.title,
            "description": self.description,
            "tags": self.tags,
            "size_bytes": self.size_bytes,
            "change_count": self.change_count,
            "heads": self.heads,
            "created_at": self.created_at,
            "updated_at": self.updated_at,
            "error": self.error,
        })
    }

    fn to_human(&self) -> String {
        if let Some(ref e) = self.error {
            return format!("error: {e}");
        }

        if !self.found {
            return "(not found)".to_string();
        }

        let mut lines = vec![format!("Document: {}", self.id.as_deref().unwrap_or("?"))];

        if let Some(ref title) = self.title {
            lines.push(format!("  Title: {title}"));
        }
        if let Some(ref ns) = self.namespace {
            lines.push(format!("  Namespace: {ns}"));
        }
        if let Some(ref desc) = self.description {
            lines.push(format!("  Description: {desc}"));
        }
        if !self.tags.is_empty() {
            lines.push(format!("  Tags: {}", self.tags.join(", ")));
        }
        if let Some(size) = self.size_bytes {
            lines.push(format!("  Size: {size} bytes"));
        }
        if let Some(count) = self.change_count {
            lines.push(format!("  Changes: {count}"));
        }
        if !self.heads.is_empty() {
            let heads_str = self.heads.iter().map(|h| &h[..16.min(h.len())]).collect::<Vec<_>>().join(", ");
            lines.push(format!("  Heads: {heads_str}"));
        }
        if let Some(ref created) = self.created_at {
            lines.push(format!("  Created: {created}"));
        }
        if let Some(ref updated) = self.updated_at {
            lines.push(format!("  Updated: {updated}"));
        }

        lines.join("\n")
    }
}

/// Exists output.
pub struct ExistsOutput {
    pub exists: bool,
    pub error: Option<String>,
}

impl Outputable for ExistsOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "exists": self.exists,
            "error": self.error,
        })
    }

    fn to_human(&self) -> String {
        if let Some(ref e) = self.error {
            format!("error: {e}")
        } else if self.exists {
            "exists".to_string()
        } else {
            "not found".to_string()
        }
    }
}

/// Merge output.
pub struct MergeOutput {
    pub success: bool,
    pub changes_applied: Option<u64>,
    pub new_heads: Vec<String>,
    pub error: Option<String>,
}

impl Outputable for MergeOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "success": self.success,
            "changes_applied": self.changes_applied,
            "new_heads": self.new_heads,
            "error": self.error,
        })
    }

    fn to_human(&self) -> String {
        if let Some(ref e) = self.error {
            format!("error: {e}")
        } else if self.success {
            let changes = self.changes_applied.unwrap_or(0);
            format!("merged ({changes} changes applied)")
        } else {
            "merge failed".to_string()
        }
    }
}

// =============================================================================
// Command handlers
// =============================================================================

impl AutomergeCommand {
    /// Execute the automerge command.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            AutomergeCommand::Create(args) => automerge_create(client, args, json).await,
            AutomergeCommand::Get(args) => automerge_get(client, args, json).await,
            AutomergeCommand::Delete(args) => automerge_delete(client, args, json).await,
            AutomergeCommand::List(args) => automerge_list(client, args, json).await,
            AutomergeCommand::Meta(args) => automerge_meta(client, args, json).await,
            AutomergeCommand::Exists(args) => automerge_exists(client, args, json).await,
            AutomergeCommand::Merge(args) => automerge_merge(client, args, json).await,
        }
    }
}

async fn automerge_create(client: &AspenClient, args: CreateArgs, json: bool) -> Result<()> {
    let request = ClientRpcRequest::AutomergeCreate {
        document_id: args.id,
        namespace: args.namespace,
        title: args.title,
        description: args.description,
        tags: args.tags,
    };

    let response = client.send(request).await?;

    let output = match response {
        ClientRpcResponse::AutomergeCreateResult(r) => CreateOutput {
            success: r.success,
            document_id: r.document_id,
            error: r.error,
        },
        other => CreateOutput {
            success: false,
            document_id: None,
            error: Some(format!("unexpected response: {other:?}")),
        },
    };

    print_output(&output, json);
    Ok(())
}

async fn automerge_get(client: &AspenClient, args: GetArgs, json: bool) -> Result<()> {
    let request = ClientRpcRequest::AutomergeGet {
        document_id: args.id.clone(),
    };

    let response = client.send(request).await?;

    let output = match response {
        ClientRpcResponse::AutomergeGetResult(r) => {
            // Note: document_bytes is base64-encoded binary - we don't try to parse as JSON
            // The actual content would need to be decoded and read via Automerge API
            let size_bytes = r.metadata.as_ref().map(|m| m.size_bytes);
            GetOutput {
                found: r.found,
                document_id: r.document_id,
                content: None, // Document content requires Automerge to parse
                size_bytes,
                error: r.error,
            }
        }
        other => GetOutput {
            found: false,
            document_id: None,
            content: None,
            size_bytes: None,
            error: Some(format!("unexpected response: {other:?}")),
        },
    };

    print_output(&output, json);
    Ok(())
}

async fn automerge_delete(client: &AspenClient, args: DeleteArgs, json: bool) -> Result<()> {
    let request = ClientRpcRequest::AutomergeDelete { document_id: args.id };

    let response = client.send(request).await?;

    let output = match response {
        ClientRpcResponse::AutomergeDeleteResult(r) => DeleteOutput {
            success: r.success,
            existed: r.existed,
            error: r.error,
        },
        other => DeleteOutput {
            success: false,
            existed: false,
            error: Some(format!("unexpected response: {other:?}")),
        },
    };

    print_output(&output, json);
    Ok(())
}

async fn automerge_list(client: &AspenClient, args: ListArgs, json: bool) -> Result<()> {
    let request = ClientRpcRequest::AutomergeList {
        namespace: args.namespace,
        tag: args.tag,
        limit: Some(args.limit),
        continuation_token: args.continuation,
    };

    let response = client.send(request).await?;

    let output = match response {
        ClientRpcResponse::AutomergeListResult(r) => {
            let count = r.documents.len() as u32;
            let documents = r
                .documents
                .into_iter()
                .map(|d| DocumentInfo {
                    id: d.document_id,
                    title: d.title,
                    namespace: d.namespace,
                    size_bytes: d.size_bytes,
                    created_at: Some(format_timestamp_ms(d.created_at_ms)),
                    updated_at: Some(format_timestamp_ms(d.updated_at_ms)),
                })
                .collect();
            ListOutput {
                documents,
                count,
                has_more: r.has_more,
                continuation_token: r.continuation_token,
                error: r.error,
            }
        }
        other => ListOutput {
            documents: vec![],
            count: 0,
            has_more: false,
            continuation_token: None,
            error: Some(format!("unexpected response: {other:?}")),
        },
    };

    print_output(&output, json);
    Ok(())
}

/// Format a timestamp in milliseconds to a human-readable string.
fn format_timestamp_ms(ms: u64) -> String {
    use chrono::DateTime;
    let secs = (ms / 1000) as i64;
    let nanos = ((ms % 1000) * 1_000_000) as u32;
    DateTime::from_timestamp(secs, nanos)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| format!("{ms}ms"))
}

async fn automerge_meta(client: &AspenClient, args: MetaArgs, json: bool) -> Result<()> {
    let request = ClientRpcRequest::AutomergeGetMetadata {
        document_id: args.id.clone(),
    };

    let response = client.send(request).await?;

    let output = match response {
        ClientRpcResponse::AutomergeGetMetadataResult(r) => {
            if let Some(meta) = r.metadata {
                MetaOutput {
                    found: true,
                    id: Some(meta.document_id),
                    namespace: meta.namespace,
                    title: meta.title,
                    description: meta.description,
                    tags: meta.tags,
                    size_bytes: Some(meta.size_bytes),
                    change_count: Some(meta.change_count),
                    heads: meta.heads,
                    created_at: Some(format_timestamp_ms(meta.created_at_ms)),
                    updated_at: Some(format_timestamp_ms(meta.updated_at_ms)),
                    error: r.error,
                }
            } else {
                MetaOutput {
                    found: false,
                    id: Some(args.id),
                    namespace: None,
                    title: None,
                    description: None,
                    tags: vec![],
                    size_bytes: None,
                    change_count: None,
                    heads: vec![],
                    created_at: None,
                    updated_at: None,
                    error: r.error,
                }
            }
        }
        other => MetaOutput {
            found: false,
            id: None,
            namespace: None,
            title: None,
            description: None,
            tags: vec![],
            size_bytes: None,
            change_count: None,
            heads: vec![],
            created_at: None,
            updated_at: None,
            error: Some(format!("unexpected response: {other:?}")),
        },
    };

    print_output(&output, json);
    Ok(())
}

async fn automerge_exists(client: &AspenClient, args: ExistsArgs, json: bool) -> Result<()> {
    let request = ClientRpcRequest::AutomergeExists { document_id: args.id };

    let response = client.send(request).await?;

    let output = match response {
        ClientRpcResponse::AutomergeExistsResult(r) => ExistsOutput {
            exists: r.exists,
            error: r.error,
        },
        other => ExistsOutput {
            exists: false,
            error: Some(format!("unexpected response: {other:?}")),
        },
    };

    print_output(&output, json);
    Ok(())
}

async fn automerge_merge(client: &AspenClient, args: MergeArgs, json: bool) -> Result<()> {
    let request = ClientRpcRequest::AutomergeMerge {
        target_document_id: args.target,
        source_document_id: args.source,
    };

    let response = client.send(request).await?;

    let output = match response {
        ClientRpcResponse::AutomergeMergeResult(r) => MergeOutput {
            success: r.success,
            changes_applied: r.change_count,
            new_heads: r.new_heads,
            error: r.error,
        },
        other => MergeOutput {
            success: false,
            changes_applied: None,
            new_heads: vec![],
            error: Some(format!("unexpected response: {other:?}")),
        },
    };

    print_output(&output, json);
    Ok(())
}
