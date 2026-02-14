//! PijulSyncHandler - Handler for incoming Pijul gossip announcements.
//!
//! This module implements the logic for responding to Pijul announcements:
//! - `HaveChanges`: Download missing changes from the offering peer
//! - `WantChanges`: Respond with changes we have
//! - `ChannelUpdate`: Detect if we're behind and request missing changes
//! - `ChangeAvailable`: Request the change if we need it
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                     PijulSyncHandler                             │
//! │  ┌─────────────────┐  ┌─────────────────┐  ┌────────────────┐  │
//! │  │ PijulStore      │  │ PijulSyncService│  │ Pending Requests│ │
//! │  │ (check/download)│  │ (send responses)│  │ (dedup requests)│ │
//! │  └─────────────────┘  └─────────────────┘  └────────────────┘  │
//! │           │                    │                    │           │
//! │           └────────────────────┴────────────────────┘           │
//! │                              │                                   │
//! │                     on_announcement()                           │
//! │                              │                                   │
//! │  ┌───────────────────────────┼───────────────────────────┐     │
//! │  │ HaveChanges │ WantChanges │ ChannelUpdate │ ChangeAvail│     │
//! │  └───────────────────────────┴───────────────────────────┘     │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

mod callback;
mod commands;
mod coordinator;
mod metrics;
mod pending;

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use aspen_blob::prelude::*;
use aspen_core::KeyValueStore;
use aspen_forge::identity::RepoId;
use iroh::PublicKey;
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tracing::debug;
use tracing::info;
use tracing::instrument;
use tracing::trace;
use tracing::warn;

use self::commands::SyncCommand;
use self::coordinator::DownloadCoordinator;
pub use self::metrics::DownloadMetrics;
use self::pending::PendingRequests;
use super::constants::DOWNLOAD_RETRY_INITIAL_BACKOFF_MS;
use super::constants::DOWNLOAD_RETRY_MAX_BACKOFF_MS;
use super::constants::MAX_CHANGES_PER_REQUEST;
use super::constants::MAX_DOWNLOAD_RETRIES;
use super::gossip::PijulAnnouncement;
use super::store::PijulStore;
use super::sync::PijulSyncService;
use super::types::ChangeHash;

/// Handle for a running sync handler, returned from `PijulSyncHandler::spawn`.
pub struct PijulSyncHandlerHandle {
    /// The worker task handle.
    worker_handle: tokio::task::JoinHandle<()>,
}

impl PijulSyncHandlerHandle {
    /// Wait for the handler worker to complete.
    pub async fn join(self) -> Result<(), tokio::task::JoinError> {
        self.worker_handle.await
    }

    /// Abort the handler worker.
    pub fn abort(&self) {
        self.worker_handle.abort();
    }
}

/// Handler for Pijul sync announcements.
///
/// Implements `PijulSyncCallback` to process incoming announcements and
/// trigger appropriate sync actions (downloads, responses, etc.).
///
/// # Thread Safety
///
/// The handler uses interior mutability with `RwLock` for pending requests
/// and sends commands to an async worker for actual network operations.
///
/// # Example
///
/// ```ignore
/// // Spawn the handler and worker together
/// let (handler, handle) = PijulSyncHandler::spawn(store, sync_service);
///
/// // Watch repositories you want to sync
/// handler.watch_repo(repo_id);
///
/// // Use as callback for PijulSyncService
/// let sync = PijulSyncService::spawn(gossip, key, events, Some(handler)).await?;
///
/// // Later, cleanup
/// handle.abort();
/// ```
pub struct PijulSyncHandler<B: BlobStore, K: KeyValueStore + ?Sized> {
    /// The Pijul store for checking/downloading changes.
    store: Arc<PijulStore<B, K>>,
    /// The sync service for sending responses.
    sync_service: Arc<PijulSyncService>,
    /// Pending requests for deduplication.
    pub(self) pending: RwLock<PendingRequests>,
    /// Command sender for the async worker.
    pub(self) command_tx: mpsc::UnboundedSender<SyncCommand>,
    /// Repos we're interested in syncing.
    watched_repos: RwLock<HashSet<RepoId>>,
    /// Coordinator for concurrent downloads.
    download_coordinator: DownloadCoordinator,
    /// Metrics for download operations.
    download_metrics: DownloadMetrics,
}

