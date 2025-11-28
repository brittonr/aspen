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
                // Handle NaN values by treating them as less than any valid number
                scores.sort_by(|a, b| {
                    if a.1.is_nan() && b.1.is_nan() {
                        std::cmp::Ordering::Equal
                    } else if a.1.is_nan() {
                        std::cmp::Ordering::Less
                    } else if b.1.is_nan() {
                        std::cmp::Ordering::Greater
                    } else {
                        // Safety: NaN values are handled above, so partial_cmp cannot return None
                        b.1.partial_cmp(&a.1).unwrap()
                    }
                });
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
                // Handle NaN values by treating them as less than any valid number
                scores.sort_by(|a, b| {
                    if a.1.is_nan() && b.1.is_nan() {
                        std::cmp::Ordering::Equal
                    } else if a.1.is_nan() {
                        std::cmp::Ordering::Less
                    } else if b.1.is_nan() {
                        std::cmp::Ordering::Greater
                    } else {
                        // Safety: NaN values are handled above, so partial_cmp cannot return None
                        b.1.partial_cmp(&a.1).unwrap()
                    }
                });
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
    use crate::adapters::{
        BackendHealth, ExecutionBackend, ExecutionConfig, ExecutionHandle, ExecutionMetadata,
        ExecutionStatus, ResourceInfo,
    };
    use crate::domain::job::Job;
    use std::collections::HashMap;
    use std::time::Duration;

    /// Mock execution backend for testing placement strategies
    struct MockBackend {
        name: String,
        can_handle_result: bool,
        resource_info: ResourceInfo,
    }

    impl MockBackend {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                can_handle_result: true,
                resource_info: ResourceInfo {
                    total_cpu_cores: 8.0,
                    available_cpu_cores: 4.0,
                    total_memory_mb: 16384,
                    available_memory_mb: 8192,
                    total_disk_mb: 102400,
                    available_disk_mb: 51200,
                    running_executions: 2,
                    max_executions: 10,
                },
            }
        }

        fn with_resources(mut self, resources: ResourceInfo) -> Self {
            self.resource_info = resources;
            self
        }

        fn with_can_handle(mut self, can_handle: bool) -> Self {
            self.can_handle_result = can_handle;
            self
        }
    }

    #[async_trait]
    impl ExecutionBackend for MockBackend {
        fn backend_type(&self) -> &str {
            &self.name
        }

        async fn submit_job(&self, _job: Job, _config: ExecutionConfig) -> Result<ExecutionHandle> {
            unimplemented!("Not needed for placement tests")
        }

        async fn get_status(&self, _handle: &ExecutionHandle) -> Result<ExecutionStatus> {
            unimplemented!("Not needed for placement tests")
        }

        async fn cancel_execution(&self, _handle: &ExecutionHandle) -> Result<()> {
            unimplemented!("Not needed for placement tests")
        }

        async fn get_metadata(&self, _handle: &ExecutionHandle) -> Result<ExecutionMetadata> {
            unimplemented!("Not needed for placement tests")
        }

        async fn wait_for_completion(
            &self,
            _handle: &ExecutionHandle,
            _timeout: Option<Duration>,
        ) -> Result<ExecutionStatus> {
            unimplemented!("Not needed for placement tests")
        }

        async fn health_check(&self) -> Result<BackendHealth> {
            Ok(BackendHealth {
                healthy: true,
                status_message: "OK".to_string(),
                resource_info: Some(self.resource_info.clone()),
                last_check: Some(0),
                details: HashMap::new(),
            })
        }

        async fn get_resource_info(&self) -> Result<ResourceInfo> {
            Ok(self.resource_info.clone())
        }

        async fn can_handle(&self, _job: &Job, _config: &ExecutionConfig) -> Result<bool> {
            Ok(self.can_handle_result)
        }

        async fn list_executions(&self) -> Result<Vec<ExecutionHandle>> {
            Ok(Vec::new())
        }

        async fn cleanup_executions(&self, _older_than: Duration) -> Result<usize> {
            Ok(0)
        }
    }

    /// Helper to create a test job
    fn create_test_job() -> Job {
        Job::default()
    }

    /// Helper to create a test execution config
    fn create_test_config() -> ExecutionConfig {
        ExecutionConfig::default()
    }

    /// Helper to create backends map
    fn create_backends(backends: Vec<Arc<dyn ExecutionBackend>>) -> HashMap<String, Arc<dyn ExecutionBackend>> {
        backends
            .into_iter()
            .map(|b| (b.backend_type().to_string(), b))
            .collect()
    }

    #[tokio::test]
    async fn test_placement_policy_default() {
        let policy = PlacementPolicy::default();
        assert_eq!(policy, PlacementPolicy::BestFit);
    }

    #[tokio::test]
    async fn test_job_placement_new() {
        let placement = JobPlacement::new(PlacementPolicy::BestFit);
        assert_eq!(placement.policy(), PlacementPolicy::BestFit);
    }

    #[tokio::test]
    async fn test_no_backends_available() {
        let placement = JobPlacement::new(PlacementPolicy::BestFit);
        let job = create_test_job();
        let config = create_test_config();
        let backends = HashMap::new();

        let result = placement.place_job(&job, &config, &backends).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No execution backends available"));
    }

    #[tokio::test]
    async fn test_no_compatible_backends() {
        let placement = JobPlacement::new(PlacementPolicy::BestFit);
        let job = create_test_job();
        let config = create_test_config();

        let backend = Arc::new(MockBackend::new("test").with_can_handle(false));
        let backends = create_backends(vec![backend]);

        let result = placement.place_job(&job, &config, &backends).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No backend can handle job"));
    }

    #[tokio::test]
    async fn test_bestfit_selects_most_available() {
        let placement = JobPlacement::new(PlacementPolicy::BestFit);
        let job = create_test_job();
        let config = create_test_config();

        // Backend with more available resources
        let backend1 = Arc::new(MockBackend::new("high_available").with_resources(ResourceInfo {
            total_cpu_cores: 16.0,
            available_cpu_cores: 12.0,
            total_memory_mb: 32768,
            available_memory_mb: 24576,
            total_disk_mb: 102400,
            available_disk_mb: 76800,
            running_executions: 2,
            max_executions: 20,
        }));

        // Backend with fewer available resources
        let backend2 = Arc::new(MockBackend::new("low_available").with_resources(ResourceInfo {
            total_cpu_cores: 8.0,
            available_cpu_cores: 2.0,
            total_memory_mb: 16384,
            available_memory_mb: 4096,
            total_disk_mb: 102400,
            available_disk_mb: 25600,
            running_executions: 6,
            max_executions: 10,
        }));

        let backends = create_backends(vec![backend1, backend2]);
        let decision = placement.place_job(&job, &config, &backends).await.unwrap();

        assert_eq!(decision.backend_name, "high_available");
        assert!(decision.score > 0.0);
    }

    #[tokio::test]
    async fn test_worstfit_selects_least_available() {
        let placement = JobPlacement::new(PlacementPolicy::WorstFit);
        let job = create_test_job();
        let config = create_test_config();

        // Backend with more available resources
        let backend1 = Arc::new(MockBackend::new("high_available").with_resources(ResourceInfo {
            total_cpu_cores: 16.0,
            available_cpu_cores: 12.0,
            total_memory_mb: 32768,
            available_memory_mb: 24576,
            total_disk_mb: 102400,
            available_disk_mb: 76800,
            running_executions: 2,
            max_executions: 20,
        }));

        // Backend with fewer available resources (more utilized)
        let backend2 = Arc::new(MockBackend::new("low_available").with_resources(ResourceInfo {
            total_cpu_cores: 8.0,
            available_cpu_cores: 2.0,
            total_memory_mb: 16384,
            available_memory_mb: 4096,
            total_disk_mb: 102400,
            available_disk_mb: 25600,
            running_executions: 6,
            max_executions: 10,
        }));

        let backends = create_backends(vec![backend1, backend2]);
        let decision = placement.place_job(&job, &config, &backends).await.unwrap();

        assert_eq!(decision.backend_name, "low_available");
        assert!(decision.score > 0.0);
    }

    #[tokio::test]
    async fn test_firstfit_selects_first_available() {
        let placement = JobPlacement::new(PlacementPolicy::FirstFit);
        let job = create_test_job();
        let config = create_test_config();

        let backend1 = Arc::new(MockBackend::new("backend1"));
        let backend2 = Arc::new(MockBackend::new("backend2"));

        let backends = create_backends(vec![backend1, backend2]);
        let decision = placement.place_job(&job, &config, &backends).await.unwrap();

        // Should select one of the backends (HashMap order is not guaranteed)
        assert!(decision.backend_name == "backend1" || decision.backend_name == "backend2");
        assert_eq!(decision.score, 100.0);
    }

    #[tokio::test]
    async fn test_roundrobin_cycles_through_backends() {
        let placement = JobPlacement::new(PlacementPolicy::RoundRobin);
        let job = create_test_job();
        let config = create_test_config();

        let backend1 = Arc::new(MockBackend::new("backend1"));
        let backend2 = Arc::new(MockBackend::new("backend2"));
        let backend3 = Arc::new(MockBackend::new("backend3"));

        let backends = create_backends(vec![backend1, backend2, backend3]);

        // Make multiple placements and verify round-robin behavior
        let mut selected_backends = Vec::new();
        for _ in 0..6 {
            let decision = placement.place_job(&job, &config, &backends).await.unwrap();
            selected_backends.push(decision.backend_name);
        }

        // Each backend should be selected twice in a round-robin fashion
        // Note: We can't guarantee exact order due to HashMap iteration order,
        // but we can verify that all backends are used
        let backend1_count = selected_backends.iter().filter(|&b| b == "backend1").count();
        let backend2_count = selected_backends.iter().filter(|&b| b == "backend2").count();
        let backend3_count = selected_backends.iter().filter(|&b| b == "backend3").count();

        assert_eq!(backend1_count, 2);
        assert_eq!(backend2_count, 2);
        assert_eq!(backend3_count, 2);
    }

    #[tokio::test]
    async fn test_explicit_selects_requested_backend() {
        let placement = JobPlacement::new(PlacementPolicy::Explicit);
        let job = create_test_job();
        let mut config = create_test_config();
        config.placement_hints.insert("backend".to_string(), "target_backend".to_string());

        let backend1 = Arc::new(MockBackend::new("other_backend"));
        let backend2 = Arc::new(MockBackend::new("target_backend"));

        let backends = create_backends(vec![backend1, backend2]);
        let decision = placement.place_job(&job, &config, &backends).await.unwrap();

        assert_eq!(decision.backend_name, "target_backend");
        assert_eq!(decision.score, 1000.0);
    }

    #[tokio::test]
    async fn test_explicit_no_matching_backend() {
        let placement = JobPlacement::new(PlacementPolicy::Explicit);
        let job = create_test_job();
        let mut config = create_test_config();
        config.placement_hints.insert("backend".to_string(), "nonexistent".to_string());

        let backend1 = Arc::new(MockBackend::new("backend1"));
        let backends = create_backends(vec![backend1]);

        let result = placement.place_job(&job, &config, &backends).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_custom_policy_uses_default_scoring() {
        let placement = JobPlacement::new(PlacementPolicy::Custom("custom_policy".to_string()));
        let job = create_test_job();
        let config = create_test_config();

        let backend = Arc::new(MockBackend::new("backend1"));
        let backends = create_backends(vec![backend]);
        let decision = placement.place_job(&job, &config, &backends).await.unwrap();

        assert_eq!(decision.backend_name, "backend1");
        assert_eq!(decision.score, 50.0);
    }

    #[tokio::test]
    async fn test_score_backend_bestfit_calculation() {
        let placement = JobPlacement::new(PlacementPolicy::BestFit);
        let job = create_test_job();
        let config = create_test_config();

        let backend = Arc::new(MockBackend::new("test").with_resources(ResourceInfo {
            total_cpu_cores: 10.0,
            available_cpu_cores: 5.0, // 50% available
            total_memory_mb: 10000,
            available_memory_mb: 5000, // 50% available
            total_disk_mb: 100000,
            available_disk_mb: 50000,
            running_executions: 5,
            max_executions: 10, // 50% available
        }));

        let score = placement.score_backend(&job, &config, &(backend as Arc<dyn ExecutionBackend>)).await.unwrap();
        assert_eq!(score, 50.0);
    }

    #[tokio::test]
    async fn test_score_backend_worstfit_calculation() {
        let placement = JobPlacement::new(PlacementPolicy::WorstFit);
        let job = create_test_job();
        let config = create_test_config();

        let backend = Arc::new(MockBackend::new("test").with_resources(ResourceInfo {
            total_cpu_cores: 10.0,
            available_cpu_cores: 5.0, // 50% available, 50% used
            total_memory_mb: 10000,
            available_memory_mb: 5000, // 50% available, 50% used
            total_disk_mb: 100000,
            available_disk_mb: 50000,
            running_executions: 5,
            max_executions: 10, // 50% running
        }));

        let score = placement.score_backend(&job, &config, &(backend as Arc<dyn ExecutionBackend>)).await.unwrap();

        // WorstFit: (0.5 * 0.4 + 0.5 * 0.4 + 0.5 * 0.2) * 100.0 = 50.0
        assert_eq!(score, 50.0);
    }

    #[tokio::test]
    async fn test_score_backend_zero_resources() {
        let placement = JobPlacement::new(PlacementPolicy::BestFit);
        let job = create_test_job();
        let config = create_test_config();

        let backend = Arc::new(MockBackend::new("test").with_resources(ResourceInfo {
            total_cpu_cores: 0.0,
            available_cpu_cores: 0.0,
            total_memory_mb: 0,
            available_memory_mb: 0,
            total_disk_mb: 0,
            available_disk_mb: 0,
            running_executions: 0,
            max_executions: 0,
        }));

        let score = placement.score_backend(&job, &config, &(backend as Arc<dyn ExecutionBackend>)).await.unwrap();
        assert!(!score.is_nan());
        assert!(score >= 0.0);
    }

    #[tokio::test]
    async fn test_score_backend_cannot_handle() {
        let placement = JobPlacement::new(PlacementPolicy::BestFit);
        let job = create_test_job();
        let config = create_test_config();

        let score = placement.score_backend(&job, &config, &(backend as Arc<dyn ExecutionBackend>)).await.unwrap();
        assert_eq!(score, 0.0);
    }

    #[tokio::test]
    async fn test_nan_handling_in_score_comparison() {
        let placement = JobPlacement::new(PlacementPolicy::BestFit);
        let job = create_test_job();
        let config = create_test_config();

        // Create a backend that might produce NaN scores
        let backend1 = Arc::new(MockBackend::new("backend1").with_resources(ResourceInfo {
            total_cpu_cores: 0.0,
            available_cpu_cores: 0.0,
            total_memory_mb: 0,
            available_memory_mb: 0,
            total_disk_mb: 0,
            available_disk_mb: 0,
            running_executions: 0,
            max_executions: 0,
        }));

        let backend2 = Arc::new(MockBackend::new("backend2").with_resources(ResourceInfo {
            total_cpu_cores: 8.0,
            available_cpu_cores: 4.0,
            total_memory_mb: 16384,
            available_memory_mb: 8192,
            total_disk_mb: 102400,
            available_disk_mb: 51200,
            running_executions: 2,
            max_executions: 10,
        }));

        let backends = create_backends(vec![backend1, backend2]);
        let decision = placement.place_job(&job, &config, &backends).await.unwrap();

        // Should select backend2 (valid score) over backend1 (potentially NaN)
        assert_eq!(decision.backend_name, "backend2");
        assert!(!decision.score.is_nan());
    }

    #[tokio::test]
    async fn test_placement_decision_includes_alternatives() {
        let placement = JobPlacement::new(PlacementPolicy::BestFit);
        let job = create_test_job();
        let config = create_test_config();

        let backend1 = Arc::new(MockBackend::new("backend1"));
        let backend2 = Arc::new(MockBackend::new("backend2"));
        let backend3 = Arc::new(MockBackend::new("backend3"));
        let backend4 = Arc::new(MockBackend::new("backend4"));

        let backends = create_backends(vec![backend1, backend2, backend3, backend4]);
        let decision = placement.place_job(&job, &config, &backends).await.unwrap();

        // Should have at most 3 alternatives
        assert!(decision.alternatives.len() <= 3);
        // Selected backend should not be in alternatives
        assert!(!decision.alternatives.contains(&decision.backend_name));
    }

    #[tokio::test]
    async fn test_placement_decision_reason() {
        let placement = JobPlacement::new(PlacementPolicy::BestFit);
        let job = create_test_job();
        let config = create_test_config();

        let backend = Arc::new(MockBackend::new("backend1"));
        let backends = create_backends(vec![backend]);
        let decision = placement.place_job(&job, &config, &backends).await.unwrap();

        assert!(decision.reason.contains("BestFit"));
    }

    #[tokio::test]
    async fn test_placement_builder_default() {
        let builder = PlacementBuilder::default();
        let strategy = builder.build();
        assert_eq!(strategy.policy(), PlacementPolicy::BestFit);
    }

    #[tokio::test]
    async fn test_placement_builder_with_policy() {
        let builder = PlacementBuilder::new().with_policy(PlacementPolicy::RoundRobin);
        let strategy = builder.build();
        assert_eq!(strategy.policy(), PlacementPolicy::RoundRobin);
    }

    #[tokio::test]
    async fn test_placement_builder_with_constraint() {
        let builder = PlacementBuilder::new()
            .with_constraint("region".to_string(), "us-west".to_string());
        let _strategy = builder.build();
        // Constraints are currently stored but not used - just verify it doesn't panic
    }

    #[tokio::test]
    async fn test_placement_builder_with_preference() {
        let builder = PlacementBuilder::new()
            .with_preference("cpu".to_string(), 0.8);
        let _strategy = builder.build();
        // Preferences are currently stored but not used - just verify it doesn't panic
    }

    #[tokio::test]
    async fn test_placement_builder_fluent_api() {
        let strategy = PlacementBuilder::new()
            .with_policy(PlacementPolicy::WorstFit)
            .with_constraint("region".to_string(), "us-east".to_string())
            .with_preference("memory".to_string(), 0.6)
            .build();

        assert_eq!(strategy.policy(), PlacementPolicy::WorstFit);
    }

    #[tokio::test]
    async fn test_bestfit_high_cpu_utilization() {
        let placement = JobPlacement::new(PlacementPolicy::BestFit);
        let job = create_test_job();
        let config = create_test_config();

        let backend_high_cpu = Arc::new(MockBackend::new("high_cpu").with_resources(ResourceInfo {
            total_cpu_cores: 16.0,
            available_cpu_cores: 14.0, // 87.5% available
            total_memory_mb: 16384,
            available_memory_mb: 8192, // 50% available
            total_disk_mb: 102400,
            available_disk_mb: 51200,
            running_executions: 5,
            max_executions: 10, // 50% available
        }));

        let backend_low_cpu = Arc::new(MockBackend::new("low_cpu").with_resources(ResourceInfo {
            total_cpu_cores: 16.0,
            available_cpu_cores: 2.0, // 12.5% available
            total_memory_mb: 16384,
            available_memory_mb: 8192, // 50% available
            total_disk_mb: 102400,
            available_disk_mb: 51200,
            running_executions: 5,
            max_executions: 10, // 50% available
        }));

        let backends = create_backends(vec![backend_high_cpu, backend_low_cpu]);
        let decision = placement.place_job(&job, &config, &backends).await.unwrap();

        // BestFit should prefer backend with more available CPU
        assert_eq!(decision.backend_name, "high_cpu");
    }

    #[tokio::test]
    async fn test_bestfit_high_memory_utilization() {
        let placement = JobPlacement::new(PlacementPolicy::BestFit);
        let job = create_test_job();
        let config = create_test_config();

        let backend_high_mem = Arc::new(MockBackend::new("high_mem").with_resources(ResourceInfo {
            total_cpu_cores: 8.0,
            available_cpu_cores: 4.0, // 50% available
            total_memory_mb: 32768,
            available_memory_mb: 28672, // 87.5% available
            total_disk_mb: 102400,
            available_disk_mb: 51200,
            running_executions: 5,
            max_executions: 10, // 50% available
        }));

        let backend_low_mem = Arc::new(MockBackend::new("low_mem").with_resources(ResourceInfo {
            total_cpu_cores: 8.0,
            available_cpu_cores: 4.0, // 50% available
            total_memory_mb: 32768,
            available_memory_mb: 4096, // 12.5% available
            total_disk_mb: 102400,
            available_disk_mb: 51200,
            running_executions: 5,
            max_executions: 10, // 50% available
        }));

        let backends = create_backends(vec![backend_high_mem, backend_low_mem]);
        let decision = placement.place_job(&job, &config, &backends).await.unwrap();

        // BestFit should prefer backend with more available memory
        assert_eq!(decision.backend_name, "high_mem");
    }

    #[tokio::test]
    async fn test_bestfit_high_capacity_utilization() {
        let placement = JobPlacement::new(PlacementPolicy::BestFit);
        let job = create_test_job();
        let config = create_test_config();

        let backend_high_capacity = Arc::new(MockBackend::new("high_capacity").with_resources(ResourceInfo {
            total_cpu_cores: 8.0,
            available_cpu_cores: 4.0, // 50% available
            total_memory_mb: 16384,
            available_memory_mb: 8192, // 50% available
            total_disk_mb: 102400,
            available_disk_mb: 51200,
            running_executions: 2,
            max_executions: 20, // 90% available
        }));

        let backend_low_capacity = Arc::new(MockBackend::new("low_capacity").with_resources(ResourceInfo {
            total_cpu_cores: 8.0,
            available_cpu_cores: 4.0, // 50% available
            total_memory_mb: 16384,
            available_memory_mb: 8192, // 50% available
            total_disk_mb: 102400,
            available_disk_mb: 51200,
            running_executions: 9,
            max_executions: 10, // 10% available
        }));

        let backends = create_backends(vec![backend_high_capacity, backend_low_capacity]);
        let decision = placement.place_job(&job, &config, &backends).await.unwrap();

        // BestFit should prefer backend with more available execution slots
        assert_eq!(decision.backend_name, "high_capacity");
    }

    #[tokio::test]
    async fn test_worstfit_prefers_busy_backend() {
        let placement = JobPlacement::new(PlacementPolicy::WorstFit);
        let job = create_test_job();
        let config = create_test_config();

        let backend_idle = Arc::new(MockBackend::new("idle").with_resources(ResourceInfo {
            total_cpu_cores: 8.0,
            available_cpu_cores: 7.0, // 87.5% available
            total_memory_mb: 16384,
            available_memory_mb: 14336, // 87.5% available
            total_disk_mb: 102400,
            available_disk_mb: 89600,
            running_executions: 1,
            max_executions: 10, // 90% available
        }));

        let backend_busy = Arc::new(MockBackend::new("busy").with_resources(ResourceInfo {
            total_cpu_cores: 8.0,
            available_cpu_cores: 1.0, // 12.5% available
            total_memory_mb: 16384,
            available_memory_mb: 2048, // 12.5% available
            total_disk_mb: 102400,
            available_disk_mb: 12800,
            running_executions: 9,
            max_executions: 10, // 10% available
        }));

        let backends = create_backends(vec![backend_idle, backend_busy]);
        let decision = placement.place_job(&job, &config, &backends).await.unwrap();

        // WorstFit should prefer the busy backend
        assert_eq!(decision.backend_name, "busy");
    }

    #[tokio::test]
    async fn test_firstfit_with_one_incompatible_backend() {
        let placement = JobPlacement::new(PlacementPolicy::FirstFit);
        let job = create_test_job();
        let config = create_test_config();

        let backend_incompatible = Arc::new(MockBackend::new("incompatible").with_can_handle(false));
        let backend_compatible = Arc::new(MockBackend::new("compatible"));

        let backends = create_backends(vec![backend_incompatible, backend_compatible]);
        let decision = placement.place_job(&job, &config, &backends).await.unwrap();

        assert_eq!(decision.backend_name, "compatible");
    }

    #[tokio::test]
    async fn test_roundrobin_with_single_backend() {
        let placement = JobPlacement::new(PlacementPolicy::RoundRobin);
        let job = create_test_job();
        let config = create_test_config();

        let backend = Arc::new(MockBackend::new("only_backend"));
        let backends = create_backends(vec![backend]);

        // Place job multiple times
        for _ in 0..5 {
            let decision = placement.place_job(&job, &config, &backends).await.unwrap();
            assert_eq!(decision.backend_name, "only_backend");
        }
    }

    #[tokio::test]
    async fn test_explicit_with_no_placement_hint() {
        let placement = JobPlacement::new(PlacementPolicy::Explicit);
        let job = create_test_job();
        let config = create_test_config(); // No placement hints

        let backend = Arc::new(MockBackend::new("backend1"));
        let backends = create_backends(vec![backend]);

        let result = placement.place_job(&job, &config, &backends).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_explicit_with_wrong_hint_key() {
        let placement = JobPlacement::new(PlacementPolicy::Explicit);
        let job = create_test_job();
        let mut config = create_test_config();
        config.placement_hints.insert("region".to_string(), "backend1".to_string()); // Wrong key

        let backend = Arc::new(MockBackend::new("backend1"));
        let backends = create_backends(vec![backend]);

        let result = placement.place_job(&job, &config, &backends).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_policy_equality() {
        assert_eq!(PlacementPolicy::BestFit, PlacementPolicy::BestFit);
        assert_eq!(PlacementPolicy::WorstFit, PlacementPolicy::WorstFit);
        assert_eq!(PlacementPolicy::FirstFit, PlacementPolicy::FirstFit);
        assert_eq!(PlacementPolicy::RoundRobin, PlacementPolicy::RoundRobin);
        assert_eq!(PlacementPolicy::Explicit, PlacementPolicy::Explicit);
        assert_eq!(
            PlacementPolicy::Custom("test".to_string()),
            PlacementPolicy::Custom("test".to_string())
        );

        assert_ne!(PlacementPolicy::BestFit, PlacementPolicy::WorstFit);
        assert_ne!(
            PlacementPolicy::Custom("test1".to_string()),
            PlacementPolicy::Custom("test2".to_string())
        );
    }

    #[tokio::test]
    async fn test_placement_decision_serialization() {
        let decision = PlacementDecision {
            backend_name: "test_backend".to_string(),
            score: 85.5,
            reason: "Best fit".to_string(),
            alternatives: vec!["alt1".to_string(), "alt2".to_string()],
        };

        let serialized = serde_json::to_string(&decision).unwrap();
        let deserialized: PlacementDecision = serde_json::from_str(&serialized).unwrap();

        assert_eq!(decision.backend_name, deserialized.backend_name);
        assert_eq!(decision.score, deserialized.score);
        assert_eq!(decision.reason, deserialized.reason);
        assert_eq!(decision.alternatives, deserialized.alternatives);
    }

    #[tokio::test]
    async fn test_placement_policy_serialization() {
        let policies = vec![
            PlacementPolicy::BestFit,
            PlacementPolicy::WorstFit,
            PlacementPolicy::FirstFit,
            PlacementPolicy::RoundRobin,
            PlacementPolicy::Explicit,
            PlacementPolicy::Custom("custom".to_string()),
        ];

        for policy in policies {
            let serialized = serde_json::to_string(&policy).unwrap();
            let deserialized: PlacementPolicy = serde_json::from_str(&serialized).unwrap();
            assert_eq!(policy, deserialized);
        }
    }

    #[tokio::test]
    async fn test_score_with_full_capacity() {
        let placement = JobPlacement::new(PlacementPolicy::BestFit);
        let job = create_test_job();
        let config = create_test_config();

        let backend = Arc::new(MockBackend::new("full").with_resources(ResourceInfo {
            total_cpu_cores: 8.0,
            available_cpu_cores: 0.0,
            total_memory_mb: 16384,
            available_memory_mb: 0,
            total_disk_mb: 102400,
            available_disk_mb: 0,
            running_executions: 10,
            max_executions: 10,
        }));

        let score = placement.score_backend(&job, &config, &backend).await.unwrap();

        // Should still return a valid score (0.0 for BestFit with no resources)
        assert_eq!(score, 0.0);
    }

    #[tokio::test]
    async fn test_score_with_over_capacity() {
        let placement = JobPlacement::new(PlacementPolicy::BestFit);
        let job = create_test_job();
        let config = create_test_config();

        let backend = Arc::new(MockBackend::new("over").with_resources(ResourceInfo {
            total_cpu_cores: 8.0,
            available_cpu_cores: 8.0,
            total_memory_mb: 16384,
            available_memory_mb: 16384,
            total_disk_mb: 102400,
            available_disk_mb: 102400,
            running_executions: 0,
            max_executions: 10,
        }));

        let score = placement.score_backend(&job, &config, &backend).await.unwrap();

        // Should return maximum score when fully available
        assert_eq!(score, 100.0);
    }

    #[tokio::test]
    async fn test_multiple_backends_with_mixed_scores() {
        let placement = JobPlacement::new(PlacementPolicy::BestFit);
        let job = create_test_job();
        let config = create_test_config();

        let backend1 = Arc::new(MockBackend::new("backend1").with_resources(ResourceInfo {
            total_cpu_cores: 8.0,
            available_cpu_cores: 2.0, // 25%
            total_memory_mb: 16384,
            available_memory_mb: 4096, // 25%
            total_disk_mb: 102400,
            available_disk_mb: 25600,
            running_executions: 7,
            max_executions: 10, // 30%
        }));

        let backend2 = Arc::new(MockBackend::new("backend2").with_resources(ResourceInfo {
            total_cpu_cores: 8.0,
            available_cpu_cores: 4.0, // 50%
            total_memory_mb: 16384,
            available_memory_mb: 8192, // 50%
            total_disk_mb: 102400,
            available_disk_mb: 51200,
            running_executions: 5,
            max_executions: 10, // 50%
        }));

        let backend3 = Arc::new(MockBackend::new("backend3").with_resources(ResourceInfo {
            total_cpu_cores: 8.0,
            available_cpu_cores: 6.0, // 75%
            total_memory_mb: 16384,
            available_memory_mb: 12288, // 75%
            total_disk_mb: 102400,
            available_disk_mb: 76800,
            running_executions: 2,
            max_executions: 10, // 80%
        }));

        let backends = create_backends(vec![backend1, backend2, backend3]);
        let decision = placement.place_job(&job, &config, &backends).await.unwrap();

        // BestFit should select backend3 (most available)
        assert_eq!(decision.backend_name, "backend3");
        assert!(decision.score > 70.0);
    }
}
