//! Issue tracking operations.

use aspen_client_api::ClientRpcResponse;

use crate::handler::handlers::ForgeNodeRef;

pub(crate) async fn handle_create_issue(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    title: String,
    body: String,
    labels: Vec<String>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeIssueInfo;
    use aspen_client_api::ForgeIssueResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
                success: false,
                issue: None,
                comments: None,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    match forge_node.cobs.create_issue(&repo_id, &title, &body, labels.clone()).await {
        Ok(issue_hash) => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
                success: true,
                issue: Some(ForgeIssueInfo {
                    id: issue_hash.to_hex().to_string(),
                    title,
                    body,
                    state: "open".to_string(),
                    labels,
                    comment_count: 0,
                    assignees: vec![],
                    created_at_ms: now,
                    updated_at_ms: now,
                }),
                comments: None,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
            success: false,
            issue: None,
            comments: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_list_issues(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    state: Option<String>,
    limit: Option<u32>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeIssueInfo;
    use aspen_client_api::ForgeIssueListResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeIssueListResult(ForgeIssueListResultResponse {
                success: false,
                issues: vec![],
                count: 0,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    let limit = limit.unwrap_or(50).min(1000) as usize;

    match forge_node.cobs.list_issues(&repo_id).await {
        Ok(issue_ids) => {
            let mut issues = Vec::new();
            // Filter first, then limit - fixes bug where closed issues wouldn't appear
            // if they weren't in the first N issues before filtering
            for issue_id in issue_ids.iter() {
                if let Ok(issue) = forge_node.cobs.resolve_issue(&repo_id, issue_id).await {
                    let issue_state = if issue.state.is_open() { "open" } else { "closed" };

                    // Filter by state if specified
                    if let Some(ref filter_state) = state
                        && filter_state != issue_state
                    {
                        continue;
                    }

                    issues.push(ForgeIssueInfo {
                        id: issue_id.to_hex().to_string(),
                        title: issue.title.clone(),
                        body: issue.body.clone(),
                        state: issue_state.to_string(),
                        labels: issue.labels.iter().cloned().collect(),
                        comment_count: issue.comments.len() as u32,
                        assignees: issue.assignees.iter().map(hex::encode).collect(),
                        created_at_ms: issue.created_at_ms,
                        updated_at_ms: issue.updated_at_ms,
                    });

                    // Apply limit after filtering
                    if issues.len() >= limit {
                        break;
                    }
                }
            }

            let count = issues.len() as u32;
            Ok(ClientRpcResponse::ForgeIssueListResult(ForgeIssueListResultResponse {
                success: true,
                issues,
                count,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::ForgeIssueListResult(ForgeIssueListResultResponse {
            success: false,
            issues: vec![],
            count: 0,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_get_issue(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    issue_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeCommentInfo;
    use aspen_client_api::ForgeIssueInfo;
    use aspen_client_api::ForgeIssueResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
                success: false,
                issue: None,
                comments: None,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    let issue_hash = match blake3::Hash::from_hex(&issue_id) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
                success: false,
                issue: None,
                comments: None,
                error: Some(format!("Invalid issue ID: {}", e)),
            }));
        }
    };

    match forge_node.cobs.resolve_issue(&repo_id, &issue_hash).await {
        Ok(issue) => {
            let comments: Vec<ForgeCommentInfo> = issue
                .comments
                .iter()
                .map(|c| ForgeCommentInfo {
                    hash: hex::encode(c.change_hash),
                    author: hex::encode(c.author),
                    body: c.body.clone(),
                    timestamp_ms: c.timestamp_ms,
                })
                .collect();

            Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
                success: true,
                issue: Some(ForgeIssueInfo {
                    id: issue_id,
                    title: issue.title,
                    body: issue.body,
                    state: if issue.state.is_open() {
                        "open".to_string()
                    } else {
                        "closed".to_string()
                    },
                    labels: issue.labels.into_iter().collect(),
                    comment_count: comments.len() as u32,
                    assignees: issue.assignees.iter().map(hex::encode).collect(),
                    created_at_ms: issue.created_at_ms,
                    updated_at_ms: issue.updated_at_ms,
                }),
                comments: Some(comments),
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
            success: false,
            issue: None,
            comments: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_comment_issue(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    issue_id: String,
    body: String,
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

    let issue_hash = match blake3::Hash::from_hex(&issue_id) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                success: false,
                error: Some(format!("Invalid issue ID: {}", e)),
            }));
        }
    };

    match forge_node.cobs.add_comment(&repo_id, &issue_hash, &body).await {
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

pub(crate) async fn handle_close_issue(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    issue_id: String,
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

    let issue_hash = match blake3::Hash::from_hex(&issue_id) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                success: false,
                error: Some(format!("Invalid issue ID: {}", e)),
            }));
        }
    };

    match forge_node.cobs.close_issue(&repo_id, &issue_hash, reason).await {
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

pub(crate) async fn handle_reopen_issue(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    issue_id: String,
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

    let issue_hash = match blake3::Hash::from_hex(&issue_id) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                success: false,
                error: Some(format!("Invalid issue ID: {}", e)),
            }));
        }
    };

    match forge_node.cobs.reopen_issue(&repo_id, &issue_hash).await {
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