impl<B: BlobStore + 'static, K: KeyValueStore + ?Sized + 'static> PijulSyncHandler<B, K> {
    /// Spawn a new sync handler with its worker task.
    ///
    /// This creates the handler and starts the background worker that processes
    /// sync commands. The handler can be used as a `PijulSyncCallback`.
    ///
    /// # Arguments
    ///
    /// - `store`: PijulStore for change operations
    /// - `sync_service`: PijulSyncService for sending responses
    ///
    /// # Returns
    ///
    /// A tuple of (handler, handle). The handler implements `PijulSyncCallback`
    /// and can be passed to `PijulSyncService`. The handle can be used to
    /// await or abort the worker.
    pub fn spawn(
        store: Arc<PijulStore<B, K>>,
        sync_service: Arc<PijulSyncService>,
    ) -> (Arc<Self>, PijulSyncHandlerHandle) {
        let (command_tx, mut command_rx) = mpsc::unbounded_channel();

        let handler = Arc::new(Self {
            store,
            sync_service,
            pending: RwLock::new(PendingRequests::new()),
            command_tx,
            watched_repos: RwLock::new(HashSet::new()),
            download_coordinator: DownloadCoordinator::new(),
            download_metrics: DownloadMetrics::default(),
        });

        // Spawn the worker task
        let handler_clone = handler.clone();
        let worker_handle = tokio::spawn(async move {
            while let Some(cmd) = command_rx.recv().await {
                match cmd {
                    SyncCommand::DownloadChanges {
                        repo_id,
                        hashes,
                        provider,
                    } => {
                        handler_clone.handle_download_changes(&repo_id, &hashes, provider).await;
                    }
                    SyncCommand::RespondWithChanges {
                        repo_id,
                        requested_hashes,
                    } => {
                        handler_clone.handle_respond_with_changes(&repo_id, &requested_hashes).await;
                    }
                    SyncCommand::CheckChannelSync {
                        repo_id,
                        channel,
                        remote_head,
                        announcer,
                    } => {
                        handler_clone.handle_check_channel_sync(&repo_id, &channel, &remote_head, announcer).await;
                    }
                    SyncCommand::RequestChange {
                        repo_id,
                        change_hash,
                        provider,
                    } => {
                        handler_clone.handle_request_change(&repo_id, &change_hash, provider).await;
                    }
                    SyncCommand::ProcessDeferredAnnouncement { announcement, signer } => {
                        // Process announcement in async context where blocking is acceptable.
                        // This handles announcements that were deferred from on_announcement()
                        // due to lock contention (try_write() returned None).
                        handler_clone.process_announcement_blocking(&announcement, &signer);
                    }
                }

                // Periodic cleanup
                handler_clone.pending.write().cleanup();
            }

            debug!("pijul sync handler worker shutting down");
        });

        let handle = PijulSyncHandlerHandle { worker_handle };
        (handler, handle)
    }

    /// Add a repository to the watch list.
    ///
    /// Only announcements for watched repos will trigger sync actions.
    pub fn watch_repo(&self, repo_id: RepoId) {
        self.watched_repos.write().insert(repo_id);
    }

    /// Remove a repository from the watch list.
    pub fn unwatch_repo(&self, repo_id: &RepoId) {
        self.watched_repos.write().remove(repo_id);
    }

    /// Check if a repo is being watched.
    pub fn is_watching(&self, repo_id: &RepoId) -> bool {
        self.watched_repos.read().contains(repo_id)
    }

    /// Get a reference to the download metrics.
    pub fn metrics(&self) -> &DownloadMetrics {
        &self.download_metrics
    }

    /// Check if a repo is being watched without blocking.
    ///
    /// Returns `None` if the lock is contended (cannot determine immediately),
    /// or `Some(bool)` with the watch status.
    pub(self) fn try_is_watching(&self, repo_id: &RepoId) -> Option<bool> {
        self.watched_repos.try_read().map(|guard| guard.contains(repo_id))
    }

