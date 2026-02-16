//! P2P locality-based job affinity for optimized job routing.
//!
//! This module provides job affinity features that leverage Aspen's P2P networking
//! to route jobs to workers based on network proximity, data locality, or other
//! affinity rules.

use std::collections::HashMap;

/// Maximum keys to consider for FollowData affinity (Tiger Style bound).
const MAX_AFFINITY_KEYS: usize = 100;
use std::sync::Arc;
use std::time::Duration;

use iroh::PublicKey as NodeId;
use serde::Deserialize;
use serde::Serialize;
use tracing::info;

use crate::error::Result;
use crate::job::Job;
use crate::job::JobId;
use crate::job::JobSpec;
use crate::manager::JobManager;

/// Affinity strategy for job placement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AffinityStrategy {
    /// No affinity - job can run anywhere.
    None,
    /// Prefer worker closest to specified node in P2P network.
    ClosestTo(NodeId),
    /// Prefer specific worker by ID.
    PreferWorker(String),
    /// Prefer specific node for execution.
    PreferNode(NodeId),
    /// Avoid specific node (for load balancing or fault isolation).
    AvoidNode(NodeId),
    /// Prefer workers with specific tags.
    RequireTags(Vec<String>),
    /// Data locality - job should run where data is stored.
    DataLocality {
        /// Hash of the data in iroh-blobs.
        blob_hash: String,
    },
    /// Follow data - place job near its input data.
    FollowData {
        /// Keys that the job will access.
        keys: Vec<String>,
    },
    /// Geographic affinity based on region.
    Geographic {
        /// Preferred region (e.g., "us-west", "eu-central").
        region: String,
    },
    /// Load-based - prefer least loaded worker.
    LeastLoaded,
    /// Avoid the Raft leader node.
    ///
    /// This strategy is designed for CI jobs to prevent resource contention
    /// with Raft consensus operations. Jobs with this affinity will be
    /// scheduled on follower nodes when possible.
    ///
    /// The leader node is determined at job scheduling time by querying
    /// Raft metrics. If leader information is unavailable, falls back to
    /// LeastLoaded strategy.
    AvoidLeader,
    /// Composite strategy - combine multiple strategies.
    Composite {
        /// List of strategies to apply in order.
        strategies: Vec<AffinityStrategy>,
    },
}

/// Job affinity configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobAffinity {
    /// Affinity strategy to use.
    pub strategy: AffinityStrategy,
    /// Whether affinity is strict (required) or soft (preferred).
    pub strict: bool,
    /// Timeout for finding affinity match before falling back.
    pub timeout: Duration,
    /// Weight for affinity scoring (0.0 to 1.0).
    pub weight: f32,
}

impl Default for JobAffinity {
    fn default() -> Self {
        Self {
            strategy: AffinityStrategy::None,
            strict: false,
            timeout: Duration::from_secs(5),
            weight: 0.7,
        }
    }
}

impl JobAffinity {
    /// Create a new job affinity configuration.
    pub fn new(strategy: AffinityStrategy) -> Self {
        Self {
            strategy,
            ..Default::default()
        }
    }

    /// Set strict affinity (must match or job fails).
    pub fn strict(mut self) -> Self {
        self.strict = true;
        self
    }

    /// Set soft affinity (best effort).
    pub fn soft(mut self) -> Self {
        self.strict = false;
        self
    }

    /// Set affinity weight for scoring.
    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight.clamp(0.0, 1.0);
        self
    }

    /// Create an affinity that avoids the Raft leader node.
    ///
    /// This is the recommended affinity for CI jobs to prevent resource
    /// contention with Raft consensus operations.
    pub fn avoid_leader() -> Self {
        Self {
            strategy: AffinityStrategy::AvoidLeader,
            strict: false, // Soft preference - run on leader if no followers available
            timeout: Duration::from_secs(5),
            weight: 0.9, // High weight to strongly prefer non-leader nodes
        }
    }

    /// Create an affinity that avoids a specific node.
    pub fn avoid_node(node_id: NodeId) -> Self {
        Self {
            strategy: AffinityStrategy::AvoidNode(node_id),
            strict: false,
            timeout: Duration::from_secs(5),
            weight: 0.8,
        }
    }
}

