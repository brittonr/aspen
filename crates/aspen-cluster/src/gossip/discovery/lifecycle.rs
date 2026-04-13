//! GossipPeerDiscovery struct definition, construction, and lifecycle management.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use aspen_core::BlobAnnouncedCallback;
use aspen_core::BlobAnnouncedInfo;
use aspen_core::DiscoveredPeer;
use aspen_core::PeerDiscoveredCallback;
use aspen_core::StaleTopologyInfo;
use aspen_core::TopologyStaleCallback;
use aspen_raft_types::NodeId;
use iroh::EndpointAddr;
use iroh::SecretKey;
use iroh_gossip::api::Event;
use iroh_gossip::api::GossipReceiver;
use iroh_gossip::api::GossipSender;
use iroh_gossip::api::GossipTopic;
use iroh_gossip::net::Gossip;
use iroh_gossip::proto::TopicId;
use n0_future::StreamExt;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;

use super::helpers::calculate_backoff_duration;
use crate::gossip::constants::*;
use crate::gossip::rate_limiter::GossipRateLimiter;
use crate::gossip::rate_limiter::RateLimitReason;
use crate::gossip::types::*;

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
    pub(super) topic_id: TopicId,
    node_id: NodeId,
    // Config stored for announce() and start()
    gossip: Arc<Gossip>,
    endpoint_addr: EndpointAddr,
    secret_key: SecretKey,
    /// Peer endpoint IDs to bootstrap gossip subscription.
    ///
    /// Without bootstrap peers, gossip has nobody to connect to after a
    /// restart. These are typically loaded from the persisted peer cache
    /// or Raft membership and passed at construction time.
    bootstrap_peers: Vec<iroh::EndpointId>,
    // Running state tracking
    pub(super) is_running: Arc<AtomicBool>,
    pub(super) cancel_token: CancellationToken,
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
            bootstrap_peers: Vec::new(),
            is_running: Arc::new(AtomicBool::new(false)),
            cancel_token: CancellationToken::new(),
            announcer_task: Mutex::new(None),
            receiver_task: Mutex::new(None),
            local_topology_version: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            on_topology_stale: Mutex::new(None),
            on_blob_announced: Mutex::new(None),
        }
    }

    /// Set bootstrap peers for gossip subscription.
    ///
    /// These endpoint IDs are passed to `gossip.subscribe()` so the gossip
    /// protocol has peers to connect to on startup. Without this, a restarted
    /// node's gossip is isolated and can never discover updated addresses.
    pub fn set_bootstrap_peers(&mut self, peers: Vec<iroh::EndpointId>) {
        self.bootstrap_peers = peers;
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

    async fn subscribe_gossip_topic(&self) -> Result<GossipTopic> {
        if self.bootstrap_peers.is_empty() {
            tracing::debug!(node_id = %self.node_id, "gossip starting with no bootstrap peers");
        } else {
            tracing::info!(
                node_id = %self.node_id,
                peer_count = self.bootstrap_peers.len(),
                "gossip starting with bootstrap peers from peer cache"
            );
        }

        tokio::time::timeout(
            GOSSIP_SUBSCRIBE_TIMEOUT,
            self.gossip.subscribe(self.topic_id, self.bootstrap_peers.clone()),
        )
        .await
        .context("timeout subscribing to gossip topic")?
        .context("failed to subscribe to gossip topic")
    }

    fn spawn_announcer_task(
        endpoint_addr: EndpointAddr,
        secret_key: SecretKey,
        cancel: CancellationToken,
        sender: GossipSender,
        node_id: NodeId,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            run_announcer_loop(endpoint_addr, secret_key, cancel, sender, node_id).await;
        })
    }

    fn spawn_receiver_task(
        cancel: CancellationToken,
        receiver: GossipReceiver,
        receiver_node_id: NodeId,
        receiver_callback: Option<PeerDiscoveredCallback<EndpointAddr>>,
        local_topology_version: Arc<std::sync::atomic::AtomicU64>,
        topology_stale_callback: Option<TopologyStaleCallback>,
        blob_announced_callback: Option<BlobAnnouncedCallback>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            run_receiver_loop(
                cancel,
                receiver,
                receiver_node_id,
                receiver_callback,
                local_topology_version,
                topology_stale_callback,
                blob_announced_callback,
            )
            .await;
        })
    }

    /// Internal implementation of start that spawns the tasks.
    pub(super) async fn start_internal(
        &self,
        on_peer_discovered: Option<PeerDiscoveredCallback<EndpointAddr>>,
    ) -> Result<()> {
        if self.is_running.load(Ordering::SeqCst) {
            anyhow::bail!("discovery is already running");
        }

        let gossip_topic = self.subscribe_gossip_topic().await?;
        let (gossip_sender, gossip_receiver) = gossip_topic.split();
        let topology_stale_callback = self.on_topology_stale.lock().await.take();
        let blob_announced_callback = self.on_blob_announced.lock().await.take();

        let announcer_task = Self::spawn_announcer_task(
            self.endpoint_addr.clone(),
            self.secret_key.clone(),
            self.cancel_token.child_token(),
            gossip_sender.clone(),
            self.node_id,
        );
        let receiver_task = Self::spawn_receiver_task(
            self.cancel_token.child_token(),
            gossip_receiver,
            self.node_id,
            on_peer_discovered,
            self.local_topology_version.clone(),
            topology_stale_callback,
            blob_announced_callback,
        );

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

async fn run_announcer_loop(
    endpoint_addr: EndpointAddr,
    secret_key: SecretKey,
    cancel: CancellationToken,
    sender: GossipSender,
    node_id: NodeId,
) {
    let mut consecutive_failures: u32 = 0;
    let mut current_interval_secs = GOSSIP_MIN_ANNOUNCE_INTERVAL_SECS;
    let mut ticker = interval(Duration::from_secs(current_interval_secs));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                tracing::debug!("gossip announcer shutting down");
                break;
            }
            _ = ticker.tick() => {
                let announcement = match PeerAnnouncement::new(node_id, endpoint_addr.clone()) {
                    Ok(announcement) => announcement,
                    Err(error) => {
                        tracing::error!("failed to create peer announcement: {}", error);
                        continue;
                    }
                };
                let signed = match SignedPeerAnnouncement::sign(announcement, &secret_key) {
                    Ok(signed) => signed,
                    Err(error) => {
                        tracing::error!("failed to sign peer announcement: {}", error);
                        continue;
                    }
                };
                let gossip_msg = GossipMessage::PeerAnnouncement(signed);
                let Ok(bytes) = gossip_msg.to_bytes() else {
                    tracing::warn!("failed to serialize signed peer announcement");
                    continue;
                };
                match sender.broadcast(bytes.into()).await {
                    Ok(()) => {
                        tracing::trace!("broadcast signed peer announcement for node_id={}", node_id);
                        if consecutive_failures > 0 {
                            consecutive_failures = 0;
                            if current_interval_secs != GOSSIP_MIN_ANNOUNCE_INTERVAL_SECS {
                                current_interval_secs = GOSSIP_MIN_ANNOUNCE_INTERVAL_SECS;
                                ticker = interval(Duration::from_secs(current_interval_secs));
                                ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                                tracing::debug!("announcement succeeded, resetting interval to {}s", current_interval_secs);
                            }
                        }
                    }
                    Err(error) => {
                        consecutive_failures = consecutive_failures.saturating_add(1);
                        tracing::warn!(
                            "failed to broadcast peer announcement (failure {}/{}): {}",
                            consecutive_failures,
                            GOSSIP_ANNOUNCE_FAILURE_THRESHOLD,
                            error
                        );
                        if consecutive_failures >= GOSSIP_ANNOUNCE_FAILURE_THRESHOLD {
                            let new_interval = (current_interval_secs * 2).min(GOSSIP_MAX_ANNOUNCE_INTERVAL_SECS);
                            if new_interval != current_interval_secs {
                                current_interval_secs = new_interval;
                                ticker = interval(Duration::from_secs(current_interval_secs));
                                ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                                tracing::info!("increasing announcement interval to {}s due to failures", current_interval_secs);
                            }
                        }
                    }
                }
            }
        }
    }
}

