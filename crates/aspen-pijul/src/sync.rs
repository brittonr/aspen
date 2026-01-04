//! PijulSyncService - P2P synchronization for Pijul repositories.
//!
//! This service manages gossip integration for Pijul, including:
//! - Subscribing to global and per-repo topics
//! - Broadcasting announcements when channels are updated
//! - Receiving and handling incoming announcements
//! - Triggering sync operations for seeded repos
//!
//! ## Architecture
//!
//! ```text
//! +------------------------------------------------------------------+
//! |                     PijulSyncService                             |
//! |  +--------------------+  +--------------------+                  |
//! |  | Gossip Subscriptions|  | Channel Listeners |                  |
//! |  | (global + per-repo) |  | (ChannelUpdateEvent)|               |
//! |  +--------------------+  +--------------------+                  |
//! |           |                        |                             |
//! |           v                        v                             |
//! |  +--------------------+  +--------------------+                  |
//! |  | Incoming Handler   |  | Outgoing Announcer |                  |
//! |  | (verify & dispatch)|  | (sign & broadcast) |                  |
//! |  +--------------------+  +--------------------+                  |
//! |           |                        |                             |
//! |           v                        v                             |
//! |  +--------------------------------------------+                  |
//! |  |           Sync Worker                       |                  |
//! |  | - Fetch missing changes from peers          |                  |
//! |  | - Apply to local pristine                   |                  |
//! |  +--------------------------------------------+                  |
//! +------------------------------------------------------------------+
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use aspen_forge::identity::RepoId;
use futures::StreamExt;
use iroh::PublicKey;
use iroh::SecretKey;
use iroh_gossip::net::Gossip;
use iroh_gossip::proto::TopicId;
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::info;
use tracing::trace;
use tracing::warn;

use super::constants::PIJUL_GOSSIP_MAX_STREAM_RETRIES;
use super::constants::PIJUL_GOSSIP_MAX_SUBSCRIBED_REPOS;
use super::constants::PIJUL_GOSSIP_STREAM_BACKOFF_SECS;
use super::constants::PIJUL_GOSSIP_SUBSCRIBE_TIMEOUT;
use super::error::PijulError;
use super::error::PijulResult;
use super::gossip::PijulAnnouncement;
use super::gossip::PijulTopic;
use super::gossip::SignedPijulAnnouncement;
use super::refs::ChannelUpdateEvent;

/// Sender half of a topic subscription.
type TopicSender = iroh_gossip::api::GossipSender;

/// Active topic subscription state.
struct TopicSubscription {
    sender: TopicSender,
    receiver_task: JoinHandle<()>,
}

/// Callback trait for handling incoming Pijul announcements.
///
/// Implement this to react to announcements (e.g., trigger sync).
pub trait PijulSyncCallback: Send + Sync + 'static {
    /// Called when a verified announcement is received.
    fn on_announcement(&self, announcement: &PijulAnnouncement, signer: &PublicKey);
}

/// PijulSyncService manages gossip integration for Pijul.
///
/// Provides:
/// - Global topic for discovery (RepoCreated, Seeding, Unseeding)
/// - Per-repo topics for updates (ChannelUpdate, ChangeAvailable)
/// - Automatic announcement broadcasting from channel events
/// - Incoming announcement handling
///
/// # Example
///
/// ```ignore
/// let service = PijulSyncService::spawn(
///     gossip,
///     secret_key,
///     channel_events,
///     handler,
/// ).await?;
///
/// // Subscribe to a specific repository
/// service.subscribe_repo(&repo_id).await?;
///
/// // Announce we're seeding
/// service.announce_seeding(&repo_id, &["main"]).await?;
///
/// // Shutdown gracefully
/// service.shutdown().await?;
/// ```
pub struct PijulSyncService {
    /// Iroh gossip instance.
    gossip: Arc<Gossip>,
    /// Secret key for signing announcements.
    secret_key: SecretKey,
    /// Our public key (node identity).
    node_id: PublicKey,
    /// Global topic subscription (for discovery).
    global_subscription: Mutex<Option<TopicSubscription>>,
    /// Per-repo topic subscriptions.
    repo_subscriptions: RwLock<HashMap<RepoId, TopicSubscription>>,
    /// Cancellation token for graceful shutdown.
    cancel_token: CancellationToken,
    /// Announcer task handle (listens to channel events).
    announcer_task: Mutex<Option<JoinHandle<()>>>,
    /// Handler for incoming announcements.
    handler: Option<Arc<dyn PijulSyncCallback>>,
}

