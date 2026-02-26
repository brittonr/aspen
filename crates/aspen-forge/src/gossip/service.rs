//! ForgeGossipService - core gossip integration for Forge.
//!
//! This service manages gossip lifecycle for Forge, including:
//! - Subscribing to global and per-repo topics
//! - Broadcasting announcements when refs/COBs are updated
//! - Receiving and handling incoming announcements
//! - Triggering sync operations for seeded repos

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use iroh::PublicKey;
use iroh::SecretKey;
use iroh_gossip::net::Gossip;
use iroh_gossip::proto::TopicId;
use n0_future::StreamExt;
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use super::rate_limiter::ForgeGossipRateLimiter;
use super::types::Announcement;
use super::types::ForgeTopic;
use super::types::SignedAnnouncement;
use crate::cob::CobUpdateEvent;
use crate::constants::FORGE_GOSSIP_ANNOUNCE_FAILURE_THRESHOLD;
use crate::constants::FORGE_GOSSIP_ANNOUNCE_INTERVAL;
use crate::constants::FORGE_GOSSIP_MAX_ANNOUNCE_INTERVAL;
use crate::constants::FORGE_GOSSIP_MAX_STREAM_RETRIES;
use crate::constants::FORGE_GOSSIP_MAX_SUBSCRIBED_REPOS;
use crate::constants::FORGE_GOSSIP_STREAM_BACKOFF_SECS;
use crate::constants::FORGE_GOSSIP_SUBSCRIBE_TIMEOUT;
use crate::error::ForgeError;
use crate::error::ForgeResult;
use crate::identity::RepoId;
use crate::refs::RefUpdateEvent;

/// Sender half of a topic subscription.
type TopicSender = iroh_gossip::api::GossipSender;

// ============================================================================
// Helper functions for spawn_receiver_task (extracted for Tiger Style compliance)
// ============================================================================

/// Process a received gossip message: rate limit, parse, verify, and dispatch.
async fn spawn_receiver_process_message(
    msg: &iroh_gossip::api::Message,
    rate_limiter: &mut ForgeGossipRateLimiter,
    node_id: PublicKey,
    handler: &Arc<RwLock<Option<Arc<dyn AnnouncementCallback>>>>,
) {
    // Rate limit check BEFORE parsing (save CPU)
    if let Err(reason) = rate_limiter.check(&msg.delivered_from) {
        tracing::trace!("rate limited forge gossip from {:?}: {:?}", msg.delivered_from, reason);
        return;
    }

    // Parse signed announcement
    let signed = match SignedAnnouncement::from_bytes(&msg.content) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("failed to parse forge gossip message: {}", e);
            return;
        }
    };

    // Verify signature
    let announcement = match signed.verify() {
        Some(ann) => ann,
        None => {
            tracing::warn!("rejected forge announcement with invalid signature from {:?}", signed.signer);
            return;
        }
    };

    let is_self_announcement = signed.signer == node_id;

    // Call handler for ALL announcements (including self).
    // This is critical for CI triggers which need to fire for local pushes.
    if let Some(ref h) = *handler.read().await {
        tracing::debug!(signer = %signed.signer, is_self = is_self_announcement, "calling announcement handler");
        h.on_announcement(announcement, &signed.signer);
    } else {
        tracing::debug!("no announcement handler registered");
    }

    // Skip logging for self-announcements (we already know about them)
    if is_self_announcement {
        tracing::trace!("processed self-announcement (handler called)");
        return;
    }

    // Log based on announcement type
    spawn_receiver_log_announcement(announcement, &signed.signer);
}

/// Log a received announcement based on its type.
fn spawn_receiver_log_announcement(announcement: &Announcement, signer: &PublicKey) {
    match announcement {
        Announcement::RefUpdate { repo_id, ref_name, .. } => {
            tracing::debug!(
                repo_id = %repo_id.to_hex(),
                ref_name = %ref_name,
                signer = %signer,
                "received RefUpdate announcement"
            );
        }
        Announcement::CobChange { repo_id, cob_type, .. } => {
            tracing::debug!(
                repo_id = %repo_id.to_hex(),
                cob_type = ?cob_type,
                signer = %signer,
                "received CobChange announcement"
            );
        }
        Announcement::Seeding {
            repo_id,
            node_id: seeder_id,
        } => {
            tracing::info!(repo_id = %repo_id.to_hex(), node_id = %seeder_id, "received Seeding announcement");
        }
        Announcement::Unseeding {
            repo_id,
            node_id: unseeder_id,
        } => {
            tracing::info!(repo_id = %repo_id.to_hex(), node_id = %unseeder_id, "received Unseeding announcement");
        }
        Announcement::RepoCreated { repo_id, name, creator } => {
            tracing::info!(
                repo_id = %repo_id.to_hex(),
                name = %name,
                creator = %creator,
                "received RepoCreated announcement"
            );
        }
    }
}

