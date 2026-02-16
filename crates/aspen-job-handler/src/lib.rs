//! Job queue RPC handlers for Aspen.
//!
//! This crate provides the job and worker handlers extracted from
//! aspen-rpc-handlers for better modularity and faster incremental builds.
//!
//! Handles job system operations:
//! - JobSubmit, JobGet, JobList, JobCancel: Job lifecycle management
//! - JobUpdateProgress, JobQueueStats: Job monitoring
//! - WorkerStatus, WorkerRegister, WorkerHeartbeat, WorkerDeregister: Worker management
//! - WorkerPollJobs, WorkerCompleteJob: Worker job execution

mod job_handler;
mod worker_handler;

use std::sync::Arc;

// Re-export core types for convenience
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::HandlerFactory;
pub use aspen_rpc_core::RequestHandler;
pub use job_handler::JobHandler;
pub use worker_handler::WorkerHandler;

// =============================================================================
// Handler Factories (Plugin Registration)
// =============================================================================

/// Factory for creating `JobHandler` instances.
///
/// This factory enables plugin-style registration via the `inventory` crate.
/// The handler is only created if the `job_manager` is available in the context.
///
/// # Priority
///
/// Priority 560 (feature handler range: 500-599).
pub struct JobHandlerFactory;

impl JobHandlerFactory {
    /// Create a new factory instance.
    pub const fn new() -> Self {
        Self
    }
}

impl Default for JobHandlerFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl HandlerFactory for JobHandlerFactory {
    fn create(&self, ctx: &ClientProtocolContext) -> Option<Arc<dyn RequestHandler>> {
        if ctx.job_manager.is_some() {
            Some(Arc::new(JobHandler))
        } else {
            None
        }
    }

    fn name(&self) -> &'static str {
        "JobHandler"
    }

    fn priority(&self) -> u32 {
        560
    }
}

// Self-register via inventory
aspen_rpc_core::submit_handler_factory!(JobHandlerFactory);

/// Factory for creating `WorkerHandler` instances.
///
/// This factory enables plugin-style registration via the `inventory` crate.
/// The handler is only created if the `job_manager` is available in the context.
///
/// # Priority
///
/// Priority 640 (worker handler range: 600-699).
pub struct WorkerHandlerFactory;

impl WorkerHandlerFactory {
    /// Create a new factory instance.
    pub const fn new() -> Self {
        Self
    }
}

impl Default for WorkerHandlerFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl HandlerFactory for WorkerHandlerFactory {
    fn create(&self, ctx: &ClientProtocolContext) -> Option<Arc<dyn RequestHandler>> {
        if let Some(job_manager) = ctx.job_manager.as_ref() {
            Some(Arc::new(WorkerHandler::new(job_manager.clone())))
        } else {
            None
        }
    }

    fn name(&self) -> &'static str {
        "WorkerHandler"
    }

    fn priority(&self) -> u32 {
        640
    }
}

// Self-register via inventory
aspen_rpc_core::submit_handler_factory!(WorkerHandlerFactory);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_handler_factory_name() {
        let factory = JobHandlerFactory;
        assert_eq!(factory.name(), "JobHandler");
    }

    #[test]
    fn test_job_handler_factory_priority() {
        let factory = JobHandlerFactory;
        assert_eq!(factory.priority(), 560);
    }

    #[test]
    fn test_worker_handler_factory_name() {
        let factory = WorkerHandlerFactory;
        assert_eq!(factory.name(), "WorkerHandler");
    }

    #[test]
    fn test_worker_handler_factory_priority() {
        let factory = WorkerHandlerFactory;
        assert_eq!(factory.priority(), 640);
    }
}
