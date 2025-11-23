// Worker Backend Trait
//
// Defines the interface for pluggable worker implementations.
// This allows different execution backends (WASM, MicroVMs, containers, etc.)
// to be swapped without changing the worker loop.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::Job;

/// Result of job execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkResult {
    /// Whether the job completed successfully
    pub success: bool,

    /// Optional output data from the job
    pub output: Option<serde_json::Value>,

    /// Optional error message if job failed
    pub error: Option<String>,
}

impl WorkResult {
    pub fn success() -> Self {
        Self {
            success: true,
            output: None,
            error: None,
        }
    }

    pub fn success_with_output(output: serde_json::Value) -> Self {
        Self {
            success: true,
            output: Some(output),
            error: None,
        }
    }

    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            success: false,
            output: None,
            error: Some(error.into()),
        }
    }
}

/// Trait for worker backend implementations
///
/// Implement this trait to create custom job execution backends.
///
/// # Example
/// ```no_run
/// use async_trait::async_trait;
/// use mvm_ci::{WorkerBackend, Job, WorkResult};
///
/// struct MyWorker;
///
/// #[async_trait]
/// impl WorkerBackend for MyWorker {
///     async fn execute(&self, job: Job) -> anyhow::Result<WorkResult> {
///         // Process the job
///         println!("Processing job: {}", job.id);
///         Ok(WorkResult::success())
///     }
/// }
/// ```
#[async_trait]
pub trait WorkerBackend: Send + Sync {
    /// Execute a job and return the result
    ///
    /// This method should:
    /// 1. Parse the job payload
    /// 2. Execute the work
    /// 3. Return success or failure
    ///
    /// The worker loop will handle status updates to the control plane.
    async fn execute(&self, job: Job) -> Result<WorkResult>;

    /// Optional: Initialize the worker backend
    ///
    /// Called once before the worker loop starts.
    /// Default implementation does nothing.
    async fn initialize(&self) -> Result<()> {
        Ok(())
    }

    /// Optional: Cleanup when worker shuts down
    ///
    /// Called when the worker is shutting down gracefully.
    /// Default implementation does nothing.
    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}
