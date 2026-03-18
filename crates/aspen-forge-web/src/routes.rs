//! URL routing via path matching. No framework needed.

use http::StatusCode;
use tracing::warn;

use crate::state::AppState;
use crate::templates;

/// Response payload from a route handler.
pub struct HtmlResponse {
    pub status: StatusCode,
    pub body: String,
}

fn ok(markup: maud::Markup) -> HtmlResponse {
    HtmlResponse { status: StatusCode::OK, body: markup.into_string() }
}

fn not_found(path: &str) -> HtmlResponse {
    HtmlResponse {
        status: StatusCode::NOT_FOUND,
        body: templates::error_page("Not Found", &format!("No page at {path}")).into_string(),
    }
}

fn err(e: anyhow::Error) -> HtmlResponse {
    warn!("handler error: {e:#}");
    HtmlResponse {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        body: templates::error_page("Error", &e.to_string()).into_string(),
    }
}

pub fn method_not_allowed() -> HtmlResponse {
    HtmlResponse {
        status: StatusCode::METHOD_NOT_ALLOWED,
        body: templates::error_page("Method Not Allowed", "Only GET is supported.").into_string(),
    }
}

/// Dispatch a GET request to the appropriate handler.
pub async fn dispatch(state: &AppState, path: &str) -> HtmlResponse {
    // Strip trailing slash (except root).
    let path = if path.len() > 1 { path.trim_end_matches('/') } else { path };
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    match segments.as_slice() {
        [] => repo_list(state).await,
        [repo_id] => repo_overview(state, repo_id).await,
        [repo_id, "tree"] => tree_root(state, repo_id).await,
        [repo_id, "tree", ref_name, rest @ ..] => {
            let sub = rest.join("/");
            tree_path(state, repo_id, ref_name, &sub).await
        }
        [repo_id, "blob", ref_name, rest @ ..] if !rest.is_empty() => {
            let sub = rest.join("/");
            blob_view(state, repo_id, ref_name, &sub).await
        }
        [repo_id, "commits"] => commits(state, repo_id).await,
        [repo_id, "issues"] => issues(state, repo_id).await,
        [repo_id, "issues", id] => issue_detail(state, repo_id, id).await,
        [repo_id, "patches"] => patches(state, repo_id).await,
        [repo_id, "patches", id] => patch_detail(state, repo_id, id).await,
        _ => not_found(path),
    }
}

// ── Handlers ────────────────────────────────────────────────────────

async fn repo_list(st: &AppState) -> HtmlResponse {
    match st.list_repos().await {
        Ok(repos) => ok(templates::repo_list(&repos)),
        Err(e) => err(e),
    }
}

async fn repo_overview(st: &AppState, repo_id: &str) -> HtmlResponse {
    let repo = match st.get_repo(repo_id).await {
        Ok(r) => r,
        Err(e) => return err(e),
    };
    let branches = st.list_branches(repo_id).await.unwrap_or_default();
    let recent = st.get_log(repo_id, None, Some(10)).await.unwrap_or_default();
    ok(templates::repo_overview(&repo, &branches, &recent))
}

async fn tree_root(st: &AppState, repo_id: &str) -> HtmlResponse {
    let repo = match st.get_repo(repo_id).await {
        Ok(r) => r,
        Err(e) => return err(e),
    };
    let ref_name = &repo.default_branch;
    tree_at(st, &repo, ref_name, "").await
}

async fn tree_path(st: &AppState, repo_id: &str, ref_name: &str, path: &str) -> HtmlResponse {
    let repo = match st.get_repo(repo_id).await {
        Ok(r) => r,
        Err(e) => return err(e),
    };
    tree_at(st, &repo, ref_name, path).await
}

async fn tree_at(
    st: &AppState,
    repo: &aspen_forge_protocol::ForgeRepoInfo,
    ref_name: &str,
    path: &str,
) -> HtmlResponse {
    let commit_hash = match st.resolve_ref(&repo.id, ref_name).await {
        Ok(h) => h,
        Err(e) => return err(e),
    };
    let commit = match st.get_commit(&commit_hash).await {
        Ok(c) => c,
        Err(e) => return err(e),
    };
    let tree_hash = match walk_tree(st, &commit.tree, path).await {
        Ok(h) => h,
        Err(e) => return err(e),
    };
    match st.get_tree(&tree_hash).await {
        Ok(entries) => ok(templates::file_browser(repo, ref_name, path, &entries)),
        Err(e) => err(e),
    }
}

async fn blob_view(st: &AppState, repo_id: &str, ref_name: &str, path: &str) -> HtmlResponse {
    let repo = match st.get_repo(repo_id).await {
        Ok(r) => r,
        Err(e) => return err(e),
    };
    let commit_hash = match st.resolve_ref(repo_id, ref_name).await {
        Ok(h) => h,
        Err(e) => return err(e),
    };
    let commit = match st.get_commit(&commit_hash).await {
        Ok(c) => c,
        Err(e) => return err(e),
    };
    let blob_hash = match walk_to_blob(st, &commit.tree, path).await {
        Ok(h) => h,
        Err(e) => return err(e),
    };
    match st.get_blob(&blob_hash).await {
        Ok(blob) => {
            let content = blob.content.as_deref();
            let size = blob.size.unwrap_or(0);
            ok(templates::file_view(&repo, ref_name, path, content, size))
        }
        Err(e) => err(e),
    }
}

async fn commits(st: &AppState, repo_id: &str) -> HtmlResponse {
    let repo = match st.get_repo(repo_id).await {
        Ok(r) => r,
        Err(e) => return err(e),
    };
    match st.get_log(repo_id, None, Some(50)).await {
        Ok(log) => ok(templates::commit_log(&repo, &log)),
        Err(e) => err(e),
    }
}

async fn issues(st: &AppState, repo_id: &str) -> HtmlResponse {
    let repo = match st.get_repo(repo_id).await {
        Ok(r) => r,
        Err(e) => return err(e),
    };
    match st.list_issues(repo_id).await {
        Ok(list) => ok(templates::issue_list(&repo, &list)),
        Err(e) => err(e),
    }
}

async fn issue_detail(st: &AppState, repo_id: &str, issue_id: &str) -> HtmlResponse {
    let repo = match st.get_repo(repo_id).await {
        Ok(r) => r,
        Err(e) => return err(e),
    };
    match st.get_issue_with_comments(repo_id, issue_id).await {
        Ok((issue, comments)) => ok(templates::issue_detail(&repo, &issue, &comments)),
        Err(e) => err(e),
    }
}

async fn patches(st: &AppState, repo_id: &str) -> HtmlResponse {
    let repo = match st.get_repo(repo_id).await {
        Ok(r) => r,
        Err(e) => return err(e),
    };
    match st.list_patches(repo_id).await {
        Ok(list) => ok(templates::patch_list(&repo, &list)),
        Err(e) => err(e),
    }
}

async fn patch_detail(st: &AppState, repo_id: &str, patch_id: &str) -> HtmlResponse {
    let repo = match st.get_repo(repo_id).await {
        Ok(r) => r,
        Err(e) => return err(e),
    };
    match st.get_patch(repo_id, patch_id).await {
        Ok(patch) => ok(templates::patch_detail(&repo, &patch)),
        Err(e) => err(e),
    }
}

// ── Tree walking ────────────────────────────────────────────────────

async fn walk_tree(st: &AppState, root_tree: &str, path: &str) -> anyhow::Result<String> {
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

async fn walk_to_blob(st: &AppState, root_tree: &str, path: &str) -> anyhow::Result<String> {
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    if parts.is_empty() {
        anyhow::bail!("empty path");
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
