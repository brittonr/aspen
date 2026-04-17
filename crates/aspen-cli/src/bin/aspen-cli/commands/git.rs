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

    /// Show commit details by hash.
    ShowCommit(ShowCommitArgs),

    /// Show commit history.
    Log(LogArgs),

    /// Push refs to the remote.
    #[cfg(feature = "forge")]
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

    /// Fork a repository.
    Fork(ForkArgs),

    /// Manage mirror configuration for a repository.
    Mirror(MirrorArgs),

    /// Show diff between two refs or commits.
    Diff(DiffArgs),
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
    #[arg(short, long = "limit", default_value = "50")]
    pub max_repos: u32,

    /// Offset for pagination.
    #[arg(long = "offset", default_value = "0")]
    pub page_start: u32,
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
pub struct ShowCommitArgs {
    /// Commit hash (hex-encoded BLAKE3).
    pub hash: String,
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
    #[arg(short = 'n', long = "limit", default_value = "10")]
    pub max_commits: u32,
}

#[cfg(feature = "forge")]
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
    #[arg(short, long = "limit", default_value = "50")]
    pub max_repos: u32,
}

#[derive(Args)]
pub struct FetchRemoteArgs {
    /// Federated ID (format: origin_key:local_id).
    pub fed_id: String,

    /// Remote cluster public key (hex-encoded).
    #[arg(long)]
    pub cluster: String,
}

#[derive(Args, Clone)]
pub struct ForkArgs {
    /// Upstream repository ID to fork from.
    #[arg(long)]
    pub upstream: String,

    /// Name for the fork.
    #[arg(short, long)]
    pub name: String,
}

#[derive(Args, Clone)]
pub struct MirrorArgs {
    /// Repository ID.
    #[arg(short, long)]
    pub repo: String,

    /// Upstream repository ID to mirror from.
    #[arg(long)]
    pub upstream: Option<String>,

    /// Sync interval in seconds.
    #[arg(long = "interval", default_value = "300")]
    pub interval_secs: u32,

    /// Disable mirror mode.
    #[arg(long)]
    pub disable: bool,

    /// Show mirror status.
    #[arg(long)]
    pub status: bool,
}

impl GitCommand {
    /// Execute the git command.
    pub async fn run(self, client: &AspenClient, is_json_output: bool) -> Result<()> {
        match self {
            GitCommand::Init(args) => git_init(client, args, is_json_output).await,
            GitCommand::List(args) => git_list(client, args, is_json_output).await,
            GitCommand::Clone(args) => git_clone(client, args, is_json_output).await,
            GitCommand::Show(args) => git_show(client, args, is_json_output).await,
            GitCommand::Commit(args) => git_commit(client, args, is_json_output).await,
            GitCommand::ShowCommit(args) => git_show_commit(client, args, is_json_output).await,
            GitCommand::Log(args) => git_log(client, args, is_json_output).await,
            #[cfg(feature = "forge")]
            GitCommand::Push(args) => git_push(client, args, is_json_output).await,
            GitCommand::GetRef(args) => git_get_ref(client, args, is_json_output).await,
            GitCommand::StoreBlob(args) => git_store_blob(client, args, is_json_output).await,
            GitCommand::GetBlob(args) => git_get_blob(client, args, is_json_output).await,
            GitCommand::CreateTree(args) => git_create_tree(client, args, is_json_output).await,
            GitCommand::GetTree(args) => git_get_tree(client, args, is_json_output).await,
            GitCommand::ExportKey(args) => git_export_key(client, args, is_json_output).await,
            GitCommand::Federate(args) => git_federate(client, args, is_json_output).await,
            GitCommand::ListFederated(args) => git_list_federated(client, args, is_json_output).await,
            GitCommand::FetchRemote(args) => git_fetch_remote(client, args, is_json_output).await,
            GitCommand::Fork(args) => git_fork(client, args, is_json_output).await,
            GitCommand::Mirror(args) => git_mirror(client, args, is_json_output).await,
            GitCommand::Diff(args) => git_diff(client, args, is_json_output).await,
        }
    }
}

