//! Blob replication operations: replicate_pull, status, trigger, repair.

use std::time::Instant;

use aspen_blob::IrohBlobStore;
use aspen_blob::prelude::*;
use aspen_client_api::BlobReplicatePullResultResponse;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::GetBlobReplicationStatusResultResponse;
use aspen_client_api::RunBlobRepairCycleResultResponse;
use aspen_client_api::TriggerBlobReplicationResultResponse;
use aspen_rpc_core::ClientProtocolContext;
use iroh::PublicKey;
use iroh_blobs::Hash;
use tracing::info;
use tracing::warn;

use super::error::sanitize_blob_error;

/// Handle BlobReplicatePull request.
///
/// This is the target-side handler for blob replication. When a source node
/// wants to replicate a blob to this node, it sends a BlobReplicatePull request.
/// This node then downloads the blob from the provider using iroh-blobs P2P.
pub(crate) async fn handle_blob_replicate_pull(
    ctx: &ClientProtocolContext,
    hash: String,
    size: u64,
    provider: String,
    tag: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    let Some(ref blob_store) = ctx.blob_store else {
        return Ok(ClientRpcResponse::BlobReplicatePullResult(BlobReplicatePullResultResponse {
            is_success: false,
            hash: None,
            size_bytes: None,
            duration_ms: None,
            error: Some("blob store not enabled".to_string()),
        }));
    };

    // Parse hash
    let hash = match hash.parse::<Hash>() {
        Ok(h) => h,
        Err(_) => {
            return Ok(ClientRpcResponse::BlobReplicatePullResult(BlobReplicatePullResultResponse {
                is_success: false,
                hash: None,
                size_bytes: None,
                duration_ms: None,
                error: Some("invalid hash format".to_string()),
            }));
        }
    };

    // Parse provider public key
    let provider_key = match provider.parse::<PublicKey>() {
        Ok(k) => k,
        Err(_) => {
            return Ok(ClientRpcResponse::BlobReplicatePullResult(BlobReplicatePullResultResponse {
                is_success: false,
                hash: Some(hash.to_string()),
                size_bytes: None,
                duration_ms: None,
                error: Some("invalid provider public key format".to_string()),
            }));
        }
    };

    // Check if we already have this blob
    match blob_store.has(&hash).await {
        Ok(true) => {
            info!(
                hash = %hash.fmt_short(),
                "blob already exists locally, skipping download"
            );
            return Ok(ClientRpcResponse::BlobReplicatePullResult(BlobReplicatePullResultResponse {
                is_success: true,
                hash: Some(hash.to_string()),
                size_bytes: Some(size),
                duration_ms: Some(0),
                error: None,
            }));
        }
        Ok(false) => {}
        Err(e) => {
            warn!(error = %e, "failed to check blob existence");
        }
    }

    // Download from provider
    let start = Instant::now();
    match blob_store.download_from_peer(&hash, provider_key).await {
        Ok(blob_ref) => {
            let duration_ms = start.elapsed().as_millis() as u64;

            info!(
                hash = %hash.fmt_short(),
                size = blob_ref.size_bytes,
                provider = %provider_key.fmt_short(),
                duration_ms,
                "blob replicated from peer"
            );

            // Apply protection tag if specified
            if let Some(ref tag_name) = tag {
                let user_tag = IrohBlobStore::user_tag(tag_name);
                if let Err(e) = blob_store.protect(&blob_ref.hash, &user_tag).await {
                    warn!(error = %e, "failed to apply tag to replicated blob");
                }
            }

            // Always apply a replication tag to prevent GC
            let replica_tag = format!("_replica:{}", hash.to_hex());
            if let Err(e) = blob_store.protect(&blob_ref.hash, &replica_tag).await {
                warn!(error = %e, "failed to apply replica tag");
            }

            Ok(ClientRpcResponse::BlobReplicatePullResult(BlobReplicatePullResultResponse {
                is_success: true,
                hash: Some(blob_ref.hash.to_string()),
                size_bytes: Some(blob_ref.size_bytes),
                duration_ms: Some(duration_ms),
                error: None,
            }))
        }
        Err(e) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            warn!(
                hash = %hash.fmt_short(),
                provider = %provider_key.fmt_short(),
                duration_ms,
                error = %e,
                "blob replication failed"
            );
            Ok(ClientRpcResponse::BlobReplicatePullResult(BlobReplicatePullResultResponse {
                is_success: false,
                hash: Some(hash.to_string()),
                size_bytes: None,
                duration_ms: Some(duration_ms),
                error: Some(sanitize_blob_error(&e)),
            }))
        }
    }
}

