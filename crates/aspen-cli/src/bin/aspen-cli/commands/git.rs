//! Git repository commands.
//!
//! Commands for managing git repositories, commits, and refs
//! on the decentralized Forge.

use anyhow::Result;
use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::CommitOutput;
use crate::output::LogOutput;
use crate::output::RefOutput;
use crate::output::RepoListItem;
use crate::output::RepoListOutput;
use crate::output::RepoOutput;
use crate::output::print_output;

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

    /// Create a tree object from entries.
    ///
    /// Trees link file names to blob hashes. Use this to build
    /// the directory structure for a commit.
    CreateTree(CreateTreeArgs),

    /// Get a tree by hash.
    GetTree(GetTreeArgs),

    /// Export the delegate key for signing canonical refs.
    ///
    /// The delegate key is used to sign updates to canonical refs
    /// like heads/main and tags/*.
    ExportKey(ExportKeyArgs),

    /// Enable federation for a repository.
    ///
    /// Allows the repository to be discovered and synced by other clusters.
    Federate(FederateArgs),

    /// List federated repositories.
    ListFederated(ListFederatedArgs),

    /// Fetch a federated repository from a remote cluster.
    FetchRemote(FetchRemoteArgs),
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

#[derive(Args)]
pub struct CreateTreeArgs {
    /// Repository ID.
    #[arg(short, long)]
    pub repo: String,

    /// Tree entries in format "mode:name:hash" (e.g., "100644:README.md:abc123...").
    ///
    /// Mode values:
    /// - 100644: Regular file
    /// - 100755: Executable file
    /// - 040000: Subdirectory (tree)
    /// - 120000: Symbolic link
    #[arg(short, long = "entry", required = true)]
    pub entries: Vec<String>,
}

#[derive(Args)]
pub struct GetTreeArgs {
    /// Tree hash (hex-encoded BLAKE3).
    pub hash: String,
}

#[derive(Args)]
pub struct ExportKeyArgs {
    /// Repository ID to get the delegate key for.
    #[arg(short, long)]
    pub repo: String,

    /// Output file for the secret key (default: stdout).
    /// WARNING: This outputs the secret key in hex format.
    #[arg(short, long)]
    pub output: Option<std::path::PathBuf>,
}

#[derive(Args)]
pub struct FederateArgs {
    /// Repository ID (hex-encoded).
    #[arg(short, long)]
    pub repo: String,

    /// Federation mode: "public" or "allowlist".
    #[arg(long, default_value = "public")]
    pub mode: String,

    /// Allowed cluster keys (hex-encoded, for allowlist mode).
    #[arg(long)]
    pub allow: Vec<String>,
}

#[derive(Args)]
pub struct ListFederatedArgs {
    /// Maximum number of repositories to return.
    #[arg(short, long, default_value = "50")]
    pub limit: u32,
}

#[derive(Args)]
pub struct FetchRemoteArgs {
    /// Federated ID (format: origin_key:local_id).
    pub fed_id: String,

    /// Remote cluster public key (hex-encoded).
    #[arg(long)]
    pub cluster: String,
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
            GitCommand::CreateTree(args) => git_create_tree(client, args, json).await,
            GitCommand::GetTree(args) => git_get_tree(client, args, json).await,
            GitCommand::ExportKey(args) => git_export_key(client, args, json).await,
            GitCommand::Federate(args) => git_federate(client, args, json).await,
            GitCommand::ListFederated(args) => git_list_federated(client, args, json).await,
            GitCommand::FetchRemote(args) => git_fetch_remote(client, args, json).await,
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
    let target_dir = args.path.unwrap_or_else(|| std::path::PathBuf::from(&repo_info.name));

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
                result
                    .ref_info
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

    // Step 4: Get the HEAD commit's tree to extract working directory
    let head_tree_hash = if let Some(head_commit) = commits.first() {
        head_commit.tree.clone()
    } else {
        String::new()
    };

    // Step 5: Fetch trees and collect blob entries with their paths
    // We store (hash, mode, path) for the HEAD tree to extract to working dir
    struct FileEntry {
        hash: String,
        #[allow(dead_code)]
        name: String, // Keep for potential future use (e.g., symlink targets)
        mode: u32,
        path: std::path::PathBuf,
    }
    let mut file_entries: Vec<FileEntry> = Vec::new();
    let mut blob_hashes: HashSet<String> = HashSet::new();