impl PijulSyncService {
    /// Spawn a new PijulSyncService.
    ///
    /// # Arguments
    ///
    /// - `gossip`: Iroh gossip instance
    /// - `secret_key`: Secret key for signing announcements
    /// - `channel_events`: Receiver for channel update events from PijulRefStore
    /// - `handler`: Optional callback for handling incoming announcements
    pub async fn spawn(
        gossip: Arc<Gossip>,
        secret_key: SecretKey,
        channel_events: broadcast::Receiver<ChannelUpdateEvent>,
        handler: Option<Arc<dyn PijulSyncCallback>>,
    ) -> Result<Arc<Self>> {
        let node_id = secret_key.public();
        let cancel_token = CancellationToken::new();

        let service = Arc::new(Self {
            gossip,
            secret_key,
            node_id,
            global_subscription: Mutex::new(None),
            repo_subscriptions: RwLock::new(HashMap::new()),
            cancel_token: cancel_token.clone(),
            announcer_task: Mutex::new(None),
            handler,
        });

        // Subscribe to global topic
        service.subscribe_global().await?;

        // Spawn announcer task that listens to channel events
        let announcer_service = service.clone();
        let announcer_cancel = cancel_token.child_token();
        let announcer_task = tokio::spawn(async move {
            announcer_service.run_announcer(announcer_cancel, channel_events).await;
        });

        *service.announcer_task.lock().await = Some(announcer_task);

        info!("pijul sync service started");
        Ok(service)
    }

    /// Subscribe to the global Pijul gossip topic.
    async fn subscribe_global(&self) -> Result<()> {
        let topic = PijulTopic::global();
        let topic_id = topic.to_topic_id();

        let gossip_topic =
            tokio::time::timeout(PIJUL_GOSSIP_SUBSCRIBE_TIMEOUT, self.gossip.subscribe(topic_id, vec![]))
                .await
                .context("timeout subscribing to global pijul topic")?
                .context("failed to subscribe to global pijul topic")?;

        let (sender, receiver) = gossip_topic.split();

        // Spawn receiver task for global topic
        let receiver_task = self.spawn_receiver_task(topic_id, receiver, None);

        *self.global_subscription.lock().await = Some(TopicSubscription { sender, receiver_task });

        info!("subscribed to global pijul gossip topic");

        Ok(())
    }

    /// Subscribe to a repository-specific gossip topic.
    ///
    /// This enables receiving ChannelUpdate and ChangeAvailable announcements for the repo.
    ///
    /// # Arguments
    ///
    /// - `repo_id`: Repository to subscribe to
    /// - `bootstrap_peers`: Optional list of known peers interested in this repo
    ///
    /// # Errors
    ///
    /// - `PijulError::TooManyChannels` if at max subscribed repos
    /// - `PijulError::SyncFailed` if subscription fails
    pub async fn subscribe_repo(&self, repo_id: &RepoId) -> PijulResult<()> {
        self.subscribe_repo_with_peers(repo_id, vec![]).await
    }

    /// Subscribe to a repository-specific gossip topic with bootstrap peers.
    ///
    /// This enables receiving ChannelUpdate and ChangeAvailable announcements for the repo.
    /// Providing bootstrap peers allows immediate gossip connectivity.
    ///
    /// # Arguments
    ///
    /// - `repo_id`: Repository to subscribe to
    /// - `bootstrap_peers`: List of peer node IDs known to be interested in this repo
    pub async fn subscribe_repo_with_peers(
        &self,
        repo_id: &RepoId,
        bootstrap_peers: Vec<PublicKey>,
    ) -> PijulResult<()> {
        let subscriptions = self.repo_subscriptions.read().await;

        // Check if already subscribed
        if subscriptions.contains_key(repo_id) {
            return Ok(());
        }

        // Check limit
        if subscriptions.len() as u32 >= PIJUL_GOSSIP_MAX_SUBSCRIBED_REPOS {
            return Err(PijulError::TooManyChannels {
                count: subscriptions.len() as u32 + 1,
                max: PIJUL_GOSSIP_MAX_SUBSCRIBED_REPOS,
            });
        }

        drop(subscriptions); // Release read lock

        let topic = PijulTopic::for_repo(*repo_id);
        let topic_id = topic.to_topic_id();

        let gossip_topic =
            tokio::time::timeout(PIJUL_GOSSIP_SUBSCRIBE_TIMEOUT, self.gossip.subscribe(topic_id, bootstrap_peers))
                .await
                .map_err(|_| PijulError::SyncFailed {
                    message: "subscription timeout".to_string(),
                })?
                .map_err(|e| PijulError::SyncFailed { message: e.to_string() })?;

        let (sender, receiver) = gossip_topic.split();

        // Spawn receiver task for this repo topic
        let receiver_task = self.spawn_receiver_task(topic_id, receiver, Some(*repo_id));

        self.repo_subscriptions.write().await.insert(*repo_id, TopicSubscription { sender, receiver_task });

        info!(repo_id = %repo_id.to_hex(), "subscribed to repo pijul gossip topic");

        Ok(())
    }

