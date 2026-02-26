//! Federation gossip receiver loop.

use std::collections::HashMap;
use std::time::Instant;

use iroh::PublicKey;
use iroh_gossip::api::Event;
use n0_future::StreamExt;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::info;
use tracing::trace;
use tracing::warn;

use super::FederationGossipService;
use super::events::FederationEvent;
use super::messages::FederationGossipMessage;
use super::messages::SignedFederationMessage;
use super::rate_limiter::FederationRateLimiter;
use crate::discovery::DiscoveredCluster;
use crate::types::FederatedId;

impl FederationGossipService {
    /// Receiver loop - process incoming gossip messages.
    pub(super) async fn receiver_loop(
        mut receiver: iroh_gossip::api::GossipReceiver,
        event_tx: mpsc::Sender<FederationEvent>,
        cancel: CancellationToken,
    ) {
        let mut rate_limiter = FederationRateLimiter::new();

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    debug!("federation gossip receiver shutting down");
                    break;
                }
                event = receiver.next() => {
                    match event {
                        Some(Ok(Event::Received(msg))) => {
                            // Parse and verify message
                            let signed = match SignedFederationMessage::from_bytes(&msg.content) {
                                Some(s) => s,
                                None => {
                                    trace!("failed to parse federation gossip message");
                                    continue;
                                }
                            };

                            // Get cluster key and rate limit check
                            let cluster_key = match signed.message.cluster_key() {
                                Some(k) => k,
                                None => continue,
                            };

                            if !rate_limiter.check(&cluster_key) {
                                trace!(
                                    cluster = %cluster_key,
                                    "rate limited federation message"
                                );
                                continue;
                            }

                            // Verify signature
                            let message = match signed.verify() {
                                Some(m) => m,
                                None => {
                                    warn!(
                                        cluster = %cluster_key,
                                        "rejected federation message with invalid signature"
                                    );
                                    continue;
                                }
                            };

                            // Process message
                            let event = match message {
                                FederationGossipMessage::ClusterOnline {
                                    cluster_key,
                                    cluster_name,
                                    node_keys,
                                    relay_urls,
                                    apps,
                                    capabilities,
                                    hlc_timestamp,
                                    ..
                                } => {
                                    let pk = match PublicKey::from_bytes(cluster_key) {
                                        Ok(k) => k,
                                        Err(_) => continue,
                                    };
                                    let node_keys: Vec<PublicKey> = node_keys
                                        .iter()
                                        .filter_map(|k| PublicKey::from_bytes(k).ok())
                                        .collect();

                                    info!(
                                        cluster_name = %cluster_name,
                                        cluster_key = %pk,
                                        nodes = node_keys.len(),
                                        apps = apps.len(),
                                        "discovered federated cluster via gossip"
                                    );

                                    FederationEvent::ClusterOnline(DiscoveredCluster {
                                        cluster_key: pk,
                                        name: cluster_name.clone(),
                                        node_keys,
                                        relay_urls: relay_urls.clone(),
                                        apps: apps.clone(),
                                        capabilities: capabilities.clone(),
                                        discovered_at: Instant::now(),
                                        announced_at_hlc: hlc_timestamp.clone(),
                                    })
                                }

                                FederationGossipMessage::ResourceSeeding {
                                    fed_id_origin,
                                    fed_id_local,
                                    cluster_key,
                                    node_keys,
                                    ref_heads,
                                    ..
                                } => {
                                    let origin = match PublicKey::from_bytes(fed_id_origin) {
                                        Ok(k) => k,
                                        Err(_) => continue,
                                    };
                                    let fed_id = FederatedId::new(origin, *fed_id_local);
                                    let ck = match PublicKey::from_bytes(cluster_key) {
                                        Ok(k) => k,
                                        Err(_) => continue,
                                    };
                                    let node_keys: Vec<PublicKey> = node_keys
                                        .iter()
                                        .filter_map(|k| PublicKey::from_bytes(k).ok())
                                        .collect();
                                    let ref_map: HashMap<String, [u8; 32]> =
                                        ref_heads.iter().cloned().collect();

                                    debug!(
                                        fed_id = %fed_id.short(),
                                        cluster = %ck,
                                        "resource seeding announcement"
                                    );

                                    FederationEvent::ResourceSeeding {
                                        fed_id,
                                        cluster_key: ck,
                                        node_keys,
                                        ref_heads: ref_map,
                                    }
                                }

                                FederationGossipMessage::ResourceUpdate {
                                    fed_id_origin,
                                    fed_id_local,
                                    cluster_key,
                                    update_type,
                                    ref_heads,
                                    ..
                                } => {
                                    let origin = match PublicKey::from_bytes(fed_id_origin) {
                                        Ok(k) => k,
                                        Err(_) => continue,
                                    };
                                    let fed_id = FederatedId::new(origin, *fed_id_local);
                                    let ck = match PublicKey::from_bytes(cluster_key) {
                                        Ok(k) => k,
                                        Err(_) => continue,
                                    };
                                    let ref_map: HashMap<String, [u8; 32]> =
                                        ref_heads.iter().cloned().collect();

                                    debug!(
                                        fed_id = %fed_id.short(),
                                        cluster = %ck,
                                        update_type = %update_type,
                                        "resource update announcement"
                                    );

                                    FederationEvent::ResourceUpdate {
                                        fed_id,
                                        cluster_key: ck,
                                        update_type: update_type.clone(),
                                        ref_heads: ref_map,
                                    }
                                }
                            };

                            // Send event (non-blocking)
                            if event_tx.try_send(event).is_err() {
                                warn!("federation event channel full, dropping event");
                            }
                        }
                        Some(Ok(Event::NeighborUp(peer))) => {
                            debug!(peer = %peer, "federation gossip neighbor up");
                        }
                        Some(Ok(Event::NeighborDown(peer))) => {
                            debug!(peer = %peer, "federation gossip neighbor down");
                        }
                        Some(Ok(Event::Lagged)) => {
                            warn!("federation gossip lagged, messages may be lost");
                        }
                        Some(Err(e)) => {
                            warn!(error = %e, "federation gossip receiver error");
                        }
                        None => {
                            info!("federation gossip stream ended");
                            break;
                        }
                    }
                }
            }
        }
    }
}
