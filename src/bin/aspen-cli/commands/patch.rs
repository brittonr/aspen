//! Patch management commands.
//!
//! Commands for managing collaborative object (COB) patches (pull requests).

use anyhow::Result;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::print_output;
use crate::output::PatchDetailOutput;
use crate::output::PatchListOutput;
use crate::output::PatchOutput;
use aspen::client_rpc::ClientRpcRequest;
use aspen::client_rpc::ClientRpcResponse;

/// Patch management operations.
#[derive(Subcommand)]
pub enum PatchCommand {
    /// List patches in a repository.
    List(PatchListArgs),

    /// Create a new patch.
    Create(PatchCreateArgs),

    /// Show patch details.
    Show(PatchShowArgs),

    /// Approve a patch.
    Approve(PatchApproveArgs),

    /// Merge a patch.
    Merge(PatchMergeArgs),

    /// Close a patch without merging.
    Close(PatchCloseArgs),
}

#[derive(Args)]
pub struct PatchListArgs {
    /// Repository ID.
    #[arg(short, long)]
    pub repo: String,

    /// Filter by state: open, merged, closed, all.
    #[arg(long, default_value = "open")]
    pub state: String,

    /// Maximum patches to show.
    #[arg(short = 'n', long, default_value = "20")]
    pub limit: u32,
}

#[derive(Args)]
pub struct PatchCreateArgs {
    /// Repository ID.
    #[arg(short, long)]
    pub repo: String,

    /// Patch title.
    #[arg(short, long)]
    pub title: String,

    /// Patch description.
    #[arg(short, long, default_value = "")]
    pub description: String,

    /// Base commit hash (what we're merging into).
    #[arg(long)]
    pub base: String,

    /// Head commit hash (what we're merging).
    #[arg(long)]
    pub head: String,
}

#[derive(Args)]
pub struct PatchShowArgs {
    /// Repository ID.
    #[arg(short, long)]
    pub repo: String,

    /// Patch ID (hex-encoded).
    pub patch: String,
}

#[derive(Args)]
pub struct PatchApproveArgs {
    /// Repository ID.
    #[arg(short, long)]
    pub repo: String,

    /// Patch ID (hex-encoded).
    pub patch: String,

    /// Approval message.
    #[arg(short, long)]
    pub message: Option<String>,
}

#[derive(Args)]
pub struct PatchMergeArgs {
    /// Repository ID.
    #[arg(short, long)]
    pub repo: String,

    /// Patch ID (hex-encoded).
    pub patch: String,

    /// Merge commit hash.
    #[arg(long)]
    pub merge_commit: String,
}

#[derive(Args)]
pub struct PatchCloseArgs {
    /// Repository ID.
    #[arg(short, long)]
    pub repo: String,

    /// Patch ID (hex-encoded).
    pub patch: String,

    /// Reason for closing.
    #[arg(long)]
    pub reason: Option<String>,
}

impl PatchCommand {
    /// Execute the patch command.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            PatchCommand::List(args) => patch_list(client, args, json).await,
            PatchCommand::Create(args) => patch_create(client, args, json).await,
            PatchCommand::Show(args) => patch_show(client, args, json).await,
            PatchCommand::Approve(args) => patch_approve(client, args, json).await,
            PatchCommand::Merge(args) => patch_merge(client, args, json).await,
            PatchCommand::Close(args) => patch_close(client, args, json).await,
        }
    }
}

