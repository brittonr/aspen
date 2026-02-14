//! Working directory command handlers.
//!
//! Handles init, add, reset, status, record, checkout, diff, conflicts,
//! and solve operations for Pijul working directories.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use aspen_blob::InMemoryBlobStore;
use aspen_forge::identity::RepoId;
use aspen_pijul::AspenChangeStore;
use aspen_pijul::ChangeDirectory;
use aspen_pijul::ChangeRecorder;
use aspen_pijul::PristineManager;
use aspen_pijul::ResolutionStrategy;
use aspen_pijul::WorkingDirectory;

use super::WdAddArgs;
use super::WdCheckoutArgs;
use super::WdConflictsArgs;
use super::WdDiffArgs;
use super::WdInitArgs;
use super::WdRecordArgs;
use super::WdResetArgs;
use super::WdSolveArgs;
use super::WdStatusArgs;
use super::output::ConflictInfo;
use super::output::DiffHunk;
use super::output::MarkerInfo;
use super::output::PijulCheckoutOutput;
use super::output::PijulDiffOutput;
use super::output::PijulRecordOutput;
use super::output::PijulStrategiesOutput;
use super::output::PijulWdAddOutput;
use super::output::PijulWdConflictsOutput;
use super::output::PijulWdInitOutput;
use super::output::PijulWdResetOutput;
use super::output::PijulWdSolveOutput;
use super::output::PijulWdStatusOutput;
use super::output::StrategyInfo;
use super::repo_cache_dir;
use crate::client::AspenClient;
use crate::output::print_output;
use crate::output::print_success;

pub(super) fn wd_init(args: WdInitArgs, json: bool) -> Result<()> {
    // Parse repo ID
    let repo_id = RepoId::from_hex(&args.repo_id).context("invalid repository ID format")?;

    // Determine working directory path
    let path = args
        .path
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    // Initialize working directory
    let wd = WorkingDirectory::init(&path, &repo_id, &args.channel, args.remote.clone())
        .context("failed to initialize working directory")?;

    let output = PijulWdInitOutput {
        path: wd.root().display().to_string(),
        repo_id: args.repo_id,
        channel: args.channel,
        remote: args.remote,
    };
    print_output(&output, json);

    Ok(())
}

pub(super) fn wd_add(args: WdAddArgs, json: bool) -> Result<()> {
    // Find or open working directory
    let start_path = args
        .path
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let wd =
        WorkingDirectory::find(&start_path).context("not in a Pijul working directory (run 'pijul wd init' first)")?;

    // Add files
    let result = wd.add(&args.paths).context("failed to add files")?;

    let output = PijulWdAddOutput {
        added: result.added,
        already_staged: result.already_staged,
        not_found: result.not_found,
    };
    print_output(&output, json);

    Ok(())
}

pub(super) fn wd_reset(args: WdResetArgs, json: bool) -> Result<()> {
    // Find or open working directory
    let start_path = args
        .path
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let wd =
        WorkingDirectory::find(&start_path).context("not in a Pijul working directory (run 'pijul wd init' first)")?;

    // Reset files
    let result = if args.all || args.paths.is_empty() {
        wd.reset_all().context("failed to reset all files")?
    } else {
        wd.reset(&args.paths).context("failed to reset files")?
    };

    let output = PijulWdResetOutput {
        removed: result.removed,
        not_staged: result.not_staged,
    };
    print_output(&output, json);

    Ok(())
}

pub(super) fn wd_status(args: WdStatusArgs, json: bool) -> Result<()> {
    // Find or open working directory
    let start_path = args
        .path
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let wd =
        WorkingDirectory::find(&start_path).context("not in a Pijul working directory (run 'pijul wd init' first)")?;

    // Get staged files
    let staged = wd.staged_files().context("failed to read staged files")?;

    // Porcelain output: machine-readable format
    if args.porcelain {
        // Output format: XY PATH
        // X = index status, Y = working tree status
        // A = added/staged, ? = untracked
        // Currently we only track staged files, so all staged files are "A "
        for path in &staged {
            println!("A  {}", path);
        }
        return Ok(());
    }

    let config = wd.config();

    let output = PijulWdStatusOutput {
        path: wd.root().display().to_string(),
        repo_id: config.repo_id.clone(),
        channel: config.channel.clone(),
        staged,
        remote: config.remote.clone(),
        last_synced_head: config.last_synced_head.clone(),
    };
    print_output(&output, json);

    Ok(())
}

