//! TriggerService for automatic CI pipeline execution.
//!
//! This service subscribes to forge gossip announcements and automatically
//! triggers CI pipelines when refs are updated (e.g., git push).
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                       TriggerService                             │
//! │                                                                  │
//! │  ┌──────────────────┐    ┌────────────────┐    ┌─────────────┐  │
//! │  │ AnnouncementCb   │───►│ ConfigLoader   │───►│ Orchestrator│  │
//! │  │ (RefUpdate)      │    │ (.aspen/ci.ncl)│    │ (start run) │  │
//! │  └──────────────────┘    └────────────────┘    └─────────────┘  │
//! │                                                                  │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use aspen_forge::gossip::Announcement;
use aspen_forge::gossip::AnnouncementCallback;
use aspen_forge::identity::RepoId;
use async_trait::async_trait;
use iroh::PublicKey;
use tokio::sync::RwLock;
use tokio::sync::mpsc;
use tokio_util::task::TaskTracker;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use crate::config::load_pipeline_config_str_async;
use crate::config::types::PipelineConfig;
use crate::error::CiError;
use crate::error::Result;

// Tiger Style: Bounded resources
/// Maximum pending triggers to queue.
const MAX_PENDING_TRIGGERS: usize = 100;
/// Maximum watched repositories.
const MAX_WATCHED_REPOS: usize = 100;
/// Default CI config path.
const DEFAULT_CI_CONFIG_PATH: &str = ".aspen/ci.ncl";

/// Configuration for the TriggerService.
#[derive(Debug, Clone)]
pub struct TriggerServiceConfig {
    /// Path to CI config file within repositories.
    pub config_path: PathBuf,
    /// Global ref patterns to trigger on (applies to all repos without explicit config).
    pub default_trigger_refs: Vec<String>,
    /// Whether to enable auto-triggering by default.
    pub auto_trigger_enabled: bool,
}

impl Default for TriggerServiceConfig {
    fn default() -> Self {
        Self {
            config_path: PathBuf::from(DEFAULT_CI_CONFIG_PATH),
            default_trigger_refs: vec!["refs/heads/main".to_string()],
            auto_trigger_enabled: true,
        }
    }
}

/// Information about a triggered pipeline.
#[derive(Debug, Clone)]
pub struct TriggerEvent {
    /// Repository ID.
    pub repo_id: RepoId,
    /// Ref that was updated.
    pub ref_name: String,
    /// New commit hash.
    pub commit_hash: [u8; 32],
    /// Previous commit hash (if any).
    pub old_hash: Option<[u8; 32]>,
    /// Public key of the node that pushed.
    pub pusher: PublicKey,
    /// Pipeline configuration.
    pub config: PipelineConfig,
}

/// Trait for fetching CI configuration from a repository at a specific commit.
///
/// This abstraction allows testing without actual forge access.
#[async_trait]
pub trait ConfigFetcher: Send + Sync + 'static {
    /// Fetch the CI configuration file content from a repository at a specific commit.
    ///
    /// # Arguments
    ///
    /// * `repo_id` - Repository identifier
    /// * `commit_hash` - Commit hash to fetch from
    /// * `config_path` - Path to the config file within the repo
    ///
    /// # Returns
    ///
    /// The config file contents as a string, or None if not found.
    async fn fetch_config(&self, repo_id: &RepoId, commit_hash: &[u8; 32], config_path: &str)
    -> Result<Option<String>>;
}

/// Trait for starting pipeline executions.
///
/// This abstraction allows testing without actual orchestrator access.
#[async_trait]
pub trait PipelineStarter: Send + Sync + 'static {
    /// Start a pipeline execution.
    ///
    /// # Arguments
    ///
    /// * `event` - The trigger event with pipeline configuration
    ///
    /// # Returns
    ///
    /// A unique run ID for the started pipeline.
    async fn start_pipeline(&self, event: TriggerEvent) -> Result<String>;
}

