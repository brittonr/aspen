//! Load balancing strategies for distributed worker coordination.
//!
//! This module provides pluggable strategies for distributing work across
//! workers in the cluster, including round-robin, least-loaded, affinity-based,
//! and consistent hashing approaches.

use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::Hash;
use std::hash::Hasher;

use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;
use tracing::debug;

use crate::worker_coordinator::WorkerInfo;

/// Trait for implementing custom load balancing strategies.
pub trait LoadBalancer: Send + Sync {
    /// Select a worker for a job.
    ///
    /// # Arguments
    ///
    /// * `workers` - Available workers
    /// * `job_type` - Type of job to execute
    /// * `context` - Additional context for routing decision
    fn select(&mut self, workers: &[WorkerInfo], job_type: &str, context: &RoutingContext) -> Result<Option<usize>>;

    /// Reset strategy state (e.g., round-robin counter).
    fn reset(&mut self) {}

    /// Get strategy metrics.
    fn metrics(&self) -> StrategyMetrics {
        StrategyMetrics::default()
    }
}

/// Context for routing decisions.
#[derive(Debug, Clone, Default)]
pub struct RoutingContext {
    /// Affinity key for consistent routing.
    pub affinity_key: Option<String>,
    /// Required worker tags.
    pub required_tags: Vec<String>,
    /// Preferred node ID.
    pub preferred_node: Option<String>,
    /// Job priority.
    pub priority: Option<Priority>,
    /// Job size estimate.
    pub estimated_size: Option<usize>,
    /// Custom metadata.
    pub metadata: HashMap<String, String>,
}

/// Job priority levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Normal,
    High,
    Critical,
}

/// Metrics for strategy performance.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StrategyMetrics {
    /// Total selections made.
    pub total_selections: u64,
    /// Selections that found no worker.
    pub no_worker_available: u64,
    /// Average selection time in microseconds.
    pub avg_selection_time_us: u64,
    /// Distribution of selections per worker.
    pub worker_distribution: HashMap<String, u64>,
}

/// Round-robin load balancing strategy.
pub struct RoundRobinStrategy {
    counter: usize,
    metrics: StrategyMetrics,
}

impl RoundRobinStrategy {
    pub fn new() -> Self {
        Self {
            counter: 0,
            metrics: StrategyMetrics::default(),
        }
    }
}

impl LoadBalancer for RoundRobinStrategy {
    fn select(&mut self, workers: &[WorkerInfo], job_type: &str, _context: &RoutingContext) -> Result<Option<usize>> {
        let start = std::time::Instant::now();

        // Filter eligible workers
        let eligible_indices: Vec<_> =
            workers.iter().enumerate().filter(|(_, w)| w.can_handle(job_type)).map(|(i, _)| i).collect();

        if eligible_indices.is_empty() {
            self.metrics.no_worker_available += 1;
            return Ok(None);
        }

        // Round-robin selection
        let selected_idx = eligible_indices[self.counter % eligible_indices.len()];
        self.counter = (self.counter + 1) % eligible_indices.len();

        // Update metrics
        self.metrics.total_selections += 1;
        let worker_id = &workers[selected_idx].worker_id;
        *self.metrics.worker_distribution.entry(worker_id.clone()).or_insert(0) += 1;

        let elapsed = start.elapsed().as_micros() as u64;
        self.metrics.avg_selection_time_us = (self.metrics.avg_selection_time_us * (self.metrics.total_selections - 1)
            + elapsed)
            / self.metrics.total_selections;

        debug!(worker_id, "round-robin selected worker");
        Ok(Some(selected_idx))
    }

    fn reset(&mut self) {
        self.counter = 0;
    }

    fn metrics(&self) -> StrategyMetrics {
        self.metrics.clone()
    }
}

/// Least-loaded worker selection strategy.
pub struct LeastLoadedStrategy {
    metrics: StrategyMetrics,
    load_weight: f32,
    queue_weight: f32,
}

impl LeastLoadedStrategy {
    pub fn new() -> Self {
        Self {
            metrics: StrategyMetrics::default(),
            load_weight: 0.7,  // CPU load is 70% of score
            queue_weight: 0.3, // Queue depth is 30% of score
        }
    }