/// Record changes from working directory with auto-detected repo and channel.
pub(super) async fn wd_record(client: &AspenClient, args: WdRecordArgs, json: bool) -> Result<()> {
    use aspen_client_api::ClientRpcRequest;
    use aspen_client_api::ClientRpcResponse;
    use aspen_pijul::ChangeMetadata;
    use aspen_pijul::PijulAuthor;
    use tracing::info;

    // Find working directory
    let start_path = args
        .path
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let wd =
        WorkingDirectory::find(&start_path).context("not in a Pijul working directory (run 'pijul wd init' first)")?;

    // Check staged files
    let staged = wd.staged_files().context("failed to read staged files")?;
    if staged.is_empty() {
        anyhow::bail!("No staged files. Use 'pijul wd add <files>' first.");
    }

    // Get repo_id and channel from working directory config
    let config = wd.config();
    let repo_id = RepoId::from_hex(&config.repo_id).context("invalid repository ID in working directory config")?;
    let channel = config.channel.clone();

    info!(
        repo_id = %config.repo_id,
        channel = %channel,
        staged_files = staged.len(),
        "recording changes from working directory"
    );

    // Determine cache directory for local pristine
    let cache_dir = repo_cache_dir(&repo_id).context("failed to get cache directory")?;

    // Create cache directory if needed
    std::fs::create_dir_all(&cache_dir).context("failed to create cache directory")?;

    // Create local pristine manager
    let pristine_mgr = PristineManager::new(&cache_dir);
    let pristine = pristine_mgr.open_or_create(&repo_id).context("failed to open/create local pristine")?;

    // Create temporary in-memory blob store for recording
    let temp_blobs = Arc::new(InMemoryBlobStore::new());
    let change_store = Arc::new(AspenChangeStore::new(temp_blobs.clone()));
    let change_dir = ChangeDirectory::new(&cache_dir, repo_id, change_store.clone());

    // Create author string
    let author_str = match (args.author, args.email) {
        (Some(name), Some(email)) => format!("{} <{}>", name, email),
        (Some(name), None) => name,
        (None, Some(email)) => format!("<{}>", email),
        (None, None) => {
            // Try to get from git config or environment
            let name = std::env::var("USER").unwrap_or_else(|_| "Unknown".to_string());
            format!("{} <{}@local>", name, name)
        }
    };

    // Record changes
    let recorder =
        ChangeRecorder::new(pristine.clone(), change_dir, wd.root().to_path_buf()).with_threads(args.threads);

    let result = recorder.record(&channel, &args.message, &author_str).await.context("failed to record changes")?;

    match result {
        Some(record_result) => {
            info!(
                hash = %record_result.hash,
                hunks = record_result.num_hunks,
                bytes = record_result.size_bytes,
                "recorded change locally"
            );

            // Push to cluster if requested
            if args.push {
                // Get the change bytes from local store
                let change_bytes = change_store
                    .get_change(&record_result.hash)
                    .await
                    .context("failed to get change from local store")?
                    .context("change not found in local store after recording")?;

                // Upload change to cluster blob store
                let blob_response = client
                    .send(ClientRpcRequest::AddBlob {
                        data: change_bytes.clone(),
                        tag: Some(format!("pijul:{}:{}", config.repo_id, record_result.hash)),
                    })
                    .await
                    .context("failed to upload change to cluster")?;

                match blob_response {
                    ClientRpcResponse::AddBlobResult(_) => {
                        info!(hash = %record_result.hash, "uploaded change to cluster blob store");
                    }
                    ClientRpcResponse::Error(e) => {
                        anyhow::bail!("failed to upload change: {}: {}", e.code, e.message);
                    }
                    _ => anyhow::bail!("unexpected response from blob add"),
                }

                // Apply the change to update channel head
                let apply_response = client
                    .send(ClientRpcRequest::PijulApply {
                        repo_id: config.repo_id.clone(),
                        channel: channel.clone(),
                        change_hash: record_result.hash.to_string(),
                    })
                    .await
                    .context("failed to apply change to cluster")?;

                match apply_response {
                    ClientRpcResponse::PijulApplyResult(_) => {
                        info!(channel = %channel, "updated channel head via Raft");
                    }
                    ClientRpcResponse::Error(e) => {
                        anyhow::bail!("failed to apply change: {}: {}", e.code, e.message);
                    }
                    _ => anyhow::bail!("unexpected response from apply"),
                }

                // Store change metadata
                let (author_name, author_email) = if let Some(start) = author_str.find('<') {
                    if let Some(end) = author_str.find('>') {
                        let name = author_str[..start].trim().to_string();
                        let email = author_str[start + 1..end].to_string();
                        (name, email)
                    } else {
                        (author_str.clone(), String::new())
                    }
                } else {
                    (author_str.clone(), String::new())
                };

                let metadata = ChangeMetadata {
                    hash: record_result.hash,
                    repo_id: RepoId::from_hex(&config.repo_id).unwrap(),
                    channel: channel.clone(),
                    message: args.message.clone(),
                    authors: vec![PijulAuthor::from_name_email(author_name, author_email)],
                    dependencies: record_result.dependencies.clone(),
                    size_bytes: record_result.size_bytes as u64,
                    recorded_at_ms: chrono::Utc::now().timestamp_millis() as u64,
                };

                let meta_bytes = postcard::to_allocvec(&metadata).context("failed to serialize change metadata")?;
                let meta_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &meta_bytes);
                let meta_key = format!("pijul:change:meta:{}:{}", config.repo_id, record_result.hash);

                let _ = client
                    .send(ClientRpcRequest::WriteKey {
                        key: meta_key,
                        value: meta_b64.into_bytes(),
                    })
                    .await;
            }

            // Clear staged files on success
            wd.reset_all().context("failed to clear staged files")?;

            let output = PijulRecordOutput {
                change_hash: record_result.hash.to_string(),
                message: args.message,
                hunks: record_result.num_hunks as u32,
                size_bytes: record_result.size_bytes as u64,
            };
            print_output(&output, json);
            Ok(())
        }
        None => {
            print_success("No changes to record", json);
            Ok(())
        }
    }
}

