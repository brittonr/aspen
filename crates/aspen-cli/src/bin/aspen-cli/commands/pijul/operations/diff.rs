//! Diff operation handlers.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use aspen_blob::InMemoryBlobStore;
use aspen_forge::identity::RepoId;
use aspen_pijul::AspenChangeStore;
use aspen_pijul::ChangeDirectory;
use aspen_pijul::PristineManager;

use super::super::DiffArgs;
use super::super::output::DiffHunk;
use super::super::output::PijulDiffOutput;
use super::super::repo_cache_dir;
use crate::output::print_output;

/// Show differences between working directory and pristine state.
///
/// This uses the same recording mechanism as `pijul record` but only shows
/// what would be recorded without actually creating a change.
pub(in super::super) async fn pijul_diff(args: DiffArgs, json: bool) -> Result<()> {
    use aspen_pijul::ChangeRecorder;
    use tracing::info;

    // Parse repo ID
    let repo_id = RepoId::from_hex(&args.repo_id).context("invalid repository ID format")?;

    // Determine cache directory
    let cache_dir = match args.data_dir {
        Some(dir) => dir,
        None => repo_cache_dir(&repo_id).context("failed to determine cache directory")?,
    };

    // Check for channel-to-channel diff
    if let Some(ref channel2) = args.channel2 {
        // Channel-to-channel diff: show changes in channel2 not in channel1
        return pijul_diff_channels(&args.repo_id, &args.channel, channel2, &cache_dir, json).await;
    }

    // Working directory diff - require working_dir
    let working_dir = args
        .working_dir
        .context("--working-dir is required when diffing working directory against a channel")?;

    // Check if cache exists
    if !cache_dir.exists() {
        anyhow::bail!("Local cache not found at {}. Run 'pijul sync {}' first.", cache_dir.display(), args.repo_id);
    }

    info!(cache_dir = %cache_dir.display(), "using local cache for diff");

    // Create local pristine manager
    let pristine_mgr = PristineManager::new(&cache_dir);
    let pristine = pristine_mgr.open(&repo_id).context("failed to open local pristine - run 'pijul sync' first")?;

    // Create temporary in-memory blob store
    let temp_blobs = Arc::new(InMemoryBlobStore::new());
    let change_store = Arc::new(AspenChangeStore::new(temp_blobs));
    let change_dir = ChangeDirectory::new(&cache_dir, repo_id, change_store);

    // Create recorder with performance options to get diff info
    let mut recorder =
        ChangeRecorder::new(pristine, change_dir, PathBuf::from(&working_dir)).with_threads(args.threads as u32);

    if let Some(ref prefix) = args.prefix {
        recorder = recorder.with_prefix(prefix);
    }

    // Get the diff (record with dry-run equivalent - we use the hunks info)
    let diff_result = recorder.diff(&args.channel).await.context("failed to compute diff")?;

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

    let output = PijulDiffOutput {
        repo_id: args.repo_id,
        channel: args.channel,
        channel2: None,
        working_dir: Some(working_dir),
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

/// Diff between two channels.
///
/// Shows changes that are in channel2 but not in channel1.
async fn pijul_diff_channels(
    repo_id_str: &str,
    channel1: &str,
    channel2: &str,
    cache_dir: &PathBuf,
    json: bool,
) -> Result<()> {
    use tracing::info;

    // Parse repo ID
    let repo_id = RepoId::from_hex(repo_id_str).context("invalid repository ID format")?;

    // Check if cache exists
    if !cache_dir.exists() {
        anyhow::bail!("Local cache not found at {}. Run 'pijul sync {}' first.", cache_dir.display(), repo_id_str);
    }

    info!(
        cache_dir = %cache_dir.display(),
        channel1 = %channel1,
        channel2 = %channel2,
        "comparing channels"
    );

    // Create local pristine manager
    let pristine_mgr = PristineManager::new(cache_dir);
    let pristine = pristine_mgr.open(&repo_id).context("failed to open local pristine - run 'pijul sync' first")?;

    // Get changes from both channels and compute the difference
    // This shows changes in channel2 that are not in channel1
    let diff_result = pristine.diff_channels(channel1, channel2).context("failed to compute channel diff")?;

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

    let output = PijulDiffOutput {
        repo_id: repo_id_str.to_string(),
        channel: channel1.to_string(),
        channel2: Some(channel2.to_string()),
        working_dir: None,
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
