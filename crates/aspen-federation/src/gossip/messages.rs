//! Federation gossip message types and signed message wrapper.

use anyhow::Context;
use anyhow::Result;
use aspen_core::Signature;
use aspen_core::hlc::SerializableTimestamp;
use iroh::PublicKey;
use serde::Deserialize;
use serde::Serialize;
use tracing::debug;

use super::MAX_MESSAGE_SIZE;
use crate::app_registry::AppManifest;
use crate::identity::ClusterIdentity;

/// Federation gossip message types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FederationGossipMessage {
    /// Announce that a cluster is online.
    ClusterOnline {
        /// Protocol version.
        version: u8,
        /// Cluster public key.
        cluster_key: [u8; 32],
        /// Human-readable cluster name.
        cluster_name: String,
        /// Iroh node public keys for connectivity.
        node_keys: Vec<[u8; 32]>,
        /// Relay URLs for NAT traversal.
        relay_urls: Vec<String>,
        /// Applications installed on this cluster.
        apps: Vec<AppManifest>,
        /// Legacy capabilities (for backwards compatibility).
        #[serde(default)]
        capabilities: Vec<String>,
        /// HLC timestamp when announced.
        hlc_timestamp: SerializableTimestamp,
    },

    /// Announce that a cluster is seeding a resource.
    ResourceSeeding {
        /// Protocol version.
        version: u8,
        /// Federated resource ID origin cluster key.
        fed_id_origin: [u8; 32],
        /// Federated resource ID local identifier.
        fed_id_local: [u8; 32],
        /// Cluster public key.
        cluster_key: [u8; 32],
        /// Node keys for fetching.
        node_keys: Vec<[u8; 32]>,
        /// Current ref heads (for sync comparison).
        ref_heads: Vec<(String, [u8; 32])>,
        /// HLC timestamp when announced.
        hlc_timestamp: SerializableTimestamp,
    },

    /// Announce an update to a resource.
    ResourceUpdate {
        /// Protocol version.
        version: u8,
        /// Federated resource ID origin cluster key.
        fed_id_origin: [u8; 32],
        /// Federated resource ID local identifier.
        fed_id_local: [u8; 32],
        /// Cluster public key (who made the update).
        cluster_key: [u8; 32],
        /// Update type (e.g., "ref_update", "cob_change").
        update_type: String,
        /// Updated ref heads.
        ref_heads: Vec<(String, [u8; 32])>,
        /// HLC timestamp when announced.
        hlc_timestamp: SerializableTimestamp,
    },
}

impl FederationGossipMessage {
    /// Get the cluster key from any message type.
    pub fn cluster_key(&self) -> Option<PublicKey> {
        let bytes = match self {
            Self::ClusterOnline { cluster_key, .. } => cluster_key,
            Self::ResourceSeeding { cluster_key, .. } => cluster_key,
            Self::ResourceUpdate { cluster_key, .. } => cluster_key,
        };
        PublicKey::from_bytes(bytes).ok()
    }

    /// Get the HLC timestamp from any message type.
    pub fn hlc_timestamp(&self) -> &SerializableTimestamp {
        match self {
            Self::ClusterOnline { hlc_timestamp, .. } => hlc_timestamp,
            Self::ResourceSeeding { hlc_timestamp, .. } => hlc_timestamp,
            Self::ResourceUpdate { hlc_timestamp, .. } => hlc_timestamp,
        }
    }
}

/// Signed federation gossip message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedFederationMessage {
    /// The message payload.
    pub message: FederationGossipMessage,
    /// Ed25519 signature over the serialized message (cluster key).
    pub signature: Signature,
}

impl SignedFederationMessage {
    /// Sign a message with the cluster's secret key.
    pub fn sign(message: FederationGossipMessage, identity: &ClusterIdentity) -> Result<Self> {
        let message_bytes = postcard::to_allocvec(&message).context("failed to serialize message for signing")?;
        let signature = identity.sign(&message_bytes);

        Ok(Self { message, signature })
    }

    /// Verify the signature and return the message if valid.
    ///
    /// Tiger Style: Debug logging added for verification failures without
    /// exposing details in the API (security: don't leak verification reasons).
    pub fn verify(&self) -> Option<&FederationGossipMessage> {
        let cluster_key = self.message.cluster_key()?;

        let message_bytes = match postcard::to_allocvec(&self.message) {
            Ok(bytes) => bytes,
            Err(e) => {
                debug!(error = %e, "federation gossip: failed to serialize message for verification");
                return None;
            }
        };

        let sig_bytes: [u8; 64] = match self.signature.0.try_into() {
            Ok(bytes) => bytes,
            Err(_) => {
                debug!("federation gossip: invalid signature length");
                return None;
            }
        };
        let sig = iroh::Signature::from_bytes(&sig_bytes);

        match cluster_key.verify(&message_bytes, &sig) {
            Ok(()) => Some(&self.message),
            Err(_) => {
                debug!("federation gossip: signature verification failed");
                None
            }
        }
    }

    /// Serialize to bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let bytes = postcard::to_allocvec(self).context("failed to serialize signed message")?;
        if bytes.len() > MAX_MESSAGE_SIZE {
            anyhow::bail!("message too large: {} > {}", bytes.len(), MAX_MESSAGE_SIZE);
        }
        Ok(bytes)
    }

    /// Deserialize from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > MAX_MESSAGE_SIZE {
            return None;
        }
        postcard::from_bytes(bytes).ok()
    }
}