/// Worker metadata for affinity matching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerMetadata {
    /// Worker ID.
    pub id: String,
    /// Node ID in P2P network.
    pub node_id: NodeId,
    /// Worker tags/capabilities.
    pub tags: Vec<String>,
    /// Geographic region.
    pub region: Option<String>,
    /// Current load (0.0 to 1.0).
    pub load: f32,
    /// Blobs available locally.
    pub local_blobs: Vec<String>,
    /// Network latency to other nodes (ms).
    pub latencies: HashMap<NodeId, u32>,
    /// Shards hosted by this worker's node (for shard-aware routing).
    /// Empty means non-sharded or worker on a node not hosting any shard.
    pub local_shards: Vec<u32>,
}

/// Compute shard ID for a key using jump consistent hash.
///
/// This matches the algorithm in aspen-sharding::ShardRouter to ensure
/// consistent key-to-shard mapping without requiring a dependency on
/// the sharding crate.
///
/// # Arguments
/// * `key` - The key to hash
/// * `num_shards` - Total number of shards (must be > 0)
///
/// # Returns
/// The shard ID (0..num_shards) that owns this key
fn compute_shard_for_key(key: &str, num_shards: u32) -> u32 {
    if num_shards <= 1 {
        return 0;
    }

    // Step 1: Hash the key to a u64 using a simple hash
    let mut key_hash = 0u64;
    for byte in key.bytes() {
        key_hash = key_hash.wrapping_mul(31).wrapping_add(u64::from(byte));
    }

    // Step 2: Jump consistent hash algorithm
    // Reference: https://arxiv.org/abs/1406.2294
    let mut b: i64 = -1;
    let mut j: i64 = 0;

    while j < i64::from(num_shards) {
        b = j;
        key_hash = key_hash.wrapping_mul(2862933555777941757).wrapping_add(1);
        let divisor = ((key_hash >> 33).wrapping_add(1)) as f64;
        j = (((b.wrapping_add(1)) as f64) * ((1i64 << 31) as f64 / divisor)) as i64;
    }

    b as u32
}

/// Affinity-aware job manager extensions.
pub struct AffinityJobManager<S: aspen_core::KeyValueStore + ?Sized> {
    manager: Arc<JobManager<S>>,
    worker_metadata: Arc<tokio::sync::RwLock<HashMap<String, WorkerMetadata>>>,
    /// Number of shards for shard-aware routing.
    /// 0 or 1 means non-sharded (pure load-based selection for FollowData).
    num_shards: u32,
}