    /// Process an announcement synchronously (blocking on locks is acceptable).
    ///
    /// This is called from the async worker task when an announcement was deferred
    /// from `on_announcement()` due to lock contention. Since we're in the async
    /// worker, brief blocking on parking_lot locks is acceptable and won't stall
    /// the gossip event loop.
    fn process_announcement_blocking(&self, announcement: &PijulAnnouncement, signer: &PublicKey) {
        let repo_id = *announcement.repo_id();

        // Check if we're watching this repo (can block here, it's safe)
        if !self.is_watching(&repo_id) {
            trace!(repo_id = %repo_id.to_hex(), "deferred: ignoring announcement for unwatched repo");
            return;
        }

        match announcement {
            PijulAnnouncement::HaveChanges { hashes, offerer, .. } => {
                let mut to_download = Vec::new();
                let mut pending = self.pending.write();

                for hash in hashes {
                    if pending.should_request(hash) {
                        pending.mark_requested(*hash);
                        to_download.push(*hash);
                    }
                }

                drop(pending);

                if !to_download.is_empty() {
                    debug!(
                        repo_id = %repo_id.to_hex(),
                        count = to_download.len(),
                        offerer = %offerer.fmt_short(),
                        "deferred: queueing download of offered changes"
                    );

                    let _ = self.command_tx.send(SyncCommand::DownloadChanges {
                        repo_id,
                        hashes: to_download,
                        provider: *offerer,
                    });
                }
            }

            PijulAnnouncement::WantChanges { hashes, requester, .. } => {
                debug!(
                    repo_id = %repo_id.to_hex(),
                    count = hashes.len(),
                    requester = %requester.fmt_short(),
                    "deferred: received WantChanges, checking if we can help"
                );

                let _ = self.command_tx.send(SyncCommand::RespondWithChanges {
                    repo_id,
                    requested_hashes: hashes.clone(),
                });
            }

            PijulAnnouncement::ChannelUpdate { channel, new_head, .. } => {
                let mut pending = self.pending.write();

                if pending.should_check_channel(&repo_id, channel, new_head) {
                    pending.mark_channel_checked(&repo_id, channel, new_head);
                    drop(pending);

                    info!(
                        repo_id = %repo_id.to_hex(),
                        channel = %channel,
                        new_head = %new_head,
                        "deferred: received ChannelUpdate, queueing sync check"
                    );

                    let _ = self.command_tx.send(SyncCommand::CheckChannelSync {
                        repo_id,
                        channel: channel.clone(),
                        remote_head: *new_head,
                        announcer: *signer,
                    });
                }
            }

            PijulAnnouncement::ChangeAvailable { change_hash, .. } => {
                let mut pending = self.pending.write();

                if pending.should_request(change_hash) {
                    pending.mark_requested(*change_hash);
                    drop(pending);

                    debug!(
                        repo_id = %repo_id.to_hex(),
                        hash = %change_hash,
                        "deferred: received ChangeAvailable, requesting change"
                    );

                    let _ = self.command_tx.send(SyncCommand::RequestChange {
                        repo_id,
                        change_hash: *change_hash,
                        provider: *signer,
                    });
                }
            }

            // These announcements don't require sync actions
            PijulAnnouncement::Seeding { node_id, channels, .. } => {
                debug!(
                    repo_id = %repo_id.to_hex(),
                    node_id = %node_id.fmt_short(),
                    channels = ?channels,
                    "deferred: peer announced seeding"
                );
            }

            PijulAnnouncement::Unseeding { node_id, .. } => {
                debug!(
                    repo_id = %repo_id.to_hex(),
                    node_id = %node_id.fmt_short(),
                    "deferred: peer announced unseeding"
                );
            }

            PijulAnnouncement::RepoCreated {
                name,
                creator,
                default_channel,
                ..
            } => {
                info!(
                    repo_id = %repo_id.to_hex(),
                    name = %name,
                    creator = %creator.fmt_short(),
                    default_channel = %default_channel,
                    "deferred: new repository created"
                );
            }
        }
    }

