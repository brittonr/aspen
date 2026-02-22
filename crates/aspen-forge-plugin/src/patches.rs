//! Patch handlers: create, list, get, update, approve, merge, close.
//!
//! Patches are stored as JSON in KV under `forge:{repo_id}:patches:{patch_id}`.
//! An index at `forge:{repo_id}:patches:idx` holds a JSON array of patch IDs
//! for listing.

use aspen_client_api::ClientRpcResponse;
use aspen_client_api::ForgeCommentInfo;
use aspen_client_api::ForgeOperationResultResponse;
use aspen_client_api::ForgePatchApproval;
use aspen_client_api::ForgePatchInfo;
use aspen_client_api::ForgePatchListResultResponse;
use aspen_client_api::ForgePatchResultResponse;
use aspen_client_api::ForgePatchRevision;

use crate::kv;
use crate::types::ApprovalData;
use crate::types::PatchData;
use crate::types::RevisionData;

/// Maximum patches returned by list.
const MAX_LIST_PATCHES: u32 = 1000;

// ============================================================================
// KV key helpers
// ============================================================================

fn patch_key(repo_id: &str, patch_id: &str) -> String {
    format!("forge:{repo_id}:patches:{patch_id}")
}

fn patch_index_key(repo_id: &str) -> String {
    format!("forge:{repo_id}:patches:idx")
}

// ============================================================================
// Internal helpers
// ============================================================================

fn generate_id(title: &str, timestamp: u64) -> String {
    let input = format!("{title}:{timestamp}");
    hex::encode(blake3::hash(input.as_bytes()).as_bytes())
}

fn generate_hash(parent_id: &str, discriminator: &str, timestamp: u64) -> String {
    let input = format!("{parent_id}:{discriminator}:{timestamp}");
    hex::encode(blake3::hash(input.as_bytes()).as_bytes())
}

fn read_patch(repo_id: &str, patch_id: &str) -> Option<PatchData> {
    let bytes = kv::kv_get(&patch_key(repo_id, patch_id)).ok()??;
    serde_json::from_slice(&bytes).ok()
}

fn write_patch(repo_id: &str, patch_id: &str, data: &PatchData) -> Result<(), String> {
    let bytes = serde_json::to_vec(data).map_err(|e| format!("serialize patch: {e}"))?;
    kv::kv_put(&patch_key(repo_id, patch_id), &bytes)
}

fn read_index(repo_id: &str) -> Vec<String> {
    match kv::kv_get(&patch_index_key(repo_id)) {
        Ok(Some(bytes)) => serde_json::from_slice(&bytes).unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn write_index(repo_id: &str, ids: &[String]) -> Result<(), String> {
    let bytes = serde_json::to_vec(ids).map_err(|e| format!("serialize index: {e}"))?;
    kv::kv_put(&patch_index_key(repo_id), &bytes)
}

fn patch_to_info(patch_id: &str, data: &PatchData) -> ForgePatchInfo {
    ForgePatchInfo {
        id: patch_id.to_string(),
        title: data.title.clone(),
        description: data.description.clone(),
        state: data.state.clone(),
        base: data.base.clone(),
        head: data.head.clone(),
        labels: data.labels.clone(),
        revision_count: data.revisions.len() as u32,
        approval_count: data.approvals.len() as u32,
        assignees: data.assignees.clone(),
        created_at_ms: data.created_at_ms,
        updated_at_ms: data.updated_at_ms,
    }
}

fn comments_to_info(data: &PatchData) -> Vec<ForgeCommentInfo> {
    data.comments
        .iter()
        .map(|c| ForgeCommentInfo {
            hash: c.hash.clone(),
            author: c.author.clone(),
            body: c.body.clone(),
            timestamp_ms: c.timestamp_ms,
        })
        .collect()
}

fn revisions_to_info(data: &PatchData) -> Vec<ForgePatchRevision> {
    data.revisions
        .iter()
        .map(|r| ForgePatchRevision {
            hash: r.hash.clone(),
            head: r.head.clone(),
            message: r.message.clone(),
            author: r.author.clone(),
            timestamp_ms: r.timestamp_ms,
        })
        .collect()
}

fn approvals_to_info(data: &PatchData) -> Vec<ForgePatchApproval> {
    data.approvals
        .iter()
        .map(|a| ForgePatchApproval {
            author: a.author.clone(),
            commit: a.commit.clone(),
            message: a.message.clone(),
            timestamp_ms: a.timestamp_ms,
        })
        .collect()
}

// ============================================================================
// Handlers
// ============================================================================

pub fn handle_create_patch(
    repo_id: String,
    title: String,
    description: String,
    base: String,
    head: String,
) -> ClientRpcResponse {
    let now = kv::hlc_now();
    let author = kv::public_key();
    let patch_id = generate_id(&title, now);

    let data = PatchData {
        title: title.clone(),
        description: description.clone(),
        state: "open".to_string(),
        close_reason: None,
        base: base.clone(),
        head: head.clone(),
        labels: Vec::new(),
        comments: Vec::new(),
        revisions: Vec::new(),
        approvals: Vec::new(),
        assignees: Vec::new(),
        created_at_ms: now,
        updated_at_ms: now,
        author,
    };

    if let Err(e) = write_patch(&repo_id, &patch_id, &data) {
        return ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
            is_success: false,
            patch: None,
            comments: None,
            revisions: None,
            approvals: None,
            error: Some(format!("failed to store patch: {e}")),
        });
    }

    // Update index
    let mut ids = read_index(&repo_id);
    ids.push(patch_id.clone());
    if let Err(e) = write_index(&repo_id, &ids) {
        return ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
            is_success: false,
            patch: None,
            comments: None,
            revisions: None,
            approvals: None,
            error: Some(format!("failed to update index: {e}")),
        });
    }

    ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
        is_success: true,
        patch: Some(patch_to_info(&patch_id, &data)),
        comments: None,
        revisions: None,
        approvals: None,
        error: None,
    })
}

