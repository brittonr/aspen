//! Git repository commands.
//!
//! Commands for managing git repositories, commits, and refs
//! on the decentralized Forge.

use anyhow::Context;
use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
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
    #[arg(short = 'f', long = "force")]
    pub is_force: bool,

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
            if result.is_success {
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
            if result.is_success {
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
            if result.is_success {
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

    let repo_info = git_clone_fetch_repo_info(client, &args.repo_id).await?;
    let target_dir = args.path.unwrap_or_else(|| std::path::PathBuf::from(&repo_info.name));

    if target_dir.exists() {
        anyhow::bail!("Directory '{}' already exists", target_dir.display());
    }

    git_clone_setup_directories(&target_dir)?;

    let branch_name = args.branch.unwrap_or_else(|| repo_info.default_branch.clone());
    let ref_name = format!("heads/{}", branch_name);

    let head_hash = match git_clone_resolve_head_ref(client, &args.repo_id, &ref_name, &branch_name).await? {
        Some(hash) => hash,
        None => {
            git_clone_write_empty_repo_config(&target_dir, &args.repo_id, &repo_info, json)?;
            return Ok(());
        }
    };

    let commits = git_clone_fetch_commits(client, &args.repo_id, &ref_name).await?;
    let head_tree_hash = commits.first().map(|c| c.tree.clone()).unwrap_or_default();

    let mut file_entries: Vec<CloneFileEntry> = Vec::new();
    let mut blob_hashes: HashSet<String> = HashSet::new();
    if !head_tree_hash.is_empty() {
        git_clone_fetch_tree_entries(
            client,
            &head_tree_hash,
            std::path::Path::new(""),
            &mut file_entries,
            &mut blob_hashes,
        )
        .await?;
    }

    let blob_contents = git_clone_fetch_blobs(client, &blob_hashes, &target_dir).await?;
    git_clone_extract_files(&target_dir, &file_entries, &blob_contents)?;
    git_clone_write_manifests(&target_dir, &args.repo_id, &repo_info, &branch_name, &head_hash, &commits)?;
    git_clone_print_result(
        json,
        &target_dir,
        &args.repo_id,
        &repo_info.name,
        &branch_name,
        &head_hash,
        &commits,
        &blob_hashes,
        &file_entries,
    );

    Ok(())
}

/// A file entry discovered during tree traversal for clone.
struct CloneFileEntry {
    hash: String,
    #[allow(dead_code)]
    name: String,
    mode: u32,
    path: std::path::PathBuf,
}

/// Fetch repository info from the server.
async fn git_clone_fetch_repo_info(client: &AspenClient, repo_id: &str) -> Result<aspen_client_api::ForgeRepoInfo> {
    let response = client
        .send(ClientRpcRequest::ForgeGetRepo {
            repo_id: repo_id.to_string(),
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeRepoResult(result) => {
            if result.is_success {
                result.repo.ok_or_else(|| anyhow::anyhow!("Repository not found"))
            } else {
                anyhow::bail!("{}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

/// Create the .aspen directory structure for a clone.
fn git_clone_setup_directories(target_dir: &std::path::Path) -> Result<()> {
    use std::fs;
    fs::create_dir_all(target_dir)?;
    fs::create_dir_all(target_dir.join(".aspen"))?;
    fs::create_dir_all(target_dir.join(".aspen/objects"))?;
    fs::create_dir_all(target_dir.join(".aspen/refs/heads"))?;
    Ok(())
}

/// Resolve the HEAD ref for the branch. Returns None if the branch does not exist (empty repo).
async fn git_clone_resolve_head_ref(
    client: &AspenClient,
    repo_id: &str,
    ref_name: &str,
    branch_name: &str,
) -> Result<Option<String>> {
    let response = client
        .send(ClientRpcRequest::ForgeGetRef {
            repo_id: repo_id.to_string(),
            ref_name: ref_name.to_string(),
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeRefResult(result) => {
            if result.is_success {
                let hash = result
                    .ref_info
                    .map(|r| r.hash)
                    .ok_or_else(|| anyhow::anyhow!("Branch '{}' not found", branch_name))?;
                Ok(Some(hash))
            } else {
                Ok(None)
            }
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

/// Write config for an empty repository (no commits yet).
fn git_clone_write_empty_repo_config(
    target_dir: &std::path::Path,
    repo_id: &str,
    repo_info: &aspen_client_api::ForgeRepoInfo,
    json: bool,
) -> Result<()> {
    use std::fs;
    use std::io::Write;

    if !json {
        println!("Created empty repository at '{}'", target_dir.display());
    }
    let config = serde_json::json!({
        "repo_id": repo_id,
        "name": repo_info.name,
        "default_branch": repo_info.default_branch,
    });
    let config_path = target_dir.join(".aspen/config.json");
    let mut file =
        fs::File::create(&config_path).with_context(|| format!("failed to create {}", config_path.display()))?;
    file.write_all(serde_json::to_string_pretty(&config)?.as_bytes())
        .with_context(|| format!("failed to write {}", config_path.display()))?;
    Ok(())
}

/// Fetch commit history from the server.
async fn git_clone_fetch_commits(
    client: &AspenClient,
    repo_id: &str,
    ref_name: &str,
) -> Result<Vec<aspen_client_api::ForgeCommitInfo>> {
    let response = client
        .send(ClientRpcRequest::ForgeLog {
            repo_id: repo_id.to_string(),
            ref_name: Some(ref_name.to_string()),
            limit: Some(1000),
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeLogResult(result) => {
            if result.is_success {
                Ok(result.commits)
            } else {
                anyhow::bail!("{}", result.error.unwrap_or_else(|| "failed to get log".to_string()))
            }
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

/// Recursively fetch tree entries, collecting file entries and blob hashes.
async fn git_clone_fetch_tree_entries(
    client: &AspenClient,
    tree_hash: &str,
    base_path: &std::path::Path,
    file_entries: &mut Vec<CloneFileEntry>,
    blob_hashes: &mut std::collections::HashSet<String>,
) -> Result<()> {
    let tree_response = client
        .send(ClientRpcRequest::ForgeGetTree {
            hash: tree_hash.to_string(),
        })
        .await?;

    if let ClientRpcResponse::ForgeTreeResult(result) = tree_response {
        if result.is_success {
            if let Some(entries) = result.entries {
                for entry in entries {
                    let entry_path = base_path.join(&entry.name);
                    if entry.mode >= 0o100000 {
                        blob_hashes.insert(entry.hash.clone());
                        file_entries.push(CloneFileEntry {
                            hash: entry.hash,
                            name: entry.name,
                            mode: entry.mode,
                            path: entry_path,
                        });
                    } else if entry.mode == 0o040000 {
                        Box::pin(git_clone_fetch_tree_entries(
                            client,
                            &entry.hash,
                            &entry_path,
                            file_entries,
                            blob_hashes,
                        ))
                        .await?;
                    }
                }
            }
        }
    }
    Ok(())
}

/// Fetch all blobs, writing them to the objects directory.
async fn git_clone_fetch_blobs(
    client: &AspenClient,
    blob_hashes: &std::collections::HashSet<String>,
    target_dir: &std::path::Path,
) -> Result<std::collections::HashMap<String, Vec<u8>>> {
    use std::fs;

    let objects_dir = target_dir.join(".aspen/objects");
    let mut blob_contents: std::collections::HashMap<String, Vec<u8>> = std::collections::HashMap::new();

    for blob_hash in blob_hashes {
        let blob_response = client
            .send(ClientRpcRequest::ForgeGetBlob {
                hash: blob_hash.clone(),
            })
            .await?;

        if let ClientRpcResponse::ForgeBlobResult(result) = blob_response {
            if result.is_success {
                if let Some(content) = result.content {
                    let blob_path = objects_dir.join(&blob_hash[..2]).join(&blob_hash[2..]);
                    if let Some(parent) = blob_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(&blob_path, &content)?;
                    blob_contents.insert(blob_hash.clone(), content);
                }
            }
        }
    }
    Ok(blob_contents)
}

/// Extract fetched blobs to the working directory.
fn git_clone_extract_files(
    target_dir: &std::path::Path,
    file_entries: &[CloneFileEntry],
    blob_contents: &std::collections::HashMap<String, Vec<u8>>,
) -> Result<()> {
    use std::fs;

    for entry in file_entries {
        if let Some(content) = blob_contents.get(&entry.hash) {
            let file_path = target_dir.join(&entry.path);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&file_path, content)?;

            #[cfg(unix)]
            if entry.mode == 0o100755 {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&file_path)?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&file_path, perms)?;
            }
        }
    }
    Ok(())
}

/// Write commits manifest, HEAD ref, and repo config to the .aspen directory.
fn git_clone_write_manifests(
    target_dir: &std::path::Path,
    repo_id: &str,
    repo_info: &aspen_client_api::ForgeRepoInfo,
    branch_name: &str,
    head_hash: &str,
    commits: &[aspen_client_api::ForgeCommitInfo],
) -> Result<()> {
    use std::fs;
    use std::io::Write;

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
    let mut file =
        fs::File::create(&commits_path).with_context(|| format!("failed to create {}", commits_path.display()))?;
    file.write_all(serde_json::to_string_pretty(&commits_manifest)?.as_bytes())
        .with_context(|| format!("failed to write {}", commits_path.display()))?;

    let head_path = target_dir.join(".aspen/refs/heads").join(branch_name);
    fs::write(&head_path, head_hash).with_context(|| format!("failed to write HEAD ref at {}", head_path.display()))?;

    let config = serde_json::json!({
        "repo_id": repo_id,
        "name": repo_info.name,
        "default_branch": repo_info.default_branch,
        "cloned_branch": branch_name,
        "head": head_hash,
    });
    let config_path = target_dir.join(".aspen/config.json");
    let mut file =
        fs::File::create(&config_path).with_context(|| format!("failed to create {}", config_path.display()))?;
    file.write_all(serde_json::to_string_pretty(&config)?.as_bytes())
        .with_context(|| format!("failed to write {}", config_path.display()))?;

    Ok(())
}

/// Print the clone result summary.
fn git_clone_print_result(
    json: bool,
    target_dir: &std::path::Path,
    repo_id: &str,
    repo_name: &str,
    branch_name: &str,
    head_hash: &str,
    commits: &[aspen_client_api::ForgeCommitInfo],
    blob_hashes: &std::collections::HashSet<String>,
    file_entries: &[CloneFileEntry],
) {
    if json {
        let output = serde_json::json!({
            "success": true,
            "path": target_dir.display().to_string(),
            "repo_id": repo_id,
            "branch": branch_name,
            "head": head_hash,
            "commits": commits.len(),
            "blobs": blob_hashes.len(),
            "files": file_entries.len(),
        });
        // JSON serialization of a json! value will not fail
        if let Ok(s) = serde_json::to_string_pretty(&output) {
            println!("{}", s);
        }
    } else {
        println!("Cloned '{}' to '{}'", repo_name, target_dir.display());
        println!("  Branch:  {}", branch_name);
        println!("  HEAD:    {}", &head_hash[..16]);
        println!("  Commits: {}", commits.len());
        println!("  Files:   {}", file_entries.len());
    }
}

async fn git_show(client: &AspenClient, args: ShowArgs, json: bool) -> Result<()> {
    let response = client.send(ClientRpcRequest::ForgeGetRepo { repo_id: args.repo }).await?;

    match response {
        ClientRpcResponse::ForgeRepoResult(result) => {
            if result.is_success {
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
            if result.is_success {
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
            if result.is_success {
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
            if result.is_success {
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

    let new_hash = git_push_parse_hash(&args.hash)?;
    let repo_id = RepoId::from_hex(&args.repo).map_err(|e| anyhow::anyhow!("invalid repo_id: {}", e))?;

    let current_hash = if !args.is_force {
        fetch_current_ref_hash(client, &args.repo, &args.ref_name).await?
    } else {
        None
    };

    let old_hash = git_push_parse_old_hash(&current_hash);
    let (signer, signature, timestamp_ms) =
        git_push_sign_update(&args.key, repo_id, &args.ref_name, new_hash, old_hash)?;

    let response = git_push_send_request(client, &args, current_hash, signer, signature, timestamp_ms).await?;

    git_push_handle_response(response, &args.ref_name, json)
}

/// Parse and validate a hex-encoded 32-byte blake3 hash.
fn git_push_parse_hash(hash_hex: &str) -> Result<blake3::Hash> {
    let hash_bytes = hex::decode(hash_hex).map_err(|e| anyhow::anyhow!("invalid hash: {}", e))?;
    if hash_bytes.len() != 32 {
        anyhow::bail!("hash must be 32 bytes");
    }
    let mut hash_arr = [0u8; 32];
    hash_arr.copy_from_slice(&hash_bytes);
    Ok(blake3::Hash::from_bytes(hash_arr))
}

/// Parse the current ref hash string into a blake3::Hash for signing.
fn git_push_parse_old_hash(current_hash: &Option<String>) -> Option<blake3::Hash> {
    let hash_str = current_hash.as_ref()?;
    let old_bytes = hex::decode(hash_str).ok()?;
    if old_bytes.len() == 32 {
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&old_bytes);
        Some(blake3::Hash::from_bytes(arr))
    } else {
        None
    }
}

/// Load a signing key and produce a signed ref update, if a key path is provided.
fn git_push_sign_update(
    key_path: &Option<std::path::PathBuf>,
    repo_id: aspen_forge::identity::RepoId,
    ref_name: &str,
    new_hash: blake3::Hash,
    old_hash: Option<blake3::Hash>,
) -> Result<(Option<String>, Option<String>, Option<u64>)> {
    use aspen_forge::refs::SignedRefUpdate;

    let Some(key_path) = key_path else {
        return Ok((None, None, None));
    };

    let key_data = std::fs::read_to_string(key_path).map_err(|e| anyhow::anyhow!("failed to read key file: {}", e))?;
    let key_bytes =
        hex::decode(key_data.trim()).map_err(|e| anyhow::anyhow!("failed to decode key (expected hex): {}", e))?;
    if key_bytes.len() != 32 {
        anyhow::bail!("secret key must be 32 bytes");
    }
    let mut key_arr = [0u8; 32];
    key_arr.copy_from_slice(&key_bytes);
    let secret_key = iroh::SecretKey::from_bytes(&key_arr);

    let update = SignedRefUpdate::sign(repo_id, ref_name, new_hash, old_hash, &secret_key);

    Ok((
        Some(update.signer.to_string()),
        Some(hex::encode(update.signature.to_bytes())),
        Some(update.timestamp_ms),
    ))
}

/// Send the push request (force set or CAS) to the server.
async fn git_push_send_request(
    client: &AspenClient,
    args: &PushArgs,
    current_hash: Option<String>,
    signer: Option<String>,
    signature: Option<String>,
    timestamp_ms: Option<u64>,
) -> Result<ClientRpcResponse> {
    if args.is_force {
        client
            .send(ClientRpcRequest::ForgeSetRef {
                repo_id: args.repo.clone(),
                ref_name: args.ref_name.clone(),
                hash: args.hash.clone(),
                signer,
                signature,
                timestamp_ms,
            })
            .await
            .map_err(Into::into)
    } else {
        client
            .send(ClientRpcRequest::ForgeCasRef {
                repo_id: args.repo.clone(),
                ref_name: args.ref_name.clone(),
                expected: current_hash,
                new_hash: args.hash.clone(),
                signer,
                signature,
                timestamp_ms,
            })
            .await
            .map_err(Into::into)
    }
}

/// Handle the push response, printing the result.
fn git_push_handle_response(response: ClientRpcResponse, ref_name: &str, json: bool) -> Result<()> {
    match response {
        ClientRpcResponse::ForgeRefResult(result) => {
            if result.is_success {
                if let Some(ref_info) = result.ref_info {
                    let output = RefOutput {
                        name: ref_info.name,
                        hash: ref_info.hash,
                    };
                    print_output(&output, json);
                } else if !json {
                    println!("Ref {} updated", ref_name);
                }
                Ok(())
            } else {
                anyhow::bail!("{}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
        }
        ClientRpcResponse::ForgeOperationResult(result) => {
            if result.is_success {
                if !json {
                    println!("Ref {} updated", ref_name);
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
            if result.is_success {
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
            if result.is_success {
                if json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "hash": result.hash,
                            "size_bytes": result.size
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
            if result.is_success {
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

        let mode: u32 = u32::from_str_radix(parts[0], 8).map_err(|_| {
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
            if result.is_success {
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
            if result.is_success {
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
            if result.is_success {
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
            if result.is_success {
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
            if result.is_success {
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
