//! Docs service executor for typed RPC dispatch.
//!
//! Implements `ServiceExecutor` to handle docs/sync operations when
//! invoked via the RPC protocol.

use std::sync::Arc;

use anyhow::Result;
use aspen_client_api::messages::AddPeerClusterResultResponse;
use aspen_client_api::messages::ClientRpcRequest;
use aspen_client_api::messages::ClientRpcResponse;
use aspen_client_api::messages::DocsDeleteResultResponse;
use aspen_client_api::messages::DocsGetResultResponse;
use aspen_client_api::messages::DocsListEntry;
use aspen_client_api::messages::DocsListResultResponse;
use aspen_client_api::messages::DocsSetResultResponse;
use aspen_client_api::messages::DocsStatusResultResponse;
use aspen_client_api::messages::KeyOriginResultResponse;
use aspen_client_api::messages::ListPeerClustersResultResponse;
use aspen_client_api::messages::PeerClusterInfo;
use aspen_client_api::messages::PeerClusterStatusResponse;
use aspen_client_api::messages::RemovePeerClusterResultResponse;
use aspen_client_api::messages::SetPeerClusterEnabledResultResponse;
use aspen_client_api::messages::UpdatePeerClusterFilterResultResponse;
use aspen_client_api::messages::UpdatePeerClusterPriorityResultResponse;
use aspen_core::context::DocsSyncProvider;
use aspen_core::context::PeerManager;
use aspen_core::context::SubscriptionFilter;
use aspen_rpc_core::ServiceExecutor;
use async_trait::async_trait;
use serde_json;
use tracing::warn;

/// Service executor for docs/sync operations.
///
/// Handles the same operations as the former native `DocsHandler`:
/// CRUD (set/get/delete/list), status, key origin, peer federation,
/// and peer configuration.
pub struct DocsServiceExecutor {
    docs_sync: Arc<dyn DocsSyncProvider>,
    peer_manager: Option<Arc<dyn PeerManager>>,
}

impl DocsServiceExecutor {
    /// Create a new docs service executor.
    pub fn new(docs_sync: Arc<dyn DocsSyncProvider>, peer_manager: Option<Arc<dyn PeerManager>>) -> Self {
        Self {
            docs_sync,
            peer_manager,
        }
    }
}

#[async_trait]
impl ServiceExecutor for DocsServiceExecutor {
    fn service_name(&self) -> &'static str {
        "docs"
    }

    fn handles(&self) -> &'static [&'static str] {
        &[
            "DocsSet",
            "DocsGet",
            "DocsDelete",
            "DocsList",
            "DocsStatus",
            "GetKeyOrigin",
            "AddPeerCluster",
            "RemovePeerCluster",
            "ListPeerClusters",
            "GetPeerClusterStatus",
            "UpdatePeerClusterFilter",
            "UpdatePeerClusterPriority",
            "SetPeerClusterEnabled",
        ]
    }

    fn priority(&self) -> u32 {
        530
    }

    fn app_id(&self) -> Option<&'static str> {
        Some("docs")
    }

    async fn execute(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::DocsSet { key, value } => self.op_set(key, value).await,
            ClientRpcRequest::DocsGet { key } => self.op_get(key).await,
            ClientRpcRequest::DocsDelete { key } => self.op_delete(key).await,
            ClientRpcRequest::DocsList { prefix, limit } => self.op_list(prefix, limit).await,
            ClientRpcRequest::DocsStatus => self.op_status().await,
            ClientRpcRequest::GetKeyOrigin { key } => self.op_get_key_origin(key).await,
            ClientRpcRequest::AddPeerCluster { ticket } => self.op_add_peer(ticket).await,
            ClientRpcRequest::RemovePeerCluster { cluster_id } => self.op_remove_peer(cluster_id).await,
            ClientRpcRequest::ListPeerClusters => self.op_list_peers().await,
            ClientRpcRequest::GetPeerClusterStatus { cluster_id } => self.op_get_peer_status(cluster_id).await,
            ClientRpcRequest::UpdatePeerClusterFilter {
                cluster_id,
                filter_type,
                prefixes,
            } => self.op_update_filter(cluster_id, filter_type, prefixes).await,
            ClientRpcRequest::UpdatePeerClusterPriority { cluster_id, priority } => {
                self.op_update_priority(cluster_id, priority).await
            }
            ClientRpcRequest::SetPeerClusterEnabled { cluster_id, is_enabled } => {
                self.op_set_enabled(cluster_id, is_enabled).await
            }
            _ => unreachable!("DocsServiceExecutor received unhandled request"),
        }
    }
}