/// Checkout channel to working directory with auto-detected repo.
pub(super) async fn wd_checkout(client: &AspenClient, args: WdCheckoutArgs, json: bool) -> Result<()> {
    use aspen_client_api::ClientRpcRequest;
    use aspen_client_api::ClientRpcResponse;
    use aspen_pijul::WorkingDirOutput;
    use tracing::info;

    // Find working directory
    let start_path = args
        .path
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let wd =
        WorkingDirectory::find(&start_path).context("not in a Pijul working directory (run 'pijul wd init' first)")?;

    // Get config
    let config = wd.config();
    let repo_id = RepoId::from_hex(&config.repo_id).context("invalid repository ID in working directory config")?;
    let channel = args.channel.unwrap_or_else(|| config.channel.clone());

    info!(
        repo_id = %config.repo_id,
        channel = %channel,
        "checking out to working directory"
    );

    // Determine cache directory
    let cache_dir = repo_cache_dir(&repo_id).context("failed to get cache directory")?;

    // Pull from cluster if requested
    if args.pull {
        // Create cache directory if needed
        std::fs::create_dir_all(&cache_dir).context("failed to create cache directory")?;

        // Sync the channel from cluster
        let response = client
            .send(ClientRpcRequest::PijulChannelInfo {
                repo_id: config.repo_id.clone(),
                name: channel.clone(),
            })
            .await?;

        match response {
            ClientRpcResponse::PijulChannelResult(_channel_info) => {
                info!(channel = %channel, "synced channel info from cluster");
            }
            ClientRpcResponse::Error(e) => {
                anyhow::bail!("failed to get channel info: {}: {}", e.code, e.message);
            }
            _ => anyhow::bail!("unexpected response type"),
        }
    }

    // Check if cache exists
    if !cache_dir.exists() {
        anyhow::bail!("Local cache not found at {}. Run 'pijul sync {}' first.", cache_dir.display(), config.repo_id);
    }

    // Create local pristine manager
    let pristine_mgr = PristineManager::new(&cache_dir);
    let pristine = pristine_mgr.open(&repo_id).context("failed to open local pristine - run 'pijul sync' first")?;

    // Create change directory
    let temp_blobs = Arc::new(InMemoryBlobStore::new());
    let change_store = Arc::new(AspenChangeStore::new(temp_blobs));
    let change_dir = ChangeDirectory::new(&cache_dir, repo_id, change_store);

    // Output to working directory
    let outputter = WorkingDirOutput::new(pristine, change_dir, wd.root().to_path_buf());
    let result = outputter.output(&channel).context("failed to output to working directory")?;

    let conflicts = result.conflict_count() as u32;

    let output = PijulCheckoutOutput {
        channel,
        output_dir: wd.root().display().to_string(),
        files_written: 0, // WorkingDirOutput doesn't track this currently
        conflicts,
    };
    print_output(&output, json);

    if conflicts > 0 {
        std::process::exit(1);
    }
    Ok(())
}

