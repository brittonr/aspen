//! CI service executor for typed RPC dispatch.
//!
//! Implements `ServiceExecutor` to handle CI/CD operations when
//! invoked via the RPC protocol.

use std::sync::Arc;

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_rpc_core::ServiceExecutor;
use async_trait::async_trait;

#[cfg(feature = "blob")]
use crate::handler::artifacts::handle_get_artifact;
use crate::handler::artifacts::handle_list_artifacts;
use crate::handler::logs::handle_get_job_logs;
#[cfg(feature = "blob")]
use crate::handler::logs::handle_get_job_output;
use crate::handler::logs::handle_subscribe_logs;
use crate::handler::pipeline::handle_cancel_run;
use crate::handler::pipeline::handle_get_status;
#[cfg(feature = "forge")]
use crate::handler::pipeline::handle_list_runs;
#[cfg(all(feature = "forge", feature = "blob"))]
use crate::handler::pipeline::handle_trigger_pipeline;
#[cfg(feature = "forge")]
use crate::handler::watch::handle_unwatch_repo;
#[cfg(feature = "forge")]
use crate::handler::watch::handle_watch_repo;

/// Type alias matching `ClientProtocolContext.forge_node`.
#[cfg(all(feature = "forge", feature = "blob"))]
pub type ForgeNodeRef = Arc<aspen_forge::ForgeNode<aspen_blob::IrohBlobStore, dyn aspen_core::KeyValueStore>>;

/// Service executor for CI/CD operations.
///
/// Captures dependencies at construction time rather than receiving
/// `ClientProtocolContext` at each request. This makes the executor
/// easier to test and more explicit about its requirements.
#[allow(dead_code)] // fields used conditionally behind feature gates
pub struct CiServiceExecutor {
    ci_orchestrator: Option<Arc<aspen_ci::PipelineOrchestrator<dyn aspen_core::KeyValueStore>>>,
    ci_trigger_service: Option<Arc<aspen_ci::TriggerService>>,
    #[cfg(all(feature = "forge", feature = "blob"))]
    forge_node: Option<ForgeNodeRef>,
    #[cfg(feature = "blob")]
    blob_store: Option<Arc<aspen_blob::IrohBlobStore>>,
    kv_store: Arc<dyn aspen_core::KeyValueStore>,
}

impl CiServiceExecutor {
    /// Create a new CI service executor with captured dependencies.
    pub fn new(
        ci_orchestrator: Option<Arc<aspen_ci::PipelineOrchestrator<dyn aspen_core::KeyValueStore>>>,
        ci_trigger_service: Option<Arc<aspen_ci::TriggerService>>,
        #[cfg(all(feature = "forge", feature = "blob"))] forge_node: Option<ForgeNodeRef>,
        #[cfg(feature = "blob")] blob_store: Option<Arc<aspen_blob::IrohBlobStore>>,
        kv_store: Arc<dyn aspen_core::KeyValueStore>,
    ) -> Self {
        Self {
            ci_orchestrator,
            ci_trigger_service,
            #[cfg(all(feature = "forge", feature = "blob"))]
            forge_node,
            #[cfg(feature = "blob")]
            blob_store,
            kv_store,
        }
    }
}

