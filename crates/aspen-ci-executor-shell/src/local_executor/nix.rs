//! Nix-specific utilities: flag injection, flake rewriting, directory copying.

use std::collections::HashMap;
use std::io;
use std::path::PathBuf;

use tracing::debug;

/// Inject nix flags for offline execution and optionally rewrite flake references.
///
/// If `flake_store_path` is provided, flake references like `.#attr` in the args
/// will be rewritten to use the store path directly (e.g., `/nix/store/xxx#attr`).
/// This avoids Nix trying to copy the workspace to the store, which can fail
/// on read-only overlay filesystems.
pub(crate) fn inject_nix_flags_with_flake_rewrite(
    args: &[String],
    flake_store_path: Option<&PathBuf>,
    job_id: &str,
) -> (String, Vec<String>) {
    let mut nix_args = args.to_vec();

    // Rewrite flake references to use the pre-archived store path
    if let Some(store_path) = flake_store_path {
        for arg in &mut nix_args {
            if arg.starts_with(".#") {
                // .#attr -> /nix/store/xxx#attr
                let attr = arg[2..].to_string();
                let new_arg = format!("{}#{}", store_path.display(), attr);
                debug!(job_id = %job_id, store_path = %store_path.display(), attr = %attr, "rewrote flake reference");
                *arg = new_arg;
            } else if arg == "." {
                // . -> /nix/store/xxx
                let new_arg = store_path.display().to_string();
                debug!(job_id = %job_id, store_path = %store_path.display(), "rewrote bare flake reference");
                *arg = new_arg;
            }
        }
    }

    if !nix_args.is_empty() {
        let mut insert_pos = 1;

        if !nix_args.iter().any(|a| a == "--offline") {
            nix_args.insert(insert_pos, "--offline".to_string());
            insert_pos += 1;
        }

        if !nix_args.iter().any(|a| a.contains("experimental-features")) {
            nix_args.insert(insert_pos, "--extra-experimental-features".to_string());
            insert_pos += 1;
            nix_args.insert(insert_pos, "nix-command flakes".to_string());
            insert_pos += 1;
        }

        if !nix_args.iter().any(|a| a == "--accept-flake-config") {
            nix_args.insert(insert_pos, "--accept-flake-config".to_string());
            insert_pos += 1;
        }

        if !nix_args.iter().any(|a| a == "--no-write-lock-file") {
            nix_args.insert(insert_pos, "--no-write-lock-file".to_string());
            insert_pos += 1;
        }

        // Redirect lock file to /tmp to avoid read-only filesystem errors.
        // Even with --no-write-lock-file, Nix tries to open the lock file for
        // process synchronization (flock), which fails on read-only paths.
        // This redirects the lock file to a writable location.
        if !nix_args.iter().any(|a| a == "--output-lock-file") {
            nix_args.insert(insert_pos, "--output-lock-file".to_string());
            nix_args.insert(insert_pos + 1, "/tmp/flake.lock".to_string());
        }
    }

    ("nix".to_string(), nix_args)
}

/// Inject nix flags for offline execution (without flake rewriting).
#[allow(dead_code)]
pub(crate) fn inject_nix_flags(args: &[String]) -> (String, Vec<String>) {
    inject_nix_flags_with_flake_rewrite(args, None, "")
}

/// Copy contents of a directory to another directory.
pub(crate) async fn copy_directory_contents(src: &std::path::Path, dst: &std::path::Path) -> io::Result<usize> {
    use tokio::fs;

    fs::create_dir_all(dst).await?;

    let mut count = 0;
    let mut entries = fs::read_dir(src).await?;

    while let Some(entry) = entries.next_entry().await? {
        let src_path = entry.path();
        let file_name = entry.file_name();
        let dst_path = dst.join(&file_name);

        let file_type = entry.file_type().await?;

        if file_type.is_dir() {
            count += Box::pin(copy_directory_contents(&src_path, &dst_path)).await?;
        } else if file_type.is_file() {
            fs::copy(&src_path, &dst_path).await?;
            count += 1;
        } else if file_type.is_symlink() {
            let target = fs::read_link(&src_path).await?;
            let _ = fs::remove_file(&dst_path).await;
            #[cfg(unix)]
            {
                tokio::fs::symlink(&target, &dst_path).await?;
            }
            count += 1;
        }
    }

    Ok(count)
}