    /// Handle downloading changes from a peer with parallel downloads.
    ///
    /// Downloads are performed concurrently with bounded parallelism using
    /// the `DownloadCoordinator`. Each download has retry logic with
    /// exponential backoff.
    #[instrument(skip(self, hashes), fields(repo_id = %repo_id.to_hex(), count = hashes.len()))]
    async fn handle_download_changes(&self, repo_id: &RepoId, hashes: &[ChangeHash], provider: PublicKey) {
        // Filter to changes we need to download
        let mut to_download = Vec::new();

        for hash in hashes {
            // Check if already being downloaded
            if self.download_coordinator.is_active(hash) {
                trace!(hash = %hash, "download already in progress, skipping");
                continue;
            }

            // Check if we already have this change
            match self.store.has_change(hash).await {
                Ok(true) => {
                    trace!(hash = %hash, "already have change, skipping download");
                    self.download_metrics.record_skipped();
                    // Still check if any channels were waiting for this change
                    self.apply_to_awaiting_channels(hash).await;
                    continue;
                }
                Ok(false) => {
                    to_download.push(*hash);
                }
                Err(e) => {
                    warn!(hash = %hash, error = %e, "failed to check if change exists");
                    continue;
                }
            }
        }

        if to_download.is_empty() {
            return;
        }

        debug!(
            count = to_download.len(),
            provider = %provider.fmt_short(),
            "starting parallel download of changes"
        );

        // Spawn parallel downloads with bounded concurrency
        // Returns (hash, retries_performed) on success
        let mut join_set: JoinSet<(ChangeHash, Result<u32, String>)> = JoinSet::new();

        for hash in to_download {
            // Mark as downloading in coordinator and pending
            self.download_coordinator.start(hash);
            self.pending.write().mark_downloading(hash);
            self.download_metrics.record_attempt();

            // Clone what we need for the spawned task
            let store = self.store.clone();
            let semaphore = self.download_coordinator.semaphore().clone();

            join_set.spawn(async move {
                // Acquire permit (bounded concurrency)
                let _permit = semaphore.acquire().await;

                // Download with retry
                let result = Self::download_with_retry(&store, &hash, provider).await;
                (hash, result)
            });
        }

        // Collect results
        while let Some(result) = join_set.join_next().await {
            match result {
                Ok((hash, Ok(retries))) => {
                    info!(hash = %hash, retries = retries, "successfully downloaded change");
                    self.download_coordinator.finish(&hash);
                    self.pending.write().mark_downloaded(&hash);
                    self.download_metrics.record_success(0, 0); // Size not available here

                    // Record retries if any
                    for _ in 0..retries {
                        self.download_metrics.record_retry();
                    }

                    // Check if any channels were waiting for this change
                    self.apply_to_awaiting_channels(&hash).await;
                }
                Ok((hash, Err(e))) => {
                    warn!(hash = %hash, error = %e, "failed to download change after retries");
                    self.download_coordinator.finish(&hash);
                    self.pending.write().mark_downloaded(&hash);
                    self.download_metrics.record_failure();
                }
                Err(join_err) => {
                    warn!(error = %join_err, "download task panicked");
                }
            }
        }
    }

    /// Download a change with exponential backoff retry.
    ///
    /// Retries up to `MAX_DOWNLOAD_RETRIES` times with exponential backoff
    /// starting at `DOWNLOAD_RETRY_INITIAL_BACKOFF_MS`.
    ///
    /// Returns Ok(retries) on success where retries is the number of retries performed.
    async fn download_with_retry(
        store: &PijulStore<B, K>,
        hash: &ChangeHash,
        provider: PublicKey,
    ) -> Result<u32, String> {
        let mut backoff_ms = DOWNLOAD_RETRY_INITIAL_BACKOFF_MS;

        for attempt in 0..MAX_DOWNLOAD_RETRIES {
            let start = Instant::now();

            match store.download_change(hash, provider).await {
                Ok(()) => {
                    let duration_ms = start.elapsed().as_millis() as u64;
                    debug!(
                        hash = %hash,
                        attempt = attempt + 1,
                        duration_ms = duration_ms,
                        "download succeeded"
                    );
                    return Ok(attempt);
                }
                Err(e) => {
                    if attempt + 1 < MAX_DOWNLOAD_RETRIES {
                        debug!(
                            hash = %hash,
                            attempt = attempt + 1,
                            max_attempts = MAX_DOWNLOAD_RETRIES,
                            backoff_ms = backoff_ms,
                            error = %e,
                            "download failed, retrying"
                        );

                        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;

                        // Exponential backoff with cap
                        backoff_ms = (backoff_ms * 2).min(DOWNLOAD_RETRY_MAX_BACKOFF_MS);
                    } else {
                        return Err(format!("all {} attempts failed: {}", MAX_DOWNLOAD_RETRIES, e));
                    }
                }
            }
        }

        Err("max retries exceeded".to_string())
    }

