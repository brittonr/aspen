//! Core gossip peer discovery implementation.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use aspen_core::DiscoveredPeer;
use aspen_core::DiscoveryHandle;
use aspen_core::PeerDiscoveredCallback;
use aspen_core::PeerDiscovery;
use aspen_raft_types::NodeId;
use async_trait::async_trait;
use futures::StreamExt;
use iroh::EndpointAddr;
use iroh::SecretKey;
use iroh_gossip::api::Event;
use iroh_gossip::net::Gossip;
use iroh_gossip::proto::TopicId;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;

use crate::constants::*;
use crate::rate_limiter::GossipRateLimiter;
use crate::rate_limiter::RateLimitReason;
use crate::types::*;

/// Pure function for calculating backoff duration.
///
/// Tiger Style: Pure function for testability and predictable behavior.
pub fn calculate_backoff_duration(restart_count: usize, backoff_durations: &[Duration]) -> Duration {
    let idx = restart_count.min(backoff_durations.len().saturating_sub(1));
    backoff_durations[idx]
}

/// Manages gossip-based peer discovery lifecycle.
///
/// Spawns two background tasks:
/// 1. Announcer: Periodically broadcasts this node's ID and EndpointAddr
/// 2. Receiver: Listens for peer announcements and automatically adds them to the network factory
///
/// Tiger Style: Bounded announcement interval (10s), explicit shutdown mechanism.
///
/// ## Task Lifecycle Management
///
/// - Uses CancellationToken for clean shutdown coordination
/// - Implements Drop to abort tasks if struct dropped without shutdown()
/// - Tasks check cancellation token for responsive shutdown
/// - Bounded shutdown timeout (10s) with explicit task abortion
///
/// ## PeerDiscovery Trait
///
/// This struct implements the `PeerDiscovery` trait for trait-based discovery abstraction.
/// Use `new()` + `start()` for the trait-based API, or `spawn()` for the legacy API.
pub struct GossipPeerDiscovery {
    topic_id: TopicId,
    node_id: NodeId,
    // Config stored for announce() and start()
    gossip: Arc<Gossip>,
    endpoint_addr: EndpointAddr,
    secret_key: SecretKey,
    // Running state tracking
    is_running: Arc<AtomicBool>,
    cancel_token: CancellationToken,
    // Tiger Style: Option allows moving out in shutdown() while still implementing Drop
    // Mutex for interior mutability (start() takes &self per trait)
    announcer_task: Mutex<Option<JoinHandle<()>>>,
    receiver_task: Mutex<Option<JoinHandle<()>>>,
}

impl GossipPeerDiscovery {
    /// Create a new gossip peer discovery instance without starting tasks.
    ///
    /// Use `start()` or the `PeerDiscovery` trait to begin discovery.
    /// For the legacy API that starts immediately, use `spawn()` instead.
    ///
    /// # Arguments
    ///
    /// * `topic_id` - The gossip topic to use for peer announcements
    /// * `node_id` - This node's logical ID
    /// * `gossip` - Arc to the iroh gossip instance
    /// * `endpoint_addr` - This node's endpoint address
    /// * `secret_key` - Secret key for signing announcements
    pub fn new(
        topic_id: TopicId,
        node_id: NodeId,
        gossip: Arc<Gossip>,
        endpoint_addr: EndpointAddr,
        secret_key: SecretKey,
    ) -> Self {
        Self {
            topic_id,
            node_id,
            gossip,
            endpoint_addr,
            secret_key,
            is_running: Arc::new(AtomicBool::new(false)),
            cancel_token: CancellationToken::new(),
            announcer_task: Mutex::new(None),
            receiver_task: Mutex::new(None),
        }
    }

