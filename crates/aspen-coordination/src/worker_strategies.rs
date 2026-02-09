//! Load balancing strategies for distributed worker coordination.
//!
//! This module provides pluggable strategies for distributing work across
//! workers in the cluster, including round-robin, least-loaded, affinity-based,
//! and consistent hashing approaches.

use std::collections::HashMap;
use std::collections::HashSet;

use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;
use tracing::debug;

use crate::pure::strategies::SelectionResult;
use crate::pure::strategies::calculate_load_score;
use crate::pure::strategies::compute_running_average;
use crate::pure::strategies::compute_virtual_node_hash;
use crate::pure::strategies::hash_key;
use crate::pure::strategies::is_worker_idle_for_stealing;
use crate::pure::strategies::lookup_hash_ring;
use crate::pure::strategies::select_from_scored;
use crate::pure::strategies::worker_matches_tags;
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

impl Default for RoundRobinStrategy {
    fn default() -> Self {
        Self::new()
    }
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

        // Round-robin selection using pure function
        let selected_local_idx = self.counter % eligible_indices.len();
        let selected_idx = eligible_indices[selected_local_idx];
        self.counter = (self.counter + 1) % eligible_indices.len();

        // Update metrics
        self.metrics.total_selections += 1;
        let worker_id = &workers[selected_idx].worker_id;
        *self.metrics.worker_distribution.entry(worker_id.clone()).or_insert(0) += 1;

        let elapsed = start.elapsed().as_micros() as u64;
        self.metrics.avg_selection_time_us = compute_running_average(
            self.metrics.avg_selection_time_us,
            self.metrics.total_selections.saturating_sub(1),
            elapsed,
        );

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

impl Default for LeastLoadedStrategy {
    fn default() -> Self {
        Self::new()
    }
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
        calculate_load_score(
            worker.load,
            worker.queue_depth as u32,
            worker.max_concurrent as u32,
            self.load_weight,
            self.queue_weight,
        )
    }
}

impl LoadBalancer for LeastLoadedStrategy {
    fn select(&mut self, workers: &[WorkerInfo], job_type: &str, context: &RoutingContext) -> Result<Option<usize>> {
        let start = std::time::Instant::now();

        // Filter and score workers using pure functions
        let mut eligible: Vec<(usize, f32)> = workers
            .iter()
            .enumerate()
            .filter(|(_, w)| w.can_handle(job_type) && worker_matches_tags(&w.tags, &context.required_tags))
            .map(|(i, w)| (i, self.calculate_score(w)))
            .collect();

        if eligible.is_empty() {
            self.metrics.no_worker_available += 1;
            return Ok(None);
        }

        // Sort by score (lower is better)
        eligible.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        // Convert for pure function
        let scored: Vec<(u32, f32)> = eligible.iter().map(|(i, s)| (*i as u32, *s)).collect();

        // Find preferred node indices
        let preferred_indices: Vec<u32> = if let Some(ref preferred) = context.preferred_node {
            eligible.iter().filter(|(i, _)| &workers[*i].node_id == preferred).map(|(i, _)| *i as u32).collect()
        } else {
            vec![]
        };

        let is_critical = matches!(context.priority, Some(Priority::Critical));

        // Use pure selection function
        let selected_idx = match select_from_scored(&scored, &preferred_indices, is_critical) {
            SelectionResult::Best(idx) | SelectionResult::Preferred(idx) => idx as usize,
            SelectionResult::None => {
                self.metrics.no_worker_available += 1;
                return Ok(None);
            }
        };

        // Update metrics
        self.metrics.total_selections += 1;
        let worker_id = &workers[selected_idx].worker_id;
        *self.metrics.worker_distribution.entry(worker_id.clone()).or_insert(0) += 1;

        let elapsed = start.elapsed().as_micros() as u64;
        self.metrics.avg_selection_time_us = compute_running_average(
            self.metrics.avg_selection_time_us,
            self.metrics.total_selections.saturating_sub(1),
            elapsed,
        );

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
            self.metrics.avg_selection_time_us = compute_running_average(
                self.metrics.avg_selection_time_us,
                self.metrics.total_selections.saturating_sub(1),
                elapsed,
            );
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

impl Default for ConsistentHashStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl ConsistentHashStrategy {
    pub fn new() -> Self {
        Self {
            ring: ConsistentHashRing::new(150), // 150 virtual nodes per worker
            metrics: StrategyMetrics::default(),
        }
    }

    pub fn with_replicas(replicas: u32) -> Self {
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
            self.metrics.avg_selection_time_us = compute_running_average(
                self.metrics.avg_selection_time_us,
                self.metrics.total_selections.saturating_sub(1),
                elapsed,
            );

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
    replicas: u32,
    ring: Vec<(u64, u32)>, // (hash, worker_index)
}

impl ConsistentHashRing {
    fn new(replicas: u32) -> Self {
        Self {
            replicas,
            ring: Vec::new(),
        }
    }

    fn update_nodes(&mut self, workers: &[(usize, &WorkerInfo)]) {
        self.ring.clear();

        for (idx, worker) in workers {
            for i in 0..self.replicas {
                let hash = compute_virtual_node_hash(&worker.worker_id, i);
                self.ring.push((hash, *idx as u32));
            }
        }

        self.ring.sort_by_key(|&(h, _)| h);
    }

    fn get_node(&self, key: &str) -> Option<usize> {
        let key_hash = hash_key(key);
        lookup_hash_ring(&self.ring, key_hash).map(|idx| idx as usize)
    }
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

        // First, try to find idle or low-load workers using pure function
        let idle_workers: Vec<_> = workers
            .iter()
            .enumerate()
            .filter(|(_, w)| {
                w.can_handle(job_type)
                    && is_worker_idle_for_stealing(w.load, self.steal_threshold, w.queue_depth as u32)
            })
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
            self.metrics.avg_selection_time_us = compute_running_average(
                self.metrics.avg_selection_time_us,
                self.metrics.total_selections.saturating_sub(1),
                elapsed,
            );
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
