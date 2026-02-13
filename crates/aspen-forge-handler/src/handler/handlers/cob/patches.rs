//! Patch tracking operations.

use aspen_client_api::ClientRpcResponse;

use crate::handler::handlers::ForgeNodeRef;

pub(crate) async fn handle_create_patch(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    title: String,
    description: String,
    base: String,
    head: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgePatchInfo;
    use aspen_client_api::ForgePatchResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                success: false,
                patch: None,
                comments: None,
                revisions: None,
                approvals: None,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    let base_hash = match blake3::Hash::from_hex(&base) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                success: false,
                patch: None,
                comments: None,
                revisions: None,
                approvals: None,
                error: Some(format!("Invalid base hash: {}", e)),
            }));
        }
    };

    let head_hash = match blake3::Hash::from_hex(&head) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                success: false,
                patch: None,
                comments: None,
                revisions: None,
                approvals: None,
                error: Some(format!("Invalid head hash: {}", e)),
            }));
        }
    };

    match forge_node.cobs.create_patch(&repo_id, &title, &description, base_hash, head_hash).await {
        Ok(patch_hash) => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                success: true,
                patch: Some(ForgePatchInfo {
                    id: patch_hash.to_hex().to_string(),
                    title,
                    description,
                    state: "open".to_string(),
                    base,
                    head,
                    labels: vec![],
                    revision_count: 1,
                    approval_count: 0,
                    assignees: vec![],
                    created_at_ms: now,
                    updated_at_ms: now,
                }),
                comments: None,
                revisions: None,
                approvals: None,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
            success: false,
            patch: None,
            comments: None,
            revisions: None,
            approvals: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_list_patches(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    state: Option<String>,
    limit: Option<u32>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgePatchInfo;
    use aspen_client_api::ForgePatchListResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgePatchListResult(ForgePatchListResultResponse {
                success: false,
                patches: vec![],
                count: 0,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    let limit = limit.unwrap_or(50).min(1000) as usize;

    match forge_node.cobs.list_patches(&repo_id).await {
        Ok(patch_ids) => {
            let mut patches = Vec::new();
            // Filter first, then limit - fixes bug where merged patches wouldn't appear
            // if they weren't in the first N patches before filtering
            for patch_id in patch_ids.iter() {
                if let Ok(patch) = forge_node.cobs.resolve_patch(&repo_id, patch_id).await {
                    let patch_state = match patch.state {
                        aspen_forge::cob::PatchState::Open => "open",
                        aspen_forge::cob::PatchState::Merged { .. } => "merged",
                        aspen_forge::cob::PatchState::Closed { .. } => "closed",
                    };

                    // Filter by state if specified
                    if let Some(ref filter_state) = state
                        && filter_state != patch_state
                    {
                        continue;
                    }

                    patches.push(ForgePatchInfo {
                        id: patch_id.to_hex().to_string(),
                        title: patch.title.clone(),
                        description: patch.description.clone(),
                        state: patch_state.to_string(),
                        base: blake3::Hash::from_bytes(patch.base).to_hex().to_string(),
                        head: blake3::Hash::from_bytes(patch.head).to_hex().to_string(),
                        labels: patch.labels.iter().cloned().collect(),
                        revision_count: patch.revisions.len() as u32,
                        approval_count: patch.approvals.len() as u32,
                        assignees: patch.assignees.iter().map(hex::encode).collect(),
                        created_at_ms: patch.created_at_ms,
                        updated_at_ms: patch.updated_at_ms,
                    });

                    // Apply limit after filtering
                    if patches.len() >= limit {
                        break;
                    }
                }
            }

            let count = patches.len() as u32;
            Ok(ClientRpcResponse::ForgePatchListResult(ForgePatchListResultResponse {
                success: true,
                patches,
                count,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::ForgePatchListResult(ForgePatchListResultResponse {
            success: false,
            patches: vec![],
            count: 0,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_get_patch(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    patch_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeCommentInfo;
    use aspen_client_api::ForgePatchApproval;
    use aspen_client_api::ForgePatchInfo;
    use aspen_client_api::ForgePatchResultResponse;
    use aspen_client_api::ForgePatchRevision;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                success: false,
                patch: None,
                comments: None,
                revisions: None,
                approvals: None,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    let patch_hash = match blake3::Hash::from_hex(&patch_id) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                success: false,
                patch: None,
                comments: None,
                revisions: None,
                approvals: None,
                error: Some(format!("Invalid patch ID: {}", e)),
            }));
        }
    };

    match forge_node.cobs.resolve_patch(&repo_id, &patch_hash).await {
        Ok(patch) => {
            let patch_state = match patch.state {
                aspen_forge::cob::PatchState::Open => "open",
                aspen_forge::cob::PatchState::Merged { .. } => "merged",
                aspen_forge::cob::PatchState::Closed { .. } => "closed",
            };

            let comments: Vec<ForgeCommentInfo> = patch
                .comments
                .iter()
                .map(|c| ForgeCommentInfo {
                    hash: hex::encode(c.change_hash),
                    author: hex::encode(c.author),
                    body: c.body.clone(),
                    timestamp_ms: c.timestamp_ms,
                })
                .collect();

            let revisions: Vec<ForgePatchRevision> = patch
                .revisions
                .iter()
                .map(|r| ForgePatchRevision {
                    hash: hex::encode(r.change_hash),
                    head: blake3::Hash::from_bytes(r.head).to_hex().to_string(),
                    message: r.message.clone(),
                    author: hex::encode(r.author),
                    timestamp_ms: r.timestamp_ms,
                })
                .collect();

            let approvals: Vec<ForgePatchApproval> = patch
                .approvals
                .iter()
                .map(|a| ForgePatchApproval {
                    author: hex::encode(a.author),
                    commit: blake3::Hash::from_bytes(a.commit).to_hex().to_string(),
                    message: a.message.clone(),
                    timestamp_ms: a.timestamp_ms,
                })
                .collect();

            Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                success: true,
                patch: Some(ForgePatchInfo {
                    id: patch_id,
                    title: patch.title,
                    description: patch.description,
                    state: patch_state.to_string(),
                    base: blake3::Hash::from_bytes(patch.base).to_hex().to_string(),
                    head: blake3::Hash::from_bytes(patch.head).to_hex().to_string(),
                    labels: patch.labels.into_iter().collect(),
                    revision_count: revisions.len() as u32,
                    approval_count: approvals.len() as u32,
                    assignees: patch.assignees.iter().map(hex::encode).collect(),
                    created_at_ms: patch.created_at_ms,
                    updated_at_ms: patch.updated_at_ms,
                }),
                comments: Some(comments),
                revisions: Some(revisions),
                approvals: Some(approvals),
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
            success: false,
            patch: None,
            comments: None,
            revisions: None,
            approvals: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_update_patch(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    patch_id: String,
    head: String,
    message: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeOperationResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                success: false,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    let patch_hash = match blake3::Hash::from_hex(&patch_id) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                success: false,
                error: Some(format!("Invalid patch ID: {}", e)),
            }));
        }
    };

    let head_hash = match blake3::Hash::from_hex(&head) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                success: false,
                error: Some(format!("Invalid head hash: {}", e)),
            }));
        }
    };

    match forge_node.cobs.update_patch(&repo_id, &patch_hash, head_hash, message).await {
        Ok(_) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            success: true,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            success: false,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_approve_patch(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    patch_id: String,
    commit: String,
    message: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeOperationResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                success: false,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    let patch_hash = match blake3::Hash::from_hex(&patch_id) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                success: false,
                error: Some(format!("Invalid patch ID: {}", e)),
            }));
        }
    };

    let commit_hash = match blake3::Hash::from_hex(&commit) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                success: false,
                error: Some(format!("Invalid commit hash: {}", e)),
            }));
        }
    };

    match forge_node.cobs.approve_patch(&repo_id, &patch_hash, commit_hash, message).await {
        Ok(_) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            success: true,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            success: false,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_merge_patch(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    patch_id: String,
    merge_commit: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeOperationResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                success: false,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    let patch_hash = match blake3::Hash::from_hex(&patch_id) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                success: false,
                error: Some(format!("Invalid patch ID: {}", e)),
            }));
        }
    };

    let merge_hash = match blake3::Hash::from_hex(&merge_commit) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                success: false,
                error: Some(format!("Invalid merge commit hash: {}", e)),
            }));
        }
    };

    match forge_node.cobs.merge_patch(&repo_id, &patch_hash, merge_hash).await {
        Ok(_) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            success: true,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            success: false,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_close_patch(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    patch_id: String,
    reason: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeOperationResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                success: false,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    let patch_hash = match blake3::Hash::from_hex(&patch_id) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                success: false,
                error: Some(format!("Invalid patch ID: {}", e)),
            }));
        }
    };

    match forge_node.cobs.close_patch(&repo_id, &patch_hash, reason).await {
        Ok(_) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            success: true,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            success: false,
            error: Some(e.to_string()),
        })),
    }
}
