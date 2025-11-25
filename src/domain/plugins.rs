//! Plugin System Module
//!
//! This module defines the plugin architecture that allows Blixard to be extended
//! with custom functionality without modifying core code.

use crate::domain::types::{Job, JobStatus, Worker};
use crate::domain::errors::DomainResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Unique plugin identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Plugin version
    pub version: String,
    /// Plugin description
    pub description: String,
    /// Plugin author
    pub author: String,
    /// Plugin capabilities
    pub capabilities: Vec<String>,
}

/// Base trait for all plugins
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> &PluginMetadata;

    /// Initialize the plugin
    async fn initialize(&mut self, config: serde_json::Value) -> DomainResult<()>;

    /// Shutdown the plugin gracefully
    async fn shutdown(&mut self) -> DomainResult<()>;

    /// Get plugin health status
    async fn health_check(&self) -> DomainResult<PluginHealth>;

    /// Cast to Any for downcasting to specific plugin types
    fn as_any(&self) -> &dyn Any;
}

/// Plugin health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginHealth {
    pub healthy: bool,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

// === Job Processing Plugins ===

/// Result of job processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobProcessingResult {
    pub success: bool,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub metrics: HashMap<String, f64>,
}

/// Plugin for processing jobs
#[async_trait]
pub trait JobProcessor: Plugin {
    /// Check if this processor can handle the given job
    async fn can_process(&self, job: &Job) -> bool;

    /// Process a job
    async fn process(&self, job: &Job) -> DomainResult<JobProcessingResult>;

    /// Get the processor type identifier
    fn processor_type(&self) -> &str;

    /// Get resource requirements for a job
    async fn get_resource_requirements(&self, job: &Job) -> DomainResult<ResourceRequirements>;

    /// Validate job before processing
    async fn validate_job(&self, job: &Job) -> DomainResult<()>;

    /// Handle job cancellation
    async fn cancel(&self, job_id: &str) -> DomainResult<()>;
}

// === Resource Allocation Plugins ===

/// Resource requirements for job execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub cpu_cores: Option<f32>,
    pub memory_mb: Option<u32>,
    pub disk_mb: Option<u32>,
    pub gpu_count: Option<u32>,
    pub custom: HashMap<String, serde_json::Value>,
}

/// Allocated resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocation {
    pub allocation_id: String,
    pub resources: ResourceRequirements,
    pub node_id: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Plugin for managing resource allocation
#[async_trait]
pub trait ResourceAllocator: Plugin {
    /// Allocate resources for a job
    async fn allocate(
        &self,
        requirements: &ResourceRequirements,
        job: &Job,
    ) -> DomainResult<ResourceAllocation>;

    /// Release allocated resources
    async fn release(&self, allocation: &ResourceAllocation) -> DomainResult<()>;

    /// Check if resources are available
    async fn check_availability(
        &self,
        requirements: &ResourceRequirements,
    ) -> DomainResult<bool>;

    /// Get current resource usage
    async fn get_usage(&self) -> DomainResult<ResourceUsage>;

    /// Reserve resources (for planning)
    async fn reserve(
        &self,
        requirements: &ResourceRequirements,
        duration_secs: u64,
    ) -> DomainResult<String>;

    /// Cancel a reservation
    async fn cancel_reservation(&self, reservation_id: &str) -> DomainResult<()>;
}

/// Resource usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub total: ResourceRequirements,
    pub used: ResourceRequirements,
    pub available: ResourceRequirements,
    pub reserved: ResourceRequirements,
}

// === Scheduling Plugins ===

/// Plugin for job scheduling
#[async_trait]
pub trait JobScheduler: Plugin {
    /// Schedule a job for execution
    async fn schedule(&self, job: &Job) -> DomainResult<ScheduleDecision>;

    /// Get next job to execute for a worker
    async fn get_next_job(&self, worker: &Worker) -> DomainResult<Option<Job>>;

    /// Update scheduling based on job completion
    async fn job_completed(&self, job: &Job, result: &JobProcessingResult) -> DomainResult<()>;

    /// Reschedule a failed job
    async fn reschedule_failed(&self, job: &Job, error: &str) -> DomainResult<ScheduleDecision>;

    /// Get scheduling queue statistics
    async fn get_queue_stats(&self) -> DomainResult<SchedulingStats>;
}

