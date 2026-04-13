//! Job queue RPC handler for Aspen.
//!
//! Provides `JobServiceExecutor` implementing the typed `ServiceExecutor`.
//! Wired into the native handler registry explicitly.
//!
//! Handles 12 operations:
//! - Job lifecycle: Submit, Get, List, Cancel, UpdateProgress, QueueStats
//! - Worker management: Status, Register, Heartbeat, Deregister, PollJobs, CompleteJob

mod executor;

use std::sync::Arc;

use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::HandlerFactory;
use aspen_rpc_core::RequestHandler;
use aspen_rpc_core::ServiceHandler;
pub use executor::JobServiceExecutor;

/// Handler factory for the job/worker service.
pub struct JobHandlerFactory;

impl Default for JobHandlerFactory {
    fn default() -> Self {
        Self
    }
}

impl JobHandlerFactory {
    pub const fn new() -> Self {
        Self
    }
}

impl HandlerFactory for JobHandlerFactory {
    fn create(&self, ctx: &ClientProtocolContext) -> anyhow::Result<Arc<dyn RequestHandler>> {
        let caps = ctx.jobs_handler_context()?;
        let executor = Arc::new(JobServiceExecutor::new(
            caps.job_manager,
            caps.kv_store,
            caps.worker_service,
            caps.worker_coordinator,
            caps.node_id,
        ));
        Ok(Arc::new(ServiceHandler::new(executor)))
    }

    fn name(&self) -> &'static str {
        "JobHandler"
    }

    fn priority(&self) -> u32 {
        560
    }

    fn app_id(&self) -> Option<&'static str> {
        Some("jobs")
    }
}