#[async_trait]
impl ServiceExecutor for CiServiceExecutor {
    fn service_name(&self) -> &'static str {
        "ci"
    }

    fn handles(&self) -> &'static [&'static str] {
        &[
            "CiTriggerPipeline",
            "CiGetStatus",
            "CiListRuns",
            "CiCancelRun",
            "CiWatchRepo",
            "CiUnwatchRepo",
            "CiListArtifacts",
            "CiGetArtifact",
            "CiGetJobLogs",
            "CiSubscribeLogs",
            "CiGetJobOutput",
        ]
    }

    fn priority(&self) -> u32 {
        600
    }

    fn app_id(&self) -> Option<&'static str> {
        Some("ci")
    }

    async fn execute(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        match request {
            // Pipeline operations
            #[cfg(all(feature = "forge", feature = "blob"))]
            ClientRpcRequest::CiTriggerPipeline {
                repo_id,
                ref_name,
                commit_hash,
            } => {
                handle_trigger_pipeline(
                    self.ci_orchestrator.as_ref(),
                    self.forge_node.as_ref(),
                    repo_id,
                    ref_name,
                    commit_hash,
                )
                .await
            }
            #[cfg(not(all(feature = "forge", feature = "blob")))]
            ClientRpcRequest::CiTriggerPipeline { .. } => {
                Ok(ClientRpcResponse::error("CI_FEATURE_UNAVAILABLE", "CI trigger requires forge and blob features"))
            }

            ClientRpcRequest::CiGetStatus { run_id } => handle_get_status(self.ci_orchestrator.as_ref(), run_id).await,

            #[cfg(feature = "forge")]
            ClientRpcRequest::CiListRuns { repo_id, status, limit } => {
                handle_list_runs(self.ci_orchestrator.as_ref(), repo_id, status, limit).await
            }
            #[cfg(not(feature = "forge"))]
            ClientRpcRequest::CiListRuns { .. } => {
                Ok(ClientRpcResponse::error("CI_FEATURE_UNAVAILABLE", "CI list runs requires forge feature"))
            }

            ClientRpcRequest::CiCancelRun { run_id, reason } => {
                handle_cancel_run(self.ci_orchestrator.as_ref(), run_id, reason).await
            }

            // Watch operations
            #[cfg(feature = "forge")]
            ClientRpcRequest::CiWatchRepo { repo_id } => {
                handle_watch_repo(self.ci_trigger_service.as_ref(), self.forge_node.as_ref(), repo_id).await
            }
            #[cfg(not(feature = "forge"))]
            ClientRpcRequest::CiWatchRepo { .. } => {
                Ok(ClientRpcResponse::error("CI_FEATURE_UNAVAILABLE", "CI watch requires forge feature"))
            }

            #[cfg(feature = "forge")]
            ClientRpcRequest::CiUnwatchRepo { repo_id } => {
                handle_unwatch_repo(self.ci_trigger_service.as_ref(), self.forge_node.as_ref(), repo_id).await
            }
            #[cfg(not(feature = "forge"))]
            ClientRpcRequest::CiUnwatchRepo { .. } => {
                Ok(ClientRpcResponse::error("CI_FEATURE_UNAVAILABLE", "CI unwatch requires forge feature"))
            }

            // Artifact operations
            ClientRpcRequest::CiListArtifacts { job_id, run_id } => {
                handle_list_artifacts(self.kv_store.as_ref(), job_id, run_id).await
            }
            #[cfg(feature = "blob")]
            ClientRpcRequest::CiGetArtifact { blob_hash } => {
                handle_get_artifact(self.kv_store.as_ref(), self.blob_store.as_deref(), blob_hash).await
            }
            #[cfg(not(feature = "blob"))]
            ClientRpcRequest::CiGetArtifact { .. } => {
                Ok(ClientRpcResponse::error("CI_FEATURE_UNAVAILABLE", "CI artifact retrieval requires blob feature"))
            }

            // Log operations
            ClientRpcRequest::CiGetJobLogs {
                run_id,
                job_id,
                start_index,
                limit,
            } => handle_get_job_logs(self.kv_store.as_ref(), run_id, job_id, start_index, limit).await,
            ClientRpcRequest::CiSubscribeLogs {
                run_id,
                job_id,
                from_index,
            } => handle_subscribe_logs(self.kv_store.as_ref(), run_id, job_id, from_index).await,
            #[cfg(feature = "blob")]
            ClientRpcRequest::CiGetJobOutput { run_id, job_id } => {
                handle_get_job_output(self.kv_store.as_ref(), self.blob_store.as_deref(), run_id, job_id).await
            }
            #[cfg(not(feature = "blob"))]
            ClientRpcRequest::CiGetJobOutput { .. } => {
                Ok(ClientRpcResponse::error("CI_FEATURE_UNAVAILABLE", "CI job output requires blob feature"))
            }

            _ => unreachable!("CiServiceExecutor received unhandled request"),
        }
    }
}
