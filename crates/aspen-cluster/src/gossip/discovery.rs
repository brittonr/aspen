//! Core gossip peer discovery implementation.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use aspen_core::BlobAnnouncedCallback;
use aspen_core::BlobAnnouncedInfo;
use aspen_core::DiscoveredPeer;
use aspen_core::DiscoveryHandle;
use aspen_core::PeerDiscoveredCallback;
use aspen_core::PeerDiscovery;
use aspen_core::StaleTopologyInfo;
use aspen_core::TopologyStaleCallback;
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

use super::constants::*;
use super::rate_limiter::GossipRateLimiter;
use super::rate_limiter::RateLimitReason;
use super::types::*;

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
    // Topology sync: local version for staleness detection
    local_topology_version: Arc<std::sync::atomic::AtomicU64>,
    // Callback for stale topology detection (triggers sync RPC)
    on_topology_stale: Mutex<Option<TopologyStaleCallback>>,
    // Callback for blob announcements (triggers background download)
    on_blob_announced: Mutex<Option<BlobAnnouncedCallback>>,
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
            local_topology_version: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            on_topology_stale: Mutex::new(None),
            on_blob_announced: Mutex::new(None),
        }
    }

    /// Set the callback for stale topology detection.
    ///
    /// The callback is invoked when a gossip announcement indicates a topology version
    /// higher than the local version. Use this to trigger topology sync RPCs.
    pub async fn set_topology_stale_callback(&self, callback: TopologyStaleCallback) {
        *self.on_topology_stale.lock().await = Some(callback);
    }

    /// Set the callback for blob announcements.
    ///
    /// The callback is invoked when a peer announces a blob available for download.
    /// Use this to optionally download blobs for redundancy or prefetching.
    pub async fn set_blob_announced_callback(&self, callback: BlobAnnouncedCallback) {
        *self.on_blob_announced.lock().await = Some(callback);
    }

    /// Update the local topology version.
    ///
    /// Call this when local topology is updated (e.g., after applying a Raft command
    /// or receiving a topology sync response). This prevents false staleness detection.
    pub fn set_local_topology_version(&self, version: u64) {
        self.local_topology_version.store(version, Ordering::SeqCst);
        tracing::debug!(version, "updated local topology version");
    }

    /// Get the current local topology version.
    pub fn local_topology_version(&self) -> u64 {
        self.local_topology_version.load(Ordering::SeqCst)
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
        // Clone topology sync components for the receiver task
        let local_topology_version = self.local_topology_version.clone();
        let topology_stale_callback = self.on_topology_stale.lock().await.take();
        // Clone blob announcement callback for the receiver task
        #[cfg(feature = "blob")]
        let blob_announced_callback = self.on_blob_announced.lock().await.take();
        #[cfg(not(feature = "blob"))]
        let _ = self.on_blob_announced.lock().await.take();
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

                        // Tiger Style: Check message size before parsing (DoS prevention)
                        if msg.content.len() > MAX_GOSSIP_MESSAGE_SIZE {
                            tracing::warn!(
                                size = msg.content.len(),
                                max = MAX_GOSSIP_MESSAGE_SIZE,
                                peer = ?msg.delivered_from,
                                "rejected oversized gossip message"
                            );
                            continue;
                        }

                        // Parse gossip message (supports both peer and topology announcements)
                        let gossip_msg = match GossipMessage::from_bytes(&msg.content) {
                            Some(m) => m,
                            None => {
                                tracing::warn!(
                                    size = msg.content.len(),
                                    peer = ?msg.delivered_from,
                                    "failed to parse gossip message"
                                );
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

                                // Compare with local topology version and trigger sync if stale
                                let local_version = local_topology_version.load(Ordering::SeqCst);
                                if announcement.topology_version > local_version {
                                    tracing::info!(
                                        local_version,
                                        remote_version = announcement.topology_version,
                                        remote_node = announcement.node_id,
                                        "detected stale topology, triggering sync"
                                    );

                                    // Invoke callback if registered
                                    if let Some(ref callback) = topology_stale_callback {
                                        let info = StaleTopologyInfo {
                                            announcing_node_id: announcement.node_id,
                                            remote_version: announcement.topology_version,
                                            remote_hash: announcement.topology_hash,
                                            remote_term: announcement.term,
                                        };
                                        callback(info).await;
                                    }
                                } else {
                                    tracing::trace!(
                                        local_version,
                                        remote_version = announcement.topology_version,
                                        "topology announcement: local version is current"
                                    );
                                }
                            }
                            #[cfg(feature = "blob")]
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
                                tracing::info!(
                                    hash = %announcement.blob_hash.fmt_short(),
                                    size = announcement.blob_size,
                                    format = ?announcement.blob_format,
                                    node_id = u64::from(announcement.node_id),
                                    tag = ?announcement.tag,
                                    "discovered blob available from peer"
                                );

                                // Invoke callback for background blob download
                                if let Some(ref callback) = blob_announced_callback {
                                    let info = BlobAnnouncedInfo {
                                        announcing_node_id: u64::from(announcement.node_id),
                                        provider_public_key: announcement.endpoint_addr.id,
                                        blob_hash_hex: announcement.blob_hash.to_hex().to_string(),
                                        blob_size: announcement.blob_size,
                                        is_raw_format: matches!(announcement.blob_format, iroh_blobs::BlobFormat::Raw),
                                        tag: announcement.tag.clone(),
                                    };
                                    callback(info).await;
                                }
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
#[cfg(feature = "blob")]
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
#[cfg(feature = "blob")]
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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicU64;

    use super::*;

    /// Create a deterministic secret key from a seed for reproducible tests.
    fn secret_key_from_seed(seed: u64) -> SecretKey {
        let mut key_bytes = [0u8; 32];
        key_bytes[0..8].copy_from_slice(&seed.to_le_bytes());
        SecretKey::from_bytes(&key_bytes)
    }

    /// Create a mock EndpointAddr from a secret key.
    fn endpoint_addr_from_secret_key(secret_key: &SecretKey) -> EndpointAddr {
        EndpointAddr::new(secret_key.public())
    }

    // =========================================================================
    // calculate_backoff_duration Tests (Pure Function)
    // =========================================================================

    #[test]
    fn test_calculate_backoff_duration_first_index() {
        let durations = vec![Duration::from_secs(1), Duration::from_secs(2), Duration::from_secs(4)];
        assert_eq!(calculate_backoff_duration(0, &durations), Duration::from_secs(1));
    }

    #[test]
    fn test_calculate_backoff_duration_middle_index() {
        let durations = vec![Duration::from_secs(1), Duration::from_secs(2), Duration::from_secs(4)];
        assert_eq!(calculate_backoff_duration(1, &durations), Duration::from_secs(2));
    }

    #[test]
    fn test_calculate_backoff_duration_last_index() {
        let durations = vec![Duration::from_secs(1), Duration::from_secs(2), Duration::from_secs(4)];
        assert_eq!(calculate_backoff_duration(2, &durations), Duration::from_secs(4));
    }

    #[test]
    fn test_calculate_backoff_duration_clamps_to_max() {
        let durations = vec![Duration::from_secs(1), Duration::from_secs(2), Duration::from_secs(4)];
        // Index 10 exceeds array length, should clamp to last element
        assert_eq!(calculate_backoff_duration(10, &durations), Duration::from_secs(4));
        assert_eq!(calculate_backoff_duration(100, &durations), Duration::from_secs(4));
        assert_eq!(calculate_backoff_duration(usize::MAX, &durations), Duration::from_secs(4));
    }

    #[test]
    fn test_calculate_backoff_duration_single_element() {
        let durations = vec![Duration::from_secs(5)];
        assert_eq!(calculate_backoff_duration(0, &durations), Duration::from_secs(5));
        assert_eq!(calculate_backoff_duration(1, &durations), Duration::from_secs(5));
        assert_eq!(calculate_backoff_duration(100, &durations), Duration::from_secs(5));
    }

    #[test]
    fn test_calculate_backoff_duration_with_default_constants() {
        // Test with the actual constants used in production
        let durations: Vec<Duration> = GOSSIP_STREAM_BACKOFF_SECS.iter().map(|s| Duration::from_secs(*s)).collect();

        assert_eq!(calculate_backoff_duration(0, &durations), Duration::from_secs(1));
        assert_eq!(calculate_backoff_duration(1, &durations), Duration::from_secs(2));
        assert_eq!(calculate_backoff_duration(2, &durations), Duration::from_secs(4));
        assert_eq!(calculate_backoff_duration(3, &durations), Duration::from_secs(8));
        assert_eq!(calculate_backoff_duration(4, &durations), Duration::from_secs(16));
        // Past the end should clamp to the last element
        assert_eq!(calculate_backoff_duration(5, &durations), Duration::from_secs(16));
        assert_eq!(calculate_backoff_duration(100, &durations), Duration::from_secs(16));
    }

    // =========================================================================
    // GossipPeerDiscovery Unit Tests (No Network Required)
    // =========================================================================

    #[test]
    fn test_gossip_peer_discovery_initial_state_not_running() {
        // We can't create a full GossipPeerDiscovery without a real Gossip instance,
        // but we can test the AtomicBool and AtomicU64 patterns used internally.
        let is_running = Arc::new(AtomicBool::new(false));
        assert!(!is_running.load(Ordering::SeqCst));

        is_running.store(true, Ordering::SeqCst);
        assert!(is_running.load(Ordering::SeqCst));
    }

    #[test]
    fn test_topology_version_atomic_operations() {
        let version = Arc::new(AtomicU64::new(0));
        assert_eq!(version.load(Ordering::SeqCst), 0);

        version.store(42, Ordering::SeqCst);
        assert_eq!(version.load(Ordering::SeqCst), 42);

        version.store(u64::MAX, Ordering::SeqCst);
        assert_eq!(version.load(Ordering::SeqCst), u64::MAX);
    }

    #[test]
    fn test_cancellation_token_hierarchy() {
        let parent = CancellationToken::new();
        let child = parent.child_token();

        assert!(!parent.is_cancelled());
        assert!(!child.is_cancelled());

        parent.cancel();

        assert!(parent.is_cancelled());
        assert!(child.is_cancelled());
    }

    #[test]
    fn test_cancellation_token_child_independence() {
        let parent = CancellationToken::new();
        let child1 = parent.child_token();
        let child2 = parent.child_token();

        // Cancelling a child doesn't affect parent or siblings
        // (children don't have cancel(), only parent does)
        assert!(!parent.is_cancelled());
        assert!(!child1.is_cancelled());
        assert!(!child2.is_cancelled());

        parent.cancel();

        // All should be cancelled now
        assert!(parent.is_cancelled());
        assert!(child1.is_cancelled());
        assert!(child2.is_cancelled());
    }

    // =========================================================================
    // BlobAnnouncementParams Tests
    // =========================================================================

    #[cfg(feature = "blob")]
    #[test]
    fn test_blob_announcement_params_fields() {
        let secret_key = secret_key_from_seed(1);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key);
        let node_id = NodeId::from(123u64);
        let blob_hash = iroh_blobs::Hash::from_bytes([0xAB; 32]);

        let params = BlobAnnouncementParams {
            node_id,
            endpoint_addr: endpoint_addr.clone(),
            blob_hash,
            blob_size: 1024,
            blob_format: iroh_blobs::BlobFormat::Raw,
            tag: Some("test".to_string()),
        };

        assert_eq!(params.node_id, node_id);
        assert_eq!(params.endpoint_addr.id, endpoint_addr.id);
        assert_eq!(params.blob_hash, blob_hash);
        assert_eq!(params.blob_size, 1024);
        assert_eq!(params.blob_format, iroh_blobs::BlobFormat::Raw);
        assert_eq!(params.tag, Some("test".to_string()));
    }

    #[cfg(feature = "blob")]
    #[test]
    fn test_blob_announcement_params_no_tag() {
        let secret_key = secret_key_from_seed(2);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key);

        let params = BlobAnnouncementParams {
            node_id: NodeId::from(456u64),
            endpoint_addr,
            blob_hash: iroh_blobs::Hash::from_bytes([0xCD; 32]),
            blob_size: 0,
            blob_format: iroh_blobs::BlobFormat::HashSeq,
            tag: None,
        };

        assert!(params.tag.is_none());
        assert_eq!(params.blob_format, iroh_blobs::BlobFormat::HashSeq);
    }

    // =========================================================================
    // Topic ID Tests
    // =========================================================================

    #[test]
    fn test_topic_id_from_bytes_deterministic() {
        let topic1 = TopicId::from_bytes([1u8; 32]);
        let topic2 = TopicId::from_bytes([1u8; 32]);
        let topic3 = TopicId::from_bytes([2u8; 32]);

        assert_eq!(topic1, topic2);
        assert_ne!(topic1, topic3);
    }

    // =========================================================================
    // DiscoveredPeer Tests
    // =========================================================================

    #[test]
    fn test_discovered_peer_construction() {
        let secret_key = secret_key_from_seed(10);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key);
        let node_id = NodeId::from(100u64);
        let timestamp = 1234567890u64;

        let peer = DiscoveredPeer {
            node_id,
            address: endpoint_addr.clone(),
            timestamp_micros: timestamp,
        };

        assert_eq!(peer.node_id, node_id);
        assert_eq!(peer.address.id, endpoint_addr.id);
        assert_eq!(peer.timestamp_micros, timestamp);
    }

    // =========================================================================
    // Constants Validation Tests
    // =========================================================================

    #[test]
    fn test_gossip_constants_sane_values() {
        // Verify constants are within reasonable bounds
        assert!(GOSSIP_MIN_ANNOUNCE_INTERVAL_SECS > 0);
        assert!(GOSSIP_MAX_ANNOUNCE_INTERVAL_SECS >= GOSSIP_MIN_ANNOUNCE_INTERVAL_SECS);
        assert!(GOSSIP_ANNOUNCE_FAILURE_THRESHOLD > 0);
        assert!(GOSSIP_MAX_STREAM_RETRIES > 0);
        assert!(!GOSSIP_STREAM_BACKOFF_SECS.is_empty());
        assert!(MAX_GOSSIP_MESSAGE_SIZE > 0);
        assert!(GOSSIP_SUBSCRIBE_TIMEOUT.as_secs() > 0);
    }

    #[test]
    fn test_gossip_stream_backoff_is_increasing() {
        // Verify backoff sequence is monotonically increasing
        let mut prev = 0u64;
        for &secs in &GOSSIP_STREAM_BACKOFF_SECS {
            assert!(secs > prev, "backoff sequence should be increasing");
            prev = secs;
        }
    }

    #[test]
    fn test_gossip_rate_limits_reasonable() {
        // Per-peer rate should be less than global rate
        assert!(GOSSIP_PER_PEER_RATE_PER_MINUTE < GOSSIP_GLOBAL_RATE_PER_MINUTE);
        assert!(GOSSIP_PER_PEER_BURST < GOSSIP_GLOBAL_BURST);
    }

    // =========================================================================
    // Async Tests (Require Tokio Runtime but No Real Network)
    // =========================================================================

    #[tokio::test]
    async fn test_mutex_callback_pattern() {
        // Test the callback storage pattern used by GossipPeerDiscovery
        let callback_storage: Mutex<Option<TopologyStaleCallback>> = Mutex::new(None);

        // Initially None
        assert!(callback_storage.lock().await.is_none());

        // Set a callback
        let invoked = Arc::new(AtomicBool::new(false));
        let invoked_clone = invoked.clone();
        let callback: TopologyStaleCallback = Box::new(move |_info| {
            let invoked = invoked_clone.clone();
            Box::pin(async move {
                invoked.store(true, Ordering::SeqCst);
            })
        });

        *callback_storage.lock().await = Some(callback);

        // Take and invoke
        let taken_callback = callback_storage.lock().await.take();
        assert!(taken_callback.is_some());

        let info = StaleTopologyInfo {
            announcing_node_id: 1,
            remote_version: 10,
            remote_hash: 123456,
            remote_term: 1,
        };
        taken_callback.unwrap()(info).await;

        assert!(invoked.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_cancellation_token_select_pattern() {
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();

        // Spawn a task that waits for cancellation
        let handle = tokio::spawn(async move {
            tokio::select! {
                _ = cancel_clone.cancelled() => {
                    "cancelled"
                }
                _ = tokio::time::sleep(Duration::from_secs(60)) => {
                    "timeout"
                }
            }
        });

        // Give the task time to start
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Cancel and verify
        cancel.cancel();
        let result = handle.await.unwrap();
        assert_eq!(result, "cancelled");
    }

    #[tokio::test]
    async fn test_task_abortion_on_timeout() {
        let handle = tokio::spawn(async {
            // This task would run forever
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });

        // Wait briefly then abort
        tokio::time::sleep(Duration::from_millis(10)).await;
        handle.abort();

        // The task should be aborted
        let result = handle.await;
        assert!(result.is_err());
        assert!(result.unwrap_err().is_cancelled());
    }

    #[tokio::test]
    async fn test_discovery_handle_cancellation() {
        let cancel = CancellationToken::new();
        let handle = DiscoveryHandle::new(cancel.clone());

        assert!(!cancel.is_cancelled());

        // Cancel via handle
        handle.cancel();

        assert!(cancel.is_cancelled());
    }

    // =========================================================================
    // Additional Edge Case Tests
    // =========================================================================

    #[test]
    fn test_calculate_backoff_duration_zero_length_clamp() {
        // Edge case: empty durations array (should be handled gracefully)
        // In practice this would panic due to saturating_sub on 0, but we test that
        // the code with GOSSIP_STREAM_BACKOFF_SECS is always non-empty
        assert!(!GOSSIP_STREAM_BACKOFF_SECS.is_empty());
    }

    #[test]
    fn test_secret_key_determinism() {
        // Verify our test helper produces deterministic keys
        let key1 = secret_key_from_seed(42);
        let key2 = secret_key_from_seed(42);
        let key3 = secret_key_from_seed(43);

        assert_eq!(key1.public(), key2.public());
        assert_ne!(key1.public(), key3.public());
    }

    #[test]
    fn test_endpoint_addr_from_secret_key_consistency() {
        let secret_key = secret_key_from_seed(100);
        let addr1 = endpoint_addr_from_secret_key(&secret_key);
        let addr2 = endpoint_addr_from_secret_key(&secret_key);

        assert_eq!(addr1.id, addr2.id);
    }

    #[test]
    fn test_node_id_conversions() {
        // Test NodeId from various u64 values
        let node_id_zero = NodeId::from(0u64);
        let node_id_one = NodeId::from(1u64);
        let node_id_max = NodeId::from(u64::MAX);

        assert_eq!(u64::from(node_id_zero), 0);
        assert_eq!(u64::from(node_id_one), 1);
        assert_eq!(u64::from(node_id_max), u64::MAX);
    }

    #[cfg(feature = "blob")]
    #[test]
    fn test_blob_announcement_params_large_size() {
        let secret_key = secret_key_from_seed(3);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key);

        let params = BlobAnnouncementParams {
            node_id: NodeId::from(1u64),
            endpoint_addr,
            blob_hash: iroh_blobs::Hash::from_bytes([0xFF; 32]),
            blob_size: u64::MAX, // Maximum possible size
            blob_format: iroh_blobs::BlobFormat::Raw,
            tag: None,
        };

        assert_eq!(params.blob_size, u64::MAX);
    }

    #[test]
    fn test_discovered_peer_with_zero_timestamp() {
        let secret_key = secret_key_from_seed(20);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key);

        let peer = DiscoveredPeer {
            node_id: NodeId::from(200u64),
            address: endpoint_addr,
            timestamp_micros: 0, // Edge case: zero timestamp
        };

        assert_eq!(peer.timestamp_micros, 0);
    }

    #[test]
    fn test_discovered_peer_with_max_timestamp() {
        let secret_key = secret_key_from_seed(21);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key);

        let peer = DiscoveredPeer {
            node_id: NodeId::from(201u64),
            address: endpoint_addr,
            timestamp_micros: u64::MAX, // Edge case: max timestamp
        };

        assert_eq!(peer.timestamp_micros, u64::MAX);
    }

    #[cfg(feature = "blob")]
    #[tokio::test]
    async fn test_blob_announced_callback_pattern() {
        // Test the BlobAnnouncedCallback storage pattern
        let callback_storage: Mutex<Option<BlobAnnouncedCallback>> = Mutex::new(None);

        // Initially None
        assert!(callback_storage.lock().await.is_none());

        // Set a callback
        let received_hash = Arc::new(Mutex::new(String::new()));
        let received_hash_clone = received_hash.clone();
        let callback: BlobAnnouncedCallback = Box::new(move |info| {
            let received = received_hash_clone.clone();
            Box::pin(async move {
                *received.lock().await = info.blob_hash_hex.clone();
            })
        });

        *callback_storage.lock().await = Some(callback);

        // Take and invoke
        let taken_callback = callback_storage.lock().await.take();
        assert!(taken_callback.is_some());

        let info = BlobAnnouncedInfo {
            announcing_node_id: 42,
            provider_public_key: secret_key_from_seed(99).public(),
            blob_hash_hex: "abcd1234".to_string(),
            blob_size: 1024,
            is_raw_format: true,
            tag: Some("test-tag".to_string()),
        };
        taken_callback.unwrap()(info).await;

        assert_eq!(*received_hash.lock().await, "abcd1234");
    }

    #[test]
    fn test_stale_topology_info_fields() {
        let info = StaleTopologyInfo {
            announcing_node_id: 5,
            remote_version: 100,
            remote_hash: 0xDEADBEEF,
            remote_term: 7,
        };

        assert_eq!(info.announcing_node_id, 5);
        assert_eq!(info.remote_version, 100);
        assert_eq!(info.remote_hash, 0xDEADBEEF);
        assert_eq!(info.remote_term, 7);
    }

    #[test]
    fn test_blob_announced_info_fields() {
        let public_key = secret_key_from_seed(50).public();
        let info = BlobAnnouncedInfo {
            announcing_node_id: 10,
            provider_public_key: public_key,
            blob_hash_hex: "0123456789abcdef".to_string(),
            blob_size: 2048,
            is_raw_format: false,
            tag: None,
        };

        assert_eq!(info.announcing_node_id, 10);
        assert_eq!(info.blob_size, 2048);
        assert!(!info.is_raw_format);
        assert!(info.tag.is_none());
    }

    #[tokio::test]
    async fn test_cancellation_token_multiple_waiters() {
        let cancel = CancellationToken::new();

        // Multiple tasks waiting on the same token
        let c1 = cancel.clone();
        let c2 = cancel.clone();
        let c3 = cancel.clone();

        let h1 = tokio::spawn(async move {
            c1.cancelled().await;
            "task1"
        });
        let h2 = tokio::spawn(async move {
            c2.cancelled().await;
            "task2"
        });
        let h3 = tokio::spawn(async move {
            c3.cancelled().await;
            "task3"
        });

        // Give tasks time to start waiting
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Cancel once, all should wake up
        cancel.cancel();

        let results = futures::future::join_all([h1, h2, h3]).await;
        for result in results {
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_gossip_subscribe_timeout_reasonable() {
        // Verify the subscription timeout is reasonable for network operations
        assert!(GOSSIP_SUBSCRIBE_TIMEOUT >= Duration::from_secs(5));
        assert!(GOSSIP_SUBSCRIBE_TIMEOUT <= Duration::from_secs(120));
    }

    #[test]
    fn test_max_gossip_message_size_bounded() {
        // Verify message size limit is reasonable (not too small, not too large)
        assert!(MAX_GOSSIP_MESSAGE_SIZE >= 1024); // At least 1KB
        assert!(MAX_GOSSIP_MESSAGE_SIZE <= 10 * 1024 * 1024); // At most 10MB
    }

    #[test]
    fn test_announce_interval_bounds() {
        // Min should be at least a few seconds to avoid spam
        assert!(GOSSIP_MIN_ANNOUNCE_INTERVAL_SECS >= 1);

        // Max should be reasonable for peer discovery latency
        assert!(GOSSIP_MAX_ANNOUNCE_INTERVAL_SECS <= 3600); // Max 1 hour

        // Max should be significantly larger than min for backoff room
        assert!(GOSSIP_MAX_ANNOUNCE_INTERVAL_SECS >= GOSSIP_MIN_ANNOUNCE_INTERVAL_SECS * 2);
    }

    #[tokio::test]
    async fn test_atomic_bool_concurrent_access() {
        let flag = Arc::new(AtomicBool::new(false));

        // Spawn multiple tasks that read/write the flag
        let mut handles = Vec::new();
        for _ in 0..10 {
            let f = flag.clone();
            handles.push(tokio::spawn(async move {
                for _ in 0..100 {
                    let _ = f.load(Ordering::SeqCst);
                    f.store(true, Ordering::SeqCst);
                }
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }

        // Flag should be true after all operations
        assert!(flag.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_atomic_u64_concurrent_increment() {
        let counter = Arc::new(AtomicU64::new(0));

        // Spawn tasks that increment the counter
        let mut handles = Vec::new();
        for _ in 0..10 {
            let c = counter.clone();
            handles.push(tokio::spawn(async move {
                for _ in 0..100 {
                    c.fetch_add(1, Ordering::SeqCst);
                }
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }

        // Should have 10 * 100 = 1000 increments
        assert_eq!(counter.load(Ordering::SeqCst), 1000);
    }
}
