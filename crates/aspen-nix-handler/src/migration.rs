//! Cache migration RPC handlers.
//!
//! Handles SNIX storage migration operations for transitioning from
//! legacy CacheEntry format to SNIX PathInfo format.
//!
//! # Operations
//!
//! - `CacheMigrationStart`: Start background migration
//! - `CacheMigrationStatus`: Get current migration progress
//! - `CacheMigrationCancel`: Cancel running migration
//! - `CacheMigrationValidate`: Validate migration completeness

use aspen_cache::CacheIndex;
use aspen_cache::KvCacheIndex;
use aspen_client_api::CacheMigrationCancelResultResponse;
use aspen_client_api::CacheMigrationProgressResponse;
use aspen_client_api::CacheMigrationStartResultResponse;
use aspen_client_api::CacheMigrationStatusResultResponse;
use aspen_client_api::CacheMigrationValidateResultResponse;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_core::KeyValueStore;
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::RequestHandler;
use async_trait::async_trait;
use tracing::debug;

/// Key prefix for migration progress in KV store.
const MIGRATION_PROGRESS_KEY: &str = "snix:migration:current";

/// Handler for cache migration operations.
///
/// Manages migration from legacy CacheEntry format to SNIX PathInfo format.
///
/// # Tiger Style
///
/// - Admin-only operations (requires ClusterAdmin permission)
/// - Progress tracked in KV store for persistence
/// - Bounded batch sizes to avoid resource exhaustion
pub struct CacheMigrationHandler;

#[async_trait]
impl RequestHandler for CacheMigrationHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::CacheMigrationStart { .. }
                | ClientRpcRequest::CacheMigrationStatus
                | ClientRpcRequest::CacheMigrationCancel
                | ClientRpcRequest::CacheMigrationValidate { .. }
        )
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::CacheMigrationStart {
                batch_size,
                batch_delay_ms,
                dry_run,
            } => handle_migration_start(ctx, batch_size, batch_delay_ms, dry_run).await,
            ClientRpcRequest::CacheMigrationStatus => handle_migration_status(ctx).await,
            ClientRpcRequest::CacheMigrationCancel => handle_migration_cancel(ctx).await,
            ClientRpcRequest::CacheMigrationValidate { max_report } => handle_migration_validate(ctx, max_report).await,
            _ => Err(anyhow::anyhow!("request not handled by CacheMigrationHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "CacheMigrationHandler"
    }
}

/// Handle CacheMigrationStart request.
///
/// Initiates migration from legacy cache to SNIX format.
async fn handle_migration_start(
    ctx: &ClientProtocolContext,
    batch_size: Option<u32>,
    batch_delay_ms: Option<u64>,
    dry_run: bool,
) -> anyhow::Result<ClientRpcResponse> {
    let batch_size = batch_size.unwrap_or(50);
    let batch_delay_ms = batch_delay_ms.unwrap_or(100);

    debug!(
        batch_size = batch_size,
        batch_delay_ms = batch_delay_ms,
        dry_run = dry_run,
        "starting cache migration"
    );

    // Check if migration is already running
    if let Some(progress) = load_progress(&ctx.kv_store).await?
        && !progress.is_complete
    {
        return Ok(ClientRpcResponse::CacheMigrationStartResult(CacheMigrationStartResultResponse {
            started: false,
            status: Some(progress_to_response(&progress)),
            error: Some("Migration already in progress".to_string()),
        }));
    }

    // Get legacy cache stats to initialize progress
    let cache_index = KvCacheIndex::new(ctx.kv_store.clone());
    let stats = cache_index.stats().await.map_err(|e| anyhow::anyhow!("Failed to get cache stats: {}", e))?;

    // Create new progress
    let progress = MigrationProgress {
        total_entries: stats.total_entries,
        migrated_count: 0,
        failed_count: 0,
        skipped_count: 0,
        started_at: current_timestamp(),
        last_updated: current_timestamp(),
        last_processed_hash: None,
        is_complete: false,
        error_message: None,
    };

    if !dry_run {
        // Save initial progress
        save_progress(&ctx.kv_store, &progress).await?;
    }

    // Note: The actual migration worker is started separately.
    // This RPC just initializes the progress tracking.
    // The worker would be started by the node's initialization code.

    Ok(ClientRpcResponse::CacheMigrationStartResult(CacheMigrationStartResultResponse {
        started: true,
        status: Some(progress_to_response(&progress)),
        error: None,
    }))
}