    pub fn with_weights(load_weight: f32, queue_weight: f32) -> Self {
        Self {
            metrics: StrategyMetrics::default(),
            load_weight,
            queue_weight,
        }
    }

    fn calculate_score(&self, worker: &WorkerInfo) -> f32 {
        // Lower score is better
        let load_score = worker.load * self.load_weight;
        let queue_score = (worker.queue_depth as f32 / worker.max_concurrent.max(1) as f32) * self.queue_weight;
        load_score + queue_score
    }
}

impl LoadBalancer for LeastLoadedStrategy {
    fn select(&mut self, workers: &[WorkerInfo], job_type: &str, context: &RoutingContext) -> Result<Option<usize>> {
        let start = std::time::Instant::now();

        // Filter and score workers
        let mut eligible: Vec<(usize, f32)> = workers
            .iter()
            .enumerate()
            .filter(|(_, w)| w.can_handle(job_type) && context.required_tags.iter().all(|t| w.tags.contains(t)))
            .map(|(i, w)| (i, self.calculate_score(w)))
            .collect();

        if eligible.is_empty() {
            self.metrics.no_worker_available += 1;
            return Ok(None);
        }

        // Sort by score (lower is better)
        eligible.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        // Apply priority boost if needed
        let selected_idx = if let Some(Priority::Critical) = context.priority {
            // For critical jobs, always pick the best worker
            eligible[0].0
        } else if context.preferred_node.is_some() {
            // Try to find worker on preferred node
            eligible
                .iter()
                .find(|(i, _)| &workers[*i].node_id == context.preferred_node.as_ref().unwrap())
                .map(|(i, _)| *i)
                .unwrap_or(eligible[0].0)
        } else {
            eligible[0].0
        };

        // Update metrics
        self.metrics.total_selections += 1;
        let worker_id = &workers[selected_idx].worker_id;
        *self.metrics.worker_distribution.entry(worker_id.clone()).or_insert(0) += 1;

        let elapsed = start.elapsed().as_micros() as u64;
        self.metrics.avg_selection_time_us = (self.metrics.avg_selection_time_us * (self.metrics.total_selections - 1)
            + elapsed)
            / self.metrics.total_selections;

        debug!(worker_id, score = eligible[0].1, "least-loaded selected worker");
        Ok(Some(selected_idx))
    }

    fn metrics(&self) -> StrategyMetrics {
        self.metrics.clone()
    }
}

/// Affinity-based routing strategy for sticky sessions.
pub struct AffinityStrategy {
    affinity_map: HashMap<String, String>, // affinity_key -> worker_id
    fallback: Box<dyn LoadBalancer>,
    metrics: StrategyMetrics,
    max_affinity_entries: usize,
}

impl AffinityStrategy {
    pub fn new(fallback: Box<dyn LoadBalancer>) -> Self {
        Self {
            affinity_map: HashMap::new(),
            fallback,
            metrics: StrategyMetrics::default(),
            max_affinity_entries: 10_000,
        }
    }

    fn cleanup_stale_affinities(&mut self, workers: &[WorkerInfo]) {
        let active_workers: HashSet<_> = workers.iter().map(|w| &w.worker_id).collect();

        self.affinity_map.retain(|_, worker_id| active_workers.contains(worker_id));
    }
}

