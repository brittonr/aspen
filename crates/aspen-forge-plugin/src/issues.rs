//! Issue handlers: create, list, get, comment, close, reopen.
//!
//! Issues are stored as JSON in KV under `forge:{repo_id}:issues:{issue_id}`.
//! An index at `forge:{repo_id}:issues:idx` holds a JSON array of issue IDs
//! for listing.

use aspen_client_api::ClientRpcResponse;
use aspen_client_api::ForgeCommentInfo;
use aspen_client_api::ForgeIssueInfo;
use aspen_client_api::ForgeIssueListResultResponse;
use aspen_client_api::ForgeIssueResultResponse;
use aspen_client_api::ForgeOperationResultResponse;

use crate::kv;
use crate::types::CommentData;
use crate::types::IssueData;

/// Maximum issues returned by list.
const MAX_LIST_ISSUES: u32 = 1000;

// ============================================================================
// KV key helpers
// ============================================================================

fn issue_key(repo_id: &str, issue_id: &str) -> String {
    format!("forge:{repo_id}:issues:{issue_id}")
}

fn issue_index_key(repo_id: &str) -> String {
    format!("forge:{repo_id}:issues:idx")
}

// ============================================================================
// Internal helpers
// ============================================================================

fn generate_id(title: &str, timestamp: u64) -> String {
    let input = format!("{title}:{timestamp}");
    hex::encode(blake3::hash(input.as_bytes()).as_bytes())
}

fn generate_comment_hash(parent_id: &str, body: &str, timestamp: u64) -> String {
    let input = format!("{parent_id}:{body}:{timestamp}");
    hex::encode(blake3::hash(input.as_bytes()).as_bytes())
}

fn read_issue(repo_id: &str, issue_id: &str) -> Option<IssueData> {
    let bytes = kv::kv_get(&issue_key(repo_id, issue_id)).ok()??;
    serde_json::from_slice(&bytes).ok()
}

fn write_issue(repo_id: &str, issue_id: &str, data: &IssueData) -> Result<(), String> {
    let bytes = serde_json::to_vec(data).map_err(|e| format!("serialize issue: {e}"))?;
    kv::kv_put(&issue_key(repo_id, issue_id), &bytes)
}

fn read_index(repo_id: &str) -> Vec<String> {
    match kv::kv_get(&issue_index_key(repo_id)) {
        Ok(Some(bytes)) => serde_json::from_slice(&bytes).unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn write_index(repo_id: &str, ids: &[String]) -> Result<(), String> {
    let bytes = serde_json::to_vec(ids).map_err(|e| format!("serialize index: {e}"))?;
    kv::kv_put(&issue_index_key(repo_id), &bytes)
}

fn issue_to_info(issue_id: &str, data: &IssueData) -> ForgeIssueInfo {
    ForgeIssueInfo {
        id: issue_id.to_string(),
        title: data.title.clone(),
        body: data.body.clone(),
        state: data.state.clone(),
        labels: data.labels.clone(),
        comment_count: data.comments.len() as u32,
        assignees: data.assignees.clone(),
        created_at_ms: data.created_at_ms,
        updated_at_ms: data.updated_at_ms,
    }
}

fn comments_to_info(data: &IssueData) -> Vec<ForgeCommentInfo> {
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

// ============================================================================
// Handlers
// ============================================================================

pub fn handle_create_issue(repo_id: String, title: String, body: String, labels: Vec<String>) -> ClientRpcResponse {
    let now = kv::hlc_now();
    let author = kv::public_key();
    let issue_id = generate_id(&title, now);

    let data = IssueData {
        title: title.clone(),
        body: body.clone(),
        state: "open".to_string(),
        close_reason: None,
        labels: labels.clone(),
        comments: Vec::new(),
        assignees: Vec::new(),
        created_at_ms: now,
        updated_at_ms: now,
        author,
    };

    if let Err(e) = write_issue(&repo_id, &issue_id, &data) {
        return ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
            is_success: false,
            issue: None,
            comments: None,
            error: Some(format!("failed to store issue: {e}")),
        });
    }

    // Update index
    let mut ids = read_index(&repo_id);
    ids.push(issue_id.clone());
    if let Err(e) = write_index(&repo_id, &ids) {
        return ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
            is_success: false,
            issue: None,
            comments: None,
            error: Some(format!("failed to update index: {e}")),
        });
    }

    ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
        is_success: true,
        issue: Some(issue_to_info(&issue_id, &data)),
        comments: None,
        error: None,
    })
}

