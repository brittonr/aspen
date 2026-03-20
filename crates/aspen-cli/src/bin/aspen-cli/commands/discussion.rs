//! Discussion management commands.
//!
//! Commands for managing collaborative object (COB) discussions.

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;

/// Discussion management operations.
#[derive(Subcommand)]
pub enum DiscussionCommand {
    /// List discussions in a repository.
    List(DiscussionListArgs),

    /// Create a new discussion.
    Create(DiscussionCreateArgs),

    /// Show discussion details.
    Show(DiscussionShowArgs),

    /// Reply to a discussion.
    Reply(DiscussionReplyArgs),

    /// Close a discussion.
    Close(DiscussionCloseArgs),

    /// Reopen a discussion.
    Reopen(DiscussionReopenArgs),

    /// Lock a discussion.
    Lock(DiscussionLockArgs),

    /// Unlock a discussion.
    Unlock(DiscussionUnlockArgs),
}

#[derive(Args, Clone)]
pub struct DiscussionListArgs {
    /// Repository ID.
    #[arg(short, long)]
    pub repo: String,

    /// Filter by state: open, closed, locked, all.
    #[arg(long, default_value = "open")]
    pub state: String,

    /// Maximum discussions to show.
    #[arg(short = 'n', long, default_value = "20")]
    pub limit: u32,
}

#[derive(Args, Clone)]
pub struct DiscussionCreateArgs {
    /// Repository ID.
    #[arg(short, long)]
    pub repo: String,

    /// Discussion title.
    #[arg(short, long)]
    pub title: String,

    /// Discussion body.
    #[arg(short, long, default_value = "")]
    pub body: String,

    /// Labels (comma-separated).
    #[arg(short, long)]
    pub labels: Option<String>,
}

#[derive(Args, Clone)]
pub struct DiscussionShowArgs {
    /// Repository ID.
    #[arg(short, long)]
    pub repo: String,

    /// Discussion ID (hex-encoded).
    pub discussion: String,
}

#[derive(Args, Clone)]
pub struct DiscussionReplyArgs {
    /// Repository ID.
    #[arg(short, long)]
    pub repo: String,

    /// Discussion ID (hex-encoded).
    pub discussion: String,

    /// Reply body.
    #[arg(short, long)]
    pub body: String,

    /// Parent reply hash (for threaded replies).
    #[arg(long)]
    pub parent: Option<String>,
}

#[derive(Args, Clone)]
pub struct DiscussionCloseArgs {
    /// Repository ID.
    #[arg(short, long)]
    pub repo: String,

    /// Discussion ID (hex-encoded).
    pub discussion: String,

    /// Reason for closing.
    #[arg(long)]
    pub reason: Option<String>,
}

#[derive(Args, Clone)]
pub struct DiscussionReopenArgs {
    /// Repository ID.
    #[arg(short, long)]
    pub repo: String,

    /// Discussion ID (hex-encoded).
    pub discussion: String,
}

#[derive(Args, Clone)]
pub struct DiscussionLockArgs {
    /// Repository ID.
    #[arg(short, long)]
    pub repo: String,

    /// Discussion ID (hex-encoded).
    pub discussion: String,
}

#[derive(Args, Clone)]
pub struct DiscussionUnlockArgs {
    /// Repository ID.
    #[arg(short, long)]
    pub repo: String,

    /// Discussion ID (hex-encoded).
    pub discussion: String,
}

impl DiscussionCommand {
    pub async fn execute(&self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            Self::List(args) => discussion_list(client, args.clone(), json).await,
            Self::Create(args) => discussion_create(client, args.clone(), json).await,
            Self::Show(args) => discussion_show(client, args.clone(), json).await,
            Self::Reply(args) => discussion_reply(client, args.clone(), json).await,
            Self::Close(args) => discussion_close(client, args.clone(), json).await,
            Self::Reopen(args) => discussion_reopen(client, args.clone(), json).await,
            Self::Lock(args) => discussion_lock(client, args.clone(), json).await,
            Self::Unlock(args) => discussion_unlock(client, args.clone(), json).await,
        }
    }
}

async fn discussion_list(client: &AspenClient, args: DiscussionListArgs, json: bool) -> Result<()> {
    let state = if args.state == "all" { None } else { Some(args.state) };

    let response = client
        .send(ClientRpcRequest::ForgeListDiscussions {
            repo_id: args.repo,
            state,
            limit: Some(args.limit),
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeDiscussionListResult(result) => {
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                for discussion in &result.discussions {
                    println!(
                        "{} {} [{}] ({})",
                        discussion.id, discussion.title, discussion.state, discussion.reply_count
                    );
                }
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn discussion_create(client: &AspenClient, args: DiscussionCreateArgs, json: bool) -> Result<()> {
    let labels: Vec<String> = args
        .labels
        .map(|s| s.split(',').map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
        .unwrap_or_default();

    let response = client
        .send(ClientRpcRequest::ForgeCreateDiscussion {
            repo_id: args.repo,
            title: args.title,
            body: args.body,
            labels,
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeDiscussionResult(result) => {
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                let discussion = &result.discussion;
                println!("Created discussion {}: {}", discussion.id, discussion.title);
            }
            Ok(())
        }
        ClientRpcResponse::ForgeOperationResult(result) => {
            if result.is_success {
                if !json {
                    println!("Discussion created");
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

async fn discussion_show(client: &AspenClient, args: DiscussionShowArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::ForgeGetDiscussion {
            repo_id: args.repo,
            discussion_id: args.discussion,
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeDiscussionResult(result) => {
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                let discussion = &result.discussion;
                println!("Discussion {}: {}", discussion.id, discussion.title);
                println!("State: {}", discussion.state);
                println!("Replies: {}", discussion.reply_count);
                if !discussion.body.is_empty() {
                    println!("\n{}", discussion.body);
                }
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn discussion_reply(client: &AspenClient, args: DiscussionReplyArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::ForgeReplyDiscussion {
            repo_id: args.repo,
            discussion_id: args.discussion.clone(),
            body: args.body,
            parent_reply: args.parent,
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeOperationResult(result) => {
            if result.is_success {
                if !json {
                    println!("Reply added to discussion {}", args.discussion);
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

async fn discussion_close(client: &AspenClient, args: DiscussionCloseArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::ForgeCloseDiscussion {
            repo_id: args.repo,
            discussion_id: args.discussion.clone(),
            reason: args.reason,
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeOperationResult(result) => {
            if result.is_success {
                if !json {
                    println!("Discussion {} closed", args.discussion);
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

async fn discussion_reopen(client: &AspenClient, args: DiscussionReopenArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::ForgeReopenDiscussion {
            repo_id: args.repo,
            discussion_id: args.discussion.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeOperationResult(result) => {
            if result.is_success {
                if !json {
                    println!("Discussion {} reopened", args.discussion);
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

async fn discussion_lock(client: &AspenClient, args: DiscussionLockArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::ForgeLockDiscussion {
            repo_id: args.repo,
            discussion_id: args.discussion.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeOperationResult(result) => {
            if result.is_success {
                if !json {
                    println!("Discussion {} locked", args.discussion);
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

async fn discussion_unlock(client: &AspenClient, args: DiscussionUnlockArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::ForgeUnlockDiscussion {
            repo_id: args.repo,
            discussion_id: args.discussion.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeOperationResult(result) => {
            if result.is_success {
                if !json {
                    println!("Discussion {} unlocked", args.discussion);
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
