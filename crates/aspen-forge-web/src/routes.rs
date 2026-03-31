//! URL routing via path matching. No framework needed.

use std::collections::HashMap;

use bytes::Bytes;
use http::StatusCode;
use tracing::warn;

use crate::state::AppState;
use crate::templates;

/// Response from a route handler — either an HTML page or raw bytes.
pub enum RouteResponse {
    /// HTML page (rendered with maud templates).
    Html { status: StatusCode, body: String },
    /// Raw content with a specific content type (for file downloads/viewing).
    Raw {
        status: StatusCode,
        content_type: &'static str,
        body: Vec<u8>,
    },
}

impl RouteResponse {
    pub fn status(&self) -> StatusCode {
        match self {
            Self::Html { status, .. } | Self::Raw { status, .. } => *status,
        }
    }

    pub fn content_type(&self) -> &str {
        match self {
            Self::Html { .. } => "text/html; charset=utf-8",
            Self::Raw { content_type, .. } => content_type,
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            Self::Html { body, .. } => body.into_bytes(),
            Self::Raw { body, .. } => body,
        }
    }
}

fn ok(markup: maud::Markup) -> RouteResponse {
    RouteResponse::Html {
        status: StatusCode::OK,
        body: markup.into_string(),
    }
}

fn not_found(path: &str) -> RouteResponse {
    RouteResponse::Html {
        status: StatusCode::NOT_FOUND,
        body: templates::error_page("Not Found", &format!("No page at {path}")).into_string(),
    }
}

fn err(e: anyhow::Error) -> RouteResponse {
    // Show a dedicated page when the forge app isn't loaded on the cluster.
    if let Some(unavailable) = e.downcast_ref::<crate::state::ForgeUnavailableError>() {
        return RouteResponse::Html {
            status: StatusCode::SERVICE_UNAVAILABLE,
            body: templates::forge_unavailable(&unavailable.message, &unavailable.hints).into_string(),
        };
    }
    warn!("handler error: {e:#}");
    RouteResponse::Html {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        body: templates::error_page("Error", &e.to_string()).into_string(),
    }
}

fn redirect(location: &str) -> RouteResponse {
    // h3 doesn't support 303 redirect natively in all clients, so we
    // return a tiny HTML page with a meta-refresh + JS redirect.
    let body = format!(
        r#"<!DOCTYPE html><html><head><meta http-equiv="refresh" content="0;url={loc}"></head><body><a href="{loc}">Redirecting…</a><script>location.replace("{loc}")</script></body></html>"#,
        loc = location
    );
    RouteResponse::Html {
        status: StatusCode::SEE_OTHER,
        body,
    }
}

fn parse_form(body: &Bytes) -> HashMap<String, String> {
    form_urlencoded::parse(body).map(|(k, v)| (k.into_owned(), v.into_owned())).collect()
}

pub fn method_not_allowed() -> RouteResponse {
    RouteResponse::Html {
        status: StatusCode::METHOD_NOT_ALLOWED,
        body: templates::error_page("Method Not Allowed", "Only GET and POST are supported.").into_string(),
    }
}

/// Dispatch a GET request to the appropriate handler.
///
/// `path` may include a query string (e.g., `/repo/search?q=term`).
pub async fn dispatch(state: &AppState, path: &str, body: Option<&Bytes>) -> RouteResponse {
    // Split path from query string.
    let (path, query_string) = path.split_once('?').unwrap_or((path, ""));
    let query: HashMap<String, String> = form_urlencoded::parse(query_string.as_bytes())
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();

    // Strip trailing slash (except root).
    let path = if path.len() > 1 {
        path.trim_end_matches('/')
    } else {
        path
    };
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    // POST routes (body is Some)
    if let Some(body) = body {
        let form = parse_form(body);
        return match segments.as_slice() {
            [repo_id, "issues", "new"] => create_issue_post(state, repo_id, &form).await,
            [repo_id, "issues", id, "comment"] => comment_issue_post(state, repo_id, id, &form).await,
            [repo_id, "issues", id, "close"] => close_issue_post(state, repo_id, id).await,
            [repo_id, "issues", id, "reopen"] => reopen_issue_post(state, repo_id, id).await,
            [repo_id, "patches", id, "merge"] => merge_patch_post(state, repo_id, id, &form).await,
            [repo_id, "patches", id, "approve"] => approve_patch_post(state, repo_id, id).await,
            ["login", "verify"] => login_verify_post(state, &form).await,
            _ => method_not_allowed(),
        };
    }

    // GET routes
    match segments.as_slice() {
        [] => repo_list(state).await,
        ["ci"] => ci_list_all(state, &query).await,
        ["cluster"] => cluster_overview(state).await,
        ["login"] => login_page(state).await,
        ["login", "challenge"] => login_challenge(state, &query).await,
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
        [repo_id, "raw", ref_name, rest @ ..] if !rest.is_empty() => {
            let sub = rest.join("/");
            raw_blob(state, repo_id, ref_name, &sub).await
        }
        [repo_id, "search"] => search(state, repo_id, &query).await,
        [repo_id, "commit", hash] => commit_detail(state, repo_id, hash).await,
        [repo_id, "commits"] => commits(state, repo_id).await,
        [repo_id, "issues"] => issues(state, repo_id).await,
        [repo_id, "issues", "new"] => new_issue_form(state, repo_id).await,
        [repo_id, "issues", id] => issue_detail(state, repo_id, id).await,
        [repo_id, "patches"] => patches(state, repo_id).await,
        [repo_id, "patches", id] => patch_detail(state, repo_id, id).await,
        [repo_id, "ci"] => ci_list_repo(state, repo_id, &query).await,
        [repo_id, "ci", run_id] => ci_run_detail(state, repo_id, run_id).await,
        [repo_id, "ci", run_id, job_id] => ci_job_logs(state, repo_id, run_id, job_id, &query).await,
        _ => not_found(path),
    }
}