    /// Apply a downloaded change to any channels that were waiting for it.
    async fn apply_to_awaiting_channels(&self, hash: &ChangeHash) {
        let awaiting = self.pending.write().take_awaiting(hash);

        for update in awaiting {
            debug!(
                repo_id = %update.repo_id.to_hex(),
                channel = %update.channel,
                hash = %hash,
                "applying downloaded change to local pristine"
            );

            // Apply the downloaded change to the local pristine.
            // This is a local-only operation - no Raft queries needed since
            // the change blob is already in our store and refs are replicated via Raft.
            match self.store.apply_downloaded_change(&update.repo_id, &update.channel, hash).await {
                Ok(result) => {
                    if result.already_synced {
                        trace!(
                            repo_id = %update.repo_id.to_hex(),
                            channel = %update.channel,
                            "change already applied to pristine"
                        );
                    } else {
                        // Log sync result
                        info!(
                            repo_id = %update.repo_id.to_hex(),
                            channel = %update.channel,
                            applied = result.changes_applied,
                            "applied change to local pristine"
                        );

                        // Log any conflicts detected
                        if let Some(ref conflict_state) = result.conflicts {
                            if conflict_state.has_conflicts() {
                                warn!(
                                    repo_id = %update.repo_id.to_hex(),
                                    channel = %update.channel,
                                    conflict_count = conflict_state.conflict_count(),
                                    paths = ?conflict_state.conflicting_paths(),
                                    "conflicts detected after applying change"
                                );
                            }
                        }
                    }

                    // Clear the channel check mark so subsequent announcements can be processed.
                    // This allows rapid sequential changes to be synced correctly.
                    self.pending.write().clear_channel_check(&update.repo_id, &update.channel, hash);
                }
                Err(e) => {
                    warn!(
                        repo_id = %update.repo_id.to_hex(),
                        channel = %update.channel,
                        error = %e,
                        "failed to apply change to local pristine"
                    );

                    // Clear even on failure so we can retry on next announcement
                    self.pending.write().clear_channel_check(&update.repo_id, &update.channel, hash);
                }
            }
        }
    }

    /// Handle responding to a WantChanges request.
    #[instrument(skip(self, requested_hashes), fields(repo_id = %repo_id.to_hex(), count = requested_hashes.len()))]
    async fn handle_respond_with_changes(&self, repo_id: &RepoId, requested_hashes: &[ChangeHash]) {
        let mut have_hashes = Vec::new();

        for hash in requested_hashes {
            match self.store.has_change(hash).await {
                Ok(true) => {
                    have_hashes.push(*hash);
                }
                Ok(false) => {
                    trace!(hash = %hash, "don't have requested change");
                }
                Err(e) => {
                    warn!(hash = %hash, error = %e, "failed to check change existence");
                }
            }

            // Limit response size
            if have_hashes.len() >= MAX_CHANGES_PER_REQUEST as usize {
                break;
            }
        }

        if have_hashes.is_empty() {
            debug!("no requested changes available, not responding");
            return;
        }

        // Send HaveChanges response
        if let Err(e) = self.sync_service.offer_changes(repo_id, have_hashes.clone()).await {
            warn!(error = %e, "failed to send HaveChanges response");
        } else {
            debug!(count = have_hashes.len(), "sent HaveChanges response");
        }
    }

