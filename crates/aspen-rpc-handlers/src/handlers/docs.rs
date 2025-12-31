//! Docs/Sync request handler.
//!
//! Handles: DocsSet, DocsGet, DocsDelete, DocsList, DocsStatus,
//! AddPeerCluster, RemovePeerCluster, ListPeerClusters, GetPeerClusterStatus,
//! UpdatePeerClusterFilter, UpdatePeerClusterPriority, SetPeerClusterEnabled, GetKeyOrigin.

use tracing::warn;

use crate::context::ClientProtocolContext;
use crate::registry::RequestHandler;
use aspen_client::AddPeerClusterResultResponse;
use aspen_client::ClientRpcRequest;
use aspen_client::ClientRpcResponse;
use aspen_client::DocsDeleteResultResponse;
use aspen_client::DocsGetResultResponse;
use aspen_client::DocsListEntry;
use aspen_client::DocsListResultResponse;
use aspen_client::DocsSetResultResponse;
use aspen_client::DocsStatusResultResponse;
use aspen_client::KeyOriginResultResponse;
use aspen_client::ListPeerClustersResultResponse;
use aspen_client::PeerClusterInfo;
use aspen_client::PeerClusterStatusResponse;
use aspen_client::RemovePeerClusterResultResponse;
use aspen_client::SetPeerClusterEnabledResultResponse;
use aspen_client::UpdatePeerClusterFilterResultResponse;
use aspen_client::UpdatePeerClusterPriorityResultResponse;

/// Handler for docs/sync operations.
pub struct DocsHandler;

#[async_trait::async_trait]
impl RequestHandler for DocsHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::DocsSet { .. }
                | ClientRpcRequest::DocsGet { .. }
                | ClientRpcRequest::DocsDelete { .. }
                | ClientRpcRequest::DocsList { .. }
                | ClientRpcRequest::DocsStatus
                | ClientRpcRequest::AddPeerCluster { .. }
                | ClientRpcRequest::RemovePeerCluster { .. }
                | ClientRpcRequest::ListPeerClusters
                | ClientRpcRequest::GetPeerClusterStatus { .. }
                | ClientRpcRequest::UpdatePeerClusterFilter { .. }
                | ClientRpcRequest::UpdatePeerClusterPriority { .. }
                | ClientRpcRequest::SetPeerClusterEnabled { .. }
                | ClientRpcRequest::GetKeyOrigin { .. }
        )
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::DocsSet { key, value } => handle_docs_set(ctx, key, value).await,

            ClientRpcRequest::DocsGet { key } => handle_docs_get(ctx, key).await,

            ClientRpcRequest::DocsDelete { key } => handle_docs_delete(ctx, key).await,

            ClientRpcRequest::DocsList { prefix, limit } => handle_docs_list(ctx, prefix, limit).await,

            ClientRpcRequest::DocsStatus => handle_docs_status(ctx).await,

            ClientRpcRequest::AddPeerCluster { ticket } => handle_add_peer_cluster(ctx, ticket).await,

            ClientRpcRequest::RemovePeerCluster { cluster_id } => handle_remove_peer_cluster(ctx, cluster_id).await,

            ClientRpcRequest::ListPeerClusters => handle_list_peer_clusters(ctx).await,

            ClientRpcRequest::GetPeerClusterStatus { cluster_id } => {
                handle_get_peer_cluster_status(ctx, cluster_id).await
            }

            ClientRpcRequest::UpdatePeerClusterFilter {
                cluster_id,
                filter_type,
                prefixes,
            } => handle_update_peer_cluster_filter(ctx, cluster_id, filter_type, prefixes).await,

            ClientRpcRequest::UpdatePeerClusterPriority { cluster_id, priority } => {
                handle_update_peer_cluster_priority(ctx, cluster_id, priority).await
            }

            ClientRpcRequest::SetPeerClusterEnabled { cluster_id, enabled } => {
                handle_set_peer_cluster_enabled(ctx, cluster_id, enabled).await
            }

            ClientRpcRequest::GetKeyOrigin { key } => handle_get_key_origin(ctx, key).await,

            _ => Err(anyhow::anyhow!("request not handled by DocsHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "DocsHandler"
    }
}