/// TriggerService handles automatic CI pipeline triggering.
///
/// It listens for RefUpdate announcements from forge gossip and
/// automatically starts pipelines for repositories with CI configuration.
pub struct TriggerService {
    /// Service configuration.
    config: TriggerServiceConfig,
    /// Config fetcher for loading CI configs from repos.
    config_fetcher: Arc<dyn ConfigFetcher>,
    /// Pipeline starter for executing pipelines.
    pipeline_starter: Arc<dyn PipelineStarter>,
    /// Repositories being watched for triggers.
    watched_repos: RwLock<HashSet<RepoId>>,
    /// Channel for sending trigger events to processing task.
    trigger_tx: mpsc::Sender<PendingTrigger>,
    /// Tracks all spawned tasks for graceful shutdown.
    task_tracker: TaskTracker,
}

/// Internal pending trigger waiting to be processed.
#[derive(Debug)]
struct PendingTrigger {
    repo_id: RepoId,
    ref_name: String,
    new_hash: [u8; 32],
    old_hash: Option<[u8; 32]>,
    signer: PublicKey,
}

impl TriggerService {
    /// Create a new TriggerService.
    ///
    /// # Arguments
    ///
    /// * `config` - Service configuration
    /// * `config_fetcher` - Fetcher for CI config files
    /// * `pipeline_starter` - Starter for pipeline executions
    pub fn new(
        config: TriggerServiceConfig,
        config_fetcher: Arc<dyn ConfigFetcher>,
        pipeline_starter: Arc<dyn PipelineStarter>,
    ) -> Arc<Self> {
        let (trigger_tx, trigger_rx) = mpsc::channel(MAX_PENDING_TRIGGERS);
        let task_tracker = TaskTracker::new();

        let service = Arc::new(Self {
            config,
            config_fetcher,
            pipeline_starter,
            watched_repos: RwLock::new(HashSet::new()),
            trigger_tx,
            task_tracker,
        });

        // Spawn the processing task
        let service_clone = service.clone();
        service.task_tracker.spawn(async move {
            service_clone.process_triggers(trigger_rx).await;
        });

        service
    }

    /// Watch a repository for CI triggers.
    ///
    /// When a RefUpdate announcement is received for this repo,
    /// the service will check for CI config and start pipelines.
    pub async fn watch_repo(&self, repo_id: RepoId) -> Result<()> {
        let mut repos = self.watched_repos.write().await;

        if repos.len() >= MAX_WATCHED_REPOS {
            return Err(CiError::InvalidConfig {
                reason: format!("Cannot watch more than {} repositories", MAX_WATCHED_REPOS),
            });
        }

        repos.insert(repo_id);
        info!(repo_id = %repo_id.to_hex(), "Now watching repository for CI triggers");

        Ok(())
    }

    /// Stop watching a repository for CI triggers.
    pub async fn unwatch_repo(&self, repo_id: &RepoId) {
        self.watched_repos.write().await.remove(repo_id);
        info!(repo_id = %repo_id.to_hex(), "Stopped watching repository for CI triggers");
    }

    /// Check if a repository is being watched.
    pub async fn is_watching(&self, repo_id: &RepoId) -> bool {
        self.watched_repos.read().await.contains(repo_id)
    }

    /// Get the number of watched repositories.
    pub async fn watched_count(&self) -> usize {
        self.watched_repos.read().await.len()
    }

    /// Check if the trigger service is ready to process triggers.
    ///
    /// Returns true if the trigger channel is open and processing task is active.
    pub fn is_ready(&self) -> bool {
        !self.trigger_tx.is_closed()
    }

    /// Get list of currently watched repository IDs.
    ///
    /// Useful for debugging and verifying watch state.
    pub async fn watched_repos(&self) -> Vec<RepoId> {
        self.watched_repos.read().await.iter().copied().collect()
    }

    /// Process incoming triggers from the channel.
    async fn process_triggers(self: Arc<Self>, mut rx: mpsc::Receiver<PendingTrigger>) {
        while let Some(trigger) = rx.recv().await {
            if let Err(e) = self.handle_trigger(trigger).await {
                error!("Failed to process trigger: {}", e);
            }
        }

        debug!("Trigger processing task shutting down");
    }