    /// Internal implementation of start that spawns the tasks.
    async fn start_internal(&self, on_peer_discovered: Option<PeerDiscoveredCallback<EndpointAddr>>) -> Result<()> {
        // Check if already running
        if self.is_running.load(Ordering::SeqCst) {
            anyhow::bail!("discovery is already running");
        }

        // Subscribe to the topic with timeout
        // Tiger Style: Explicit timeout prevents indefinite blocking during subscription.
        // If gossip is unavailable, the node should continue without it (non-fatal).
        let gossip_topic = tokio::time::timeout(GOSSIP_SUBSCRIBE_TIMEOUT, self.gossip.subscribe(self.topic_id, vec![]))
            .await
            .context("timeout subscribing to gossip topic")?
            .context("failed to subscribe to gossip topic")?;

        // Split into sender and receiver
        let (gossip_sender, mut gossip_receiver) = gossip_topic.split();

        // Clone fields for the spawned tasks
        let endpoint_addr = self.endpoint_addr.clone();
        let secret_key = self.secret_key.clone();

        // Spawn announcer task
        let announcer_cancel = self.cancel_token.child_token();
        let announcer_sender = gossip_sender.clone();
        let announcer_node_id = self.node_id;
        let announcer_task = tokio::spawn(async move {
            // Adaptive interval tracking for sender-side rate limiting
            let mut consecutive_failures: u32 = 0;
            let mut current_interval_secs = GOSSIP_MIN_ANNOUNCE_INTERVAL_SECS;
            let mut ticker = interval(Duration::from_secs(current_interval_secs));
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                tokio::select! {
                    _ = announcer_cancel.cancelled() => {
                        tracing::debug!("gossip announcer shutting down");
                        break;
                    }
                    _ = ticker.tick() => {
                        let announcement =
                            match PeerAnnouncement::new(announcer_node_id, endpoint_addr.clone()) {
                                Ok(ann) => ann,
                                Err(e) => {
                                    tracing::error!("failed to create peer announcement: {}", e);
                                    continue;
                                }
                            };

                        // Sign the announcement with our secret key
                        let signed = match SignedPeerAnnouncement::sign(announcement, &secret_key) {
                            Ok(s) => s,
                            Err(e) => {
                                tracing::error!("failed to sign peer announcement: {}", e);
                                continue;
                            }
                        };

                        // Wrap in GossipMessage envelope for versioned message handling
                        let gossip_msg = GossipMessage::PeerAnnouncement(signed);
                        match gossip_msg.to_bytes() {
                            Ok(bytes) => {
                                match announcer_sender.broadcast(bytes.into()).await {
                                    Ok(()) => {
                                        tracing::trace!(
                                            "broadcast signed peer announcement for node_id={}",
                                            announcer_node_id
                                        );
                                        // Reset on success if we were in backoff mode
                                        if consecutive_failures > 0 {
                                            consecutive_failures = 0;
                                            if current_interval_secs != GOSSIP_MIN_ANNOUNCE_INTERVAL_SECS {
                                                current_interval_secs = GOSSIP_MIN_ANNOUNCE_INTERVAL_SECS;
                                                ticker = interval(Duration::from_secs(current_interval_secs));
                                                ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                                                tracing::debug!(
                                                    "announcement succeeded, resetting interval to {}s",
                                                    current_interval_secs
                                                );
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        consecutive_failures += 1;
                                        tracing::warn!(
                                            "failed to broadcast peer announcement (failure {}/{}): {}",
                                            consecutive_failures,
                                            GOSSIP_ANNOUNCE_FAILURE_THRESHOLD,
                                            e
                                        );

                                        // Increase interval after threshold failures
                                        if consecutive_failures >= GOSSIP_ANNOUNCE_FAILURE_THRESHOLD {
                                            let new_interval = (current_interval_secs * 2)
                                                .min(GOSSIP_MAX_ANNOUNCE_INTERVAL_SECS);
                                            if new_interval != current_interval_secs {
                                                current_interval_secs = new_interval;
                                                ticker = interval(Duration::from_secs(current_interval_secs));
                                                ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                                                tracing::info!(
                                                    "increasing announcement interval to {}s due to failures",
                                                    current_interval_secs
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!("failed to serialize signed peer announcement: {}", e);
                            }
                        }
                    }
                }
            }
        });

        // Spawn receiver task
        let receiver_cancel = self.cancel_token.child_token();
        let receiver_node_id = self.node_id;
        let receiver_callback = on_peer_discovered;
        let receiver_task = tokio::spawn(async move {
            // Rate limiter for incoming gossip messages (HIGH-6 security)
            let mut rate_limiter = GossipRateLimiter::new();

            // Error recovery state for transient stream errors
            let mut consecutive_errors: u32 = 0;
            let backoff_durations: Vec<Duration> =
                GOSSIP_STREAM_BACKOFF_SECS.iter().map(|s| Duration::from_secs(*s)).collect();

            loop {
                tokio::select! {
                    _ = receiver_cancel.cancelled() => {
                        tracing::debug!("gossip receiver shutting down");
                        break;
                    }
                    event = gossip_receiver.next() => match event {
                    Some(Ok(Event::Received(msg))) => {
                        // Reset error count on successful message
                        consecutive_errors = 0;
                        // Rate limit check BEFORE parsing/signature verification (save CPU)
                        // Use delivered_from as the peer identifier
                        if let Err(reason) = rate_limiter.check(&msg.delivered_from) {
                            match reason {
                                RateLimitReason::PerPeer => {
                                    // Per-peer limit: drop silently (avoid log spam)
                                    tracing::trace!(
                                        "rate limited gossip message from peer={:?}",
                                        msg.delivered_from
                                    );
                                }
                                RateLimitReason::Global => {
                                    // Global limit: log at warn level (indicates attack)
                                    tracing::warn!(
                                        "global gossip rate limit exceeded, dropping message from peer={:?}",
                                        msg.delivered_from
                                    );
                                }
                            }
                            continue;
                        }

                        // Parse gossip message (supports both peer and topology announcements)
                        let gossip_msg = match GossipMessage::from_bytes(&msg.content) {
                            Some(m) => m,
                            None => {
                                tracing::warn!("failed to parse gossip message");
                                continue;
                            }
                        };

                        match gossip_msg {
                            GossipMessage::PeerAnnouncement(signed) => {
                                // Verify signature - reject if invalid
                                let announcement = match signed.verify() {
                                    Some(ann) => ann,
                                    None => {
                                        tracing::warn!(
                                            "rejected peer announcement with invalid signature from endpoint_id={:?}",
                                            signed.announcement.endpoint_addr.id
                                        );
                                        continue;
                                    }
                                };

                                // Filter out our own announcements
                                if announcement.node_id == receiver_node_id {
                                    tracing::trace!("ignoring self-announcement");
                                    continue;
                                }

                                tracing::debug!(
                                    "received verified peer announcement from node_id={}, endpoint_id={:?}",
                                    announcement.node_id,
                                    announcement.endpoint_addr.id
                                );

                                // Add peer to network factory's fallback cache.
                                if let Some(ref callback) = receiver_callback {
                                    let discovered = DiscoveredPeer {
                                        node_id: announcement.node_id,
                                        address: announcement.endpoint_addr.clone(),
                                        timestamp_micros: announcement.timestamp_micros,
                                    };
                                    callback(discovered).await;
                                }

                                // Log the discovery details
                                let relay_urls: Vec<_> =
                                    announcement.endpoint_addr.relay_urls().collect();
                                tracing::info!(
                                    "discovered verified peer: node_id={}, endpoint_id={:?}, relay={:?}, direct_addresses={}",
                                    announcement.node_id,
                                    announcement.endpoint_addr.id,
                                    relay_urls,
                                    announcement.endpoint_addr.addrs.len()
                                );
                            }
                            GossipMessage::TopologyAnnouncement(signed) => {
                                // Verify signature - reject if invalid
                                let announcement = match signed.verify() {
                                    Some(ann) => ann,
                                    None => {
                                        tracing::warn!(
                                            "rejected topology announcement with invalid signature from node_id={}",
                                            signed.announcement.node_id
                                        );
                                        continue;
                                    }
                                };

                                // Filter out our own announcements
                                if announcement.node_id == u64::from(receiver_node_id) {
                                    tracing::trace!("ignoring self-topology-announcement");
                                    continue;
                                }

                                tracing::debug!(
                                    "received verified topology announcement from node_id={}, version={}, hash={}",
                                    announcement.node_id,
                                    announcement.topology_version,
                                    announcement.topology_hash
                                );

                                // TODO: Compare with local topology version and trigger sync if stale.
                                // This will be implemented when we add the topology sync RPC.
                                // For now, just log the announcement so nodes can observe topology changes.
                                tracing::info!(
                                    "topology announcement: node_id={}, version={}, term={}",
                                    announcement.node_id,
                                    announcement.topology_version,
                                    announcement.term
                                );
                            }
                            GossipMessage::BlobAnnouncement(signed) => {
                                // Verify signature - reject if invalid
                                let announcement = match signed.verify() {
                                    Some(ann) => ann,
                                    None => {
                                        tracing::warn!(
                                            "rejected blob announcement with invalid signature from node_id={}",
                                            u64::from(signed.announcement.node_id)
                                        );
                                        continue;
                                    }
                                };

                                // Filter out our own announcements
                                if u64::from(announcement.node_id) == u64::from(receiver_node_id) {
                                    tracing::trace!("ignoring self-blob-announcement");
                                    continue;
                                }

                                // Log the blob availability announcement
                                // In the future, this could trigger background blob fetching
                                // for redundancy or prefetching based on access patterns.
                                tracing::info!(
                                    hash = %announcement.blob_hash.fmt_short(),
                                    size = announcement.blob_size,
                                    format = ?announcement.blob_format,
                                    node_id = u64::from(announcement.node_id),
                                    tag = ?announcement.tag,
                                    "discovered blob available from peer"
                                );

                                // TODO: Optionally trigger background blob download for redundancy
                                // if let Some(blob_store) = &blob_store_opt {
                                //     let provider = announcement.endpoint_addr.id;
                                //     if !blob_store.has(&announcement.blob_hash).await.unwrap_or(false) {
                                //         // Download in background for redundancy
                                //         blob_store.download_from_peer(&announcement.blob_hash, provider).await;
                                //     }
                                // }
                            }
                        }
                    }
                    Some(Ok(Event::NeighborUp(neighbor_id))) => {
                        tracing::debug!("neighbor up: {:?}", neighbor_id);
                    }
                    Some(Ok(Event::NeighborDown(neighbor_id))) => {
                        tracing::debug!("neighbor down: {:?}", neighbor_id);
                    }
                    Some(Ok(Event::Lagged)) => {
                        tracing::warn!("gossip receiver lagged, messages may be lost");
                    }
                    Some(Err(e)) => {
                        consecutive_errors += 1;

                        // Check if we've exceeded max retries
                        if consecutive_errors > GOSSIP_MAX_STREAM_RETRIES {
                            tracing::error!(
                                "gossip receiver exceeded max retries ({}), giving up: {}",
                                GOSSIP_MAX_STREAM_RETRIES,
                                e
                            );
                            break;
                        }

                        // Calculate backoff duration using existing pure function
                        let backoff = calculate_backoff_duration(
                            (consecutive_errors as usize).saturating_sub(1),
                            &backoff_durations
                        );

                        tracing::warn!(
                            "gossip receiver error (retry {}/{}), backing off for {:?}: {}",
                            consecutive_errors,
                            GOSSIP_MAX_STREAM_RETRIES,
                            backoff,
                            e
                        );

                        // Wait before retrying - stream may recover
                        tokio::time::sleep(backoff).await;
                        continue;
                    }
                    None => {
                        tracing::info!("gossip receiver stream ended");
                        break;
                    }
                    } // end of match event
                } // end of tokio::select!
            }
        });

        // Store tasks and mark as running
        *self.announcer_task.lock().await = Some(announcer_task);
        *self.receiver_task.lock().await = Some(receiver_task);
        self.is_running.store(true, Ordering::SeqCst);

        Ok(())
    }

    /// Get the topic ID for this discovery instance.
    pub fn get_topic_id(&self) -> TopicId {
        self.topic_id
    }

    /// Shutdown the discovery tasks and wait for completion.
    ///
    /// Tiger Style: Bounded wait time (10 seconds max), explicit task abortion on timeout.
    pub async fn shutdown(self) -> Result<()> {
        self.stop().await
    }

    /// Stop the discovery tasks (can be called on &self).
    ///
    /// This is similar to shutdown() but doesn't consume self.
    async fn stop(&self) -> Result<()> {
        tracing::info!("shutting down gossip peer discovery");

        // Mark as not running
        self.is_running.store(false, Ordering::SeqCst);

        // Signal cancellation to both tasks
        self.cancel_token.cancel();

        // Wait for tasks with timeout
        let timeout = Duration::from_secs(10);

        // Take tasks out of Option (they will be None after this, preventing Drop from aborting)
        let mut announcer_task = self
            .announcer_task
            .lock()
            .await
            .take()
            .context("announcer task not initialized or already consumed")?;
        let mut receiver_task = self
            .receiver_task
            .lock()
            .await
            .take()
            .context("receiver task not initialized or already consumed")?;

        tokio::select! {
            result = &mut announcer_task => {
                match result {
                    Ok(()) => tracing::debug!("announcer task completed"),
                    Err(e) => tracing::error!("announcer task panicked: {}", e),
                }
            }
            _ = tokio::time::sleep(timeout) => {
                tracing::warn!("announcer task did not complete within timeout, aborting");
                // Tiger Style: Explicit task abortion to prevent leak
                announcer_task.abort();
            }
        }

        tokio::select! {
            result = &mut receiver_task => {
                match result {
                    Ok(()) => tracing::debug!("receiver task completed"),
                    Err(e) => tracing::error!("receiver task panicked: {}", e),
                }
            }
            _ = tokio::time::sleep(timeout) => {
                tracing::warn!("receiver task did not complete within timeout, aborting");
                // Tiger Style: Explicit task abortion to prevent leak
                receiver_task.abort();
            }
        }

        Ok(())
    }

    /// Broadcast a peer announcement immediately.
    ///
    /// This is useful for forcing an announcement after a significant event
    /// (e.g., address change, rejoin after network partition).
    pub async fn broadcast_announcement(&self) -> Result<()> {
        // Create and sign announcement
        let announcement = PeerAnnouncement::new(self.node_id, self.endpoint_addr.clone())?;
        let signed = SignedPeerAnnouncement::sign(announcement, &self.secret_key)?;
        let gossip_msg = GossipMessage::PeerAnnouncement(signed);
        let bytes = gossip_msg.to_bytes()?;

        // Subscribe briefly to get a sender, then broadcast
        let mut topic = self
            .gossip
            .subscribe(self.topic_id, vec![])
            .await
            .context("failed to subscribe to gossip topic for announcement")?;

        topic.broadcast(bytes.into()).await.context("failed to broadcast peer announcement")?;

        tracing::debug!("broadcast immediate peer announcement for node_id={}", self.node_id);
        Ok(())
    }
}

// ============================================================================
// PeerDiscovery Trait Implementation
// ============================================================================

#[async_trait]
impl PeerDiscovery for GossipPeerDiscovery {
    type Address = EndpointAddr;
    type TopicId = TopicId;

    fn topic_id(&self) -> &Self::TopicId {
        &self.topic_id
    }

    async fn start(
        &self,
        on_peer_discovered: Option<PeerDiscoveredCallback<Self::Address>>,
    ) -> Result<DiscoveryHandle> {
        self.start_internal(on_peer_discovered).await?;
        Ok(DiscoveryHandle::new(self.cancel_token.clone()))
    }

    async fn announce(&self) -> Result<()> {
        self.broadcast_announcement().await
    }

    fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }
}

/// Parameters for broadcasting a blob announcement.
///
/// Groups related parameters to avoid too many function arguments.
pub struct BlobAnnouncementParams {
    /// Our node ID
    pub node_id: NodeId,
    /// Our endpoint address
    pub endpoint_addr: EndpointAddr,
    /// BLAKE3 hash of the blob
    pub blob_hash: iroh_blobs::Hash,
    /// Size of the blob in bytes
    pub blob_size: u64,
    /// Format of the blob (Raw or HashSeq)
    pub blob_format: iroh_blobs::BlobFormat,
    /// Optional categorization tag
    pub tag: Option<String>,
}

/// Broadcast a blob announcement to the gossip network.
///
/// This announces that the local node has a blob available for P2P download.
/// Other nodes can use this information to fetch the blob for redundancy
/// or when they need the content.
///
/// # Arguments
/// * `gossip` - The gossip instance
/// * `topic_id` - The gossip topic to broadcast on
/// * `secret_key` - Secret key for signing
/// * `params` - Blob announcement parameters
///
/// # Returns
/// Ok(()) on success, Err if broadcast fails
pub async fn broadcast_blob_announcement(
    gossip: &Gossip,
    topic_id: TopicId,
    secret_key: &SecretKey,
    params: BlobAnnouncementParams,
) -> Result<()> {
    // Create and sign the announcement
    let announcement = BlobAnnouncement::new(
        params.node_id,
        params.endpoint_addr,
        params.blob_hash,
        params.blob_size,
        params.blob_format,
        params.tag,
    )?;
    let signed = SignedBlobAnnouncement::sign(announcement, secret_key)?;
    let message = GossipMessage::BlobAnnouncement(signed);
    let bytes = message.to_bytes()?;

    // Subscribe briefly to get a sender, then broadcast
    // Note: This creates a new subscription which may not be ideal for frequent announcements.
    // For high-frequency blob announcements, consider caching the topic/sender.
    let mut topic = gossip.subscribe(topic_id, vec![]).await.context("failed to subscribe to gossip topic")?;
    topic.broadcast(bytes.into()).await.context("failed to broadcast blob announcement")?;

    tracing::debug!(
        hash = %params.blob_hash.fmt_short(),
        size = params.blob_size,
        format = ?params.blob_format,
        "broadcast blob announcement"
    );

    Ok(())
}

/// Tiger Style: Abort tasks if dropped without explicit shutdown().
///
/// This prevents task leaks when GossipPeerDiscovery is dropped (e.g., on panic
/// or if shutdown() is not called). The tasks will be aborted immediately.
///
/// If shutdown() was called, tasks will already be None and this is a no-op.
impl Drop for GossipPeerDiscovery {
    fn drop(&mut self) {
        // Use try_lock since Drop is synchronous
        let announcer_task = self.announcer_task.try_lock().ok().and_then(|mut guard| guard.take());
        let receiver_task = self.receiver_task.try_lock().ok().and_then(|mut guard| guard.take());

        let has_tasks = announcer_task.is_some() || receiver_task.is_some();
        if has_tasks {
            tracing::warn!("GossipPeerDiscovery dropped without shutdown(), aborting tasks");
            if let Some(task) = announcer_task {
                task.abort();
            }
            if let Some(task) = receiver_task {
                task.abort();
            }
        }
    }
}