impl DocsServiceExecutor {
    // =========================================================================
    // CRUD operations
    // =========================================================================

    async fn op_set(&self, key: String, value: Vec<u8>) -> Result<ClientRpcResponse> {
        let value_len = value.len() as u64;
        match self.docs_sync.set_entry(key.as_bytes().to_vec(), value).await {
            Ok(()) => Ok(ClientRpcResponse::DocsSetResult(DocsSetResultResponse {
                is_success: true,
                key: Some(key),
                size_bytes: Some(value_len),
                error: None,
            })),
            Err(e) => {
                warn!(key = %key, error = %e, "docs set failed");
                Ok(ClientRpcResponse::DocsSetResult(DocsSetResultResponse {
                    is_success: false,
                    key: Some(key),
                    size_bytes: None,
                    error: Some("docs operation failed".to_string()),
                }))
            }
        }
    }

    async fn op_get(&self, key: String) -> Result<ClientRpcResponse> {
        match self.docs_sync.get_entry(key.as_bytes()).await {
            Ok(Some((value, size, _hash))) => Ok(ClientRpcResponse::DocsGetResult(DocsGetResultResponse {
                was_found: true,
                value: Some(value),
                size_bytes: Some(size),
                error: None,
            })),
            Ok(None) => Ok(ClientRpcResponse::DocsGetResult(DocsGetResultResponse {
                was_found: false,
                value: None,
                size_bytes: None,
                error: None,
            })),
            Err(e) => {
                warn!(key = %key, error = %e, "docs get failed");
                Ok(ClientRpcResponse::DocsGetResult(DocsGetResultResponse {
                    was_found: false,
                    value: None,
                    size_bytes: None,
                    error: Some("docs operation failed".to_string()),
                }))
            }
        }
    }

    async fn op_delete(&self, key: String) -> Result<ClientRpcResponse> {
        match self.docs_sync.delete_entry(key.as_bytes().to_vec()).await {
            Ok(()) => Ok(ClientRpcResponse::DocsDeleteResult(DocsDeleteResultResponse {
                is_success: true,
                error: None,
            })),
            Err(e) => {
                warn!(key = %key, error = %e, "docs delete failed");
                Ok(ClientRpcResponse::DocsDeleteResult(DocsDeleteResultResponse {
                    is_success: false,
                    error: Some("docs operation failed".to_string()),
                }))
            }
        }
    }

    async fn op_list(&self, prefix: Option<String>, limit: Option<u32>) -> Result<ClientRpcResponse> {
        match self.docs_sync.list_entries(prefix, limit).await {
            Ok(entries) => {
                let max_entries = limit.unwrap_or(100) as usize;
                let has_more = entries.len() > max_entries;
                let mut result_entries = entries;
                if has_more {
                    result_entries.pop();
                }

                let count = result_entries.len() as u32;
                let entries_typed: Vec<DocsListEntry> = result_entries
                    .into_iter()
                    .map(|e| DocsListEntry {
                        key: e.key,
                        size_bytes: e.size_bytes,
                        hash: e.hash,
                    })
                    .collect();

                Ok(ClientRpcResponse::DocsListResult(DocsListResultResponse {
                    entries: entries_typed,
                    count,
                    has_more,
                    error: None,
                }))
            }
            Err(e) => {
                warn!(error = %e, "docs list failed");
                Ok(ClientRpcResponse::DocsListResult(DocsListResultResponse {
                    entries: vec![],
                    count: 0,
                    has_more: false,
                    error: Some("docs list operation failed".to_string()),
                }))
            }
        }
    }

    // =========================================================================
    // Status / key origin
    // =========================================================================