// ============================================================================
// Docs Operation Handlers
// ============================================================================

async fn handle_docs_set(
    ctx: &ClientProtocolContext,
    key: String,
    value: Vec<u8>,
) -> anyhow::Result<ClientRpcResponse> {
    use crate::docs::BlobBackedDocsWriter;
    use crate::docs::DocsWriter;

    let Some(ref docs_sync) = ctx.docs_sync else {
        return Ok(ClientRpcResponse::DocsSetResult(DocsSetResultResponse {
            success: false,
            key: None,
            size: None,
            error: Some("docs not enabled".to_string()),
        }));
    };

    let Some(ref blob_store) = ctx.blob_store else {
        return Ok(ClientRpcResponse::DocsSetResult(DocsSetResultResponse {
            success: false,
            key: None,
            size: None,
            error: Some("blob store not enabled (required for docs)".to_string()),
        }));
    };

    let writer = BlobBackedDocsWriter::new(
        docs_sync.sync_handle.clone(),
        docs_sync.namespace_id,
        docs_sync.author.clone(),
        blob_store.clone(),
    );

    let value_len = value.len() as u64;
    match writer.set_entry(key.as_bytes().to_vec(), value).await {
        Ok(()) => Ok(ClientRpcResponse::DocsSetResult(DocsSetResultResponse {
            success: true,
            key: Some(key),
            size: Some(value_len),
            error: None,
        })),
        Err(e) => {
            warn!(key = %key, error = %e, "docs set failed");
            Ok(ClientRpcResponse::DocsSetResult(DocsSetResultResponse {
                success: false,
                key: Some(key),
                size: None,
                error: Some("docs operation failed".to_string()),
            }))
        }
    }
}

async fn handle_docs_get(ctx: &ClientProtocolContext, key: String) -> anyhow::Result<ClientRpcResponse> {
    use aspen_blob::BlobStore;

    let Some(ref docs_sync) = ctx.docs_sync else {
        return Ok(ClientRpcResponse::DocsGetResult(DocsGetResultResponse {
            found: false,
            value: None,
            size: None,
            error: Some("docs not enabled".to_string()),
        }));
    };

    let key_bytes: bytes::Bytes = bytes::Bytes::from(key.as_bytes().to_vec());
    match docs_sync
        .sync_handle
        .get_exact(docs_sync.namespace_id, docs_sync.author.id(), key_bytes, false)
        .await
    {
        Ok(Some(entry)) => {
            // Entry found, now get the content
            let content_hash = entry.content_hash();
            let content_len = entry.content_len();

            // Try to get content from blob store if available
            if let Some(ref blob_store) = ctx.blob_store {
                match blob_store.get_bytes(&content_hash).await {
                    Ok(Some(bytes)) => Ok(ClientRpcResponse::DocsGetResult(DocsGetResultResponse {
                        found: true,
                        value: Some(bytes.to_vec()),
                        size: Some(content_len),
                        error: None,
                    })),
                    Ok(None) => {
                        // Content exists but not in blob store
                        Ok(ClientRpcResponse::DocsGetResult(DocsGetResultResponse {
                            found: true,
                            value: None,
                            size: Some(content_len),
                            error: Some("content not available locally".to_string()),
                        }))
                    }
                    Err(e) => {
                        warn!(key = %key, error = %e, "docs get blob failed");
                        Ok(ClientRpcResponse::DocsGetResult(DocsGetResultResponse {
                            found: true,
                            value: None,
                            size: Some(content_len),
                            error: Some("failed to retrieve content".to_string()),
                        }))
                    }
                }
            } else {
                // No blob store available
                Ok(ClientRpcResponse::DocsGetResult(DocsGetResultResponse {
                    found: true,
                    value: None,
                    size: Some(content_len),
                    error: Some("blob store not available".to_string()),
                }))
            }
        }
        Ok(None) => Ok(ClientRpcResponse::DocsGetResult(DocsGetResultResponse {
            found: false,
            value: None,
            size: None,
            error: None,
        })),
        Err(e) => {
            warn!(key = %key, error = %e, "docs get failed");
            Ok(ClientRpcResponse::DocsGetResult(DocsGetResultResponse {
                found: false,
                value: None,
                size: None,
                error: Some("docs operation failed".to_string()),
            }))
        }
    }
}

