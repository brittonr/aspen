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

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use iroh::PublicKey;
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tracing::{debug, info, instrument, trace, warn};

use crate::api::KeyValueStore;
use crate::blob::BlobStore;
use crate::forge::identity::RepoId;

use super::constants::{
    MAX_CHANGES_PER_REQUEST, PIJUL_SYNC_DOWNLOAD_TIMEOUT_SECS,
    PIJUL_SYNC_REQUEST_DEDUP_SECS,
};
use super::gossip::PijulAnnouncement;
use super::store::PijulStore;
use super::sync::{PijulSyncCallback, PijulSyncService};
use super::types::ChangeHash;

/// Command sent to the async worker for processing.
enum SyncCommand {
    /// Download changes from a peer.
    DownloadChanges {
        repo_id: RepoId,
        hashes: Vec<ChangeHash>,
        provider: PublicKey,
    },
    /// Respond to a WantChanges request.
    RespondWithChanges {
        repo_id: RepoId,
        requested_hashes: Vec<ChangeHash>,
    },
    /// Check if we're behind on a channel and request updates.
    CheckChannelSync {
        repo_id: RepoId,
        channel: String,
        remote_head: ChangeHash,
        announcer: PublicKey,
    },
    /// Request a specific change that was announced.
    RequestChange {
        repo_id: RepoId,
        change_hash: ChangeHash,
        provider: PublicKey,
    },
}

/// Pending channel update waiting for a change to be downloaded.
#[derive(Debug, Clone)]
struct PendingChannelUpdate {
    /// Repository ID.
    repo_id: RepoId,
    /// Channel name.
    channel: String,
    /// When this update was recorded.
    recorded_at: Instant,
}

/// Tracks pending requests to avoid duplicate work.
struct PendingRequests {
    /// Changes currently being downloaded (hash -> request time).
    downloading: HashMap<ChangeHash, Instant>,
    /// Changes we've recently requested (hash -> request time).
    requested: HashMap<ChangeHash, Instant>,
    /// Repos we've checked for sync recently (repo_id:channel -> check time).
    channel_checks: HashMap<String, Instant>,
    /// Channel updates waiting for specific changes (hash -> channel update).
    /// When a change is downloaded, we check if any channel was waiting for it.
    awaiting_changes: HashMap<ChangeHash, Vec<PendingChannelUpdate>>,
}

impl PendingRequests {
    fn new() -> Self {
        Self {
            downloading: HashMap::new(),
            requested: HashMap::new(),
            channel_checks: HashMap::new(),
            awaiting_changes: HashMap::new(),
        }
    }

    /// Check if we should request this change (not already pending).
    fn should_request(&self, hash: &ChangeHash) -> bool {
        let now = Instant::now();
        let dedup_duration = Duration::from_secs(PIJUL_SYNC_REQUEST_DEDUP_SECS);

        // Check if already downloading
        if let Some(started) = self.downloading.get(hash) {
            if now.duration_since(*started) < Duration::from_secs(PIJUL_SYNC_DOWNLOAD_TIMEOUT_SECS) {
                return false;
            }
        }

        // Check if recently requested
        if let Some(requested) = self.requested.get(hash) {
            if now.duration_since(*requested) < dedup_duration {
                return false;
            }
        }

        true
    }

    /// Mark a change as being downloaded.
    fn mark_downloading(&mut self, hash: ChangeHash) {
        self.downloading.insert(hash, Instant::now());
    }

    /// Mark a change download as complete.
    fn mark_downloaded(&mut self, hash: &ChangeHash) {
        self.downloading.remove(hash);
    }

    /// Mark a change as requested.
    fn mark_requested(&mut self, hash: ChangeHash) {
        self.requested.insert(hash, Instant::now());
    }

