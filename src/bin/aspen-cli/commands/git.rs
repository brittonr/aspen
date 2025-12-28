//! Git repository commands.
//!
//! Commands for managing git repositories, commits, and refs
//! on the decentralized Forge.

use anyhow::Result;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::print_output;
use crate::output::CommitOutput;
use crate::output::LogOutput;
use crate::output::RefOutput;
use crate::output::RepoOutput;
use aspen::client_rpc::ClientRpcRequest;
use aspen::client_rpc::ClientRpcResponse;

/// Git repository operations.
#[derive(Subcommand)]
pub enum GitCommand {
    /// Initialize a new repository.
    Init(InitArgs),

    /// Show repository information.
    Show(ShowArgs),

    /// Create a new commit.
    Commit(CommitArgs),

    /// Show commit history.
    Log(LogArgs),

    /// Push refs to the remote.
    Push(PushArgs),

    /// Get a ref value.
    GetRef(GetRefArgs),

    /// Store a blob (file content).
    StoreBlob(StoreBlobArgs),

    /// Get a blob by hash.
    GetBlob(GetBlobArgs),
}

#[derive(Args)]
pub struct InitArgs {
    /// Repository name.
    pub name: String,

    /// Optional description.
    #[arg(short, long)]
    pub description: Option<String>,

    /// Default branch name.
    #[arg(long, default_value = "main")]
    pub default_branch: String,
}

#[derive(Args)]
pub struct ShowArgs {
    /// Repository ID (hex-encoded).
    #[arg(short, long)]
    pub repo: String,
}

#[derive(Args)]
pub struct CommitArgs {
    /// Repository ID.
    #[arg(short, long)]
    pub repo: String,

    /// Tree hash (hex-encoded).
    #[arg(long)]
    pub tree: String,

    /// Parent commit hashes (hex-encoded).
    #[arg(long)]
    pub parent: Vec<String>,

    /// Commit message.
    #[arg(short, long)]
    pub message: String,
}

#[derive(Args)]
pub struct LogArgs {
    /// Repository ID.
    #[arg(short, long)]
    pub repo: String,

    /// Ref name (e.g., heads/main). Uses default branch if not specified.
    #[arg(long)]
    pub ref_name: Option<String>,

    /// Maximum commits to show.
    #[arg(short = 'n', long, default_value = "10")]
    pub limit: u32,
}

#[derive(Args)]
pub struct PushArgs {
    /// Repository ID.
    #[arg(short, long)]
    pub repo: String,

    /// Ref name (e.g., heads/main).
    #[arg(long)]
    pub ref_name: String,

    /// Commit hash to set ref to.
    #[arg(long)]
    pub hash: String,

    /// Force push (skip conflict check).
    #[arg(short, long)]
    pub force: bool,
}

#[derive(Args)]
pub struct GetRefArgs {
    /// Repository ID.
    #[arg(short, long)]
    pub repo: String,

    /// Ref name (e.g., heads/main, tags/v1.0).
    pub ref_name: String,
}

#[derive(Args)]
pub struct StoreBlobArgs {
    /// Repository ID.
    #[arg(short, long)]
    pub repo: String,

    /// File to store.
    pub file: std::path::PathBuf,
}

#[derive(Args)]
pub struct GetBlobArgs {
    /// Blob hash (hex-encoded BLAKE3).
    pub hash: String,

    /// Output file (stdout if not specified).
    #[arg(short, long)]
    pub output: Option<std::path::PathBuf>,
}

impl GitCommand {
    /// Execute the git command.
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            GitCommand::Init(args) => git_init(client, args, json).await,
            GitCommand::Show(args) => git_show(client, args, json).await,
            GitCommand::Commit(args) => git_commit(client, args, json).await,
            GitCommand::Log(args) => git_log(client, args, json).await,
            GitCommand::Push(args) => git_push(client, args, json).await,
            GitCommand::GetRef(args) => git_get_ref(client, args, json).await,
            GitCommand::StoreBlob(args) => git_store_blob(client, args, json).await,
            GitCommand::GetBlob(args) => git_get_blob(client, args, json).await,
        }
    }
}

