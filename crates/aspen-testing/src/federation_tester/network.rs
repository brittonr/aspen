//! Network partition simulation for federation testing.

use std::collections::HashSet;

use parking_lot::RwLock;
use tracing::info;

/// Network partition state for simulating cluster isolation.
#[derive(Debug, Default)]
pub struct NetworkPartitions {
    /// Set of partitioned cluster pairs (a, b) where a < b lexicographically.
    partitions: RwLock<HashSet<(String, String)>>,
}

impl NetworkPartitions {
    /// Create a new partition tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Partition two clusters (bidirectional).
    pub fn partition(&self, cluster_a: &str, cluster_b: &str) {
        let pair = Self::ordered_pair(cluster_a, cluster_b);
        self.partitions.write().insert(pair);
        info!(a = %cluster_a, b = %cluster_b, "network partition created");
    }

    /// Heal a partition between two clusters.
    pub fn heal(&self, cluster_a: &str, cluster_b: &str) {
        let pair = Self::ordered_pair(cluster_a, cluster_b);
        self.partitions.write().remove(&pair);
        info!(a = %cluster_a, b = %cluster_b, "network partition healed");
    }

    /// Check if two clusters can communicate.
    pub fn can_communicate(&self, cluster_a: &str, cluster_b: &str) -> bool {
        let pair = Self::ordered_pair(cluster_a, cluster_b);
        !self.partitions.read().contains(&pair)
    }

    /// Heal all partitions.
    pub fn heal_all(&self) {
        let count = self.partitions.read().len();
        self.partitions.write().clear();
        info!(count, "healed all network partitions");
    }

    fn ordered_pair(a: &str, b: &str) -> (String, String) {
        if a < b {
            (a.to_string(), b.to_string())
        } else {
            (b.to_string(), a.to_string())
        }
    }
}