    async fn op_status(&self) -> Result<ClientRpcResponse> {
        let namespace_id = self.docs_sync.namespace_id();
        let author_id = self.docs_sync.author_id();

        match self.docs_sync.get_status().await {
            Ok(status) => Ok(ClientRpcResponse::DocsStatusResult(DocsStatusResultResponse {
                is_enabled: status.is_enabled,
                namespace_id: Some(namespace_id),
                author_id: Some(author_id),
                entry_count: status.entry_count,
                replica_open: status.replica_open,
                error: None,
            })),
            Err(e) => {
                warn!(error = %e, "docs status failed");
                Ok(ClientRpcResponse::DocsStatusResult(DocsStatusResultResponse {
                    is_enabled: true,
                    namespace_id: Some(namespace_id),
                    author_id: Some(author_id),
                    entry_count: None,
                    replica_open: None,
                    error: Some("status query failed".to_string()),
                }))
            }
        }
    }

    async fn op_get_key_origin(&self, key: String) -> Result<ClientRpcResponse> {
        let peer_manager = match &self.peer_manager {
            Some(pm) => pm,
            None => {
                return Ok(ClientRpcResponse::KeyOriginResult(KeyOriginResultResponse {
                    was_found: false,
                    key,
                    cluster_id: None,
                    priority: None,
                    timestamp_secs: None,
                    is_local: None,
                }));
            }
        };

        let importer = match peer_manager.importer() {
            Some(imp) => imp,
            None => {
                return Ok(ClientRpcResponse::KeyOriginResult(KeyOriginResultResponse {
                    was_found: false,
                    key,
                    cluster_id: None,
                    priority: None,
                    timestamp_secs: None,
                    is_local: None,
                }));
            }
        };

        match importer.get_key_origin(&key).await {
            Some(origin) => {
                let is_local = origin.is_local();
                Ok(ClientRpcResponse::KeyOriginResult(KeyOriginResultResponse {
                    was_found: true,
                    key,
                    cluster_id: Some(origin.cluster_id),
                    priority: Some(origin.priority),
                    timestamp_secs: Some(origin.timestamp_secs),
                    is_local: Some(is_local),
                }))
            }
            None => Ok(ClientRpcResponse::KeyOriginResult(KeyOriginResultResponse {
                was_found: false,
                key,
                cluster_id: None,
                priority: None,
                timestamp_secs: None,
                is_local: None,
            })),
        }
    }

    // =========================================================================
    // Peer federation
    // =========================================================================

    async fn op_add_peer(&self, ticket: String) -> Result<ClientRpcResponse> {
        let peer_manager = match &self.peer_manager {
            Some(pm) => pm,
            None => {
                return Ok(ClientRpcResponse::AddPeerClusterResult(AddPeerClusterResultResponse {
                    is_success: false,
                    cluster_id: None,
                    priority: None,
                    error: Some("peer sync not enabled".to_string()),
                }));
            }
        };

        // The ticket string is the cluster_id; priority defaults to 0
        // (This matches the old behavior where cluster_id = ticket_str, priority = 0)
        let cluster_id = ticket;
        let priority = 0u32;

        let docs_ticket = aspen_core::AspenDocsTicket {
            cluster_id: cluster_id.clone(),
            priority: priority as u8,
        };

        match peer_manager.add_peer(docs_ticket).await {
            Ok(()) => Ok(ClientRpcResponse::AddPeerClusterResult(AddPeerClusterResultResponse {
                is_success: true,
                cluster_id: Some(cluster_id),
                priority: Some(priority),
                error: None,
            })),
            Err(e) => {
                warn!(error = %e, "add peer cluster failed");
                Ok(ClientRpcResponse::AddPeerClusterResult(AddPeerClusterResultResponse {
                    is_success: false,
                    cluster_id: Some(cluster_id),
                    priority: None,
                    error: Some("peer cluster operation failed".to_string()),
                }))
            }
        }
    }

    async fn op_remove_peer(&self, cluster_id: String) -> Result<ClientRpcResponse> {
        let peer_manager = match &self.peer_manager {
            Some(pm) => pm,
            None => {
                return Ok(ClientRpcResponse::RemovePeerClusterResult(RemovePeerClusterResultResponse {
                    is_success: false,
                    cluster_id,
                    error: Some("peer sync not enabled".to_string()),
                }));
            }
        };

        match peer_manager.remove_peer(&cluster_id).await {
            Ok(()) => Ok(ClientRpcResponse::RemovePeerClusterResult(RemovePeerClusterResultResponse {
                is_success: true,
                cluster_id,
                error: None,
            })),
            Err(e) => {
                warn!(error = %e, "remove peer cluster failed");
                Ok(ClientRpcResponse::RemovePeerClusterResult(RemovePeerClusterResultResponse {
                    is_success: false,
                    cluster_id,
                    error: Some("peer cluster operation failed".to_string()),
                }))
            }
        }
    }

