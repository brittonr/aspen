//! Job placement strategies for routing jobs to appropriate execution backends

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::types::Job;

use super::{ExecutionBackend, ExecutionConfig};

/// Placement decision for a job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacementDecision {
    /// Selected backend for execution
    pub backend_name: String,
    /// Score indicating how well the backend matches (higher is better)
    pub score: f32,
    /// Reason for the placement decision
    pub reason: String,
    /// Alternative backends that could handle the job
    pub alternatives: Vec<String>,
}

/// Policy for job placement
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PlacementPolicy {
    /// Place on backend with most available resources
    BestFit,
    /// Place on backend with least available resources that can still handle the job
    WorstFit,
    /// Place on first available backend
    FirstFit,
    /// Round-robin across available backends
    RoundRobin,
    /// Use backend specified in job metadata
    Explicit,
    /// Custom policy with scoring function
    Custom(String),
}

impl Default for PlacementPolicy {
    fn default() -> Self {
        PlacementPolicy::BestFit
    }
}

/// Strategy for placing jobs on execution backends
#[async_trait]
pub trait PlacementStrategy: Send + Sync {
    /// Decide which backend should handle a job
    async fn place_job(
        &self,
        job: &Job,
        config: &ExecutionConfig,
        backends: &HashMap<String, Arc<dyn ExecutionBackend>>,
    ) -> Result<PlacementDecision>;

    /// Get the policy type
    fn policy(&self) -> PlacementPolicy;
}

/// Default placement strategy implementation
pub struct JobPlacement {
    policy: PlacementPolicy,
    round_robin_counter: std::sync::atomic::AtomicUsize,
}

