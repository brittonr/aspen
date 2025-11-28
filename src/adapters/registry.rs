//! Registry for managing multiple execution backends
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use crate::domain::types::Job;
use crate::common::timestamp::current_timestamp_or_zero;
use super::{
    BackendHealth, ExecutionBackend, ExecutionConfig, ExecutionHandle, ExecutionStatus,
    JobPlacement, PlacementPolicy, PlacementStrategy,
};
use super::cleanup::{CleanupConfig, CleanupMetrics};
/// Configuration for the execution registry
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    /// Default placement policy for job routing
    pub placement_policy: PlacementPolicy,
    /// Whether to automatically failover when a backend is unhealthy
    pub enable_failover: bool,
    /// Interval for health checks in seconds
    pub health_check_interval: u64,
    /// Maximum retries for failed job submissions
    pub max_submission_retries: u32,
    /// Cleanup configuration for handle tracking
    pub cleanup_config: CleanupConfig,
}
impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            placement_policy: PlacementPolicy::BestFit,
            enable_failover: true,
            health_check_interval: 30,
            max_submission_retries: 3,
            cleanup_config: CleanupConfig::default(),
        }
    }
}
/// Tracking entry for handle-to-backend mapping
#[derive(Debug, Clone)]
struct HandleMapping {
    backend_name: String,
    created_at: u64,
    last_accessed: u64,
}

/// Registry for managing multiple execution backends
///
/// This provides a unified interface for submitting jobs to various backends,
/// with automatic routing, health monitoring, and failover capabilities.
pub struct ExecutionRegistry {
    /// Registered execution backends
    backends: Arc<RwLock<HashMap<String, Arc<dyn ExecutionBackend>>>>,
    /// Placement strategy for routing jobs
    placement_strategy: Arc<dyn PlacementStrategy>,
    /// Registry configuration
    config: RegistryConfig,
    /// Health status of backends (cached for performance)
    health_cache: Arc<RwLock<HashMap<String, BackendHealth>>>,
    /// Mapping from execution handles to backend names with metadata
    handle_to_backend: Arc<RwLock<HashMap<String, HandleMapping>>>,
    /// Cleanup metrics
    cleanup_metrics: Arc<RwLock<CleanupMetrics>>,
    /// Background cleanup task handle
    cleanup_task_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}
impl ExecutionRegistry {
    /// Create a new execution registry
    pub fn new(config: RegistryConfig) -> Self {
        let placement_strategy = Arc::new(JobPlacement::new(config.placement_policy.clone()));
        let registry = Self {
            backends: Arc::new(RwLock::new(HashMap::new())),
            placement_strategy: placement_strategy as Arc<dyn PlacementStrategy>,
            config: config.clone(),
            health_cache: Arc::new(RwLock::new(HashMap::new())),
            handle_to_backend: Arc::new(RwLock::new(HashMap::new())),
            cleanup_metrics: Arc::new(RwLock::new(CleanupMetrics::new())),
            cleanup_task_handle: Arc::new(RwLock::new(None)),
        };

        // Start background cleanup if enabled
        if config.cleanup_config.enable_background_cleanup {
            registry.start_background_cleanup();
        }

        registry
    }

