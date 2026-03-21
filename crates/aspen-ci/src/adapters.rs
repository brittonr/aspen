//! Integration adapters for connecting CI TriggerService to Forge and PipelineOrchestrator.
//!
//! This module provides concrete implementations of the `ConfigFetcher` and `PipelineStarter`
//! traits, enabling automatic CI triggering when Forge refs are updated.
//!
//! # Architecture
//!
//! ```text
//! ForgeGossipService ──► CiTriggerHandler ──► TriggerService
//!                                                   │
//!                           ┌───────────────────────┼───────────────────────┐
//!                           │                       │                       │
//!                           ▼                       ▼                       ▼
//!                   ForgeConfigFetcher    OrchestratorPipelineStarter   watch_repo()
//!                           │                       │
//!                           ▼                       ▼
//!                     ForgeNode.git           PipelineOrchestrator
//!                   (fetch .aspen/ci.ncl)      (execute pipeline)
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use aspen_blob::prelude::*;
#[cfg(feature = "shell-executor")]
use aspen_ci_executor_shell::create_source_archive;
use aspen_core::KeyValueStore;
use aspen_forge::ForgeNode;
use aspen_forge::identity::RepoId;
use async_trait::async_trait;
use snafu::ResultExt;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::error::CiError;
use crate::error::LoadBlobFailedSnafu;
use crate::error::LoadTreeFailedSnafu;
use crate::error::Result;
use crate::orchestrator::PipelineContext;
use crate::orchestrator::PipelineOrchestrator;
use crate::orchestrator::PipelineStatus;
use crate::trigger::ConfigFetcher;
use crate::trigger::PipelineStarter;
use crate::trigger::TriggerEvent;

// Tiger Style: Bounded resource limits
/// Maximum CI config file size (1 MB).
const MAX_CONFIG_FILE_SIZE: usize = 1024 * 1024;

/// Adapter that fetches CI configuration from Forge git objects.
///
/// Implements `ConfigFetcher` by walking the git tree at a specific commit
/// to find and read the `.aspen/ci.ncl` configuration file.
///
/// # Example
///
/// ```ignore
/// let fetcher = ForgeConfigFetcher::new(forge_node.clone());
/// let config = fetcher.fetch_config(&repo_id, &commit_hash, ".aspen/ci.ncl").await?;
/// ```
pub struct ForgeConfigFetcher<B: BlobStore, K: KeyValueStore + ?Sized> {
    forge: Arc<ForgeNode<B, K>>,
}

impl<B: BlobStore, K: KeyValueStore + ?Sized> ForgeConfigFetcher<B, K> {
    /// Create a new ForgeConfigFetcher.
    ///
    /// # Arguments
    ///
    /// * `forge` - Reference to the ForgeNode for accessing git objects
    pub fn new(forge: Arc<ForgeNode<B, K>>) -> Self {
        Self { forge }
    }

    /// Walk a tree to find a file at the given path.
    ///
    /// Supports nested paths like "dir/subdir/file.ncl".
    async fn walk_tree_for_file(&self, tree_hash: blake3::Hash, path_parts: &[&str]) -> Result<Option<Vec<u8>>> {
        if path_parts.is_empty() {
            return Ok(None);
        }

        let tree = self.forge.git.get_tree(&tree_hash).await.context(LoadTreeFailedSnafu {
            tree_hash: tree_hash.to_hex().to_string(),
        })?;

        let target_name = path_parts[0];
        let remaining = &path_parts[1..];

        for entry in &tree.entries {
            if entry.name == target_name {
                let entry_hash = blake3::Hash::from_bytes(entry.hash);

                if remaining.is_empty() {
                    // This is the file we're looking for
                    if entry.mode == 0o100644 || entry.mode == 0o100755 {
                        // Regular file
                        let content = self.forge.git.get_blob(&entry_hash).await.context(LoadBlobFailedSnafu {
                            blob_hash: entry_hash.to_hex().to_string(),
                        })?;

                        if content.len() > MAX_CONFIG_FILE_SIZE {
                            return Err(CiError::InvalidConfig {
                                reason: format!(
                                    "Config file too large: {} bytes (max {})",
                                    content.len(),
                                    MAX_CONFIG_FILE_SIZE
                                ),
                            });
                        }

                        return Ok(Some(content));
                    }
                } else if entry.mode == 0o040000 {
                    // Directory - recurse
                    return Box::pin(self.walk_tree_for_file(entry_hash, remaining)).await;
                }
            }
        }

        Ok(None)
    }
}

