//! Nix database dump loading for CI agent VMs.
//!
//! The host generates a database dump after prefetching the build closure.
//! This dump contains metadata for store paths shared via virtiofs - the
//! paths exist in /nix/store but the VM's nix-daemon doesn't know about them.
//! Loading this dump makes nix recognize these paths as valid.

use std::process::Stdio;
use std::time::Instant;

use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

/// Check if a command is nix-related (needs database dump loaded).
///
/// This handles:
/// - Direct nix commands: nix, nix-build, nix-shell, etc.
/// - Full paths: /nix/store/.../bin/nix, /run/current-system/sw/bin/nix
/// - Shell wrappers: Commands that might invoke nix internally
pub(crate) fn is_nix_command(cmd: &str) -> bool {
    // Direct command match
    if matches!(cmd, "nix" | "nix-build" | "nix-shell" | "nix-store" | "nix-env" | "nix-instantiate") {
        return true;
    }

    // Check if command is a path containing nix binary
    if cmd.contains("/nix") && cmd.contains("/bin/nix") {
        return true;
    }

    // Shell commands that might run nix internally should also trigger DB load
    // since they commonly wrap nix builds in CI pipelines
    if matches!(cmd, "sh" | "bash" | "zsh") {
        return true;
    }

    false
}

/// Metadata for the nix database dump, written by the host.
#[derive(Debug, serde::Deserialize)]
struct DbDumpMeta {
    /// Schema version (currently 1)
    #[allow(dead_code)]
    version: u32,
    /// Derivation path that was dumped
    drv_path: String,
    /// Number of store paths in the dump
    path_count: u64,
    /// Size of the dump file in bytes
    dump_size_bytes: u64,
    /// Timestamp when the dump was generated
    #[allow(dead_code)]
    generated_at: String,
}

/// Load nix database dump from the workspace if present.
///
/// This function also reads the metadata file for verification and logging.
pub(crate) async fn load_nix_db_dump(workspace_root: &std::path::Path) {
    let dump_path = workspace_root.join(".nix-db-dump");
    let meta_path = workspace_root.join(".nix-db-dump.meta");

    if !dump_path.exists() {
        info!(dump_path = %dump_path.display(), "no nix database dump found - skipping DB load");
        return;
    }

    let start = Instant::now();

    // Read metadata if available for logging
    let meta: Option<DbDumpMeta> = if meta_path.exists() {
        match tokio::fs::read_to_string(&meta_path).await {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(m) => Some(m),
                Err(e) => {
                    debug!("failed to parse dump metadata: {}", e);
                    None
                }
            },
            Err(e) => {
                debug!("failed to read dump metadata: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Read the dump file size for logging
    let dump_size = match tokio::fs::metadata(&dump_path).await {
        Ok(meta) => meta.len(),
        Err(e) => {
            error!("failed to stat nix database dump: {}", e);
            return;
        }
    };

    // Verify dump size matches metadata if available
    if let Some(ref m) = meta
        && dump_size != m.dump_size_bytes
    {
        warn!(
            expected = m.dump_size_bytes,
            actual = dump_size,
            "dump file size mismatch - file may be corrupted or incomplete"
        );
    }

    info!(
        dump_path = %dump_path.display(),
        dump_size_bytes = dump_size,
        path_count = meta.as_ref().map(|m| m.path_count),
        drv_path = meta.as_ref().map(|m| m.drv_path.as_str()),
        "loading nix database dump"
    );

    // Open the dump file for reading
    let dump_file = match File::open(&dump_path).await {
        Ok(f) => f,
        Err(e) => {
            error!("failed to open nix database dump: {}", e);
            return;
        }
    };

    // Read the entire dump into memory (should be reasonable size)
    let mut dump_contents = Vec::new();
    let mut dump_reader = tokio::io::BufReader::new(dump_file);
    if let Err(e) = dump_reader.read_to_end(&mut dump_contents).await {
        error!("failed to read nix database dump: {}", e);
        return;
    }

    // Load the database using nix-store --load-db
    // This requires piping the dump to stdin
    let mut child = match Command::new("nix-store")
        .arg("--load-db")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            error!("failed to spawn nix-store --load-db: {}", e);
            return;
        }
    };

    // Write dump to stdin
    if let Some(mut stdin) = child.stdin.take()
        && let Err(e) = stdin.write_all(&dump_contents).await
    {
        error!("failed to write to nix-store stdin: {}", e);
        return;
    }
    // stdin is dropped here to close it and signal EOF

    // Wait for completion
    match child.wait().await {
        Ok(status) => {
            let elapsed = start.elapsed();
            if status.success() {
                info!(
                    dump_size_bytes = dump_size,
                    path_count = meta.as_ref().map(|m| m.path_count),
                    elapsed_ms = elapsed.as_millis(),
                    "nix database dump loaded successfully - store paths should now be recognized"
                );
            } else {
                error!(
                    exit_code = status.code(),
                    elapsed_ms = elapsed.as_millis(),
                    "nix-store --load-db failed - build will likely rebuild from scratch"
                );
            }
        }
        Err(e) => {
            error!("failed to wait for nix-store --load-db: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_nix_command_direct_commands() {
        // Direct nix commands
        assert!(is_nix_command("nix"));
        assert!(is_nix_command("nix-build"));
        assert!(is_nix_command("nix-shell"));
        assert!(is_nix_command("nix-store"));
        assert!(is_nix_command("nix-env"));
        assert!(is_nix_command("nix-instantiate"));
    }

    #[test]
    fn test_is_nix_command_paths() {
        // Full paths to nix binaries
        assert!(is_nix_command("/nix/store/abc123/bin/nix"));
        assert!(is_nix_command("/run/current-system/sw/bin/nix-build"));
        assert!(is_nix_command("/nix/var/nix/profiles/default/bin/nix"));
    }

    #[test]
    fn test_is_nix_command_shell_wrappers() {
        // Shell commands that might invoke nix
        assert!(is_nix_command("sh"));
        assert!(is_nix_command("bash"));
        assert!(is_nix_command("zsh"));
    }

    #[test]
    fn test_is_nix_command_non_nix() {
        // Commands that should not trigger DB load
        assert!(!is_nix_command("cargo"));
        assert!(!is_nix_command("rustc"));
        assert!(!is_nix_command("make"));
        assert!(!is_nix_command("gcc"));
        assert!(!is_nix_command("ls"));
        assert!(!is_nix_command("/usr/bin/python"));
    }
}