    async fn op_list_peers(&self) -> Result<ClientRpcResponse> {
        let peer_manager = match &self.peer_manager {
            Some(pm) => pm,
            None => {
                return Ok(ClientRpcResponse::ListPeerClustersResult(ListPeerClustersResultResponse {
                    peers: vec![],
                    count: 0,
                    error: Some("peer sync not enabled".to_string()),
                }));
            }
        };

        let peers = peer_manager.list_peers().await;
        let count = peers.len() as u32;
        let peers_typed: Vec<PeerClusterInfo> = peers
            .into_iter()
            .map(|p| PeerClusterInfo {
                cluster_id: p.cluster_id,
                name: p.name,
                state: format!("{:?}", p.state),
                priority: p.priority,
                is_enabled: p.is_enabled,
                sync_count: p.sync_count,
                failure_count: p.failure_count,
            })
            .collect();

        Ok(ClientRpcResponse::ListPeerClustersResult(ListPeerClustersResultResponse {
            peers: peers_typed,
            count,
            error: None,
        }))
    }

    // =========================================================================
    // Peer configuration
    // =========================================================================

    async fn op_get_peer_status(&self, cluster_id: String) -> Result<ClientRpcResponse> {
        let peer_manager = match &self.peer_manager {
            Some(pm) => pm,
            None => {
                return Ok(ClientRpcResponse::PeerClusterStatus(PeerClusterStatusResponse {
                    was_found: false,
                    cluster_id,
                    state: "unknown".to_string(),
                    is_syncing: false,
                    entries_received: 0,
                    entries_imported: 0,
                    entries_skipped: 0,
                    entries_filtered: 0,
                    error: Some("peer sync not enabled".to_string()),
                }));
            }
        };

        match peer_manager.sync_status(&cluster_id).await {
            Some(status) => Ok(ClientRpcResponse::PeerClusterStatus(PeerClusterStatusResponse {
                was_found: true,
                cluster_id: status.cluster_id,
                state: format!("{:?}", status.state),
                is_syncing: status.is_syncing,
                entries_received: status.entries_received,
                entries_imported: status.entries_imported,
                entries_skipped: status.entries_skipped,
                entries_filtered: status.entries_filtered,
                error: None,
            })),
            None => Ok(ClientRpcResponse::PeerClusterStatus(PeerClusterStatusResponse {
                was_found: false,
                cluster_id,
                state: "unknown".to_string(),
                is_syncing: false,
                entries_received: 0,
                entries_imported: 0,
                entries_skipped: 0,
                entries_filtered: 0,
                error: None,
            })),
        }
    }

    async fn op_update_filter(
        &self,
        cluster_id: String,
        filter_type: String,
        prefixes: Option<String>,
    ) -> Result<ClientRpcResponse> {
        let peer_manager = match &self.peer_manager {
            Some(pm) => pm,
            None => {
                return Ok(ClientRpcResponse::UpdatePeerClusterFilterResult(UpdatePeerClusterFilterResultResponse {
                    is_success: false,
                    cluster_id,
                    filter_type: None,
                    error: Some("peer sync not enabled".to_string()),
                }));
            }
        };

        // Parse prefixes from JSON string if provided
        let prefix_list: Vec<String> = if let Some(prefixes_str) = prefixes {
            serde_json::from_str(&prefixes_str).unwrap_or_default()
        } else {
            vec![]
        };

        let filter = match filter_type.to_lowercase().as_str() {
            "full" | "fullreplication" => SubscriptionFilter::FullReplication,
            "include" | "prefixfilter" => SubscriptionFilter::PrefixFilter(prefix_list),
            "exclude" | "prefixexclude" => SubscriptionFilter::PrefixExclude(prefix_list),
            other => {
                return Ok(ClientRpcResponse::UpdatePeerClusterFilterResult(UpdatePeerClusterFilterResultResponse {
                    is_success: false,
                    cluster_id,
                    filter_type: None,
                    error: Some(format!("invalid filter type: {other}")),
                }));
            }
        };

        let importer = match peer_manager.importer() {
            Some(imp) => imp,
            None => {
                return Ok(ClientRpcResponse::UpdatePeerClusterFilterResult(UpdatePeerClusterFilterResultResponse {
                    is_success: false,
                    cluster_id,
                    filter_type: None,
                    error: Some("importer not available".to_string()),
                }));
            }
        };

        match importer.update_filter(&cluster_id, filter).await {
            Ok(()) => Ok(ClientRpcResponse::UpdatePeerClusterFilterResult(UpdatePeerClusterFilterResultResponse {
                is_success: true,
                cluster_id,
                filter_type: Some(filter_type),
                error: None,
            })),
            Err(e) => {
                warn!(error = %e, "update peer cluster filter failed");
                Ok(ClientRpcResponse::UpdatePeerClusterFilterResult(UpdatePeerClusterFilterResultResponse {
                    is_success: false,
                    cluster_id,
                    filter_type: None,
                    error: Some("peer cluster operation failed".to_string()),
                }))
            }
        }
    }

