//! Gossip announcement for topology changes.

use serde::Deserialize;
use serde::Serialize;

/// Gossip announcement for topology changes.
///
/// Broadcast via gossip to inform all nodes of topology updates.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TopologyAnnouncement {
    /// Protocol version for forward compatibility.
    pub version: u8,
    /// Node ID of the announcing node.
    pub node_id: u64,
    /// Current topology version.
    pub topology_version: u64,
    /// Hash of the full topology (for consistency checking).
    pub topology_hash: u64,
    /// Raft term when this topology was committed.
    pub term: u64,
    /// Timestamp of announcement (Unix micros).
    pub timestamp_micros: u64,
}

impl TopologyAnnouncement {
    /// Current protocol version for topology announcements.
    pub const PROTOCOL_VERSION: u8 = 1;

    /// Create a new topology announcement.
    pub fn new(node_id: u64, topology_version: u64, topology_hash: u64, term: u64, timestamp_micros: u64) -> Self {
        Self {
            version: Self::PROTOCOL_VERSION,
            node_id,
            topology_version,
            topology_hash,
            term,
            timestamp_micros,
        }
    }
}
