//! Patch management commands.
//!
//! Commands for managing collaborative object (COB) patches (pull requests).

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::messages::ForgeCommentInfo;
use aspen_client_api::messages::ForgePatchApproval;
use aspen_client_api::messages::ForgePatchInfo;
use aspen_client_api::messages::ForgePatchResultResponse;
use aspen_client_api::messages::ForgePatchRevision;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::PatchDetailOutput;
use crate::output::PatchListOutput;
use crate::output::PatchOutput;
use crate::output::print_output;

/// Patch management operations.
#[derive(Subcommand)]
pub enum PatchCommand {
    /// List patches in a repository.
    List(PatchListArgs),

    /// Create a new patch.
    Create(PatchCreateArgs),

    /// Show patch details.
    Show(PatchShowArgs),

    /// Update a patch (push new commits).
    Update(PatchUpdateArgs),

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
    #[arg(short = 'n', long = "limit", default_value = "20")]
    pub max_results: u32,
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
pub struct PatchUpdateArgs {
    /// Repository ID.
    #[arg(short, long)]
    pub repo: String,

    /// Patch ID (hex-encoded).
    pub patch: String,

    /// New head commit hash.
    #[arg(long)]
    pub head: String,

    /// Update message describing the changes.
    #[arg(short, long)]
    pub message: Option<String>,
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

    /// Merge strategy: merge (default), fast-forward, squash.
    #[arg(short, long)]
    pub strategy: Option<String>,

    /// Custom merge commit message.
    #[arg(short, long)]
    pub message: Option<String>,
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
    pub async fn run(self, client: &AspenClient, is_json: bool) -> Result<()> {
        match self {
            PatchCommand::List(args) => patch_list(client, args, is_json).await,
            PatchCommand::Create(args) => patch_create(client, args, is_json).await,
            PatchCommand::Show(args) => patch_show(client, args, is_json).await,
            PatchCommand::Update(args) => patch_update(client, args, is_json).await,
            PatchCommand::Approve(args) => patch_approve(client, args, is_json).await,
            PatchCommand::Merge(args) => patch_merge(client, args, is_json).await,
            PatchCommand::Close(args) => patch_close(client, args, is_json).await,
        }
    }
}

fn build_patch_output(patch: ForgePatchInfo) -> PatchOutput {
    PatchOutput {
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
    }
}

fn comment_output(comment: ForgeCommentInfo) -> crate::output::CommentOutput {
    crate::output::CommentOutput {
        hash: comment.hash,
        author: comment.author,
        body: comment.body,
        timestamp_ms: comment.timestamp_ms,
    }
}

fn revision_output(revision: ForgePatchRevision) -> crate::output::RevisionOutput {
    crate::output::RevisionOutput {
        hash: revision.hash,
        head: revision.head,
        message: revision.message,
        author: revision.author,
        timestamp_ms: revision.timestamp_ms,
    }
}

fn approval_output(approval: ForgePatchApproval) -> crate::output::ApprovalOutput {
    crate::output::ApprovalOutput {
        author: approval.author,
        commit: approval.commit,
        message: approval.message,
        timestamp_ms: approval.timestamp_ms,
    }
}

fn build_patch_detail_output(result: ForgePatchResultResponse, patch: ForgePatchInfo) -> PatchDetailOutput {
    PatchDetailOutput {
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
        comments: result.comments.map(|comments| comments.into_iter().map(comment_output).collect()),
        revisions: result.revisions.map(|revisions| revisions.into_iter().map(revision_output).collect()),
        approvals: result.approvals.map(|approvals| approvals.into_iter().map(approval_output).collect()),
    }
}

#[derive(Clone, Copy)]
enum PatchAction {
    Updated,
    Approved,
    Closed,
}

impl PatchAction {
    fn json_key(self) -> &'static str {
        match self {
            Self::Updated => "updated",
            Self::Approved => "approved",
            Self::Closed => "closed",
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::Updated => "updated",
            Self::Approved => "approved",
            Self::Closed => "closed",
        }
    }
}

