//! Federation gossip announcer loop and resource announcement methods.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use aspen_core::hlc::HLC;
use aspen_core::hlc::SerializableTimestamp;
use iroh::Endpoint;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::trace;
use tracing::warn;

use super::CLUSTER_ANNOUNCE_INTERVAL;
use super::FEDERATION_GOSSIP_VERSION;
use super::FederationGossipService;
use super::messages::FederationGossipMessage;
use super::messages::SignedFederationMessage;
use crate::app_registry::AppManifest;
use crate::app_registry::SharedAppRegistry;
use crate::identity::ClusterIdentity;
use crate::types::FederatedId;

impl FederationGossipService {
    /// Announcer loop - periodically announce cluster presence.
    pub(super) async fn announcer_loop(
        identity: ClusterIdentity,
        sender: iroh_gossip::api::GossipSender,
        endpoint: Arc<Endpoint>,
        hlc: Arc<HLC>,
        app_registry: Option<SharedAppRegistry>,
        cancel: CancellationToken,
    ) {
        let mut interval = tokio::time::interval(CLUSTER_ANNOUNCE_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    debug!("federation gossip announcer shutting down");
                    break;
                }
                _ = interval.tick() => {
                    let node_keys = vec![*endpoint.id().as_bytes()];
                    let relay_urls: Vec<String> = endpoint
                        .addr()
                        .relay_urls()
                        .map(|u| u.to_string())
                        .collect();

                    // Get apps from registry or use default Forge app for backwards compatibility
                    let apps = match &app_registry {
                        Some(registry) => {
                            let registered = registry.to_announcement_list();
                            if registered.is_empty() {
                                // No apps registered - use default Forge
                                vec![Self::default_forge_app()]
                            } else {
                                registered
                            }
                        }
                        None => {
                            // No registry provided - use default Forge
                            vec![Self::default_forge_app()]
                        }
                    };

                    // Compute legacy capabilities from apps
                    let capabilities: Vec<String> = apps
                        .iter()
                        .flat_map(|app| app.capabilities.iter().cloned())
                        .collect();

                    let message = FederationGossipMessage::ClusterOnline {
                        version: FEDERATION_GOSSIP_VERSION,
                        cluster_key: *identity.public_key().as_bytes(),
                        cluster_name: identity.name().to_string(),
                        node_keys,
                        relay_urls,
                        apps,
                        capabilities,
                        hlc_timestamp: SerializableTimestamp::from(hlc.new_timestamp()),
                    };

                    match SignedFederationMessage::sign(message, &identity) {
                        Ok(signed) => match signed.to_bytes() {
                            Ok(bytes) => {
                                if let Err(e) = sender.broadcast(bytes.into()).await {
                                    warn!(error = %e, "failed to broadcast cluster online");
                                } else {
                                    trace!(
                                        cluster = %identity.name(),
                                        "broadcast cluster online announcement"
                                    );
                                }
                            }
                            Err(e) => {
                                warn!(error = %e, "failed to serialize cluster announcement");
                            }
                        },
                        Err(e) => {
                            warn!(error = %e, "failed to sign cluster announcement");
                        }
                    }
                }
            }
        }
    }

    /// Returns the default Forge app manifest for backwards compatibility.
    ///
    /// Used when no app registry is provided or when no apps are registered.
    pub(super) fn default_forge_app() -> AppManifest {
        AppManifest::new("forge", "1.0.0")
            .with_name("Aspen Forge")
            .with_capabilities(vec!["git", "issues", "patches"])
    }

    /// Announce that we're seeding a federated resource.
    pub async fn announce_resource_seeding(
        &self,
        fed_id: &FederatedId,
        ref_heads: Vec<(String, [u8; 32])>,
    ) -> Result<()> {
        let sender = self.sender.read().clone().context("gossip sender not initialized")?;

        let node_keys = vec![*self.endpoint.id().as_bytes()];

        let message = FederationGossipMessage::ResourceSeeding {
            version: FEDERATION_GOSSIP_VERSION,
            fed_id_origin: *fed_id.origin().as_bytes(),
            fed_id_local: *fed_id.local_id(),
            cluster_key: *self.cluster_identity.public_key().as_bytes(),
            node_keys,
            ref_heads,
            hlc_timestamp: SerializableTimestamp::from(self.hlc.new_timestamp()),
        };

        let signed = SignedFederationMessage::sign(message, &self.cluster_identity)?;
        let bytes = signed.to_bytes()?;

        sender.broadcast(bytes.into()).await.context("failed to broadcast resource seeding")?;

        debug!(
            fed_id = %fed_id.short(),
            "announced resource seeding"
        );

        Ok(())
    }

    /// Announce an update to a federated resource.
    pub async fn announce_resource_update(
        &self,
        fed_id: &FederatedId,
        update_type: &str,
        ref_heads: Vec<(String, [u8; 32])>,
    ) -> Result<()> {
        let sender = self.sender.read().clone().context("gossip sender not initialized")?;

        let message = FederationGossipMessage::ResourceUpdate {
            version: FEDERATION_GOSSIP_VERSION,
            fed_id_origin: *fed_id.origin().as_bytes(),
            fed_id_local: *fed_id.local_id(),
            cluster_key: *self.cluster_identity.public_key().as_bytes(),
            update_type: update_type.to_string(),
            ref_heads,
            hlc_timestamp: SerializableTimestamp::from(self.hlc.new_timestamp()),
        };

        let signed = SignedFederationMessage::sign(message, &self.cluster_identity)?;
        let bytes = signed.to_bytes()?;

        sender.broadcast(bytes.into()).await.context("failed to broadcast resource update")?;

        debug!(
            fed_id = %fed_id.short(),
            update_type = %update_type,
            "announced resource update"
        );

        Ok(())
    }
}