pub fn handle_list_patches(repo_id: String, state: Option<String>, limit: Option<u32>) -> ClientRpcResponse {
    let limit = limit.unwrap_or(50).min(MAX_LIST_PATCHES) as usize;
    let ids = read_index(&repo_id);

    let mut patches = Vec::new();
    for id in &ids {
        let Some(data) = read_patch(&repo_id, id) else {
            continue;
        };

        // Filter by state if specified
        if let Some(ref filter_state) = state {
            if &data.state != filter_state {
                continue;
            }
        }

        patches.push(patch_to_info(id, &data));

        if patches.len() >= limit {
            break;
        }
    }

    let count = patches.len() as u32;
    ClientRpcResponse::ForgePatchListResult(ForgePatchListResultResponse {
        is_success: true,
        patches,
        count,
        error: None,
    })
}

pub fn handle_get_patch(repo_id: String, patch_id: String) -> ClientRpcResponse {
    match read_patch(&repo_id, &patch_id) {
        Some(data) => {
            let comments = comments_to_info(&data);
            let revisions = revisions_to_info(&data);
            let approvals = approvals_to_info(&data);
            ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                is_success: true,
                patch: Some(patch_to_info(&patch_id, &data)),
                comments: Some(comments),
                revisions: Some(revisions),
                approvals: Some(approvals),
                error: None,
            })
        }
        None => ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
            is_success: false,
            patch: None,
            comments: None,
            revisions: None,
            approvals: None,
            error: Some(format!("patch not found: {patch_id}")),
        }),
    }
}

pub fn handle_update_patch(
    repo_id: String,
    patch_id: String,
    head: String,
    message: Option<String>,
) -> ClientRpcResponse {
    let Some(mut data) = read_patch(&repo_id, &patch_id) else {
        return ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            is_success: false,
            error: Some(format!("patch not found: {patch_id}")),
        });
    };

    let now = kv::hlc_now();
    let author = kv::public_key();
    let hash = generate_hash(&patch_id, &head, now);

    data.revisions.push(RevisionData {
        hash,
        head: head.clone(),
        message,
        author,
        timestamp_ms: now,
    });
    data.head = head;
    data.updated_at_ms = now;

    match write_patch(&repo_id, &patch_id, &data) {
        Ok(()) => ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            is_success: true,
            error: None,
        }),
        Err(e) => ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            is_success: false,
            error: Some(format!("failed to update patch: {e}")),
        }),
    }
}

pub fn handle_approve_patch(
    repo_id: String,
    patch_id: String,
    commit: String,
    message: Option<String>,
) -> ClientRpcResponse {
    let Some(mut data) = read_patch(&repo_id, &patch_id) else {
        return ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            is_success: false,
            error: Some(format!("patch not found: {patch_id}")),
        });
    };

    let now = kv::hlc_now();
    let author = kv::public_key();

    data.approvals.push(ApprovalData {
        author,
        commit,
        message,
        timestamp_ms: now,
    });
    data.updated_at_ms = now;

    match write_patch(&repo_id, &patch_id, &data) {
        Ok(()) => ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            is_success: true,
            error: None,
        }),
        Err(e) => ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            is_success: false,
            error: Some(format!("failed to update patch: {e}")),
        }),
    }
}

pub fn handle_merge_patch(repo_id: String, patch_id: String, merge_commit: String) -> ClientRpcResponse {
    let Some(mut data) = read_patch(&repo_id, &patch_id) else {
        return ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            is_success: false,
            error: Some(format!("patch not found: {patch_id}")),
        });
    };

    data.state = "merged".to_string();
    data.head = merge_commit;
    data.updated_at_ms = kv::hlc_now();

    match write_patch(&repo_id, &patch_id, &data) {
        Ok(()) => ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            is_success: true,
            error: None,
        }),
        Err(e) => ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            is_success: false,
            error: Some(format!("failed to update patch: {e}")),
        }),
    }
}

pub fn handle_close_patch(repo_id: String, patch_id: String, reason: Option<String>) -> ClientRpcResponse {
    let Some(mut data) = read_patch(&repo_id, &patch_id) else {
        return ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            is_success: false,
            error: Some(format!("patch not found: {patch_id}")),
        });
    };

    data.state = "closed".to_string();
    data.close_reason = reason;
    data.updated_at_ms = kv::hlc_now();

    match write_patch(&repo_id, &patch_id, &data) {
        Ok(()) => ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            is_success: true,
            error: None,
        }),
        Err(e) => ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            is_success: false,
            error: Some(format!("failed to update patch: {e}")),
        }),
    }
}