    /// Start background cleanup task for handle mappings
    fn start_background_cleanup(&self) {
        let handle_to_backend = self.handle_to_backend.clone();
        let backends = self.backends.clone();
        let config = self.config.cleanup_config.clone();
        let metrics = self.cleanup_metrics.clone();
        let cleanup_task_handle = self.cleanup_task_handle.clone();

        let task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.cleanup_interval);
            loop {
                interval.tick().await;

                debug!("Running background cleanup for execution registry");
                let start = std::time::Instant::now();

                // Clean up orphaned handles (backends that no longer exist)
                let mut handle_map = handle_to_backend.write().await;
                let backend_list = backends.read().await;

                let mut orphaned = 0;
                handle_map.retain(|_handle_id, mapping| {
                    let exists = backend_list.contains_key(&mapping.backend_name);
                    if !exists {
                        orphaned += 1;
                    }
                    exists
                });
                drop(backend_list);

                // Clean up old handles based on age (using configured TTL)
                let now = current_timestamp_or_zero() as u64;
                let cutoff = now.saturating_sub(config.completed_ttl.as_secs());

                let mut ttl_cleaned = 0;
                handle_map.retain(|_handle_id, mapping| {
                    if mapping.created_at < cutoff {
                        ttl_cleaned += 1;
                        return false;
                    }
                    true
                });

                // LRU eviction if over size limit
                let mut lru_cleaned = 0;
                if handle_map.len() > config.max_entries {
                    let mut entries: Vec<_> = handle_map
                        .iter()
                        .map(|(k, v)| (k.clone(), v.last_accessed))
                        .collect();

                    entries.sort_by_key(|(_, last_accessed)| *last_accessed);

                    let to_remove = handle_map.len() - config.max_entries;

                    for (key, _) in entries.iter().take(to_remove) {
                        handle_map.remove(key);
                        lru_cleaned += 1;
                    }
                }

                drop(handle_map);

                let duration = start.elapsed();

                if orphaned > 0 || ttl_cleaned > 0 || lru_cleaned > 0 {
                    info!(
                        "Registry cleanup: {} orphaned, {} TTL expired, {} LRU evicted in {:?}",
                        orphaned, ttl_cleaned, lru_cleaned, duration
                    );
                }

                // Update metrics
                let mut m = metrics.write().await;
                m.record_cleanup(ttl_cleaned + orphaned, lru_cleaned, duration);
            }
        });

        // Store task handle
        let handle_clone = cleanup_task_handle.clone();
        tokio::spawn(async move {
            let mut handle = handle_clone.write().await;
            *handle = Some(task);
        });
    }

    /// Get cleanup metrics
    pub async fn get_cleanup_metrics(&self) -> CleanupMetrics {
        let metrics = self.cleanup_metrics.read().await;
        metrics.clone()
    }
    /// Register a new execution backend
    pub async fn register_backend(
        &self,
        name: String,
        backend: Arc<dyn ExecutionBackend>,
    ) -> Result<()> {
        info!("Registering execution backend: {}", name);
        // Initialize the backend
        backend.initialize().await?;
        // Perform initial health check
        let health = backend.health_check().await?;
        if !health.healthy {
            warn!(
                "Backend {} is not healthy on registration: {}",
                name, health.status_message
            );
        }
        // Store backend and health status
        let mut backends = self.backends.write().await;
        let mut health_cache = self.health_cache.write().await;
        backends.insert(name.clone(), backend);
        health_cache.insert(name.clone(), health);
        info!("Successfully registered backend: {}", name);
        Ok(())
    }
    /// Unregister an execution backend
    pub async fn unregister_backend(&self, name: &str) -> Result<()> {
        info!("Unregistering execution backend: {}", name);
        let mut backends = self.backends.write().await;
        let mut health_cache = self.health_cache.write().await;
        if let Some(backend) = backends.remove(name) {
            // Shutdown the backend
            if let Err(e) = backend.shutdown().await {
                error!("Error shutting down backend {}: {}", name, e);
            }
        }
        health_cache.remove(name);
        info!("Successfully unregistered backend: {}", name);
        Ok(())
    }
    /// Get a backend by name
    pub async fn get_backend(&self, name: &str) -> Option<Arc<dyn ExecutionBackend>> {
        let backends = self.backends.read().await;
        backends.get(name).cloned()
    }
    /// List all registered backends
    pub async fn list_backends(&self) -> Vec<String> {
        let backends = self.backends.read().await;
        backends.keys().cloned().collect()
    }
    /// Submit a job to an appropriate backend
    pub async fn submit_job(&self, job: Job, config: ExecutionConfig) -> Result<ExecutionHandle> {
        let mut retries = 0;
        let max_retries = self.config.max_submission_retries;

        loop {
            let healthy_backends = self.get_healthy_backends().await?;
            if healthy_backends.is_empty() {
                return Err(anyhow::anyhow!("No healthy execution backends available"));
            }

            let decision = self
                .placement_strategy
                .place_job(&job, &config, &healthy_backends)
                .await?;

            info!(
                "Placing job {} on backend {} (score: {:.2})",
                job.id, decision.backend_name, decision.score
            );

            match self.try_submit_with_backend(
                &job,
                &config,
                &decision.backend_name,
                &healthy_backends
            ).await {
                Ok(handle) => return Ok(handle),
                Err(e) => {
                    warn!(
                        "Failed to submit job {} to backend {}: {}",
                        job.id, decision.backend_name, e
                    );

                    retries += 1;
                    if retries >= max_retries {
                        // Try failover to alternative backends
                        if let Ok(handle) = self.attempt_failover(
                            &job,
                            &config,
                            &decision.alternatives,
                            &healthy_backends
                        ).await {
                            return Ok(handle);
                        }

                        return Err(anyhow::anyhow!(
                            "Failed to submit job after {} retries: {}",
                            max_retries,
                            e
                        ));
                    }

                    // Wait before retrying
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    }

    /// Try to submit a job to a specific backend
    async fn try_submit_with_backend(
        &self,
        job: &Job,
        config: &ExecutionConfig,
        backend_name: &str,
        healthy_backends: &HashMap<String, Arc<dyn ExecutionBackend>>,
    ) -> Result<ExecutionHandle> {
        let backend = healthy_backends
            .get(backend_name)
            .ok_or_else(|| anyhow::anyhow!("Backend {} not found", backend_name))?;

        let handle = backend.submit_job(job.clone(), config.clone()).await?;

        // Record the handle mapping
        self.record_handle_mapping(handle.id.clone(), backend_name.to_string()).await;

        info!(
            "Successfully submitted job {} to backend {}",
            job.id, backend_name
        );

        Ok(handle)
    }

    /// Attempt failover to alternative backends
    async fn attempt_failover(
        &self,
        job: &Job,
        config: &ExecutionConfig,
        alternatives: &[String],
        healthy_backends: &HashMap<String, Arc<dyn ExecutionBackend>>,
    ) -> Result<ExecutionHandle> {
        if !self.config.enable_failover || alternatives.is_empty() {
            return Err(anyhow::anyhow!("Failover not available"));
        }

        info!(
            "Attempting failover for job {} to alternative backends",
            job.id
        );

        for alt_backend_name in alternatives {
            match self.try_submit_with_backend(
                job,
                config,
                alt_backend_name,
                healthy_backends
            ).await {
                Ok(handle) => {
                    info!(
                        "Failover successful: job {} submitted to {}",
                        job.id, alt_backend_name
                    );
                    return Ok(handle);
                }
                Err(e) => {
                    warn!("Failover to {} failed: {}", alt_backend_name, e);
                }
            }
        }

        Err(anyhow::anyhow!("All failover attempts failed"))
    }

    /// Record a handle mapping in the tracking map
    async fn record_handle_mapping(&self, handle_id: String, backend_name: String) {
        let now = current_timestamp_or_zero() as u64;
        let mapping = HandleMapping {
            backend_name,
            created_at: now,
            last_accessed: now,
        };

        let mut handle_map = self.handle_to_backend.write().await;
        handle_map.insert(handle_id, mapping);
    }
    /// Get the status of an execution
    pub async fn get_status(&self, handle: &ExecutionHandle) -> Result<ExecutionStatus> {
        let backend = self.get_backend_for_handle(handle).await?;
        backend.get_status(handle).await
    }
    /// Cancel an execution
    pub async fn cancel_execution(&self, handle: &ExecutionHandle) -> Result<()> {
        let backend = self.get_backend_for_handle(handle).await?;
        backend.cancel_execution(handle).await
    }

    /// Clean up orphaned execution handles
    ///
    /// Removes handles for backends that no longer exist.
    /// For proper status-based cleanup, backends should implement their own
    /// execution tracking with TTL or completion callbacks.
    pub async fn cleanup_orphaned_handles(&self) -> Result<usize> {
        let mut handle_map = self.handle_to_backend.write().await;
        let backends = self.backends.read().await;

        let mut removed_count = 0;
        handle_map.retain(|_handle_id, mapping| {
            let exists = backends.contains_key(&mapping.backend_name);
            if !exists {
                removed_count += 1;
            }
            exists
        });

        if removed_count > 0 {
            info!("Cleaned up {} orphaned handles from deregistered backends", removed_count);
        }
        Ok(removed_count)
    }

    /// Wait for an execution to complete
    pub async fn wait_for_completion(
        &self,
        handle: &ExecutionHandle,
        timeout: Option<std::time::Duration>,
    ) -> Result<ExecutionStatus> {
        let backend = self.get_backend_for_handle(handle).await?;
        backend.wait_for_completion(handle, timeout).await
    }
    /// Update health status for all backends
    pub async fn update_health_status(&self) -> Result<()> {
        let backends = self.backends.read().await;
        let mut health_cache = self.health_cache.write().await;
        for (name, backend) in backends.iter() {
            match backend.health_check().await {
                Ok(health) => {
                    if !health.healthy {
                        warn!("Backend {} is unhealthy: {}", name, health.status_message);
                    }
                    health_cache.insert(name.clone(), health);
                }
                Err(e) => {
                    error!("Health check failed for backend {}: {}", name, e);
                    // Mark as unhealthy
                    health_cache.insert(
                        name.clone(),
                        BackendHealth {
                            healthy: false,
                            status_message: format!("Health check failed: {}", e),
                            resource_info: None,
                            last_check: Some(current_timestamp_or_zero() as u64),
                            details: HashMap::new(),
                        },
                    );
                }
            }
        }
        Ok(())
    }
    /// Get health status of all backends
    pub async fn get_health_status(&self) -> HashMap<String, BackendHealth> {
        let health_cache = self.health_cache.read().await;
        health_cache.clone()
    }
    /// Get only healthy backends
    async fn get_healthy_backends(&self) -> Result<HashMap<String, Arc<dyn ExecutionBackend>>> {
        let backends = self.backends.read().await;
        let health_cache = self.health_cache.read().await;
        let mut healthy = HashMap::new();
        for (name, backend) in backends.iter() {
            // Check cached health status
            if let Some(health) = health_cache.get(name) {
                if health.healthy {
                    healthy.insert(name.clone(), backend.clone());
                }
            } else {
                // No cached health status, assume healthy but trigger update
                healthy.insert(name.clone(), backend.clone());
            }
        }
        Ok(healthy)
    }
    /// Get the backend managing a specific execution handle
    async fn get_backend_for_handle(
        &self,
        handle: &ExecutionHandle,
    ) -> Result<Arc<dyn ExecutionBackend>> {
        // First check our mapping
        let mut handle_map = self.handle_to_backend.write().await;
        if let Some(mapping) = handle_map.get_mut(&handle.id) {
            // Update last_accessed for LRU
            mapping.last_accessed = current_timestamp_or_zero() as u64;
            let backend_name = mapping.backend_name.clone();
            drop(handle_map);

            if let Some(backend) = self.get_backend(&backend_name).await {
                return Ok(backend);
            }
        } else {
            drop(handle_map);
        }

        // Fall back to checking the backend_name in the handle
        if let Some(backend) = self.get_backend(&handle.backend_name).await {
            return Ok(backend);
        }
        Err(anyhow::anyhow!(
            "No backend found for handle: {}",
            handle.id
        ))
    }
    /// Clean up completed executions across all backends
    pub async fn cleanup_executions(&self, older_than: std::time::Duration) -> Result<usize> {
        let backends = self.backends.read().await;
        let mut total_cleaned = 0;
        for (name, backend) in backends.iter() {
            match backend.cleanup_executions(older_than).await {
                Ok(count) => {
                    if count > 0 {
                        info!("Cleaned up {} executions from backend {}", count, name);
                    }
                    total_cleaned += count;
                }
                Err(e) => {
                    warn!("Failed to cleanup executions from backend {}: {}", name, e);
                }
            }
        }
        Ok(total_cleaned)
    }
    /// Shutdown all backends and clean up resources
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down execution registry");

        // Stop background cleanup task
        let mut handle = self.cleanup_task_handle.write().await;
        if let Some(task) = handle.take() {
            task.abort();
            info!("Stopped execution registry background cleanup task");
        }
        drop(handle);

        let backends = self.backends.read().await;
        for (name, backend) in backends.iter() {
            info!("Shutting down backend: {}", name);
            if let Err(e) = backend.shutdown().await {
                error!("Error shutting down backend {}: {}", name, e);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::time::Duration;
    use tokio::sync::Mutex;
    use crate::worker_trait::WorkResult;
    use crate::adapters::{
        ExecutionHandleType, ExecutionMetadata, ResourceInfo, ResourceRequirements,
    };

    /// Mock backend for testing registry operations
    struct TestBackend {
        name: String,
        initialized: Arc<AtomicBool>,
        shutdown_called: Arc<AtomicBool>,
        healthy: Arc<AtomicBool>,
        submit_count: Arc<AtomicU32>,
        submission_delay: Duration,
        should_fail_submit: Arc<AtomicBool>,
        should_fail_init: Arc<AtomicBool>,
        resource_info: Arc<Mutex<ResourceInfo>>,
        executions: Arc<RwLock<HashMap<String, ExecutionStatus>>>,
        can_handle_result: bool,
    }

    impl TestBackend {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                initialized: Arc::new(AtomicBool::new(false)),
                shutdown_called: Arc::new(AtomicBool::new(false)),
                healthy: Arc::new(AtomicBool::new(true)),
                submit_count: Arc::new(AtomicU32::new(0)),
                submission_delay: Duration::from_millis(0),
                should_fail_submit: Arc::new(AtomicBool::new(false)),
                should_fail_init: Arc::new(AtomicBool::new(false)),
                resource_info: Arc::new(Mutex::new(ResourceInfo {
                    total_cpu_cores: 8.0,
                    available_cpu_cores: 4.0,
                    total_memory_mb: 16384,
                    available_memory_mb: 8192,
                    total_disk_mb: 102400,
                    available_disk_mb: 51200,
                    running_executions: 0,
                    max_executions: 10,
                })),
                executions: Arc::new(RwLock::new(HashMap::new())),
                can_handle_result: true,
            }
        }

        fn with_healthy(mut self, healthy: bool) -> Self {
            self.healthy = Arc::new(AtomicBool::new(healthy));
            self
        }

        fn with_submit_failure(mut self, should_fail: bool) -> Self {
            self.should_fail_submit = Arc::new(AtomicBool::new(should_fail));
            self
        }

        fn with_init_failure(mut self, should_fail: bool) -> Self {
            self.should_fail_init = Arc::new(AtomicBool::new(should_fail));
            self
        }

        fn with_delay(mut self, delay: Duration) -> Self {
            self.submission_delay = delay;
            self
        }

        fn with_resources(mut self, resources: ResourceInfo) -> Self {
            self.resource_info = Arc::new(Mutex::new(resources));
            self
        }

        fn with_can_handle(mut self, can_handle: bool) -> Self {
            self.can_handle_result = can_handle;
            self
        }

        fn is_initialized(&self) -> bool {
            self.initialized.load(Ordering::SeqCst)
        }

        fn is_shutdown(&self) -> bool {
            self.shutdown_called.load(Ordering::SeqCst)
        }

        fn get_submit_count(&self) -> u32 {
            self.submit_count.load(Ordering::SeqCst)
        }

        fn set_healthy(&self, healthy: bool) {
            self.healthy.store(healthy, Ordering::SeqCst);
        }
    }

    #[async_trait]
    impl ExecutionBackend for TestBackend {
        fn backend_type(&self) -> &str {
            &self.name
        }

        async fn submit_job(&self, job: Job, _config: ExecutionConfig) -> Result<ExecutionHandle> {
            if self.submission_delay > Duration::from_millis(0) {
                tokio::time::sleep(self.submission_delay).await;
            }

            if self.should_fail_submit.load(Ordering::SeqCst) {
                return Err(anyhow::anyhow!("Simulated submission failure"));
            }

            let count = self.submit_count.fetch_add(1, Ordering::SeqCst);
            let handle_id = format!("{}-handle-{}", self.name, count);

            let handle = ExecutionHandle {
                id: handle_id.clone(),
                job_id: job.id.clone(),
                backend_name: self.name.clone(),
                handle_type: ExecutionHandleType::Process { pid: count as u32 },
                created_at: current_timestamp_or_zero() as u64,
            };

            let mut executions = self.executions.write().await;
            executions.insert(handle_id, ExecutionStatus::Running);

            Ok(handle)
        }

        async fn get_status(&self, handle: &ExecutionHandle) -> Result<ExecutionStatus> {
            let executions = self.executions.read().await;
            executions
                .get(&handle.id)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Handle not found"))
        }

        async fn cancel_execution(&self, handle: &ExecutionHandle) -> Result<()> {
            let mut executions = self.executions.write().await;
            executions.insert(handle.id.clone(), ExecutionStatus::Cancelled);
            Ok(())
        }

        async fn get_metadata(&self, handle: &ExecutionHandle) -> Result<ExecutionMetadata> {
            Ok(ExecutionMetadata {
                execution_id: handle.id.clone(),
                backend_type: self.name.clone(),
                started_at: handle.created_at,
                completed_at: None,
                node_id: Some("test-node".to_string()),
                custom: HashMap::new(),
            })
        }

        async fn wait_for_completion(
            &self,
            handle: &ExecutionHandle,
            timeout: Option<Duration>,
        ) -> Result<ExecutionStatus> {
            let timeout = timeout.unwrap_or(Duration::from_secs(5));
            let start = std::time::Instant::now();

            loop {
                let status = self.get_status(handle).await?;
                match status {
                    ExecutionStatus::Running | ExecutionStatus::Preparing => {
                        if start.elapsed() > timeout {
                            return Err(anyhow::anyhow!("Timeout waiting for completion"));
                        }
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                    _ => return Ok(status),
                }
            }
        }

        async fn health_check(&self) -> Result<BackendHealth> {
            let healthy = self.healthy.load(Ordering::SeqCst);
            let resources = self.resource_info.lock().await;
            Ok(BackendHealth {
                healthy,
                status_message: if healthy { "OK".to_string() } else { "Unhealthy".to_string() },
                resource_info: Some(resources.clone()),
                last_check: Some(current_timestamp_or_zero() as u64),
                details: HashMap::new(),
            })
        }

        async fn get_resource_info(&self) -> Result<ResourceInfo> {
            let resources = self.resource_info.lock().await;
            Ok(resources.clone())
        }

        async fn can_handle(&self, _job: &Job, _config: &ExecutionConfig) -> Result<bool> {
            Ok(self.can_handle_result)
        }

        async fn list_executions(&self) -> Result<Vec<ExecutionHandle>> {
            let executions = self.executions.read().await;
            Ok(executions
                .keys()
                .map(|id| ExecutionHandle {
                    id: id.clone(),
                    job_id: "test-job".to_string(),
                    backend_name: self.name.clone(),
                    handle_type: ExecutionHandleType::Process { pid: 1 },
                    created_at: 0,
                })
                .collect())
        }

        async fn cleanup_executions(&self, _older_than: Duration) -> Result<usize> {
            let mut executions = self.executions.write().await;
            let count = executions.len();
            executions.clear();
            Ok(count)
        }

        async fn initialize(&self) -> Result<()> {
            if self.should_fail_init.load(Ordering::SeqCst) {
                return Err(anyhow::anyhow!("Simulated initialization failure"));
            }
            self.initialized.store(true, Ordering::SeqCst);
            Ok(())
        }

        async fn shutdown(&self) -> Result<()> {
            self.shutdown_called.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    fn create_test_job() -> Job {
        Job {
            id: "test-job-1".to_string(),
            typ: "test".to_string(),
            payload: vec![1, 2, 3],
            created: current_timestamp_or_zero(),
            updated: current_timestamp_or_zero(),
        }
    }

    fn create_test_config() -> ExecutionConfig {
        ExecutionConfig {
            resources: ResourceRequirements {
                cpu_cores: 1.0,
                memory_mb: 1024,
                disk_mb: 5120,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn create_registry_config() -> RegistryConfig {
        RegistryConfig {
            placement_policy: PlacementPolicy::BestFit,
            enable_failover: true,
            health_check_interval: 30,
            max_submission_retries: 3,
            cleanup_config: CleanupConfig {
                enable_background_cleanup: false, // Disable for tests
                ..CleanupConfig::default()
            },
        }
    }

    // Registry Construction Tests

    #[tokio::test]
    async fn test_registry_new_creates_empty_registry() {
        let config = create_registry_config();
        let registry = ExecutionRegistry::new(config);

        let backends = registry.list_backends().await;
        assert!(backends.is_empty());
    }

    #[tokio::test]
    async fn test_registry_default_config() {
        let config = RegistryConfig::default();
        assert_eq!(config.placement_policy, PlacementPolicy::BestFit);
        assert!(config.enable_failover);
        assert_eq!(config.health_check_interval, 30);
        assert_eq!(config.max_submission_retries, 3);
    }

    // Backend Registration Tests

    #[tokio::test]
    async fn test_register_backend_success() {
        let registry = ExecutionRegistry::new(create_registry_config());
        let backend = Arc::new(TestBackend::new("test-backend"));
        let backend_clone = backend.clone();

        let result = registry
            .register_backend("test-backend".to_string(), backend_clone)
            .await;

        assert!(result.is_ok());
        assert!(backend.is_initialized());

        let backends = registry.list_backends().await;
        assert_eq!(backends.len(), 1);
        assert!(backends.contains(&"test-backend".to_string()));
    }

    #[tokio::test]
    async fn test_register_multiple_backends() {
        let registry = ExecutionRegistry::new(create_registry_config());

        let backend1 = Arc::new(TestBackend::new("backend1"));
        let backend2 = Arc::new(TestBackend::new("backend2"));
        let backend3 = Arc::new(TestBackend::new("backend3"));

        registry
            .register_backend("backend1".to_string(), backend1)
            .await
            .unwrap();
        registry
            .register_backend("backend2".to_string(), backend2)
            .await
            .unwrap();
        registry
            .register_backend("backend3".to_string(), backend3)
            .await
            .unwrap();

        let backends = registry.list_backends().await;
        assert_eq!(backends.len(), 3);
        assert!(backends.contains(&"backend1".to_string()));
        assert!(backends.contains(&"backend2".to_string()));
        assert!(backends.contains(&"backend3".to_string()));
    }

    #[tokio::test]
    async fn test_register_backend_initialization_failure() {
        let registry = ExecutionRegistry::new(create_registry_config());
        let backend = Arc::new(TestBackend::new("failing").with_init_failure(true));

        let result = registry
            .register_backend("failing".to_string(), backend)
            .await;

        assert!(result.is_err());

        let backends = registry.list_backends().await;
        assert!(backends.is_empty());
    }

    #[tokio::test]
    async fn test_register_unhealthy_backend_logs_warning() {
        let registry = ExecutionRegistry::new(create_registry_config());
        let backend = Arc::new(TestBackend::new("unhealthy").with_healthy(false));

        let result = registry
            .register_backend("unhealthy".to_string(), backend)
            .await;

        // Should succeed but log warning
        assert!(result.is_ok());

        let backends = registry.list_backends().await;
        assert_eq!(backends.len(), 1);
    }

    #[tokio::test]
    async fn test_register_duplicate_backend_replaces() {
        let registry = ExecutionRegistry::new(create_registry_config());

        let backend1 = Arc::new(TestBackend::new("backend"));
        registry
            .register_backend("backend".to_string(), backend1.clone())
            .await
            .unwrap();

        let backend2 = Arc::new(TestBackend::new("backend-v2"));
        registry
            .register_backend("backend".to_string(), backend2)
            .await
            .unwrap();

        let backends = registry.list_backends().await;
        assert_eq!(backends.len(), 1);
    }

    // Backend Unregistration Tests

    #[tokio::test]
    async fn test_unregister_backend_success() {
        let registry = ExecutionRegistry::new(create_registry_config());
        let backend = Arc::new(TestBackend::new("test-backend"));
        let backend_clone = backend.clone();

        registry
            .register_backend("test-backend".to_string(), backend_clone)
            .await
            .unwrap();

        let result = registry.unregister_backend("test-backend").await;
        assert!(result.is_ok());
        assert!(backend.is_shutdown());

        let backends = registry.list_backends().await;
        assert!(backends.is_empty());
    }

    #[tokio::test]
    async fn test_unregister_nonexistent_backend() {
        let registry = ExecutionRegistry::new(create_registry_config());

        let result = registry.unregister_backend("nonexistent").await;
        assert!(result.is_ok()); // Should not fail
    }

    #[tokio::test]
    async fn test_unregister_multiple_backends() {
        let registry = ExecutionRegistry::new(create_registry_config());

        let backend1 = Arc::new(TestBackend::new("backend1"));
        let backend2 = Arc::new(TestBackend::new("backend2"));
        let backend1_clone = backend1.clone();
        let backend2_clone = backend2.clone();

        registry
            .register_backend("backend1".to_string(), backend1_clone)
            .await
            .unwrap();
        registry
            .register_backend("backend2".to_string(), backend2_clone)
            .await
            .unwrap();

        registry.unregister_backend("backend1").await.unwrap();
        assert!(backend1.is_shutdown());
        assert!(!backend2.is_shutdown());

        let backends = registry.list_backends().await;
        assert_eq!(backends.len(), 1);
        assert!(backends.contains(&"backend2".to_string()));
    }

    // Backend Retrieval Tests

    #[tokio::test]
    async fn test_get_backend_success() {
        let registry = ExecutionRegistry::new(create_registry_config());
        let backend = Arc::new(TestBackend::new("test-backend"));

        registry
            .register_backend("test-backend".to_string(), backend)
            .await
            .unwrap();

        let retrieved = registry.get_backend("test-backend").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().backend_type(), "test-backend");
    }

    #[tokio::test]
    async fn test_get_backend_not_found() {
        let registry = ExecutionRegistry::new(create_registry_config());

        let retrieved = registry.get_backend("nonexistent").await;
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_list_backends_empty() {
        let registry = ExecutionRegistry::new(create_registry_config());

        let backends = registry.list_backends().await;
        assert!(backends.is_empty());
    }

    #[tokio::test]
    async fn test_list_backends_multiple() {
        let registry = ExecutionRegistry::new(create_registry_config());

        for i in 1..=5 {
            let backend = Arc::new(TestBackend::new(&format!("backend{}", i)));
            registry
                .register_backend(format!("backend{}", i), backend)
                .await
                .unwrap();
        }

        let backends = registry.list_backends().await;
        assert_eq!(backends.len(), 5);
    }

    // Job Submission Tests

    #[tokio::test]
    async fn test_submit_job_success() {
        let registry = ExecutionRegistry::new(create_registry_config());
        let backend = Arc::new(TestBackend::new("test-backend"));
        let backend_clone = backend.clone();

        registry
            .register_backend("test-backend".to_string(), backend_clone)
            .await
            .unwrap();

        let job = create_test_job();
        let config = create_test_config();

        let handle = registry.submit_job(job, config).await.unwrap();

        assert_eq!(handle.backend_name, "test-backend");
        assert_eq!(backend.get_submit_count(), 1);
    }

    #[tokio::test]
    async fn test_submit_job_no_backends() {
        let registry = ExecutionRegistry::new(create_registry_config());

        let job = create_test_job();
        let config = create_test_config();

        let result = registry.submit_job(job, config).await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No healthy execution backends"));
    }

    #[tokio::test]
    async fn test_submit_job_all_backends_unhealthy() {
        let registry = ExecutionRegistry::new(create_registry_config());
        let backend = Arc::new(TestBackend::new("unhealthy").with_healthy(false));

        registry
            .register_backend("unhealthy".to_string(), backend)
            .await
            .unwrap();

        // Update health status
        registry.update_health_status().await.unwrap();

        let job = create_test_job();
        let config = create_test_config();

        let result = registry.submit_job(job, config).await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No healthy execution backends"));
    }

    #[tokio::test]
    async fn test_submit_job_with_multiple_backends() {
        let registry = ExecutionRegistry::new(create_registry_config());

        let backend1 = Arc::new(TestBackend::new("backend1"));
        let backend2 = Arc::new(TestBackend::new("backend2"));

        registry
            .register_backend("backend1".to_string(), backend1.clone())
            .await
            .unwrap();
        registry
            .register_backend("backend2".to_string(), backend2.clone())
            .await
            .unwrap();

        let job = create_test_job();
        let config = create_test_config();

        let handle = registry.submit_job(job, config).await.unwrap();

        // Should submit to one of the backends
        assert!(handle.backend_name == "backend1" || handle.backend_name == "backend2");

        let total_submits = backend1.get_submit_count() + backend2.get_submit_count();
        assert_eq!(total_submits, 1);
    }

    #[tokio::test]
    async fn test_submit_job_retry_on_failure() {
        let mut config = create_registry_config();
        config.max_submission_retries = 2;
        let registry = ExecutionRegistry::new(config);

        let backend = Arc::new(TestBackend::new("backend").with_submit_failure(true));

        registry
            .register_backend("backend".to_string(), backend.clone())
            .await
            .unwrap();

        let job = create_test_job();
        let exec_config = create_test_config();

        let result = registry.submit_job(job, exec_config).await;

        assert!(result.is_err());
        // Should have attempted retries
        assert!(backend.get_submit_count() >= 2);
    }

    #[tokio::test]
    async fn test_submit_job_failover_to_alternative() {
        let mut config = create_registry_config();
        config.enable_failover = true;
        let registry = ExecutionRegistry::new(config);

        let backend1 = Arc::new(TestBackend::new("failing").with_submit_failure(true));
        let backend2 = Arc::new(TestBackend::new("working"));

        registry
            .register_backend("failing".to_string(), backend1.clone())
            .await
            .unwrap();
        registry
            .register_backend("working".to_string(), backend2.clone())
            .await
            .unwrap();

        let job = create_test_job();
        let exec_config = create_test_config();

        let handle = registry.submit_job(job, exec_config).await.unwrap();

        // Should have failed over to working backend
        assert_eq!(handle.backend_name, "working");
        assert_eq!(backend2.get_submit_count(), 1);
    }

    #[tokio::test]
    async fn test_submit_job_failover_disabled() {
        let mut config = create_registry_config();
        config.enable_failover = false;
        config.max_submission_retries = 1;
        let registry = ExecutionRegistry::new(config);

        let backend1 = Arc::new(TestBackend::new("failing").with_submit_failure(true));
        let backend2 = Arc::new(TestBackend::new("working"));

        registry
            .register_backend("failing".to_string(), backend1)
            .await
            .unwrap();
        registry
            .register_backend("working".to_string(), backend2.clone())
            .await
            .unwrap();

        let job = create_test_job();
        let exec_config = create_test_config();

        let result = registry.submit_job(job, exec_config).await;

        // Should fail without failover
        assert!(result.is_err());
        // Working backend should not be used
        assert_eq!(backend2.get_submit_count(), 0);
    }

    // Execution Status and Control Tests

    #[tokio::test]
    async fn test_get_status_success() {
        let registry = ExecutionRegistry::new(create_registry_config());
        let backend = Arc::new(TestBackend::new("backend"));

        registry
            .register_backend("backend".to_string(), backend)
            .await
            .unwrap();

        let job = create_test_job();
        let config = create_test_config();

        let handle = registry.submit_job(job, config).await.unwrap();
        let status = registry.get_status(&handle).await.unwrap();

        assert_eq!(status, ExecutionStatus::Running);
    }

    #[tokio::test]
    async fn test_get_status_backend_not_found() {
        let registry = ExecutionRegistry::new(create_registry_config());

        let handle = ExecutionHandle {
            id: "test-handle".to_string(),
            job_id: "test-job".to_string(),
            backend_name: "nonexistent".to_string(),
            handle_type: ExecutionHandleType::Process { pid: 1 },
            created_at: 0,
        };

        let result = registry.get_status(&handle).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No backend found"));
    }

    #[tokio::test]
    async fn test_cancel_execution_success() {
        let registry = ExecutionRegistry::new(create_registry_config());
        let backend = Arc::new(TestBackend::new("backend"));

        registry
            .register_backend("backend".to_string(), backend)
            .await
            .unwrap();

        let job = create_test_job();
        let config = create_test_config();

        let handle = registry.submit_job(job, config).await.unwrap();
        let result = registry.cancel_execution(&handle).await;

        assert!(result.is_ok());

        let status = registry.get_status(&handle).await.unwrap();
        assert_eq!(status, ExecutionStatus::Cancelled);
    }

    #[tokio::test]
    async fn test_wait_for_completion_success() {
        let registry = ExecutionRegistry::new(create_registry_config());
        let backend = Arc::new(TestBackend::new("backend"));

        registry
            .register_backend("backend".to_string(), backend.clone())
            .await
            .unwrap();

        let job = create_test_job();
        let config = create_test_config();

        let handle = registry.submit_job(job, config).await.unwrap();

        // Mark as completed in background
        let handle_clone = handle.clone();
        let backend_clone = backend.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let mut executions = backend_clone.executions.write().await;
            executions.insert(
                handle_clone.id.clone(),
                ExecutionStatus::Completed(WorkResult {
                    success: true,
                    error_message: None,
                    data: vec![],
                }),
            );
        });

        let status = registry
            .wait_for_completion(&handle, Some(Duration::from_secs(2)))
            .await
            .unwrap();

        match status {
            ExecutionStatus::Completed(_) => {}
            _ => panic!("Expected Completed status"),
        }
    }

    // Health Check Tests

    #[tokio::test]
    async fn test_update_health_status_all_healthy() {
        let registry = ExecutionRegistry::new(create_registry_config());

        let backend1 = Arc::new(TestBackend::new("backend1"));
        let backend2 = Arc::new(TestBackend::new("backend2"));

        registry
            .register_backend("backend1".to_string(), backend1)
            .await
            .unwrap();
        registry
            .register_backend("backend2".to_string(), backend2)
            .await
            .unwrap();

        registry.update_health_status().await.unwrap();

        let health = registry.get_health_status().await;
        assert_eq!(health.len(), 2);
        assert!(health.get("backend1").unwrap().healthy);
        assert!(health.get("backend2").unwrap().healthy);
    }

    #[tokio::test]
    async fn test_update_health_status_some_unhealthy() {
        let registry = ExecutionRegistry::new(create_registry_config());

        let backend1 = Arc::new(TestBackend::new("healthy"));
        let backend2 = Arc::new(TestBackend::new("unhealthy").with_healthy(false));

        registry
            .register_backend("healthy".to_string(), backend1)
            .await
            .unwrap();
        registry
            .register_backend("unhealthy".to_string(), backend2)
            .await
            .unwrap();

        registry.update_health_status().await.unwrap();

        let health = registry.get_health_status().await;
        assert!(health.get("healthy").unwrap().healthy);
        assert!(!health.get("unhealthy").unwrap().healthy);
    }

    #[tokio::test]
    async fn test_get_health_status_empty_registry() {
        let registry = ExecutionRegistry::new(create_registry_config());

        let health = registry.get_health_status().await;
        assert!(health.is_empty());
    }

    #[tokio::test]
    async fn test_health_cache_updated_on_registration() {
        let registry = ExecutionRegistry::new(create_registry_config());
        let backend = Arc::new(TestBackend::new("backend"));

        registry
            .register_backend("backend".to_string(), backend)
            .await
            .unwrap();

        let health = registry.get_health_status().await;
        assert_eq!(health.len(), 1);
        assert!(health.contains_key("backend"));
    }

    // Handle Mapping Tests

    #[tokio::test]
    async fn test_handle_mapping_recorded_on_submit() {
        let registry = ExecutionRegistry::new(create_registry_config());
        let backend = Arc::new(TestBackend::new("backend"));

        registry
            .register_backend("backend".to_string(), backend)
            .await
            .unwrap();

        let job = create_test_job();
        let config = create_test_config();

        let handle = registry.submit_job(job, config).await.unwrap();

        // Verify handle can be used to get status (mapping exists)
        let status = registry.get_status(&handle).await;
        assert!(status.is_ok());
    }

    #[tokio::test]
    async fn test_cleanup_orphaned_handles() {
        let registry = ExecutionRegistry::new(create_registry_config());
        let backend = Arc::new(TestBackend::new("backend"));

        registry
            .register_backend("backend".to_string(), backend)
            .await
            .unwrap();

        let job = create_test_job();
        let config = create_test_config();

        let handle = registry.submit_job(job, config).await.unwrap();

        // Unregister the backend
        registry.unregister_backend("backend").await.unwrap();

        // Clean up orphaned handles
        let removed = registry.cleanup_orphaned_handles().await.unwrap();
        assert_eq!(removed, 1);

        // Verify handle is orphaned
        let result = registry.get_status(&handle).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cleanup_orphaned_handles_no_orphans() {
        let registry = ExecutionRegistry::new(create_registry_config());
        let backend = Arc::new(TestBackend::new("backend"));

        registry
            .register_backend("backend".to_string(), backend)
            .await
            .unwrap();

        let job = create_test_job();
        let config = create_test_config();

        registry.submit_job(job, config).await.unwrap();

        // No backends unregistered, so no orphans
        let removed = registry.cleanup_orphaned_handles().await.unwrap();
        assert_eq!(removed, 0);
    }

    // Backend Cleanup Tests

    #[tokio::test]
    async fn test_cleanup_executions_across_backends() {
        let registry = ExecutionRegistry::new(create_registry_config());

        let backend1 = Arc::new(TestBackend::new("backend1"));
        let backend2 = Arc::new(TestBackend::new("backend2"));

        registry
            .register_backend("backend1".to_string(), backend1.clone())
            .await
            .unwrap();
        registry
            .register_backend("backend2".to_string(), backend2.clone())
            .await
            .unwrap();

        // Create some executions
        let job = create_test_job();
        let config = create_test_config();
        registry.submit_job(job.clone(), config.clone()).await.unwrap();
        registry.submit_job(job, config).await.unwrap();

        let cleaned = registry
            .cleanup_executions(Duration::from_secs(0))
            .await
            .unwrap();

        assert_eq!(cleaned, 2);
    }

    #[tokio::test]
    async fn test_cleanup_executions_empty_registry() {
        let registry = ExecutionRegistry::new(create_registry_config());

        let cleaned = registry
            .cleanup_executions(Duration::from_secs(0))
            .await
            .unwrap();

        assert_eq!(cleaned, 0);
    }

    // Shutdown Tests

    #[tokio::test]
    async fn test_shutdown_calls_backend_shutdown() {
        let registry = ExecutionRegistry::new(create_registry_config());
        let backend = Arc::new(TestBackend::new("backend"));
        let backend_clone = backend.clone();

        registry
            .register_backend("backend".to_string(), backend_clone)
            .await
            .unwrap();

        registry.shutdown().await.unwrap();

        assert!(backend.is_shutdown());
    }

    #[tokio::test]
    async fn test_shutdown_all_backends() {
        let registry = ExecutionRegistry::new(create_registry_config());

        let backend1 = Arc::new(TestBackend::new("backend1"));
        let backend2 = Arc::new(TestBackend::new("backend2"));
        let backend3 = Arc::new(TestBackend::new("backend3"));

        registry
            .register_backend("backend1".to_string(), backend1.clone())
            .await
            .unwrap();
        registry
            .register_backend("backend2".to_string(), backend2.clone())
            .await
            .unwrap();
        registry
            .register_backend("backend3".to_string(), backend3.clone())
            .await
            .unwrap();

        registry.shutdown().await.unwrap();

        assert!(backend1.is_shutdown());
        assert!(backend2.is_shutdown());
        assert!(backend3.is_shutdown());
    }

    #[tokio::test]
    async fn test_shutdown_empty_registry() {
        let registry = ExecutionRegistry::new(create_registry_config());

        let result = registry.shutdown().await;
        assert!(result.is_ok());
    }

    // Cleanup Metrics Tests

    #[tokio::test]
    async fn test_get_cleanup_metrics_initial() {
        let registry = ExecutionRegistry::new(create_registry_config());

        let metrics = registry.get_cleanup_metrics().await;
        assert_eq!(metrics.total_cleaned, 0);
        assert_eq!(metrics.ttl_expired, 0);
        assert_eq!(metrics.lru_evicted, 0);
    }

    // Concurrent Access Tests

    #[tokio::test]
    async fn test_concurrent_backend_registration() {
        let registry = Arc::new(ExecutionRegistry::new(create_registry_config()));

        let mut handles = vec![];
        for i in 0..10 {
            let registry_clone = registry.clone();
            let handle = tokio::spawn(async move {
                let backend = Arc::new(TestBackend::new(&format!("backend{}", i)));
                registry_clone
                    .register_backend(format!("backend{}", i), backend)
                    .await
                    .unwrap();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        let backends = registry.list_backends().await;
        assert_eq!(backends.len(), 10);
    }

    #[tokio::test]
    async fn test_concurrent_job_submissions() {
        let registry = Arc::new(ExecutionRegistry::new(create_registry_config()));
        let backend = Arc::new(TestBackend::new("backend"));

        registry
            .register_backend("backend".to_string(), backend.clone())
            .await
            .unwrap();

        let mut handles = vec![];
        for i in 0..20 {
            let registry_clone = registry.clone();
            let handle = tokio::spawn(async move {
                let mut job = create_test_job();
                job.id = format!("job-{}", i);
                let config = create_test_config();
                registry_clone.submit_job(job, config).await
            });
            handles.push(handle);
        }

        let mut success_count = 0;
        for handle in handles {
            if handle.await.unwrap().is_ok() {
                success_count += 1;
            }
        }

        assert_eq!(success_count, 20);
        assert_eq!(backend.get_submit_count(), 20);
    }

    #[tokio::test]
    async fn test_concurrent_health_checks() {
        let registry = Arc::new(ExecutionRegistry::new(create_registry_config()));

        let backend1 = Arc::new(TestBackend::new("backend1"));
        let backend2 = Arc::new(TestBackend::new("backend2"));

        registry
            .register_backend("backend1".to_string(), backend1)
            .await
            .unwrap();
        registry
            .register_backend("backend2".to_string(), backend2)
            .await
            .unwrap();

        let mut handles = vec![];
        for _ in 0..10 {
            let registry_clone = registry.clone();
            let handle = tokio::spawn(async move {
                registry_clone.update_health_status().await
            });
            handles.push(handle);
        }

        for handle in handles {
            assert!(handle.await.unwrap().is_ok());
        }
    }

    #[tokio::test]
    async fn test_concurrent_registration_and_retrieval() {
        let registry = Arc::new(ExecutionRegistry::new(create_registry_config()));

        let mut handles = vec![];

        // Registration tasks
        for i in 0..5 {
            let registry_clone = registry.clone();
            let handle = tokio::spawn(async move {
                let backend = Arc::new(TestBackend::new(&format!("backend{}", i)));
                registry_clone
                    .register_backend(format!("backend{}", i), backend)
                    .await
            });
            handles.push(handle);
        }

        // Retrieval tasks
        for i in 0..5 {
            let registry_clone = registry.clone();
            let handle = tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(50)).await;
                registry_clone.get_backend(&format!("backend{}", i)).await
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        let backends = registry.list_backends().await;
        assert_eq!(backends.len(), 5);
    }

    // Edge Cases

    #[tokio::test]
    async fn test_get_backend_for_handle_uses_mapping() {
        let registry = ExecutionRegistry::new(create_registry_config());
        let backend = Arc::new(TestBackend::new("backend"));

        registry
            .register_backend("backend".to_string(), backend)
            .await
            .unwrap();

        let job = create_test_job();
        let config = create_test_config();

        let handle = registry.submit_job(job, config).await.unwrap();

        // Should find backend through mapping
        let status = registry.get_status(&handle).await;
        assert!(status.is_ok());
    }

    #[tokio::test]
    async fn test_get_backend_for_handle_falls_back_to_handle_name() {
        let registry = ExecutionRegistry::new(create_registry_config());
        let backend = Arc::new(TestBackend::new("backend"));

        registry
            .register_backend("backend".to_string(), backend)
            .await
            .unwrap();

        // Create handle without going through submit (no mapping)
        let handle = ExecutionHandle {
            id: "manual-handle".to_string(),
            job_id: "test-job".to_string(),
            backend_name: "backend".to_string(),
            handle_type: ExecutionHandleType::Process { pid: 1 },
            created_at: 0,
        };

        // Should find backend through handle's backend_name field
        let result = registry.get_status(&handle).await;
        // Will fail because execution doesn't exist, but should find backend
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Handle not found"));
    }

    #[tokio::test]
    async fn test_submit_job_incompatible_backend_skipped() {
        let registry = ExecutionRegistry::new(create_registry_config());

        let backend_incompatible = Arc::new(TestBackend::new("incompatible").with_can_handle(false));
        let backend_compatible = Arc::new(TestBackend::new("compatible"));

        registry
            .register_backend("incompatible".to_string(), backend_incompatible.clone())
            .await
            .unwrap();
        registry
            .register_backend("compatible".to_string(), backend_compatible.clone())
            .await
            .unwrap();

        let job = create_test_job();
        let config = create_test_config();

        let handle = registry.submit_job(job, config).await.unwrap();

        assert_eq!(handle.backend_name, "compatible");
        assert_eq!(backend_incompatible.get_submit_count(), 0);
        assert_eq!(backend_compatible.get_submit_count(), 1);
    }

    #[tokio::test]
    async fn test_registry_config_clone() {
        let config1 = RegistryConfig::default();
        let config2 = config1.clone();

        assert_eq!(config1.placement_policy, config2.placement_policy);
        assert_eq!(config1.enable_failover, config2.enable_failover);
        assert_eq!(config1.health_check_interval, config2.health_check_interval);
        assert_eq!(config1.max_submission_retries, config2.max_submission_retries);
    }
}