    // Helper function to recursively fetch tree entries
    async fn fetch_tree_entries(
        client: &AspenClient,
        tree_hash: &str,
        base_path: &std::path::Path,
        file_entries: &mut Vec<FileEntry>,
        blob_hashes: &mut HashSet<String>,
    ) -> Result<()> {
        let tree_response = client
            .send(ClientRpcRequest::ForgeGetTree {
                hash: tree_hash.to_string(),
            })
            .await?;

        if let ClientRpcResponse::ForgeTreeResult(result) = tree_response {
            if result.success {
                if let Some(entries) = result.entries {
                    for entry in entries {
                        let entry_path = base_path.join(&entry.name);
                        // Mode >= 0o100000 = blob (file), < 0o100000 = tree (directory)
                        // 0o100644 = regular file, 0o100755 = executable, 0o040000 = tree
                        if entry.mode >= 0o100000 {
                            blob_hashes.insert(entry.hash.clone());
                            file_entries.push(FileEntry {
                                hash: entry.hash,
                                name: entry.name,
                                mode: entry.mode,
                                path: entry_path,
                            });
                        } else if entry.mode == 0o040000 {
                            // Recursively fetch subtree
                            Box::pin(fetch_tree_entries(client, &entry.hash, &entry_path, file_entries, blob_hashes))
                                .await?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    // Fetch the HEAD tree entries for working directory extraction
    if !head_tree_hash.is_empty() {
        fetch_tree_entries(client, &head_tree_hash, std::path::Path::new(""), &mut file_entries, &mut blob_hashes)
            .await?;
    }

    // Step 7: Fetch blobs and write to both objects directory and working directory
    let objects_dir = target_dir.join(".aspen/objects");
    let mut blob_contents: std::collections::HashMap<String, Vec<u8>> = std::collections::HashMap::new();

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

                    // Store content for working directory extraction
                    blob_contents.insert(blob_hash.clone(), content);
                }
            }
        }
    }

    // Step 8: Extract files to working directory
    let mut files_extracted = 0;
    for entry in &file_entries {
        if let Some(content) = blob_contents.get(&entry.hash) {
            let file_path = target_dir.join(&entry.path);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&file_path, content)?;

            // Set executable permission if mode indicates executable
            #[cfg(unix)]
            if entry.mode == 0o100755 {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&file_path)?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&file_path, perms)?;
            }

            files_extracted += 1;
        }
    }
    let _ = files_extracted; // Silence unused warning on non-unix

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
            "files": file_entries.len(),
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("Cloned '{}' to '{}'", repo_info.name, target_dir.display());
        println!("  Branch:  {}", branch_name);
        println!("  HEAD:    {}", &head_hash[..16]);
        println!("  Commits: {}", commits.len());
        println!("  Files:   {}", file_entries.len());
    }

    Ok(())
}

async fn git_show(client: &AspenClient, args: ShowArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::ForgeGetRepo { repo_id: args.repo }).await?;

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

