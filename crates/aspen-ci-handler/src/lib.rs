//! CI/CD pipeline RPC handler for Aspen distributed cluster.
//!
//! This crate implements the RPC handler for CI/CD operations, providing:
//! - Pipeline triggering and execution
//! - Pipeline status queries
//! - Pipeline run listing and filtering
//! - Repository watching for automatic CI triggers
//! - Artifact listing and retrieval
//! - Job log fetching and subscription
//! - Job output retrieval
//!
//! # Architecture
//!
//! The handler uses the `ServiceExecutor` pattern from `aspen-rpc-core`,
//! allowing it to be registered with the `HandlerRegistry` in `aspen-rpc-handlers`.
//!
//! ```text
//! aspen-rpc-handlers
//!        |
//!        v
//!   HandlerRegistry
//!        |
//!        v
//!   ServiceHandler<CiServiceExecutor> (this crate)
//!        |
//!        +---> Pipeline Operations (trigger, status, list, cancel)
//!        +---> Watch Operations (watch/unwatch repo)
//!        +---> Artifact Operations (list, get)
//!        +---> Log Operations (get logs, subscribe, get output)
//! ```

mod executor;
pub mod handler;

use std::sync::Arc;

// Re-export types that handlers need from aspen-rpc-core
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::HandlerFactory;
pub use aspen_rpc_core::RequestHandler;
pub use aspen_rpc_core::ServiceHandler;
pub use executor::CiServiceExecutor;

// =============================================================================
// Handler Factory (Plugin Registration)
// =============================================================================

/// Factory for creating CI service handler instances.
///
/// This factory enables plugin-style registration via the `inventory` crate.
/// The handler is only created if CI services (orchestrator or trigger) are
/// available in the context.
///
/// # Priority
///
/// Priority 600 (feature handler range: 500-699).
pub struct CiHandlerFactory;

impl CiHandlerFactory {
    /// Create a new factory instance.
    pub const fn new() -> Self {
        Self
    }
}

impl Default for CiHandlerFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl HandlerFactory for CiHandlerFactory {
    fn create(&self, ctx: &ClientProtocolContext) -> anyhow::Result<Arc<dyn RequestHandler>> {
        let caps = ctx.ci_handler_context()?;

        // Set up deploy dispatcher on the orchestrator so it can
        // automatically spawn deploy monitors for both trigger paths
        // (direct RPC and auto-trigger via gossip).
        if let Some(ref orchestrator) = caps.ci_orchestrator {
            let deploy_dispatcher: Arc<dyn aspen_ci::DeployDispatcher> =
                Arc::new(handler::deploy::RpcDeployDispatcher::new(
                    caps.kv_store.clone(),
                    caps.controller.clone(),
                    caps.endpoint_manager.endpoint().clone(),
                    caps.node_id,
                ));
            orchestrator.set_deploy_dispatcher(deploy_dispatcher);

            // Set up status reporter so CI writes commit statuses back to Forge
            let status_reporter: Arc<dyn aspen_ci::StatusReporter> =
                Arc::new(aspen_ci::ForgeStatusReporter::new(caps.kv_store.clone()));
            orchestrator.set_status_reporter(status_reporter);
        }

        let executor = Arc::new(CiServiceExecutor::new(
            caps.ci_orchestrator,
            caps.ci_trigger_service,
            #[cfg(all(feature = "forge", feature = "blob"))]
            caps.forge_node,
            #[cfg(feature = "blob")]
            caps.blob_store,
            caps.kv_store,
        ));
        Ok(Arc::new(ServiceHandler::new(executor)))
    }

    fn name(&self) -> &'static str {
        "CiHandler"
    }

    fn priority(&self) -> u32 {
        600
    }

    fn app_id(&self) -> Option<&'static str> {
        Some("ci")
    }
}

// Self-register via inventory

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factory_name() {
        let factory = CiHandlerFactory;
        assert_eq!(factory.name(), "CiHandler");
    }

    #[test]
    fn test_factory_priority() {
        let factory = CiHandlerFactory;
        assert_eq!(factory.priority(), 600);
    }

    #[test]
    fn test_factory_app_id() {
        let factory = CiHandlerFactory;
        assert_eq!(factory.app_id(), Some("ci"));
    }

    /// Expected CI handles (must stay in sync with executor.rs).
    const CI_HANDLES: &[&str] = &[
        "CiTriggerPipeline",
        "CiGetStatus",
        "CiGetRefStatus",
        "CiListRuns",
        "CiCancelRun",
        "CiWatchRepo",
        "CiUnwatchRepo",
        "CiListArtifacts",
        "CiGetArtifact",
        "CiGetJobLogs",
        "CiSubscribeLogs",
        "CiGetJobOutput",
    ];

    #[test]
    fn test_handles_count() {
        assert_eq!(CI_HANDLES.len(), 12, "CI handler should handle 12 operations");
    }

    #[test]
    fn test_no_duplicate_handles() {
        let mut sorted: Vec<_> = CI_HANDLES.to_vec();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), CI_HANDLES.len(), "no duplicate CI handles");
    }

    #[test]
    fn test_handles_pipeline_ops() {
        assert!(CI_HANDLES.contains(&"CiTriggerPipeline"));
        assert!(CI_HANDLES.contains(&"CiGetStatus"));
        assert!(CI_HANDLES.contains(&"CiGetRefStatus"));
        assert!(CI_HANDLES.contains(&"CiListRuns"));
        assert!(CI_HANDLES.contains(&"CiCancelRun"));
    }

    #[test]
    fn test_handles_watch_ops() {
        assert!(CI_HANDLES.contains(&"CiWatchRepo"));
        assert!(CI_HANDLES.contains(&"CiUnwatchRepo"));
    }

    #[test]
    fn test_handles_artifact_ops() {
        assert!(CI_HANDLES.contains(&"CiListArtifacts"));
        assert!(CI_HANDLES.contains(&"CiGetArtifact"));
    }

    #[test]
    fn test_handles_log_ops() {
        assert!(CI_HANDLES.contains(&"CiGetJobLogs"));
        assert!(CI_HANDLES.contains(&"CiSubscribeLogs"));
        assert!(CI_HANDLES.contains(&"CiGetJobOutput"));
    }
}