#[async_trait]
impl<B: BlobStore + 'static, K: KeyValueStore + ?Sized + 'static> ConfigFetcher for ForgeConfigFetcher<B, K> {
    async fn fetch_config(
        &self,
        repo_id: &RepoId,
        commit_hash: &[u8; 32],
        config_path: &str,
    ) -> Result<Option<String>> {
        let blake3_hash = blake3::Hash::from_bytes(*commit_hash);

        debug!(
            repo_id = %repo_id.to_hex(),
            commit = %hex::encode(commit_hash),
            path = config_path,
            "Fetching CI config from commit"
        );

        // Get the commit object
        let commit = match self.forge.git.get_commit(&blake3_hash).await {
            Ok(c) => c,
            Err(e) => {
                warn!(
                    repo_id = %repo_id.to_hex(),
                    commit = %hex::encode(commit_hash),
                    error = %e,
                    "Failed to load commit for CI config"
                );
                return Ok(None);
            }
        };

        // Get the tree from the commit
        let tree_hash = blake3::Hash::from_bytes(commit.tree);

        // Split path into parts for tree traversal
        let path_parts: Vec<&str> = config_path.split('/').filter(|s| !s.is_empty()).collect();

        // Walk the tree to find the config file
        let content_bytes = match self.walk_tree_for_file(tree_hash, &path_parts).await? {
            Some(bytes) => bytes,
            None => {
                debug!(
                    repo_id = %repo_id.to_hex(),
                    path = config_path,
                    "CI config file not found in commit"
                );
                return Ok(None);
            }
        };

        // Convert to string
        let content = String::from_utf8(content_bytes).map_err(|e| CiError::InvalidConfig {
            reason: format!("CI config is not valid UTF-8: {}", e),
        })?;

        info!(
            repo_id = %repo_id.to_hex(),
            path = config_path,
            size = content.len(),
            "Loaded CI config from commit"
        );

        Ok(Some(content))
    }

    async fn list_changed_paths(
        &self,
        repo_id: &RepoId,
        old_hash: &[u8; 32],
        new_hash: &[u8; 32],
    ) -> Result<Option<Vec<String>>> {
        let old_commit_hash = blake3::Hash::from_bytes(*old_hash);
        let new_commit_hash = blake3::Hash::from_bytes(*new_hash);

        // Get both commit objects
        let old_commit = match self.forge.git.get_commit(&old_commit_hash).await {
            Ok(c) => c,
            Err(e) => {
                debug!(
                    repo_id = %repo_id.to_hex(),
                    commit = %hex::encode(old_hash),
                    error = %e,
                    "Failed to load old commit for diff, skipping path filter"
                );
                return Ok(None);
            }
        };

        let new_commit = match self.forge.git.get_commit(&new_commit_hash).await {
            Ok(c) => c,
            Err(e) => {
                debug!(
                    repo_id = %repo_id.to_hex(),
                    commit = %hex::encode(new_hash),
                    error = %e,
                    "Failed to load new commit for diff, skipping path filter"
                );
                return Ok(None);
            }
        };

        let old_tree = blake3::Hash::from_bytes(old_commit.tree);
        let new_tree = blake3::Hash::from_bytes(new_commit.tree);

        // Diff the two trees
        let mut changed: Vec<String> = Vec::new();
        let max_paths: u32 = 10_000;
        self.diff_trees(old_tree, new_tree, String::new(), &mut changed, max_paths).await?;

        debug!(
            repo_id = %repo_id.to_hex(),
            changed_count = changed.len(),
            "Diffed commit trees for path filter"
        );

        Ok(Some(changed))
    }
}

