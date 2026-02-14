//! Nix binary cache RPC handlers.
//!
//! Handles cache queries, statistics, and download ticket generation
//! for the distributed Nix binary cache.

use aspen_blob::prelude::*;
use aspen_cache::CacheEntry;
use aspen_cache::CacheIndex;
use aspen_cache::KvCacheIndex;
use aspen_client_api::CacheDownloadResultResponse;
use aspen_client_api::CacheEntryResponse;
use aspen_client_api::CacheQueryResultResponse;
use aspen_client_api::CacheStatsResultResponse;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::RequestHandler;
use async_trait::async_trait;
use tracing::debug;
use tracing::warn;

/// Handler for Nix binary cache operations.
///
/// Processes cache RPC requests for querying cache entries,
/// fetching statistics, and obtaining download tickets.
///
/// # Tiger Style
///
/// - Read-only operations (cache is populated by CI only)
/// - Bounded query results
/// - Clear error reporting for missing entries
pub struct CacheHandler;

#[async_trait]
impl RequestHandler for CacheHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::CacheQuery { .. } | ClientRpcRequest::CacheStats | ClientRpcRequest::CacheDownload { .. }
        )
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::CacheQuery { store_hash } => handle_cache_query(ctx, store_hash).await,
            ClientRpcRequest::CacheStats => handle_cache_stats(ctx).await,
            ClientRpcRequest::CacheDownload { store_hash } => handle_cache_download(ctx, store_hash).await,
            _ => Err(anyhow::anyhow!("request not handled by CacheHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "CacheHandler"
    }
}

/// Handle CacheQuery request.
///
/// Queries the cache for a store path by its hash.
async fn handle_cache_query(ctx: &ClientProtocolContext, store_hash: String) -> anyhow::Result<ClientRpcResponse> {
    debug!(store_hash = %store_hash, "querying cache for store path");

    // Validate store hash format (32-64 lowercase alphanumeric)
    if !is_valid_store_hash(&store_hash) {
        return Ok(ClientRpcResponse::CacheQueryResult(CacheQueryResultResponse {
            found: false,
            entry: None,
            error: Some("Invalid store hash format".to_string()),
        }));
    }

    // Create cache index from KV store
    let cache_index = KvCacheIndex::new(ctx.kv_store.clone());

    match cache_index.get(&store_hash).await {
        Ok(Some(entry)) => {
            debug!(store_hash = %store_hash, blob_hash = %entry.blob_hash, "cache hit");
            Ok(ClientRpcResponse::CacheQueryResult(CacheQueryResultResponse {
                found: true,
                entry: Some(cache_entry_to_response(&entry)),
                error: None,
            }))
        }
        Ok(None) => {
            debug!(store_hash = %store_hash, "cache miss");
            Ok(ClientRpcResponse::CacheQueryResult(CacheQueryResultResponse {
                found: false,
                entry: None,
                error: None,
            }))
        }
        Err(e) => {
            warn!(store_hash = %store_hash, error = %e, "cache query failed");
            Ok(ClientRpcResponse::CacheQueryResult(CacheQueryResultResponse {
                found: false,
                entry: None,
                error: Some(format!("Cache query failed: {}", e)),
            }))
        }
    }
}

/// Handle CacheStats request.
///
/// Returns cache statistics including entry count and storage usage.
async fn handle_cache_stats(ctx: &ClientProtocolContext) -> anyhow::Result<ClientRpcResponse> {
    debug!("fetching cache statistics");

    // Create cache index from KV store
    let cache_index = KvCacheIndex::new(ctx.kv_store.clone());

    match cache_index.stats().await {
        Ok(stats) => Ok(ClientRpcResponse::CacheStatsResult(CacheStatsResultResponse {
            total_entries: stats.total_entries,
            total_nar_bytes: stats.total_nar_bytes,
            query_hits: stats.hit_count,
            query_misses: stats.miss_count,
            node_id: ctx.node_id,
            error: None,
        })),
        Err(e) => {
            // Stats might not exist yet - return zeros
            warn!(error = %e, "cache stats query failed, returning zeros");
            Ok(ClientRpcResponse::CacheStatsResult(CacheStatsResultResponse {
                total_entries: 0,
                total_nar_bytes: 0,
                query_hits: 0,
                query_misses: 0,
                node_id: ctx.node_id,
                error: None,
            }))
        }
    }
}