/// Scheduling decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleDecision {
    pub action: ScheduleAction,
    pub delay_secs: Option<u64>,
    pub target_worker: Option<String>,
    pub priority: i32,
    pub reason: String,
}

/// Scheduling action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScheduleAction {
    Execute,
    Queue,
    Delay,
    Reject,
    Retry,
}

/// Scheduling statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingStats {
    pub queued_jobs: usize,
    pub running_jobs: usize,
    pub avg_wait_time_secs: f64,
    pub avg_execution_time_secs: f64,
}

// === Monitoring Plugins ===

/// Plugin for monitoring and metrics
#[async_trait]
pub trait MonitoringPlugin: Plugin {
    /// Record a metric
    async fn record_metric(&self, name: &str, value: f64, tags: HashMap<String, String>) -> DomainResult<()>;

    /// Record an event
    async fn record_event(&self, event: &MonitoringEvent) -> DomainResult<()>;

    /// Start a span for tracing
    async fn start_span(&self, name: &str, attributes: HashMap<String, String>) -> DomainResult<String>;

    /// End a span
    async fn end_span(&self, span_id: &str) -> DomainResult<()>;

    /// Query metrics
    async fn query_metrics(
        &self,
        query: &str,
        start_time: i64,
        end_time: i64,
    ) -> DomainResult<Vec<MetricPoint>>;
}

/// Monitoring event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringEvent {
    pub timestamp: i64,
    pub level: EventLevel,
    pub message: String,
    pub attributes: HashMap<String, serde_json::Value>,
}

/// Event level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventLevel {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

/// Metric data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricPoint {
    pub timestamp: i64,
    pub value: f64,
    pub tags: HashMap<String, String>,
}

// === Hook Plugins ===

/// Plugin for lifecycle hooks
#[async_trait]
pub trait LifecycleHook: Plugin {
    /// Called before a job is submitted
    async fn pre_submit(&self, job: &mut Job) -> DomainResult<()>;

    /// Called after a job is submitted
    async fn post_submit(&self, job: &Job) -> DomainResult<()>;

    /// Called before a job is claimed
    async fn pre_claim(&self, job: &Job, worker: &Worker) -> DomainResult<bool>;

    /// Called after a job is claimed
    async fn post_claim(&self, job: &Job, worker: &Worker) -> DomainResult<()>;

    /// Called before job execution
    async fn pre_execute(&self, job: &Job) -> DomainResult<()>;

    /// Called after job execution
    async fn post_execute(&self, job: &Job, result: &JobProcessingResult) -> DomainResult<()>;

    /// Called on job status change
    async fn on_status_change(
        &self,
        job: &Job,
        old_status: JobStatus,
        new_status: JobStatus,
    ) -> DomainResult<()>;
}

// === Plugin Registry ===

/// Registry for managing plugins
pub struct PluginRegistry {
    plugins: HashMap<String, Arc<dyn Plugin>>,
    processors: HashMap<String, Arc<dyn JobProcessor>>,
    allocators: HashMap<String, Arc<dyn ResourceAllocator>>,
    schedulers: HashMap<String, Arc<dyn JobScheduler>>,
    monitors: HashMap<String, Arc<dyn MonitoringPlugin>>,
    hooks: Vec<Arc<dyn LifecycleHook>>,
}