async fn patch_list(client: &AspenClient, args: PatchListArgs, json: bool) -> Result<()> {
    let state = match args.state.as_str() {
        "all" => None,
        s => Some(s.to_string()),
    };

    let response = client
        .send(ClientRpcRequest::ForgeListPatches {
            repo_id: args.repo,
            state,
            limit: Some(args.limit),
        })
        .await?;

    match response {
        ClientRpcResponse::ForgePatchListResult(result) => {
            if result.success {
                let patches: Vec<PatchOutput> = result
                    .patches
                    .into_iter()
                    .map(|p| PatchOutput {
                        id: p.id,
                        title: p.title,
                        state: p.state,
                        base: p.base,
                        head: p.head,
                        labels: p.labels,
                        revision_count: p.revision_count,
                        approval_count: p.approval_count,
                        created_at_ms: p.created_at_ms,
                        updated_at_ms: p.updated_at_ms,
                    })
                    .collect();
                let output = PatchListOutput {
                    patches,
                    count: result.count,
                };
                print_output(&output, json);
                Ok(())
            } else {
                anyhow::bail!("{}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
        }
        ClientRpcResponse::ForgeOperationResult(result) => {
            anyhow::bail!("{}", result.error.unwrap_or_else(|| "operation failed".to_string()))
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn patch_create(client: &AspenClient, args: PatchCreateArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::ForgeCreatePatch {
            repo_id: args.repo,
            title: args.title,
            description: args.description,
            base: args.base,
            head: args.head,
        })
        .await?;

    match response {
        ClientRpcResponse::ForgePatchResult(result) => {
            if result.success {
                if let Some(patch) = result.patch {
                    let output = PatchOutput {
                        id: patch.id,
                        title: patch.title,
                        state: patch.state,
                        base: patch.base,
                        head: patch.head,
                        labels: patch.labels,
                        revision_count: patch.revision_count,
                        approval_count: patch.approval_count,
                        created_at_ms: patch.created_at_ms,
                        updated_at_ms: patch.updated_at_ms,
                    };
                    print_output(&output, json);
                }
                Ok(())
            } else {
                anyhow::bail!("{}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
        }
        ClientRpcResponse::ForgeOperationResult(result) => {
            if result.success {
                if !json {
                    println!("Patch created");
                }
                Ok(())
            } else {
                anyhow::bail!("{}", result.error.unwrap_or_else(|| "operation failed".to_string()))
            }
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn patch_show(client: &AspenClient, args: PatchShowArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::ForgeGetPatch {
            repo_id: args.repo,
            patch_id: args.patch,
        })
        .await?;

    match response {
        ClientRpcResponse::ForgePatchResult(result) => {
            if result.success {
                if let Some(patch) = result.patch {
                    let output = PatchDetailOutput {
                        id: patch.id,
                        title: patch.title,
                        description: patch.description,
                        state: patch.state,
                        base: patch.base,
                        head: patch.head,
                        labels: patch.labels,
                        assignees: patch.assignees,
                        revision_count: patch.revision_count,
                        approval_count: patch.approval_count,
                        created_at_ms: patch.created_at_ms,
                        updated_at_ms: patch.updated_at_ms,
                        comments: result.comments.map(|cs| {
                            cs.into_iter()
                                .map(|c| crate::output::CommentOutput {
                                    hash: c.hash,
                                    author: c.author,
                                    body: c.body,
                                    timestamp_ms: c.timestamp_ms,
                                })
                                .collect()
                        }),
                        revisions: result.revisions.map(|rs| {
                            rs.into_iter()
                                .map(|r| crate::output::RevisionOutput {
                                    hash: r.hash,
                                    head: r.head,
                                    message: r.message,
                                    author: r.author,
                                    timestamp_ms: r.timestamp_ms,
                                })
                                .collect()
                        }),
                        approvals: result.approvals.map(|as_| {
                            as_.into_iter()
                                .map(|a| crate::output::ApprovalOutput {
                                    author: a.author,
                                    commit: a.commit,
                                    message: a.message,
                                    timestamp_ms: a.timestamp_ms,
                                })
                                .collect()
                        }),
                    };
                    print_output(&output, json);
                } else if !json {
                    println!("Patch not found");
                }
                Ok(())
            } else {
                anyhow::bail!("{}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
        }
        ClientRpcResponse::ForgeOperationResult(result) => {
            anyhow::bail!("{}", result.error.unwrap_or_else(|| "not found".to_string()))
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn patch_approve(client: &AspenClient, args: PatchApproveArgs, json: bool) -> Result<()> {
    // First get the patch to find current head
    let show_response = client
        .send(ClientRpcRequest::ForgeGetPatch {
            repo_id: args.repo.clone(),
            patch_id: args.patch.clone(),
        })
        .await?;

    let commit = match show_response {
        ClientRpcResponse::ForgePatchResult(result) => {
            if let Some(patch) = result.patch {
                patch.head
            } else {
                anyhow::bail!("Patch not found");
            }
        }
        ClientRpcResponse::ForgeOperationResult(result) => {
            anyhow::bail!("{}", result.error.unwrap_or_else(|| "patch not found".to_string()))
        }
        _ => anyhow::bail!("unexpected response type"),
    };

    let response = client
        .send(ClientRpcRequest::ForgeApprovePatch {
            repo_id: args.repo,
            patch_id: args.patch.clone(),
            commit,
            message: args.message,
        })
        .await?;

    match response {
        ClientRpcResponse::ForgePatchResult(result) => {
            if result.success {
                if !json {
                    println!("Patch {} approved", args.patch);
                } else {
                    println!(r#"{{"approved": true, "patch": "{}"}}"#, args.patch);
                }
                Ok(())
            } else {
                anyhow::bail!("{}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
        }
        ClientRpcResponse::ForgeOperationResult(result) => {
            if result.success {
                if !json {
                    println!("Patch {} approved", args.patch);
                }
                Ok(())
            } else {
                anyhow::bail!("{}", result.error.unwrap_or_else(|| "operation failed".to_string()))
            }
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn patch_merge(client: &AspenClient, args: PatchMergeArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::ForgeMergePatch {
            repo_id: args.repo,
            patch_id: args.patch.clone(),
            merge_commit: args.merge_commit,
        })
        .await?;

    match response {
        ClientRpcResponse::ForgePatchResult(result) => {
            if result.success {
                if !json {
                    println!("Patch {} merged", args.patch);
                } else {
                    println!(r#"{{"merged": true, "patch": "{}"}}"#, args.patch);
                }
                Ok(())
            } else {
                anyhow::bail!("{}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
        }
        ClientRpcResponse::ForgeOperationResult(result) => {
            if result.success {
                if !json {
                    println!("Patch {} merged", args.patch);
                }
                Ok(())
            } else {
                anyhow::bail!("{}", result.error.unwrap_or_else(|| "operation failed".to_string()))
            }
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn patch_close(client: &AspenClient, args: PatchCloseArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::ForgeClosePatch {
            repo_id: args.repo,
            patch_id: args.patch.clone(),
            reason: args.reason,
        })
        .await?;

    match response {
        ClientRpcResponse::ForgePatchResult(result) => {
            if result.success {
                if !json {
                    println!("Patch {} closed", args.patch);
                } else {
                    println!(r#"{{"closed": true, "patch": "{}"}}"#, args.patch);
                }
                Ok(())
            } else {
                anyhow::bail!("{}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
        }
        ClientRpcResponse::ForgeOperationResult(result) => {
            if result.success {
                if !json {
                    println!("Patch {} closed", args.patch);
                }
                Ok(())
            } else {
                anyhow::bail!("{}", result.error.unwrap_or_else(|| "operation failed".to_string()))
            }
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}
