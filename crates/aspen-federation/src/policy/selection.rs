//! Cluster selection strategies for federation.
//!
//! Controls how clusters are ranked and selected when multiple seeders
//! are available for a resource. Different use cases benefit from
//! different strategies.

use std::sync::Arc;
use std::time::Instant;

use serde::Deserialize;
use serde::Serialize;

use crate::discovery::DiscoveredCluster;
use crate::discovery::DiscoveredSeeder;
use crate::trust::TrustLevel;
use crate::trust::TrustManager;

// ============================================================================
// Types
// ============================================================================

/// Strategy for selecting which cluster to sync from.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionStrategy {
    /// Composite score based on trust, freshness, and other factors (default).
    #[default]
    Scored,
    /// Prefer seeders with the most recent discovery/announcement.
    FreshestFirst,
    /// Prefer seeders with lowest expected latency.
    LowestLatency,
    /// Prefer seeders closest in trust graph.
    TrustProximity,
    /// Random selection (useful for load spreading).
    Random,
}

/// A seeder with a computed ranking score.
#[derive(Debug, Clone)]
pub struct RankedSeeder {
    /// The original discovered seeder.
    pub seeder: DiscoveredSeeder,
    /// Composite score (higher = preferred). Range: 0.0 to 1.0.
    pub score: f64,
    /// Human-readable reason for the ranking (for debugging/logging).
    pub reason: String,
}

/// A cluster with a computed ranking score (for proxy routing).
#[derive(Debug, Clone)]
pub struct RankedCluster {
    /// The original discovered cluster.
    pub cluster: DiscoveredCluster,
    /// Composite score (higher = preferred). Range: 0.0 to 1.0.
    pub score: f64,
    /// Human-readable reason for the ranking (for debugging/logging).
    pub reason: String,
}

// ============================================================================
// Scoring Constants (Tiger Style: fixed, documented)
// ============================================================================

/// Weight for trust score in composite ranking.
const TRUST_WEIGHT: f64 = 0.40;

/// Weight for freshness score in composite ranking.
const FRESHNESS_WEIGHT: f64 = 0.30;

/// Weight for ref coverage score in composite ranking.
const COVERAGE_WEIGHT: f64 = 0.20;

/// Weight for node count score in composite ranking.
const NODE_COUNT_WEIGHT: f64 = 0.10;

/// Maximum age (seconds) before a seeder's freshness score hits zero.
const MAX_FRESHNESS_AGE_SECS: f64 = 3600.0; // 1 hour

// ============================================================================
// Cluster Selector Trait
// ============================================================================

/// Trait for custom cluster selection logic.
///
/// Implement this for application-specific ranking beyond the built-in
/// strategies. Used via `SelectionStrategy::Custom`.
pub trait ClusterSelector: Send + Sync {
    /// Rank available seeders for a resource.
    ///
    /// Returns seeders in preferred order (highest score first), potentially filtered.
    fn rank_seeders(&self, seeders: &[DiscoveredSeeder], trust_manager: &TrustManager) -> Vec<RankedSeeder>;
}

// ============================================================================
// Built-in Selector Implementation
// ============================================================================

/// Default cluster selector that implements the built-in strategies.
pub struct DefaultClusterSelector {
    strategy: SelectionStrategy,
}

impl DefaultClusterSelector {
    /// Create a new selector with the given strategy.
    pub fn new(strategy: SelectionStrategy) -> Self {
        Self { strategy }
    }