/// Show diff between working directory and channel with auto-detected repo.
pub(super) async fn wd_diff(args: WdDiffArgs, json: bool) -> Result<()> {
    use tracing::info;

    // Find working directory
    let start_path = args
        .path
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let wd =
        WorkingDirectory::find(&start_path).context("not in a Pijul working directory (run 'pijul wd init' first)")?;

    // Get config
    let config = wd.config();
    let repo_id = RepoId::from_hex(&config.repo_id).context("invalid repository ID in working directory config")?;
    let channel = config.channel.clone();

    info!(
        repo_id = %config.repo_id,
        channel = %channel,
        "computing diff for working directory"
    );

    // Determine cache directory
    let cache_dir = repo_cache_dir(&repo_id).context("failed to get cache directory")?;

    // Check if cache exists
    if !cache_dir.exists() {
        anyhow::bail!("Local cache not found at {}. Run 'pijul sync {}' first.", cache_dir.display(), config.repo_id);
    }

    // Create local pristine manager
    let pristine_mgr = PristineManager::new(&cache_dir);
    let pristine = pristine_mgr.open(&repo_id).context("failed to open local pristine - run 'pijul sync' first")?;

    // Create change directory
    let temp_blobs = Arc::new(InMemoryBlobStore::new());
    let change_store = Arc::new(AspenChangeStore::new(temp_blobs));
    let change_dir = ChangeDirectory::new(&cache_dir, repo_id, change_store);

    // Create recorder to get diff info
    let recorder = ChangeRecorder::new(pristine, change_dir, wd.root().to_path_buf());

    // Get the diff
    let diff_result = recorder.diff(&channel).await.context("failed to compute diff")?;

    // Convert to output format
    let mut hunks = Vec::new();
    let mut files_added = 0u32;
    let mut files_deleted = 0u32;
    let mut files_modified = 0u32;
    let mut lines_added = 0u32;
    let mut lines_deleted = 0u32;

    for hunk_info in &diff_result.hunks {
        let change_type = match hunk_info.kind.as_str() {
            "add" | "new" => {
                files_added += 1;
                "add"
            }
            "delete" | "remove" => {
                files_deleted += 1;
                "delete"
            }
            "modify" | "edit" => {
                files_modified += 1;
                "modify"
            }
            "rename" => {
                files_modified += 1;
                "rename"
            }
            "permission" | "perm" => {
                files_modified += 1;
                "permission"
            }
            _ => {
                files_modified += 1;
                "modify"
            }
        };

        lines_added += hunk_info.additions;
        lines_deleted += hunk_info.deletions;

        hunks.push(DiffHunk {
            path: hunk_info.path.clone(),
            change_type: change_type.to_string(),
            additions: hunk_info.additions,
            deletions: hunk_info.deletions,
        });
    }

    let no_changes = hunks.is_empty();

    // Show summary only if requested
    if args.summary {
        if no_changes {
            print_success("No changes", json);
        } else {
            let summary = format!(
                "{} files changed: +{} added, -{} deleted, ~{} modified ({} insertions, {} deletions)",
                files_added + files_deleted + files_modified,
                files_added,
                files_deleted,
                files_modified,
                lines_added,
                lines_deleted
            );
            print_success(&summary, json);
        }
        return Ok(());
    }

    let output = PijulDiffOutput {
        repo_id: config.repo_id.clone(),
        channel: channel.clone(),
        channel2: None,
        working_dir: Some(wd.root().display().to_string()),
        hunks,
        files_added,
        files_deleted,
        files_modified,
        lines_added,
        lines_deleted,
        no_changes,
    };

    print_output(&output, json);
    Ok(())
}