async fn git_init(client: &AspenClient, args: InitArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.name.is_empty(), "git init requires a repository name");
    debug_assert!(!args.default_branch.is_empty(), "git init requires a default branch");

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
                        threshold: repo.threshold_delegates,
                        created_at_ms: repo.created_at_ms,
                    };
                    print_output(&output, is_json_output);
                }
                Ok(())
            } else {
                anyhow::bail!("{}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
        }
        ClientRpcResponse::ForgeOperationResult(result) => {
            if result.is_success {
                if !is_json_output {
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

async fn git_list(client: &AspenClient, args: ListArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(args.max_repos > 0, "git list should request at least one repository");
    debug_assert!(
        args.page_start <= u32::MAX.saturating_sub(args.max_repos),
        "git list pagination must stay representable"
    );

    let response = client
        .send(ClientRpcRequest::ForgeListRepos {
            limit: Some(args.max_repos),
            offset: Some(args.page_start),
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
                print_output(&output, is_json_output);
                Ok(())
            } else {
                anyhow::bail!("{}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn git_clone(client: &AspenClient, args: CloneArgs, is_json_output: bool) -> Result<()> {
    use std::collections::HashSet;

    let repo_info = git_clone_fetch_repo_info(client, &args.repo_id).await?;
    let target_dir = args.path.unwrap_or_else(|| std::path::PathBuf::from(&repo_info.name));

    if target_dir.exists() {
        anyhow::bail!("Directory '{}' already exists", target_dir.display());
    }

    git_clone_setup_directories(&target_dir)?;

    let branch_name = args.branch.unwrap_or_else(|| repo_info.default_branch.clone());
    let ref_name = format!("heads/{}", branch_name);
    debug_assert!(!branch_name.is_empty(), "git clone must resolve a non-empty branch name");
    debug_assert!(ref_name.starts_with("heads/"), "git clone HEAD ref must use heads/* namespace");
    let ref_request = CloneRefRequest {
        repo_id: &args.repo_id,
        ref_name: &ref_name,
        branch_name: &branch_name,
    };

    let head_hash = match git_clone_resolve_head_ref(client, ref_request).await? {
        Some(hash) => hash,
        None => {
            git_clone_write_empty_repo_config(&target_dir, &args.repo_id, &repo_info, is_json_output)?;
            return Ok(());
        }
    };

    let commits = git_clone_fetch_commits(client, CloneLogRequest {
        repo_id: &args.repo_id,
        ref_name: &ref_name,
    })
    .await?;
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

    let clone_summary = CloneSummary {
        repo_id: &args.repo_id,
        repo_name: &repo_info.name,
        branch_name: &branch_name,
        head_hash: &head_hash,
    };

    let blob_contents = git_clone_fetch_blobs(client, &blob_hashes, &target_dir).await?;
    git_clone_extract_files(&target_dir, &file_entries, &blob_contents)?;
    git_clone_write_manifests(&target_dir, &repo_info, clone_summary, &commits)?;
    git_clone_print_result(is_json_output, &target_dir, clone_summary, CloneArtifacts {
        commits: &commits,
        blob_hashes: &blob_hashes,
        file_entries: &file_entries,
    });

    Ok(())
}

/// A file entry discovered during tree traversal for clone.
struct CloneFileEntry {
    hash: String,
    mode: u32,
    path: std::path::PathBuf,
}

#[derive(Clone, Copy)]
struct CloneRefRequest<'a> {
    repo_id: &'a str,
    ref_name: &'a str,
    branch_name: &'a str,
}

#[derive(Clone, Copy)]
struct CloneLogRequest<'a> {
    repo_id: &'a str,
    ref_name: &'a str,
}

#[derive(Clone, Copy)]
struct CloneSummary<'a> {
    repo_id: &'a str,
    repo_name: &'a str,
    branch_name: &'a str,
    head_hash: &'a str,
}

#[derive(Clone, Copy)]
struct CloneArtifacts<'a> {
    commits: &'a [aspen_client_api::ForgeCommitInfo],
    blob_hashes: &'a std::collections::HashSet<String>,
    file_entries: &'a [CloneFileEntry],
}

#[derive(Clone, Copy)]
struct RefLookupRequest<'a> {
    repo_id: &'a str,
    ref_name: &'a str,
}

#[derive(Clone, Copy)]
struct MirrorTarget<'a> {
    repo_id: &'a str,
    upstream_repo_id: &'a str,
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
async fn git_clone_resolve_head_ref(client: &AspenClient, request: CloneRefRequest<'_>) -> Result<Option<String>> {
    debug_assert!(!request.repo_id.is_empty(), "git clone repo_id must not be empty");
    debug_assert!(!request.ref_name.is_empty(), "git clone ref_name must not be empty");

    let response = client
        .send(ClientRpcRequest::ForgeGetRef {
            repo_id: request.repo_id.to_string(),
            ref_name: request.ref_name.to_string(),
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeRefResult(result) => {
            if result.is_success {
                let hash = result
                    .ref_info
                    .map(|r| r.hash)
                    .ok_or_else(|| anyhow::anyhow!("Branch '{}' not found", request.branch_name))?;
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
    is_json_output: bool,
) -> Result<()> {
    use std::fs;
    use std::io::Write;

    if !is_json_output {
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
    request: CloneLogRequest<'_>,
) -> Result<Vec<aspen_client_api::ForgeCommitInfo>> {
    debug_assert!(!request.repo_id.is_empty(), "git clone log repo_id must not be empty");
    debug_assert!(!request.ref_name.is_empty(), "git clone log ref_name must not be empty");

    let response = client
        .send(ClientRpcRequest::ForgeLog {
            repo_id: request.repo_id.to_string(),
            ref_name: Some(request.ref_name.to_string()),
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
    debug_assert!(!tree_hash.is_empty(), "git clone tree traversal requires a tree hash");
    debug_assert!(!base_path.is_absolute(), "git clone tree paths must stay relative to target dir");

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
    let mut blob_contents: std::collections::HashMap<String, Vec<u8>> =
        std::collections::HashMap::with_capacity(blob_hashes.len());

    for blob_hash in blob_hashes {
        debug_assert!(!blob_hash.is_empty(), "git clone blob hash must not be empty");
        debug_assert!(blob_hash.len() >= 2, "git clone blob hash must have prefix bytes for object path");

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
        debug_assert!(!entry.hash.is_empty(), "git clone extracted files must reference a blob hash");
        debug_assert!(!entry.path.is_absolute(), "git clone extracted file paths must stay relative");
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
    repo_info: &aspen_client_api::ForgeRepoInfo,
    clone_summary: CloneSummary<'_>,
    commits: &[aspen_client_api::ForgeCommitInfo],
) -> Result<()> {
    use std::fs;
    use std::io::Write;

    debug_assert!(!clone_summary.branch_name.is_empty(), "git clone manifests need a branch name");
    debug_assert!(!clone_summary.head_hash.is_empty(), "git clone manifests need a HEAD hash");

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

    let head_path = target_dir.join(".aspen/refs/heads").join(clone_summary.branch_name);
    fs::write(&head_path, clone_summary.head_hash)
        .with_context(|| format!("failed to write HEAD ref at {}", head_path.display()))?;

    let config = serde_json::json!({
        "repo_id": clone_summary.repo_id,
        "name": repo_info.name,
        "default_branch": repo_info.default_branch,
        "cloned_branch": clone_summary.branch_name,
        "head": clone_summary.head_hash,
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
    is_json_output: bool,
    target_dir: &std::path::Path,
    clone_summary: CloneSummary<'_>,
    artifacts: CloneArtifacts<'_>,
) {
    if is_json_output {
        let output = serde_json::json!({
            "success": true,
            "path": target_dir.display().to_string(),
            "repo_id": clone_summary.repo_id,
            "branch": clone_summary.branch_name,
            "head": clone_summary.head_hash,
            "commits": artifacts.commits.len(),
            "blobs": artifacts.blob_hashes.len(),
            "files": artifacts.file_entries.len(),
        });
        // JSON serialization of a json! value will not fail
        if let Ok(s) = serde_json::to_string_pretty(&output) {
            println!("{}", s);
        }
    } else {
        println!("Cloned '{}' to '{}'", clone_summary.repo_name, target_dir.display());
        println!("  Branch:  {}", clone_summary.branch_name);
        println!("  HEAD:    {}", &clone_summary.head_hash[..16]);
        println!("  Commits: {}", artifacts.commits.len());
        println!("  Files:   {}", artifacts.file_entries.len());
    }
}

async fn git_show(client: &AspenClient, args: ShowArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.repo.is_empty(), "git show requires a repository id");
    debug_assert!(args.repo.len() <= 64, "git show repo id should stay within hash length bounds");

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
                        threshold: repo.threshold_delegates,
                        created_at_ms: repo.created_at_ms,
                    };
                    print_output(&output, is_json_output);
                } else if !is_json_output {
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

async fn git_commit(client: &AspenClient, args: CommitArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.repo.is_empty(), "git commit requires a repository id");
    debug_assert!(!args.tree.is_empty(), "git commit requires a tree hash");
    debug_assert!(!args.message.is_empty(), "git commit requires a message");

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
                    print_output(&output, is_json_output);
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

async fn git_show_commit(client: &AspenClient, args: ShowCommitArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.hash.is_empty(), "git show-commit requires a hash");
    debug_assert!(args.hash.len() == 64, "git show-commit expects a 64-char BLAKE3 hash");

    let response = client
        .send(ClientRpcRequest::ForgeGetCommit {
            hash: args.hash.clone(),
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
                    print_output(&output, is_json_output);
                } else if !is_json_output {
                    println!("Commit not found: {}", args.hash);
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

async fn git_log(client: &AspenClient, args: LogArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.repo.is_empty(), "git log requires a repository id");
    debug_assert!(args.max_commits > 0, "git log should request at least one commit");

    let response = client
        .send(ClientRpcRequest::ForgeLog {
            repo_id: args.repo,
            ref_name: args.ref_name,
            limit: Some(args.max_commits),
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
                print_output(&output, is_json_output);
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
async fn fetch_current_ref_hash(client: &AspenClient, request: RefLookupRequest<'_>) -> Result<Option<String>> {
    debug_assert!(!request.repo_id.is_empty(), "git ref lookup repo_id must not be empty");
    debug_assert!(!request.ref_name.is_empty(), "git ref lookup ref_name must not be empty");

    let response = client
        .send(ClientRpcRequest::ForgeGetRef {
            repo_id: request.repo_id.to_string(),
            ref_name: request.ref_name.to_string(),
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

#[cfg(feature = "forge")]
async fn git_push(client: &AspenClient, args: PushArgs, is_json_output: bool) -> Result<()> {
    use aspen_forge::identity::RepoId;

    let new_hash = git_push_parse_hash(&args.hash)?;
    let repo_id = RepoId::from_hex(&args.repo).map_err(|e| anyhow::anyhow!("invalid repo_id: {}", e))?;

    let current_hash = if !args.is_force {
        fetch_current_ref_hash(client, RefLookupRequest {
            repo_id: &args.repo,
            ref_name: &args.ref_name,
        })
        .await?
    } else {
        None
    };

    let old_hash = git_push_parse_old_hash(&current_hash);
    let (signer, signature, timestamp_ms) =
        git_push_sign_update(&args.key, repo_id, &args.ref_name, new_hash, old_hash)?;

    let response = git_push_send_request(client, &args, current_hash, signer, signature, timestamp_ms).await?;

    git_push_handle_response(response, &args.ref_name, is_json_output)
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
#[cfg(feature = "forge")]
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
#[cfg(feature = "forge")]
async fn git_push_send_request(
    client: &AspenClient,
    args: &PushArgs,
    current_hash: Option<String>,
    signer: Option<String>,
    signature: Option<String>,
    timestamp_ms: Option<u64>,
) -> Result<ClientRpcResponse> {
    if args.is_force {
        Ok(client
            .send(ClientRpcRequest::ForgeSetRef {
                repo_id: args.repo.clone(),
                ref_name: args.ref_name.clone(),
                hash: args.hash.clone(),
                signer,
                signature,
                timestamp_ms,
            })
            .await?)
    } else {
        Ok(client
            .send(ClientRpcRequest::ForgeCasRef {
                repo_id: args.repo.clone(),
                ref_name: args.ref_name.clone(),
                expected: current_hash,
                new_hash: args.hash.clone(),
                signer,
                signature,
                timestamp_ms,
            })
            .await?)
    }
}

/// Handle the push response, printing the result.
fn git_push_handle_response(response: ClientRpcResponse, ref_name: &str, is_json_output: bool) -> Result<()> {
    match response {
        ClientRpcResponse::ForgeRefResult(result) => {
            if result.is_success {
                if let Some(ref_info) = result.ref_info {
                    let output = RefOutput {
                        name: ref_info.name,
                        hash: ref_info.hash,
                    };
                    print_output(&output, is_json_output);
                } else if !is_json_output {
                    println!("Ref {} updated", ref_name);
                }
                Ok(())
            } else {
                anyhow::bail!("{}", result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
        }
        ClientRpcResponse::ForgeOperationResult(result) => {
            if result.is_success {
                if !is_json_output {
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

async fn git_get_ref(client: &AspenClient, args: GetRefArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.repo.is_empty(), "git get-ref requires a repository id");
    debug_assert!(!args.ref_name.is_empty(), "git get-ref requires a ref name");

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
                    print_output(&output, is_json_output);
                } else if !is_json_output {
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

async fn git_store_blob(client: &AspenClient, args: StoreBlobArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.repo.is_empty(), "git store-blob requires a repository id");
    debug_assert!(args.file.file_name().is_some(), "git store-blob requires a concrete file path");

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
                if is_json_output {
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

fn git_get_blob_write_to_path(hash: &str, path: &std::path::Path, content: &[u8], is_json_output: bool) -> Result<()> {
    debug_assert!(!hash.is_empty(), "git get-blob path write needs a blob hash");
    debug_assert!(!content.is_empty(), "git get-blob path write expects blob content");

    std::fs::write(path, content).map_err(|e| anyhow::anyhow!("failed to write file: {}", e))?;
    if !is_json_output {
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
    Ok(())
}

fn git_get_blob_print_content(hash: &str, content: &[u8], is_json_output: bool) {
    debug_assert!(!hash.is_empty(), "git get-blob output needs a blob hash");
    debug_assert!(!content.is_empty(), "git get-blob output expects blob content");

    if is_json_output {
        println!(
            "{}",
            serde_json::json!({
                "success": true,
                "hash": hash,
                "size": content.len(),
                "content_base64": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, content)
            })
        );
        return;
    }

    match std::str::from_utf8(content) {
        Ok(text) => print!("{}", text),
        Err(_) => println!("<binary: {} bytes>", content.len()),
    }
}

fn git_get_blob_exit_not_found(hash: &str, is_json_output: bool) -> ! {
    if is_json_output {
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

async fn git_get_blob(client: &AspenClient, args: GetBlobArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.hash.is_empty(), "git get-blob requires a blob hash");
    debug_assert!(args.hash.len() == 64, "git get-blob expects a 64-char BLAKE3 hash");

    let hash = args.hash.clone();
    let response = client.send(ClientRpcRequest::ForgeGetBlob { hash: args.hash }).await?;

    match response {
        ClientRpcResponse::ForgeBlobResult(result) if result.is_success => {
            let content = match result.content {
                Some(content) => content,
                None => git_get_blob_exit_not_found(&hash, is_json_output),
            };
            if let Some(path) = args.output.as_deref() {
                git_get_blob_write_to_path(&hash, path, &content, is_json_output)
            } else {
                git_get_blob_print_content(&hash, &content, is_json_output);
                Ok(())
            }
        }
        ClientRpcResponse::ForgeBlobResult(result) => {
            let error = result.error.unwrap_or_else(|| "unknown error".to_string());
            if is_json_output {
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

async fn git_create_tree(client: &AspenClient, args: CreateTreeArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.repo.is_empty(), "git create-tree requires a repository id");
    debug_assert!(!args.entries.is_empty(), "git create-tree requires at least one entry");

    // Parse entries from "mode:name:hash" format
    let mut entries = Vec::with_capacity(args.entries.len());
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
                if is_json_output {
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

async fn git_get_tree(client: &AspenClient, args: GetTreeArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.hash.is_empty(), "git get-tree requires a tree hash");
    debug_assert!(args.hash.len() == 64, "git get-tree expects a 64-char BLAKE3 hash");

    let response = client
        .send(ClientRpcRequest::ForgeGetTree {
            hash: args.hash.clone(),
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeTreeResult(result) => {
            if result.is_success {
                if is_json_output {
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

async fn git_export_key(client: &AspenClient, args: ExportKeyArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.repo.is_empty(), "git export-key requires a repository id");
    debug_assert!(args.repo.len() <= 64, "git export-key repo id should stay within hash length bounds");

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
                        if !is_json_output {
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
                    } else if is_json_output {
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

async fn git_federate(client: &AspenClient, args: FederateArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.repo.is_empty(), "git federate requires a repository id");
    debug_assert!(!args.mode.is_empty(), "git federate requires a federation mode");

    let response = client
        .send(ClientRpcRequest::FederateRepository {
            repo_id: args.repo,
            mode: args.mode,
        })
        .await?;

    match response {
        ClientRpcResponse::FederateRepositoryResult(result) => {
            if result.is_success {
                if is_json_output {
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

async fn git_list_federated(client: &AspenClient, _args: ListFederatedArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(std::mem::size_of::<ListFederatedArgs>() == 0, "git list-federated args should stay empty");
    debug_assert!(std::mem::size_of::<ClientRpcRequest>() > 0, "git list-federated request type must be available");

    let response = client.send(ClientRpcRequest::ListFederatedRepositories).await?;

    match response {
        ClientRpcResponse::FederatedRepositories(result) => {
            if result.error.is_none() {
                if is_json_output {
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

async fn git_fetch_remote(client: &AspenClient, args: FetchRemoteArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.fed_id.is_empty(), "git fetch-remote requires a federated id");
    debug_assert!(!args.cluster.is_empty(), "git fetch-remote requires a cluster name");

    let response = client
        .send(ClientRpcRequest::ForgeFetchFederated {
            federated_id: args.fed_id,
            remote_cluster: args.cluster,
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeFetchResult(result) => {
            if result.is_success {
                if is_json_output {
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

async fn git_fork(client: &AspenClient, args: ForkArgs, is_json_output: bool) -> Result<()> {
    debug_assert!(!args.upstream.is_empty(), "git fork requires an upstream repository id");
    debug_assert!(!args.name.is_empty(), "git fork requires a target repository name");

    let response = client
        .send(ClientRpcRequest::ForgeForkRepo {
            upstream_repo_id: args.upstream.clone(),
            name: args.name.clone(),
            description: None,
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeRepoResult(result) => {
            if result.is_success {
                if let Some(repo) = &result.repo {
                    if is_json_output {
                        println!("{}", serde_json::to_string_pretty(&repo)?);
                    } else {
                        println!("Forked {} → {}", args.upstream, repo.name);
                        println!("  ID: {}", repo.id);
                    }
                }
                Ok(())
            } else {
                anyhow::bail!("{}", result.error.unwrap_or_else(|| "fork failed".to_string()))
            }
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn git_mirror_show_status(client: &AspenClient, repo_id: &str, is_json_output: bool) -> Result<()> {
    debug_assert!(!repo_id.is_empty(), "git mirror status requires a repo id");
    debug_assert!(repo_id.len() <= 64, "git mirror repo id should stay within hash length bounds");

    let response = client
        .send(ClientRpcRequest::ForgeGetMirrorStatus {
            repo_id: repo_id.to_string(),
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeMirrorStatus(status) => {
            if let Some(status_info) = &status {
                if is_json_output {
                    println!("{}", serde_json::to_string_pretty(status_info)?);
                } else {
                    println!("Mirror for {}", repo_id);
                    println!("  Upstream: {}", status_info.upstream_repo_id);
                    println!("  Interval: {}s", status_info.interval_secs);
                    println!("  Enabled:  {}", status_info.enabled);
                    println!("  Last sync: {}ms", status_info.last_sync_ms);
                    println!("  Synced refs: {}", status_info.synced_refs_count);
                    println!("  Due: {}", status_info.is_due);
                }
            } else if is_json_output {
                println!("null");
            } else {
                println!("No mirror configured for {}", repo_id);
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn git_mirror_disable(client: &AspenClient, repo_id: &str, is_json_output: bool) -> Result<()> {
    debug_assert!(!repo_id.is_empty(), "git mirror disable requires a repo id");
    debug_assert!(repo_id.len() <= 64, "git mirror repo id should stay within hash length bounds");

    let response = client
        .send(ClientRpcRequest::ForgeDisableMirror {
            repo_id: repo_id.to_string(),
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeOperationResult(result) if result.is_success => {
            if !is_json_output {
                println!("Mirror disabled for {}", repo_id);
            }
            Ok(())
        }
        ClientRpcResponse::ForgeOperationResult(result) => {
            anyhow::bail!("{}", result.error.unwrap_or_else(|| "failed".to_string()))
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn git_mirror_set(
    client: &AspenClient,
    target: MirrorTarget<'_>,
    interval_secs: u32,
    is_json_output: bool,
) -> Result<()> {
    debug_assert!(!target.repo_id.is_empty(), "git mirror set requires a repo id");
    debug_assert!(!target.upstream_repo_id.is_empty(), "git mirror set requires an upstream repo id");

    let response = client
        .send(ClientRpcRequest::ForgeSetMirror {
            repo_id: target.repo_id.to_string(),
            upstream_repo_id: target.upstream_repo_id.to_string(),
            interval_secs,
        })
        .await?;

    match response {
        ClientRpcResponse::ForgeOperationResult(result) if result.is_success => {
            if !is_json_output {
                println!(
                    "Mirror enabled for {} → {} (interval: {}s)",
                    target.repo_id, target.upstream_repo_id, interval_secs
                );
            }
            Ok(())
        }
        ClientRpcResponse::ForgeOperationResult(result) => {
            anyhow::bail!("{}", result.error.unwrap_or_else(|| "failed".to_string()))
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}

async fn git_mirror(client: &AspenClient, args: MirrorArgs, is_json_output: bool) -> Result<()> {
    if args.status {
        return git_mirror_show_status(client, &args.repo, is_json_output).await;
    }

    if args.disable {
        return git_mirror_disable(client, &args.repo, is_json_output).await;
    }

    let upstream = args.upstream.ok_or_else(|| anyhow::anyhow!("--upstream is required to set mirror"))?;
    git_mirror_set(
        client,
        MirrorTarget {
            repo_id: &args.repo,
            upstream_repo_id: &upstream,
        },
        args.interval_secs,
        is_json_output,
    )
    .await
}

#[derive(Args)]
pub struct DiffArgs {
    /// Repository name or ID.
    pub repo: String,

    /// Old ref or commit hash.
    pub old_ref: String,

    /// New ref or commit hash (if omitted, shows parent→HEAD diff).
    pub new_ref: Option<String>,

    /// Show diffstat summary instead of full diff.
    #[arg(long)]
    pub stat: bool,

    /// Show only changed file names.
    #[arg(long)]
    pub name_only: bool,

    /// Number of context lines (default 3).
    #[arg(long, default_value = "3")]
    pub context: u32,
}

async fn git_diff(client: &AspenClient, args: DiffArgs, is_json_output: bool) -> Result<()> {
    let repo_id = args.repo.clone();
    let should_include_content = !args.name_only;
    let context_lines = Some(args.context);

    let response = if let Some(new_ref) = &args.new_ref {
        client
            .send(ClientRpcRequest::ForgeDiffRefs {
                repo_id: repo_id.clone(),
                old_ref: args.old_ref.clone(),
                new_ref: new_ref.clone(),
                include_content: should_include_content,
                context_lines,
            })
            .await?
    } else {
        // Single ref: resolve to commit, diff against its parent
        let ref_response = client
            .send(ClientRpcRequest::ForgeGetRef {
                repo_id: repo_id.clone(),
                ref_name: args.old_ref.clone(),
            })
            .await?;

        let commit_hash = match ref_response {
            ClientRpcResponse::ForgeRefResult(r) if r.was_found => r.ref_info.context("missing ref info")?.hash,
            ClientRpcResponse::ForgeRefResult(_) => {
                anyhow::bail!("ref not found: {}", args.old_ref);
            }
            ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
            _ => anyhow::bail!("unexpected response type"),
        };

        // Get the commit to find its parent
        let commit_response = client
            .send(ClientRpcRequest::ForgeGetCommit {
                hash: commit_hash.clone(),
            })
            .await?;

        let parent_hash = match commit_response {
            ClientRpcResponse::ForgeCommitResult(r) if r.is_success => {
                let commit = r.commit.context("missing commit")?;
                commit.parents.first().cloned()
            }
            ClientRpcResponse::ForgeCommitResult(r) => {
                anyhow::bail!("{}", r.error.unwrap_or_else(|| "commit not found".to_string()));
            }
            ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
            _ => anyhow::bail!("unexpected response type"),
        };

        match parent_hash {
            Some(parent) => {
                client
                    .send(ClientRpcRequest::ForgeDiffCommits {
                        repo_id: repo_id.clone(),
                        old_commit: parent,
                        new_commit: commit_hash,
                        include_content: should_include_content,
                        context_lines,
                    })
                    .await?
            }
            None => {
                // Root commit — diff against empty tree
                // Return empty diff for now
                println!("(root commit — no parent to diff against)");
                return Ok(());
            }
        }
    };

    match response {
        ClientRpcResponse::ForgeDiffResult(result) => {
            if is_json_output {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else if args.name_only {
                for entry in &result.entries {
                    println!("{}", entry.path);
                }
            } else if args.stat {
                // Stat mode: show summary from entries
                for entry in &result.entries {
                    let kind_char = match entry.kind.as_str() {
                        "added" => 'A',
                        "removed" => 'D',
                        "modified" => 'M',
                        "renamed" => 'R',
                        _ => '?',
                    };
                    if let Some(old_path) = &entry.old_path {
                        println!("{kind_char}\t{old_path} => {}", entry.path);
                    } else {
                        println!("{kind_char}\t{}", entry.path);
                    }
                }
            } else if let Some(diff_text) = &result.unified_diff {
                // Color the output
                for line in diff_text.lines() {
                    if line.starts_with('+') {
                        println!("\x1b[32m{line}\x1b[0m");
                    } else if line.starts_with('-') {
                        println!("\x1b[31m{line}\x1b[0m");
                    } else if line.starts_with("@@") {
                        println!("\x1b[36m{line}\x1b[0m");
                    } else if line.starts_with("rename ") {
                        println!("\x1b[33m{line}\x1b[0m");
                    } else {
                        println!("{line}");
                    }
                }
            } else {
                // No content loaded, just show entry summaries
                for entry in &result.entries {
                    println!("{}: {}", entry.kind, entry.path);
                }
            }

            if result.truncated {
                eprintln!("(diff truncated — too many changed files)");
            }

            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response type"),
    }
}
