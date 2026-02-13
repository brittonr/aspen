//! Channel operations: channel_list, channel_create, channel_delete, channel_fork, channel_info.

use aspen_client_api::ClientRpcResponse;

use super::PijulStoreRef;
use super::helpers::current_time_ms;

/// Handle PijulChannelList request.
pub(crate) async fn handle_channel_list(
    pijul_store: &PijulStoreRef,
    repo_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ErrorResponse;
    use aspen_client_api::PijulChannelListResponse;
    use aspen_client_api::PijulChannelResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::Error(ErrorResponse {
                code: "INVALID_REPO_ID".to_string(),
                message: format!("Invalid repo ID: {}", e),
            }));
        }
    };

    match pijul_store.list_channels(&repo_id).await {
        Ok(channels) => {
            let count = channels.len() as u32;
            let response_channels: Vec<_> = channels
                .into_iter()
                .map(|ch| PijulChannelResponse {
                    name: ch.name,
                    head: ch.head.map(|h| h.to_hex()),
                    updated_at_ms: ch.updated_at_ms,
                })
                .collect();

            Ok(ClientRpcResponse::PijulChannelListResult(PijulChannelListResponse {
                channels: response_channels,
                count,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::Error(ErrorResponse {
            code: "CHANNEL_LIST_FAILED".to_string(),
            message: format!("{}", e),
        })),
    }
}

/// Handle PijulChannelCreate request.
pub(crate) async fn handle_channel_create(
    pijul_store: &PijulStoreRef,
    repo_id: String,
    name: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ErrorResponse;
    use aspen_client_api::PijulChannelResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::Error(ErrorResponse {
                code: "INVALID_REPO_ID".to_string(),
                message: format!("Invalid repo ID: {}", e),
            }));
        }
    };

    match pijul_store.create_channel(&repo_id, &name).await {
        Ok(()) => Ok(ClientRpcResponse::PijulChannelResult(PijulChannelResponse {
            name,
            head: None,
            updated_at_ms: current_time_ms(),
        })),
        Err(e) => Ok(ClientRpcResponse::Error(ErrorResponse {
            code: "CHANNEL_CREATE_FAILED".to_string(),
            message: format!("{}", e),
        })),
    }
}

/// Handle PijulChannelDelete request.
pub(crate) async fn handle_channel_delete(
    pijul_store: &PijulStoreRef,
    repo_id: String,
    name: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ErrorResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::Error(ErrorResponse {
                code: "INVALID_REPO_ID".to_string(),
                message: format!("Invalid repo ID: {}", e),
            }));
        }
    };

    match pijul_store.delete_channel(&repo_id, &name).await {
        Ok(()) => Ok(ClientRpcResponse::PijulSuccess),
        Err(e) => Ok(ClientRpcResponse::Error(ErrorResponse {
            code: "CHANNEL_DELETE_FAILED".to_string(),
            message: format!("{}", e),
        })),
    }
}

/// Handle PijulChannelFork request.
pub(crate) async fn handle_channel_fork(
    pijul_store: &PijulStoreRef,
    repo_id: String,
    source: String,
    target: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ErrorResponse;
    use aspen_client_api::PijulChannelResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::Error(ErrorResponse {
                code: "INVALID_REPO_ID".to_string(),
                message: format!("Invalid repo ID: {}", e),
            }));
        }
    };

    match pijul_store.fork_channel(&repo_id, &source, &target).await {
        Ok(channel) => Ok(ClientRpcResponse::PijulChannelResult(PijulChannelResponse {
            name: channel.name,
            head: channel.head.map(|h| h.to_hex()),
            updated_at_ms: channel.updated_at_ms,
        })),
        Err(e) => Ok(ClientRpcResponse::Error(ErrorResponse {
            code: "CHANNEL_FORK_FAILED".to_string(),
            message: format!("{}", e),
        })),
    }
}

/// Handle PijulChannelInfo request.
pub(crate) async fn handle_channel_info(
    pijul_store: &PijulStoreRef,
    repo_id: String,
    name: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ErrorResponse;
    use aspen_client_api::PijulChannelResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::Error(ErrorResponse {
                code: "INVALID_REPO_ID".to_string(),
                message: format!("Invalid repo ID: {}", e),
            }));
        }
    };

    match pijul_store.get_channel(&repo_id, &name).await {
        Ok(Some(channel)) => Ok(ClientRpcResponse::PijulChannelResult(PijulChannelResponse {
            name: channel.name,
            head: channel.head.map(|h| h.to_hex()),
            updated_at_ms: channel.updated_at_ms,
        })),
        Ok(None) => Ok(ClientRpcResponse::Error(ErrorResponse {
            code: "CHANNEL_NOT_FOUND".to_string(),
            message: format!("Channel '{}' not found", name),
        })),
        Err(e) => Ok(ClientRpcResponse::Error(ErrorResponse {
            code: "CHANNEL_INFO_FAILED".to_string(),
            message: format!("{}", e),
        })),
    }
}
