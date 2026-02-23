//! Forge service executor for typed RPC dispatch.
//!
//! Implements `ServiceExecutor` to handle native forge operations
//! (federation + git bridge) when invoked via the RPC protocol.

use std::sync::Arc;

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_rpc_core::ServiceExecutor;
use async_trait::async_trait;

use crate::handler::handlers::ForgeNodeRef;

/// Service executor for Forge operations (federation + git bridge).
///
/// Repos, objects, refs, issues, and patches are handled by the WASM
/// `aspen-forge-plugin`. This executor retains only operations that
/// require `ForgeNode` context or federation infrastructure.
pub struct ForgeServiceExecutor {
    forge_node: ForgeNodeRef,
    #[cfg(feature = "global-discovery")]
    content_discovery: Option<Arc<dyn aspen_core::ContentDiscovery>>,
    #[cfg(feature = "global-discovery")]
    federation_discovery: Option<Arc<aspen_cluster::federation::FederationDiscoveryService>>,
    federation_identity: Option<Arc<aspen_cluster::federation::SignedClusterIdentity>>,
    federation_trust_manager: Option<Arc<aspen_cluster::federation::TrustManager>>,
}

impl ForgeServiceExecutor {
    /// Variant names handled by this executor (for testing without constructing).
    pub const HANDLES: &'static [&'static str] = &[
        "ForgeGetDelegateKey",
        "GetFederationStatus",
        "ListDiscoveredClusters",
        "GetDiscoveredCluster",
        "TrustCluster",
        "UntrustCluster",
        "FederateRepository",
        "ListFederatedRepositories",
        "ForgeFetchFederated",
        "GitBridgeListRefs",
        "GitBridgeFetch",
        "GitBridgePush",
        "GitBridgePushStart",
        "GitBridgePushChunk",
        "GitBridgePushComplete",
    ];

    pub const SERVICE_NAME: &'static str = "forge";
    pub const PRIORITY: u32 = 540;
    pub const APP_ID: Option<&'static str> = Some("forge");

    /// Create a new forge service executor with captured dependencies.
    pub fn new(
        forge_node: ForgeNodeRef,
        #[cfg(feature = "global-discovery")] content_discovery: Option<Arc<dyn aspen_core::ContentDiscovery>>,
        #[cfg(feature = "global-discovery")] federation_discovery: Option<
            Arc<aspen_cluster::federation::FederationDiscoveryService>,
        >,
        federation_identity: Option<Arc<aspen_cluster::federation::SignedClusterIdentity>>,
        federation_trust_manager: Option<Arc<aspen_cluster::federation::TrustManager>>,
    ) -> Self {
        Self {
            forge_node,
            #[cfg(feature = "global-discovery")]
            content_discovery,
            #[cfg(feature = "global-discovery")]
            federation_discovery,
            federation_identity,
            federation_trust_manager,
        }
    }
}