impl<S: aspen_core::KeyValueStore + ?Sized + 'static> AffinityJobManager<S> {
    /// Create a new affinity-aware job manager (non-sharded mode).
    ///
    /// In non-sharded mode, FollowData affinity uses load-based worker selection
    /// since all Raft nodes have the complete dataset.
    pub fn new(manager: Arc<JobManager<S>>) -> Self {
        Self {
            manager,
            worker_metadata: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            num_shards: 0,
        }
    }

    /// Create a new affinity-aware job manager with shard configuration.
    ///
    /// When `num_shards > 1`, FollowData affinity will prefer workers on nodes
    /// that host the shard owning the requested keys.
    ///
    /// # Arguments
    /// * `manager` - The underlying job manager
    /// * `num_shards` - Total number of shards in the cluster
    pub fn with_shard_config(manager: Arc<JobManager<S>>, num_shards: u32) -> Self {
        Self {
            manager,
            worker_metadata: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            num_shards,
        }
    }

    /// Submit a job with affinity rules.
    pub async fn submit_with_affinity(&self, mut spec: JobSpec, affinity: JobAffinity) -> Result<JobId> {
        // Store affinity in job metadata
        let affinity_json =
            serde_json::to_string(&affinity).map_err(|e| crate::error::JobError::SerializationError { source: e })?;

        if spec.payload.is_object() {
            spec.payload["__affinity"] = serde_json::Value::String(affinity_json);
        }

        // Find best worker based on affinity
        if let Some(preferred_worker) = self.find_best_worker(&affinity).await {
            info!(
                worker_id = %preferred_worker,
                ?affinity.strategy,
                "selected worker based on affinity"
            );
            spec.payload["__preferred_worker"] = serde_json::Value::String(preferred_worker);
        } else if affinity.strict {
            return Err(crate::error::JobError::NoWorkersAvailable {
                job_type: spec.job_type.clone(),
            });
        }

        self.manager.submit(spec).await
    }

    /// Update worker metadata for affinity calculations.
    pub async fn update_worker_metadata(&self, metadata: WorkerMetadata) {
        let mut workers = self.worker_metadata.write().await;
        workers.insert(metadata.id.clone(), metadata);
    }

    /// Find worker with lowest latency to target node.
    fn find_best_worker_closest_to(workers: &HashMap<String, WorkerMetadata>, target_node: &NodeId) -> Option<String> {
        workers
            .values()
            .filter_map(|w| w.latencies.get(target_node).map(|latency| (w.id.clone(), *latency)))
            .min_by_key(|(_, latency)| *latency)
            .map(|(id, _)| id)
    }

    /// Find least loaded worker from the collection.
    fn find_best_worker_least_loaded(workers: &HashMap<String, WorkerMetadata>) -> Option<String> {
        workers
            .values()
            .min_by(|a, b| a.load.partial_cmp(&b.load).unwrap_or(std::cmp::Ordering::Equal))
            .map(|w| w.id.clone())
    }

    /// Find best worker for FollowData affinity with shard awareness.
    fn find_best_worker_follow_data(
        &self,
        workers: &HashMap<String, WorkerMetadata>,
        keys: &[String],
    ) -> Option<String> {
        if keys.is_empty() {
            return Self::find_best_worker_least_loaded(workers);
        }

        // Tiger Style: Bound the number of keys we process
        let keys_to_check = &keys[..keys.len().min(MAX_AFFINITY_KEYS)];

        // Shard-aware routing when num_shards > 1
        if self.num_shards > 1 {
            // Build a map of shard_id -> count of keys in that shard
            let mut shard_counts: HashMap<u32, usize> = HashMap::new();
            for key in keys_to_check {
                let shard_id = compute_shard_for_key(key, self.num_shards);
                *shard_counts.entry(shard_id).or_insert(0) += 1;
            }

            // Find the dominant shard (most keys)
            if let Some((target_shard, _)) = shard_counts.iter().max_by_key(|(_, count)| *count) {
                // Prefer workers on nodes hosting the target shard
                let local_workers: Vec<_> =
                    workers.values().filter(|w| w.local_shards.contains(target_shard)).collect();

                if !local_workers.is_empty() {
                    // Among local workers, pick least loaded
                    return local_workers
                        .into_iter()
                        .min_by(|a, b| a.load.partial_cmp(&b.load).unwrap_or(std::cmp::Ordering::Equal))
                        .map(|w| w.id.clone());
                }
            }
        }

        // Fallback: use least loaded worker
        Self::find_best_worker_least_loaded(workers)
    }

    /// Find the best worker based on affinity strategy.
    async fn find_best_worker(&self, affinity: &JobAffinity) -> Option<String> {
        let workers = self.worker_metadata.read().await;

        if workers.is_empty() {
            return None;
        }

        match &affinity.strategy {
            AffinityStrategy::None => None,

            AffinityStrategy::ClosestTo(target_node) => Self::find_best_worker_closest_to(&workers, target_node),

            AffinityStrategy::PreferWorker(worker_id) => {
                if workers.contains_key(worker_id) {
                    Some(worker_id.clone())
                } else {
                    None
                }
            }

            AffinityStrategy::PreferNode(node_id) => {
                workers.values().find(|w| w.node_id == *node_id).map(|w| w.id.clone())
            }

            AffinityStrategy::AvoidNode(node_id) => {
                workers.values().find(|w| w.node_id != *node_id).map(|w| w.id.clone())
            }

            AffinityStrategy::AvoidLeader => {
                // This requires runtime context (leader node ID) which isn't
                // available at this level. The distributed worker coordinator
                // or scheduler should handle this by filtering workers before
                // calling find_best_worker. Return None to indicate no match
                // (forcing fallback to load-based selection).
                None
            }

            AffinityStrategy::RequireTags(required_tags) => workers
                .values()
                .find(|w| required_tags.iter().all(|tag| w.tags.contains(tag)))
                .map(|w| w.id.clone()),

            AffinityStrategy::DataLocality { blob_hash } => {
                workers.values().find(|w| w.local_blobs.contains(blob_hash)).map(|w| w.id.clone())
            }

            AffinityStrategy::FollowData { keys } => self.find_best_worker_follow_data(&workers, keys),

            AffinityStrategy::Geographic { region } => {
                workers.values().find(|w| w.region.as_ref() == Some(region)).map(|w| w.id.clone())
            }

            AffinityStrategy::LeastLoaded => Self::find_best_worker_least_loaded(&workers),

            AffinityStrategy::Composite { strategies } => {
                // Apply strategies in order until one returns a worker
                for strategy in strategies {
                    let sub_affinity = JobAffinity::new(strategy.clone());
                    if let Some(worker_id) = Box::pin(self.find_best_worker(&sub_affinity)).await {
                        return Some(worker_id);
                    }
                }
                None
            }
        }
    }

    /// Calculate score for FollowData affinity strategy.
    fn calculate_affinity_score_follow_data(&self, worker: &WorkerMetadata, keys: &[String]) -> f32 {
        if keys.is_empty() {
            // No keys = neutral score based on load
            return 1.0 - worker.load;
        }

        // Tiger Style: Bound the number of keys we process
        let keys_to_check = &keys[..keys.len().min(MAX_AFFINITY_KEYS)];

        // Calculate shard locality score when sharding is enabled
        if self.num_shards > 1 {
            let mut local_key_count = 0;
            let total_keys = keys_to_check.len();

            for key in keys_to_check {
                let shard_id = compute_shard_for_key(key, self.num_shards);
                if worker.local_shards.contains(&shard_id) {
                    local_key_count += 1;
                }
            }

            if total_keys > 0 {
                // Score is proportion of keys that are local to this worker
                // Combined with inverse load for tie-breaking
                let locality_score = local_key_count as f32 / total_keys as f32;
                let load_score = 1.0 - worker.load;

                // Weight locality higher than load (70/30 split)
                return locality_score * 0.7 + load_score * 0.3;
            }
        }

        // Non-sharded mode: all nodes have all data, score based on load only
        1.0 - worker.load
    }

    /// Calculate score for a simple binary match (returns 1.0 if match, 0.0 otherwise).
    fn calculate_affinity_score_binary(matches: bool) -> f32 {
        if matches { 1.0 } else { 0.0 }
    }

    /// Calculate affinity score between a job and worker.
    pub fn calculate_affinity_score(&self, _job: &Job, worker: &WorkerMetadata, affinity: &JobAffinity) -> f32 {
        let base_score = match &affinity.strategy {
            AffinityStrategy::None => 0.5,

            AffinityStrategy::ClosestTo(target) => {
                if let Some(latency) = worker.latencies.get(target) {
                    // Convert latency to score (lower is better)
                    1.0 - (*latency as f32 / 1000.0).min(1.0)
                } else {
                    0.0
                }
            }

            AffinityStrategy::PreferWorker(id) => Self::calculate_affinity_score_binary(worker.id == *id),

            AffinityStrategy::PreferNode(node_id) => Self::calculate_affinity_score_binary(worker.node_id == *node_id),

            AffinityStrategy::AvoidNode(node_id) => Self::calculate_affinity_score_binary(worker.node_id != *node_id),

            AffinityStrategy::AvoidLeader => {
                // This strategy requires runtime context (leader node ID) which
                // isn't available in the pure affinity calculation. The distributed
                // worker coordinator handles this by filtering workers before scoring.
                // Here we give a neutral score; actual filtering happens at scheduling.
                0.5
            }

            AffinityStrategy::RequireTags(tags) => {
                let matching = tags.iter().filter(|tag| worker.tags.contains(tag)).count() as f32;
                matching / tags.len() as f32
            }

            AffinityStrategy::DataLocality { blob_hash } => {
                Self::calculate_affinity_score_binary(worker.local_blobs.contains(blob_hash))
            }

            AffinityStrategy::FollowData { keys } => self.calculate_affinity_score_follow_data(worker, keys),

            AffinityStrategy::Geographic { region } => {
                Self::calculate_affinity_score_binary(worker.region.as_ref() == Some(region))
            }

            AffinityStrategy::LeastLoaded => 1.0 - worker.load,

            AffinityStrategy::Composite { strategies } => {
                // Calculate average score across all strategies
                let scores: Vec<f32> = strategies
                    .iter()
                    .map(|strategy| {
                        let sub_affinity = JobAffinity::new(strategy.clone());
                        self.calculate_affinity_score(_job, worker, &sub_affinity)
                    })
                    .collect();

                if scores.is_empty() {
                    0.5
                } else {
                    scores.iter().sum::<f32>() / scores.len() as f32
                }
            }
        };

        // Apply weight to affinity score
        base_score * affinity.weight + (1.0 - affinity.weight) * 0.5
    }
}