/// Fetch the current hash of a ref, returning None if the ref doesn't exist.
async fn fetch_current_ref_hash(client: &AspenClient, repo_id: &str, ref_name: &str) -> Result<Option<String>> {
    let response = client
        .send(ClientRpcRequest::ForgeGetRef {
            repo_id: repo_id.to_string(),
            ref_name: ref_name.to_string(),
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeRefResult(result) => {
            if result.success {
                Ok(result.ref_info.map(|info| info.hash))
            } else {
                // Ref doesn't exist - this is OK for new refs
                Ok(None)
            }
        }
        ClientRpcResponse::ForgeOperationResult(_) => {
            // ForgeOperationResult doesn't contain ref info, treat as not found
            Ok(None)
        }
        ClientRpcResponse::Error(e) => {
            // Treat "not found" errors as None, propagate others
            if e.message.contains("not found") || e.code == "NOT_FOUND" {
                Ok(None)
            } else {
                anyhow::bail!("{}: {}", e.code, e.message)
            }
        }
        _ => Ok(None),
    }
}

async fn git_push(client: &AspenClient, args: PushArgs, json: bool) -> Result<()> {
    use aspen_forge::identity::RepoId;
    use aspen_forge::refs::SignedRefUpdate;

    // Parse the hash
    let hash_bytes = hex::decode(&args.hash).map_err(|e| anyhow::anyhow!("invalid hash: {}", e))?;
    if hash_bytes.len() != 32 {
        anyhow::bail!("hash must be 32 bytes");
    }
    let mut hash_arr = [0u8; 32];
    hash_arr.copy_from_slice(&hash_bytes);
    let new_hash = blake3::Hash::from_bytes(hash_arr);

    // Parse repo_id
    let repo_id = RepoId::from_hex(&args.repo).map_err(|e| anyhow::anyhow!("invalid repo_id: {}", e))?;

    // For non-force pushes, fetch current ref for CAS comparison
    let current_hash = if !args.force {
        fetch_current_ref_hash(client, &args.repo, &args.ref_name).await?
    } else {
        None
    };

    // Parse current hash for signing if available
    let old_hash = if let Some(ref hash_str) = current_hash {
        let old_bytes = hex::decode(hash_str).ok();
        old_bytes.and_then(|b| {
            if b.len() == 32 {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&b);
                Some(blake3::Hash::from_bytes(arr))
            } else {
                None
            }
        })
    } else {
        None
    };

    // Load and sign if key provided
    let (signer, signature, timestamp_ms) = if let Some(key_path) = &args.key {
        let key_data =
            std::fs::read_to_string(key_path).map_err(|e| anyhow::anyhow!("failed to read key file: {}", e))?;
        let key_bytes =
            hex::decode(key_data.trim()).map_err(|e| anyhow::anyhow!("failed to decode key (expected hex): {}", e))?;
        if key_bytes.len() != 32 {
            anyhow::bail!("secret key must be 32 bytes");
        }
        let mut key_arr = [0u8; 32];
        key_arr.copy_from_slice(&key_bytes);
        let secret_key = iroh::SecretKey::from_bytes(&key_arr);

        // Create signed update with old_hash for proper CAS signing
        let update = SignedRefUpdate::sign(repo_id, &args.ref_name, new_hash, old_hash, &secret_key);

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
        // Use CAS for safe push with current ref value as expected
        client
            .send(ClientRpcRequest::ForgeCasRef {
                repo_id: args.repo,
                ref_name: args.ref_name.clone(),
                expected: current_hash,
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
    let hash = args.hash.clone();
    let response = client.send(ClientRpcRequest::ForgeGetBlob { hash: args.hash }).await?;

    match response {
        ClientRpcResponse::ForgeBlobResult(result) => {
            if result.success {
                if let Some(content) = result.content {
                    if let Some(ref path) = args.output {
                        std::fs::write(path, &content).map_err(|e| anyhow::anyhow!("failed to write file: {}", e))?;
                        if !json {
                            println!("Wrote {} bytes to {}", content.len(), path.display());
                        } else {
                            println!(
                                "{}",
                                serde_json::json!({
                                    "success": true,
                                    "hash": hash,
                                    "size": content.len(),
                                    "path": path.display().to_string()
                                })
                            );
                        }
                    } else if json {
                        println!(
                            "{}",
                            serde_json::json!({
                                "success": true,
                                "hash": hash,
                                "size": content.len(),
                                "content_base64": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &content)
                            })
                        );
                    } else {
                        // Display content nicely like blob get does
                        match String::from_utf8(content.clone()) {
                            Ok(s) => print!("{}", s),
                            Err(_) => println!("<binary: {} bytes>", content.len()),
                        }
                    }
                    Ok(())
                } else {
                    if json {
                        println!(
                            "{}",
                            serde_json::json!({
                                "success": false,
                                "hash": hash,
                                "error": "blob not found"
                            })
                        );
                    } else {
                        eprintln!("Blob not found: {}", hash);
                    }
                    std::process::exit(1);
                }
            } else {
                let error = result.error.unwrap_or_else(|| "unknown error".to_string());
                if json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "success": false,
                            "hash": hash,
                            "error": error
                        })
                    );
                }
                anyhow::bail!("{}", error)
            }
        }
        ClientRpcResponse::ForgeOperationResult(result) => {
            anyhow::bail!("{}", result.error.unwrap_or_else(|| "not found".to_string()))
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

// ============================================================================
// Tree Commands
// ============================================================================

async fn git_create_tree(client: &AspenClient, args: CreateTreeArgs, json: bool) -> Result<()> {
    // Parse entries from "mode:name:hash" format
    let mut entries = Vec::new();
    for entry_str in &args.entries {
        let parts: Vec<&str> = entry_str.splitn(3, ':').collect();
        if parts.len() != 3 {
            anyhow::bail!(
                "invalid entry format: '{}'. Expected 'mode:name:hash' (e.g., '100644:README.md:abc123...')",
                entry_str
            );
        }

        let mode: u32 = parts[0].parse().map_err(|_| {
            anyhow::anyhow!("invalid mode '{}'. Expected octal number like 100644, 100755, or 040000", parts[0])
        })?;
        let name = parts[1].to_string();
        let hash = parts[2].to_string();

        // Validate hash is 64 hex chars
        if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            anyhow::bail!("invalid hash '{}': must be 64 hex characters", hash);
        }

        entries.push(serde_json::json!({
            "mode": mode,
            "name": name,
            "hash": hash
        }));
    }

    let entries_json = serde_json::to_string(&entries)?;

    let response = client
        .send(ClientRpcRequest::ForgeCreateTree {
            repo_id: args.repo,
            entries_json,
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeTreeResult(result) => {
            if result.success {
                if json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "success": true,
                            "hash": result.hash,
                            "entries": result.entries.as_ref().map(|e| e.len()).unwrap_or(0)
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

async fn git_get_tree(client: &AspenClient, args: GetTreeArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::ForgeGetTree {
            hash: args.hash.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeTreeResult(result) => {
            if result.success {
                if json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "success": true,
                            "hash": args.hash,
                            "entries": result.entries
                        })
                    );
                } else if let Some(entries) = result.entries {
                    if entries.is_empty() {
                        println!("(empty tree)");
                    } else {
                        println!("{:<8} {:<40} {}", "MODE", "NAME", "HASH");
                        println!("{}", "-".repeat(80));
                        for entry in entries {
                            println!("{:0>6o}   {:<40} {}", entry.mode, entry.name, entry.hash);
                        }
                    }
                } else {
                    println!("Tree not found: {}", args.hash);
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

// ============================================================================
// Key Management Commands
// ============================================================================

async fn git_export_key(client: &AspenClient, args: ExportKeyArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::ForgeGetDelegateKey {
            repo_id: args.repo.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeKeyResult(result) => {
            if result.success {
                if let Some(ref secret_key) = result.secret_key {
                    if let Some(ref path) = args.output {
                        std::fs::write(path, format!("{}\n", secret_key))
                            .map_err(|e| anyhow::anyhow!("failed to write key file: {}", e))?;
                        if !json {
                            println!("Delegate key written to {}", path.display());
                            println!("Public key: {}", result.public_key.as_deref().unwrap_or("unknown"));
                        } else {
                            println!(
                                "{}",
                                serde_json::json!({
                                    "success": true,
                                    "public_key": result.public_key,
                                    "path": path.display().to_string()
                                })
                            );
                        }
                    } else if json {
                        println!(
                            "{}",
                            serde_json::json!({
                                "success": true,
                                "public_key": result.public_key,
                                "secret_key": secret_key
                            })
                        );
                    } else {
                        // Print key to stdout
                        println!("# Delegate key for repository {}", args.repo);
                        println!("# Public key: {}", result.public_key.as_deref().unwrap_or("unknown"));
                        println!("# WARNING: Keep this secret key secure!");
                        println!("{}", secret_key);
                    }
                    Ok(())
                } else {
                    anyhow::bail!("delegate key not available for this repository")
                }
            } else {
                anyhow::bail!("{}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

// ============================================================================
// Federation Commands
// ============================================================================

async fn git_federate(client: &AspenClient, args: FederateArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::FederateRepository {
            repo_id: args.repo,
            mode: args.mode,
        })
        .await?;

    match response {
        ClientRpcResponse::FederateRepositoryResult(result) => {
            if result.success {
                if json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "federated_id": result.fed_id,
                        })
                    );
                } else {
                    println!("Repository federated successfully");
                    if let Some(fed_id) = result.fed_id {
                        println!("Federated ID: {}", fed_id);
                    }
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

async fn git_list_federated(client: &AspenClient, _args: ListFederatedArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::ListFederatedRepositories).await?;

    match response {
        ClientRpcResponse::FederatedRepositories(result) => {
            if result.error.is_none() {
                if json {
                    println!("{}", serde_json::to_string_pretty(&result.repositories)?);
                } else {
                    if result.repositories.is_empty() {
                        println!("No federated repositories");
                    } else {
                        println!("{:<40} {:<10} {:<40}", "REPO ID", "MODE", "FEDERATED ID");
                        println!("{}", "-".repeat(90));
                        for repo in &result.repositories {
                            println!("{:<40} {:<10} {:<40}", repo.repo_id, repo.mode, repo.fed_id);
                        }
                        println!("\nTotal: {}", result.count);
                    }
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

async fn git_fetch_remote(client: &AspenClient, args: FetchRemoteArgs, json: bool) -> Result<()> {
    let response = client
        .send(ClientRpcRequest::ForgeFetchFederated {
            federated_id: args.fed_id,
            remote_cluster: args.cluster,
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeFetchResult(result) => {
            if result.success {
                if json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "remote_cluster": result.remote_cluster,
                            "fetched": result.fetched,
                            "already_present": result.already_present,
                            "errors": result.errors,
                        })
                    );
                } else {
                    println!("Fetch complete from {}", result.remote_cluster.unwrap_or_else(|| "unknown".to_string()));
                    println!("  Fetched: {}", result.fetched);
                    println!("  Already present: {}", result.already_present);
                    if !result.errors.is_empty() {
                        println!("  Errors: {}", result.errors.len());
                        for err in &result.errors {
                            println!("    - {}", err);
                        }
                    }
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