#[async_trait]
impl ServiceExecutor for ForgeServiceExecutor {
    fn service_name(&self) -> &'static str {
        "forge"
    }

    fn handles(&self) -> &'static [&'static str] {
        Self::HANDLES
    }

    fn priority(&self) -> u32 {
        540
    }

    fn app_id(&self) -> Option<&'static str> {
        Some("forge")
    }

    async fn execute(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        use crate::handler::handlers::federation::*;

        match request {
            // Federation operations
            ClientRpcRequest::ForgeGetDelegateKey { repo_id } => {
                handle_get_delegate_key(&self.forge_node, repo_id).await
            }
            ClientRpcRequest::GetFederationStatus => {
                #[cfg(feature = "global-discovery")]
                {
                    handle_get_federation_status(
                        &self.forge_node,
                        self.content_discovery.as_ref(),
                        self.federation_discovery.as_ref(),
                        self.federation_identity.as_ref(),
                    )
                    .await
                }
                #[cfg(not(feature = "global-discovery"))]
                {
                    handle_get_federation_status(&self.forge_node, self.federation_identity.as_ref()).await
                }
            }
            ClientRpcRequest::ListDiscoveredClusters => {
                #[cfg(feature = "global-discovery")]
                {
                    handle_list_discovered_clusters(self.federation_discovery.as_ref()).await
                }
                #[cfg(not(feature = "global-discovery"))]
                {
                    handle_list_discovered_clusters().await
                }
            }
            ClientRpcRequest::GetDiscoveredCluster { cluster_key } => {
                #[cfg(feature = "global-discovery")]
                {
                    handle_get_discovered_cluster(self.federation_discovery.as_ref(), cluster_key).await
                }
                #[cfg(not(feature = "global-discovery"))]
                {
                    handle_get_discovered_cluster(cluster_key).await
                }
            }
            ClientRpcRequest::TrustCluster { cluster_key } => {
                handle_trust_cluster(self.federation_trust_manager.as_ref(), cluster_key).await
            }
            ClientRpcRequest::UntrustCluster { cluster_key } => {
                handle_untrust_cluster(self.federation_trust_manager.as_ref(), cluster_key).await
            }
            ClientRpcRequest::FederateRepository { repo_id, mode } => {
                handle_federate_repository(&self.forge_node, repo_id, mode).await
            }
            ClientRpcRequest::ListFederatedRepositories => handle_list_federated_repositories(&self.forge_node).await,
            ClientRpcRequest::ForgeFetchFederated {
                federated_id,
                remote_cluster,
            } => handle_fetch_federated(&self.forge_node, federated_id, remote_cluster).await,

            // Git Bridge operations
            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::GitBridgeListRefs { repo_id } => {
                crate::handler::handlers::git_bridge::handle_git_bridge_list_refs(&self.forge_node, repo_id).await
            }
            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::GitBridgeFetch { repo_id, want, have } => {
                crate::handler::handlers::git_bridge::handle_git_bridge_fetch(&self.forge_node, repo_id, want, have)
                    .await
            }
            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::GitBridgePush { repo_id, objects, refs } => {
                crate::handler::handlers::git_bridge::handle_git_bridge_push(&self.forge_node, repo_id, objects, refs)
                    .await
            }
            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::GitBridgePushStart {
                repo_id,
                total_objects,
                total_size_bytes,
                refs,
                metadata,
            } => {
                crate::handler::handlers::git_bridge::handle_git_bridge_push_start(
                    &self.forge_node,
                    repo_id,
                    total_objects,
                    total_size_bytes,
                    refs,
                    metadata,
                )
                .await
            }
            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::GitBridgePushChunk {
                session_id,
                chunk_id,
                total_chunks,
                objects,
                chunk_hash,
            } => {
                crate::handler::handlers::git_bridge::handle_git_bridge_push_chunk(
                    &self.forge_node,
                    session_id,
                    chunk_id,
                    total_chunks,
                    objects,
                    chunk_hash,
                )
                .await
            }
            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::GitBridgePushComplete {
                session_id,
                content_hash,
            } => {
                crate::handler::handlers::git_bridge::handle_git_bridge_push_complete(
                    &self.forge_node,
                    session_id,
                    content_hash,
                )
                .await
            }

            #[cfg(not(feature = "git-bridge"))]
            ClientRpcRequest::GitBridgeListRefs { .. }
            | ClientRpcRequest::GitBridgeFetch { .. }
            | ClientRpcRequest::GitBridgePush { .. }
            | ClientRpcRequest::GitBridgePushStart { .. }
            | ClientRpcRequest::GitBridgePushChunk { .. }
            | ClientRpcRequest::GitBridgePushComplete { .. } => Ok(ClientRpcResponse::error(
                "GIT_BRIDGE_UNAVAILABLE",
                "Git bridge feature not enabled. Rebuild with --features git-bridge",
            )),

            _ => unreachable!("ForgeServiceExecutor received unhandled request"),
        }
    }
}