async fn handle_docs_delete(ctx: &ClientProtocolContext, key: String) -> anyhow::Result<ClientRpcResponse> {
    use crate::docs::BlobBackedDocsWriter;
    use crate::docs::DocsWriter;

    let Some(ref docs_sync) = ctx.docs_sync else {
        return Ok(ClientRpcResponse::DocsDeleteResult(DocsDeleteResultResponse {
            success: false,
            error: Some("docs not enabled".to_string()),
        }));
    };

    let Some(ref blob_store) = ctx.blob_store else {
        return Ok(ClientRpcResponse::DocsDeleteResult(DocsDeleteResultResponse {
            success: false,
            error: Some("blob store not enabled (required for docs)".to_string()),
        }));
    };

    let writer = BlobBackedDocsWriter::new(
        docs_sync.sync_handle.clone(),
        docs_sync.namespace_id,
        docs_sync.author.clone(),
        blob_store.clone(),
    );

    match writer.delete_entry(key.as_bytes().to_vec()).await {
        Ok(()) => Ok(ClientRpcResponse::DocsDeleteResult(DocsDeleteResultResponse {
            success: true,
            error: None,
        })),
        Err(e) => {
            warn!(key = %key, error = %e, "docs delete failed");
            Ok(ClientRpcResponse::DocsDeleteResult(DocsDeleteResultResponse {
                success: false,
                error: Some("docs operation failed".to_string()),
            }))
        }
    }
}

async fn handle_docs_list(
    ctx: &ClientProtocolContext,
    prefix: Option<String>,
    limit: Option<u32>,
) -> anyhow::Result<ClientRpcResponse> {
    use iroh_docs::store::Query;

    let Some(ref docs_sync) = ctx.docs_sync else {
        return Ok(ClientRpcResponse::DocsListResult(DocsListResultResponse {
            entries: vec![],
            count: 0,
            has_more: false,
            error: Some("docs not enabled".to_string()),
        }));
    };

    // Build query based on prefix filter
    // Add 1 to limit to detect if there are more entries
    let effective_limit = limit.unwrap_or(100) as u64 + 1;
    let query: Query = if let Some(ref prefix_str) = prefix {
        Query::single_latest_per_key().key_prefix(prefix_str.as_bytes()).limit(effective_limit).into()
    } else {
        Query::single_latest_per_key().limit(effective_limit).into()
    };

    // Create irpc channel to receive results
    let (tx, mut rx) = irpc::channel::mpsc::channel::<iroh_docs::api::RpcResult<iroh_docs::SignedEntry>>(1000);

    // Start the query
    if let Err(e) = docs_sync.sync_handle.get_many(docs_sync.namespace_id, query, tx).await {
        warn!(error = %e, "docs list query failed");
        return Ok(ClientRpcResponse::DocsListResult(DocsListResultResponse {
            entries: vec![],
            count: 0,
            has_more: false,
            error: Some("docs list query failed".to_string()),
        }));
    }

    // Collect results from the channel
    let mut entries = Vec::new();
    let max_entries = limit.unwrap_or(100) as usize;

    while let Ok(Some(result)) = rx.recv().await {
        if let Ok(entry) = result {
            let key = String::from_utf8_lossy(entry.key()).to_string();
            let size = entry.content_len();
            let hash = entry.content_hash().to_string();
            entries.push(DocsListEntry { key, size, hash });

            if entries.len() > max_entries {
                break;
            }
        }
    }

    let has_more = entries.len() > max_entries;
    if has_more {
        entries.pop(); // Remove the extra entry used for has_more detection
    }

    let count = entries.len() as u32;
    Ok(ClientRpcResponse::DocsListResult(DocsListResultResponse {
        entries,
        count,
        has_more,
        error: None,
    }))
}