impl LoadBalancer for AffinityStrategy {
    fn select(&mut self, workers: &[WorkerInfo], job_type: &str, context: &RoutingContext) -> Result<Option<usize>> {
        let start = std::time::Instant::now();

        // Clean up periodically
        if self.affinity_map.len() > self.max_affinity_entries {
            self.cleanup_stale_affinities(workers);
        }

        let selected_idx = if let Some(ref key) = context.affinity_key {
            // Check affinity map
            if let Some(worker_id) = self.affinity_map.get(key) {
                // Find worker index
                if let Some((idx, _)) =
                    workers.iter().enumerate().find(|(_, w)| &w.worker_id == worker_id && w.can_handle(job_type))
                {
                    debug!(worker_id, affinity_key = key, "affinity cache hit");
                    Some(idx)
                } else {
                    // Worker no longer available, remove from map
                    self.affinity_map.remove(key);

                    // Use fallback
                    let result = self.fallback.select(workers, job_type, context)?;

                    // Update affinity map with new selection
                    if let Some(idx) = result {
                        self.affinity_map.insert(key.clone(), workers[idx].worker_id.clone());
                    }

                    result
                }
            } else {
                // No affinity exists, select with fallback
                let result = self.fallback.select(workers, job_type, context)?;

                // Store affinity
                if let Some(idx) = result {
                    self.affinity_map.insert(key.clone(), workers[idx].worker_id.clone());
                }

                result
            }
        } else {
            // No affinity key, use fallback
            self.fallback.select(workers, job_type, context)?
        };

        // Update metrics
        if let Some(idx) = selected_idx {
            self.metrics.total_selections += 1;
            let worker_id = &workers[idx].worker_id;
            *self.metrics.worker_distribution.entry(worker_id.clone()).or_insert(0) += 1;

            let elapsed = start.elapsed().as_micros() as u64;
            self.metrics.avg_selection_time_us =
                (self.metrics.avg_selection_time_us * (self.metrics.total_selections - 1) + elapsed)
                    / self.metrics.total_selections;
        } else {
            self.metrics.no_worker_available += 1;
        }

        Ok(selected_idx)
    }

    fn reset(&mut self) {
        self.affinity_map.clear();
        self.fallback.reset();
    }

    fn metrics(&self) -> StrategyMetrics {
        self.metrics.clone()
    }
}

/// Consistent hashing strategy for deterministic routing.
pub struct ConsistentHashStrategy {
    ring: ConsistentHashRing,
    metrics: StrategyMetrics,
}

impl ConsistentHashStrategy {
    pub fn new() -> Self {
        Self {
            ring: ConsistentHashRing::new(150), // 150 virtual nodes per worker
            metrics: StrategyMetrics::default(),
        }
    }

    pub fn with_replicas(replicas: usize) -> Self {
        Self {
            ring: ConsistentHashRing::new(replicas),
            metrics: StrategyMetrics::default(),
        }
    }
}

impl LoadBalancer for ConsistentHashStrategy {
    fn select(&mut self, workers: &[WorkerInfo], job_type: &str, context: &RoutingContext) -> Result<Option<usize>> {
        let start = std::time::Instant::now();

        // Filter eligible workers
        let eligible: Vec<_> = workers.iter().enumerate().filter(|(_, w)| w.can_handle(job_type)).collect();

        if eligible.is_empty() {
            self.metrics.no_worker_available += 1;
            return Ok(None);
        }

        // Update ring if workers changed
        self.ring.update_nodes(&eligible);

        // Hash the key
        let key = context.affinity_key.as_deref().unwrap_or(job_type);
        let selected_idx = self.ring.get_node(key);

        // Update metrics
        if let Some(idx) = selected_idx {
            self.metrics.total_selections += 1;
            let worker_id = &workers[idx].worker_id;
            *self.metrics.worker_distribution.entry(worker_id.clone()).or_insert(0) += 1;

            let elapsed = start.elapsed().as_micros() as u64;
            self.metrics.avg_selection_time_us =
                (self.metrics.avg_selection_time_us * (self.metrics.total_selections - 1) + elapsed)
                    / self.metrics.total_selections;

            debug!(worker_id, key, "consistent hash selected worker");
        }

        Ok(selected_idx)
    }

    fn metrics(&self) -> StrategyMetrics {
        self.metrics.clone()
    }
}

/// Consistent hash ring implementation.
struct ConsistentHashRing {
    replicas: usize,
    ring: Vec<(u64, usize)>, // (hash, worker_index)
}

impl ConsistentHashRing {
    fn new(replicas: usize) -> Self {
        Self {
            replicas,
            ring: Vec::new(),
        }
    }