    /// Rank seeders using the configured strategy.
    pub fn rank(&self, seeders: &[DiscoveredSeeder], trust_manager: &TrustManager) -> Vec<RankedSeeder> {
        let mut ranked: Vec<RankedSeeder> = seeders.iter().map(|s| self.score_seeder(s, trust_manager)).collect();

        match &self.strategy {
            SelectionStrategy::Scored => {
                ranked.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
            }
            SelectionStrategy::FreshestFirst => {
                ranked.sort_by(|a, b| {
                    a.seeder
                        .discovered_at
                        .elapsed()
                        .partial_cmp(&b.seeder.discovered_at.elapsed())
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            SelectionStrategy::TrustProximity => {
                ranked.sort_by(|a, b| {
                    let ta = trust_score(trust_manager.trust_level(&a.seeder.cluster_key));
                    let tb = trust_score(trust_manager.trust_level(&b.seeder.cluster_key));
                    tb.partial_cmp(&ta)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal))
                });
            }
            SelectionStrategy::LowestLatency => {
                // No latency data yet — fall back to freshest (most recently seen = likely closest)
                ranked.sort_by(|a, b| {
                    a.seeder
                        .discovered_at
                        .elapsed()
                        .partial_cmp(&b.seeder.discovered_at.elapsed())
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            SelectionStrategy::Random => {
                // Deterministic "random" based on cluster key XOR with time
                let now_nanos = Instant::now().elapsed().as_nanos() as u64;
                ranked.sort_by_key(|r| {
                    let key_bytes = r.seeder.cluster_key.as_bytes();
                    let mix = u64::from_le_bytes(key_bytes[..8].try_into().unwrap_or([0; 8]));
                    mix ^ now_nanos
                });
            }
        }

        ranked
    }

    /// Score a single seeder using the composite scoring model.
    fn score_seeder(&self, seeder: &DiscoveredSeeder, trust_manager: &TrustManager) -> RankedSeeder {
        let trust = trust_score(trust_manager.trust_level(&seeder.cluster_key));
        let freshness = freshness_score(seeder.discovered_at);
        let coverage = coverage_score(seeder);
        let node_count = node_count_score(seeder);

        let composite = (trust * TRUST_WEIGHT)
            + (freshness * FRESHNESS_WEIGHT)
            + (coverage * COVERAGE_WEIGHT)
            + (node_count * NODE_COUNT_WEIGHT);

        let reason = format!("trust={:.2} fresh={:.2} cover={:.2} nodes={:.2}", trust, freshness, coverage, node_count);

        RankedSeeder {
            seeder: seeder.clone(),
            score: composite,
            reason,
        }
    }
}

impl DefaultClusterSelector {
    /// Rank discovered clusters for proxy routing.
    ///
    /// Unlike `rank()` which works with seeders (per-resource),
    /// this ranks clusters (per-app) for request forwarding.
    pub fn rank_clusters(&self, clusters: &[DiscoveredCluster], trust_manager: &TrustManager) -> Vec<RankedCluster> {
        let mut ranked: Vec<RankedCluster> = clusters.iter().map(|c| self.score_cluster(c, trust_manager)).collect();

        match &self.strategy {
            SelectionStrategy::Scored => {
                ranked.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
            }
            SelectionStrategy::FreshestFirst => {
                ranked.sort_by(|a, b| {
                    a.cluster
                        .discovered_at
                        .elapsed()
                        .partial_cmp(&b.cluster.discovered_at.elapsed())
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            SelectionStrategy::TrustProximity => {
                ranked.sort_by(|a, b| {
                    let ta = trust_score(trust_manager.trust_level(&a.cluster.cluster_key));
                    let tb = trust_score(trust_manager.trust_level(&b.cluster.cluster_key));
                    tb.partial_cmp(&ta)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal))
                });
            }
            SelectionStrategy::LowestLatency | SelectionStrategy::Random => {
                ranked.sort_by(|a, b| {
                    a.cluster
                        .discovered_at
                        .elapsed()
                        .partial_cmp(&b.cluster.discovered_at.elapsed())
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
        }

        ranked
    }

    /// Score a single cluster for proxy routing.
    fn score_cluster(&self, cluster: &DiscoveredCluster, trust_manager: &TrustManager) -> RankedCluster {
        let trust = trust_score(trust_manager.trust_level(&cluster.cluster_key));
        let freshness = freshness_score(cluster.discovered_at);
        let capability = capability_score(cluster);
        let nodes = cluster_node_score(cluster);

        let composite = (trust * TRUST_WEIGHT)
            + (freshness * FRESHNESS_WEIGHT)
            + (capability * COVERAGE_WEIGHT)
            + (nodes * NODE_COUNT_WEIGHT);

        let reason = format!("trust={:.2} fresh={:.2} caps={:.2} nodes={:.2}", trust, freshness, capability, nodes);

        RankedCluster {
            cluster: cluster.clone(),
            score: composite,
            reason,
        }
    }
}

impl ClusterSelector for DefaultClusterSelector {
    fn rank_seeders(&self, seeders: &[DiscoveredSeeder], trust_manager: &TrustManager) -> Vec<RankedSeeder> {
        self.rank(seeders, trust_manager)
    }
}

// ============================================================================
// Scoring Functions (pure, deterministic except for Instant::now)
// ============================================================================

/// Score based on trust level. Trusted > Public > Blocked.
fn trust_score(level: TrustLevel) -> f64 {
    match level {
        TrustLevel::Trusted => 1.0,
        TrustLevel::Public => 0.3,
        TrustLevel::Blocked => 0.0,
    }
}

/// Score based on how recently the seeder was discovered.
/// 1.0 = just discovered, decays linearly to 0.0 at MAX_FRESHNESS_AGE_SECS.
fn freshness_score(discovered_at: Instant) -> f64 {
    let age_secs = discovered_at.elapsed().as_secs_f64();
    (1.0 - (age_secs / MAX_FRESHNESS_AGE_SECS)).max(0.0)
}

/// Score based on how many refs the seeder has (more = better coverage).
fn coverage_score(seeder: &DiscoveredSeeder) -> f64 {
    let ref_count = seeder.ref_heads.len() as f64;
    // Diminishing returns: 1 ref = 0.5, 5 refs = 0.83, 10+ refs ≈ 1.0
    1.0 - (1.0 / (1.0 + ref_count))
}

/// Score based on number of accessible nodes (more = higher availability).
fn node_count_score(seeder: &DiscoveredSeeder) -> f64 {
    let count = seeder.node_keys.len() as f64;
    // 0 nodes = 0.0, 1 node = 0.5, 3+ nodes ≈ 1.0
    1.0 - (1.0 / (1.0 + count))
}

/// Score based on number of capabilities a cluster has (more = richer).
fn capability_score(cluster: &DiscoveredCluster) -> f64 {
    let cap_count = cluster.capabilities.len() as f64;
    // Diminishing returns: 1 cap = 0.5, 5 caps = 0.83, 10+ caps ≈ 1.0
    1.0 - (1.0 / (1.0 + cap_count))
}

/// Score based on number of accessible nodes on a cluster.
fn cluster_node_score(cluster: &DiscoveredCluster) -> f64 {
    let count = cluster.node_keys.len() as f64;
    1.0 - (1.0 / (1.0 + count))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::time::Instant;

    use iroh::PublicKey;

    use super::*;
    use crate::discovery::DiscoveredSeeder;
    use crate::types::FederatedId;

    fn test_key() -> PublicKey {
        iroh::SecretKey::generate(&mut rand::rng()).public()
    }

    fn test_fed_id() -> FederatedId {
        FederatedId::new(test_key(), [0xab; 32])
    }

    fn make_seeder(cluster_key: PublicKey, ref_count: usize, node_count: usize) -> DiscoveredSeeder {
        let mut ref_heads = HashMap::new();
        for i in 0..ref_count {
            ref_heads.insert(format!("ref_{}", i), [i as u8; 32]);
        }

        let node_keys: Vec<PublicKey> = (0..node_count).map(|_| test_key()).collect();

        DiscoveredSeeder {
            fed_id: test_fed_id(),
            cluster_key,
            node_keys,
            relay_urls: vec![],
            ref_heads,
            discovered_at: Instant::now(),
        }
    }

    #[test]
    fn test_trust_score_values() {
        assert_eq!(trust_score(TrustLevel::Trusted), 1.0);
        assert_eq!(trust_score(TrustLevel::Public), 0.3);
        assert_eq!(trust_score(TrustLevel::Blocked), 0.0);
    }

    #[test]
    fn test_freshness_score_just_discovered() {
        let score = freshness_score(Instant::now());
        assert!(score > 0.99, "just discovered should be ~1.0, got {}", score);
    }

    #[test]
    fn test_coverage_score_empty() {
        let seeder = make_seeder(test_key(), 0, 1);
        let score = coverage_score(&seeder);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_coverage_score_many_refs() {
        let seeder = make_seeder(test_key(), 10, 1);
        let score = coverage_score(&seeder);
        assert!(score > 0.9, "10 refs should score >0.9, got {}", score);
    }

    #[test]
    fn test_selector_trusted_ranks_higher() {
        let trust = TrustManager::new();
        let trusted_key = test_key();
        let public_key = test_key();

        trust.add_trusted(trusted_key, "trusted".to_string(), None);

        let seeders = vec![make_seeder(public_key, 5, 2), make_seeder(trusted_key, 5, 2)];

        let selector = DefaultClusterSelector::new(SelectionStrategy::Scored);
        let ranked = selector.rank(&seeders, &trust);

        // Trusted should rank first
        assert_eq!(ranked[0].seeder.cluster_key, trusted_key);
        assert!(ranked[0].score > ranked[1].score);
    }

    #[test]
    fn test_selector_blocked_ranks_last() {
        let trust = TrustManager::new();
        let blocked_key = test_key();
        let public_key = test_key();

        trust.block(blocked_key);

        let seeders = vec![make_seeder(blocked_key, 5, 2), make_seeder(public_key, 5, 2)];

        let selector = DefaultClusterSelector::new(SelectionStrategy::Scored);
        let ranked = selector.rank(&seeders, &trust);

        // Blocked should rank last
        assert_eq!(ranked.last().unwrap().seeder.cluster_key, blocked_key);
    }

    #[test]
    fn test_selection_strategy_roundtrip() {
        let strategies = vec![
            SelectionStrategy::Scored,
            SelectionStrategy::FreshestFirst,
            SelectionStrategy::LowestLatency,
            SelectionStrategy::TrustProximity,
            SelectionStrategy::Random,
        ];

        for strategy in strategies {
            let bytes = postcard::to_allocvec(&strategy).unwrap();
            let parsed: SelectionStrategy = postcard::from_bytes(&bytes).unwrap();
            // Just verify it round-trips without error
            assert!(!format!("{:?}", parsed).is_empty());
        }
    }
}
