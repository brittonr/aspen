//! Issue management commands.
//!
//! Commands for managing collaborative object (COB) issues.

use anyhow::Result;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::print_output;
use crate::output::IssueDetailOutput;
use crate::output::IssueListOutput;
use crate::output::IssueOutput;
use aspen::client_rpc::ClientRpcRequest;
use aspen::client_rpc::ClientRpcResponse;

/// Issue management operations.
#[derive(Subcommand)]
pub enum IssueCommand {
    /// List issues in a repository.
    List(IssueListArgs),

    /// Create a new issue.
    Create(IssueCreateArgs),

    /// Show issue details.
    Show(IssueShowArgs),

    /// Close an issue.
    Close(IssueCloseArgs),

    /// Reopen an issue.
    Reopen(IssueReopenArgs),

    /// Add a comment to an issue.
    Comment(IssueCommentArgs),
}

#[derive(Args)]
pub struct IssueListArgs {
    /// Repository ID.
    #[arg(short, long)]
    pub repo: String,

    /// Filter by state: open, closed, all.
    #[arg(long, default_value = "open")]
    pub state: String,

    /// Maximum issues to show.
    #[arg(short = 'n', long, default_value = "20")]
    pub limit: u32,
}

#[derive(Args)]
pub struct IssueCreateArgs {
    /// Repository ID.
    #[arg(short, long)]
    pub repo: String,

    /// Issue title.
    #[arg(short, long)]
    pub title: String,

    /// Issue body.
    #[arg(short, long, default_value = "")]
    pub body: String,

    /// Labels (comma-separated).
    #[arg(short, long)]
    pub labels: Option<String>,
}

#[derive(Args)]
pub struct IssueShowArgs {
    /// Repository ID.
    #[arg(short, long)]
    pub repo: String,

    /// Issue ID (hex-encoded).
    pub issue: String,
}

#[derive(Args)]
pub struct IssueCloseArgs {
    /// Repository ID.
    #[arg(short, long)]
    pub repo: String,

    /// Issue ID (hex-encoded).
    pub issue: String,

    /// Reason for closing.
    #[arg(long)]
    pub reason: Option<String>,
}

#[derive(Args)]
pub struct IssueReopenArgs {
    /// Repository ID.
    #[arg(short, long)]
    pub repo: String,

    /// Issue ID (hex-encoded).
    pub issue: String,
}

#[derive(Args)]
pub struct IssueCommentArgs {
    /// Repository ID.
    #[arg(short, long)]
    pub repo: String,

    /// Issue ID (hex-encoded).
    pub issue: String,

    /// Comment body.
    #[arg(short, long)]
    pub body: String,
}

impl IssueCommand {
    /// Execute the issue command.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            IssueCommand::List(args) => issue_list(client, args, json).await,
            IssueCommand::Create(args) => issue_create(client, args, json).await,
            IssueCommand::Show(args) => issue_show(client, args, json).await,
            IssueCommand::Close(args) => issue_close(client, args, json).await,
            IssueCommand::Reopen(args) => issue_reopen(client, args, json).await,
            IssueCommand::Comment(args) => issue_comment(client, args, json).await,
        }
    }
}

async fn issue_list(client: &AspenClient, args: IssueListArgs, json: bool) -> Result<()> {
    let state = match args.state.as_str() {
        "all" => None,
        s => Some(s.to_string()),
    };

    let response = client
        .send(ClientRpcRequest::ForgeListIssues {
            repo_id: args.repo,
            state,
            limit: Some(args.limit),
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeIssueListResult(result) => {
            if result.success {
                let issues: Vec<IssueOutput> = result
                    .issues
                    .into_iter()
                    .map(|i| IssueOutput {
                        id: i.id,
                        title: i.title,
                        state: i.state,
                        labels: i.labels,
                        comment_count: i.comment_count,
                        created_at_ms: i.created_at_ms,
                        updated_at_ms: i.updated_at_ms,
                    })
                    .collect();
                let output = IssueListOutput {
                    issues,
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

async fn issue_create(client: &AspenClient, args: IssueCreateArgs, json: bool) -> Result<()> {
    let labels: Vec<String> = args
        .labels
        .map(|s| s.split(',').map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
        .unwrap_or_default();

    let response = client
        .send(ClientRpcRequest::ForgeCreateIssue {
            repo_id: args.repo,
            title: args.title,
            body: args.body,
            labels,
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeIssueResult(result) => {
            if result.success {
                if let Some(issue) = result.issue {
                    let output = IssueOutput {
                        id: issue.id,
                        title: issue.title,
                        state: issue.state,
                        labels: issue.labels,
                        comment_count: issue.comment_count,
                        created_at_ms: issue.created_at_ms,
                        updated_at_ms: issue.updated_at_ms,
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
                    println!("Issue created");
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

async fn issue_show(client: &AspenClient, args: IssueShowArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::ForgeGetIssue {
            repo_id: args.repo,
            issue_id: args.issue,
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeIssueResult(result) => {
            if result.success {
                if let Some(issue) = result.issue {
                    let output = IssueDetailOutput {
                        id: issue.id,
                        title: issue.title,
                        body: issue.body,
                        state: issue.state,
                        labels: issue.labels,
                        assignees: issue.assignees,
                        comment_count: issue.comment_count,
                        created_at_ms: issue.created_at_ms,
                        updated_at_ms: issue.updated_at_ms,
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
                    };
                    print_output(&output, json);
                } else if !json {
                    println!("Issue not found");
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

async fn issue_close(client: &AspenClient, args: IssueCloseArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::ForgeCloseIssue {
            repo_id: args.repo,
            issue_id: args.issue.clone(),
            reason: args.reason,
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeIssueResult(result) => {
            if result.success {
                if !json {
                    println!("Issue {} closed", args.issue);
                } else {
                    println!(r#"{{"closed": true, "issue": "{}"}}"#, args.issue);
                }
                Ok(())
            } else {
                anyhow::bail!("{}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
        }
        ClientRpcResponse::ForgeOperationResult(result) => {
            if result.success {
                if !json {
                    println!("Issue {} closed", args.issue);
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

async fn issue_reopen(client: &AspenClient, args: IssueReopenArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::ForgeReopenIssue {
            repo_id: args.repo,
            issue_id: args.issue.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeIssueResult(result) => {
            if result.success {
                if !json {
                    println!("Issue {} reopened", args.issue);
                } else {
                    println!(r#"{{"reopened": true, "issue": "{}"}}"#, args.issue);
                }
                Ok(())
            } else {
                anyhow::bail!("{}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
        }
        ClientRpcResponse::ForgeOperationResult(result) => {
            if result.success {
                if !json {
                    println!("Issue {} reopened", args.issue);
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

async fn issue_comment(client: &AspenClient, args: IssueCommentArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::ForgeCommentIssue {
            repo_id: args.repo,
            issue_id: args.issue.clone(),
            body: args.body,
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeIssueResult(result) => {
            if result.success {
                if !json {
                    println!("Comment added to issue {}", args.issue);
                } else {
                    println!(r#"{{"commented": true, "issue": "{}"}}"#, args.issue);
                }
                Ok(())
            } else {
                anyhow::bail!("{}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
        }
        ClientRpcResponse::ForgeOperationResult(result) => {
            if result.success {
                if !json {
                    println!("Comment added to issue {}", args.issue);
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
