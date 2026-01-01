//! P2P locality-based job affinity for optimized job routing.
//!
//! This module provides job affinity features that leverage Aspen's P2P networking
//! to route jobs to workers based on network proximity, data locality, or other
//! affinity rules.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use iroh::PublicKey as NodeId;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::Result;
use crate::job::{Job, JobId, JobSpec};
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
    /// Prefer workers with specific tags.
    RequireTags(Vec<String>),
    /// Data locality - job should run where data is stored.
    DataLocality {
        /// Hash of the data in iroh-blobs.
        blob_hash: String,
    },
    /// Geographic affinity based on region.
    Geographic {
        /// Preferred region (e.g., "us-west", "eu-central").
        region: String,
    },
    /// Load-based - prefer least loaded worker.
    LeastLoaded,
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
}

/// Affinity-aware job manager extensions.
pub struct AffinityJobManager<S: aspen_core::KeyValueStore + ?Sized> {
    manager: Arc<JobManager<S>>,
    worker_metadata: Arc<tokio::sync::RwLock<HashMap<String, WorkerMetadata>>>,
}

impl<S: aspen_core::KeyValueStore + ?Sized + 'static> AffinityJobManager<S> {
    /// Create a new affinity-aware job manager.
    pub fn new(manager: Arc<JobManager<S>>) -> Self {
        Self {
            manager,
            worker_metadata: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Submit a job with affinity rules.
    pub async fn submit_with_affinity(
        &self,
        mut spec: JobSpec,
        affinity: JobAffinity,
    ) -> Result<JobId> {
        // Store affinity in job metadata
        let affinity_json = serde_json::to_string(&affinity)
            .map_err(|e| crate::error::JobError::SerializationError { source: e })?;

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

    /// Find the best worker based on affinity strategy.
    async fn find_best_worker(&self, affinity: &JobAffinity) -> Option<String> {
        let workers = self.worker_metadata.read().await;

        if workers.is_empty() {
            return None;
        }

        match &affinity.strategy {
            AffinityStrategy::None => None,

            AffinityStrategy::ClosestTo(target_node) => {
                // Find worker with lowest latency to target
                workers
                    .values()
                    .filter_map(|w| {
                        w.latencies
                            .get(target_node)
                            .map(|latency| (w.id.clone(), *latency))
                    })
                    .min_by_key(|(_, latency)| *latency)
                    .map(|(id, _)| id)
            }

            AffinityStrategy::PreferWorker(worker_id) => {
                if workers.contains_key(worker_id) {
                    Some(worker_id.clone())
                } else {
                    None
                }
            }

            AffinityStrategy::RequireTags(required_tags) => {
                workers
                    .values()
                    .find(|w| {
                        required_tags.iter().all(|tag| w.tags.contains(tag))
                    })
                    .map(|w| w.id.clone())
            }

            AffinityStrategy::DataLocality { blob_hash } => {
                // Find worker that has the blob locally
                workers
                    .values()
                    .find(|w| w.local_blobs.contains(blob_hash))
                    .map(|w| w.id.clone())
            }

            AffinityStrategy::Geographic { region } => {
                workers
                    .values()
                    .find(|w| w.region.as_ref() == Some(region))
                    .map(|w| w.id.clone())
            }

            AffinityStrategy::LeastLoaded => {
                workers
                    .values()
                    .min_by(|a, b| {
                        a.load.partial_cmp(&b.load).unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|w| w.id.clone())
            }
        }
    }

    /// Calculate affinity score between a job and worker.
    pub fn calculate_affinity_score(
        &self,
        _job: &Job,
        worker: &WorkerMetadata,
        affinity: &JobAffinity,
    ) -> f32 {
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

            AffinityStrategy::PreferWorker(id) => {
                if worker.id == *id { 1.0 } else { 0.0 }
            }

            AffinityStrategy::RequireTags(tags) => {
                let matching = tags.iter()
                    .filter(|tag| worker.tags.contains(tag))
                    .count() as f32;
                matching / tags.len() as f32
            }

            AffinityStrategy::DataLocality { blob_hash } => {
                if worker.local_blobs.contains(blob_hash) { 1.0 } else { 0.0 }
            }

            AffinityStrategy::Geographic { region } => {
                if worker.region.as_ref() == Some(region) { 1.0 } else { 0.0 }
            }

            AffinityStrategy::LeastLoaded => {
                1.0 - worker.load
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
        let affinity_json = serde_json::to_value(&affinity)
            .map_err(|e| crate::error::JobError::SerializationError { source: e })?;

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
            node_id: iroh::SecretKey::generate(&mut rand::thread_rng()).public(),
            tags: vec!["gpu".to_string(), "ml".to_string()],
            region: Some("us-west".to_string()),
            load: 0.3,
            local_blobs: vec!["hash123".to_string()],
            latencies: HashMap::new(),
        };

        let job = Job::from_spec(JobSpec::new("test"));
        let manager = AffinityJobManager::<aspen_core::inmemory::DeterministicKeyValueStore> {
            manager: Arc::new(JobManager::new(Arc::new(
                aspen_core::inmemory::DeterministicKeyValueStore::new()
            ))),
            worker_metadata: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        };

        // Test tag matching
        let affinity = JobAffinity::new(AffinityStrategy::RequireTags(
            vec!["gpu".to_string(), "ml".to_string()]
        ));
        let score = manager.calculate_affinity_score(&job, &worker, &affinity);
        assert_eq!(score, 0.7 * 1.0 + 0.3 * 0.5); // Full match with 0.7 weight

        // Test data locality
        let affinity = JobAffinity::new(AffinityStrategy::DataLocality {
            blob_hash: "hash123".to_string()
        });
        let score = manager.calculate_affinity_score(&job, &worker, &affinity);
        assert_eq!(score, 0.7 * 1.0 + 0.3 * 0.5); // Has blob

        // Test least loaded
        let affinity = JobAffinity::new(AffinityStrategy::LeastLoaded);
        let score = manager.calculate_affinity_score(&job, &worker, &affinity);
        assert_eq!(score, 0.7 * 0.7 + 0.3 * 0.5); // 1.0 - 0.3 load
    }
}