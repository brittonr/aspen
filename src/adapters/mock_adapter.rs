//! Mock execution backend for testing

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::domain::types::Job;
use crate::worker_trait::WorkResult;

use super::{
    BackendHealth, ExecutionBackend, ExecutionConfig, ExecutionHandle, ExecutionMetadata,
    ExecutionStatus, ResourceInfo,
};

/// Configuration for mock adapter behavior
#[derive(Debug, Clone)]
pub struct MockConfig {
    /// Default execution delay in milliseconds
    pub execution_delay_ms: u64,
    /// Probability of job failure (0.0 to 1.0)
    pub failure_rate: f32,
    /// Maximum concurrent executions
    pub max_concurrent: usize,
    /// Whether to simulate resource constraints
    pub simulate_resources: bool,
    /// Total available CPU cores
    pub total_cpu: f32,
    /// Total available memory in MB
    pub total_memory_mb: u32,
}

impl Default for MockConfig {
    fn default() -> Self {
        Self {
            execution_delay_ms: 100,
            failure_rate: 0.0,
            max_concurrent: 10,
            simulate_resources: false,
            total_cpu: 8.0,
            total_memory_mb: 16384,
        }
    }
}

/// Mock execution state
#[derive(Debug, Clone)]
struct MockExecution {
    handle: ExecutionHandle,
    job: Job,
    config: ExecutionConfig,
    status: ExecutionStatus,
    started_at: u64,
    completed_at: Option<u64>,
}

/// Mock execution backend for testing
pub struct MockAdapter {
    config: MockConfig,
    executions: Arc<RwLock<HashMap<String, MockExecution>>>,
    execution_counter: Arc<RwLock<u64>>,
    resource_usage: Arc<RwLock<(f32, u32)>>, // (cpu, memory)
}

impl MockAdapter {
    /// Create a new mock adapter
    pub fn new(config: MockConfig) -> Self {
        Self {
            config,
            executions: Arc::new(RwLock::new(HashMap::new())),
            execution_counter: Arc::new(RwLock::new(0)),
            resource_usage: Arc::new(RwLock::new((0.0, 0))),
        }
    }

    /// Create a mock adapter with default configuration
    pub fn default() -> Self {
        Self::new(MockConfig::default())
    }

    /// Set the failure rate for testing error handling
    pub fn with_failure_rate(mut self, rate: f32) -> Self {
        self.config.failure_rate = rate.clamp(0.0, 1.0);
        self
    }

    /// Set the execution delay for testing async behavior
    pub fn with_delay(mut self, delay_ms: u64) -> Self {
        self.config.execution_delay_ms = delay_ms;
        self
    }

    /// Generate a unique execution ID
    async fn generate_id(&self) -> String {
        let mut counter = self.execution_counter.write().await;
        *counter += 1;
        format!("mock-exec-{}", *counter)
    }

