//! Axum routes for the forge web interface.

use axum::{
    extract::{Path, State},
    response::Html,
    routing::get,
    Router,
};

use crate::{state::AppState, templates};

/// Build the forge web router.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(repo_list))
        .route("/{repo_id}", get(repo_overview))
        .route("/{repo_id}/tree", get(tree_root))
        .route("/{repo_id}/tree/{ref_name}/{*path}", get(tree_path))
        .route("/{repo_id}/blob/{ref_name}/{*path}", get(blob_view))
        .route("/{repo_id}/commits", get(commits))
        .route("/{repo_id}/issues", get(issues))
        .route("/{repo_id}/issues/{issue_id}", get(issue_detail))
        .route("/{repo_id}/patches", get(patches))
        .route("/{repo_id}/patches/{patch_id}", get(patch_detail))
        .with_state(state)
}

async fn repo_list(State(st): State<AppState>) -> Result<Html<String>, AppError> {
    let repos = st.list_repos().await?;
    Ok(Html(templates::repo_list(&repos).into_string()))
}

async fn repo_overview(
    State(st): State<AppState>,
    Path(repo_id): Path<String>,
) -> Result<Html<String>, AppError> {
    let repo = st.get_repo(&repo_id).await?;
    let branches = st.list_branches(&repo_id).await.unwrap_or_default();
    let recent = st.get_log(&repo_id, None, Some(10)).await.unwrap_or_default();
    Ok(Html(templates::repo_overview(&repo, &branches, &recent).into_string()))
}

async fn tree_root(
    State(st): State<AppState>,
    Path(repo_id): Path<String>,
) -> Result<Html<String>, AppError> {
    let repo = st.get_repo(&repo_id).await?;
    let ref_name = &repo.default_branch;
    let commit_hash = st.resolve_ref(&repo_id, ref_name).await?;
    let commit = st.get_commit(&commit_hash).await?;
    let entries = st.get_tree(&commit.tree).await?;
    Ok(Html(templates::file_browser(&repo, ref_name, "", &entries).into_string()))
}

async fn tree_path(
    State(st): State<AppState>,
    Path((repo_id, ref_name, path)): Path<(String, String, String)>,
) -> Result<Html<String>, AppError> {
    let repo = st.get_repo(&repo_id).await?;
    let commit_hash = st.resolve_ref(&repo_id, &ref_name).await?;
    let commit = st.get_commit(&commit_hash).await?;
    let tree_hash = walk_tree(&st, &commit.tree, &path).await?;
    let entries = st.get_tree(&tree_hash).await?;
    Ok(Html(templates::file_browser(&repo, &ref_name, &path, &entries).into_string()))
}

async fn blob_view(
    State(st): State<AppState>,
    Path((repo_id, ref_name, path)): Path<(String, String, String)>,
) -> Result<Html<String>, AppError> {
    let repo = st.get_repo(&repo_id).await?;
    let commit_hash = st.resolve_ref(&repo_id, &ref_name).await?;
    let commit = st.get_commit(&commit_hash).await?;
    let blob_hash = walk_to_blob(&st, &commit.tree, &path).await?;
    let blob = st.get_blob(&blob_hash).await?;
    let content = blob.content.as_deref();
    let size = blob.size.unwrap_or(0);
    Ok(Html(templates::file_view(&repo, &ref_name, &path, content, size).into_string()))
}

async fn commits(
    State(st): State<AppState>,
    Path(repo_id): Path<String>,
) -> Result<Html<String>, AppError> {
    let repo = st.get_repo(&repo_id).await?;
    let log = st.get_log(&repo_id, None, Some(50)).await?;
    Ok(Html(templates::commit_log(&repo, &log).into_string()))
}

async fn issues(
    State(st): State<AppState>,
    Path(repo_id): Path<String>,
) -> Result<Html<String>, AppError> {
    let repo = st.get_repo(&repo_id).await?;
    let list = st.list_issues(&repo_id).await?;
    Ok(Html(templates::issue_list(&repo, &list).into_string()))
}

async fn issue_detail(
    State(st): State<AppState>,
    Path((repo_id, issue_id)): Path<(String, String)>,
) -> Result<Html<String>, AppError> {
    let repo = st.get_repo(&repo_id).await?;
    // Fetch issue with comments in one request
    let resp = st
        .client()
        .send(aspen_client_api::messages::ClientRpcRequest::ForgeGetIssue {
            repo_id: repo_id.clone(),
            issue_id,
        })
        .await?;
    let (issue, comments) = match resp {
        aspen_client_api::messages::ClientRpcResponse::ForgeIssueResult(r) => {
            let issue = r.issue.ok_or_else(|| anyhow::anyhow!("issue not found"))?;
            let comments = r.comments.unwrap_or_default();
            (issue, comments)
        }
        other => return Err(anyhow::anyhow!("unexpected response: {other:?}").into()),
    };
    Ok(Html(templates::issue_detail(&repo, &issue, &comments).into_string()))
}

async fn patches(
    State(st): State<AppState>,
    Path(repo_id): Path<String>,
) -> Result<Html<String>, AppError> {
    let repo = st.get_repo(&repo_id).await?;
    let list = st.list_patches(&repo_id).await?;
    Ok(Html(templates::patch_list(&repo, &list).into_string()))
}

async fn patch_detail(
    State(st): State<AppState>,
    Path((repo_id, patch_id)): Path<(String, String)>,
) -> Result<Html<String>, AppError> {
    let repo = st.get_repo(&repo_id).await?;
    let patch = st.get_patch(&repo_id, patch_id).await?;
    Ok(Html(templates::patch_detail(&repo, &patch).into_string()))
}

// ── Tree walking ────────────────────────────────────────────────────

/// Walk a tree to find the subtree at a given path.
async fn walk_tree(st: &AppState, root_tree: &str, path: &str) -> Result<String, AppError> {
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    let mut current = root_tree.to_string();
    for part in parts {
        let entries = st.get_tree(&current).await?;
        let entry = entries
            .iter()
            .find(|e| e.name == part && e.mode == 0o40000)
            .ok_or_else(|| anyhow::anyhow!("directory not found: {part}"))?;
        current = entry.hash.clone();
    }
    Ok(current)
}

/// Walk a tree to find the blob hash for a file path.
async fn walk_to_blob(st: &AppState, root_tree: &str, path: &str) -> Result<String, AppError> {
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    if parts.is_empty() {
        return Err(anyhow::anyhow!("empty path").into());
    }
    let (dirs, file) = parts.split_at(parts.len() - 1);
    let mut current = root_tree.to_string();
    for dir in dirs {
        let entries = st.get_tree(&current).await?;
        let entry = entries
            .iter()
            .find(|e| e.name == *dir && e.mode == 0o40000)
            .ok_or_else(|| anyhow::anyhow!("directory not found: {dir}"))?;
        current = entry.hash.clone();
    }
    let entries = st.get_tree(&current).await?;
    let entry = entries
        .iter()
        .find(|e| e.name == file[0])
        .ok_or_else(|| anyhow::anyhow!("file not found: {}", file[0]))?;
    Ok(entry.hash.clone())
}

// ── Error handling ──────────────────────────────────────────────────

struct AppError(anyhow::Error);

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        Self(err)
    }
}

impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        tracing::error!("request error: {:#}", self.0);
        let html = templates::error_page("Error", &self.0.to_string());
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Html(html.into_string()),
        )
            .into_response()
    }
}