/// Extension trait for JobSpec to add affinity.
impl JobSpec {
    /// Add affinity to this job specification.
    pub fn with_affinity(mut self, affinity: JobAffinity) -> Result<Self> {
        let affinity_json =
            serde_json::to_value(&affinity).map_err(|e| crate::error::JobError::SerializationError { source: e })?;

        if let Some(obj) = self.payload.as_object_mut() {
            obj.insert("__affinity".to_string(), affinity_json);
        }

        Ok(self)
    }

    /// Set job to prefer running near a specific node.
    pub fn prefer_near_node(self, node_id: NodeId) -> Result<Self> {
        self.with_affinity(JobAffinity::new(AffinityStrategy::ClosestTo(node_id)))
    }

    /// Set job to run where specific data is located.
    pub fn with_data_locality(self, blob_hash: String) -> Result<Self> {
        self.with_affinity(JobAffinity::new(AffinityStrategy::DataLocality { blob_hash }))
    }

    /// Set job to run on least loaded worker.
    pub fn prefer_least_loaded(self) -> Result<Self> {
        self.with_affinity(JobAffinity::new(AffinityStrategy::LeastLoaded))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_affinity_scoring() {
        let worker = WorkerMetadata {
            id: "worker-1".to_string(),
            node_id: iroh::SecretKey::generate(&mut rand::rng()).public(),
            tags: vec!["gpu".to_string(), "ml".to_string()],
            region: Some("us-west".to_string()),
            load: 0.3,
            local_blobs: vec!["hash123".to_string()],
            latencies: HashMap::new(),
            local_shards: vec![],
        };

        let job = Job::from_spec(JobSpec::new("test"));
        let manager = AffinityJobManager::<aspen_testing::DeterministicKeyValueStore> {
            manager: Arc::new(JobManager::new(aspen_testing::DeterministicKeyValueStore::new())),
            worker_metadata: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            num_shards: 0,
        };

        // Test tag matching
        let affinity = JobAffinity::new(AffinityStrategy::RequireTags(vec!["gpu".to_string(), "ml".to_string()]));
        let score = manager.calculate_affinity_score(&job, &worker, &affinity);
        assert_eq!(score, 0.7 * 1.0 + 0.3 * 0.5); // Full match with 0.7 weight

        // Test data locality
        let affinity = JobAffinity::new(AffinityStrategy::DataLocality {
            blob_hash: "hash123".to_string(),
        });
        let score = manager.calculate_affinity_score(&job, &worker, &affinity);
        assert_eq!(score, 0.7 * 1.0 + 0.3 * 0.5); // Has blob

        // Test least loaded
        let affinity = JobAffinity::new(AffinityStrategy::LeastLoaded);
        let score = manager.calculate_affinity_score(&job, &worker, &affinity);
        assert_eq!(score, 0.7 * 0.7 + 0.3 * 0.5); // 1.0 - 0.3 load
    }
}