/// Handle CacheMigrationStatus request.
///
/// Returns current migration progress.
async fn handle_migration_status(ctx: &ClientProtocolContext) -> anyhow::Result<ClientRpcResponse> {
    debug!("getting migration status");

    match load_progress(&ctx.kv_store).await? {
        Some(progress) => Ok(ClientRpcResponse::CacheMigrationStatusResult(CacheMigrationStatusResultResponse {
            is_running: !progress.is_complete,
            progress: Some(progress_to_response(&progress)),
            error: None,
        })),
        None => Ok(ClientRpcResponse::CacheMigrationStatusResult(CacheMigrationStatusResultResponse {
            is_running: false,
            progress: None,
            error: None,
        })),
    }
}

/// Handle CacheMigrationCancel request.
///
/// Signals the migration worker to stop.
async fn handle_migration_cancel(ctx: &ClientProtocolContext) -> anyhow::Result<ClientRpcResponse> {
    debug!("cancelling migration");

    // Set a cancellation flag in the progress
    match load_progress(&ctx.kv_store).await? {
        Some(mut progress) => {
            if progress.is_complete {
                return Ok(ClientRpcResponse::CacheMigrationCancelResult(CacheMigrationCancelResultResponse {
                    cancelled: false,
                    error: Some("Migration already complete".to_string()),
                }));
            }

            progress.is_complete = true;
            progress.error_message = Some("Cancelled by user".to_string());
            progress.last_updated = current_timestamp();
            save_progress(&ctx.kv_store, &progress).await?;

            Ok(ClientRpcResponse::CacheMigrationCancelResult(CacheMigrationCancelResultResponse {
                cancelled: true,
                error: None,
            }))
        }
        None => Ok(ClientRpcResponse::CacheMigrationCancelResult(CacheMigrationCancelResultResponse {
            cancelled: false,
            error: Some("No migration in progress".to_string()),
        })),
    }
}

/// Handle CacheMigrationValidate request.
///
/// Validates that all legacy entries have been migrated.
async fn handle_migration_validate(
    ctx: &ClientProtocolContext,
    max_report: Option<u32>,
) -> anyhow::Result<ClientRpcResponse> {
    let max_report = max_report.unwrap_or(100) as usize;

    debug!(max_report = max_report, "validating migration");

    // For now, just report based on progress tracking.
    // A full validation would scan the legacy cache and check SNIX for each entry.
    match load_progress(&ctx.kv_store).await? {
        Some(progress) => {
            let is_complete = progress.is_complete && progress.failed_count == 0;
            let missing_count = progress.failed_count
                + (progress.total_entries - progress.migrated_count - progress.skipped_count - progress.failed_count);

            Ok(ClientRpcResponse::CacheMigrationValidateResult(CacheMigrationValidateResultResponse {
                is_complete,
                validated_count: progress.migrated_count + progress.skipped_count,
                missing_count,
                missing_hashes: vec![], // Would need full scan to populate
                error: None,
            }))
        }
        None => Ok(ClientRpcResponse::CacheMigrationValidateResult(CacheMigrationValidateResultResponse {
            is_complete: false,
            validated_count: 0,
            missing_count: 0,
            missing_hashes: vec![],
            error: Some("No migration progress found".to_string()),
        })),
    }
}

// ============================================================================
// Helper Types and Functions
// ============================================================================

/// Internal migration progress structure.
///
/// Matches the format used in aspen-snix migration module.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct MigrationProgress {
    total_entries: u64,
    migrated_count: u64,
    failed_count: u64,
    skipped_count: u64,
    started_at: u64,
    last_updated: u64,
    last_processed_hash: Option<String>,
    is_complete: bool,
    error_message: Option<String>,
}