    /// Unsubscribe from a repository-specific gossip topic.
    pub async fn unsubscribe_repo(&self, repo_id: &RepoId) -> PijulResult<()> {
        if let Some(sub) = self.repo_subscriptions.write().await.remove(repo_id) {
            sub.receiver_task.abort();
            info!(repo_id = %repo_id.to_hex(), "unsubscribed from repo pijul gossip topic");
        }

        Ok(())
    }

    /// Broadcast an announcement to the appropriate topic.
    ///
    /// - Global announcements (RepoCreated, Seeding, Unseeding) go to global topic
    /// - Per-repo announcements (ChannelUpdate, ChangeAvailable) go to repo topic
    pub async fn broadcast(&self, announcement: PijulAnnouncement) -> PijulResult<()> {
        let signed = SignedPijulAnnouncement::sign(announcement.clone(), &self.secret_key);
        let bytes = signed.to_bytes();

        if announcement.is_global() {
            // Broadcast to global topic
            let guard = self.global_subscription.lock().await;
            if let Some(ref sub) = *guard {
                sub.sender.broadcast(bytes.into()).await.map_err(|e| PijulError::SyncFailed {
                    message: format!("failed to broadcast to global topic: {}", e),
                })?;
            } else {
                return Err(PijulError::SyncFailed {
                    message: "gossip not initialized".to_string(),
                });
            }
        } else {
            // Broadcast to repo topic
            let repo_id = announcement.repo_id();
            let subscriptions = self.repo_subscriptions.read().await;

            if let Some(sub) = subscriptions.get(repo_id) {
                debug!(repo_id = %repo_id.to_hex(), "found subscription, broadcasting to repo topic");
                sub.sender.broadcast(bytes.into()).await.map_err(|e| PijulError::SyncFailed {
                    message: format!("failed to broadcast to repo topic: {}", e),
                })?;
            } else {
                // Not subscribed to this repo, skip broadcast
                warn!(repo_id = %repo_id.to_hex(), subscribed_count = subscriptions.len(), "skipping broadcast - not subscribed to repo");
            }
        }

        Ok(())
    }

    /// Announce that a new Pijul repository was created.
    pub async fn announce_repo_created(&self, repo_id: &RepoId, name: &str, default_channel: &str) -> PijulResult<()> {
        let announcement = PijulAnnouncement::RepoCreated {
            repo_id: *repo_id,
            name: name.to_string(),
            default_channel: default_channel.to_string(),
            creator: self.node_id,
        };

        self.broadcast(announcement).await
    }

    /// Announce that this node is seeding a Pijul repository.
    pub async fn announce_seeding(&self, repo_id: &RepoId, channels: &[&str]) -> PijulResult<()> {
        let announcement = PijulAnnouncement::Seeding {
            repo_id: *repo_id,
            node_id: self.node_id,
            channels: channels.iter().map(|s| s.to_string()).collect(),
        };

        self.broadcast(announcement).await
    }

    /// Announce that this node stopped seeding a Pijul repository.
    pub async fn announce_unseeding(&self, repo_id: &RepoId) -> PijulResult<()> {
        let announcement = PijulAnnouncement::Unseeding {
            repo_id: *repo_id,
            node_id: self.node_id,
        };

        self.broadcast(announcement).await
    }

    /// Announce that a change is available for download.
    pub async fn announce_change_available(
        &self,
        repo_id: &RepoId,
        change_hash: super::types::ChangeHash,
        size_bytes: u32,
        dependencies: Vec<super::types::ChangeHash>,
    ) -> PijulResult<()> {
        let announcement = PijulAnnouncement::ChangeAvailable {
            repo_id: *repo_id,
            change_hash,
            size_bytes,
            dependencies,
        };

        self.broadcast(announcement).await
    }