async fn handle_docs_status(ctx: &ClientProtocolContext) -> anyhow::Result<ClientRpcResponse> {
    use iroh_docs::store::Query;

    let Some(ref docs_sync) = ctx.docs_sync else {
        return Ok(ClientRpcResponse::DocsStatusResult(DocsStatusResultResponse {
            enabled: false,
            namespace_id: None,
            author_id: None,
            entry_count: None,
            replica_open: None,
            error: None,
        }));
    };

    let namespace_id = docs_sync.namespace_id.to_string();
    let author_id = docs_sync.author.id().to_string();

    // Count entries using SyncHandle::get_many
    // Tiger Style: Use bounded query to avoid unbounded memory use
    let query: Query = Query::single_latest_per_key().limit(10001).into(); // Count up to 10000, +1 to detect more
    let (tx, mut rx) = irpc::channel::mpsc::channel::<iroh_docs::api::RpcResult<iroh_docs::SignedEntry>>(1000);

    let entry_count = if docs_sync.sync_handle.get_many(docs_sync.namespace_id, query, tx).await.is_ok() {
        let mut count: u64 = 0;
        while let Ok(Some(_)) = rx.recv().await {
            count += 1;
            if count > 10000 {
                break; // Tiger Style: bounded count
            }
        }
        Some(count)
    } else {
        None
    };

    Ok(ClientRpcResponse::DocsStatusResult(DocsStatusResultResponse {
        enabled: true,
        namespace_id: Some(namespace_id),
        author_id: Some(author_id),
        entry_count,
        replica_open: Some(true),
        error: None,
    }))
}

// ============================================================================
// Peer Cluster Operation Handlers
// ============================================================================