/// Handle a stream error with backoff. Returns true if the loop should break.
async fn spawn_receiver_handle_error(
    consecutive_errors: u32,
    backoff_durations: &[Duration],
    error: &iroh_gossip::api::ApiError,
) -> bool {
    if consecutive_errors > FORGE_GOSSIP_MAX_STREAM_RETRIES {
        tracing::error!("forge gossip receiver exceeded max retries, giving up: {}", error);
        return true;
    }

    let idx = (consecutive_errors as usize).saturating_sub(1);
    let backoff = backoff_durations
        .get(idx)
        .copied()
        .unwrap_or_else(|| *backoff_durations.last().unwrap_or(&Duration::from_secs(16)));

    tracing::warn!(
        "forge gossip receiver error (retry {}/{}), backing off {:?}: {}",
        consecutive_errors,
        FORGE_GOSSIP_MAX_STREAM_RETRIES,
        backoff,
        error
    );

    tokio::time::sleep(backoff).await;
    false
}

/// Active topic subscription state.
struct TopicSubscription {
    sender: TopicSender,
    receiver_task: JoinHandle<()>,
}

/// ForgeGossipService manages gossip integration for Forge.
///
/// Provides:
/// - Global topic for discovery (RepoCreated, Seeding, Unseeding)
/// - Per-repo topics for updates (RefUpdate, CobChange)
/// - Automatic announcement broadcasting from store events
/// - Incoming announcement handling with rate limiting
///
/// # Example
///
/// ```ignore
/// let service = ForgeGossipService::spawn(
///     gossip,
///     secret_key,
///     ref_events,
///     cob_events,
///     handler,
/// ).await?;
///
/// // Subscribe to a specific repository
/// service.subscribe_repo(&repo_id).await?;
///
/// // Announce we're seeding
/// service.announce_seeding(&repo_id).await?;
///
/// // Shutdown gracefully
/// service.shutdown().await?;
/// ```
pub struct ForgeGossipService {
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
    /// Announcer task handle (listens to store events).
    announcer_task: Mutex<Option<JoinHandle<()>>>,
    /// Rate limiter for outgoing announcements.
    #[allow(dead_code)]
    rate_limiter: Mutex<ForgeGossipRateLimiter>,
    /// Handler for incoming announcements.
    /// Uses Arc<RwLock<...>> so it can be shared with spawned tasks and updated later.
    handler: Arc<RwLock<Option<Arc<dyn AnnouncementCallback>>>>,
}

/// Callback trait for handling incoming announcements.
///
/// Implement this to react to announcements (e.g., trigger sync).
pub trait AnnouncementCallback: Send + Sync + 'static {
    /// Called when a verified announcement is received.
    fn on_announcement(&self, announcement: &Announcement, signer: &PublicKey);
}