async fn git_init(client: &AspenClient, args: InitArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::ForgeCreateRepo {
            name: args.name,
            description: args.description,
            default_branch: Some(args.default_branch),
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeRepoResult(result) => {
            if result.success {
                if let Some(repo) = result.repo {
                    let output = RepoOutput {
                        id: repo.id,
                        name: repo.name,
                        description: repo.description,
                        default_branch: repo.default_branch,
                        delegates: repo.delegates,
                        threshold: repo.threshold,
                        created_at_ms: repo.created_at_ms,
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
                    println!("Repository created");
                }
                Ok(())
            } else {
                anyhow::bail!("{}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn git_show(client: &AspenClient, args: ShowArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::ForgeGetRepo { repo_id: args.repo })
        .await?;

    match response {
        ClientRpcResponse::ForgeRepoResult(result) => {
            if result.success {
                if let Some(repo) = result.repo {
                    let output = RepoOutput {
                        id: repo.id,
                        name: repo.name,
                        description: repo.description,
                        default_branch: repo.default_branch,
                        delegates: repo.delegates,
                        threshold: repo.threshold,
                        created_at_ms: repo.created_at_ms,
                    };
                    print_output(&output, json);
                } else if !json {
                    println!("Repository not found");
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

async fn git_commit(client: &AspenClient, args: CommitArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::ForgeCommit {
            repo_id: args.repo,
            tree: args.tree,
            parents: args.parent,
            message: args.message,
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeCommitResult(result) => {
            if result.success {
                if let Some(commit) = result.commit {
                    let output = CommitOutput {
                        hash: commit.hash,
                        tree: commit.tree,
                        parents: commit.parents,
                        author_name: commit.author_name,
                        author_email: commit.author_email,
                        message: commit.message,
                        timestamp_ms: commit.timestamp_ms,
                    };
                    print_output(&output, json);
                }
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

async fn git_log(client: &AspenClient, args: LogArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::ForgeLog {
            repo_id: args.repo,
            ref_name: args.ref_name,
            limit: Some(args.limit),
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeLogResult(result) => {
            if result.success {
                let commits: Vec<CommitOutput> = result
                    .commits
                    .into_iter()
                    .map(|c| CommitOutput {
                        hash: c.hash,
                        tree: c.tree,
                        parents: c.parents,
                        author_name: c.author_name,
                        author_email: c.author_email,
                        message: c.message,
                        timestamp_ms: c.timestamp_ms,
                    })
                    .collect();
                let output = LogOutput {
                    commits,
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

async fn git_push(client: &AspenClient, args: PushArgs, json: bool) -> Result<()> {
    let response = if args.force {
        client
            .send(ClientRpcRequest::ForgeSetRef {
                repo_id: args.repo,
                ref_name: args.ref_name.clone(),
                hash: args.hash,
            })
            .await?
    } else {
        // Use CAS for safe push
        client
            .send(ClientRpcRequest::ForgeCasRef {
                repo_id: args.repo,
                ref_name: args.ref_name.clone(),
                expected: None, // TODO: get current ref value first
                new_hash: args.hash,
            })
            .await?
    };

    match response {
        ClientRpcResponse::ForgeRefResult(result) => {
            if result.success {
                if let Some(ref_info) = result.ref_info {
                    let output = RefOutput {
                        name: ref_info.name,
                        hash: ref_info.hash,
                    };
                    print_output(&output, json);
                } else if !json {
                    println!("Ref {} updated", args.ref_name);
                }
                Ok(())
            } else {
                anyhow::bail!("{}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
        }
        ClientRpcResponse::ForgeOperationResult(result) => {
            if result.success {
                if !json {
                    println!("Ref {} updated", args.ref_name);
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

async fn git_get_ref(client: &AspenClient, args: GetRefArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::ForgeGetRef {
            repo_id: args.repo,
            ref_name: args.ref_name,
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeRefResult(result) => {
            if result.success {
                if let Some(ref_info) = result.ref_info {
                    let output = RefOutput {
                        name: ref_info.name,
                        hash: ref_info.hash,
                    };
                    print_output(&output, json);
                } else if !json {
                    println!("Ref not found");
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

async fn git_store_blob(client: &AspenClient, args: StoreBlobArgs, json: bool) -> Result<()> {
    let content = std::fs::read(&args.file).map_err(|e| anyhow::anyhow!("failed to read file: {}", e))?;

    let response = client
        .send(ClientRpcRequest::ForgeStoreBlob {
            repo_id: args.repo,
            content,
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeBlobResult(result) => {
            if result.success {
                if json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "hash": result.hash,
                            "size": result.size
                        })
                    );
                } else if let Some(hash) = result.hash {
                    println!("{}", hash);
                }
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

async fn git_get_blob(client: &AspenClient, args: GetBlobArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::ForgeGetBlob { hash: args.hash })
        .await?;

    match response {
        ClientRpcResponse::ForgeBlobResult(result) => {
            if result.success {
                if let Some(content) = result.content {
                    if let Some(ref path) = args.output {
                        std::fs::write(path, &content).map_err(|e| anyhow::anyhow!("failed to write file: {}", e))?;
                        if !json {
                            println!("Written {} bytes to {:?}", content.len(), path);
                        }
                    } else if json {
                        println!(
                            "{}",
                            serde_json::json!({
                                "hash": result.hash,
                                "size": result.size,
                                "content_base64": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &content)
                            })
                        );
                    } else {
                        // Write raw bytes to stdout
                        use std::io::Write;
                        std::io::stdout().write_all(&content)?;
                    }
                } else if !json {
                    println!("Blob not found");
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