/// Load migration progress from KV store.
async fn load_progress(kv: &std::sync::Arc<dyn KeyValueStore>) -> anyhow::Result<Option<MigrationProgress>> {
    let result = kv
        .read(aspen_core::kv::ReadRequest::new(MIGRATION_PROGRESS_KEY))
        .await
        .map_err(|e| anyhow::anyhow!("KV read error: {}", e))?;

    match result.kv {
        Some(kv) => {
            let progress: MigrationProgress =
                serde_json::from_str(&kv.value).map_err(|e| anyhow::anyhow!("Failed to parse progress: {}", e))?;
            Ok(Some(progress))
        }
        None => Ok(None),
    }
}

/// Save migration progress to KV store.
async fn save_progress(kv: &std::sync::Arc<dyn KeyValueStore>, progress: &MigrationProgress) -> anyhow::Result<()> {
    let value = serde_json::to_string(progress).map_err(|e| anyhow::anyhow!("Failed to serialize progress: {}", e))?;

    kv.write(aspen_core::kv::WriteRequest {
        command: aspen_core::kv::WriteCommand::Set {
            key: MIGRATION_PROGRESS_KEY.to_string(),
            value,
        },
    })
    .await
    .map_err(|e| anyhow::anyhow!("KV write error: {}", e))?;

    Ok(())
}

/// Convert internal progress to RPC response format.
fn progress_to_response(progress: &MigrationProgress) -> CacheMigrationProgressResponse {
    let progress_percent = if progress.total_entries == 0 {
        100.0
    } else {
        let processed = progress.migrated_count + progress.failed_count + progress.skipped_count;
        (processed as f64 / progress.total_entries as f64) * 100.0
    };

    CacheMigrationProgressResponse {
        total_entries: progress.total_entries,
        migrated_count: progress.migrated_count,
        failed_count: progress.failed_count,
        skipped_count: progress.skipped_count,
        started_at: progress.started_at,
        last_updated: progress.last_updated,
        last_processed_hash: progress.last_processed_hash.clone(),
        is_complete: progress.is_complete,
        progress_percent,
        error_message: progress.error_message.clone(),
    }
}

/// Get current Unix timestamp in seconds.
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .inspect_err(|e| tracing::error!("system time error: {e}"))
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_handle_migration_requests() {
        let handler = CacheMigrationHandler;

        assert!(handler.can_handle(&ClientRpcRequest::CacheMigrationStart {
            batch_size: None,
            batch_delay_ms: None,
            dry_run: false,
        }));
        assert!(handler.can_handle(&ClientRpcRequest::CacheMigrationStatus));
        assert!(handler.can_handle(&ClientRpcRequest::CacheMigrationCancel));
        assert!(handler.can_handle(&ClientRpcRequest::CacheMigrationValidate { max_report: None }));
    }

    #[test]
    fn test_progress_to_response() {
        let progress = MigrationProgress {
            total_entries: 100,
            migrated_count: 50,
            failed_count: 5,
            skipped_count: 10,
            started_at: 1000,
            last_updated: 2000,
            last_processed_hash: Some("abc123".to_string()),
            is_complete: false,
            error_message: None,
        };

        let response = progress_to_response(&progress);

        assert_eq!(response.total_entries, 100);
        assert_eq!(response.migrated_count, 50);
        assert_eq!(response.failed_count, 5);
        assert_eq!(response.skipped_count, 10);
        assert_eq!(response.progress_percent, 65.0);
        assert!(!response.is_complete);
    }

    #[test]
    fn test_progress_percent_empty() {
        let progress = MigrationProgress {
            total_entries: 0,
            migrated_count: 0,
            failed_count: 0,
            skipped_count: 0,
            started_at: 0,
            last_updated: 0,
            last_processed_hash: None,
            is_complete: true,
            error_message: None,
        };

        let response = progress_to_response(&progress);
        assert_eq!(response.progress_percent, 100.0);
    }
}