    /// Handle checking if we're behind on a channel.
    #[instrument(skip(self), fields(repo_id = %repo_id.to_hex(), channel = %channel))]
    async fn handle_check_channel_sync(
        &self,
        repo_id: &RepoId,
        channel: &str,
        remote_head: &ChangeHash,
        _announcer: PublicKey,
    ) {
        info!(
            remote_head = %remote_head,
            "handler: checking channel sync"
        );

        // Get our current head using local (stale) reads - this works on follower nodes
        // without needing to go through the Raft leader
        let our_head = match self.store.get_channel_local(repo_id, channel).await {
            Ok(Some(ch)) => ch.head,
            Ok(None) => {
                info!(remote_head = %remote_head, "handler: channel doesn't exist locally, requesting head change");
                // Record that this channel is waiting for the head change
                self.pending.write().await_change(*remote_head, *repo_id, channel.to_string());
                // Request the head change
                match self.sync_service.request_changes(repo_id, vec![*remote_head]).await {
                    Ok(()) => {
                        info!(remote_head = %remote_head, "handler: sent WantChanges request");
                    }
                    Err(e) => {
                        warn!(error = %e, "handler: failed to request channel head");
                    }
                }
                return;
            }
            Err(e) => {
                warn!(error = %e, "failed to get local channel head");
                return;
            }
        };

        // Even if we have the same channel head ref, we might not have the actual change data!
        // The ref is replicated through Raft, but the change files are stored locally.
        // Always check if we have the change data before assuming we're synced.
        let has_change = match self.store.has_change(remote_head).await {
            Ok(has) => has,
            Err(e) => {
                warn!(error = %e, "handler: failed to check if we have the change");
                false
            }
        };

        info!(
            local_head = ?our_head,
            remote_head = %remote_head,
            has_change = has_change,
            "handler: comparing heads"
        );

        if has_change {
            // We have the change data
            match our_head {
                Some(head) if head == *remote_head => {
                    info!("handler: already at remote head with change data");
                }
                Some(head) => {
                    info!(
                        local_head = %head,
                        remote_head = %remote_head,
                        "handler: have change, syncing pristine"
                    );
                    // We have the change but pristine might not be up to date
                    if let Err(e) = self.store.sync_channel_pristine(repo_id, channel).await {
                        warn!(error = %e, "handler: failed to sync channel pristine");
                    }
                }
                None => {
                    info!("handler: have change but local channel empty, syncing pristine");
                    if let Err(e) = self.store.sync_channel_pristine(repo_id, channel).await {
                        warn!(error = %e, "handler: failed to sync channel pristine");
                    }
                }
            }
            // Clear check mark so subsequent changes can be processed
            self.pending.write().clear_channel_check(repo_id, channel, remote_head);
        } else {
            // We don't have the change data - need to download it
            info!(remote_head = %remote_head, "handler: don't have change data, requesting");
            // Record that this channel is waiting for the remote head
            self.pending.write().await_change(*remote_head, *repo_id, channel.to_string());
            match self.sync_service.request_changes(repo_id, vec![*remote_head]).await {
                Ok(()) => {
                    info!(remote_head = %remote_head, "handler: sent WantChanges request");
                }
                Err(e) => {
                    warn!(error = %e, "handler: failed to request channel head");
                }
            }
        }
    }

