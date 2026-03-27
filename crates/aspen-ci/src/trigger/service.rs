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
/// Maximum changed paths to diff before falling back to always-trigger.
const MAX_DIFF_PATHS: usize = 10_000;
/// Maximum buffered announcements for the watch-before-push race window.
const MAX_REPLAY_BUFFER: usize = 32;
/// How long to keep buffered announcements before eviction (30 seconds).
const REPLAY_BUFFER_TTL_MS: u64 = 30_000;
/// Default CI config path.
const DEFAULT_CI_CONFIG_PATH: &str = ".aspen/ci.ncl";
/// Interval between periodic mirror scans (seconds).
const MIRROR_SCAN_INTERVAL_SECS: u64 = 60;
/// KV prefix for federation mirror metadata.
const MIRROR_KV_PREFIX: &str = "_fed:mirror:";
/// Maximum mirror entries to scan per sweep.
const MAX_MIRROR_SCAN: u32 = 100;

/// Configuration for the TriggerService.
#[derive(Debug, Clone)]
pub struct TriggerServiceConfig {
    /// Path to CI config file within repositories.
    pub config_path: PathBuf,
    /// Global ref patterns to trigger on (applies to all repos without explicit config).
    pub default_trigger_refs: Vec<String>,
    /// Whether to enable auto-triggering by default.
    pub auto_trigger_enabled: bool,
    /// Whether to trigger CI for federation mirror repos.
    /// Defaults to `false` — operators must opt in.
    pub federation_ci_enabled: bool,
}

