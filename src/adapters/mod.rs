//! Execution backend adapters for decoupled orchestration
//!
//! This module provides a trait-based abstraction layer that allows the control plane
//! to orchestrate various execution backends (VMs, containers, processes, etc.) without
//! being tightly coupled to any specific implementation.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

pub mod handle;
pub mod placement;
pub mod registry;

// Concrete adapter implementations
pub mod flawless_adapter;
pub mod local_adapter;
pub mod mock_adapter;
pub mod vm_adapter;

pub use handle::{ExecutionHandle, ExecutionHandleType};
pub use placement::{JobPlacement, PlacementPolicy, PlacementStrategy};
pub use registry::{ExecutionRegistry, RegistryConfig};

use crate::domain::types::Job;
use crate::worker_trait::WorkResult;

/// Status of an execution instance
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutionStatus {
    /// Execution is being prepared
    Preparing,
    /// Execution is running
    Running,
    /// Execution completed successfully
    Completed(WorkResult),
    /// Execution failed
    Failed(String),
    /// Execution was cancelled
    Cancelled,
    /// Status unknown or backend unavailable
    Unknown,
}

/// Resource requirements for job execution
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceRequirements {
    /// CPU cores required (fractional for shared CPU)
    pub cpu_cores: f32,
    /// Memory required in MB
    pub memory_mb: u32,
    /// Disk space required in MB
    pub disk_mb: u32,
    /// GPU devices required
    pub gpu_count: u32,
    /// Network bandwidth required in Mbps
    pub network_mbps: Option<u32>,
    /// Additional backend-specific requirements
    pub custom: HashMap<String, String>,
}

/// Information about available resources in a backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceInfo {
    /// Total CPU cores available
    pub total_cpu_cores: f32,
    /// Available CPU cores
    pub available_cpu_cores: f32,
    /// Total memory in MB
    pub total_memory_mb: u32,
    /// Available memory in MB
    pub available_memory_mb: u32,
    /// Total disk space in MB
    pub total_disk_mb: u32,
    /// Available disk space in MB
    pub available_disk_mb: u32,
    /// Number of running executions
    pub running_executions: usize,
    /// Maximum concurrent executions
    pub max_executions: usize,
}

/// Health status of an execution backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendHealth {
    /// Whether the backend is healthy
    pub healthy: bool,
    /// Human-readable status message
    pub status_message: String,
    /// Resource utilization information
    pub resource_info: Option<ResourceInfo>,
    /// Last successful health check timestamp
    pub last_check: Option<u64>,
    /// Backend-specific health details
    pub details: HashMap<String, String>,
}

/// Configuration for job execution
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionConfig {
    /// Resource requirements
    pub resources: ResourceRequirements,
    /// Execution timeout
    pub timeout: Option<Duration>,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Isolation level (backend-specific interpretation)
    pub isolation_level: Option<String>,
    /// Placement hints for the placement strategy
    pub placement_hints: HashMap<String, String>,
    /// Backend-specific configuration
    pub backend_config: HashMap<String, serde_json::Value>,
}

/// Metadata about an execution instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    /// Unique execution ID
    pub execution_id: String,
    /// Backend type that's handling this execution
    pub backend_type: String,
    /// When the execution started
    pub started_at: u64,
    /// When the execution completed (if applicable)
    pub completed_at: Option<u64>,
    /// Node or host where execution is running
    pub node_id: Option<String>,
    /// Additional backend-specific metadata
    pub custom: HashMap<String, serde_json::Value>,
}

/// Core trait for execution backends
///
/// This trait defines the interface that all execution backends must implement.
/// It provides methods for submitting jobs, monitoring their status, and managing
/// the execution lifecycle.
#[async_trait]
pub trait ExecutionBackend: Send + Sync {
    /// Get the type identifier for this backend
    fn backend_type(&self) -> &str;

    /// Submit a job for execution
    ///
    /// Returns an ExecutionHandle that can be used to track and control the execution.
    async fn submit_job(&self, job: Job, config: ExecutionConfig) -> Result<ExecutionHandle>;

    /// Get the current status of an execution
    async fn get_status(&self, handle: &ExecutionHandle) -> Result<ExecutionStatus>;

    /// Cancel a running execution
    ///
    /// This should attempt to gracefully stop the execution if possible.
    async fn cancel_execution(&self, handle: &ExecutionHandle) -> Result<()>;

    /// Get detailed metadata about an execution
    async fn get_metadata(&self, handle: &ExecutionHandle) -> Result<ExecutionMetadata>;

    /// Wait for an execution to complete
    ///
    /// This method should block until the execution completes or the timeout is reached.
    async fn wait_for_completion(
        &self,
        handle: &ExecutionHandle,
        timeout: Option<Duration>,
    ) -> Result<ExecutionStatus>;

    /// Check the health of the backend
    async fn health_check(&self) -> Result<BackendHealth>;

    /// Get current resource information
    async fn get_resource_info(&self) -> Result<ResourceInfo>;

    /// Check if the backend can handle a job with given requirements
    async fn can_handle(&self, job: &Job, config: &ExecutionConfig) -> Result<bool>;

    /// List all active executions managed by this backend
    async fn list_executions(&self) -> Result<Vec<ExecutionHandle>>;

    /// Clean up completed or failed executions
    ///
    /// This should remove any resources associated with completed executions.
    async fn cleanup_executions(&self, older_than: Duration) -> Result<usize>;

    /// Initialize the backend
    ///
    /// This is called once when the backend is registered.
    async fn initialize(&self) -> Result<()> {
        Ok(())
    }

    /// Shutdown the backend
    ///
    /// This should gracefully stop all executions and clean up resources.
    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

/// Factory for creating execution backends
pub trait BackendFactory: Send + Sync {
    /// Create a new instance of the backend
    async fn create(&self, config: HashMap<String, String>) -> Result<Arc<dyn ExecutionBackend>>;

    /// Get the type identifier for backends created by this factory
    fn backend_type(&self) -> &str;
}

