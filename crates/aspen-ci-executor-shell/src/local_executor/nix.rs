//! Nix-specific utilities: flag injection, flake rewriting, directory copying.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::process::Command;

use tracing::debug;

/// Flake inputs that must be resolved from host-prefetched store paths in VM-CI.
///
/// Rewriting every input to `/nix/store/...` path nodes forces guest Nix to
/// materialize huge source trees (notably nixpkgs) inside the microVM store,
/// which re-enters the virtiofs/open-file boundary. Keep public/cacheable inputs
/// on their normal locked fetchers and only pin private/project-local inputs
/// whose upstream cannot be fetched directly by the guest.
const VMCI_PREFETCHED_PATH_REWRITE_INPUTS: &[&str] = &["tigerstyle", "ucan-src"];

/// Inject nix flags for offline execution and optionally rewrite flake references.
///
/// If `flake_store_path` is provided, flake references like `.#attr` in the args
/// will be rewritten to use the store path directly (e.g., `/nix/store/xxx#attr`).
/// This avoids Nix trying to copy the workspace to the store, which can fail
/// on read-only overlay filesystems.
pub(crate) const VMCI_LOCAL_STORE_ROOT_ENV: &str = "ASPEN_CI_NIX_LOCAL_STORE_ROOT";

pub(crate) fn vmci_local_store_root_from_env(env: &HashMap<String, String>) -> Option<String> {
    env.get(VMCI_LOCAL_STORE_ROOT_ENV)
        .cloned()
        .or_else(|| std::env::var(VMCI_LOCAL_STORE_ROOT_ENV).ok())
        .map(|root| root.trim().to_string())
        .filter(|root| !root.is_empty())
}