/// List conflicts in the working directory.
pub(super) fn wd_conflicts(args: WdConflictsArgs, json: bool) -> Result<()> {
    // Find working directory
    let start_path = args
        .path
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let wd =
        WorkingDirectory::find(&start_path).context("not in a Pijul working directory (run 'pijul wd init' first)")?;

    let config = wd.config();

    // For now, we scan the working directory for conflict markers
    // In a full implementation, this would integrate with the pristine database
    let conflicts = scan_for_conflict_markers(wd.root(), args.detailed)?;

    let output = PijulWdConflictsOutput {
        path: wd.root().display().to_string(),
        channel: config.channel.clone(),
        total: conflicts.len(),
        conflicts,
    };

    print_output(&output, json);
    Ok(())
}

/// Scan a directory for files containing Pijul conflict markers.
pub(super) fn scan_for_conflict_markers(root: &std::path::Path, detailed: bool) -> Result<Vec<ConflictInfo>> {
    use std::fs;

    let mut conflicts = Vec::new();

    // Walk the directory tree, skipping .pijul directory
    fn visit_dir(
        dir: &std::path::Path,
        root: &std::path::Path,
        detailed: bool,
        conflicts: &mut Vec<ConflictInfo>,
    ) -> Result<()> {
        if !dir.is_dir() {
            return Ok(());
        }

        for entry in fs::read_dir(dir).context("failed to read directory")? {
            let entry = entry.context("failed to read directory entry")?;
            let path = entry.path();

            // Skip .pijul directory
            if path.file_name().map(|n| n == ".pijul").unwrap_or(false) {
                continue;
            }

            if path.is_dir() {
                visit_dir(&path, root, detailed, conflicts)?;
            } else if path.is_file() {
                // Check for conflict markers in text files
                if let Some(conflict) = check_file_for_conflicts(&path, root, detailed) {
                    conflicts.push(conflict);
                }
            }
        }

        Ok(())
    }

    visit_dir(root, root, detailed, &mut conflicts)?;
    Ok(conflicts)
}

/// Check a single file for Pijul conflict markers.
fn check_file_for_conflicts(path: &std::path::Path, root: &std::path::Path, detailed: bool) -> Option<ConflictInfo> {
    use std::fs::File;
    use std::io::BufRead;
    use std::io::BufReader;

    // Try to open and read the file
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);

    let mut start_line: Option<u32> = None;
    let mut end_line: Option<u32> = None;
    let mut has_conflict = false;

    // Pijul conflict markers
    const CONFLICT_START: &str = ">>>>>";
    const CONFLICT_END: &str = "<<<<<";

    for (line_num, line_result) in reader.lines().enumerate() {
        let line = line_result.ok()?;

        if line.starts_with(CONFLICT_START) {
            has_conflict = true;
            if start_line.is_none() {
                start_line = Some((line_num + 1) as u32);
            }
        } else if line.starts_with(CONFLICT_END) {
            end_line = Some((line_num + 1) as u32);
        }
    }

    if !has_conflict {
        return None;
    }

    let relative_path = path.strip_prefix(root).ok()?.to_string_lossy().to_string();

    let markers = if detailed {
        start_line.map(|start| MarkerInfo {
            start_line: start,
            end_line: end_line.unwrap_or(start),
        })
    } else {
        None
    };

    Some(ConflictInfo {
        file_path: relative_path,
        kind: "order".to_string(), // Most common type
        status: "unresolved".to_string(),
        markers,
    })
}