// ── Handlers ────────────────────────────────────────────────────────

async fn repo_list(st: &AppState) -> RouteResponse {
    match st.list_repos().await {
        Ok(repos) => ok(templates::repo_list(&repos)),
        Err(e) => err(e),
    }
}

async fn repo_overview(st: &AppState, repo_id: &str) -> RouteResponse {
    let repo = match st.get_repo(repo_id).await {
        Ok(r) => r,
        Err(e) => return err(e),
    };
    let branches = st.list_branches(repo_id).await.unwrap_or_default();
    let recent = st.get_log(repo_id, None, Some(10)).await.unwrap_or_default();
    let readme_html = st.get_readme(&repo).await.and_then(|bytes| {
        let text = String::from_utf8(bytes).ok()?;
        Some(render_markdown(&text))
    });
    let ci_status = st.get_latest_ci_status(repo_id).await;
    let ticket = st.ticket_str();
    ok(templates::repo_overview(
        &repo,
        &branches,
        &recent,
        readme_html.as_deref(),
        &ticket,
        ci_status.as_ref(),
    ))
}

async fn tree_root(st: &AppState, repo_id: &str) -> RouteResponse {
    let repo = match st.get_repo(repo_id).await {
        Ok(r) => r,
        Err(e) => return err(e),
    };
    let ref_name = &repo.default_branch;
    tree_at(st, &repo, ref_name, "").await
}

