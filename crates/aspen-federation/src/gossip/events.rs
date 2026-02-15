//! Federation gossip event types.

use std::collections::HashMap;

use iroh::PublicKey;

use crate::discovery::DiscoveredCluster;
use crate::types::FederatedId;

/// Event from the federation gossip service.
#[derive(Debug, Clone)]
pub enum FederationEvent {
    /// A cluster came online.
    ClusterOnline(DiscoveredCluster),
    /// A cluster is seeding a resource.
    ResourceSeeding {
        /// The federated resource ID.
        fed_id: FederatedId,
        /// The cluster seeding this resource.
        cluster_key: PublicKey,
        /// Node keys available for fetching.
        node_keys: Vec<PublicKey>,
        /// Current ref heads for sync comparison.
        ref_heads: HashMap<String, [u8; 32]>,
    },
    /// A resource was updated.
    ResourceUpdate {
        /// The federated resource ID.
        fed_id: FederatedId,
        /// The cluster that made the update.
        cluster_key: PublicKey,
        /// Type of update (e.g., "ref_update", "cob_change").
        update_type: String,
        /// Updated ref heads.
        ref_heads: HashMap<String, [u8; 32]>,
    },
}
