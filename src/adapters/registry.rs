//! Registry for managing multiple execution backends
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use crate::domain::types::Job;
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
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
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
            // Get healthy backends
            let healthy_backends = self.get_healthy_backends().await?;
            if healthy_backends.is_empty() {
                return Err(anyhow::anyhow!("No healthy execution backends available"));
            }
            // Place the job
            let decision = self
                .placement_strategy
                .place_job(&job, &config, &healthy_backends)
                .await?;
            info!(
                "Placing job {} on backend {} (score: {:.2})",
                job.id, decision.backend_name, decision.score
            );
            // Get the selected backend
            let backend = healthy_backends
                .get(&decision.backend_name)
                .ok_or_else(|| anyhow::anyhow!("Selected backend not found"))?;
            // Try to submit the job
            match backend.submit_job(job.clone(), config.clone()).await {
                Ok(handle) => {
                    // Record the mapping with metadata
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    let mapping = HandleMapping {
                        backend_name: decision.backend_name.clone(),
                        created_at: now,
                        last_accessed: now,
                    };
                    let mut handle_map = self.handle_to_backend.write().await;
                    handle_map.insert(handle.id.clone(), mapping);
                    info!(
                        "Successfully submitted job {} to backend {}",
                        job.id, decision.backend_name
                    );
                    return Ok(handle);
                }
                Err(e) => {
                    warn!(
                        "Failed to submit job {} to backend {}: {}",
                        job.id, decision.backend_name, e
                    );
                    retries += 1;
                    if retries >= max_retries {
                        // Try failover if enabled
                        if self.config.enable_failover && !decision.alternatives.is_empty() {
                            info!(
                                "Attempting failover for job {} to alternative backends",
                                job.id
                            );
                            for alt in &decision.alternatives {
                                if let Some(alt_backend) = healthy_backends.get(alt) {
                                    match alt_backend.submit_job(job.clone(), config.clone()).await
                                    {
                                        Ok(handle) => {
                                            let now = std::time::SystemTime::now()
                                                .duration_since(std::time::UNIX_EPOCH)
                                                .map(|d| d.as_secs())
                                                .unwrap_or(0);
                                            let mapping = HandleMapping {
                                                backend_name: alt.clone(),
                                                created_at: now,
                                                last_accessed: now,
                                            };
                                            let mut handle_map =
                                                self.handle_to_backend.write().await;
                                            handle_map.insert(handle.id.clone(), mapping);
                                            info!(
                                                "Failover successful: job {} submitted to {}",
                                                job.id, alt
                                            );
                                            return Ok(handle);
                                        }
                                        Err(e) => {
                                            warn!("Failover to {} failed: {}", alt, e);
                                        }
                                    }
                                }
                            }
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
                            last_check: Some(
                                std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .expect("System time is before UNIX epoch")
                                    .as_secs(),
                            ),
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
            mapping.last_accessed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
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