    async fn op_update_priority(&self, cluster_id: String, priority: u32) -> Result<ClientRpcResponse> {
        let peer_manager = match &self.peer_manager {
            Some(pm) => pm,
            None => {
                return Ok(ClientRpcResponse::UpdatePeerClusterPriorityResult(
                    UpdatePeerClusterPriorityResultResponse {
                        is_success: false,
                        cluster_id,
                        previous_priority: None,
                        new_priority: None,
                        error: Some("peer sync not enabled".to_string()),
                    },
                ));
            }
        };

        let previous_priority =
            peer_manager.list_peers().await.into_iter().find(|p| p.cluster_id == cluster_id).map(|p| p.priority);

        let importer = match peer_manager.importer() {
            Some(imp) => imp,
            None => {
                return Ok(ClientRpcResponse::UpdatePeerClusterPriorityResult(
                    UpdatePeerClusterPriorityResultResponse {
                        is_success: false,
                        cluster_id,
                        previous_priority,
                        new_priority: None,
                        error: Some("importer not available".to_string()),
                    },
                ));
            }
        };

        match importer.update_priority(&cluster_id, priority).await {
            Ok(()) => Ok(ClientRpcResponse::UpdatePeerClusterPriorityResult(UpdatePeerClusterPriorityResultResponse {
                is_success: true,
                cluster_id,
                previous_priority,
                new_priority: Some(priority),
                error: None,
            })),
            Err(e) => {
                warn!(error = %e, "update peer cluster priority failed");
                Ok(ClientRpcResponse::UpdatePeerClusterPriorityResult(UpdatePeerClusterPriorityResultResponse {
                    is_success: false,
                    cluster_id,
                    previous_priority,
                    new_priority: None,
                    error: Some("peer cluster operation failed".to_string()),
                }))
            }
        }
    }

    async fn op_set_enabled(&self, cluster_id: String, enabled: bool) -> Result<ClientRpcResponse> {
        let peer_manager = match &self.peer_manager {
            Some(pm) => pm,
            None => {
                return Ok(ClientRpcResponse::SetPeerClusterEnabledResult(SetPeerClusterEnabledResultResponse {
                    is_success: false,
                    cluster_id,
                    is_enabled: None,
                    error: Some("peer sync not enabled".to_string()),
                }));
            }
        };

        let importer = match peer_manager.importer() {
            Some(imp) => imp,
            None => {
                return Ok(ClientRpcResponse::SetPeerClusterEnabledResult(SetPeerClusterEnabledResultResponse {
                    is_success: false,
                    cluster_id,
                    is_enabled: None,
                    error: Some("importer not available".to_string()),
                }));
            }
        };

        match importer.set_enabled(&cluster_id, enabled).await {
            Ok(()) => Ok(ClientRpcResponse::SetPeerClusterEnabledResult(SetPeerClusterEnabledResultResponse {
                is_success: true,
                cluster_id,
                is_enabled: Some(enabled),
                error: None,
            })),
            Err(e) => {
                warn!(error = %e, "set peer cluster enabled failed");
                Ok(ClientRpcResponse::SetPeerClusterEnabledResult(SetPeerClusterEnabledResultResponse {
                    is_success: false,
                    cluster_id,
                    is_enabled: None,
                    error: Some("peer cluster operation failed".to_string()),
                }))
            }
        }
    }
}