impl ForgeGossipService {
    /// Spawn a new ForgeGossipService.
    ///
    /// # Arguments
    ///
    /// - `gossip`: Iroh gossip instance
    /// - `secret_key`: Secret key for signing announcements
    /// - `ref_events`: Receiver for ref update events from RefStore
    /// - `cob_events`: Receiver for COB update events from CobStore
    /// - `handler`: Optional callback for handling incoming announcements
    pub async fn spawn(
        gossip: Arc<Gossip>,
        secret_key: SecretKey,
        ref_events: broadcast::Receiver<RefUpdateEvent>,
        cob_events: broadcast::Receiver<CobUpdateEvent>,
        handler: Option<Arc<dyn AnnouncementCallback>>,
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
            rate_limiter: Mutex::new(ForgeGossipRateLimiter::new()),
            handler: Arc::new(RwLock::new(handler)),
        });

        // Subscribe to global topic
        service.subscribe_global().await?;

        // Spawn announcer task that listens to store events
        let announcer_service = service.clone();
        let announcer_cancel = cancel_token.child_token();
        let announcer_task = tokio::spawn(async move {
            announcer_service.run_announcer(announcer_cancel, ref_events, cob_events).await;
        });

        *service.announcer_task.lock().await = Some(announcer_task);

        Ok(service)
    }

    /// Subscribe to the global Forge gossip topic.
    async fn subscribe_global(&self) -> Result<()> {
        let topic = ForgeTopic::global();
        let topic_id = topic.to_topic_id();

        let gossip_topic =
            tokio::time::timeout(FORGE_GOSSIP_SUBSCRIBE_TIMEOUT, self.gossip.subscribe(topic_id, vec![]))
                .await
                .context("timeout subscribing to global forge topic")?
                .context("failed to subscribe to global forge topic")?;

        let (sender, receiver) = gossip_topic.split();

        // Spawn receiver task for global topic
        let receiver_task = self.spawn_receiver_task(topic_id, receiver, None);

        *self.global_subscription.lock().await = Some(TopicSubscription { sender, receiver_task });

        tracing::info!("subscribed to global forge gossip topic");

        Ok(())
    }

    /// Set or replace the announcement handler.
    ///
    /// This can be called after the service is created to register a handler
    /// for incoming announcements (e.g., CI trigger handler).
    ///
    /// The handler is used by all existing and future receiver tasks.
    pub async fn set_handler(&self, handler: Option<Arc<dyn AnnouncementCallback>>) {
        *self.handler.write().await = handler;
        tracing::debug!("announcement handler updated");
    }

    /// Subscribe to a repository-specific gossip topic.
    ///
    /// This enables receiving RefUpdate and CobChange announcements for the repo.
    ///
    /// # Errors
    ///
    /// - `ForgeError::TooManySubscriptions` if at max subscribed repos
    /// - `ForgeError::GossipTopicError` if subscription fails
    pub async fn subscribe_repo(&self, repo_id: &RepoId) -> ForgeResult<()> {
        let subscriptions = self.repo_subscriptions.read().await;

        // Check if already subscribed
        if subscriptions.contains_key(repo_id) {
            return Ok(());
        }

        // Check limit
        if subscriptions.len() as u32 >= FORGE_GOSSIP_MAX_SUBSCRIBED_REPOS {
            return Err(ForgeError::TooManySubscriptions {
                count: subscriptions.len() as u32 + 1,
                max: FORGE_GOSSIP_MAX_SUBSCRIBED_REPOS,
            });
        }

        drop(subscriptions); // Release read lock

        let topic = ForgeTopic::for_repo(*repo_id);
        let topic_id = topic.to_topic_id();

        let gossip_topic =
            tokio::time::timeout(FORGE_GOSSIP_SUBSCRIBE_TIMEOUT, self.gossip.subscribe(topic_id, vec![]))
                .await
                .map_err(|_| ForgeError::GossipTopicError {
                    topic: hex::encode(topic_id.as_bytes()),
                    message: "subscription timeout".to_string(),
                })?
                .map_err(|e| ForgeError::GossipTopicError {
                    topic: hex::encode(topic_id.as_bytes()),
                    message: e.to_string(),
                })?;

        let (sender, receiver) = gossip_topic.split();

        // Spawn receiver task for this repo topic
        let receiver_task = self.spawn_receiver_task(topic_id, receiver, Some(*repo_id));

        self.repo_subscriptions.write().await.insert(*repo_id, TopicSubscription { sender, receiver_task });

        tracing::info!(repo_id = %repo_id.to_hex(), "subscribed to repo forge gossip topic");

        Ok(())
    }

    /// Unsubscribe from a repository-specific gossip topic.
    pub async fn unsubscribe_repo(&self, repo_id: &RepoId) -> ForgeResult<()> {
        if let Some(sub) = self.repo_subscriptions.write().await.remove(repo_id) {
            sub.receiver_task.abort();
            tracing::info!(repo_id = %repo_id.to_hex(), "unsubscribed from repo forge gossip topic");
        }

        Ok(())
    }

    /// Broadcast an announcement to the appropriate topic.
    ///
    /// - Global announcements (RepoCreated, Seeding, Unseeding) go to global topic
    /// - Per-repo announcements (RefUpdate, CobChange) go to repo topic
    pub async fn broadcast(&self, announcement: Announcement) -> ForgeResult<()> {
        let signed = SignedAnnouncement::sign(announcement.clone(), &self.secret_key);
        let bytes = signed.to_bytes();

        // Determine which topic to use
        let is_global = matches!(
            announcement,
            Announcement::RepoCreated { .. } | Announcement::Seeding { .. } | Announcement::Unseeding { .. }
        );

        if is_global {
            // Broadcast to global topic
            // CRITICAL: Clone sender BEFORE awaiting to avoid deadlock.
            // Holding a lock across an async await point can cause deadlock
            // when other tasks try to acquire the lock.
            let sender = {
                let guard = self.global_subscription.lock().await;
                (*guard).as_ref().map(|sub| sub.sender.clone())
            }; // Lock released here

            match sender {
                Some(sender) => {
                    sender.broadcast(bytes.into()).await.map_err(|e| ForgeError::GossipError {
                        message: format!("failed to broadcast to global topic: {}", e),
                    })?;
                }
                None => {
                    return Err(ForgeError::GossipNotInitialized);
                }
            }
        } else {
            // Broadcast to repo topic
            // CRITICAL: Clone sender BEFORE awaiting to avoid deadlock.
            // Holding a RwLock across an async await point blocks write lock
            // acquisitions and can cause deadlock with subscribe_repo().
            let repo_id = announcement.repo_id();
            let sender = {
                let subscriptions = self.repo_subscriptions.read().await;
                subscriptions.get(repo_id).map(|sub| sub.sender.clone())
            }; // Lock released here

            if let Some(sender) = sender {
                sender.broadcast(bytes.into()).await.map_err(|e| ForgeError::GossipError {
                    message: format!("failed to broadcast to repo topic: {}", e),
                })?;
            } else {
                // Not subscribed to this repo, skip broadcast
                tracing::trace!(repo_id = %repo_id.to_hex(), "skipping broadcast - not subscribed to repo");
            }
        }

        Ok(())
    }

    /// Announce that a new repository was created.
    pub async fn announce_repo_created(&self, repo_id: &RepoId, name: &str) -> ForgeResult<()> {
        let announcement = Announcement::RepoCreated {
            repo_id: *repo_id,
            name: name.to_string(),
            creator: self.node_id,
        };

        self.broadcast(announcement).await
    }

    /// Announce that this node is seeding a repository.
    pub async fn announce_seeding(&self, repo_id: &RepoId) -> ForgeResult<()> {
        let announcement = Announcement::Seeding {
            repo_id: *repo_id,
            node_id: self.node_id,
        };

        self.broadcast(announcement).await
    }

    /// Announce that this node stopped seeding a repository.
    pub async fn announce_unseeding(&self, repo_id: &RepoId) -> ForgeResult<()> {
        let announcement = Announcement::Unseeding {
            repo_id: *repo_id,
            node_id: self.node_id,
        };

        self.broadcast(announcement).await
    }

    /// Shutdown the gossip service gracefully.
    pub async fn shutdown(self: Arc<Self>) -> Result<()> {
        tracing::info!("shutting down forge gossip service");

        // Signal cancellation
        self.cancel_token.cancel();

        let timeout = Duration::from_secs(10);

        // Shutdown announcer task
        if let Some(task) = self.announcer_task.lock().await.take() {
            tokio::select! {
                result = task => {
                    if let Err(e) = result {
                        tracing::error!("announcer task panicked: {}", e);
                    }
                }
                _ = tokio::time::sleep(timeout) => {
                    tracing::warn!("announcer task did not complete within timeout");
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
            tracing::debug!(repo_id = %repo_id.to_hex(), "aborted repo receiver task");
        }

        Ok(())
    }

    /// Handle a ref update event by invoking handler and broadcasting.
    ///
    /// Returns the new consecutive failure count.
    async fn run_announcer_handle_ref_event(&self, ref_event: RefUpdateEvent, consecutive_failures: u32) -> u32 {
        tracing::debug!(
            repo_id = %ref_event.repo_id.to_hex(),
            ref_name = %ref_event.ref_name,
            "announcer received RefUpdateEvent, broadcasting"
        );

        let announcement = Announcement::RefUpdate {
            repo_id: ref_event.repo_id,
            ref_name: ref_event.ref_name.clone(),
            new_hash: *ref_event.new_hash.as_bytes(),
            old_hash: ref_event.old_hash.map(|h| *h.as_bytes()),
        };

        // Call handler directly for local events.
        // Gossip doesn't deliver messages back to the sender,
        // so we need to invoke the handler here for local triggers (like CI).
        if let Some(ref h) = *self.handler.read().await {
            tracing::info!(
                repo_id = %ref_event.repo_id.to_hex(),
                ref_name = %ref_event.ref_name,
                "invoking CI handler for local RefUpdate"
            );
            h.on_announcement(&announcement, &self.node_id);
        } else {
            tracing::warn!(
                repo_id = %ref_event.repo_id.to_hex(),
                ref_name = %ref_event.ref_name,
                "no handler registered for RefUpdate - CI will not trigger"
            );
        }

        if let Err(e) = self.broadcast(announcement).await {
            let new_failures = consecutive_failures + 1;
            tracing::warn!("failed to broadcast ref update (failure {}): {}", new_failures, e);

            if new_failures >= FORGE_GOSSIP_ANNOUNCE_FAILURE_THRESHOLD {
                let backoff = FORGE_GOSSIP_MAX_ANNOUNCE_INTERVAL.min(FORGE_GOSSIP_ANNOUNCE_INTERVAL * new_failures);
                tokio::time::sleep(backoff).await;
            }
            new_failures
        } else {
            tracing::debug!(
                repo_id = %ref_event.repo_id.to_hex(),
                ref_name = %ref_event.ref_name,
                "broadcast ref update announcement to gossip"
            );
            0
        }
    }

    /// Handle a COB update event by broadcasting.
    ///
    /// Returns the new consecutive failure count.
    async fn run_announcer_handle_cob_event(&self, cob_event: CobUpdateEvent, consecutive_failures: u32) -> u32 {
        let announcement = Announcement::CobChange {
            repo_id: cob_event.repo_id,
            cob_type: cob_event.cob_type,
            cob_id: *cob_event.cob_id.as_bytes(),
            change_hash: *cob_event.change_hash.as_bytes(),
        };

        if let Err(e) = self.broadcast(announcement).await {
            let new_failures = consecutive_failures + 1;
            tracing::warn!("failed to broadcast COB change (failure {}): {}", new_failures, e);
            new_failures
        } else {
            tracing::trace!(
                repo_id = %cob_event.repo_id.to_hex(),
                cob_type = ?cob_event.cob_type,
                "broadcast COB change announcement"
            );
            0
        }
    }

    /// Run the announcer task that listens to store events.
    async fn run_announcer(
        &self,
        cancel: CancellationToken,
        mut ref_events: broadcast::Receiver<RefUpdateEvent>,
        mut cob_events: broadcast::Receiver<CobUpdateEvent>,
    ) {
        let mut consecutive_failures: u32 = 0;

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    tracing::debug!("forge gossip announcer shutting down");
                    break;
                }

                event = ref_events.recv() => {
                    match event {
                        Ok(ref_event) => {
                            consecutive_failures = self.run_announcer_handle_ref_event(ref_event, consecutive_failures).await;
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!("ref events receiver lagged by {} messages", n);
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            tracing::debug!("ref events channel closed");
                        }
                    }
                }

                event = cob_events.recv() => {
                    match event {
                        Ok(cob_event) => {
                            consecutive_failures = self.run_announcer_handle_cob_event(cob_event, consecutive_failures).await;
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!("COB events receiver lagged by {} messages", n);
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            tracing::debug!("COB events channel closed");
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
        let handler = Arc::clone(&self.handler);

        tokio::spawn(async move {
            let mut rate_limiter = ForgeGossipRateLimiter::new();
            let mut consecutive_errors: u32 = 0;
            let backoff_durations: Vec<Duration> =
                FORGE_GOSSIP_STREAM_BACKOFF_SECS.iter().map(|s| Duration::from_secs(*s)).collect();

            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        tracing::debug!("forge gossip receiver shutting down");
                        break;
                    }

                    event = receiver.next() => match event {
                        Some(Ok(iroh_gossip::api::Event::Received(msg))) => {
                            consecutive_errors = 0;
                            spawn_receiver_process_message(&msg, &mut rate_limiter, node_id, &handler).await;
                        }

                        Some(Ok(iroh_gossip::api::Event::NeighborUp(neighbor))) => {
                            tracing::debug!(topic = ?topic_id, neighbor = ?neighbor, "neighbor up");
                        }

                        Some(Ok(iroh_gossip::api::Event::NeighborDown(neighbor))) => {
                            tracing::debug!(topic = ?topic_id, neighbor = ?neighbor, "neighbor down");
                        }

                        Some(Ok(iroh_gossip::api::Event::Lagged)) => {
                            tracing::warn!(topic = ?topic_id, "gossip receiver lagged");
                        }

                        Some(Err(e)) => {
                            consecutive_errors += 1;
                            let should_break = spawn_receiver_handle_error(
                                consecutive_errors,
                                &backoff_durations,
                                &e,
                            ).await;
                            if should_break {
                                break;
                            }
                        }

                        None => {
                            tracing::info!(topic = ?topic_id, repo_id = ?repo_id, "gossip stream ended");
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
}

#[cfg(test)]
mod tests {
    // Tests would require mocking iroh-gossip which is complex.
    // Integration tests are more appropriate for this module.
}
