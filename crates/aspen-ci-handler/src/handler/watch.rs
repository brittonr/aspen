//! Repository watch operations: watch, unwatch.

use std::sync::Arc;

use aspen_client_api::CiUnwatchRepoResponse;
use aspen_client_api::CiWatchRepoResponse;
use aspen_client_api::ClientRpcResponse;
use tracing::debug;
use tracing::info;
use tracing::warn;

/// Type alias for forge node to match executor.
pub type ForgeNodeRef = Arc<aspen_forge::ForgeNode<aspen_blob::IrohBlobStore, dyn aspen_core::KeyValueStore>>;

/// Handle CiWatchRepo request.
///
/// Subscribes to forge gossip events for automatic CI triggering.
/// This performs two operations:
/// 1. Registers the repo with TriggerService (in-memory watch list)
/// 2. Subscribes to the repo's gossip topic for multi-node announcements
#[cfg(feature = "forge")]
pub async fn handle_watch_repo(
    trigger_service: Option<&Arc<aspen_ci::TriggerService>>,
    forge_node: Option<&ForgeNodeRef>,
    repo_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_forge::identity::RepoId;

    let Some(trigger_service) = trigger_service else {
        return Ok(ClientRpcResponse::CiWatchRepoResult(CiWatchRepoResponse {
            is_success: false,
            error: Some("CI trigger service not available".to_string()),
        }));
    };

    info!(%repo_id, "watching repository for CI triggers");

    // Parse repo_id from hex string
    let repo_id_parsed = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::CiWatchRepoResult(CiWatchRepoResponse {
                is_success: false,
                error: Some(format!("Invalid repo_id: {}", e)),
            }));
        }
    };

    // Step 1: Register with TriggerService (in-memory watch list)
    if let Err(e) = trigger_service.watch_repo(repo_id_parsed).await {
        return Ok(ClientRpcResponse::CiWatchRepoResult(CiWatchRepoResponse {
            is_success: false,
            error: Some(format!("Failed to watch repository: {}", e)),
        }));
    }

    // Step 2: Subscribe to gossip topic for multi-node announcements
    #[cfg(feature = "forge")]
    if let Some(forge_node) = forge_node {
        if let Err(e) = forge_node.subscribe_repo_gossip(&repo_id_parsed).await {
            warn!(
                repo_id = %repo_id,
                error = %e,
                "Failed to subscribe to repo gossip (multi-node triggers may not work)"
            );
            // Continue - local triggers will still work
        } else {
            info!(repo_id = %repo_id, "subscribed to repo gossip for CI triggers");
        }
    }

    // Verify and log the watch status
    let is_watching = trigger_service.is_watching(&repo_id_parsed).await;
    let watched_count = trigger_service.watched_count().await;
    info!(
        repo_id = %repo_id,
        is_watching = is_watching,
        watched_count = watched_count,
        "CI watch registered and verified"
    );

    Ok(ClientRpcResponse::CiWatchRepoResult(CiWatchRepoResponse {
        is_success: true,
        error: None,
    }))
}

/// Handle CiUnwatchRepo request.
///
/// Removes CI trigger subscription for a repository.
/// This performs two operations:
/// 1. Unregisters the repo from TriggerService
/// 2. Unsubscribes from the repo's gossip topic
#[cfg(feature = "forge")]
pub async fn handle_unwatch_repo(
    trigger_service: Option<&Arc<aspen_ci::TriggerService>>,
    forge_node: Option<&ForgeNodeRef>,
    repo_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_forge::identity::RepoId;

    let Some(trigger_service) = trigger_service else {
        return Ok(ClientRpcResponse::CiUnwatchRepoResult(CiUnwatchRepoResponse {
            is_success: false,
            error: Some("CI trigger service not available".to_string()),
        }));
    };

    info!(%repo_id, "unwatching repository");

    // Parse repo_id from hex string
    let repo_id_parsed = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::CiUnwatchRepoResult(CiUnwatchRepoResponse {
                is_success: false,
                error: Some(format!("Invalid repo_id: {}", e)),
            }));
        }
    };

    // Step 1: Unregister from TriggerService
    trigger_service.unwatch_repo(&repo_id_parsed).await;

    // Step 2: Unsubscribe from gossip topic
    #[cfg(feature = "forge")]
    if let Some(forge_node) = forge_node {
        if let Err(e) = forge_node.unsubscribe_repo_gossip(&repo_id_parsed).await {
            warn!(
                repo_id = %repo_id,
                error = %e,
                "Failed to unsubscribe from repo gossip"
            );
            // Continue - the watch is already removed
        } else {
            debug!(repo_id = %repo_id, "unsubscribed from repo gossip");
        }
    }

    info!(repo_id = %repo_id, "CI watch removed");

    Ok(ClientRpcResponse::CiUnwatchRepoResult(CiUnwatchRepoResponse {
        is_success: true,
        error: None,
    }))
}