/// Pre-fetch flake inputs and rewrite flake.lock for offline evaluation.
///
/// Returns the store path of the flake source itself (from archive output `path` field).
/// This can be used to rewrite `.#attr` references to `/nix/store/xxx#attr` to avoid
/// Nix trying to copy the workspace to the store (which fails on read-only overlay).
pub(crate) async fn prefetch_and_rewrite_flake_lock(workspace: &std::path::Path) -> io::Result<Option<PathBuf>> {
    use std::process::Stdio;

    use tokio::process::Command;

    let archive_output = Command::new("nix")
        .args([
            "flake",
            "archive",
            "--json",
            "--no-write-lock-file",
            "--accept-flake-config",
        ])
        .current_dir(workspace)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    if !archive_output.status.success() {
        let stderr = String::from_utf8_lossy(&archive_output.stderr);
        return Err(io::Error::other(format!(
            "nix flake archive failed: {}",
            stderr.chars().take(500).collect::<String>()
        )));
    }

    let stdout = String::from_utf8(archive_output.stdout)
        .map_err(|e| io::Error::other(format!("invalid UTF-8 in archive output: {e}")))?;

    let archive_json: serde_json::Value =
        serde_json::from_str(&stdout).map_err(|e| io::Error::other(format!("failed to parse archive JSON: {e}")))?;

    let mut input_paths = HashMap::new();
    extract_archive_paths(&archive_json, &mut input_paths);

    // Extract the root flake store path from archive output.
    let flake_store_path = archive_json.get("path").and_then(|v| v.as_str()).map(PathBuf::from);

    rewrite_flake_lock_for_offline(workspace, &input_paths)?;

    // Sync to ensure virtiofsd sees the changes
    let _ = Command::new("sync").output().await;

    Ok(flake_store_path)
}

/// Extract input name -> store path mappings from archive JSON output.
fn extract_archive_paths(json: &serde_json::Value, paths: &mut HashMap<String, PathBuf>) {
    if let Some(inputs) = json.get("inputs").and_then(|v| v.as_object()) {
        for (name, value) in inputs {
            if let Some(path) = value.get("path").and_then(|v| v.as_str()) {
                paths.insert(name.clone(), PathBuf::from(path));
            }
            extract_archive_paths(value, paths);
        }
    }
}

/// Rewrite flake.lock to use path: URLs for offline evaluation.
fn rewrite_flake_lock_for_offline(
    workspace: &std::path::Path,
    input_paths: &HashMap<String, PathBuf>,
) -> io::Result<()> {
    let lock_path = workspace.join("flake.lock");
    let lock_content = std::fs::read_to_string(&lock_path)
        .map_err(|e| io::Error::other(format!("failed to read {}: {e}", lock_path.display())))?;
    let mut lock: serde_json::Value = serde_json::from_str(&lock_content)
        .map_err(|e| io::Error::other(format!("failed to parse {}: {e}", lock_path.display())))?;

    if let Some(nodes) = lock.get_mut("nodes").and_then(|v| v.as_object_mut()) {
        for (node_name, node_value) in nodes.iter_mut() {
            if node_name == "root" {
                continue;
            }

            if let Some(store_path) = input_paths.get(node_name) {
                rewrite_locked_node_to_path(node_value, store_path);
            }
        }
    }

    let modified_lock = serde_json::to_string_pretty(&lock)
        .map_err(|e| io::Error::other(format!("failed to serialize {}: {e}", lock_path.display())))?;
    let temp_path = lock_path.with_extension("lock.tmp");
    std::fs::write(&temp_path, &modified_lock)
        .map_err(|e| io::Error::other(format!("failed to write {}: {e}", temp_path.display())))?;
    std::fs::rename(&temp_path, &lock_path).map_err(|e| {
        io::Error::other(format!("failed to rename {} to {}: {e}", temp_path.display(), lock_path.display()))
    })?;

    Ok(())
}

/// Rewrite a single locked node to use a path: URL.
fn rewrite_locked_node_to_path(node: &mut serde_json::Value, store_path: &std::path::Path) {
    if let Some(locked) = node.get_mut("locked").and_then(|v| v.as_object_mut()) {
        locked.insert("type".to_string(), serde_json::json!("path"));
        locked.insert("path".to_string(), serde_json::json!(store_path.display().to_string()));
        locked.remove("owner");
        locked.remove("repo");
        locked.remove("url");
        locked.remove("ref");
    }
}
