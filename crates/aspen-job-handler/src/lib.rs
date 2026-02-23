//! Job queue RPC handler for Aspen.
//!
//! Provides `JobServiceExecutor` implementing the typed `ServiceExecutor`
//! trait. Registered as a native handler via `submit_handler_factory!`.
//!
//! Handles 10 operations:
//! - Job lifecycle: Submit, Get, List, Cancel, UpdateProgress, QueueStats
//! - Worker management: Status, Register, Heartbeat, Deregister

mod executor;

use std::sync::Arc;

use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::HandlerFactory;
use aspen_rpc_core::RequestHandler;
use aspen_rpc_core::ServiceHandler;
use aspen_rpc_core::submit_handler_factory;

pub use executor::JobServiceExecutor;

/// Handler factory for the job/worker service.
pub struct JobHandlerFactory;

impl JobHandlerFactory {
    pub const fn new() -> Self {
        Self
    }
}

impl HandlerFactory for JobHandlerFactory {
    fn create(&self, ctx: &ClientProtocolContext) -> Option<Arc<dyn RequestHandler>> {
        let job_manager = ctx.job_manager.as_ref()?.clone();
        let kv_store = ctx.kv_store.clone();
        let worker_service = ctx.worker_service.clone();
        let worker_coordinator = ctx.worker_coordinator.clone();
        let node_id = ctx.node_id;

        let executor = Arc::new(JobServiceExecutor::new(
            job_manager,
            kv_store,
            worker_service,
            worker_coordinator,
            node_id,
        ));
        Some(Arc::new(ServiceHandler::new(executor)))
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

submit_handler_factory!(JobHandlerFactory);