    fn update_nodes(&mut self, workers: &[(usize, &WorkerInfo)]) {
        self.ring.clear();

        for (idx, worker) in workers {
            for i in 0..self.replicas {
                let virtual_key = format!("{}:{}", worker.worker_id, i);
                let hash = hash_key(&virtual_key);
                self.ring.push((hash, *idx));
            }
        }

        self.ring.sort_by_key(|&(hash, _)| hash);
    }

    fn get_node(&self, key: &str) -> Option<usize> {
        if self.ring.is_empty() {
            return None;
        }

        let hash = hash_key(key);

        // Binary search for the first node with hash >= key_hash
        match self.ring.binary_search_by_key(&hash, |&(h, _)| h) {
            Ok(idx) => Some(self.ring[idx].1),
            Err(idx) => {
                // Wrap around to the first node if we're past the end
                let idx = if idx >= self.ring.len() { 0 } else { idx };
                Some(self.ring[idx].1)
            }
        }
    }
}

/// Hash a string key to u64.
fn hash_key(key: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;

    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

/// Work-stealing aware strategy that considers queue depths.
pub struct WorkStealingStrategy {
    base_strategy: Box<dyn LoadBalancer>,
    steal_threshold: f32,
    metrics: StrategyMetrics,
}

impl WorkStealingStrategy {
    pub fn new(base_strategy: Box<dyn LoadBalancer>) -> Self {
        Self {
            base_strategy,
            steal_threshold: 0.3, // Prefer workers with < 30% load
            metrics: StrategyMetrics::default(),
        }
    }

    pub fn with_threshold(base_strategy: Box<dyn LoadBalancer>, threshold: f32) -> Self {
        Self {
            base_strategy,
            steal_threshold: threshold,
            metrics: StrategyMetrics::default(),
        }
    }
}

impl LoadBalancer for WorkStealingStrategy {
    fn select(&mut self, workers: &[WorkerInfo], job_type: &str, context: &RoutingContext) -> Result<Option<usize>> {
        let start = std::time::Instant::now();

        // First, try to find idle or low-load workers
        let idle_workers: Vec<_> = workers
            .iter()
            .enumerate()
            .filter(|(_, w)| w.can_handle(job_type) && w.load < self.steal_threshold && w.queue_depth == 0)
            .collect();

        let selected_idx = if !idle_workers.is_empty() {
            // Prefer idle workers for work stealing
            let (idx, worker) = idle_workers[0];
            debug!(
                worker_id = %worker.worker_id,
                load = worker.load,
                "work-stealing selected idle worker"
            );
            Some(idx)
        } else {
            // Fall back to base strategy
            self.base_strategy.select(workers, job_type, context)?
        };

        // Update metrics
        if let Some(idx) = selected_idx {
            self.metrics.total_selections += 1;
            let worker_id = &workers[idx].worker_id;
            *self.metrics.worker_distribution.entry(worker_id.clone()).or_insert(0) += 1;

            let elapsed = start.elapsed().as_micros() as u64;
            self.metrics.avg_selection_time_us =
                (self.metrics.avg_selection_time_us * (self.metrics.total_selections - 1) + elapsed)
                    / self.metrics.total_selections;
        } else {
            self.metrics.no_worker_available += 1;
        }

        Ok(selected_idx)
    }

    fn reset(&mut self) {
        self.base_strategy.reset();
    }

    fn metrics(&self) -> StrategyMetrics {
        self.metrics.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::HealthStatus;
    use crate::types::now_unix_ms;

    fn create_test_workers() -> Vec<WorkerInfo> {
        vec![
            WorkerInfo {
                worker_id: "w1".to_string(),
                node_id: "n1".to_string(),
                peer_id: None,
                capabilities: vec!["email".to_string()],
                load: 0.2,
                active_jobs: 2,
                max_concurrent: 10,
                queue_depth: 3,
                health: HealthStatus::Healthy,
                tags: vec!["region:us-east".to_string()],
                last_heartbeat_ms: now_unix_ms(),
                started_at_ms: now_unix_ms(),
                total_processed: 100,
                total_failed: 1,
                avg_processing_time_ms: 50,
                groups: HashSet::new(),
            },
            WorkerInfo {
                worker_id: "w2".to_string(),
                node_id: "n2".to_string(),
                peer_id: None,
                capabilities: vec!["email".to_string(), "sms".to_string()],
                load: 0.8,
                active_jobs: 8,
                max_concurrent: 10,
                queue_depth: 15,
                health: HealthStatus::Healthy,
                tags: vec!["region:us-west".to_string()],
                last_heartbeat_ms: now_unix_ms(),
                started_at_ms: now_unix_ms(),
                total_processed: 200,
                total_failed: 5,
                avg_processing_time_ms: 75,
                groups: HashSet::new(),
            },
            WorkerInfo {
                worker_id: "w3".to_string(),
                node_id: "n1".to_string(),
                peer_id: None,
                capabilities: vec![], // Can handle any job type
                load: 0.5,
                active_jobs: 5,
                max_concurrent: 10,
                queue_depth: 8,
                health: HealthStatus::Healthy,
                tags: vec!["region:us-east".to_string()],
                last_heartbeat_ms: now_unix_ms(),
                started_at_ms: now_unix_ms(),
                total_processed: 150,
                total_failed: 3,
                avg_processing_time_ms: 60,
                groups: HashSet::new(),
            },
        ]
    }

    #[test]
    fn test_round_robin_strategy() {
        let mut strategy = RoundRobinStrategy::new();
        let workers = create_test_workers();
        let context = RoutingContext::default();

        // Should cycle through eligible workers
        let idx1 = strategy.select(&workers, "email", &context).unwrap().unwrap();
        let idx2 = strategy.select(&workers, "email", &context).unwrap().unwrap();
        let idx3 = strategy.select(&workers, "email", &context).unwrap().unwrap();

        // All workers can handle "email", so should cycle
        assert_ne!(idx1, idx2);
        assert_ne!(idx2, idx3);

        let metrics = strategy.metrics();
        assert_eq!(metrics.total_selections, 3);
    }

    #[test]
    fn test_least_loaded_strategy() {
        let mut strategy = LeastLoadedStrategy::new();
        let workers = create_test_workers();
        let context = RoutingContext::default();

        // Should select w1 (lowest load at 0.2)
        let idx = strategy.select(&workers, "email", &context).unwrap().unwrap();
        assert_eq!(workers[idx].worker_id, "w1");
    }

    #[test]
    fn test_affinity_strategy() {
        let fallback = Box::new(RoundRobinStrategy::new());
        let mut strategy = AffinityStrategy::new(fallback);
        let workers = create_test_workers();

        let mut context = RoutingContext::default();
        context.affinity_key = Some("user123".to_string());

        // First call should create affinity
        let idx1 = strategy.select(&workers, "email", &context).unwrap().unwrap();

        // Second call with same key should return same worker
        let idx2 = strategy.select(&workers, "email", &context).unwrap().unwrap();
        assert_eq!(idx1, idx2);
    }

    #[test]
    fn test_consistent_hash_strategy() {
        let mut strategy = ConsistentHashStrategy::new();
        let workers = create_test_workers();

        let mut context = RoutingContext::default();
        context.affinity_key = Some("job123".to_string());

        // Same key should always map to same worker
        let idx1 = strategy.select(&workers, "email", &context).unwrap().unwrap();
        let idx2 = strategy.select(&workers, "email", &context).unwrap().unwrap();
        assert_eq!(idx1, idx2);

        // Different key might map to different worker
        context.affinity_key = Some("job456".to_string());
        let idx3 = strategy.select(&workers, "email", &context).unwrap();
        // May or may not be different, but should be consistent
    }

    #[test]
    fn test_work_stealing_strategy() {
        let base = Box::new(LeastLoadedStrategy::new());
        let mut strategy = WorkStealingStrategy::new(base);
        let workers = create_test_workers();
        let context = RoutingContext::default();

        // Should prefer w1 (lowest load and under threshold)
        let idx = strategy.select(&workers, "email", &context).unwrap().unwrap();
        assert_eq!(workers[idx].worker_id, "w1");
    }
}