impl<B: BlobStore, K: KeyValueStore + ?Sized> ForgeConfigFetcher<B, K> {
    /// Recursively diff two trees and collect changed file paths.
    ///
    /// Entries are compared by name and hash. If hashes differ, the entry is
    /// considered changed. Entries present in only one tree are additions/deletions.
    ///
    /// Bounded by `max_paths` to prevent unbounded growth on large diffs.
    async fn diff_trees(
        &self,
        old_tree: blake3::Hash,
        new_tree: blake3::Hash,
        prefix: String,
        changed: &mut Vec<String>,
        max_paths: u32,
    ) -> Result<()> {
        if changed.len() as u32 >= max_paths {
            return Ok(());
        }

        // If trees are identical, no changes
        if old_tree == new_tree {
            return Ok(());
        }

        let old_entries = match self.forge.git.get_tree(&old_tree).await {
            Ok(t) => t.entries,
            Err(_) => vec![],
        };
        let new_entries = match self.forge.git.get_tree(&new_tree).await {
            Ok(t) => t.entries,
            Err(_) => vec![],
        };

        // Build maps by name for O(n) comparison
        let old_map: std::collections::HashMap<&str, &aspen_forge::git::TreeEntry> =
            old_entries.iter().map(|e| (e.name.as_str(), e)).collect();
        let new_map: std::collections::HashMap<&str, &aspen_forge::git::TreeEntry> =
            new_entries.iter().map(|e| (e.name.as_str(), e)).collect();

        // Check entries in new tree
        for (name, new_entry) in &new_map {
            if changed.len() as u32 >= max_paths {
                return Ok(());
            }

            let path = if prefix.is_empty() {
                (*name).to_string()
            } else {
                format!("{prefix}/{name}")
            };

            match old_map.get(name) {
                Some(old_entry) => {
                    if old_entry.hash != new_entry.hash {
                        if new_entry.mode == 0o040000 && old_entry.mode == 0o040000 {
                            // Both are trees — recurse
                            Box::pin(self.diff_trees(
                                blake3::Hash::from_bytes(old_entry.hash),
                                blake3::Hash::from_bytes(new_entry.hash),
                                path,
                                changed,
                                max_paths,
                            ))
                            .await?;
                        } else {
                            // File content changed or mode changed
                            changed.push(path);
                        }
                    }
                }
                None => {
                    // New entry (addition)
                    if new_entry.mode == 0o040000 {
                        // New directory — collect all files
                        Box::pin(self.collect_tree_paths(
                            blake3::Hash::from_bytes(new_entry.hash),
                            path,
                            changed,
                            max_paths,
                        ))
                        .await?;
                    } else {
                        changed.push(path);
                    }
                }
            }
        }

        // Check for deletions (in old but not in new)
        for (name, old_entry) in &old_map {
            if changed.len() as u32 >= max_paths {
                return Ok(());
            }

            if !new_map.contains_key(name) {
                let path = if prefix.is_empty() {
                    (*name).to_string()
                } else {
                    format!("{prefix}/{name}")
                };

                if old_entry.mode == 0o040000 {
                    Box::pin(self.collect_tree_paths(
                        blake3::Hash::from_bytes(old_entry.hash),
                        path,
                        changed,
                        max_paths,
                    ))
                    .await?;
                } else {
                    changed.push(path);
                }
            }
        }

        Ok(())
    }