/// Handle GetBlobReplicationStatus request.
///
/// Returns the replication metadata for a blob including which nodes have
/// replicas, the policy, and health status.
pub(crate) async fn handle_get_blob_replication_status(
    ctx: &ClientProtocolContext,
    hash: String,
) -> anyhow::Result<ClientRpcResponse> {
    // Parse hash
    let hash = match hash.parse::<Hash>() {
        Ok(h) => h,
        Err(_) => {
            return Ok(ClientRpcResponse::GetBlobReplicationStatusResult(GetBlobReplicationStatusResultResponse {
                was_found: false,
                hash: None,
                size_bytes: None,
                replica_nodes: None,
                replication_factor: None,
                min_replicas: None,
                status: None,
                replicas_needed: None,
                updated_at: None,
                error: Some("invalid hash format".to_string()),
            }));
        }
    };

    // Read replica metadata from KV store
    let replica_key = format!("_system:blob:replica:{}", hash.to_hex());

    match ctx.kv_store.read(aspen_core::kv::ReadRequest::new(&replica_key)).await {
        Ok(result) => {
            if let Some(kv) = result.kv {
                // Parse the replica set JSON
                match serde_json::from_str::<serde_json::Value>(&kv.value) {
                    Ok(json) => {
                        let nodes = json
                            .get("nodes")
                            .and_then(|n| n.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_u64()).collect::<Vec<_>>());

                        let policy = json.get("policy");
                        let replication_factor =
                            policy.and_then(|p| p.get("replication_factor")).and_then(|f| f.as_u64()).map(|f| f as u32);
                        let min_replicas =
                            policy.and_then(|p| p.get("min_replicas")).and_then(|m| m.as_u64()).map(|m| m as u32);

                        let size = json.get("size").and_then(|s| s.as_u64());
                        let updated_at = json.get("updated_at").and_then(|u| u.as_str()).map(String::from);

                        // Calculate status
                        let node_count = nodes.as_ref().map(|n| n.len() as u32).unwrap_or(0);
                        let target = replication_factor.unwrap_or(3);
                        let min = min_replicas.unwrap_or(2);

                        let status = if node_count == 0 {
                            "critical"
                        } else if node_count < min {
                            "under_replicated"
                        } else if node_count < target {
                            "degraded"
                        } else if node_count == target {
                            "healthy"
                        } else {
                            "over_replicated"
                        };

                        let replicas_needed = if node_count < target {
                            Some(target - node_count)
                        } else {
                            Some(0)
                        };

                        Ok(ClientRpcResponse::GetBlobReplicationStatusResult(GetBlobReplicationStatusResultResponse {
                            was_found: true,
                            hash: Some(hash.to_string()),
                            size_bytes: size,
                            replica_nodes: nodes,
                            replication_factor,
                            min_replicas,
                            status: Some(status.to_string()),
                            replicas_needed,
                            updated_at,
                            error: None,
                        }))
                    }
                    Err(e) => {
                        Ok(ClientRpcResponse::GetBlobReplicationStatusResult(GetBlobReplicationStatusResultResponse {
                            was_found: false,
                            hash: Some(hash.to_string()),
                            size_bytes: None,
                            replica_nodes: None,
                            replication_factor: None,
                            min_replicas: None,
                            status: None,
                            replicas_needed: None,
                            updated_at: None,
                            error: Some(format!("failed to parse replica metadata: {}", e)),
                        }))
                    }
                }
            } else {
                // No replication metadata exists for this blob
                Ok(ClientRpcResponse::GetBlobReplicationStatusResult(GetBlobReplicationStatusResultResponse {
                    was_found: false,
                    hash: Some(hash.to_string()),
                    size_bytes: None,
                    replica_nodes: None,
                    replication_factor: None,
                    min_replicas: None,
                    status: None,
                    replicas_needed: None,
                    updated_at: None,
                    error: None,
                }))
            }
        }
        Err(e) => Ok(ClientRpcResponse::GetBlobReplicationStatusResult(GetBlobReplicationStatusResultResponse {
            was_found: false,
            hash: Some(hash.to_string()),
            size_bytes: None,
            replica_nodes: None,
            replication_factor: None,
            min_replicas: None,
            status: None,
            replicas_needed: None,
            updated_at: None,
            error: Some(format!("failed to read replica metadata: {}", e)),
        })),
    }
}

