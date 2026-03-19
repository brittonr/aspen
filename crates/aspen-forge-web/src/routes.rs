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
            ["login", "verify"] => login_verify_post(state, &form).await,
            _ => method_not_allowed(),
        };
    }

    // GET routes
    match segments.as_slice() {
        [] => repo_list(state).await,
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
    let ticket = st.ticket_str();
    ok(templates::repo_overview(&repo, &branches, &recent, readme_html.as_deref(), &ticket))
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
    match st.get_patch(repo_id, patch_id).await {
        Ok(patch) => ok(templates::patch_detail(&repo, &patch)),
        Err(e) => err(e),
    }
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
}