    /// Check if we should check this channel for sync.
    fn should_check_channel(&self, repo_id: &RepoId, channel: &str) -> bool {
        let key = format!("{}:{}", repo_id.to_hex(), channel);
        let now = Instant::now();
        let dedup_duration = Duration::from_secs(PIJUL_SYNC_REQUEST_DEDUP_SECS);

        if let Some(checked) = self.channel_checks.get(&key) {
            if now.duration_since(*checked) < dedup_duration {
                return false;
            }
        }

        true
    }

    /// Mark a channel as checked.
    fn mark_channel_checked(&mut self, repo_id: &RepoId, channel: &str) {
        let key = format!("{}:{}", repo_id.to_hex(), channel);
        self.channel_checks.insert(key, Instant::now());
    }

    /// Record that a channel is waiting for a specific change.
    fn await_change(&mut self, hash: ChangeHash, repo_id: RepoId, channel: String) {
        let update = PendingChannelUpdate {
            repo_id,
            channel,
            recorded_at: Instant::now(),
        };
        self.awaiting_changes.entry(hash).or_default().push(update);
    }

    /// Take all channel updates waiting for a specific change.
    fn take_awaiting(&mut self, hash: &ChangeHash) -> Vec<PendingChannelUpdate> {
        self.awaiting_changes.remove(hash).unwrap_or_default()
    }

    /// Cleanup old entries.
    fn cleanup(&mut self) {
        let now = Instant::now();
        let timeout = Duration::from_secs(PIJUL_SYNC_DOWNLOAD_TIMEOUT_SECS);
        let dedup = Duration::from_secs(PIJUL_SYNC_REQUEST_DEDUP_SECS);

        self.downloading.retain(|_, t| now.duration_since(*t) < timeout);
        self.requested.retain(|_, t| now.duration_since(*t) < dedup);
        self.channel_checks.retain(|_, t| now.duration_since(*t) < dedup);
        // Cleanup old awaiting entries
        self.awaiting_changes.retain(|_, updates| {
            updates.retain(|u| now.duration_since(u.recorded_at) < timeout);
            !updates.is_empty()
        });
    }
}

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
    pending: RwLock<PendingRequests>,
    /// Command sender for the async worker.
    command_tx: mpsc::UnboundedSender<SyncCommand>,
    /// Repos we're interested in syncing.
    watched_repos: RwLock<HashSet<RepoId>>,
}