    /// Handle a single trigger event.
    async fn handle_trigger(&self, trigger: PendingTrigger) -> Result<()> {
        let repo_hex = trigger.repo_id.to_hex();
        info!(
            repo_id = %repo_hex,
            ref_name = %trigger.ref_name,
            "Processing CI trigger"
        );

        // Fetch CI config from the commit
        let config_path = self.config.config_path.to_string_lossy();
        let config_content =
            self.config_fetcher.fetch_config(&trigger.repo_id, &trigger.new_hash, &config_path).await?;

        let config_content = match config_content {
            Some(c) => c,
            None => {
                debug!(
                    repo_id = %repo_hex,
                    config_path = %config_path,
                    "No CI config found, skipping trigger"
                );
                return Ok(());
            }
        };

        // Parse the configuration using async version for large stack
        let source_name = format!("{}:{}", repo_hex, config_path);
        let config = load_pipeline_config_str_async(config_content, source_name).await?;

        // Check if this ref should trigger a build
        if !config.triggers.should_trigger(&trigger.ref_name) {
            debug!(
                repo_id = %repo_hex,
                ref_name = %trigger.ref_name,
                "Ref does not match trigger patterns, skipping"
            );
            return Ok(());
        }

        // Create trigger event
        let event = TriggerEvent {
            repo_id: trigger.repo_id,
            ref_name: trigger.ref_name.clone(),
            commit_hash: trigger.new_hash,
            old_hash: trigger.old_hash,
            pusher: trigger.signer,
            config,
        };

        // Start the pipeline
        let run_id = self.pipeline_starter.start_pipeline(event).await?;

        info!(
            repo_id = %repo_hex,
            ref_name = %trigger.ref_name,
            run_id = %run_id,
            "Started CI pipeline"
        );

        Ok(())
    }

    /// Shutdown the trigger service.
    pub async fn shutdown(&self) {
        // Close the tracker so no new tasks can be spawned, then wait for all
        // in-flight tasks to complete.
        self.task_tracker.close();
        self.task_tracker.wait().await;

        info!("TriggerService shutdown complete");
    }
}

/// Handler for forge gossip announcements that triggers CI pipelines.
///
/// This type implements `AnnouncementCallback` and can be registered
/// with `ForgeGossipService` to receive RefUpdate announcements.
pub struct CiTriggerHandler {
    /// Reference to the trigger service.
    trigger_service: Arc<TriggerService>,
}

impl CiTriggerHandler {
    /// Create a new CI trigger handler.
    pub fn new(trigger_service: Arc<TriggerService>) -> Self {
        Self { trigger_service }
    }
}