impl JobPlacement {
    /// Create a new job placement strategy
    pub fn new(policy: PlacementPolicy) -> Self {
        Self {
            policy,
            round_robin_counter: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Score a backend for a given job
    async fn score_backend(
        &self,
        job: &Job,
        config: &ExecutionConfig,
        backend: &Arc<dyn ExecutionBackend>,
    ) -> Result<f32> {
        // Check if backend can handle the job
        if !backend.can_handle(job, config).await? {
            return Ok(0.0);
        }

        // Get resource info
        let resources = backend.get_resource_info().await?;

        // Calculate score based on policy
        match self.policy {
            PlacementPolicy::BestFit => {
                // Score based on available resources
                let cpu_score = resources.available_cpu_cores / resources.total_cpu_cores.max(1.0);
                let mem_score = resources.available_memory_mb as f32
                    / resources.total_memory_mb.max(1) as f32;
                let capacity_score = (resources.max_executions - resources.running_executions)
                    as f32
                    / resources.max_executions.max(1) as f32;

                // Weighted average
                Ok((cpu_score * 0.4 + mem_score * 0.4 + capacity_score * 0.2) * 100.0)
            }
            PlacementPolicy::WorstFit => {
                // Inverse of BestFit - prefer backends with less available resources
                let cpu_score = 1.0
                    - (resources.available_cpu_cores / resources.total_cpu_cores.max(1.0));
                let mem_score = 1.0
                    - (resources.available_memory_mb as f32
                        / resources.total_memory_mb.max(1) as f32);
                let capacity_score = resources.running_executions as f32
                    / resources.max_executions.max(1) as f32;

                // Weighted average
                Ok((cpu_score * 0.4 + mem_score * 0.4 + capacity_score * 0.2) * 100.0)
            }
            PlacementPolicy::FirstFit | PlacementPolicy::RoundRobin => {
                // Simple binary score - can handle or not
                Ok(100.0)
            }
            PlacementPolicy::Explicit => {
                // Check if this is the explicitly requested backend
                if let Some(requested) = config.placement_hints.get("backend") {
                    if requested == backend.backend_type() {
                        return Ok(1000.0); // Very high score for explicit match
                    }
                }
                Ok(0.0)
            }
            PlacementPolicy::Custom(_) => {
                // Default scoring for custom policies
                Ok(50.0)
            }
        }
    }
}

#[async_trait]
impl PlacementStrategy for JobPlacement {
    async fn place_job(
        &self,
        job: &Job,
        config: &ExecutionConfig,
        backends: &HashMap<String, Arc<dyn ExecutionBackend>>,
    ) -> Result<PlacementDecision> {
        if backends.is_empty() {
            return Err(anyhow::anyhow!("No execution backends available"));
        }

        // Score all backends
        let mut scores: Vec<(String, f32, Arc<dyn ExecutionBackend>)> = Vec::new();
        for (name, backend) in backends {
            let score = self.score_backend(job, config, backend).await?;
            if score > 0.0 {
                scores.push((name.clone(), score, backend.clone()));
            }
        }

        if scores.is_empty() {
            return Err(anyhow::anyhow!(
                "No backend can handle job with given requirements"
            ));
        }

        // Select backend based on policy
        let selected = match self.policy {
            PlacementPolicy::BestFit | PlacementPolicy::WorstFit | PlacementPolicy::Explicit => {
                // Sort by score and pick the highest
                scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
                &scores[0]
            }
            PlacementPolicy::FirstFit => {
                // Pick the first available
                &scores[0]
            }
            PlacementPolicy::RoundRobin => {
                // Round-robin selection
                let counter = self
                    .round_robin_counter
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                &scores[counter % scores.len()]
            }
            PlacementPolicy::Custom(_) => {
                // Default to best score for custom policies
                scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
                &scores[0]
            }
        };

        // Build list of alternatives
        let alternatives: Vec<String> = scores
            .iter()
            .filter(|(name, _, _)| name != &selected.0)
            .map(|(name, _, _)| name.clone())
            .take(3) // Limit to top 3 alternatives
            .collect();

        Ok(PlacementDecision {
            backend_name: selected.0.clone(),
            score: selected.1,
            reason: format!("Selected using {:?} policy", self.policy),
            alternatives,
        })
    }

    fn policy(&self) -> PlacementPolicy {
        self.policy.clone()
    }
}

/// Builder for placement strategies with advanced configuration
pub struct PlacementBuilder {
    policy: PlacementPolicy,
    constraints: HashMap<String, String>,
    preferences: HashMap<String, f32>,
}

impl PlacementBuilder {
    /// Create a new placement builder
    pub fn new() -> Self {
        Self {
            policy: PlacementPolicy::default(),
            constraints: HashMap::new(),
            preferences: HashMap::new(),
        }
    }

    /// Set the placement policy
    pub fn with_policy(mut self, policy: PlacementPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Add a hard constraint (must be satisfied)
    pub fn with_constraint(mut self, key: String, value: String) -> Self {
        self.constraints.insert(key, value);
        self
    }

    /// Add a soft preference (affects scoring)
    pub fn with_preference(mut self, key: String, weight: f32) -> Self {
        self.preferences.insert(key, weight);
        self
    }

    /// Build the placement strategy
    pub fn build(self) -> Box<dyn PlacementStrategy> {
        Box::new(JobPlacement::new(self.policy))
    }
}

impl Default for PlacementBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_placement_policy_default() {
        let policy = PlacementPolicy::default();
        assert_eq!(policy, PlacementPolicy::BestFit);
    }

    #[test]
    fn test_placement_builder() {
        let strategy = PlacementBuilder::new()
            .with_policy(PlacementPolicy::RoundRobin)
            .with_constraint("backend_type".to_string(), "vm".to_string())
            .with_preference("low_latency".to_string(), 0.8)
            .build();

        assert_eq!(strategy.policy(), PlacementPolicy::RoundRobin);
    }

    #[test]
    fn test_placement_decision_serialization() {
        let decision = PlacementDecision {
            backend_name: "vm".to_string(),
            score: 85.5,
            reason: "Best fit".to_string(),
            alternatives: vec!["local".to_string(), "flawless".to_string()],
        };

        let serialized = serde_json::to_string(&decision).unwrap();
        let deserialized: PlacementDecision = serde_json::from_str(&serialized).unwrap();
        assert_eq!(decision.backend_name, deserialized.backend_name);
        assert_eq!(decision.score, deserialized.score);
    }
}