impl Default for TriggerServiceConfig {
    fn default() -> Self {
        Self {
            config_path: PathBuf::from(DEFAULT_CI_CONFIG_PATH),
            default_trigger_refs: vec!["refs/heads/main".to_string()],
            auto_trigger_enabled: true,
            federation_ci_enabled: false,
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

    /// List file paths changed between two commits.
    ///
    /// Used by the trigger service to evaluate `ignore_paths` and `only_paths`
    /// filters. Returns `None` if diffing is not supported, in which case
    /// path filters are skipped (trigger always fires).
    ///
    /// Implementations should cap results at `MAX_DIFF_PATHS` entries to
    /// prevent unbounded memory usage on large diffs.
    async fn list_changed_paths(
        &self,
        _repo_id: &RepoId,
        _old_hash: &[u8; 32],
        _new_hash: &[u8; 32],
    ) -> Result<Option<Vec<String>>> {
        // Default: diffing not supported, skip path filters
        Ok(None)
    }
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
    /// Repo IDs that are federation mirrors (for gating by `federation_ci_enabled`).
    mirror_repos: RwLock<HashSet<RepoId>>,
    /// Channel for sending trigger events to processing task.
    trigger_tx: mpsc::Sender<PendingTrigger>,
    /// Tracks all spawned tasks for graceful shutdown.
    task_tracker: TaskTracker,
    /// Bounded replay buffer for announcements that arrive before watch_repo().
    /// Solves the watch-before-push race: announcement arrives via gossip before
    /// the watch_repo() call, gets buffered here and replayed when watch_repo()
    /// is called. Entries expire after REPLAY_BUFFER_TTL_MS.
    replay_buffer: RwLock<std::collections::VecDeque<(tokio::time::Instant, PendingTrigger)>>,
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

/// Check if a file path matches any of the given glob patterns.
///
/// Supports basic glob patterns:
/// - `*` matches any single path component (not `/`)
/// - `**` matches any number of path components
/// - `*.ext` matches files with that extension
/// - `dir/*` matches files directly under `dir/`
/// - `dir/**` matches files anywhere under `dir/`
///
/// This is a verified function: deterministic, no I/O, no side effects.
fn matches_any_pattern(path: &str, patterns: &[String]) -> bool {
    for pattern in patterns {
        if glob_match(path, pattern) {
            return true;
        }
    }
    false
}

/// Simple glob matching for path patterns.
///
/// Supports `*` (single component), `**` (recursive), and literal segments.
fn glob_match(path: &str, pattern: &str) -> bool {
    // Split into segments
    let path_parts: Vec<&str> = path.split('/').collect();
    let pattern_parts: Vec<&str> = pattern.split('/').collect();
    glob_match_segments(&path_parts, &pattern_parts)
}

fn glob_match_segments(path: &[&str], pattern: &[&str]) -> bool {
    if pattern.is_empty() {
        return path.is_empty();
    }
    if path.is_empty() {
        // Pattern remaining — only match if all remaining are `**`
        return pattern.iter().all(|p| *p == "**");
    }

    let pat = pattern[0];
    if pat == "**" {
        // `**` matches zero or more segments
        // Try matching 0, 1, 2, ... segments
        for skip in 0..=path.len() {
            if glob_match_segments(&path[skip..], &pattern[1..]) {
                return true;
            }
        }
        false
    } else if segment_matches(path[0], pat) {
        glob_match_segments(&path[1..], &pattern[1..])
    } else {
        false
    }
}

/// Match a single path segment against a pattern segment.
/// `*` matches anything, `*.ext` matches extension, otherwise literal.
fn segment_matches(segment: &str, pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(suffix) = pattern.strip_prefix('*') {
        return segment.ends_with(suffix);
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return segment.starts_with(prefix);
    }
    segment == pattern
}

/// Evaluate path-based trigger filters (ignore_paths / only_paths).
///
/// Returns `true` if the trigger should fire, `false` if it should be skipped.
///
/// Parse a mirror metadata JSON value to extract the local repo ID.
///
/// Returns `None` if parsing fails or the repo ID is malformed.
fn parse_mirror_repo_id(json_str: &str) -> Option<RepoId> {
    let v: serde_json::Value = serde_json::from_str(json_str).ok()?;
    let repo_hex = v.get("local_repo_id")?.as_str()?;
    let bytes = hex::decode(repo_hex).ok()?;
    if bytes.len() != 32 {
        return None;
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Some(RepoId::from_hash(blake3::Hash::from_bytes(arr)))
}

/// Rules:
/// - If `only_paths` is non-empty: trigger fires only if ANY changed path matches
/// - If `ignore_paths` is non-empty: trigger fires only if ANY changed path does NOT match
/// - Both empty: always trigger
/// - `changed_paths` is `None` (diffing unavailable): always trigger
fn should_trigger_for_paths(changed_paths: Option<&[String]>, ignore_paths: &[String], only_paths: &[String]) -> bool {
    let paths = match changed_paths {
        Some(p) => p,
        None => return true, // Diffing not available, always trigger
    };

    if paths.is_empty() {
        return false; // No files changed — skip trigger
    }

    // only_paths: at least one changed file must match
    if !only_paths.is_empty() {
        return paths.iter().any(|p| matches_any_pattern(p, only_paths));
    }

    // ignore_paths: at least one changed file must NOT match the ignore patterns
    if !ignore_paths.is_empty() {
        return paths.iter().any(|p| !matches_any_pattern(p, ignore_paths));
    }

    // Neither filter set — always trigger
    true
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
            mirror_repos: RwLock::new(HashSet::new()),
            trigger_tx,
            task_tracker,
            replay_buffer: RwLock::new(std::collections::VecDeque::with_capacity(MAX_REPLAY_BUFFER)),
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
        {
            let mut repos = self.watched_repos.write().await;

            if repos.len() >= MAX_WATCHED_REPOS {
                return Err(CiError::InvalidConfig {
                    reason: format!("Cannot watch more than {} repositories", MAX_WATCHED_REPOS),
                });
            }

            repos.insert(repo_id);
        }
        info!(repo_id = %repo_id.to_hex(), "Now watching repository for CI triggers");

        // Replay any buffered announcements for this repo (watch-before-push race fix).
        // Announcements that arrived via gossip before this watch_repo() call are
        // buffered in replay_buffer. Now that we're watching, replay matching entries.
        self.replay_buffered_for_repo(&repo_id).await;

        Ok(())
    }

    /// Replay buffered announcements for a newly-watched repo.
    ///
    /// Removes matching entries from the buffer and sends them to the trigger channel.
    async fn replay_buffered_for_repo(&self, repo_id: &RepoId) {
        let now = tokio::time::Instant::now();
        let ttl = std::time::Duration::from_millis(REPLAY_BUFFER_TTL_MS);
        let mut buffer = self.replay_buffer.write().await;

        // Evict expired entries first
        while let Some((ts, _)) = buffer.front() {
            if now.duration_since(*ts) > ttl {
                buffer.pop_front();
            } else {
                break;
            }
        }

        // Extract and replay matching triggers
        let mut replayed: u32 = 0;
        let mut remaining = std::collections::VecDeque::with_capacity(buffer.len());
        for (ts, trigger) in buffer.drain(..) {
            if trigger.repo_id == *repo_id && now.duration_since(ts) <= ttl {
                if let Err(e) = self.trigger_tx.try_send(trigger) {
                    warn!(
                        repo_id = %repo_id.to_hex(),
                        error = %e,
                        "Failed to replay buffered announcement"
                    );
                } else {
                    replayed += 1;
                }
            } else {
                remaining.push_back((ts, trigger));
            }
        }
        *buffer = remaining;

        if replayed > 0 {
            info!(
                repo_id = %repo_id.to_hex(),
                replayed_count = replayed,
                "Replayed buffered announcements for newly-watched repo"
            );
        }
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

    /// Check if a repo is a known federation mirror.
    pub async fn is_mirror_repo(&self, repo_id: &RepoId) -> bool {
        self.mirror_repos.read().await.contains(repo_id)
    }

    /// Scan KV for federation mirrors and auto-watch them.
    ///
    /// Reads `_fed:mirror:*` entries, extracts the `local_repo_id` from
    /// each mirror's JSON metadata, and calls `watch_repo()` for any
    /// mirrors not yet being watched.
    pub async fn scan_and_watch_mirrors(&self, kv: &dyn aspen_core::KeyValueStore) -> u32 {
        let entries = match kv
            .scan(aspen_core::ScanRequest {
                prefix: MIRROR_KV_PREFIX.to_string(),
                limit_results: Some(MAX_MIRROR_SCAN),
                continuation_token: None,
            })
            .await
        {
            Ok(result) => result.entries,
            Err(e) => {
                warn!(error = %e, "failed to scan for federation mirrors");
                return 0;
            }
        };

        let mut newly_watched = 0u32;
        for entry in &entries {
            let repo_id = match parse_mirror_repo_id(&entry.value) {
                Some(id) => id,
                None => continue,
            };

            self.mirror_repos.write().await.insert(repo_id);

            if !self.is_watching(&repo_id).await {
                if let Err(e) = self.watch_repo(repo_id).await {
                    warn!(
                        repo_id = %hex::encode(repo_id.0),
                        error = %e,
                        "failed to auto-watch federation mirror"
                    );
                } else {
                    newly_watched += 1;
                    info!(
                        repo_id = %hex::encode(repo_id.0),
                        "auto-watching federation mirror for CI triggers"
                    );
                }
            }
        }

        if newly_watched > 0 {
            let total_mirrors = self.mirror_repos.read().await.len();
            info!(newly_watched = newly_watched, total_mirrors = total_mirrors, "federation mirror scan complete");
        }

        newly_watched
    }

    /// Start periodic mirror scanning in the background.
    ///
    /// Only active when `federation_ci_enabled` is true.
    pub fn start_mirror_scan_task(self: &Arc<Self>, kv: Arc<dyn aspen_core::KeyValueStore>) {
        if !self.config.federation_ci_enabled {
            return;
        }

        let service = self.clone();
        self.task_tracker.spawn(async move {
            let interval = std::time::Duration::from_secs(MIRROR_SCAN_INTERVAL_SECS);
            loop {
                service.scan_and_watch_mirrors(kv.as_ref()).await;
                tokio::time::sleep(interval).await;
            }
        });

        info!("federation mirror scan task started (interval: {}s)", MIRROR_SCAN_INTERVAL_SECS);
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

        // Evaluate path-based filters (ignore_paths / only_paths)
        if !config.triggers.ignore_paths.is_empty() || !config.triggers.only_paths.is_empty() {
            if let Some(old_hash) = &trigger.old_hash {
                match self.config_fetcher.list_changed_paths(&trigger.repo_id, old_hash, &trigger.new_hash).await {
                    Ok(Some(paths)) => {
                        if paths.len() >= MAX_DIFF_PATHS {
                            warn!(
                                repo_id = %repo_hex,
                                changed_count = paths.len(),
                                "Diff exceeds {MAX_DIFF_PATHS} paths, skipping path filter"
                            );
                        } else if !should_trigger_for_paths(
                            Some(&paths),
                            &config.triggers.ignore_paths,
                            &config.triggers.only_paths,
                        ) {
                            info!(
                                repo_id = %repo_hex,
                                ref_name = %trigger.ref_name,
                                changed_count = paths.len(),
                                "All changed paths filtered by ignore_paths/only_paths, skipping"
                            );
                            return Ok(());
                        }
                    }
                    Ok(None) => {
                        debug!(
                            repo_id = %repo_hex,
                            "Diffing not supported, skipping path filter"
                        );
                    }
                    Err(e) => {
                        warn!(
                            repo_id = %repo_hex,
                            error = %e,
                            "Failed to diff commits for path filter, triggering anyway"
                        );
                    }
                }
            }
            // First push (old_hash = None) always triggers
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
                // Buffer the announcement for replay when watch_repo() is called.
                // This fixes the race where gossip delivers the push announcement
                // before the orchestrator has called watch_repo().
                let trigger = PendingTrigger {
                    repo_id,
                    ref_name: ref_name.clone(),
                    new_hash,
                    old_hash,
                    signer,
                };
                let mut buffer = trigger_service.replay_buffer.write().await;
                if buffer.len() >= MAX_REPLAY_BUFFER {
                    buffer.pop_front(); // Evict oldest to stay bounded
                }
                buffer.push_back((tokio::time::Instant::now(), trigger));
                debug!(
                    repo_id = %repo_id.to_hex(),
                    buffer_size = buffer.len(),
                    "Buffered announcement for unwatched repo (will replay on watch)"
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

            // Gate federation mirror repos behind federation_ci_enabled
            if trigger_service.is_mirror_repo(&repo_id).await && !trigger_service.config.federation_ci_enabled {
                debug!(
                    repo_id = %repo_id.to_hex(),
                    "federation CI disabled - skipping trigger for mirror repo"
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

    // ── Path filter tests ────────────────────────────────────────────

    #[test]
    fn test_glob_match_exact() {
        assert!(glob_match("README.md", "README.md"));
        assert!(!glob_match("README.md", "CHANGELOG.md"));
    }

    #[test]
    fn test_glob_match_star_extension() {
        assert!(glob_match("README.md", "*.md"));
        assert!(glob_match("CHANGELOG.md", "*.md"));
        assert!(!glob_match("src/main.rs", "*.md"));
    }

    #[test]
    fn test_glob_match_dir_star() {
        assert!(glob_match("docs/README.md", "docs/*"));
        assert!(!glob_match("docs/sub/file.md", "docs/*"));
        assert!(!glob_match("src/main.rs", "docs/*"));
    }

    #[test]
    fn test_glob_match_double_star() {
        assert!(glob_match("docs/README.md", "docs/**"));
        assert!(glob_match("docs/sub/file.md", "docs/**"));
        assert!(glob_match("docs/a/b/c/file.txt", "docs/**"));
        assert!(!glob_match("src/main.rs", "docs/**"));
    }

    #[test]
    fn test_glob_match_double_star_extension() {
        // **/*.md matches .md files anywhere
        assert!(glob_match("docs/README.md", "**/*.md"));
        assert!(glob_match("a/b/c.md", "**/*.md"));
        assert!(!glob_match("src/main.rs", "**/*.md"));
    }

    #[test]
    fn test_should_trigger_for_paths_no_filters() {
        let paths = vec!["src/main.rs".to_string()];
        assert!(should_trigger_for_paths(Some(&paths), &[], &[]));
    }

    #[test]
    fn test_should_trigger_for_paths_none_changed() {
        // Diffing not available — always trigger
        assert!(should_trigger_for_paths(None, &["*.md".to_string()], &[]));
    }

    #[test]
    fn test_should_trigger_for_paths_empty_changed() {
        // No files changed — skip trigger
        assert!(!should_trigger_for_paths(Some(&[]), &[], &[]));
    }

    #[test]
    fn test_ignore_paths_docs_only_push() {
        let paths = vec!["README.md".to_string(), "docs/guide.md".to_string()];
        let ignore = vec!["*.md".to_string(), "docs/*".to_string()];
        // All changed files match ignore patterns → skip trigger
        assert!(!should_trigger_for_paths(Some(&paths), &ignore, &[]));
    }

    #[test]
    fn test_ignore_paths_mixed_push() {
        let paths = vec!["README.md".to_string(), "src/main.rs".to_string()];
        let ignore = vec!["*.md".to_string()];
        // src/main.rs does NOT match ignore → trigger fires
        assert!(should_trigger_for_paths(Some(&paths), &ignore, &[]));
    }

    #[test]
    fn test_only_paths_match() {
        let paths = vec!["src/main.rs".to_string(), "docs/README.md".to_string()];
        let only = vec!["src/*".to_string()];
        // src/main.rs matches only_paths → trigger fires
        assert!(should_trigger_for_paths(Some(&paths), &[], &only));
    }

    #[test]
    fn test_only_paths_no_match() {
        let paths = vec!["docs/README.md".to_string()];
        let only = vec!["src/*".to_string()];
        // No changed file matches only_paths → skip trigger
        assert!(!should_trigger_for_paths(Some(&paths), &[], &only));
    }

    // ── Replay buffer tests ─────────────────────────────────────────

    /// Generate a valid test public key.
    fn test_public_key() -> PublicKey {
        iroh::SecretKey::generate(&mut rand::rng()).public()
    }

    #[tokio::test]
    async fn test_replay_buffer_announcement_then_watch() {
        let config = TriggerServiceConfig::default();
        let fetcher = Arc::new(MockConfigFetcher { config: None });
        let starter = Arc::new(MockPipelineStarter {
            started_count: AtomicU32::new(0),
        });

        let service = TriggerService::new(config, fetcher, starter);

        let repo_id = RepoId::from_hash(blake3::hash(b"replay-test"));

        // Simulate an announcement arriving for an unwatched repo
        let trigger = PendingTrigger {
            repo_id,
            ref_name: "refs/heads/main".to_string(),
            new_hash: [1u8; 32],
            old_hash: Some([0u8; 32]),
            signer: test_public_key(),
        };

        {
            let mut buffer = service.replay_buffer.write().await;
            buffer.push_back((tokio::time::Instant::now(), trigger));
        }

        assert_eq!(service.replay_buffer.read().await.len(), 1);

        // Now watch the repo — should replay the buffered announcement
        service.watch_repo(repo_id).await.unwrap();

        // Buffer should be drained for this repo
        assert_eq!(service.replay_buffer.read().await.len(), 0);
    }

    #[tokio::test]
    async fn test_replay_buffer_bounded() {
        let config = TriggerServiceConfig::default();
        let fetcher = Arc::new(MockConfigFetcher { config: None });
        let starter = Arc::new(MockPipelineStarter {
            started_count: AtomicU32::new(0),
        });

        let service = TriggerService::new(config, fetcher, starter);
        let signer = test_public_key();

        // Fill buffer beyond capacity
        for i in 0..MAX_REPLAY_BUFFER + 5 {
            let repo_id = RepoId::from_hash(blake3::hash(format!("repo-{i}").as_bytes()));
            let trigger = PendingTrigger {
                repo_id,
                ref_name: "refs/heads/main".to_string(),
                new_hash: [i as u8; 32],
                old_hash: None,
                signer,
            };
            let mut buffer = service.replay_buffer.write().await;
            if buffer.len() >= MAX_REPLAY_BUFFER {
                buffer.pop_front();
            }
            buffer.push_back((tokio::time::Instant::now(), trigger));
        }

        // Should be capped at MAX_REPLAY_BUFFER
        assert_eq!(service.replay_buffer.read().await.len(), MAX_REPLAY_BUFFER);
    }

    // ── Mirror scan tests ───────────────────────────────────────────

    #[tokio::test]
    async fn test_scan_and_watch_mirrors() {
        use aspen_core::KeyValueStore;
        use aspen_core::WriteRequest;

        let config = TriggerServiceConfig {
            federation_ci_enabled: true,
            ..Default::default()
        };
        let fetcher = Arc::new(MockConfigFetcher { config: None });
        let starter = Arc::new(MockPipelineStarter {
            started_count: AtomicU32::new(0),
        });

        let service = TriggerService::new(config, fetcher, starter);

        // Create an in-memory KV store with a mirror entry
        let kv = aspen_testing_core::DeterministicKeyValueStore::new();
        let repo_hex = hex::encode([42u8; 32]);
        let mirror_json = serde_json::json!({
            "fed_id": "abc:def",
            "origin_cluster_key": "0000",
            "local_repo_id": repo_hex,
            "last_sync_timestamp": 1000,
            "created_at": 1000
        });
        aspen_core::KeyValueStore::write(
            kv.as_ref(),
            WriteRequest::set("_fed:mirror:abc:def", mirror_json.to_string()),
        )
        .await
        .unwrap();

        // Scan should discover and watch the mirror
        let newly_watched = service.scan_and_watch_mirrors(kv.as_ref()).await;
        assert_eq!(newly_watched, 1);

        // The repo should now be watched and tracked as a mirror
        let repo_id = RepoId::from_hash(blake3::Hash::from_bytes([42u8; 32]));
        assert!(service.is_watching(&repo_id).await);
        assert!(service.is_mirror_repo(&repo_id).await);

        // Second scan should not re-watch
        let newly_watched = service.scan_and_watch_mirrors(kv.as_ref()).await;
        assert_eq!(newly_watched, 0);
    }

    #[tokio::test]
    async fn test_mirror_repo_gated_when_federation_ci_disabled() {
        let config = TriggerServiceConfig {
            federation_ci_enabled: false,
            ..Default::default()
        };
        let fetcher = Arc::new(MockConfigFetcher { config: None });
        let starter = Arc::new(MockPipelineStarter {
            started_count: AtomicU32::new(0),
        });

        let service = TriggerService::new(config, fetcher, starter);

        // Manually mark a repo as a mirror and watch it
        let repo_id = RepoId::from_hash(blake3::hash(b"mirror-repo"));
        service.mirror_repos.write().await.insert(repo_id);
        service.watch_repo(repo_id).await.unwrap();

        assert!(service.is_mirror_repo(&repo_id).await);
        assert!(service.is_watching(&repo_id).await);
        // federation_ci_enabled = false, so triggers should be dropped for this repo
        assert!(!service.config.federation_ci_enabled);
    }

    #[test]
    fn test_parse_mirror_repo_id_valid() {
        let repo_hex = hex::encode([99u8; 32]);
        let json = serde_json::json!({
            "fed_id": "abc:def",
            "origin_cluster_key": "0000",
            "local_repo_id": repo_hex,
            "last_sync_timestamp": 1000,
            "created_at": 1000
        });
        let parsed = parse_mirror_repo_id(&json.to_string());
        assert!(parsed.is_some());
    }

    #[test]
    fn test_parse_mirror_repo_id_invalid() {
        assert!(parse_mirror_repo_id("not json").is_none());
        assert!(parse_mirror_repo_id("{}").is_none());
        assert!(parse_mirror_repo_id(r#"{"local_repo_id": "short"}"#).is_none());
    }
}