    /// Collect all file paths under a tree (for additions/deletions of directories).
    async fn collect_tree_paths(
        &self,
        tree_hash: blake3::Hash,
        prefix: String,
        paths: &mut Vec<String>,
        max_paths: u32,
    ) -> Result<()> {
        if paths.len() as u32 >= max_paths {
            return Ok(());
        }

        let tree = match self.forge.git.get_tree(&tree_hash).await {
            Ok(t) => t,
            Err(_) => return Ok(()),
        };

        for entry in &tree.entries {
            if paths.len() as u32 >= max_paths {
                return Ok(());
            }

            let path = if prefix.is_empty() {
                entry.name.clone()
            } else {
                format!("{prefix}/{}", entry.name)
            };

            if entry.mode == 0o040000 {
                Box::pin(self.collect_tree_paths(blake3::Hash::from_bytes(entry.hash), path, paths, max_paths)).await?;
            } else {
                paths.push(path);
            }
        }

        Ok(())
    }
}

/// Adapter that starts pipelines via the PipelineOrchestrator.
///
/// Implements `PipelineStarter` by converting `TriggerEvent` into a
/// `PipelineContext` and calling the orchestrator's execute method.
/// Also handles repository checkout before pipeline execution.
///
/// # Example
///
/// ```ignore
/// let starter = OrchestratorPipelineStarter::new(orchestrator.clone(), forge.clone());
/// let run_id = starter.start_pipeline(trigger_event).await?;
/// ```
pub struct OrchestratorPipelineStarter<B: BlobStore, K: KeyValueStore + ?Sized> {
    orchestrator: Arc<PipelineOrchestrator<K>>,
    forge: Arc<ForgeNode<B, K>>,
}

impl<B: BlobStore, K: KeyValueStore + ?Sized> OrchestratorPipelineStarter<B, K> {
    /// Create a new OrchestratorPipelineStarter.
    ///
    /// # Arguments
    ///
    /// * `orchestrator` - Reference to the PipelineOrchestrator
    /// * `forge` - Reference to ForgeNode for repository checkout
    pub fn new(orchestrator: Arc<PipelineOrchestrator<K>>, forge: Arc<ForgeNode<B, K>>) -> Self {
        Self { orchestrator, forge }
    }
}

#[async_trait]
impl<B: BlobStore + 'static, K: KeyValueStore + ?Sized + 'static> PipelineStarter
    for OrchestratorPipelineStarter<B, K>
{
    async fn start_pipeline(&self, event: TriggerEvent) -> Result<String> {
        info!(
            repo_id = %event.repo_id.to_hex(),
            ref_name = %event.ref_name,
            pipeline = %event.config.name,
            "Starting CI pipeline from trigger"
        );

        let context = self.start_pipeline_build_initial_context(&event);
        let run = self.orchestrator.create_early_run(event.config.name.clone(), context).await?;
        let run_id = run.id.clone();

        info!(run_id = %run_id, repo_id = %event.repo_id.to_hex(), "Pipeline run created and persisted");

        self.orchestrator.update_run_status(&run_id, PipelineStatus::CheckingOut, None).await?;
        let checkout_dir = crate::checkout::checkout_dir_for_run(&run_id);

        info!(
            run_id = %run_id,
            repo_id = %event.repo_id.to_hex(),
            commit = %hex::encode(event.commit_hash),
            checkout_dir = %checkout_dir.display(),
            "Checking out repository for CI"
        );

        self.start_pipeline_perform_checkout(&run_id, &event.commit_hash, &checkout_dir).await?;
        self.start_pipeline_prepare_for_build(&run_id, &event.commit_hash, &checkout_dir).await?;

        // Pre-fetch flake inputs into the host nix store so VMs can use them
        // via the shared virtiofs /nix/.ro-store mount without downloading.
        self.prefetch_flake_inputs(&run_id, &checkout_dir).await;

        let source_hash = self.start_pipeline_create_source_archive(&run_id, &checkout_dir).await;
        let updated_context = self.start_pipeline_build_updated_context(&event, &run_id, &checkout_dir, source_hash);
        self.orchestrator.update_run_context(&run_id, updated_context).await?;

        let run = self.orchestrator.execute_existing_run(&run_id, event.config).await?;

        info!(
            run_id = %run.id,
            repo_id = %event.repo_id.to_hex(),
            ref_name = %event.ref_name,
            checkout_dir = %checkout_dir.display(),
            "Pipeline started successfully with repository checkout"
        );

        Ok(run.id)
    }
}