async fn run_receiver_loop(
    cancel: CancellationToken,
    mut receiver: GossipReceiver,
    receiver_node_id: NodeId,
    receiver_callback: Option<PeerDiscoveredCallback<EndpointAddr>>,
    local_topology_version: Arc<std::sync::atomic::AtomicU64>,
    topology_stale_callback: Option<TopologyStaleCallback>,
    blob_announced_callback: Option<BlobAnnouncedCallback>,
) {
    #[cfg(not(feature = "blob"))]
    let _ = &blob_announced_callback;

    let mut rate_limiter = GossipRateLimiter::new();
    let mut consecutive_errors: u32 = 0;
    let backoff_durations: Vec<Duration> = GOSSIP_STREAM_BACKOFF_SECS.iter().map(|s| Duration::from_secs(*s)).collect();

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                tracing::debug!("gossip receiver shutting down");
                break;
            }
            event = receiver.next() => match event {
                Some(Ok(Event::Received(msg))) => {
                    consecutive_errors = 0;
                    if let Err(reason) = rate_limiter.check(&msg.delivered_from) {
                        match reason {
                            RateLimitReason::PerPeer => tracing::trace!("rate limited gossip message from peer={:?}", msg.delivered_from),
                            RateLimitReason::Global => tracing::warn!("global gossip rate limit exceeded, dropping message from peer={:?}", msg.delivered_from),
                        }
                        continue;
                    }
                    if msg.content.len() > MAX_GOSSIP_MESSAGE_SIZE {
                        tracing::warn!(size = msg.content.len(), max = MAX_GOSSIP_MESSAGE_SIZE, peer = ?msg.delivered_from, "rejected oversized gossip message");
                        continue;
                    }
                    let Some(gossip_msg) = GossipMessage::from_bytes(&msg.content) else {
                        tracing::warn!(size = msg.content.len(), peer = ?msg.delivered_from, "failed to parse gossip message");
                        continue;
                    };
                    match gossip_msg {
                        GossipMessage::PeerAnnouncement(signed) => {
                            let Some(announcement) = signed.verify() else {
                                tracing::warn!("rejected peer announcement with invalid signature from endpoint_id={:?}", signed.announcement.endpoint_addr.id);
                                continue;
                            };
                            if announcement.node_id == receiver_node_id {
                                tracing::trace!("ignoring self-announcement");
                                continue;
                            }
                            tracing::debug!("received verified peer announcement from node_id={}, endpoint_id={:?}", announcement.node_id, announcement.endpoint_addr.id);
                            if let Some(ref callback) = receiver_callback {
                                callback(DiscoveredPeer {
                                    node_id: announcement.node_id,
                                    address: announcement.endpoint_addr.clone(),
                                    timestamp_micros: announcement.timestamp_micros,
                                }).await;
                            }
                            let relay_urls: Vec<_> = announcement.endpoint_addr.relay_urls().collect();
                            tracing::info!(
                                "discovered verified peer: node_id={}, endpoint_id={:?}, relay={:?}, direct_addresses={}",
                                announcement.node_id,
                                announcement.endpoint_addr.id,
                                relay_urls,
                                announcement.endpoint_addr.addrs.len()
                            );
                        }
                        GossipMessage::TopologyAnnouncement(signed) => {
                            let Some(announcement) = signed.verify() else {
                                tracing::warn!("rejected topology announcement with invalid signature from node_id={}", signed.announcement.node_id);
                                continue;
                            };
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
                            let local_version = local_topology_version.load(Ordering::SeqCst);
                            if announcement.topology_version > local_version {
                                tracing::info!(local_version, remote_version = announcement.topology_version, remote_node = announcement.node_id, "detected stale topology, triggering sync");
                                if let Some(ref callback) = topology_stale_callback {
                                    callback(StaleTopologyInfo {
                                        announcing_node_id: announcement.node_id,
                                        remote_version: announcement.topology_version,
                                        remote_hash: announcement.topology_hash,
                                        remote_term: announcement.term,
                                    }).await;
                                }
                            } else {
                                tracing::trace!(local_version, remote_version = announcement.topology_version, "topology announcement: local version is current");
                            }
                        }
                        #[cfg(feature = "blob")]
                        GossipMessage::BlobAnnouncement(signed) => {
                            let Some(announcement) = signed.verify() else {
                                tracing::warn!("rejected blob announcement with invalid signature from node_id={}", u64::from(signed.announcement.node_id));
                                continue;
                            };
                            if u64::from(announcement.node_id) == u64::from(receiver_node_id) {
                                tracing::trace!("ignoring self-blob-announcement");
                                continue;
                            }
                            tracing::info!(hash = %announcement.blob_hash.fmt_short(), size = announcement.blob_size, format = ?announcement.blob_format, node_id = u64::from(announcement.node_id), tag = ?announcement.tag, "discovered blob available from peer");
                            if let Some(ref callback) = blob_announced_callback {
                                callback(BlobAnnouncedInfo {
                                    announcing_node_id: u64::from(announcement.node_id),
                                    provider_public_key: announcement.endpoint_addr.id,
                                    blob_hash_hex: announcement.blob_hash.to_hex().to_string(),
                                    blob_size: announcement.blob_size,
                                    is_raw_format: matches!(announcement.blob_format, iroh_blobs::BlobFormat::Raw),
                                    tag: announcement.tag.clone(),
                                }).await;
                            }
                        }
                    }
                }
                Some(Ok(Event::NeighborUp(neighbor_id))) => tracing::debug!("neighbor up: {:?}", neighbor_id),
                Some(Ok(Event::NeighborDown(neighbor_id))) => tracing::debug!("neighbor down: {:?}", neighbor_id),
                Some(Ok(Event::Lagged)) => tracing::warn!("gossip receiver lagged, messages may be lost"),
                Some(Err(error)) => {
                    consecutive_errors = consecutive_errors.saturating_add(1);
                    if consecutive_errors > GOSSIP_MAX_STREAM_RETRIES {
                        tracing::error!("gossip receiver exceeded max retries ({}), giving up: {}", GOSSIP_MAX_STREAM_RETRIES, error);
                        break;
                    }
                    let backoff = calculate_backoff_duration(consecutive_errors.saturating_sub(1), &backoff_durations);
                    tracing::warn!("gossip receiver error (retry {}/{}), backing off for {:?}: {}", consecutive_errors, GOSSIP_MAX_STREAM_RETRIES, backoff, error);
                    tokio::time::sleep(backoff).await;
                }
                None => {
                    tracing::info!("gossip receiver stream ended");
                    break;
                }
            }
        }
    }
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