pub fn handle_list_issues(repo_id: String, state: Option<String>, limit: Option<u32>) -> ClientRpcResponse {
    let limit = limit.unwrap_or(50).min(MAX_LIST_ISSUES) as usize;
    let ids = read_index(&repo_id);

    let mut issues = Vec::new();
    for id in &ids {
        let Some(data) = read_issue(&repo_id, id) else {
            continue;
        };

        // Filter by state if specified
        if let Some(ref filter_state) = state {
            if &data.state != filter_state {
                continue;
            }
        }

        issues.push(issue_to_info(id, &data));

        if issues.len() >= limit {
            break;
        }
    }

    let count = issues.len() as u32;
    ClientRpcResponse::ForgeIssueListResult(ForgeIssueListResultResponse {
        is_success: true,
        issues,
        count,
        error: None,
    })
}

pub fn handle_get_issue(repo_id: String, issue_id: String) -> ClientRpcResponse {
    match read_issue(&repo_id, &issue_id) {
        Some(data) => {
            let comments = comments_to_info(&data);
            ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
                is_success: true,
                issue: Some(issue_to_info(&issue_id, &data)),
                comments: Some(comments),
                error: None,
            })
        }
        None => ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
            is_success: false,
            issue: None,
            comments: None,
            error: Some(format!("issue not found: {issue_id}")),
        }),
    }
}

pub fn handle_comment_issue(repo_id: String, issue_id: String, body: String) -> ClientRpcResponse {
    let Some(mut data) = read_issue(&repo_id, &issue_id) else {
        return ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            is_success: false,
            error: Some(format!("issue not found: {issue_id}")),
        });
    };

    let now = kv::hlc_now();
    let author = kv::public_key();
    let hash = generate_comment_hash(&issue_id, &body, now);

    data.comments.push(CommentData {
        hash,
        author,
        body,
        timestamp_ms: now,
    });
    data.updated_at_ms = now;

    match write_issue(&repo_id, &issue_id, &data) {
        Ok(()) => ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            is_success: true,
            error: None,
        }),
        Err(e) => ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            is_success: false,
            error: Some(format!("failed to update issue: {e}")),
        }),
    }
}

pub fn handle_close_issue(repo_id: String, issue_id: String, reason: Option<String>) -> ClientRpcResponse {
    let Some(mut data) = read_issue(&repo_id, &issue_id) else {
        return ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            is_success: false,
            error: Some(format!("issue not found: {issue_id}")),
        });
    };

    data.state = "closed".to_string();
    data.close_reason = reason;
    data.updated_at_ms = kv::hlc_now();

    match write_issue(&repo_id, &issue_id, &data) {
        Ok(()) => ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            is_success: true,
            error: None,
        }),
        Err(e) => ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            is_success: false,
            error: Some(format!("failed to update issue: {e}")),
        }),
    }
}

pub fn handle_reopen_issue(repo_id: String, issue_id: String) -> ClientRpcResponse {
    let Some(mut data) = read_issue(&repo_id, &issue_id) else {
        return ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            is_success: false,
            error: Some(format!("issue not found: {issue_id}")),
        });
    };

    data.state = "open".to_string();
    data.close_reason = None;
    data.updated_at_ms = kv::hlc_now();

    match write_issue(&repo_id, &issue_id, &data) {
        Ok(()) => ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            is_success: true,
            error: None,
        }),
        Err(e) => ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            is_success: false,
            error: Some(format!("failed to update issue: {e}")),
        }),
    }
}