impl<B: BlobStore + 'static, K: KeyValueStore + 'static> PijulSyncHandler<B, K> {
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
        });

        // Spawn the worker task
        let handler_clone = handler.clone();
        let worker_handle = tokio::spawn(async move {
            while let Some(cmd) = command_rx.recv().await {
                match cmd {
                    SyncCommand::DownloadChanges { repo_id, hashes, provider } => {
                        handler_clone.handle_download_changes(&repo_id, &hashes, provider).await;
                    }
                    SyncCommand::RespondWithChanges { repo_id, requested_hashes } => {
                        handler_clone.handle_respond_with_changes(&repo_id, &requested_hashes).await;
                    }
                    SyncCommand::CheckChannelSync { repo_id, channel, remote_head, announcer } => {
                        handler_clone.handle_check_channel_sync(&repo_id, &channel, &remote_head, announcer).await;
                    }
                    SyncCommand::RequestChange { repo_id, change_hash, provider } => {
                        handler_clone.handle_request_change(&repo_id, &change_hash, provider).await;
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

    /// Handle downloading changes from a peer.
    #[instrument(skip(self, hashes), fields(repo_id = %repo_id.to_hex(), count = hashes.len()))]
    async fn handle_download_changes(
        &self,
        repo_id: &RepoId,
        hashes: &[ChangeHash],
        provider: PublicKey,
    ) {
        for hash in hashes {
            // Check if we already have this change
            match self.store.has_change(hash).await {
                Ok(true) => {
                    trace!(hash = %hash, "already have change, skipping download");
                    // Still check if any channels were waiting for this change
                    self.apply_to_awaiting_channels(hash).await;
                    continue;
                }
                Ok(false) => {}
                Err(e) => {
                    warn!(hash = %hash, error = %e, "failed to check if change exists");
                    continue;
                }
            }

            // Mark as downloading
            self.pending.write().mark_downloading(*hash);

            // Download from peer
            debug!(hash = %hash, provider = %provider.fmt_short(), "downloading change from peer");

            match self.store.download_change(hash, provider).await {
                Ok(()) => {
                    info!(hash = %hash, "successfully downloaded change");
                    self.pending.write().mark_downloaded(hash);

                    // Check if any channels were waiting for this change and sync them
                    self.apply_to_awaiting_channels(hash).await;
                }
                Err(e) => {
                    warn!(hash = %hash, error = %e, "failed to download change");
                    self.pending.write().mark_downloaded(hash);
                }
            }
        }
    }

    /// Apply a downloaded change to any channels that were waiting for it.
    async fn apply_to_awaiting_channels(&self, hash: &ChangeHash) {
        let awaiting = self.pending.write().take_awaiting(hash);

        for update in awaiting {
            debug!(
                repo_id = %update.repo_id.to_hex(),
                channel = %update.channel,
                hash = %hash,
                "syncing channel that was waiting for this change"
            );

            // Sync the channel's pristine and check for conflicts
            match self.store.sync_and_check_conflicts(&update.repo_id, &update.channel).await {
                Ok(result) => {
                    if result.already_synced {
                        trace!(
                            repo_id = %update.repo_id.to_hex(),
                            channel = %update.channel,
                            "channel already synced"
                        );
                    } else {
                        // Log sync result
                        info!(
                            repo_id = %update.repo_id.to_hex(),
                            channel = %update.channel,
                            applied = result.changes_applied,
                            "channel synced after download"
                        );

                        // Log any conflicts detected
                        if let Some(ref conflict_state) = result.conflicts {
                            if conflict_state.has_conflicts() {
                                warn!(
                                    repo_id = %update.repo_id.to_hex(),
                                    channel = %update.channel,
                                    conflict_count = conflict_state.conflict_count(),
                                    paths = ?conflict_state.conflicting_paths(),
                                    "conflicts detected after sync"
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        repo_id = %update.repo_id.to_hex(),
                        channel = %update.channel,
                        error = %e,
                        "failed to sync channel after download"
                    );
                }
            }
        }
    }

    /// Handle responding to a WantChanges request.
    #[instrument(skip(self, requested_hashes), fields(repo_id = %repo_id.to_hex(), count = requested_hashes.len()))]
    async fn handle_respond_with_changes(
        &self,
        repo_id: &RepoId,
        requested_hashes: &[ChangeHash],
    ) {
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
        // Get our current head
        let our_head = match self.store.get_channel(repo_id, channel).await {
            Ok(Some(ch)) => ch.head,
            Ok(None) => {
                debug!("channel doesn't exist locally, requesting head change");
                // Record that this channel is waiting for the head change
                self.pending.write().await_change(*remote_head, *repo_id, channel.to_string());
                // Request the head change
                if let Err(e) = self.sync_service.request_changes(repo_id, vec![*remote_head]).await {
                    warn!(error = %e, "failed to request channel head");
                }
                return;
            }
            Err(e) => {
                warn!(error = %e, "failed to get local channel head");
                return;
            }
        };

        match our_head {
            Some(head) if head == *remote_head => {
                trace!("already at remote head");
            }
            Some(head) => {
                debug!(
                    local_head = %head,
                    remote_head = %remote_head,
                    "local head differs from remote, checking if behind"
                );
                // We have a different head - might be behind or diverged
                // For now, just request the remote head if we don't have it
                match self.store.has_change(remote_head).await {
                    Ok(false) => {
                        debug!("requesting remote head change");
                        // Record that this channel is waiting for the remote head
                        self.pending.write().await_change(*remote_head, *repo_id, channel.to_string());
                        if let Err(e) = self.sync_service.request_changes(repo_id, vec![*remote_head]).await {
                            warn!(error = %e, "failed to request remote head");
                        }
                    }
                    Ok(true) => {
                        // We have the change but pristine might not be up to date
                        // Try to sync the pristine
                        debug!("have remote head change, syncing pristine");
                        if let Err(e) = self.store.sync_channel_pristine(repo_id, channel).await {
                            warn!(error = %e, "failed to sync channel pristine");
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, "failed to check remote head existence");
                    }
                }
            }
            None => {
                debug!("local channel is empty, requesting remote head");
                // Record that this channel is waiting for the head change
                self.pending.write().await_change(*remote_head, *repo_id, channel.to_string());
                if let Err(e) = self.sync_service.request_changes(repo_id, vec![*remote_head]).await {
                    warn!(error = %e, "failed to request channel head");
                }
            }
        }
    }

    /// Handle requesting a specific change.
    #[instrument(skip(self), fields(repo_id = %repo_id.to_hex(), hash = %change_hash))]
    async fn handle_request_change(
        &self,
        repo_id: &RepoId,
        change_hash: &ChangeHash,
        provider: PublicKey,
    ) {
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

impl<B: BlobStore + 'static, K: KeyValueStore + 'static> PijulSyncCallback for PijulSyncHandler<B, K> {
    fn on_announcement(&self, announcement: &PijulAnnouncement, signer: &PublicKey) {
        let repo_id = *announcement.repo_id();

        // Only process announcements for watched repos
        if !self.is_watching(&repo_id) {
            trace!(repo_id = %repo_id.to_hex(), "ignoring announcement for unwatched repo");
            return;
        }

        match announcement {
            PijulAnnouncement::HaveChanges { hashes, offerer, .. } => {
                // A peer has changes we might want
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
                        "queueing download of offered changes"
                    );

                    let _ = self.command_tx.send(SyncCommand::DownloadChanges {
                        repo_id,
                        hashes: to_download,
                        provider: *offerer,
                    });
                }
            }

            PijulAnnouncement::WantChanges { hashes, requester, .. } => {
                // A peer wants changes - check if we have them
                debug!(
                    repo_id = %repo_id.to_hex(),
                    count = hashes.len(),
                    requester = %requester.fmt_short(),
                    "received WantChanges, checking if we can help"
                );

                let _ = self.command_tx.send(SyncCommand::RespondWithChanges {
                    repo_id,
                    requested_hashes: hashes.clone(),
                });
            }

            PijulAnnouncement::ChannelUpdate { channel, new_head, .. } => {
                // A channel was updated - check if we're behind
                let mut pending = self.pending.write();

                if pending.should_check_channel(&repo_id, channel) {
                    pending.mark_channel_checked(&repo_id, channel);
                    drop(pending);

                    debug!(
                        repo_id = %repo_id.to_hex(),
                        channel = %channel,
                        new_head = %new_head,
                        "received ChannelUpdate, checking if we need to sync"
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
                // A new change is available
                let mut pending = self.pending.write();

                if pending.should_request(change_hash) {
                    pending.mark_requested(*change_hash);
                    drop(pending);

                    debug!(
                        repo_id = %repo_id.to_hex(),
                        hash = %change_hash,
                        "received ChangeAvailable, requesting change"
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
                    "peer announced seeding"
                );
            }

            PijulAnnouncement::Unseeding { node_id, .. } => {
                debug!(
                    repo_id = %repo_id.to_hex(),
                    node_id = %node_id.fmt_short(),
                    "peer announced unseeding"
                );
            }

            PijulAnnouncement::RepoCreated { name, creator, default_channel, .. } => {
                info!(
                    repo_id = %repo_id.to_hex(),
                    name = %name,
                    creator = %creator.fmt_short(),
                    default_channel = %default_channel,
                    "new repository created"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        // Should initially allow
        assert!(pending.should_check_channel(&repo_id, channel));

        // Mark as checked
        pending.mark_channel_checked(&repo_id, channel);

        // Should now deny
        assert!(!pending.should_check_channel(&repo_id, channel));
    }
}
