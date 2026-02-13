//! Change operations: apply, unrecord, log, show, blame.

use aspen_client_api::ClientRpcResponse;

use super::PijulStoreRef;

/// Handle PijulApply request.
pub(crate) async fn handle_apply(
    pijul_store: &PijulStoreRef,
    repo_id: String,
    channel: String,
    change_hash: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ErrorResponse;
    use aspen_client_api::PijulApplyResponse;
    use aspen_forge::identity::RepoId;
    use aspen_pijul::types::ChangeHash;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::Error(ErrorResponse {
                code: "INVALID_REPO_ID".to_string(),
                message: format!("Invalid repo ID: {}", e),
            }));
        }
    };

    let hash = match ChangeHash::from_hex(&change_hash) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::Error(ErrorResponse {
                code: "INVALID_CHANGE_HASH".to_string(),
                message: format!("Invalid change hash: {}", e),
            }));
        }
    };

    match pijul_store.apply_change(&repo_id, &channel, &hash).await {
        Ok(result) => Ok(ClientRpcResponse::PijulApplyResult(PijulApplyResponse {
            operations: result.changes_applied,
        })),
        Err(e) => Ok(ClientRpcResponse::Error(ErrorResponse {
            code: "APPLY_FAILED".to_string(),
            message: format!("{}", e),
        })),
    }
}

/// Handle PijulUnrecord request.
pub(crate) async fn handle_unrecord(
    pijul_store: &PijulStoreRef,
    repo_id: String,
    channel: String,
    change_hash: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ErrorResponse;
    use aspen_client_api::PijulUnrecordResponse;
    use aspen_forge::identity::RepoId;
    use aspen_pijul::types::ChangeHash;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::Error(ErrorResponse {
                code: "INVALID_REPO_ID".to_string(),
                message: format!("Invalid repo ID: {}", e),
            }));
        }
    };

    let hash = match ChangeHash::from_hex(&change_hash) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::Error(ErrorResponse {
                code: "INVALID_CHANGE_HASH".to_string(),
                message: format!("Invalid change hash: {}", e),
            }));
        }
    };

    match pijul_store.unrecord_change(&repo_id, &channel, &hash).await {
        Ok(unrecorded) => Ok(ClientRpcResponse::PijulUnrecordResult(PijulUnrecordResponse { unrecorded })),
        Err(e) => Ok(ClientRpcResponse::Error(ErrorResponse {
            code: "UNRECORD_FAILED".to_string(),
            message: format!("{}", e),
        })),
    }
}

/// Handle PijulLog request.
pub(crate) async fn handle_log(
    pijul_store: &PijulStoreRef,
    repo_id: String,
    channel: String,
    limit: u32,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ErrorResponse;
    use aspen_client_api::PijulLogEntry;
    use aspen_client_api::PijulLogResponse;
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

    // Tiger Style: cap limit to 1000
    let limit = limit.min(1000);

    match pijul_store.get_change_log(&repo_id, &channel, limit).await {
        Ok(metadata_list) => {
            let entries: Vec<PijulLogEntry> = metadata_list
                .into_iter()
                .map(|m| PijulLogEntry {
                    change_hash: m.hash.to_hex(),
                    message: m.message,
                    author: m.authors.first().map(|a| {
                        if let Some(email) = &a.email {
                            format!("{} <{}>", a.name, email)
                        } else {
                            a.name.clone()
                        }
                    }),
                    timestamp_ms: m.recorded_at_ms,
                })
                .collect();

            let count = entries.len() as u32;
            Ok(ClientRpcResponse::PijulLogResult(PijulLogResponse { entries, count }))
        }
        Err(e) => Ok(ClientRpcResponse::Error(ErrorResponse {
            code: "LOG_FAILED".to_string(),
            message: format!("{}", e),
        })),
    }
}

/// Handle PijulShow request.
pub(crate) async fn handle_show(
    pijul_store: &PijulStoreRef,
    repo_id: String,
    change_hash: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ErrorResponse;
    use aspen_client_api::PijulAuthorInfo;
    use aspen_client_api::PijulShowResponse;
    use aspen_forge::identity::RepoId;
    use aspen_pijul::types::ChangeHash;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::Error(ErrorResponse {
                code: "INVALID_REPO_ID".to_string(),
                message: format!("Invalid repo ID: {}", e),
            }));
        }
    };

    let hash = match ChangeHash::from_hex(&change_hash) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::Error(ErrorResponse {
                code: "INVALID_CHANGE_HASH".to_string(),
                message: format!("Invalid change hash: {}", e),
            }));
        }
    };

    match pijul_store.get_change_metadata(&repo_id, &hash).await {
        Ok(Some(metadata)) => Ok(ClientRpcResponse::PijulShowResult(PijulShowResponse {
            change_hash: metadata.hash.to_hex(),
            repo_id: metadata.repo_id.to_hex(),
            channel: metadata.channel,
            message: metadata.message,
            authors: metadata
                .authors
                .into_iter()
                .map(|a| PijulAuthorInfo {
                    name: a.name,
                    email: a.email,
                })
                .collect(),
            dependencies: metadata.dependencies.into_iter().map(|d| d.to_hex()).collect(),
            size_bytes: metadata.size_bytes,
            recorded_at_ms: metadata.recorded_at_ms,
        })),
        Ok(None) => Ok(ClientRpcResponse::Error(ErrorResponse {
            code: "CHANGE_NOT_FOUND".to_string(),
            message: format!("Change '{}' not found", change_hash),
        })),
        Err(e) => Ok(ClientRpcResponse::Error(ErrorResponse {
            code: "SHOW_FAILED".to_string(),
            message: format!("{}", e),
        })),
    }
}

/// Handle PijulBlame request.
pub(crate) async fn handle_blame(
    pijul_store: &PijulStoreRef,
    repo_id: String,
    channel: String,
    path: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ErrorResponse;
    use aspen_client_api::PijulBlameEntry;
    use aspen_client_api::PijulBlameResponse;
    use aspen_forge::identity::RepoId;

    let repo_id_parsed = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::Error(ErrorResponse {
                code: "INVALID_REPO_ID".to_string(),
                message: format!("Invalid repo ID: {}", e),
            }));
        }
    };

    // Get blame information for the file
    // Currently returns change-level attribution. Per-line blame requires
    // additional libpijul graph traversal.
    match pijul_store.blame_file(&repo_id_parsed, &channel, &path).await {
        Ok(result) => {
            let attributions = result
                .attributions
                .into_iter()
                .map(|attr| PijulBlameEntry {
                    change_hash: attr.change_hash.to_hex(),
                    author: attr.author,
                    author_email: attr.author_email,
                    message: attr.message,
                    recorded_at_ms: attr.recorded_at_ms,
                    change_type: attr.change_type,
                })
                .collect();

            Ok(ClientRpcResponse::PijulBlameResult(PijulBlameResponse {
                path: result.path,
                channel: result.channel,
                repo_id,
                attributions,
                file_exists: result.file_exists,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::Error(ErrorResponse {
            code: "BLAME_FAILED".to_string(),
            message: format!("{}", e),
        })),
    }
}