    /// Request missing changes from peers.
    pub async fn request_changes(&self, repo_id: &RepoId, hashes: Vec<super::types::ChangeHash>) -> PijulResult<()> {
        let announcement = PijulAnnouncement::WantChanges {
            repo_id: *repo_id,
            hashes,
            requester: self.node_id,
        };

        self.broadcast(announcement).await
    }

    /// Offer changes to peers.
    pub async fn offer_changes(&self, repo_id: &RepoId, hashes: Vec<super::types::ChangeHash>) -> PijulResult<()> {
        let announcement = PijulAnnouncement::HaveChanges {
            repo_id: *repo_id,
            hashes,
            offerer: self.node_id,
        };

        self.broadcast(announcement).await
    }

    /// Shutdown the sync service gracefully.
    pub async fn shutdown(self: Arc<Self>) -> Result<()> {
        info!("shutting down pijul sync service");

        // Signal cancellation
        self.cancel_token.cancel();

        let timeout = Duration::from_secs(10);

        // Shutdown announcer task
        if let Some(task) = self.announcer_task.lock().await.take() {
            tokio::select! {
                result = task => {
                    if let Err(e) = result {
                        warn!("announcer task panicked: {}", e);
                    }
                }
                _ = tokio::time::sleep(timeout) => {
                    warn!("announcer task did not complete within timeout");
                }
            }
        }

        // Shutdown global subscription
        if let Some(sub) = self.global_subscription.lock().await.take() {
            sub.receiver_task.abort();
        }

        // Shutdown all repo subscriptions
        let mut subscriptions = self.repo_subscriptions.write().await;
        for (repo_id, sub) in subscriptions.drain() {
            sub.receiver_task.abort();
            debug!(repo_id = %repo_id.to_hex(), "aborted repo receiver task");
        }

        Ok(())
    }

    /// Run the announcer task that listens to channel events.
    async fn run_announcer(
        &self,
        cancel: CancellationToken,
        mut channel_events: broadcast::Receiver<ChannelUpdateEvent>,
    ) {
        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    debug!("pijul gossip announcer shutting down");
                    break;
                }