pub(crate) fn inject_nix_flags_with_flake_rewrite(
    args: &[String],
    flake_store_path: Option<&PathBuf>,
    job_id: &str,
    vmci_local_store_root: Option<&str>,
) -> (String, Vec<String>) {
    let mut nix_args = args.to_vec();

    // Rewrite flake references to use the pre-archived store path.
    // Payloads may already expand `.#attr` into `path:/tmp/workspaces/<job>#attr`;
    // rewrite both shapes so guest Nix evaluates the post-rewrite archived flake.
    if let Some(store_path) = flake_store_path {
        for arg in &mut nix_args {
            if let Some(rewritten) = rewrite_flake_arg_to_store_path(arg, store_path) {
                debug!(job_id = %job_id, store_path = %store_path.display(), original = %arg, rewritten = %rewritten, "rewrote flake reference");
                *arg = rewritten;
            }
        }
    }

    if !nix_args.is_empty() {
        let mut insert_pos = 1;

        // Note: --offline is NOT injected. VM CI workers have network access
        // via TAP/bridge, and --offline causes "Truncated tar archive" errors
        // when nix reads flake input tarballs through the virtiofs nix store
        // overlay. Without --offline, nix can fetch inputs from the network
        // as a fallback when cached tarballs are unreadable.

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

        // VM CI rewrites prefetched flake inputs to local /nix/store path nodes.
        // Nix considers hashless path nodes "unlocked"; accept them because the
        // paths were produced by the trusted host prefetch step and avoid stale
        // narHash validation against virtiofs-visible store paths.
        if !nix_args.iter().any(|a| a == "--allow-dirty-locks") {
            nix_args.insert(insert_pos, "--allow-dirty-locks".to_string());
            insert_pos += 1;
        }

        insert_pos = inject_vmci_local_store_flags(&mut nix_args, insert_pos, vmci_local_store_root);

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

fn rewrite_flake_arg_to_store_path(arg: &str, store_path: &std::path::Path) -> Option<String> {
    if arg == "." || (arg.starts_with("path:") && !arg.contains('#')) {
        return Some(store_path.display().to_string());
    }

    if let Some(attr) = arg.strip_prefix(".#") {
        return Some(format!("{}#{attr}", store_path.display()));
    }

    let attr = arg.strip_prefix("path:")?.split_once('#')?.1;
    Some(format!("{}#{attr}", store_path.display()))
}

/// Inject nix flags for offline execution (without flake rewriting).
#[allow(dead_code)]
pub(crate) fn inject_nix_flags(args: &[String]) -> (String, Vec<String>) {
    inject_nix_flags_with_flake_rewrite(args, None, "", None)
}

pub(crate) fn inject_vmci_local_store_flags(
    args: &mut Vec<String>,
    insert_pos: usize,
    store_root: Option<&str>,
) -> usize {
    let Some(store_root) = store_root.map(str::trim).filter(|root| !root.is_empty()) else {
        return insert_pos;
    };

    if args.iter().any(|arg| arg == "--store") {
        return insert_pos;
    }

    let store_uri = format!("local?root={store_root}");
    let build_dir = format!("{}/.build-dir", store_root.trim_end_matches('/'));
    let local_store_flags = [
        "--store".to_string(),
        store_uri,
        "--option".to_string(),
        "build-dir".to_string(),
        build_dir,
        "--option".to_string(),
        "min-free".to_string(),
        "0".to_string(),
        "--option".to_string(),
        "max-free".to_string(),
        "0".to_string(),
    ];
    let inserted = local_store_flags.len();
    args.splice(insert_pos..insert_pos, local_store_flags);
    insert_pos + inserted
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
    use tokio::process::Command;

    let archive_json = archive_flake_json(workspace).await?;

    let mut input_paths = HashMap::new();
    extract_archive_paths(&archive_json, &mut input_paths);

    rewrite_flake_inputs_to_prefetched_paths(workspace, &input_paths)?;
    rewrite_flake_lock_for_offline(workspace, &input_paths)?;

    // Sync to ensure virtiofsd sees the changes before the post-rewrite archive.
    let _ = Command::new("sync").output().await;

    // The first archive's root store path still contains the original flake.nix,
    // including private git+ssh input URLs. Archive again after rewriting so
    // `.#attr` arguments are redirected to a store source that also points those
    // inputs at prefetched local paths.
    let rewritten_archive_json = archive_flake_json(workspace).await?;
    Ok(rewritten_archive_json.get("path").and_then(|v| v.as_str()).map(PathBuf::from))
}

async fn archive_flake_json(workspace: &std::path::Path) -> io::Result<serde_json::Value> {
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

    serde_json::from_str(&stdout).map_err(|e| io::Error::other(format!("failed to parse archive JSON: {e}")))
}

/// Extract input name -> store path mappings from archive JSON output.
fn extract_archive_paths(json: &serde_json::Value, paths: &mut HashMap<String, PrefetchedInputPath>) {
    if let Some(inputs) = json.get("inputs").and_then(|v| v.as_object()) {
        for (name, value) in inputs {
            if let Some(path) = value.get("path").and_then(|v| v.as_str()) {
                paths.insert(name.clone(), PrefetchedInputPath {
                    path: PathBuf::from(path),
                    nar_hash: None,
                });
            }
            extract_archive_paths(value, paths);
        }
    }
}

#[derive(Clone, Debug)]
struct PrefetchedInputPath {
    path: PathBuf,
    nar_hash: Option<String>,
}

impl PrefetchedInputPath {
    fn parse(value: &str) -> Self {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(value) {
            if let Some(path) = json.get("path").and_then(|v| v.as_str()) {
                return Self {
                    path: PathBuf::from(path),
                    nar_hash: json.get("narHash").and_then(|v| v.as_str()).map(str::to_string),
                };
            }
        }

        Self {
            path: PathBuf::from(value),
            nar_hash: None,
        }
    }
}

/// Rewrite flake.lock to use host-prefetched store paths.
pub fn rewrite_flake_lock_with_store_paths(
    workspace: &std::path::Path,
    input_paths: &BTreeMap<String, String>,
) -> io::Result<()> {
    let paths = input_paths
        .iter()
        .map(|(name, path)| (name.clone(), PrefetchedInputPath::parse(path)))
        .collect::<HashMap<_, _>>();
    rewrite_flake_inputs_to_prefetched_paths(workspace, &paths)?;
    rewrite_flake_lock_for_offline(workspace, &paths)
}

fn rewrite_flake_inputs_to_prefetched_paths(
    workspace: &std::path::Path,
    input_paths: &HashMap<String, PrefetchedInputPath>,
) -> io::Result<()> {
    let flake_path = workspace.join("flake.nix");
    let flake_content = std::fs::read_to_string(&flake_path)
        .map_err(|e| io::Error::other(format!("failed to read {}: {e}", flake_path.display())))?;
    let mut lines = Vec::new();
    let mut iter = flake_content.lines().peekable();
    let mut modified = false;

    while let Some(line) = iter.next() {
        let Some((input_name, indent)) = parse_rewriteable_input_block_start(line) else {
            lines.push(line.to_string());
            continue;
        };

        if !VMCI_PREFETCHED_PATH_REWRITE_INPUTS.contains(&input_name) {
            lines.push(line.to_string());
            continue;
        }
        let Some(input_path) = input_paths.get(input_name) else {
            lines.push(line.to_string());
            continue;
        };

        let mut block = vec![line.to_string()];
        let mut has_flake_false = false;
        for block_line in iter.by_ref() {
            has_flake_false |= block_line.trim() == "flake = false;";
            block.push(block_line.to_string());
            if block_line.trim() == "};" {
                break;
            }
        }

        if block.last().is_none_or(|last| last.trim() != "};") {
            lines.extend(block);
            continue;
        }

        lines.push(format!("{indent}{input_name} = {{"));
        lines.push(format!("{indent}  url = \"path:{}\";", input_path.path.display()));
        if has_flake_false {
            lines.push(format!("{indent}  flake = false;"));
        }
        lines.push(format!("{indent}}};"));
        modified = true;
    }

    if modified {
        let mut rewritten = lines.join("\n");
        rewritten.push('\n');
        std::fs::write(&flake_path, rewritten)
            .map_err(|e| io::Error::other(format!("failed to write {}: {e}", flake_path.display())))?;
    }

    Ok(())
}

fn parse_rewriteable_input_block_start(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim_start();
    let indent_len = line.len() - trimmed.len();
    let (name, suffix) = trimmed.split_once(" = {")?;
    if !suffix.trim().is_empty() {
        return None;
    }
    Some((name, &line[..indent_len]))
}

/// Rewrite flake.lock to use path: URLs for offline evaluation.
fn rewrite_flake_lock_for_offline(
    workspace: &std::path::Path,
    input_paths: &HashMap<String, PrefetchedInputPath>,
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

            if !VMCI_PREFETCHED_PATH_REWRITE_INPUTS.contains(&node_name.as_str()) {
                continue;
            }

            if let Some(input_path) = input_paths.get(node_name) {
                rewrite_locked_node_to_path(node_value, input_path)?;
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

/// Rewrite a single locked node to use a local store path.
fn rewrite_locked_node_to_path(node: &mut serde_json::Value, input_path: &PrefetchedInputPath) -> io::Result<()> {
    let nar_hash = match &input_path.nar_hash {
        Some(nar_hash) => nar_hash.clone(),
        None => compute_path_nar_hash(&input_path.path)?,
    };
    rewrite_locked_node_to_path_with_nar_hash(node, &input_path.path, &nar_hash);
    Ok(())
}

fn compute_path_nar_hash(store_path: &std::path::Path) -> io::Result<String> {
    let output = Command::new("nix")
        .args(["hash", "path", "--type", "sha256", "--sri"])
        .arg(store_path)
        .output()
        .map_err(|e| io::Error::other(format!("failed to compute narHash for {}: {e}", store_path.display())))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::other(format!(
            "failed to compute narHash for {}: {}",
            store_path.display(),
            stderr.trim()
        )));
    }

    let hash = String::from_utf8(output.stdout)
        .map_err(|e| io::Error::other(format!("nix hash path output was not UTF-8: {e}")))?
        .trim()
        .to_string();
    if hash.is_empty() {
        return Err(io::Error::other(format!("nix hash path produced an empty narHash for {}", store_path.display())));
    }
    Ok(hash)
}

fn rewrite_locked_node_to_path_with_nar_hash(
    node: &mut serde_json::Value,
    store_path: &std::path::Path,
    nar_hash: &str,
) {
    if let Some(locked) = node.get_mut("locked").and_then(|v| v.as_object_mut()) {
        rewrite_lock_metadata_to_path(locked, store_path, Some(nar_hash));
    }

    if let Some(original) = node.get_mut("original").and_then(|v| v.as_object_mut()) {
        rewrite_lock_metadata_to_path(original, store_path, None);
    }
}

fn rewrite_lock_metadata_to_path(
    metadata: &mut serde_json::Map<String, serde_json::Value>,
    store_path: &std::path::Path,
    nar_hash: Option<&str>,
) {
    metadata.clear();
    metadata.insert("type".to_string(), serde_json::json!("path"));
    metadata.insert("path".to_string(), serde_json::json!(store_path.display().to_string()));
    if let Some(nar_hash) = nar_hash {
        metadata.insert("narHash".to_string(), serde_json::json!(nar_hash));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flake_arg_rewrite_handles_workspace_path_refs() {
        let store_path = std::path::Path::new("/nix/store/post-rewrite-source");

        assert_eq!(
            rewrite_flake_arg_to_store_path(".#checks.x86_64-linux.build-cli", store_path),
            Some("/nix/store/post-rewrite-source#checks.x86_64-linux.build-cli".to_string())
        );
        assert_eq!(
            rewrite_flake_arg_to_store_path("path:/tmp/workspaces/job-123#checks.x86_64-linux.build-cli", store_path),
            Some("/nix/store/post-rewrite-source#checks.x86_64-linux.build-cli".to_string())
        );
        assert_eq!(
            rewrite_flake_arg_to_store_path("path:/tmp/workspaces/job-123", store_path),
            Some("/nix/store/post-rewrite-source".to_string())
        );
        assert_eq!(rewrite_flake_arg_to_store_path("--print-out-paths", store_path), None);
    }

    #[test]
    fn rewrite_flake_lock_for_offline_only_rewrites_selected_inputs() {
        let workspace = tempfile::tempdir().expect("workspace");
        std::fs::write(
            workspace.path().join("flake.nix"),
            r#"{
  inputs = {
    nixpkgs = { url = "github:NixOS/nixpkgs"; };
    tigerstyle = {
      url = "github:onixresearch/octet";
    };
    ucan-src = {
      url = "git+ssh://git@github.com/OnixResearch/ucan.git?rev=ad61b53e89fa45f9bf7d313ce14c45de645bf53d";
      flake = false;
    };
  };
  outputs = { self, ... }: {};
}
"#,
        )
        .expect("write flake");
        std::fs::write(
            workspace.path().join("flake.lock"),
            serde_json::to_string_pretty(&serde_json::json!({
                "nodes": {
                    "root": { "inputs": { "nixpkgs": "nixpkgs", "tigerstyle": "tigerstyle", "ucan-src": "ucan-src" } },
                    "nixpkgs": {
                        "locked": {
                            "type": "github",
                            "owner": "NixOS",
                            "repo": "nixpkgs",
                            "rev": "e145f2bc80a57cc069583aa952dc22def6213aa4",
                            "narHash": "sha256-public="
                        },
                        "original": { "type": "github", "owner": "NixOS", "repo": "nixpkgs" }
                    },
                    "tigerstyle": {
                        "locked": {
                            "type": "github",
                            "owner": "onixresearch",
                            "repo": "octet",
                            "rev": "7fb94b717496c53f61048161d5cc34b8cfec0b40",
                            "narHash": "sha256-private="
                        },
                        "original": { "type": "github", "owner": "onixresearch", "repo": "octet" }
                    },
                    "ucan-src": {
                        "flake": false,
                        "locked": {
                            "type": "git",
                            "url": "ssh://git@github.com/OnixResearch/ucan.git",
                            "rev": "ad61b53e89fa45f9bf7d313ce14c45de645bf53d",
                            "narHash": "sha256-ucan="
                        },
                        "original": {
                            "type": "git",
                            "url": "ssh://git@github.com/OnixResearch/ucan.git",
                            "rev": "ad61b53e89fa45f9bf7d313ce14c45de645bf53d"
                        }
                    }
                },
                "root": "root",
                "version": 7
            }))
            .expect("serialize lock"),
        )
        .expect("write lock");

        let mut input_paths = HashMap::new();
        input_paths.insert("nixpkgs".to_string(), PrefetchedInputPath {
            path: PathBuf::from("/nix/store/nixpkgs-source"),
            nar_hash: Some("sha256-nixpkgs-prefetched=".to_string()),
        });
        input_paths.insert("tigerstyle".to_string(), PrefetchedInputPath {
            path: PathBuf::from("/nix/store/octet-source"),
            nar_hash: Some("sha256-octet-prefetched=".to_string()),
        });
        input_paths.insert("ucan-src".to_string(), PrefetchedInputPath {
            path: PathBuf::from("/nix/store/ucan-source"),
            nar_hash: Some("sha256-ucan-prefetched=".to_string()),
        });

        rewrite_flake_inputs_to_prefetched_paths(workspace.path(), &input_paths).expect("rewrite flake inputs");
        rewrite_flake_lock_for_offline(workspace.path(), &input_paths).expect("rewrite lock");

        let lock: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(workspace.path().join("flake.lock")).expect("read lock"))
                .expect("parse lock");
        let nixpkgs_locked = &lock["nodes"]["nixpkgs"]["locked"];
        assert_eq!(nixpkgs_locked["type"], serde_json::json!("github"));
        assert_eq!(nixpkgs_locked["owner"], serde_json::json!("NixOS"));
        assert!(nixpkgs_locked.get("path").is_none());

        let tigerstyle_locked = &lock["nodes"]["tigerstyle"]["locked"];
        assert_eq!(tigerstyle_locked["type"], serde_json::json!("path"));
        assert_eq!(tigerstyle_locked["path"], serde_json::json!("/nix/store/octet-source"));
        assert_eq!(tigerstyle_locked["narHash"], serde_json::json!("sha256-octet-prefetched="));

        let ucan_locked = &lock["nodes"]["ucan-src"]["locked"];
        assert_eq!(ucan_locked["type"], serde_json::json!("path"));
        assert_eq!(ucan_locked["path"], serde_json::json!("/nix/store/ucan-source"));
        assert_eq!(ucan_locked["narHash"], serde_json::json!("sha256-ucan-prefetched="));
        assert_eq!(lock["nodes"]["ucan-src"]["original"]["path"], serde_json::json!("/nix/store/ucan-source"));
        assert_eq!(lock["nodes"]["ucan-src"]["original"]["type"], serde_json::json!("path"));
        assert!(lock["nodes"]["ucan-src"]["original"].get("url").is_none());

        let rewritten_flake = std::fs::read_to_string(workspace.path().join("flake.nix")).expect("read flake");
        assert!(rewritten_flake.contains("url = \"path:/nix/store/ucan-source\";"));
        assert!(rewritten_flake.contains("url = \"path:/nix/store/octet-source\";"));
        assert!(rewritten_flake.contains("flake = false;"));
        assert!(rewritten_flake.contains("url = \"github:NixOS/nixpkgs\";"));
    }

    #[test]
    fn rewrite_locked_node_to_path_strips_locked_metadata_but_preserves_original() {
        let mut node = serde_json::json!({
            "locked": {
                "type": "github",
                "owner": "NixOS",
                "repo": "nixpkgs",
                "ref": "nixos-unstable",
                "rev": "e145f2bc80a57cc069583aa952dc22def6213aa4",
                "lastModified": 1776631944,
                "narHash": "sha256-xGWfYN+KqQ8QVfU7swYQyjbutrN6atNZWsWNfPe+8AE="
            },
            "original": {
                "type": "github",
                "owner": "NixOS",
                "repo": "nixpkgs",
                "ref": "nixos-unstable"
            }
        });

        rewrite_locked_node_to_path_with_nar_hash(
            &mut node,
            std::path::Path::new("/nix/store/i2gsp87gqp16whm9mw0ybk9n84zir01x-source"),
            "sha256-actualNarHash=",
        );

        let locked = node.get("locked").and_then(|value| value.as_object()).unwrap();
        assert_eq!(locked.get("type"), Some(&serde_json::json!("path")));
        assert_eq!(locked.get("path"), Some(&serde_json::json!("/nix/store/i2gsp87gqp16whm9mw0ybk9n84zir01x-source")));
        assert_eq!(locked.get("narHash"), Some(&serde_json::json!("sha256-actualNarHash=")));
        assert!(locked.get("rev").is_none());
        assert!(locked.get("lastModified").is_none());
        assert!(locked.get("owner").is_none());
        assert!(locked.get("repo").is_none());
        assert!(locked.get("ref").is_none());
        assert_eq!(locked.len(), 3);

        let original = node.get("original").and_then(|value| value.as_object()).unwrap();
        assert_eq!(original.get("type"), Some(&serde_json::json!("path")));
        assert_eq!(
            original.get("path"),
            Some(&serde_json::json!("/nix/store/i2gsp87gqp16whm9mw0ybk9n84zir01x-source"))
        );
        assert!(original.get("owner").is_none());
        assert!(original.get("repo").is_none());
        assert!(original.get("ref").is_none());
        assert!(original.get("narHash").is_none());
        assert_eq!(original.len(), 2);
    }
}