/// Handle CacheDownload request.
///
/// Returns a blob ticket for downloading a NAR from the cache.
async fn handle_cache_download(ctx: &ClientProtocolContext, store_hash: String) -> anyhow::Result<ClientRpcResponse> {
    debug!(store_hash = %store_hash, "generating cache download ticket");

    // Validate store hash format
    if !is_valid_store_hash(&store_hash) {
        return Ok(ClientRpcResponse::CacheDownloadResult(CacheDownloadResultResponse {
            found: false,
            blob_ticket: None,
            blob_hash: None,
            nar_size: None,
            error: Some("Invalid store hash format".to_string()),
        }));
    }

    // Create cache index from KV store
    let cache_index = KvCacheIndex::new(ctx.kv_store.clone());

    // Look up the cache entry
    let entry = match cache_index.get(&store_hash).await {
        Ok(Some(e)) => e,
        Ok(None) => {
            return Ok(ClientRpcResponse::CacheDownloadResult(CacheDownloadResultResponse {
                found: false,
                blob_ticket: None,
                blob_hash: None,
                nar_size: None,
                error: None,
            }));
        }
        Err(e) => {
            return Ok(ClientRpcResponse::CacheDownloadResult(CacheDownloadResultResponse {
                found: false,
                blob_ticket: None,
                blob_hash: None,
                nar_size: None,
                error: Some(format!("Cache lookup failed: {}", e)),
            }));
        }
    };

    // Get blob store for ticket generation
    let Some(blob_store) = &ctx.blob_store else {
        return Ok(ClientRpcResponse::CacheDownloadResult(CacheDownloadResultResponse {
            found: true,
            blob_ticket: None,
            blob_hash: Some(entry.blob_hash.clone()),
            nar_size: Some(entry.nar_size),
            error: Some("Blob store not available".to_string()),
        }));
    };

    // Parse blob hash
    let hash: iroh_blobs::Hash = match entry.blob_hash.parse() {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::CacheDownloadResult(CacheDownloadResultResponse {
                found: true,
                blob_ticket: None,
                blob_hash: Some(entry.blob_hash.clone()),
                nar_size: Some(entry.nar_size),
                error: Some(format!("Invalid blob hash: {}", e)),
            }));
        }
    };

    // Generate blob ticket
    match blob_store.ticket(&hash).await {
        Ok(ticket) => {
            let ticket_str = ticket.to_string();
            Ok(ClientRpcResponse::CacheDownloadResult(CacheDownloadResultResponse {
                found: true,
                blob_ticket: Some(ticket_str),
                blob_hash: Some(entry.blob_hash),
                nar_size: Some(entry.nar_size),
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::CacheDownloadResult(CacheDownloadResultResponse {
            found: true,
            blob_ticket: None,
            blob_hash: Some(entry.blob_hash),
            nar_size: Some(entry.nar_size),
            error: Some(format!("Failed to generate ticket: {}", e)),
        })),
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Validate store hash format.
///
/// Store hashes should be 32-64 lowercase alphanumeric characters.
fn is_valid_store_hash(hash: &str) -> bool {
    let len = hash.len();
    (32..=64).contains(&len) && hash.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
}

/// Convert internal CacheEntry to RPC response type.
fn cache_entry_to_response(entry: &CacheEntry) -> CacheEntryResponse {
    CacheEntryResponse {
        store_path: entry.store_path.clone(),
        store_hash: entry.store_hash.clone(),
        blob_hash: entry.blob_hash.clone(),
        nar_size: entry.nar_size,
        nar_hash: entry.nar_hash.clone(),
        file_size: entry.file_size,
        references: entry.references.clone(),
        deriver: entry.deriver.clone(),
        created_at_ms: entry.created_at,
        created_by_node: entry.created_by_node,
        ci_job_id: entry.ci_job_id.clone(),
        ci_run_id: entry.ci_run_id.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_store_hash() {
        // Valid hashes
        assert!(is_valid_store_hash("abcdefghijklmnopqrstuvwxyz012345")); // 32 chars
        assert!(is_valid_store_hash(&"a".repeat(64))); // 64 chars

        // Invalid hashes
        assert!(!is_valid_store_hash("abc123def456")); // 12 chars - too short
        assert!(!is_valid_store_hash("ABC123")); // uppercase
        assert!(!is_valid_store_hash("abc-123")); // hyphen
        assert!(!is_valid_store_hash("")); // empty
    }

    #[test]
    fn test_valid_store_hash_boundaries() {
        // Exactly 32 chars - valid
        assert!(is_valid_store_hash(&"a".repeat(32)));
        // 31 chars - invalid
        assert!(!is_valid_store_hash(&"a".repeat(31)));
        // 64 chars - valid
        assert!(is_valid_store_hash(&"a".repeat(64)));
        // 65 chars - invalid
        assert!(!is_valid_store_hash(&"a".repeat(65)));
    }
}