impl<B: BlobStore + 'static, K: KeyValueStore + ?Sized + 'static> OrchestratorPipelineStarter<B, K> {
    /// Build initial pipeline context before checkout.
    fn start_pipeline_build_initial_context(&self, event: &TriggerEvent) -> PipelineContext {
        let mut env = HashMap::new();
        env.insert("CI_TRIGGERED_BY".to_string(), event.pusher.to_string());
        env.insert(
            "CI_PREVIOUS_COMMIT".to_string(),
            event.old_hash.map(hex::encode).unwrap_or_else(|| "none".to_string()),
        );

        PipelineContext {
            repo_id: event.repo_id,
            commit_hash: event.commit_hash,
            ref_name: event.ref_name.clone(),
            triggered_by: event.pusher.to_string(),
            run_id: String::new(), // Set by orchestrator after run ID generation
            env,
            checkout_dir: None,
            source_hash: None,
        }
    }

    /// Perform repository checkout with error handling and cleanup.
    async fn start_pipeline_perform_checkout(
        &self,
        run_id: &str,
        commit_hash: &[u8; 32],
        checkout_dir: &std::path::Path,
    ) -> Result<()> {
        if let Err(e) = crate::checkout::checkout_repository(&self.forge, commit_hash, checkout_dir).await {
            let error_msg = format!("Repository checkout failed: {}", e);
            warn!(run_id = %run_id, error = %error_msg, "CI checkout failed");

            self.start_pipeline_cleanup_on_failure(run_id, checkout_dir).await;
            self.orchestrator
                .update_run_status(run_id, PipelineStatus::CheckoutFailed, Some(error_msg.clone()))
                .await?;

            return Err(CiError::Checkout { reason: error_msg });
        }
        Ok(())
    }

    /// Prepare checkout directory for CI build.
    async fn start_pipeline_prepare_for_build(
        &self,
        run_id: &str,
        commit_hash: &[u8; 32],
        checkout_dir: &std::path::Path,
    ) -> Result<()> {
        if let Err(e) = crate::checkout::prepare_for_ci_build(checkout_dir, commit_hash).await {
            let error_msg = format!("CI build preparation failed: {}", e);
            warn!(run_id = %run_id, error = %error_msg, "CI build preparation failed");

            self.start_pipeline_cleanup_on_failure(run_id, checkout_dir).await;
            self.orchestrator
                .update_run_status(run_id, PipelineStatus::CheckoutFailed, Some(error_msg.clone()))
                .await?;

            return Err(CiError::Checkout { reason: error_msg });
        }
        Ok(())
    }

    /// Clean up checkout directory on failure (logs errors but does not fail).
    async fn start_pipeline_cleanup_on_failure(&self, run_id: &str, checkout_dir: &std::path::Path) {
        if let Err(cleanup_err) = crate::checkout::cleanup_checkout(checkout_dir).await {
            warn!(
                run_id = %run_id,
                checkout_dir = %checkout_dir.display(),
                error = %cleanup_err,
                "Failed to clean up checkout directory after failure"
            );
        }
    }