/// Handle TriggerBlobReplication request.
///
/// Manually triggers replication of a blob to additional nodes.
/// Uses the BlobReplicationManager to coordinate replication across the cluster.
pub(crate) async fn handle_trigger_blob_replication(
    ctx: &ClientProtocolContext,
    hash: String,
    target_nodes: Vec<u64>,
    _replication_factor: u32,
) -> anyhow::Result<ClientRpcResponse> {
    let start = Instant::now();

    // Parse hash
    let hash = match hash.parse::<Hash>() {
        Ok(h) => h,
        Err(_) => {
            return Ok(ClientRpcResponse::TriggerBlobReplicationResult(TriggerBlobReplicationResultResponse {
                is_success: false,
                hash: None,
                successful_nodes: None,
                failed_nodes: None,
                duration_ms: None,
                error: Some("invalid hash format".to_string()),
            }));
        }
    };

    // Check if blob exists locally and get its size
    let blob_size = match &ctx.blob_store {
        Some(blob_store) => {
            match blob_store.has(&hash).await {
                Ok(false) => {
                    return Ok(ClientRpcResponse::TriggerBlobReplicationResult(TriggerBlobReplicationResultResponse {
                        is_success: false,
                        hash: Some(hash.to_string()),
                        successful_nodes: None,
                        failed_nodes: None,
                        duration_ms: Some(start.elapsed().as_millis() as u64),
                        error: Some("blob not found locally".to_string()),
                    }));
                }
                Err(e) => {
                    return Ok(ClientRpcResponse::TriggerBlobReplicationResult(TriggerBlobReplicationResultResponse {
                        is_success: false,
                        hash: Some(hash.to_string()),
                        successful_nodes: None,
                        failed_nodes: None,
                        duration_ms: Some(start.elapsed().as_millis() as u64),
                        error: Some(format!("failed to check blob existence: {}", e)),
                    }));
                }
                Ok(true) => {
                    // Get the size via status() - required for ReplicationRequest
                    match blob_store.status(&hash).await {
                        Ok(Some(status)) => match status.size_bytes {
                            Some(size) => size,
                            None => {
                                return Ok(ClientRpcResponse::TriggerBlobReplicationResult(
                                    TriggerBlobReplicationResultResponse {
                                        is_success: false,
                                        hash: Some(hash.to_string()),
                                        successful_nodes: None,
                                        failed_nodes: None,
                                        duration_ms: Some(start.elapsed().as_millis() as u64),
                                        error: Some("blob exists but size unavailable".to_string()),
                                    },
                                ));
                            }
                        },
                        Ok(None) => {
                            return Ok(ClientRpcResponse::TriggerBlobReplicationResult(
                                TriggerBlobReplicationResultResponse {
                                    is_success: false,
                                    hash: Some(hash.to_string()),
                                    successful_nodes: None,
                                    failed_nodes: None,
                                    duration_ms: Some(start.elapsed().as_millis() as u64),
                                    error: Some("blob status unavailable".to_string()),
                                },
                            ));
                        }
                        Err(e) => {
                            return Ok(ClientRpcResponse::TriggerBlobReplicationResult(
                                TriggerBlobReplicationResultResponse {
                                    is_success: false,
                                    hash: Some(hash.to_string()),
                                    successful_nodes: None,
                                    failed_nodes: None,
                                    duration_ms: Some(start.elapsed().as_millis() as u64),
                                    error: Some(format!("failed to get blob status: {}", e)),
                                },
                            ));
                        }
                    }
                }
            }
        }
        None => {
            return Ok(ClientRpcResponse::TriggerBlobReplicationResult(TriggerBlobReplicationResultResponse {
                is_success: false,
                hash: Some(hash.to_string()),
                successful_nodes: None,
                failed_nodes: None,
                duration_ms: Some(start.elapsed().as_millis() as u64),
                error: Some("blob store not available".to_string()),
            }));
        }
    };

    // Check if replication manager is available
    let replication_manager = match &ctx.blob_replication_manager {
        Some(manager) => manager,
        None => {
            return Ok(ClientRpcResponse::TriggerBlobReplicationResult(TriggerBlobReplicationResultResponse {
                is_success: false,
                hash: Some(hash.to_string()),
                successful_nodes: None,
                failed_nodes: None,
                duration_ms: Some(start.elapsed().as_millis() as u64),
                error: Some("blob replication not enabled on this node".to_string()),
            }));
        }
    };

    // Create replication request
    // If target_nodes is empty, the placement strategy will select targets automatically
    let request = aspen_blob::ReplicationRequest::new(hash, blob_size, target_nodes).with_ack(true);

    // Trigger replication through the manager
    match replication_manager.replicate(request).await {
        Ok(result) => {
            let is_success = result.failed.is_empty();
            let failed_nodes: Option<Vec<(u64, String)>> = if result.failed.is_empty() {
                None
            } else {
                Some(result.failed)
            };

            info!(
                hash = %hash.to_hex(),
                successful_count = result.successful.len(),
                failed_count = failed_nodes.as_ref().map(|f| f.len()).unwrap_or(0),
                duration_ms = result.duration_ms,
                "blob replication triggered"
            );

            Ok(ClientRpcResponse::TriggerBlobReplicationResult(TriggerBlobReplicationResultResponse {
                is_success,
                hash: Some(hash.to_string()),
                successful_nodes: Some(result.successful),
                failed_nodes,
                duration_ms: Some(result.duration_ms),
                error: None,
            }))
        }
        Err(e) => {
            warn!(hash = %hash.to_hex(), error = %e, "blob replication failed");
            Ok(ClientRpcResponse::TriggerBlobReplicationResult(TriggerBlobReplicationResultResponse {
                is_success: false,
                hash: Some(hash.to_string()),
                successful_nodes: None,
                failed_nodes: None,
                duration_ms: Some(start.elapsed().as_millis() as u64),
                error: Some(format!("replication failed: {}", e)),
            }))
        }
    }
}