impl AnnouncementCallback for CiTriggerHandler {
    fn on_announcement(&self, announcement: &Announcement, signer: &PublicKey) {
        // Only handle RefUpdate announcements
        let (repo_id, ref_name, new_hash, old_hash) = match announcement {
            Announcement::RefUpdate {
                repo_id,
                ref_name,
                new_hash,
                old_hash,
            } => (repo_id, ref_name, new_hash, old_hash),
            _ => return, // Ignore other announcement types
        };

        // Clone data for async context
        let trigger_service = self.trigger_service.clone();
        let repo_id = *repo_id;
        let ref_name = ref_name.clone();
        let new_hash = *new_hash;
        let old_hash = *old_hash;
        let signer = *signer;

        // Spawn tracked task to check if we should trigger
        self.trigger_service.task_tracker.spawn(async move {
            info!(
                repo_id = %repo_id.to_hex(),
                ref_name = %ref_name,
                "CI trigger handler received RefUpdate announcement"
            );

            // Check if we're watching this repo
            let is_watching = trigger_service.is_watching(&repo_id).await;
            let watched_count = trigger_service.watched_count().await;

            info!(
                repo_id = %repo_id.to_hex(),
                is_watching = is_watching,
                watched_count = watched_count,
                "CI trigger handler checking watch status"
            );

            if !is_watching {
                info!(
                    repo_id = %repo_id.to_hex(),
                    watched_count = watched_count,
                    "repo not being watched for CI triggers - skipping"
                );
                return;
            }

            // Check if auto-triggering is enabled
            if !trigger_service.config.auto_trigger_enabled {
                info!(
                    repo_id = %repo_id.to_hex(),
                    "auto-triggering disabled in config - skipping"
                );
                return;
            }

            info!(
                repo_id = %repo_id.to_hex(),
                ref_name = %ref_name,
                "queueing CI trigger for ref update"
            );

            // Send trigger to processing queue
            let trigger = PendingTrigger {
                repo_id,
                ref_name,
                new_hash,
                old_hash,
                signer,
            };

            if let Err(e) = trigger_service.trigger_tx.send(trigger).await {
                warn!("Failed to queue CI trigger: {}", e);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicU32;
    use std::sync::atomic::Ordering;

    use super::*;

    /// Mock config fetcher for testing.
    struct MockConfigFetcher {
        config: Option<String>,
    }

    #[async_trait]
    impl ConfigFetcher for MockConfigFetcher {
        async fn fetch_config(
            &self,
            _repo_id: &RepoId,
            _commit_hash: &[u8; 32],
            _config_path: &str,
        ) -> Result<Option<String>> {
            Ok(self.config.clone())
        }
    }

    /// Mock pipeline starter for testing.
    struct MockPipelineStarter {
        started_count: AtomicU32,
    }

    #[async_trait]
    impl PipelineStarter for MockPipelineStarter {
        async fn start_pipeline(&self, _event: TriggerEvent) -> Result<String> {
            let count = self.started_count.fetch_add(1, Ordering::SeqCst);
            Ok(format!("run-{}", count))
        }
    }

    #[tokio::test]
    async fn test_watch_and_unwatch_repo() {
        let config = TriggerServiceConfig::default();
        let fetcher = Arc::new(MockConfigFetcher { config: None });
        let starter = Arc::new(MockPipelineStarter {
            started_count: AtomicU32::new(0),
        });

        let service = TriggerService::new(config, fetcher, starter);

        let repo_id = RepoId::from_hash(blake3::hash(b"test-repo"));

        // Initially not watching
        assert!(!service.is_watching(&repo_id).await);
        assert_eq!(service.watched_count().await, 0);

        // Watch repo
        service.watch_repo(repo_id).await.unwrap();
        assert!(service.is_watching(&repo_id).await);
        assert_eq!(service.watched_count().await, 1);

        // Unwatch repo
        service.unwatch_repo(&repo_id).await;
        assert!(!service.is_watching(&repo_id).await);
        assert_eq!(service.watched_count().await, 0);
    }

    #[tokio::test]
    async fn test_max_watched_repos() {
        let config = TriggerServiceConfig::default();
        let fetcher = Arc::new(MockConfigFetcher { config: None });
        let starter = Arc::new(MockPipelineStarter {
            started_count: AtomicU32::new(0),
        });

        let service = TriggerService::new(config, fetcher, starter);

        // Add maximum repos
        for i in 0..MAX_WATCHED_REPOS {
            let repo_id = RepoId::from_hash(blake3::hash(format!("repo-{}", i).as_bytes()));
            service.watch_repo(repo_id).await.unwrap();
        }

        assert_eq!(service.watched_count().await, MAX_WATCHED_REPOS);

        // Adding one more should fail
        let extra_repo = RepoId::from_hash(blake3::hash(b"extra-repo"));
        let result = service.watch_repo(extra_repo).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_trigger_service_config_default() {
        let config = TriggerServiceConfig::default();
        assert_eq!(config.config_path, PathBuf::from(".aspen/ci.ncl"));
        assert!(config.auto_trigger_enabled);
        assert!(!config.default_trigger_refs.is_empty());
    }
}