    /// Handle requesting a specific change.
    #[instrument(skip(self), fields(repo_id = %repo_id.to_hex(), hash = %change_hash))]
    async fn handle_request_change(&self, repo_id: &RepoId, change_hash: &ChangeHash, provider: PublicKey) {
        // Check if we already have it
        match self.store.has_change(change_hash).await {
            Ok(true) => {
                trace!("already have change");
                // Still check if any channels were waiting for this change
                self.apply_to_awaiting_channels(change_hash).await;
                return;
            }
            Ok(false) => {}
            Err(e) => {
                warn!(error = %e, "failed to check change existence");
                return;
            }
        }

        // Download directly if we have a provider
        self.pending.write().mark_downloading(*change_hash);

        match self.store.download_change(change_hash, provider).await {
            Ok(()) => {
                info!("successfully downloaded requested change");
                self.pending.write().mark_downloaded(change_hash);

                // Check if any channels were waiting for this change and sync them
                self.apply_to_awaiting_channels(change_hash).await;
            }
            Err(e) => {
                warn!(error = %e, "failed to download requested change");
                self.pending.write().mark_downloaded(change_hash);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;

    use aspen_forge::identity::RepoId;

    use super::coordinator::DownloadCoordinator;
    use super::metrics::DownloadMetrics;
    use super::pending::PendingRequests;
    use crate::types::ChangeHash;

    #[test]
    fn test_pending_requests_dedup() {
        let mut pending = PendingRequests::new();
        let hash = ChangeHash([1u8; 32]);

        // Should initially allow request
        assert!(pending.should_request(&hash));

        // Mark as requested
        pending.mark_requested(hash);

        // Should now deny (within dedup window)
        assert!(!pending.should_request(&hash));
    }

    #[test]
    fn test_pending_requests_downloading() {
        let mut pending = PendingRequests::new();
        let hash = ChangeHash([2u8; 32]);

        // Mark as downloading
        pending.mark_downloading(hash);

        // Should deny while downloading
        assert!(!pending.should_request(&hash));

        // Mark as complete
        pending.mark_downloaded(&hash);

        // Should now allow (but mark_requested would still block)
        // Actually after download complete, dedup by requested time doesn't apply
        assert!(pending.should_request(&hash));
    }

    #[test]
    fn test_channel_check_dedup() {
        let mut pending = PendingRequests::new();
        let repo_id = RepoId::from_hash(blake3::hash(b"test"));
        let channel = "main";
        let head = ChangeHash([3u8; 32]);

        // Should initially allow
        assert!(pending.should_check_channel(&repo_id, channel, &head));

        // Mark as checked
        pending.mark_channel_checked(&repo_id, channel, &head);

        // Should now deny same head
        assert!(!pending.should_check_channel(&repo_id, channel, &head));

        // But a different head should be allowed
        let head2 = ChangeHash([4u8; 32]);
        assert!(pending.should_check_channel(&repo_id, channel, &head2));
    }

    #[test]
    fn test_download_coordinator_active_tracking() {
        let coordinator = DownloadCoordinator::new();
        let hash = ChangeHash([5u8; 32]);

        // Initially not active
        assert!(!coordinator.is_active(&hash));

        // Start download
        coordinator.start(hash);
        assert!(coordinator.is_active(&hash));

        // Finish download
        coordinator.finish(&hash);
        assert!(!coordinator.is_active(&hash));
    }

    #[test]
    fn test_download_coordinator_multiple_hashes() {
        let coordinator = DownloadCoordinator::new();
        let hash1 = ChangeHash([6u8; 32]);
        let hash2 = ChangeHash([7u8; 32]);
        let hash3 = ChangeHash([8u8; 32]);

        // Start multiple downloads
        coordinator.start(hash1);
        coordinator.start(hash2);

        assert!(coordinator.is_active(&hash1));
        assert!(coordinator.is_active(&hash2));
        assert!(!coordinator.is_active(&hash3));

        // Finish one
        coordinator.finish(&hash1);
        assert!(!coordinator.is_active(&hash1));
        assert!(coordinator.is_active(&hash2));
    }

    #[test]
    fn test_download_metrics_counters() {
        let metrics = DownloadMetrics::default();

        // Initial state
        assert_eq!(metrics.downloads_attempted.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.downloads_succeeded.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.downloads_failed.load(Ordering::Relaxed), 0);

        // Record operations
        metrics.record_attempt();
        metrics.record_attempt();
        metrics.record_success(1024, 100);
        metrics.record_failure();
        metrics.record_skipped();
        metrics.record_retry();

        assert_eq!(metrics.downloads_attempted.load(Ordering::Relaxed), 2);
        assert_eq!(metrics.downloads_succeeded.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.downloads_failed.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.downloads_skipped.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.bytes_downloaded.load(Ordering::Relaxed), 1024);
        assert_eq!(metrics.download_time_ms.load(Ordering::Relaxed), 100);
        assert_eq!(metrics.retries_performed.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_download_metrics_accumulation() {
        let metrics = DownloadMetrics::default();

        // Multiple successes should accumulate
        metrics.record_success(500, 50);
        metrics.record_success(700, 30);
        metrics.record_success(300, 20);

        assert_eq!(metrics.downloads_succeeded.load(Ordering::Relaxed), 3);
        assert_eq!(metrics.bytes_downloaded.load(Ordering::Relaxed), 1500);
        assert_eq!(metrics.download_time_ms.load(Ordering::Relaxed), 100);
    }
}