/// Resolve conflicts in the working directory.
pub(super) fn wd_solve(args: WdSolveArgs, json: bool) -> Result<()> {
    // Handle --list flag
    if args.list {
        let strategies = vec![
            StrategyInfo {
                name: "ours".to_string(),
                description: "Keep our version, discard theirs".to_string(),
            },
            StrategyInfo {
                name: "theirs".to_string(),
                description: "Keep their version, discard ours".to_string(),
            },
            StrategyInfo {
                name: "newest".to_string(),
                description: "Keep the most recently authored change".to_string(),
            },
            StrategyInfo {
                name: "oldest".to_string(),
                description: "Keep the oldest change".to_string(),
            },
            StrategyInfo {
                name: "manual".to_string(),
                description: "User manually resolves in the working directory".to_string(),
            },
        ];

        let output = PijulStrategiesOutput { strategies };
        print_output(&output, json);
        return Ok(());
    }

    // Parse strategy
    let strategy: ResolutionStrategy = args.strategy.parse().map_err(|e: String| anyhow::anyhow!(e))?;

    // Find working directory
    let start_path = args
        .path
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let wd =
        WorkingDirectory::find(&start_path).context("not in a Pijul working directory (run 'pijul wd init' first)")?;

    // Collect files to resolve
    let files_to_resolve: Vec<String> = if args.all {
        // Scan for all conflicts
        let conflicts = scan_for_conflict_markers(wd.root(), false)?;
        conflicts.into_iter().map(|c| c.file_path).collect()
    } else if args.paths.is_empty() {
        anyhow::bail!("specify file path(s) or use --all to resolve all conflicts");
    } else {
        args.paths.clone()
    };

    if files_to_resolve.is_empty() {
        let output = PijulWdSolveOutput {
            path: wd.root().display().to_string(),
            resolved: vec![],
            strategy: strategy.to_string(),
            remaining: 0,
        };
        print_output(&output, json);
        return Ok(());
    }

    // Resolve conflicts based on strategy
    let mut resolved = Vec::new();
    let mut errors = Vec::new();

    for file_path in &files_to_resolve {
        let full_path = wd.root().join(file_path);
        if !full_path.exists() {
            errors.push(format!("{}: file not found", file_path));
            continue;
        }

        match resolve_file_conflict(&full_path, strategy) {
            Ok(true) => resolved.push(file_path.clone()),
            Ok(false) => {} // No conflict in file
            Err(e) => errors.push(format!("{}: {}", file_path, e)),
        }
    }

    // Report errors
    if !errors.is_empty() && !json {
        for error in &errors {
            eprintln!("Warning: {}", error);
        }
    }

    // Get remaining conflicts
    let remaining = scan_for_conflict_markers(wd.root(), false)?;

    let output = PijulWdSolveOutput {
        path: wd.root().display().to_string(),
        resolved,
        strategy: strategy.to_string(),
        remaining: remaining.len(),
    };

    print_output(&output, json);
    Ok(())
}

/// Resolve conflicts in a single file based on the specified strategy.
fn resolve_file_conflict(path: &std::path::Path, strategy: ResolutionStrategy) -> Result<bool> {
    use std::fs;

    let content = fs::read_to_string(path).context("failed to read file")?;

    // Pijul conflict markers
    const CONFLICT_START: &str = ">>>>>";
    const CONFLICT_SEPARATOR: &str = "=====";
    const CONFLICT_END: &str = "<<<<<";

    if !content.contains(CONFLICT_START) {
        return Ok(false);
    }

    let mut result = String::new();
    let mut in_conflict = false;
    let mut in_ours = false;
    let mut in_theirs = false;
    let mut ours_content = String::new();
    let mut theirs_content = String::new();

    for line in content.lines() {
        if line.starts_with(CONFLICT_START) {
            in_conflict = true;
            in_ours = true;
            in_theirs = false;
            ours_content.clear();
            theirs_content.clear();
        } else if line.starts_with(CONFLICT_SEPARATOR) && in_conflict {
            in_ours = false;
            in_theirs = true;
        } else if line.starts_with(CONFLICT_END) && in_conflict {
            // Resolve based on strategy
            let resolved_content = match strategy {
                ResolutionStrategy::Ours => &ours_content,
                ResolutionStrategy::Theirs => &theirs_content,
                ResolutionStrategy::Newest | ResolutionStrategy::Oldest => {
                    // For now, treat newest/oldest like theirs (would need metadata)
                    &theirs_content
                }
                ResolutionStrategy::Manual => {
                    // Keep the conflict markers for manual resolution
                    result.push_str(CONFLICT_START);
                    result.push('\n');
                    result.push_str(&ours_content);
                    result.push_str(CONFLICT_SEPARATOR);
                    result.push('\n');
                    result.push_str(&theirs_content);
                    result.push_str(CONFLICT_END);
                    result.push('\n');
                    in_conflict = false;
                    in_ours = false;
                    in_theirs = false;
                    continue;
                }
            };
            result.push_str(resolved_content);
            in_conflict = false;
            in_ours = false;
            in_theirs = false;
        } else if in_ours {
            ours_content.push_str(line);
            ours_content.push('\n');
        } else if in_theirs {
            theirs_content.push_str(line);
            theirs_content.push('\n');
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }

    // Write back the resolved content
    fs::write(path, result).context("failed to write resolved file")?;

    Ok(true)
}