impl PluginRegistry {
    /// Create a new plugin registry
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            processors: HashMap::new(),
            allocators: HashMap::new(),
            schedulers: HashMap::new(),
            monitors: HashMap::new(),
            hooks: Vec::new(),
        }
    }

    /// Register a job processor
    pub fn register_processor(&mut self, processor: Arc<dyn JobProcessor>) {
        let metadata = processor.metadata();
        self.processors.insert(metadata.id.clone(), processor.clone());
        self.plugins.insert(metadata.id.clone(), processor);
    }

    /// Register a resource allocator
    pub fn register_allocator(&mut self, allocator: Arc<dyn ResourceAllocator>) {
        let metadata = allocator.metadata();
        self.allocators.insert(metadata.id.clone(), allocator.clone());
        self.plugins.insert(metadata.id.clone(), allocator);
    }

    /// Register a scheduler
    pub fn register_scheduler(&mut self, scheduler: Arc<dyn JobScheduler>) {
        let metadata = scheduler.metadata();
        self.schedulers.insert(metadata.id.clone(), scheduler.clone());
        self.plugins.insert(metadata.id.clone(), scheduler);
    }

    /// Register a monitoring plugin
    pub fn register_monitor(&mut self, monitor: Arc<dyn MonitoringPlugin>) {
        let metadata = monitor.metadata();
        self.monitors.insert(metadata.id.clone(), monitor.clone());
        self.plugins.insert(metadata.id.clone(), monitor);
    }

    /// Register a lifecycle hook
    pub fn register_hook(&mut self, hook: Arc<dyn LifecycleHook>) {
        let metadata = hook.metadata();
        self.hooks.push(hook.clone());
        self.plugins.insert(metadata.id.clone(), hook);
    }

    /// Get a processor by ID
    pub fn get_processor(&self, id: &str) -> Option<Arc<dyn JobProcessor>> {
        self.processors.get(id).cloned()
    }

    /// Get an allocator by ID
    pub fn get_allocator(&self, id: &str) -> Option<Arc<dyn ResourceAllocator>> {
        self.allocators.get(id).cloned()
    }

    /// Get a scheduler by ID
    pub fn get_scheduler(&self, id: &str) -> Option<Arc<dyn JobScheduler>> {
        self.schedulers.get(id).cloned()
    }

    /// Get a monitor by ID
    pub fn get_monitor(&self, id: &str) -> Option<Arc<dyn MonitoringPlugin>> {
        self.monitors.get(id).cloned()
    }

    /// Get all hooks
    pub fn get_hooks(&self) -> &[Arc<dyn LifecycleHook>] {
        &self.hooks
    }

    /// Find a processor that can handle a job
    pub async fn find_processor(&self, job: &Job) -> Option<Arc<dyn JobProcessor>> {
        for processor in self.processors.values() {
            if processor.can_process(job).await {
                return Some(processor.clone());
            }
        }
        None
    }

    /// Get all registered plugins
    pub fn list_plugins(&self) -> Vec<PluginMetadata> {
        self.plugins
            .values()
            .map(|p| p.metadata().clone())
            .collect()
    }

    /// Initialize all plugins
    pub async fn initialize_all(&mut self, configs: HashMap<String, serde_json::Value>) -> DomainResult<()> {
        for (id, _plugin) in &mut self.plugins {
            let config = configs
                .get(id)
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));

            // Need to get mutable access through Arc
            // This would require Arc<Mutex<dyn Plugin>> in real implementation
            // For now, we'll assume plugins are internally mutable

            tracing::info!(plugin_id = %id, "Initializing plugin");
        }
        Ok(())
    }

    /// Shutdown all plugins
    pub async fn shutdown_all(&mut self) -> DomainResult<()> {
        for (id, _plugin) in &self.plugins {
            tracing::info!(plugin_id = %id, "Shutting down plugin");
            // Similar to initialize_all, would need internal mutability
        }
        Ok(())
    }

    /// Health check all plugins
    pub async fn health_check_all(&self) -> HashMap<String, PluginHealth> {
        let mut results = HashMap::new();

        for (id, plugin) in &self.plugins {
            match plugin.health_check().await {
                Ok(health) => {
                    results.insert(id.clone(), health);
                }
                Err(e) => {
                    results.insert(
                        id.clone(),
                        PluginHealth {
                            healthy: false,
                            message: format!("Health check failed: {}", e),
                            details: None,
                        },
                    );
                }
            }
        }

        results
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Plugin builder for easy plugin creation
pub struct PluginBuilder {
    metadata: PluginMetadata,
}

impl PluginBuilder {
    /// Create a new plugin builder
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            metadata: PluginMetadata {
                id: id.into(),
                name: String::new(),
                version: "0.1.0".to_string(),
                description: String::new(),
                author: String::new(),
                capabilities: Vec::new(),
            },
        }
    }

    /// Set plugin name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.metadata.name = name.into();
        self
    }

    /// Set plugin version
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.metadata.version = version.into();
        self
    }

    /// Set plugin description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.metadata.description = description.into();
        self
    }

    /// Set plugin author
    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.metadata.author = author.into();
        self
    }

    /// Add a capability
    pub fn capability(mut self, capability: impl Into<String>) -> Self {
        self.metadata.capabilities.push(capability.into());
        self
    }

    /// Build the plugin metadata
    pub fn build(self) -> PluginMetadata {
        self.metadata
    }
}