                // Handle channel update events
                event = channel_events.recv() => {
                    match event {
                        Ok(channel_event) => {
                            let announcement = PijulAnnouncement::ChannelUpdate {
                                repo_id: channel_event.repo_id,
                                channel: channel_event.channel.clone(),
                                new_head: channel_event.new_head,
                                old_head: channel_event.old_head,
                                merkle: channel_event.merkle,
                            };

                            info!(
                                repo_id = %channel_event.repo_id.to_hex(),
                                channel = %channel_event.channel,
                                new_head = %channel_event.new_head,
                                "received channel update event, broadcasting"
                            );

                            if let Err(e) = self.broadcast(announcement).await {
                                warn!(
                                    "failed to broadcast channel update: {}",
                                    e
                                );
                            } else {
                                info!(
                                    repo_id = %channel_event.repo_id.to_hex(),
                                    channel = %channel_event.channel,
                                    "broadcast channel update announcement successfully"
                                );
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            warn!("channel events receiver lagged by {} messages", n);
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            debug!("channel events channel closed");
                        }
                    }
                }
            }
        }
    }

    /// Spawn a receiver task for a gossip topic.
    fn spawn_receiver_task(
        &self,
        topic_id: TopicId,
        mut receiver: iroh_gossip::api::GossipReceiver,
        repo_id: Option<RepoId>,
    ) -> JoinHandle<()> {
        let cancel = self.cancel_token.child_token();
        let node_id = self.node_id;
        let handler = self.handler.clone();

        tokio::spawn(async move {
            let mut consecutive_errors: u32 = 0;
            let backoff_durations: Vec<Duration> =
                PIJUL_GOSSIP_STREAM_BACKOFF_SECS.iter().map(|s| Duration::from_secs(*s)).collect();

            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        debug!("pijul gossip receiver shutting down");
                        break;
                    }

                    event = receiver.next() => match event {
                        Some(Ok(iroh_gossip::api::Event::Received(msg))) => {
                            consecutive_errors = 0;

                            // Parse signed announcement
                            let signed = match SignedPijulAnnouncement::from_bytes(&msg.content) {
                                Ok(s) => s,
                                Err(e) => {
                                    warn!("failed to parse pijul gossip message: {}", e);
                                    continue;
                                }
                            };

                            // Verify signature
                            let announcement = match signed.verify() {
                                Some(ann) => ann,
                                None => {
                                    warn!(
                                        "rejected pijul announcement with invalid signature from {:?}",
                                        signed.signer
                                    );
                                    continue;
                                }
                            };

                            // Filter self-announcements
                            if signed.signer == node_id {
                                trace!("ignoring self-announcement");
                                continue;
                            }

                            // Log based on announcement type
                            match announcement {
                                PijulAnnouncement::ChannelUpdate { repo_id, channel, new_head, .. } => {
                                    info!(
                                        repo_id = %repo_id.to_hex(),
                                        channel = %channel,
                                        new_head = %new_head,
                                        signer = %signed.signer.fmt_short(),
                                        "received ChannelUpdate announcement via gossip"
                                    );
                                }
                                PijulAnnouncement::ChangeAvailable { repo_id, change_hash, .. } => {
                                    info!(
                                        repo_id = %repo_id.to_hex(),
                                        change_hash = %change_hash,
                                        signer = %signed.signer.fmt_short(),
                                        "received ChangeAvailable announcement via gossip"
                                    );
                                }
                                PijulAnnouncement::Seeding { repo_id, node_id: seeder_id, .. } => {
                                    info!(
                                        repo_id = %repo_id.to_hex(),
                                        node_id = %seeder_id,
                                        "received Seeding announcement"
                                    );
                                }
                                PijulAnnouncement::Unseeding { repo_id, node_id: unseeder_id } => {
                                    info!(
                                        repo_id = %repo_id.to_hex(),
                                        node_id = %unseeder_id,
                                        "received Unseeding announcement"
                                    );
                                }
                                PijulAnnouncement::RepoCreated { repo_id, name, creator, .. } => {
                                    info!(
                                        repo_id = %repo_id.to_hex(),
                                        name = %name,
                                        creator = %creator,
                                        "received RepoCreated announcement"
                                    );
                                }
                                PijulAnnouncement::WantChanges { repo_id, hashes, requester } => {
                                    debug!(
                                        repo_id = %repo_id.to_hex(),
                                        count = hashes.len(),
                                        requester = %requester,
                                        "received WantChanges announcement"
                                    );
                                }
                                PijulAnnouncement::HaveChanges { repo_id, hashes, offerer } => {
                                    debug!(
                                        repo_id = %repo_id.to_hex(),
                                        count = hashes.len(),
                                        offerer = %offerer,
                                        "received HaveChanges announcement"
                                    );
                                }
                            }

                            // Call handler if registered
                            if let Some(ref h) = handler {
                                h.on_announcement(announcement, &signed.signer);
                            }
                        }

                        Some(Ok(iroh_gossip::api::Event::NeighborUp(neighbor))) => {
                            info!(repo_id = ?repo_id, neighbor = %neighbor.fmt_short(), "pijul gossip neighbor up");
                        }

                        Some(Ok(iroh_gossip::api::Event::NeighborDown(neighbor))) => {
                            info!(repo_id = ?repo_id, neighbor = %neighbor.fmt_short(), "pijul gossip neighbor down");
                        }

                        Some(Ok(iroh_gossip::api::Event::Lagged)) => {
                            warn!(topic = ?topic_id, "gossip receiver lagged");
                        }

                        Some(Err(e)) => {
                            consecutive_errors += 1;

                            if consecutive_errors > PIJUL_GOSSIP_MAX_STREAM_RETRIES {
                                warn!(
                                    "pijul gossip receiver exceeded max retries, giving up: {}",
                                    e
                                );
                                break;
                            }

                            let idx = (consecutive_errors as usize).saturating_sub(1);
                            let backoff = backoff_durations.get(idx).copied()
                                .unwrap_or_else(|| *backoff_durations.last().unwrap_or(&Duration::from_secs(16)));

                            warn!(
                                "pijul gossip receiver error (retry {}/{}), backing off {:?}: {}",
                                consecutive_errors,
                                PIJUL_GOSSIP_MAX_STREAM_RETRIES,
                                backoff,
                                e
                            );

                            tokio::time::sleep(backoff).await;
                        }

                        None => {
                            info!(topic = ?topic_id, repo_id = ?repo_id, "gossip stream ended");
                            break;
                        }
                    }
                }
            }
        })
    }

    /// Get the number of subscribed repos.
    #[cfg(test)]
    pub async fn subscribed_repo_count(&self) -> usize {
        self.repo_subscriptions.read().await.len()
    }

    /// Get our node ID.
    pub fn node_id(&self) -> &PublicKey {
        &self.node_id
    }
}

#[cfg(test)]
mod tests {
    // Tests would require mocking iroh-gossip which is complex.
    // Integration tests are more appropriate for this module.
}