/// Handle RunBlobRepairCycle request.
///
/// Manually triggers a full repair cycle across all blobs in the cluster.
/// Scans for under-replicated blobs and repairs them in priority order:
/// 1. Critical (0 replicas)
/// 2. UnderReplicated (below min_replicas)
/// 3. Degraded (below replication_factor)
///
/// Returns immediately - repairs happen asynchronously in the background.
pub(crate) async fn handle_run_blob_repair_cycle(ctx: &ClientProtocolContext) -> anyhow::Result<ClientRpcResponse> {
    // Check if replication manager is available
    let replication_manager = match &ctx.blob_replication_manager {
        Some(manager) => manager,
        None => {
            return Ok(ClientRpcResponse::RunBlobRepairCycleResult(RunBlobRepairCycleResultResponse {
                is_success: false,
                error: Some("blob replication not enabled on this node".to_string()),
            }));
        }
    };

    // Trigger repair cycle (fire and forget)
    match replication_manager.run_repair_cycle().await {
        Ok(()) => {
            info!("blob repair cycle initiated via RPC");
            Ok(ClientRpcResponse::RunBlobRepairCycleResult(RunBlobRepairCycleResultResponse {
                is_success: true,
                error: None,
            }))
        }
        Err(e) => {
            warn!(error = %e, "failed to initiate blob repair cycle");
            Ok(ClientRpcResponse::RunBlobRepairCycleResult(RunBlobRepairCycleResultResponse {
                is_success: false,
                error: Some(format!("repair cycle initiation failed: {}", e)),
            }))
        }
    }
}