    /// Pre-fetch flake inputs into the host nix store.
    ///
    /// Runs `nix flake archive` on the checkout directory to fetch all flake
    /// inputs (nixpkgs, crane, rust-overlay, etc.) into `/nix/store`. These
    /// store paths are then available to VMs via the virtiofs `/nix/.ro-store`
    /// mount, avoiding multi-GB downloads inside VMs.
    async fn prefetch_flake_inputs(&self, run_id: &str, checkout_dir: &std::path::Path) {
        let flake_nix = checkout_dir.join("flake.nix");
        if !flake_nix.exists() {
            debug!(run_id = %run_id, "No flake.nix in checkout, skipping flake input prefetch");
            return;
        }

        info!(run_id = %run_id, checkout_dir = %checkout_dir.display(), "Pre-fetching flake inputs for VM builds");

        // Tiger Style: bounded timeout — nix flake archive can hang indefinitely
        // when the network is unavailable (e.g., inside NixOS VM tests).
        let prefetch_timeout = std::time::Duration::from_secs(120);
        let child = match tokio::process::Command::new("nix")
            .args(["flake", "archive", "--no-write-lock-file", "--accept-flake-config"])
            .current_dir(checkout_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(e) => {
                warn!(run_id = %run_id, error = %e, "Failed to run nix flake archive (nix not in PATH?)");
                return;
            }
        };

        match tokio::time::timeout(prefetch_timeout, child.wait_with_output()).await {
            Ok(Ok(output)) if output.status.success() => {
                info!(run_id = %run_id, "Flake inputs pre-fetched into host nix store");
            }
            Ok(Ok(output)) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                warn!(
                    run_id = %run_id,
                    exit_code = ?output.status.code(),
                    stderr = %stderr.chars().take(500).collect::<String>(),
                    "nix flake archive failed (VMs will download inputs directly)"
                );
            }
            Ok(Err(e)) => {
                warn!(run_id = %run_id, error = %e, "nix flake archive process error");
            }
            Err(_elapsed) => {
                warn!(run_id = %run_id, timeout_secs = prefetch_timeout.as_secs(),
                    "nix flake archive timed out (network unavailable? VMs will download inputs directly)");
                // The child process is dropped here which sends SIGKILL.
            }
        }
    }

    /// Create source archive for VM jobs if blob store is available.
    async fn start_pipeline_create_source_archive(
        &self,
        run_id: &str,
        checkout_dir: &std::path::Path,
    ) -> Option<String> {
        let Some(blob_store) = self.orchestrator.blob_store() else {
            debug!(run_id = %run_id, "No blob store configured - VM jobs will not have source archive");
            return None;
        };

        match create_source_archive(checkout_dir, &blob_store).await {
            Ok(hash) => {
                info!(run_id = %run_id, source_hash = %hash, "Created source archive for VM jobs");
                Some(hash.to_string())
            }
            Err(e) => {
                warn!(run_id = %run_id, error = %e, "Failed to create source archive (VM jobs may fail)");
                None
            }
        }
    }

    /// Build updated context after successful checkout.
    ///
    /// The `run_id` is passed in from the already-created run to preserve it.
    /// Previously this was set to `String::new()` which caused log streaming
    /// to write empty run_id into KV keys, making logs unretrievable by the
    /// CLI (which queries with the real run_id).
    fn start_pipeline_build_updated_context(
        &self,
        event: &TriggerEvent,
        run_id: &str,
        checkout_dir: &std::path::Path,
        source_hash: Option<String>,
    ) -> PipelineContext {
        let mut env = HashMap::new();
        env.insert("CI_TRIGGERED_BY".to_string(), event.pusher.to_string());
        env.insert(
            "CI_PREVIOUS_COMMIT".to_string(),
            event.old_hash.map(hex::encode).unwrap_or_else(|| "none".to_string()),
        );
        env.insert("CI_CHECKOUT_DIR".to_string(), checkout_dir.to_string_lossy().to_string());

        PipelineContext {
            repo_id: event.repo_id,
            commit_hash: event.commit_hash,
            ref_name: event.ref_name.clone(),
            triggered_by: event.pusher.to_string(),
            run_id: run_id.to_string(),
            env,
            checkout_dir: Some(checkout_dir.to_path_buf()),
            source_hash,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_path_splitting() {
        let path = ".aspen/ci.ncl";
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        assert_eq!(parts, vec![".aspen", "ci.ncl"]);
    }

    #[test]
    fn test_nested_config_path() {
        let path = "config/ci/pipeline.ncl";
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        assert_eq!(parts, vec!["config", "ci", "pipeline.ncl"]);
    }

    #[test]
    fn test_max_config_size() {
        assert_eq!(MAX_CONFIG_FILE_SIZE, 1024 * 1024);
    }
}