async fn tree_path(st: &AppState, repo_id: &str, ref_name: &str, path: &str) -> RouteResponse {
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
) -> RouteResponse {
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

async fn blob_view(st: &AppState, repo_id: &str, ref_name: &str, path: &str) -> RouteResponse {
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

async fn search(st: &AppState, repo_id: &str, query: &HashMap<String, String>) -> RouteResponse {
    let repo = match st.get_repo(repo_id).await {
        Ok(r) => r,
        Err(e) => return err(e),
    };
    let q = query.get("q").map(|s| s.as_str()).unwrap_or("");
    if q.len() < 2 {
        return ok(templates::search_results(&repo, q, None));
    }
    match st.search_code(repo_id, q).await {
        Ok(results) => ok(templates::search_results(&repo, q, Some(&results))),
        Err(e) => err(e),
    }
}

async fn commit_detail(st: &AppState, repo_id: &str, hash: &str) -> RouteResponse {
    let repo = match st.get_repo(repo_id).await {
        Ok(r) => r,
        Err(e) => return err(e),
    };
    let commit = match st.get_commit(hash).await {
        Ok(c) => c,
        Err(e) => return err(e),
    };

    // Get parent tree (None for root commits)
    let parent_tree = if let Some(parent_hash) = commit.parents.first() {
        st.get_commit(parent_hash).await.ok().map(|c| c.tree)
    } else {
        None
    };

    // Diff the trees
    let files = match st.diff_trees(parent_tree.as_deref(), &commit.tree).await {
        Ok(f) => f,
        Err(e) => return err(e),
    };

    let truncated = files.len() >= crate::state::AppState::MAX_DIFF_FILES;

    // Compute line-level diffs for each file
    let mut file_diffs = Vec::with_capacity(files.len());
    for file in &files {
        let lines = st.compute_file_diff(file).await;
        file_diffs.push((file, lines));
    }

    ok(templates::commit_detail(&repo, &commit, &file_diffs, truncated))
}

async fn commits(st: &AppState, repo_id: &str) -> RouteResponse {
    let repo = match st.get_repo(repo_id).await {
        Ok(r) => r,
        Err(e) => return err(e),
    };
    match st.get_log(repo_id, None, Some(50)).await {
        Ok(log) => ok(templates::commit_log(&repo, &log)),
        Err(e) => err(e),
    }
}

async fn issues(st: &AppState, repo_id: &str) -> RouteResponse {
    let repo = match st.get_repo(repo_id).await {
        Ok(r) => r,
        Err(e) => return err(e),
    };
    match st.list_issues(repo_id).await {
        Ok(list) => ok(templates::issue_list(&repo, &list)),
        Err(e) => err(e),
    }
}

async fn issue_detail(st: &AppState, repo_id: &str, issue_id: &str) -> RouteResponse {
    let repo = match st.get_repo(repo_id).await {
        Ok(r) => r,
        Err(e) => return err(e),
    };
    match st.get_issue_with_comments(repo_id, issue_id).await {
        Ok((issue, comments)) => ok(templates::issue_detail(&repo, &issue, &comments)),
        Err(e) => err(e),
    }
}

async fn patches(st: &AppState, repo_id: &str) -> RouteResponse {
    let repo = match st.get_repo(repo_id).await {
        Ok(r) => r,
        Err(e) => return err(e),
    };
    match st.list_patches(repo_id).await {
        Ok(list) => ok(templates::patch_list(&repo, &list)),
        Err(e) => err(e),
    }
}

async fn patch_detail(st: &AppState, repo_id: &str, patch_id: &str) -> RouteResponse {
    let repo = match st.get_repo(repo_id).await {
        Ok(r) => r,
        Err(e) => return err(e),
    };
    let patch = match st.get_patch(repo_id, patch_id).await {
        Ok(p) => p,
        Err(e) => return err(e),
    };

    // Fetch merge check and diff in parallel (best-effort)
    let merge_check = if patch.state == "open" {
        st.check_merge(repo_id, patch_id).await.ok()
    } else {
        None
    };

    let diff = st.diff_trees(Some(&patch.base), &patch.head).await.unwrap_or_default();

    ok(templates::patch_detail_full(&repo, &patch, merge_check.as_ref(), &diff))
}

async fn new_issue_form(st: &AppState, repo_id: &str) -> RouteResponse {
    let repo = match st.get_repo(repo_id).await {
        Ok(r) => r,
        Err(e) => return err(e),
    };
    ok(templates::new_issue_form(&repo))
}

async fn create_issue_post(st: &AppState, repo_id: &str, form: &HashMap<String, String>) -> RouteResponse {
    let title = form.get("title").map(|s| s.as_str()).unwrap_or("");
    let body = form.get("body").map(|s| s.as_str()).unwrap_or("");

    if title.is_empty() {
        let repo = match st.get_repo(repo_id).await {
            Ok(r) => r,
            Err(e) => return err(e),
        };
        return ok(templates::new_issue_form_with_error(&repo, "Title is required.", title, body));
    }

    let labels: Vec<String> = form
        .get("labels")
        .map(|s| s.split(',').map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
        .unwrap_or_default();

    if let Err(e) = st.create_issue(repo_id, title, body, labels).await {
        return err(e);
    }

    redirect(&format!("/{repo_id}/issues"))
}

async fn comment_issue_post(
    st: &AppState,
    repo_id: &str,
    issue_id: &str,
    form: &HashMap<String, String>,
) -> RouteResponse {
    let body = form.get("body").map(|s| s.as_str()).unwrap_or("");

    if body.is_empty() {
        // Redirect back to the issue — the form is still on the page.
        return redirect(&format!("/{repo_id}/issues/{issue_id}"));
    }

    if let Err(e) = st.comment_issue(repo_id, issue_id, body).await {
        return err(e);
    }

    redirect(&format!("/{repo_id}/issues/{issue_id}"))
}

async fn close_issue_post(st: &AppState, repo_id: &str, issue_id: &str) -> RouteResponse {
    if let Err(e) = st.close_issue(repo_id, issue_id).await {
        return err(e);
    }
    redirect(&format!("/{repo_id}/issues/{issue_id}"))
}

async fn reopen_issue_post(st: &AppState, repo_id: &str, issue_id: &str) -> RouteResponse {
    if let Err(e) = st.reopen_issue(repo_id, issue_id).await {
        return err(e);
    }
    redirect(&format!("/{repo_id}/issues/{issue_id}"))
}

async fn merge_patch_post(
    st: &AppState,
    repo_id: &str,
    patch_id: &str,
    form: &HashMap<String, String>,
) -> RouteResponse {
    let strategy = form.get("strategy").map(|s| s.as_str());
    match st.merge_patch(repo_id, patch_id, strategy).await {
        Ok(result) if result.is_success => redirect(&format!("/{repo_id}/patches/{patch_id}")),
        Ok(result) => {
            let msg = result.error.unwrap_or_else(|| "merge failed".into());
            err(anyhow::anyhow!(msg))
        }
        Err(e) => err(e),
    }
}

async fn approve_patch_post(st: &AppState, repo_id: &str, patch_id: &str) -> RouteResponse {
    if let Err(e) = st.approve_patch(repo_id, patch_id).await {
        return err(e);
    }
    redirect(&format!("/{repo_id}/patches/{patch_id}"))
}

// ── CI handlers ─────────────────────────────────────────────────────

async fn ci_list_all(st: &AppState, query: &HashMap<String, String>) -> RouteResponse {
    let status_filter = query.get("status").map(|s| s.as_str()).unwrap_or("");
    let filter = if status_filter.is_empty() {
        None
    } else {
        Some(status_filter)
    };
    match st.list_runs(None, filter, Some(50)).await {
        Ok(resp) => ok(templates::pipeline_list(&resp.runs, None, None, status_filter)),
        Err(e) => {
            if e.downcast_ref::<crate::state::ForgeUnavailableError>().is_some() {
                return ok(templates::ci_unavailable());
            }
            err(e)
        }
    }
}

async fn ci_list_repo(st: &AppState, repo_id: &str, query: &HashMap<String, String>) -> RouteResponse {
    let repo = match st.get_repo(repo_id).await {
        Ok(r) => r,
        Err(e) => return err(e),
    };
    let status_filter = query.get("status").map(|s| s.as_str()).unwrap_or("");
    let filter = if status_filter.is_empty() {
        None
    } else {
        Some(status_filter)
    };
    match st.list_runs(Some(repo_id), filter, Some(50)).await {
        Ok(resp) => ok(templates::pipeline_list(&resp.runs, Some(repo_id), Some(&repo.name), status_filter)),
        Err(e) => {
            if e.downcast_ref::<crate::state::ForgeUnavailableError>().is_some() {
                return ok(templates::ci_unavailable());
            }
            err(e)
        }
    }
}

async fn ci_run_detail(st: &AppState, repo_id: &str, run_id: &str) -> RouteResponse {
    let repo = match st.get_repo(repo_id).await {
        Ok(r) => r,
        Err(e) => return err(e),
    };
    match st.get_run_status(run_id).await {
        Ok(resp) if !resp.was_found => not_found(&format!("/{repo_id}/ci/{run_id}")),
        Ok(resp) => ok(templates::pipeline_detail(repo_id, &repo.name, &resp)),
        Err(e) => err(e),
    }
}

async fn ci_job_logs(
    st: &AppState,
    repo_id: &str,
    run_id: &str,
    job_id: &str,
    query: &HashMap<String, String>,
) -> RouteResponse {
    let repo = match st.get_repo(repo_id).await {
        Ok(r) => r,
        Err(e) => return err(e),
    };

    let start_index: u32 = query.get("start").and_then(|s| s.parse().ok()).unwrap_or(0);

    // Get job info from the run status
    let run_status = match st.get_run_status(run_id).await {
        Ok(r) => r,
        Err(e) => return err(e),
    };

    let job = run_status.stages.iter().flat_map(|s| s.jobs.iter()).find(|j| j.id == job_id);

    let (job_name, job_status, job_started, job_ended) = match job {
        Some(j) => (j.name.as_str(), j.status.as_str(), j.started_at_ms, j.ended_at_ms),
        None => return not_found(&format!("/{repo_id}/ci/{run_id}/{job_id}")),
    };

    match st.get_job_logs(run_id, job_id, start_index, Some(100)).await {
        Ok(logs) => ok(templates::job_log_viewer(&templates::JobLogParams {
            repo_id,
            repo_name: &repo.name,
            run_id,
            job_id,
            job_name,
            job_status,
            job_started_ms: job_started,
            job_ended_ms: job_ended,
            logs_resp: &logs,
            start_index,
        })),
        Err(e) => err(e),
    }
}

// ── Cluster handler ─────────────────────────────────────────────────

async fn cluster_overview(st: &AppState) -> RouteResponse {
    let health = match st.get_health().await {
        Ok(h) => h,
        Err(e) => return err(e),
    };
    let cluster = match st.get_cluster_state().await {
        Ok(c) => c,
        Err(e) => return err(e),
    };
    ok(templates::cluster_overview(&health, &cluster))
}

// ── Login handlers ──────────────────────────────────────────────────

async fn login_page(_st: &AppState) -> RouteResponse {
    ok(templates::login_page())
}

async fn login_challenge(st: &AppState, query: &HashMap<String, String>) -> RouteResponse {
    let npub = match query.get("npub") {
        Some(n) if n.len() == 64 => n.as_str(),
        _ => {
            return RouteResponse::Raw {
                status: StatusCode::BAD_REQUEST,
                content_type: "application/json",
                body: b"{\"error\":\"npub query param required (64 hex chars)\"}".to_vec(),
            };
        }
    };
    match st.nostr_auth_challenge(npub).await {
        Ok((challenge_id, challenge_hex)) => RouteResponse::Raw {
            status: StatusCode::OK,
            content_type: "application/json",
            body: serde_json::json!({
                "challenge_id": challenge_id,
                "challenge_hex": challenge_hex,
            })
            .to_string()
            .into_bytes(),
        },
        Err(e) => RouteResponse::Raw {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            content_type: "application/json",
            body: format!("{{\"error\":\"{e}\"}}").into_bytes(),
        },
    }
}

async fn login_verify_post(st: &AppState, form: &HashMap<String, String>) -> RouteResponse {
    let npub = form.get("npub").map(|s| s.as_str()).unwrap_or("");
    let challenge_id = form.get("challenge_id").map(|s| s.as_str()).unwrap_or("");
    let signature = form.get("signature").map(|s| s.as_str()).unwrap_or("");

    if npub.is_empty() || challenge_id.is_empty() || signature.is_empty() {
        return err(anyhow::anyhow!("missing npub, challenge_id, or signature"));
    }

    match st.nostr_auth_verify(npub, challenge_id, signature).await {
        Ok(token) => {
            // Set token as cookie and redirect to home
            let body = format!(
                r#"<!DOCTYPE html><html><head>
                <meta http-equiv="refresh" content="0;url=/">
                <script>document.cookie="aspen_token={token};path=/;SameSite=Strict;max-age=86400";location.replace("/")</script>
                </head><body>Redirecting…</body></html>"#
            );
            RouteResponse::Html {
                status: StatusCode::OK,
                body,
            }
        }
        Err(e) => err(e),
    }
}

async fn raw_blob(st: &AppState, repo_id: &str, ref_name: &str, path: &str) -> RouteResponse {
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
        Ok(blob) => match blob.content {
            Some(data) => RouteResponse::Raw {
                status: StatusCode::OK,
                content_type: content_type_for_path(path),
                body: data,
            },
            None => not_found(path),
        },
        Err(e) => err(e),
    }
}

// ── Markdown rendering ──────────────────────────────────────────────

fn render_markdown(source: &str) -> String {
    templates::render_markdown(source)
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

// ── Content-type detection ──────────────────────────────────────────

/// Derive a content-type from the file extension in a path.
fn content_type_for_path(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("");
    match ext {
        // Text
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" => "application/json",
        "xml" => "application/xml",
        "txt" | "md" | "rs" | "toml" | "yaml" | "yml" | "nix" | "sh" | "py" | "rb" | "go" | "c" | "h" | "cpp"
        | "hpp" | "java" | "ts" | "tsx" | "jsx" | "lock" | "cfg" | "ini" | "conf" | "diff" | "patch" | "csv"
        | "sql" | "graphql" | "proto" | "zig" | "el" | "ex" | "exs" | "erl" | "hrl" | "hs" | "ml" | "mli" | "lua"
        | "vim" | "svelte" | "vue" | "tf" | "hcl" | "kdl" | "ncl" => "text/plain; charset=utf-8",
        "svg" => "image/svg+xml",
        // Images
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "avif" => "image/avif",
        // Fonts
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        // Archives / binaries
        "wasm" => "application/wasm",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "gz" | "tgz" => "application/gzip",
        "tar" => "application/x-tar",
        "xz" => "application/x-xz",
        "zst" => "application/zstd",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_type_source_files() {
        assert_eq!(content_type_for_path("lib.rs"), "text/plain; charset=utf-8");
        assert_eq!(content_type_for_path("flake.nix"), "text/plain; charset=utf-8");
        assert_eq!(content_type_for_path("Cargo.toml"), "text/plain; charset=utf-8");
        assert_eq!(content_type_for_path("deep/nested/path.py"), "text/plain; charset=utf-8");
    }

    #[test]
    fn content_type_web_assets() {
        assert_eq!(content_type_for_path("style.css"), "text/css; charset=utf-8");
        assert_eq!(content_type_for_path("app.js"), "text/javascript; charset=utf-8");
        assert_eq!(content_type_for_path("index.html"), "text/html; charset=utf-8");
        assert_eq!(content_type_for_path("data.json"), "application/json");
    }

    #[test]
    fn content_type_images() {
        assert_eq!(content_type_for_path("logo.png"), "image/png");
        assert_eq!(content_type_for_path("photo.jpg"), "image/jpeg");
        assert_eq!(content_type_for_path("icon.svg"), "image/svg+xml");
    }

    #[test]
    fn content_type_binary_fallback() {
        assert_eq!(content_type_for_path("binary"), "application/octet-stream");
        assert_eq!(content_type_for_path("data.unknown"), "application/octet-stream");
        assert_eq!(content_type_for_path("program.wasm"), "application/wasm");
    }

    #[test]
    fn route_response_html_content_type() {
        let resp = ok(maud::html! { p { "hello" } });
        assert_eq!(resp.content_type(), "text/html; charset=utf-8");
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[test]
    fn route_response_raw_content_type() {
        let resp = RouteResponse::Raw {
            status: StatusCode::OK,
            content_type: "image/png",
            body: vec![0x89, 0x50, 0x4E, 0x47],
        };
        assert_eq!(resp.content_type(), "image/png");
        assert_eq!(resp.into_bytes(), vec![0x89, 0x50, 0x4E, 0x47]);
    }

    #[test]
    fn not_found_is_404() {
        let resp = not_found("/missing");
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn parse_form_urlencoded() {
        let body = Bytes::from("title=Bug+report&body=It%27s+broken&labels=bug%2Curgent");
        let form = parse_form(&body);
        assert_eq!(form.get("title").unwrap(), "Bug report");
        assert_eq!(form.get("body").unwrap(), "It's broken");
        assert_eq!(form.get("labels").unwrap(), "bug,urgent");
    }

    #[test]
    fn parse_form_empty() {
        let form = parse_form(&Bytes::new());
        assert!(form.is_empty());
    }

    #[test]
    fn redirect_is_303() {
        let resp = redirect("/repo/issues");
        assert_eq!(resp.status(), StatusCode::SEE_OTHER);
        let body = resp.into_bytes();
        let html = String::from_utf8(body).unwrap();
        assert!(html.contains("/repo/issues"));
    }

    #[test]
    fn method_not_allowed_is_405() {
        let resp = method_not_allowed();
        assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[test]
    fn render_markdown_headings_and_code() {
        let md = "# Hello\n\nSome `inline` code.\n\n```rust\nfn main() {}\n```\n";
        let html = render_markdown(md);
        assert!(html.contains("<h1>Hello</h1>"));
        assert!(html.contains("<code>inline</code>"));
        assert!(html.contains("<pre><code class=\"language-rust\">"));
    }

    #[test]
    fn render_markdown_tables() {
        let md = "| a | b |\n|---|---|\n| 1 | 2 |\n";
        let html = render_markdown(md);
        assert!(html.contains("<table>"));
        assert!(html.contains("<td>1</td>"));
    }

    #[test]
    fn render_markdown_truncates_large_input() {
        let big = "x".repeat(crate::templates::MAX_MARKDOWN_BYTES + 100);
        let html = render_markdown(&big);
        // Should render without panic; output comes from truncated input.
        assert!(!html.is_empty());
    }

    #[test]
    fn forge_unavailable_is_503() {
        let e: anyhow::Error = crate::state::ForgeUnavailableError {
            message: "the 'forge' app is not loaded on this cluster".into(),
            hints: vec![],
        }
        .into();
        let resp = err(e);
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = String::from_utf8(resp.into_bytes()).unwrap();
        assert!(body.contains("Forge Unavailable"));
        assert!(body.contains("--features forge,blob"));
    }

    #[test]
    fn forge_unavailable_with_hints() {
        let e: anyhow::Error = crate::state::ForgeUnavailableError {
            message: "forge not loaded".into(),
            hints: vec![aspen_client_api::CapabilityHint {
                cluster_key: "abc123".into(),
                name: "prod-cluster".into(),
                app_version: Some("0.5.0".into()),
            }],
        }
        .into();
        let resp = err(e);
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = String::from_utf8(resp.into_bytes()).unwrap();
        assert!(body.contains("prod-cluster"));
        assert!(body.contains("abc123"));
        assert!(body.contains("0.5.0"));
    }

    #[test]
    fn generic_error_still_500() {
        let e = anyhow::anyhow!("something broke");
        let resp = err(e);
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    // ── CI and cluster template rendering tests ──────────────────────

    #[test]
    fn pipeline_list_empty_state() {
        let markup = templates::pipeline_list(&[], None, None, "");
        let html = markup.into_string();
        assert!(html.contains("CI Pipelines"));
        assert!(html.contains("No pipeline runs yet"));
    }

    #[test]
    fn pipeline_list_with_runs() {
        let runs = vec![
            aspen_client_api::messages::CiRunInfo {
                run_id: "abc123def456".into(),
                repo_id: "repo1".into(),
                ref_name: "main".into(),
                status: "succeeded".into(),
                created_at_ms: 1700000000000,
            },
            aspen_client_api::messages::CiRunInfo {
                run_id: "xyz789".into(),
                repo_id: "repo1".into(),
                ref_name: "main".into(),
                status: "failed".into(),
                created_at_ms: 1700000100000,
            },
        ];
        let html = templates::pipeline_list(&runs, None, None, "").into_string();
        assert!(html.contains("abc123def456"));
        assert!(html.contains("ci-succeeded"));
        assert!(html.contains("ci-failed"));
        assert!(html.contains("main"));
    }

    #[test]
    fn pipeline_list_with_status_filter() {
        let html = templates::pipeline_list(&[], None, None, "running").into_string();
        assert!(html.contains("ci-filter-active"));
    }

    #[test]
    fn pipeline_detail_running_has_meta_refresh() {
        let resp = aspen_client_api::messages::CiGetStatusResponse {
            was_found: true,
            run_id: Some("run123".into()),
            repo_id: Some("repo1".into()),
            ref_name: Some("main".into()),
            commit_hash: Some("deadbeef".into()),
            status: Some("running".into()),
            stages: vec![],
            created_at_ms: Some(1700000000000),
            completed_at_ms: None,
            error: None,
        };
        let html = templates::pipeline_detail("repo1", "my-repo", &resp).into_string();
        assert!(html.contains("meta http-equiv=\"refresh\""));
        assert!(html.contains("ci-running"));
    }

    #[test]
    fn pipeline_detail_completed_no_meta_refresh() {
        let resp = aspen_client_api::messages::CiGetStatusResponse {
            was_found: true,
            run_id: Some("run123".into()),
            repo_id: Some("repo1".into()),
            ref_name: Some("main".into()),
            commit_hash: Some("deadbeef".into()),
            status: Some("succeeded".into()),
            stages: vec![],
            created_at_ms: Some(1700000000000),
            completed_at_ms: Some(1700000060000),
            error: None,
        };
        let html = templates::pipeline_detail("repo1", "my-repo", &resp).into_string();
        assert!(!html.contains("meta http-equiv=\"refresh\""));
        assert!(html.contains("ci-succeeded"));
    }

    #[test]
    fn pipeline_detail_with_stages_and_jobs() {
        let resp = aspen_client_api::messages::CiGetStatusResponse {
            was_found: true,
            run_id: Some("run456".into()),
            repo_id: Some("repo1".into()),
            ref_name: Some("main".into()),
            commit_hash: None,
            status: Some("succeeded".into()),
            stages: vec![aspen_client_api::messages::CiStageInfo {
                name: "build".into(),
                status: "succeeded".into(),
                jobs: vec![aspen_client_api::messages::CiJobInfo {
                    id: "job1".into(),
                    name: "compile".into(),
                    status: "succeeded".into(),
                    started_at_ms: Some(1700000000000),
                    ended_at_ms: Some(1700000045000),
                    error: None,
                }],
            }],
            created_at_ms: Some(1700000000000),
            completed_at_ms: Some(1700000060000),
            error: None,
        };
        let html = templates::pipeline_detail("repo1", "my-repo", &resp).into_string();
        assert!(html.contains("build"));
        assert!(html.contains("compile"));
        assert!(html.contains("45s"));
        assert!(html.contains("logs"));
    }

    #[test]
    fn job_log_viewer_running_has_meta_refresh() {
        let logs = aspen_client_api::messages::CiGetJobLogsResponse {
            was_found: true,
            chunks: vec![aspen_client_api::messages::CiLogChunkInfo {
                index: 0,
                content: "Building...".into(),
                timestamp_ms: 1700000000000,
            }],
            last_index: 0,
            has_more: false,
            is_complete: false,
            error: None,
        };
        let html = templates::job_log_viewer(&templates::JobLogParams {
            repo_id: "repo1",
            repo_name: "my-repo",
            run_id: "run1",
            job_id: "job1",
            job_name: "build",
            job_status: "running",
            job_started_ms: Some(1700000000000),
            job_ended_ms: None,
            logs_resp: &logs,
            start_index: 0,
        })
        .into_string();
        assert!(html.contains("meta http-equiv=\"refresh\""));
        assert!(html.contains("Building..."));
    }

    #[test]
    fn job_log_viewer_has_load_more() {
        let logs = aspen_client_api::messages::CiGetJobLogsResponse {
            was_found: true,
            chunks: vec![],
            last_index: 99,
            has_more: true,
            is_complete: false,
            error: None,
        };
        let html = templates::job_log_viewer(&templates::JobLogParams {
            repo_id: "repo1",
            repo_name: "my-repo",
            run_id: "run1",
            job_id: "job1",
            job_name: "build",
            job_status: "succeeded",
            job_started_ms: Some(1700000000000),
            job_ended_ms: Some(1700000060000),
            logs_resp: &logs,
            start_index: 0,
        })
        .into_string();
        assert!(html.contains("Load more"));
    }

    #[test]
    fn cluster_overview_renders() {
        let health = aspen_client_api::messages::HealthResponse {
            status: "healthy".into(),
            node_id: 1,
            raft_node_id: Some(1),
            uptime_seconds: 3661,
            is_initialized: true,
            membership_node_count: Some(3),
            iroh_node_id: None,
        };
        let cluster = aspen_client_api::messages::ClusterStateResponse {
            nodes: vec![
                aspen_client_api::messages::NodeDescriptor {
                    node_id: 1,
                    endpoint_addr: "addr1".into(),
                    is_voter: true,
                    is_learner: false,
                    is_leader: true,
                },
                aspen_client_api::messages::NodeDescriptor {
                    node_id: 2,
                    endpoint_addr: "addr2".into(),
                    is_voter: true,
                    is_learner: false,
                    is_leader: false,
                },
            ],
            leader_id: Some(1),
            this_node_id: 1,
        };
        let html = templates::cluster_overview(&health, &cluster).into_string();
        assert!(html.contains("Cluster Overview"));
        assert!(html.contains("meta http-equiv=\"refresh\""));
        assert!(html.contains("healthy"));
        assert!(html.contains("leader"));
        assert!(html.contains("follower"));
        assert!(html.contains("1h 1m"));
    }

    #[test]
    fn ci_unavailable_page() {
        let html = templates::ci_unavailable().into_string();
        assert!(html.contains("CI Unavailable"));
        assert!(html.contains("--features ci"));
    }

    #[test]
    fn ci_status_badge_variants() {
        let run = aspen_client_api::messages::CiRunInfo {
            run_id: "r1".into(),
            repo_id: "repo1".into(),
            ref_name: "main".into(),
            status: "succeeded".into(),
            created_at_ms: 0,
        };
        let html = templates::ci_status_badge("repo1", &run).into_string();
        assert!(html.contains("CI: passing"));
        assert!(html.contains("ci-succeeded"));

        let failed = aspen_client_api::messages::CiRunInfo {
            status: "failed".into(),
            ..run.clone()
        };
        let html = templates::ci_status_badge("repo1", &failed).into_string();
        assert!(html.contains("CI: failing"));
        assert!(html.contains("ci-failed"));
    }

    // ── ANSI to HTML conversion tests ────────────────────────────────

    #[test]
    fn ansi_to_html_plain_text() {
        let result = ansi_to_html::convert("hello world").unwrap();
        assert!(result.contains("hello world"));
    }

    #[test]
    fn ansi_to_html_red_text() {
        let input = "\x1b[31merror\x1b[0m";
        let result = ansi_to_html::convert(input).unwrap();
        // Should contain a span with red color
        assert!(result.contains("error"));
        assert!(result.contains("<span"));
    }

    #[test]
    fn ansi_to_html_nested_colors() {
        let input = "\x1b[31mred \x1b[32mgreen\x1b[0m after";
        let result = ansi_to_html::convert(input).unwrap();
        assert!(result.contains("red"));
        assert!(result.contains("green"));
        assert!(result.contains("after"));
    }

    #[test]
    fn ansi_to_html_reset() {
        let input = "\x1b[1mbold\x1b[0m normal";
        let result = ansi_to_html::convert(input).unwrap();
        assert!(result.contains("bold"));
        assert!(result.contains("normal"));
    }

    #[test]
    fn ansi_to_html_no_escapes() {
        let input = "plain text with no escapes";
        let result = ansi_to_html::convert(input).unwrap();
        assert_eq!(result, "plain text with no escapes");
    }
}
