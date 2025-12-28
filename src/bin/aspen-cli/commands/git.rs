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
use crate::output::RepoListItem;
use crate::output::RepoListOutput;
use crate::output::RepoOutput;
use aspen::client_rpc::ClientRpcRequest;
use aspen::client_rpc::ClientRpcResponse;

/// Git repository operations.
#[derive(Subcommand)]
pub enum GitCommand {
    /// Initialize a new repository.
    Init(InitArgs),

    /// List all repositories.
    List(ListArgs),

    /// Clone a repository to a local directory.
    Clone(CloneArgs),

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
pub struct ListArgs {
    /// Maximum number of repositories to return.
    #[arg(short, long, default_value = "50")]
    pub limit: u32,

    /// Offset for pagination.
    #[arg(long, default_value = "0")]
    pub offset: u32,
}

#[derive(Args)]
pub struct CloneArgs {
    /// Repository ID (hex-encoded).
    pub repo_id: String,

    /// Local directory to clone into.
    #[arg(short, long)]
    pub path: Option<std::path::PathBuf>,

    /// Branch to clone (defaults to default_branch).
    #[arg(short, long)]
    pub branch: Option<String>,
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

    /// Path to secret key file for signing (required for canonical refs).
    #[arg(short, long)]
    pub key: Option<std::path::PathBuf>,
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
            GitCommand::List(args) => git_list(client, args, json).await,
            GitCommand::Clone(args) => git_clone(client, args, json).await,
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

async fn git_list(client: &AspenClient, args: ListArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::ForgeListRepos {
            limit: Some(args.limit),
            offset: Some(args.offset),
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeRepoListResult(result) => {
            if result.success {
                let output = RepoListOutput {
                    repos: result
                        .repos
                        .into_iter()
                        .map(|r| RepoListItem {
                            id: r.id,
                            name: r.name,
                            description: r.description,
                            default_branch: r.default_branch,
                        })
                        .collect(),
                    count: result.count,
                };
                print_output(&output, json);
                Ok(())
            } else {
                anyhow::bail!("{}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn git_clone(client: &AspenClient, args: CloneArgs, json: bool) -> Result<()> {
    use std::collections::HashSet;
    use std::fs;
    use std::io::Write;

    // Step 1: Get repository info
    let repo_response = client
        .send(ClientRpcRequest::ForgeGetRepo {
            repo_id: args.repo_id.clone(),
        })
        .await?;

    let repo_info = match repo_response {
        ClientRpcResponse::ForgeRepoResult(result) => {
            if result.success {
                result.repo.ok_or_else(|| anyhow::anyhow!("Repository not found"))?
            } else {
                anyhow::bail!("{}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    };

    // Determine target directory
    let target_dir = args.path.unwrap_or_else(|| {
        std::path::PathBuf::from(&repo_info.name)
    });

    if target_dir.exists() {
        anyhow::bail!("Directory '{}' already exists", target_dir.display());
    }

    // Create directory structure
    fs::create_dir_all(&target_dir)?;
    fs::create_dir_all(target_dir.join(".aspen"))?;
    fs::create_dir_all(target_dir.join(".aspen/objects"))?;
    fs::create_dir_all(target_dir.join(".aspen/refs/heads"))?;

    // Determine which branch to clone
    let branch_name = args.branch.unwrap_or_else(|| repo_info.default_branch.clone());
    let ref_name = format!("heads/{}", branch_name);

    // Step 2: Get the ref value
    let ref_response = client
        .send(ClientRpcRequest::ForgeGetRef {
            repo_id: args.repo_id.clone(),
            ref_name: ref_name.clone(),
        })
        .await?;

    let head_hash = match ref_response {
        ClientRpcResponse::ForgeRefResult(result) => {
            if result.success {
                result.ref_info
                    .map(|r| r.hash)
                    .ok_or_else(|| anyhow::anyhow!("Branch '{}' not found", branch_name))?
            } else {
                // Branch might not exist yet (empty repo)
                if !json {
                    println!("Created empty repository at '{}'", target_dir.display());
                }
                // Write repo config
                let config = serde_json::json!({
                    "repo_id": args.repo_id,
                    "name": repo_info.name,
                    "default_branch": repo_info.default_branch,
                });
                let config_path = target_dir.join(".aspen/config.json");
                let mut file = fs::File::create(&config_path)?;
                file.write_all(serde_json::to_string_pretty(&config)?.as_bytes())?;
                return Ok(());
            }
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    };

    // Step 3: Walk commit history and fetch commits
    let log_response = client
        .send(ClientRpcRequest::ForgeLog {
            repo_id: args.repo_id.clone(),
            ref_name: Some(ref_name.clone()),
            limit: Some(1000), // Fetch up to 1000 commits
        })
        .await?;

    let commits = match log_response {
        ClientRpcResponse::ForgeLogResult(result) => {
            if result.success {
                result.commits
            } else {
                anyhow::bail!("{}", result.error.unwrap_or_else(|| "failed to get log".to_string()))
            }
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    };

    // Step 4: Collect all tree hashes we need to fetch
    let mut tree_hashes: HashSet<String> = HashSet::new();
    for commit in &commits {
        tree_hashes.insert(commit.tree.clone());
    }

    // Step 5: Fetch trees and collect blob hashes
    let mut blob_hashes: HashSet<String> = HashSet::new();
    for tree_hash in &tree_hashes {
        let tree_response = client
            .send(ClientRpcRequest::ForgeGetTree {
                hash: tree_hash.clone(),
            })
            .await?;

        if let ClientRpcResponse::ForgeTreeResult(result) = tree_response {
            if result.success {
                if let Some(entries) = result.entries {
                    for entry in entries {
                        // Mode >= 0o100000 = blob (file), < 0o100000 = tree (directory)
                        // 0o100644 = regular file, 0o100755 = executable, 0o040000 = tree
                        if entry.mode >= 0o100000 {
                            blob_hashes.insert(entry.hash);
                        }
                        // Note: For simplicity, we don't recursively fetch subtrees
                    }
                }
            }
        }
    }

    // Step 6: Fetch blobs and write to objects directory
    let objects_dir = target_dir.join(".aspen/objects");
    for blob_hash in &blob_hashes {
        let blob_response = client
            .send(ClientRpcRequest::ForgeGetBlob {
                hash: blob_hash.clone(),
            })
            .await?;

        if let ClientRpcResponse::ForgeBlobResult(result) = blob_response {
            if result.success {
                if let Some(content) = result.content {
                    // Write blob to objects directory
                    let blob_path = objects_dir.join(&blob_hash[..2]).join(&blob_hash[2..]);
                    fs::create_dir_all(blob_path.parent().unwrap())?;
                    fs::write(&blob_path, &content)?;
                }
            }
        }
    }

    // Step 7: Write commits manifest
    let commits_manifest: Vec<serde_json::Value> = commits
        .iter()
        .map(|c| {
            serde_json::json!({
                "hash": c.hash,
                "tree": c.tree,
                "parents": c.parents,
                "author": c.author_name,
                "message": c.message,
                "timestamp_ms": c.timestamp_ms,
            })
        })
        .collect();

    let commits_path = target_dir.join(".aspen/commits.json");
    let mut file = fs::File::create(&commits_path)?;
    file.write_all(serde_json::to_string_pretty(&commits_manifest)?.as_bytes())?;

    // Step 8: Write HEAD ref
    let head_path = target_dir.join(".aspen/refs/heads").join(&branch_name);
    fs::write(&head_path, &head_hash)?;

    // Step 9: Write repo config
    let config = serde_json::json!({
        "repo_id": args.repo_id,
        "name": repo_info.name,
        "default_branch": repo_info.default_branch,
        "cloned_branch": branch_name,
        "head": head_hash,
    });
    let config_path = target_dir.join(".aspen/config.json");
    let mut file = fs::File::create(&config_path)?;
    file.write_all(serde_json::to_string_pretty(&config)?.as_bytes())?;

    if json {
        let output = serde_json::json!({
            "success": true,
            "path": target_dir.display().to_string(),
            "repo_id": args.repo_id,
            "branch": branch_name,
            "head": head_hash,
            "commits": commits.len(),
            "blobs": blob_hashes.len(),
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("Cloned '{}' to '{}'", repo_info.name, target_dir.display());
        println!("  Branch: {}", branch_name);
        println!("  HEAD:   {}", &head_hash[..16]);
        println!("  Commits: {}", commits.len());
        println!("  Blobs:   {}", blob_hashes.len());
    }

    Ok(())
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
    use aspen::forge::identity::RepoId;
    use aspen::forge::refs::SignedRefUpdate;

    // Parse the hash
    let hash_bytes = hex::decode(&args.hash)
        .map_err(|e| anyhow::anyhow!("invalid hash: {}", e))?;
    if hash_bytes.len() != 32 {
        anyhow::bail!("hash must be 32 bytes");
    }
    let mut hash_arr = [0u8; 32];
    hash_arr.copy_from_slice(&hash_bytes);
    let new_hash = blake3::Hash::from_bytes(hash_arr);

    // Parse repo_id
    let repo_id = RepoId::from_hex(&args.repo)
        .map_err(|e| anyhow::anyhow!("invalid repo_id: {}", e))?;

    // Load and sign if key provided
    let (signer, signature, timestamp_ms) = if let Some(key_path) = &args.key {
        let key_data = std::fs::read_to_string(key_path)
            .map_err(|e| anyhow::anyhow!("failed to read key file: {}", e))?;
        let key_bytes = hex::decode(key_data.trim())
            .map_err(|e| anyhow::anyhow!("failed to decode key (expected hex): {}", e))?;
        if key_bytes.len() != 32 {
            anyhow::bail!("secret key must be 32 bytes");
        }
        let mut key_arr = [0u8; 32];
        key_arr.copy_from_slice(&key_bytes);
        let secret_key = iroh::SecretKey::from_bytes(&key_arr);

        // Create signed update
        let update = SignedRefUpdate::sign(
            repo_id,
            &args.ref_name,
            new_hash,
            None, // old_hash would require fetching current ref first
            &secret_key,
        );

        (
            Some(update.signer.to_string()),
            Some(hex::encode(update.signature.to_bytes())),
            Some(update.timestamp_ms),
        )
    } else {
        (None, None, None)
    };

    let response = if args.force {
        client
            .send(ClientRpcRequest::ForgeSetRef {
                repo_id: args.repo,
                ref_name: args.ref_name.clone(),
                hash: args.hash,
                signer,
                signature,
                timestamp_ms,
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
                signer,
                signature,
                timestamp_ms,
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