fn print_patch_status(action: PatchAction, patch_id: &str, is_json: bool) {
    debug_assert!(!patch_id.is_empty());
    debug_assert!(!action.message().is_empty());
    if is_json {
        println!(r#"{{"{}": true, "patch": "{}"}}"#, action.json_key(), patch_id);
        return;
    }
    println!("Patch {} {}", patch_id, action.message());
}

fn print_merge_status(patch_id: &str, merge_commit: Option<&str>, is_json: bool) {
    debug_assert!(!patch_id.is_empty());
    debug_assert!(!is_json || merge_commit.is_none() || !merge_commit.unwrap_or_default().is_empty());
    if is_json {
        let merge_commit_json =
            merge_commit.map(|commit| format!(r#""{commit}""#)).unwrap_or_else(|| "null".to_string());
        println!(r#"{{"merged": true, "patch": "{}", "merge_commit": {}}}"#, patch_id, merge_commit_json);
        return;
    }
    if let Some(commit) = merge_commit {
        println!("Patch {} merged ({})", patch_id, commit);
        return;
    }
    println!("Patch {} merged", patch_id);
}

fn print_patch_created(is_json: bool) {
    if !is_json {
        println!("Patch created");
    }
}

fn print_patch_not_found(is_json: bool) {
    if !is_json {
        println!("Patch not found");
    }
}

async fn patch_list(client: &AspenClient, args: PatchListArgs, is_json: bool) -> Result<()> {
    debug_assert!(!args.repo.is_empty());
    debug_assert!(args.max_results > 0);
    let state = match args.state.as_str() {
        "all" => None,
        state_name => Some(state_name.to_string()),
    };
    debug_assert!(state.is_none() || !state.as_deref().unwrap_or_default().is_empty());

    let response = client
        .send(ClientRpcRequest::ForgeListPatches {
            repo_id: args.repo,
            state,
            limit: Some(args.max_results),
        })
        .await?;

    match response {
        ClientRpcResponse::ForgePatchListResult(result) => {
            if result.is_success {
                let patches = result.patches.into_iter().map(build_patch_output).collect();
                let output = PatchListOutput {
                    patches,
                    count: result.count,
                };
                print_output(&output, is_json);
                return Ok(());
            }
            anyhow::bail!("{}", result.error.unwrap_or_else(|| "unknown error".to_string()))
        }
        ClientRpcResponse::ForgeOperationResult(result) => {
            anyhow::bail!("{}", result.error.unwrap_or_else(|| "operation failed".to_string()))
        }
        ClientRpcResponse::Error(error) => anyhow::bail!("{}: {}", error.code, error.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn patch_create(client: &AspenClient, args: PatchCreateArgs, is_json: bool) -> Result<()> {
    debug_assert!(!args.repo.is_empty());
    debug_assert!(!args.title.is_empty());
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
            if result.is_success {
                if let Some(patch) = result.patch {
                    print_output(&build_patch_output(patch), is_json);
                }
                return Ok(());
            }
            anyhow::bail!("{}", result.error.unwrap_or_else(|| "unknown error".to_string()))
        }
        ClientRpcResponse::ForgeOperationResult(result) => {
            if result.is_success {
                print_patch_created(is_json);
                return Ok(());
            }
            anyhow::bail!("{}", result.error.unwrap_or_else(|| "operation failed".to_string()))
        }
        ClientRpcResponse::Error(error) => anyhow::bail!("{}: {}", error.code, error.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn patch_show(client: &AspenClient, args: PatchShowArgs, is_json: bool) -> Result<()> {
    debug_assert!(!args.repo.is_empty());
    debug_assert!(!args.patch.is_empty());
    let response = client
        .send(ClientRpcRequest::ForgeGetPatch {
            repo_id: args.repo,
            patch_id: args.patch,
        })
        .await?;

    match response {
        ClientRpcResponse::ForgePatchResult(result) => {
            if result.is_success {
                if let Some(patch) = result.patch.clone() {
                    let output = build_patch_detail_output(result, patch);
                    print_output(&output, is_json);
                } else {
                    print_patch_not_found(is_json);
                }
                return Ok(());
            }
            anyhow::bail!("{}", result.error.unwrap_or_else(|| "unknown error".to_string()))
        }
        ClientRpcResponse::ForgeOperationResult(result) => {
            anyhow::bail!("{}", result.error.unwrap_or_else(|| "not found".to_string()))
        }
        ClientRpcResponse::Error(error) => anyhow::bail!("{}: {}", error.code, error.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn patch_update(client: &AspenClient, args: PatchUpdateArgs, is_json: bool) -> Result<()> {
    debug_assert!(!args.repo.is_empty());
    debug_assert!(!args.patch.is_empty());
    let patch_id = args.patch.clone();
    let response = client
        .send(ClientRpcRequest::ForgeUpdatePatch {
            repo_id: args.repo,
            patch_id: patch_id.clone(),
            head: args.head,
            message: args.message,
        })
        .await?;

    match response {
        ClientRpcResponse::ForgePatchResult(result) => {
            if result.is_success {
                if let Some(patch) = result.patch {
                    print_output(&build_patch_output(patch), is_json);
                } else {
                    print_patch_status(PatchAction::Updated, &patch_id, is_json);
                }
                return Ok(());
            }
            anyhow::bail!("{}", result.error.unwrap_or_else(|| "unknown error".to_string()))
        }
        ClientRpcResponse::ForgeOperationResult(result) => {
            if result.is_success {
                print_patch_status(PatchAction::Updated, &patch_id, is_json);
                return Ok(());
            }
            anyhow::bail!("{}", result.error.unwrap_or_else(|| "operation failed".to_string()))
        }
        ClientRpcResponse::Error(error) => anyhow::bail!("{}: {}", error.code, error.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn patch_approve(client: &AspenClient, args: PatchApproveArgs, is_json: bool) -> Result<()> {
    debug_assert!(!args.repo.is_empty());
    debug_assert!(!args.patch.is_empty());
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
    debug_assert!(!commit.is_empty());

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
            if result.is_success {
                print_patch_status(PatchAction::Approved, &args.patch, is_json);
                return Ok(());
            }
            anyhow::bail!("{}", result.error.unwrap_or_else(|| "unknown error".to_string()))
        }
        ClientRpcResponse::ForgeOperationResult(result) => {
            if result.is_success {
                print_patch_status(PatchAction::Approved, &args.patch, is_json);
                return Ok(());
            }
            anyhow::bail!("{}", result.error.unwrap_or_else(|| "operation failed".to_string()))
        }
        ClientRpcResponse::Error(error) => anyhow::bail!("{}: {}", error.code, error.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn patch_merge(client: &AspenClient, args: PatchMergeArgs, is_json: bool) -> Result<()> {
    debug_assert!(!args.repo.is_empty());
    debug_assert!(!args.patch.is_empty());
    let patch_id = args.patch.clone();
    let response = client
        .send(ClientRpcRequest::ForgeMergePatch {
            repo_id: args.repo,
            patch_id: patch_id.clone(),
            strategy: args.strategy,
            message: args.message,
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeMergeCheckResult(result) => {
            if result.is_success {
                print_merge_status(&patch_id, result.merge_commit.as_deref(), is_json);
                return Ok(());
            }
            anyhow::bail!("{}", result.error.unwrap_or_else(|| "merge failed".to_string()))
        }
        ClientRpcResponse::ForgeOperationResult(result) => {
            if result.is_success {
                print_merge_status(&patch_id, None, is_json);
                return Ok(());
            }
            anyhow::bail!("{}", result.error.unwrap_or_else(|| "operation failed".to_string()))
        }
        ClientRpcResponse::Error(error) => anyhow::bail!("{}: {}", error.code, error.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn patch_close(client: &AspenClient, args: PatchCloseArgs, is_json: bool) -> Result<()> {
    debug_assert!(!args.repo.is_empty());
    debug_assert!(!args.patch.is_empty());
    let patch_id = args.patch.clone();
    let response = client
        .send(ClientRpcRequest::ForgeClosePatch {
            repo_id: args.repo,
            patch_id: patch_id.clone(),
            reason: args.reason,
        })
        .await?;

    match response {
        ClientRpcResponse::ForgePatchResult(result) => {
            if result.is_success {
                print_patch_status(PatchAction::Closed, &patch_id, is_json);
                return Ok(());
            }
            anyhow::bail!("{}", result.error.unwrap_or_else(|| "unknown error".to_string()))
        }
        ClientRpcResponse::ForgeOperationResult(result) => {
            if result.is_success {
                print_patch_status(PatchAction::Closed, &patch_id, is_json);
                return Ok(());
            }
            anyhow::bail!("{}", result.error.unwrap_or_else(|| "operation failed".to_string()))
        }
        ClientRpcResponse::Error(error) => anyhow::bail!("{}: {}", error.code, error.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}