    /// Simulate job execution
    async fn simulate_execution(&self, handle: ExecutionHandle) -> Result<()> {
        // Wait for configured delay
        tokio::time::sleep(Duration::from_millis(self.config.execution_delay_ms)).await;

        // Determine success or failure
        // Use a simple deterministic approach for now (can be improved with rand later)
        let should_fail = if self.config.failure_rate > 0.0 {
            // Use execution time as a pseudo-random source
            let now_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis();
            ((now_ms % 100) as f32 / 100.0) < self.config.failure_rate
        } else {
            false
        };

        let mut executions = self.executions.write().await;
        if let Some(exec) = executions.get_mut(&handle.id) {
            let completed_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            if should_fail {
                exec.status = ExecutionStatus::Failed("Simulated failure".to_string());
            } else {
                exec.status = ExecutionStatus::Completed(WorkResult {
                    success: true,
                    output: Some(serde_json::Value::String("Mock execution completed successfully".to_string())),
                    error: None,
                });
            }
            exec.completed_at = Some(completed_at);

            // Release resources if simulating
            if self.config.simulate_resources {
                let mut usage = self.resource_usage.write().await;
                usage.0 -= exec.config.resources.cpu_cores;
                usage.1 -= exec.config.resources.memory_mb;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl ExecutionBackend for MockAdapter {
    fn backend_type(&self) -> &str {
        "mock"
    }

    async fn submit_job(&self, job: Job, config: ExecutionConfig) -> Result<ExecutionHandle> {
        debug!("Mock adapter: submitting job {}", job.id);

        // Check resource availability if simulating
        if self.config.simulate_resources {
            let usage = self.resource_usage.read().await;
            if usage.0 + config.resources.cpu_cores > self.config.total_cpu {
                return Err(anyhow::anyhow!("Insufficient CPU resources"));
            }
            if usage.1 + config.resources.memory_mb > self.config.total_memory_mb {
                return Err(anyhow::anyhow!("Insufficient memory resources"));
            }
        }

        // Check concurrent execution limit
        let executions = self.executions.read().await;
        let running_count = executions
            .values()
            .filter(|e| matches!(e.status, ExecutionStatus::Running))
            .count();

        if running_count >= self.config.max_concurrent {
            return Err(anyhow::anyhow!("Maximum concurrent executions reached"));
        }
        drop(executions);

        // Generate execution handle
        let exec_id = self.generate_id().await;
        let handle = ExecutionHandle::mock(exec_id.clone(), job.id.clone());

        // Create execution record
        let execution = MockExecution {
            handle: handle.clone(),
            job: job.clone(),
            config: config.clone(),
            status: ExecutionStatus::Running,
            started_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            completed_at: None,
        };

        // Store execution
        let mut executions = self.executions.write().await;
        executions.insert(handle.id.clone(), execution);
        drop(executions);

        // Update resource usage if simulating
        if self.config.simulate_resources {
            let mut usage = self.resource_usage.write().await;
            usage.0 += config.resources.cpu_cores;
            usage.1 += config.resources.memory_mb;
        }

        // Spawn background task to simulate execution
        let self_clone = Arc::new(self.clone());
        let handle_clone = handle.clone();
        tokio::spawn(async move {
            if let Err(e) = self_clone.simulate_execution(handle_clone).await {
                tracing::error!("Mock execution simulation failed: {}", e);
            }
        });

        info!("Mock adapter: job {} submitted as {}", job.id, handle.id);
        Ok(handle)
    }

    async fn get_status(&self, handle: &ExecutionHandle) -> Result<ExecutionStatus> {
        let executions = self.executions.read().await;
        executions
            .get(&handle.id)
            .map(|e| e.status.clone())
            .ok_or_else(|| anyhow::anyhow!("Execution not found: {}", handle.id))
    }

    async fn cancel_execution(&self, handle: &ExecutionHandle) -> Result<()> {
        let mut executions = self.executions.write().await;
        if let Some(exec) = executions.get_mut(&handle.id) {
            if matches!(exec.status, ExecutionStatus::Running) {
                exec.status = ExecutionStatus::Cancelled;
                exec.completed_at = Some(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                );

                // Release resources if simulating
                if self.config.simulate_resources {
                    let mut usage = self.resource_usage.write().await;
                    usage.0 -= exec.config.resources.cpu_cores;
                    usage.1 -= exec.config.resources.memory_mb;
                }

                info!("Mock adapter: cancelled execution {}", handle.id);
                Ok(())
            } else {
                Err(anyhow::anyhow!("Execution is not running"))
            }
        } else {
            Err(anyhow::anyhow!("Execution not found: {}", handle.id))
        }
    }

    async fn get_metadata(&self, handle: &ExecutionHandle) -> Result<ExecutionMetadata> {
        let executions = self.executions.read().await;
        let exec = executions
            .get(&handle.id)
            .ok_or_else(|| anyhow::anyhow!("Execution not found: {}", handle.id))?;

        Ok(ExecutionMetadata {
            execution_id: handle.id.clone(),
            backend_type: "mock".to_string(),
            started_at: exec.started_at,
            completed_at: exec.completed_at,
            node_id: Some("mock-node".to_string()),
            custom: HashMap::new(),
        })
    }

    async fn wait_for_completion(
        &self,
        handle: &ExecutionHandle,
        timeout: Option<Duration>,
    ) -> Result<ExecutionStatus> {
        let start = std::time::Instant::now();
        let timeout = timeout.unwrap_or(Duration::from_secs(300)); // 5 min default

        loop {
            let status = self.get_status(handle).await?;
            match status {
                ExecutionStatus::Completed(_) | ExecutionStatus::Failed(_) | ExecutionStatus::Cancelled => {
                    return Ok(status);
                }
                _ => {
                    if start.elapsed() >= timeout {
                        return Err(anyhow::anyhow!("Timeout waiting for completion"));
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
    }

    async fn health_check(&self) -> Result<BackendHealth> {
        let executions = self.executions.read().await;
        let running_count = executions
            .values()
            .filter(|e| matches!(e.status, ExecutionStatus::Running))
            .count();

        let resource_info = self.get_resource_info().await?;

        Ok(BackendHealth {
            healthy: true,
            status_message: format!("Mock adapter healthy, {} executions running", running_count),
            resource_info: Some(resource_info),
            last_check: Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            ),
            details: HashMap::from([
                ("type".to_string(), "mock".to_string()),
                ("failure_rate".to_string(), self.config.failure_rate.to_string()),
            ]),
        })
    }

    async fn get_resource_info(&self) -> Result<ResourceInfo> {
        let executions = self.executions.read().await;
        let running_count = executions
            .values()
            .filter(|e| matches!(e.status, ExecutionStatus::Running))
            .count();

        let (used_cpu, used_memory) = if self.config.simulate_resources {
            let usage = self.resource_usage.read().await;
            (usage.0, usage.1)
        } else {
            (0.0, 0)
        };

        Ok(ResourceInfo {
            total_cpu_cores: self.config.total_cpu,
            available_cpu_cores: self.config.total_cpu - used_cpu,
            total_memory_mb: self.config.total_memory_mb,
            available_memory_mb: self.config.total_memory_mb - used_memory,
            total_disk_mb: 100000, // Mock value
            available_disk_mb: 50000, // Mock value
            running_executions: running_count,
            max_executions: self.config.max_concurrent,
        })
    }

    async fn can_handle(&self, _job: &Job, config: &ExecutionConfig) -> Result<bool> {
        // Check if we have resources
        if self.config.simulate_resources {
            let usage = self.resource_usage.read().await;
            if usage.0 + config.resources.cpu_cores > self.config.total_cpu {
                return Ok(false);
            }
            if usage.1 + config.resources.memory_mb > self.config.total_memory_mb {
                return Ok(false);
            }
        }

        // Check concurrent limit
        let executions = self.executions.read().await;
        let running_count = executions
            .values()
            .filter(|e| matches!(e.status, ExecutionStatus::Running))
            .count();

        Ok(running_count < self.config.max_concurrent)
    }

    async fn list_executions(&self) -> Result<Vec<ExecutionHandle>> {
        let executions = self.executions.read().await;
        Ok(executions.values().map(|e| e.handle.clone()).collect())
    }

    async fn cleanup_executions(&self, older_than: Duration) -> Result<usize> {
        let mut executions = self.executions.write().await;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let cutoff = now - older_than.as_secs();
        let mut cleaned = 0;

        executions.retain(|_, exec| {
            if let Some(completed_at) = exec.completed_at {
                if completed_at < cutoff {
                    cleaned += 1;
                    return false;
                }
            }
            true
        });

        debug!("Mock adapter: cleaned up {} old executions", cleaned);
        Ok(cleaned)
    }
}

impl Clone for MockAdapter {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            executions: self.executions.clone(),
            execution_counter: self.execution_counter.clone(),
            resource_usage: self.resource_usage.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_adapter_creation() {
        let adapter = MockAdapter::default();
        assert_eq!(adapter.backend_type(), "mock");

        let health = adapter.health_check().await.unwrap();
        assert!(health.healthy);
    }

    #[tokio::test]
    async fn test_mock_job_submission() {
        let adapter = MockAdapter::default();

        let job = Job {
            id: "test-job".to_string(),
            data: vec![],
            compatible_worker_types: vec!["mock".to_string()],
            ..Default::default()
        };

        let config = ExecutionConfig::default();
        let handle = adapter.submit_job(job, config).await.unwrap();

        assert!(handle.is_mock());
        assert_eq!(handle.job_id, "test-job");

        // Check status
        let status = adapter.get_status(&handle).await.unwrap();
        assert!(matches!(status, ExecutionStatus::Running));

        // Wait for completion
        let final_status = adapter
            .wait_for_completion(&handle, Some(Duration::from_secs(1)))
            .await
            .unwrap();
        assert!(matches!(final_status, ExecutionStatus::Completed(_)));
    }

    #[tokio::test]
    async fn test_mock_adapter_with_failure() {
        let adapter = MockAdapter::default().with_failure_rate(1.0); // Always fail

        let job = Job {
            id: "failing-job".to_string(),
            data: vec![],
            compatible_worker_types: vec!["mock".to_string()],
            ..Default::default()
        };

        let config = ExecutionConfig::default();
        let handle = adapter.submit_job(job, config).await.unwrap();

        // Wait for completion
        let final_status = adapter
            .wait_for_completion(&handle, Some(Duration::from_secs(1)))
            .await
            .unwrap();
        assert!(matches!(final_status, ExecutionStatus::Failed(_)));
    }

    #[tokio::test]
    async fn test_mock_adapter_cancellation() {
        let adapter = MockAdapter::default().with_delay(5000); // Long delay

        let job = Job {
            id: "cancel-job".to_string(),
            data: vec![],
            compatible_worker_types: vec!["mock".to_string()],
            ..Default::default()
        };

        let config = ExecutionConfig::default();
        let handle = adapter.submit_job(job, config).await.unwrap();

        // Cancel immediately
        adapter.cancel_execution(&handle).await.unwrap();

        let status = adapter.get_status(&handle).await.unwrap();
        assert!(matches!(status, ExecutionStatus::Cancelled));
    }
}