async fn handle_add_peer_cluster(ctx: &ClientProtocolContext, ticket: String) -> anyhow::Result<ClientRpcResponse> {
    use crate::docs::ticket::AspenDocsTicket;

    let Some(ref peer_manager) = ctx.peer_manager else {
        return Ok(ClientRpcResponse::AddPeerClusterResult(AddPeerClusterResultResponse {
            success: false,
            cluster_id: None,
            priority: None,
            error: Some("peer sync not enabled".to_string()),
        }));
    };

    // Parse the ticket
    let docs_ticket = match AspenDocsTicket::deserialize(&ticket) {
        Ok(t) => t,
        Err(_) => {
            return Ok(ClientRpcResponse::AddPeerClusterResult(AddPeerClusterResultResponse {
                success: false,
                cluster_id: None,
                priority: None,
                error: Some("invalid ticket".to_string()),
            }));
        }
    };

    let cluster_id = docs_ticket.cluster_id.clone();
    let priority = docs_ticket.priority as u32;

    match peer_manager.add_peer(docs_ticket).await {
        Ok(()) => Ok(ClientRpcResponse::AddPeerClusterResult(AddPeerClusterResultResponse {
            success: true,
            cluster_id: Some(cluster_id),
            priority: Some(priority),
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "add peer cluster failed");
            Ok(ClientRpcResponse::AddPeerClusterResult(AddPeerClusterResultResponse {
                success: false,
                cluster_id: Some(cluster_id),
                priority: None,
                error: Some("peer cluster operation failed".to_string()),
            }))
        }
    }
}

async fn handle_remove_peer_cluster(
    ctx: &ClientProtocolContext,
    cluster_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    let Some(ref peer_manager) = ctx.peer_manager else {
        return Ok(ClientRpcResponse::RemovePeerClusterResult(RemovePeerClusterResultResponse {
            success: false,
            cluster_id: cluster_id.clone(),
            error: Some("peer sync not enabled".to_string()),
        }));
    };

    match peer_manager.remove_peer(&cluster_id).await {
        Ok(()) => Ok(ClientRpcResponse::RemovePeerClusterResult(RemovePeerClusterResultResponse {
            success: true,
            cluster_id,
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "remove peer cluster failed");
            Ok(ClientRpcResponse::RemovePeerClusterResult(RemovePeerClusterResultResponse {
                success: false,
                cluster_id,
                error: Some("peer cluster operation failed".to_string()),
            }))
        }
    }
}

async fn handle_list_peer_clusters(ctx: &ClientProtocolContext) -> anyhow::Result<ClientRpcResponse> {
    let Some(ref peer_manager) = ctx.peer_manager else {
        return Ok(ClientRpcResponse::ListPeerClustersResult(ListPeerClustersResultResponse {
            peers: vec![],
            count: 0,
            error: Some("peer sync not enabled".to_string()),
        }));
    };

    let peers = peer_manager.list_peers().await;
    let count = peers.len() as u32;
    let peer_infos: Vec<PeerClusterInfo> = peers
        .into_iter()
        .map(|p| PeerClusterInfo {
            cluster_id: p.cluster_id,
            name: p.name,
            state: format!("{:?}", p.state),
            priority: p.priority,
            enabled: p.enabled,
            sync_count: p.sync_count,
            failure_count: p.failure_count,
        })
        .collect();

    Ok(ClientRpcResponse::ListPeerClustersResult(ListPeerClustersResultResponse {
        peers: peer_infos,
        count,
        error: None,
    }))
}

async fn handle_get_peer_cluster_status(
    ctx: &ClientProtocolContext,
    cluster_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    let Some(ref peer_manager) = ctx.peer_manager else {
        return Ok(ClientRpcResponse::PeerClusterStatus(PeerClusterStatusResponse {
            found: false,
            cluster_id: cluster_id.clone(),
            state: "unknown".to_string(),
            syncing: false,
            entries_received: 0,
            entries_imported: 0,
            entries_skipped: 0,
            entries_filtered: 0,
            error: Some("peer sync not enabled".to_string()),
        }));
    };

    match peer_manager.sync_status(&cluster_id).await {
        Some(status) => Ok(ClientRpcResponse::PeerClusterStatus(PeerClusterStatusResponse {
            found: true,
            cluster_id: status.cluster_id,
            state: format!("{:?}", status.state),
            syncing: status.syncing,
            entries_received: status.entries_received,
            entries_imported: status.entries_imported,
            entries_skipped: status.entries_skipped,
            entries_filtered: status.entries_filtered,
            error: None,
        })),
        None => Ok(ClientRpcResponse::PeerClusterStatus(PeerClusterStatusResponse {
            found: false,
            cluster_id,
            state: "unknown".to_string(),
            syncing: false,
            entries_received: 0,
            entries_imported: 0,
            entries_skipped: 0,
            entries_filtered: 0,
            error: None,
        })),
    }
}

async fn handle_update_peer_cluster_filter(
    ctx: &ClientProtocolContext,
    cluster_id: String,
    filter_type: String,
    prefixes: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    use crate::client::SubscriptionFilter;

    let Some(ref peer_manager) = ctx.peer_manager else {
        return Ok(ClientRpcResponse::UpdatePeerClusterFilterResult(UpdatePeerClusterFilterResultResponse {
            success: false,
            cluster_id: cluster_id.clone(),
            filter_type: None,
            error: Some("peer sync not enabled".to_string()),
        }));
    };

    // Parse filter type and prefixes
    let filter = match filter_type.to_lowercase().as_str() {
        "full" | "fullreplication" => SubscriptionFilter::FullReplication,
        "include" | "prefixfilter" => {
            let prefix_list: Vec<String> =
                prefixes.as_ref().map(|p| serde_json::from_str(p).unwrap_or_default()).unwrap_or_default();
            SubscriptionFilter::PrefixFilter(prefix_list)
        }
        "exclude" | "prefixexclude" => {
            let prefix_list: Vec<String> =
                prefixes.as_ref().map(|p| serde_json::from_str(p).unwrap_or_default()).unwrap_or_default();
            SubscriptionFilter::PrefixExclude(prefix_list)
        }
        other => {
            return Ok(ClientRpcResponse::UpdatePeerClusterFilterResult(
                UpdatePeerClusterFilterResultResponse {
                    success: false,
                    cluster_id,
                    filter_type: None,
                    error: Some(format!("invalid filter type: {}", other)),
                },
            ));
        }
    };

    match peer_manager.importer().update_filter(&cluster_id, filter.clone()).await {
        Ok(()) => Ok(ClientRpcResponse::UpdatePeerClusterFilterResult(UpdatePeerClusterFilterResultResponse {
            success: true,
            cluster_id,
            filter_type: Some(filter_type),
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "update peer cluster filter failed");
            Ok(ClientRpcResponse::UpdatePeerClusterFilterResult(UpdatePeerClusterFilterResultResponse {
                success: false,
                cluster_id,
                filter_type: None,
                error: Some("peer cluster operation failed".to_string()),
            }))
        }
    }
}

async fn handle_update_peer_cluster_priority(
    ctx: &ClientProtocolContext,
    cluster_id: String,
    priority: u32,
) -> anyhow::Result<ClientRpcResponse> {
    let Some(ref peer_manager) = ctx.peer_manager else {
        return Ok(ClientRpcResponse::UpdatePeerClusterPriorityResult(
            UpdatePeerClusterPriorityResultResponse {
                success: false,
                cluster_id: cluster_id.clone(),
                previous_priority: None,
                new_priority: None,
                error: Some("peer sync not enabled".to_string()),
            },
        ));
    };

    // Get current priority before update
    let previous_priority =
        peer_manager.list_peers().await.into_iter().find(|p| p.cluster_id == cluster_id).map(|p| p.priority);

    match peer_manager.importer().update_priority(&cluster_id, priority).await {
        Ok(()) => {
            Ok(ClientRpcResponse::UpdatePeerClusterPriorityResult(UpdatePeerClusterPriorityResultResponse {
                success: true,
                cluster_id,
                previous_priority,
                new_priority: Some(priority),
                error: None,
            }))
        }
        Err(e) => {
            warn!(error = %e, "update peer cluster priority failed");
            Ok(ClientRpcResponse::UpdatePeerClusterPriorityResult(UpdatePeerClusterPriorityResultResponse {
                success: false,
                cluster_id,
                previous_priority,
                new_priority: None,
                error: Some("peer cluster operation failed".to_string()),
            }))
        }
    }
}

async fn handle_set_peer_cluster_enabled(
    ctx: &ClientProtocolContext,
    cluster_id: String,
    enabled: bool,
) -> anyhow::Result<ClientRpcResponse> {
    let Some(ref peer_manager) = ctx.peer_manager else {
        return Ok(ClientRpcResponse::SetPeerClusterEnabledResult(SetPeerClusterEnabledResultResponse {
            success: false,
            cluster_id: cluster_id.clone(),
            enabled: None,
            error: Some("peer sync not enabled".to_string()),
        }));
    };

    match peer_manager.importer().set_enabled(&cluster_id, enabled).await {
        Ok(()) => Ok(ClientRpcResponse::SetPeerClusterEnabledResult(SetPeerClusterEnabledResultResponse {
            success: true,
            cluster_id,
            enabled: Some(enabled),
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "set peer cluster enabled failed");
            Ok(ClientRpcResponse::SetPeerClusterEnabledResult(SetPeerClusterEnabledResultResponse {
                success: false,
                cluster_id,
                enabled: None,
                error: Some("peer cluster operation failed".to_string()),
            }))
        }
    }
}

async fn handle_get_key_origin(ctx: &ClientProtocolContext, key: String) -> anyhow::Result<ClientRpcResponse> {
    let Some(ref peer_manager) = ctx.peer_manager else {
        return Ok(ClientRpcResponse::KeyOriginResult(KeyOriginResultResponse {
            found: false,
            key: key.clone(),
            cluster_id: None,
            priority: None,
            timestamp_secs: None,
            is_local: None,
        }));
    };

    match peer_manager.importer().get_key_origin(&key).await {
        Some(origin) => {
            let is_local = origin.is_local();
            Ok(ClientRpcResponse::KeyOriginResult(KeyOriginResultResponse {
                found: true,
                key,
                cluster_id: Some(origin.cluster_id),
                priority: Some(origin.priority),
                timestamp_secs: Some(origin.timestamp_secs),
                is_local: Some(is_local),
            }))
        }
        None => Ok(ClientRpcResponse::KeyOriginResult(KeyOriginResultResponse {
            found: false,
            key,
            cluster_id: None,
            priority: None,
            timestamp_secs: None,
            is_local: None,
        })),
    }
}
