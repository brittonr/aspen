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

use aspen_blob::BlobStore;
use aspen_core::KeyValueStore;
use aspen_forge::ForgeNode;
use aspen_forge::identity::RepoId;
use async_trait::async_trait;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::error::CiError;
use crate::error::Result;
use crate::orchestrator::PipelineContext;
use crate::orchestrator::PipelineOrchestrator;
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

        let tree = self.forge.git.get_tree(&tree_hash).await.map_err(|e| CiError::ForgeOperation {
            reason: format!("Failed to load tree: {}", e),
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
                        let content =
                            self.forge.git.get_blob(&entry_hash).await.map_err(|e| CiError::ForgeOperation {
                                reason: format!("Failed to load blob: {}", e),
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

        // Generate a unique run ID for the checkout directory
        let run_id = uuid::Uuid::new_v4().to_string();
        let checkout_dir = crate::checkout::checkout_dir_for_run(&run_id);

        // Checkout the repository to the temp directory
        info!(
            repo_id = %event.repo_id.to_hex(),
            commit = %hex::encode(event.commit_hash),
            checkout_dir = %checkout_dir.display(),
            "Checking out repository for CI"
        );

        crate::checkout::checkout_repository(&self.forge, &event.commit_hash, &checkout_dir).await?;

        // Clone external dependencies (e.g., snix)
        crate::checkout::clone_external_dependencies(&checkout_dir).await?;

        // Build environment variables from trigger context
        let mut env = HashMap::new();
        env.insert("CI_TRIGGERED_BY".to_string(), event.pusher.to_string());
        env.insert(
            "CI_PREVIOUS_COMMIT".to_string(),
            event.old_hash.map(hex::encode).unwrap_or_else(|| "none".to_string()),
        );
        // Add checkout directory as environment variable
        env.insert("CI_CHECKOUT_DIR".to_string(), checkout_dir.to_string_lossy().to_string());

        // Create pipeline context from trigger event with checkout directory
        let context = PipelineContext {
            repo_id: event.repo_id,
            commit_hash: event.commit_hash,
            ref_name: event.ref_name.clone(),
            triggered_by: event.pusher.to_string(),
            env,
            checkout_dir: Some(checkout_dir.clone()),
        };

        // Execute the pipeline
        let run = self.orchestrator.execute(event.config, context).await?;

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
