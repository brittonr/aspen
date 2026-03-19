//! HTML templates using maud for the Forge web frontend.

use aspen_forge_protocol::ForgeCommitInfo;
use aspen_forge_protocol::ForgeIssueInfo;
use aspen_forge_protocol::ForgePatchInfo;
use aspen_forge_protocol::ForgeRefInfo;
use aspen_forge_protocol::ForgeRepoInfo;
use aspen_forge_protocol::ForgeTreeEntry;
use maud::DOCTYPE;
use maud::Markup;
use maud::PreEscaped;
use maud::html;

/// Format ms-since-epoch as a human-readable date.
fn format_time(ms: u64) -> String {
    let secs = (ms / 1000) as i64;
    chrono::DateTime::from_timestamp(secs, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "unknown".into())
}

/// Truncate a hash for display.
fn short_hash(hash: &str) -> &str {
    if hash.len() > 12 { &hash[..12] } else { hash }
}

/// Check if a tree entry mode is a directory.
fn is_dir(mode: u32) -> bool {
    mode == 0o40000
}

fn file_icon(mode: u32) -> &'static str {
    if mode == 0o40000 {
        "📁"
    } else if mode == 0o120000 {
        "🔗"
    } else {
        "📄"
    }
}

/// Base page layout.
pub fn base_layout(title: &str, content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (title) " — Aspen Forge" }
                style { (PreEscaped(CSS)) }
            }
            body {
                nav { a href="/" { "🌲 Aspen Forge" } }
                main { (content) }
                footer {
                    "Powered by " a href="https://github.com/aspen-lang/aspen" { "Aspen Forge" }
                }
            }
        }
    }
}

/// Repository list page.
pub fn repo_list(repos: &[ForgeRepoInfo]) -> Markup {
    base_layout("Repositories", html! {
        h1 { "Repositories" }
        @if repos.is_empty() {
            p.muted { "No repositories yet." }
        } @else {
            @for repo in repos {
                div.card {
                    h3 { a href=(format!("/{}", repo.id)) { (&repo.name) } }
                    @if let Some(ref desc) = repo.description {
                        p.muted { (desc) }
                    }
                    span.meta {
                        "default: " code { (&repo.default_branch) }
                        " · " (repo.delegates.len()) " delegates"
                        " · " (format_time(repo.created_at_ms))
                    }
                }
            }
        }
    })
}

/// Repository overview page.
pub fn repo_overview(repo: &ForgeRepoInfo, branches: &[ForgeRefInfo], recent_commits: &[ForgeCommitInfo]) -> Markup {
    base_layout(&repo.name, html! {
        h1 { (&repo.name) }
        @if let Some(ref desc) = repo.description {
            p.muted { (desc) }
        }

        div.tabs {
            a.active href=(format!("/{}", repo.id)) { "Overview" }
            a href=(format!("/{}/tree", repo.id)) { "Files" }
            a href=(format!("/{}/commits", repo.id)) { "Commits" }
            a href=(format!("/{}/issues", repo.id)) { "Issues" }
            a href=(format!("/{}/patches", repo.id)) { "Patches" }
        }

        h2 { "Branches" }
        @if branches.is_empty() {
            p.muted { "No branches." }
        } @else {
            ul {
                @for b in branches {
                    li {
                        a href=(format!("/{}/tree/{}", repo.id, b.name)) { (&b.name) }
                        " → " code { (short_hash(&b.hash)) }
                    }
                }
            }
        }

        h2 { "Recent Commits" }
        @if recent_commits.is_empty() {
            p.muted { "No commits." }
        } @else {
            (commit_table(repo, recent_commits))
        }
    })
}

/// File browser page (directory listing).
pub fn file_browser(repo: &ForgeRepoInfo, ref_name: &str, path: &str, entries: &[ForgeTreeEntry]) -> Markup {
    base_layout(&format!("{} — {}", repo.name, if path.is_empty() { "/" } else { path }), html! {
        h1 {
            a href=(format!("/{}", repo.id)) { (&repo.name) }
            " / "
            a href=(format!("/{}/tree/{}", repo.id, ref_name)) { code { (ref_name) } }
            @if !path.is_empty() {
                (breadcrumb_path(repo, ref_name, path))
            }
        }

        div.tabs {
            a href=(format!("/{}", repo.id)) { "Overview" }
            a.active href=(format!("/{}/tree", repo.id)) { "Files" }
            a href=(format!("/{}/commits", repo.id)) { "Commits" }
            a href=(format!("/{}/issues", repo.id)) { "Issues" }
            a href=(format!("/{}/patches", repo.id)) { "Patches" }
        }

        table.file-list {
            @if !path.is_empty() {
                tr {
                    td { a href=(parent_link(repo, ref_name, path)) { "📁 .." } }
                    td {}
                }
            }
            @for entry in entries {
                tr {
                    td {
                        @let link = entry_link(repo, ref_name, path, &entry.name, entry.mode);
                        a href=(link) { (file_icon(entry.mode)) " " (&entry.name) }
                    }
                    td.hash { code { (short_hash(&entry.hash)) } }
                }
            }
        }
    })
}

/// Blob (file content) page.
pub fn file_view(repo: &ForgeRepoInfo, ref_name: &str, path: &str, content: Option<&[u8]>, size: u64) -> Markup {
    let is_text = content.as_ref().is_some_and(|c| is_likely_text(c));
    let raw_url = format!("/{}/raw/{}/{}", repo.id, ref_name, path);
    base_layout(&format!("{} — {}", repo.name, path), html! {
        h1 {
            a href=(format!("/{}", repo.id)) { (&repo.name) }
            " / "
            code { (ref_name) }
            " / " (path)
        }

        p.meta {
            (size) " bytes"
            " · "
            a.raw-link href=(raw_url) { "Raw" }
        }

        @if is_text {
            @if let Some(bytes) = content {
                pre.code-block {
                    table.code-table {
                        @let text = String::from_utf8_lossy(bytes);
                        @for (line_no, line) in text.lines().enumerate() {
                            tr {
                                td.ln { (line_no + 1) }
                                td.line { code { (line) } }
                            }
                        }
                    }
                }
            }
        } @else {
            p.muted {
                "Binary file (" (size) " bytes) · "
                a href=(raw_url) { "Download" }
            }
        }
    })
}

/// Commit log page.
pub fn commit_log(repo: &ForgeRepoInfo, commits: &[ForgeCommitInfo]) -> Markup {
    base_layout(&format!("{} — Commits", repo.name), html! {
        h1 { a href=(format!("/{}", repo.id)) { (&repo.name) } " — Commits" }

        div.tabs {
            a href=(format!("/{}", repo.id)) { "Overview" }
            a href=(format!("/{}/tree", repo.id)) { "Files" }
            a.active href=(format!("/{}/commits", repo.id)) { "Commits" }
            a href=(format!("/{}/issues", repo.id)) { "Issues" }
            a href=(format!("/{}/patches", repo.id)) { "Patches" }
        }

        @if commits.is_empty() {
            p.muted { "No commits." }
        } @else {
            (commit_table(repo, commits))
        }
    })
}

/// Issue list page.
pub fn issue_list(repo: &ForgeRepoInfo, issues: &[ForgeIssueInfo]) -> Markup {
    base_layout(&format!("{} — Issues", repo.name), html! {
        h1 { a href=(format!("/{}", repo.id)) { (&repo.name) } " — Issues" }

        div.tabs {
            a href=(format!("/{}", repo.id)) { "Overview" }
            a href=(format!("/{}/tree", repo.id)) { "Files" }
            a href=(format!("/{}/commits", repo.id)) { "Commits" }
            a.active href=(format!("/{}/issues", repo.id)) { "Issues" }
            a href=(format!("/{}/patches", repo.id)) { "Patches" }
        }

        p { a.btn href=(format!("/{}/issues/new", repo.id)) { "+ New Issue" } }

        @if issues.is_empty() {
            p.muted { "No issues." }
        } @else {
            @for issue in issues {
                div.card {
                    h3 {
                        a href=(format!("/{}/issues/{}", repo.id, issue.id)) {
                            (&issue.title)
                        }
                    }
                    span.meta {
                        span class={ "badge " (if issue.state == "open" { "open" } else { "closed" }) } {
                            (&issue.state)
                        }
                        " · " (issue.comment_count) " comments"
                        " · " (format_time(issue.created_at_ms))
                    }
                }
            }
        }
    })
}

/// Issue detail page.
pub fn issue_detail(
    repo: &ForgeRepoInfo,
    issue: &ForgeIssueInfo,
    comments: &[aspen_forge_protocol::ForgeCommentInfo],
) -> Markup {
    base_layout(&format!("{} — #{}", repo.name, short_hash(&issue.id)), html! {
        p { a href=(format!("/{}/issues", repo.id)) { "← Issues" } }
        h1 { (&issue.title) }
        span class={ "badge " (if issue.state == "open" { "open" } else { "closed" }) } {
            (&issue.state)
        }
        div.card { p { (&issue.body) } }

        h2 { "Comments (" (comments.len()) ")" }
        @for c in comments {
            div.card {
                p.meta { code { (short_hash(&c.author)) } " · " (format_time(c.timestamp_ms)) }
                p { (&c.body) }
            }
        }

        h2 { "Add Comment" }
        form.card method="POST" action=(format!("/{}/issues/{}/comment", repo.id, issue.id)) {
            textarea name="body" rows="4" placeholder="Write a comment…" required {} {}
            div.form-actions { button type="submit" { "Comment" } }
        }
    })
}

/// Patch list page.
pub fn patch_list(repo: &ForgeRepoInfo, patches: &[ForgePatchInfo]) -> Markup {
    base_layout(&format!("{} — Patches", repo.name), html! {
        h1 { a href=(format!("/{}", repo.id)) { (&repo.name) } " — Patches" }

        div.tabs {
            a href=(format!("/{}", repo.id)) { "Overview" }
            a href=(format!("/{}/tree", repo.id)) { "Files" }
            a href=(format!("/{}/commits", repo.id)) { "Commits" }
            a href=(format!("/{}/issues", repo.id)) { "Issues" }
            a.active href=(format!("/{}/patches", repo.id)) { "Patches" }
        }

        @if patches.is_empty() {
            p.muted { "No patches." }
        } @else {
            @for patch in patches {
                div.card {
                    h3 {
                        a href=(format!("/{}/patches/{}", repo.id, patch.id)) {
                            (&patch.title)
                        }
                    }
                    span.meta {
                        span class={ "badge " (if patch.state == "open" { "open" } else { "closed" }) } {
                            (&patch.state)
                        }
                        " · " (patch.revision_count) " revisions"
                        " · " (patch.approval_count) " approvals"
                        " · " (format_time(patch.created_at_ms))
                    }
                }
            }
        }
    })
}

/// Patch detail page.
pub fn patch_detail(repo: &ForgeRepoInfo, patch: &ForgePatchInfo) -> Markup {
    base_layout(&format!("{} — {}", repo.name, patch.title), html! {
        p { a href=(format!("/{}/patches", repo.id)) { "← Patches" } }
        h1 { (&patch.title) }
        span class={ "badge " (if patch.state == "open" { "open" } else { "closed" }) } {
            (&patch.state)
        }

        @if !patch.description.is_empty() {
            div.card { p { (&patch.description) } }
        }

        p.meta {
            "base: " code { (short_hash(&patch.base)) }
            " → head: " code { (short_hash(&patch.head)) }
        }

        p.meta {
            (patch.revision_count) " revisions · "
            (patch.approval_count) " approvals · "
            (format_time(patch.created_at_ms))
        }
    })
}

/// New issue form page.
pub fn new_issue_form(repo: &ForgeRepoInfo) -> Markup {
    new_issue_form_inner(repo, None, "", "")
}

/// New issue form with validation error.
pub fn new_issue_form_with_error(repo: &ForgeRepoInfo, error: &str, title: &str, body: &str) -> Markup {
    new_issue_form_inner(repo, Some(error), title, body)
}

fn new_issue_form_inner(repo: &ForgeRepoInfo, error: Option<&str>, title: &str, body: &str) -> Markup {
    base_layout(&format!("{} — New Issue", repo.name), html! {
        p { a href=(format!("/{}/issues", repo.id)) { "← Issues" } }
        h1 { "New Issue" }

        @if let Some(msg) = error {
            div.error { (msg) }
        }

        form.card method="POST" action=(format!("/{}/issues/new", repo.id)) {
            label for="title" { "Title" }
            input type="text" name="title" id="title" value=(title) required
                placeholder="Issue title" {}

            label for="body" { "Description" }
            textarea name="body" id="body" rows="8" placeholder="Describe the issue…" { (body) }

            label for="labels" { "Labels " span.muted { "(comma-separated)" } }
            input type="text" name="labels" id="labels" placeholder="bug, docs" {}

            div.form-actions { button type="submit" { "Create Issue" } }
        }
    })
}

/// Error page.
pub fn error_page(title: &str, message: &str) -> Markup {
    base_layout(title, html! {
        h1 { (title) }
        p.muted { (message) }
        p { a href="/" { "← Back to repositories" } }
    })
}

// ── Helpers ──────────────────────────────────────────────────────────

fn commit_table(_repo: &ForgeRepoInfo, commits: &[ForgeCommitInfo]) -> Markup {
    html! {
        table.commits {
            @for c in commits {
                tr {
                    td.hash { code { (short_hash(&c.hash)) } }
                    td { (first_line(&c.message)) }
                    td.meta { (&c.author_name) }
                    td.meta { (format_time(c.timestamp_ms)) }
                }
            }
        }
    }
}

fn first_line(s: &str) -> &str {
    s.lines().next().unwrap_or(s)
}

fn entry_link(repo: &ForgeRepoInfo, ref_name: &str, path: &str, name: &str, mode: u32) -> String {
    let full = if path.is_empty() {
        name.to_string()
    } else {
        format!("{path}/{name}")
    };
    if is_dir(mode) {
        format!("/{}/tree/{}/{}", repo.id, ref_name, full)
    } else {
        format!("/{}/blob/{}/{}", repo.id, ref_name, full)
    }
}

fn parent_link(repo: &ForgeRepoInfo, ref_name: &str, path: &str) -> String {
    match path.rsplit_once('/') {
        Some((parent, _)) => format!("/{}/tree/{}/{}", repo.id, ref_name, parent),
        None => format!("/{}/tree/{}", repo.id, ref_name),
    }
}

fn is_likely_text(data: &[u8]) -> bool {
    let check_len = data.len().min(8192);
    let nul_count = data[..check_len].iter().filter(|&&b| b == 0).count();
    nul_count == 0
}

fn breadcrumb_path(repo: &ForgeRepoInfo, ref_name: &str, path: &str) -> Markup {
    let segments: Vec<&str> = path.split('/').collect();
    let mut cumulative_paths = Vec::new();
    let mut current_path = String::new();

    for (i, segment) in segments.iter().enumerate() {
        if i > 0 {
            current_path.push('/');
        }
        current_path.push_str(segment);
        cumulative_paths.push((segment, current_path.clone()));
    }

    html! {
        @for (segment, cumulative) in cumulative_paths {
            " / "
            a href=(format!("/{}/tree/{}/{}", repo.id, ref_name, cumulative)) { (segment) }
        }
    }
}

// ── CSS ──────────────────────────────────────────────────────────────

const CSS: &str = r#"
*{box-sizing:border-box;margin:0;padding:0}
body{font-family:system-ui,-apple-system,sans-serif;background:#0d1117;color:#c9d1d9;line-height:1.6}
a{color:#58a6ff;text-decoration:none}
a:hover{text-decoration:underline}
nav{background:#161b22;padding:.75rem 1.5rem;border-bottom:1px solid #30363d}
nav a{color:#f0f6fc;font-weight:600;font-size:1.1rem}
main{max-width:960px;margin:0 auto;padding:1.5rem}
h1,h2,h3{color:#f0f6fc;margin:.75rem 0 .5rem}
h1{font-size:1.5rem}
code{font-family:'JetBrains Mono',monospace;font-size:.85em;background:#161b22;padding:.1em .3em;border-radius:3px}
.muted,.meta{color:#8b949e;font-size:.85rem}
.card{background:#161b22;border:1px solid #30363d;border-radius:6px;padding:1rem;margin:.5rem 0}
.tabs{display:flex;gap:0;border-bottom:1px solid #30363d;margin:1rem 0}
.tabs a{padding:.5rem 1rem;color:#8b949e;border-bottom:2px solid transparent}
.tabs a.active{color:#f0f6fc;border-bottom-color:#58a6ff}
.tabs a:hover{color:#f0f6fc;text-decoration:none}
.badge{display:inline-block;padding:.15em .5em;border-radius:99px;font-size:.75rem;font-weight:600}
.badge.open{background:#238636;color:#fff}
.badge.closed{background:#da3633;color:#fff}
table{width:100%;border-collapse:collapse}
table td{padding:.4rem .6rem;border-bottom:1px solid #21262d}
table.file-list td:first-child{width:70%}
table.file-list tr:hover{background:#161b22}
table.commits td:first-child{width:100px}
table.code-table{border:none;background:transparent}
table.code-table td{border:none;padding:0;vertical-align:top}
.hash{color:#8b949e;text-align:right}
.ln{color:#484f58;text-align:right;user-select:none;padding-right:1em;width:1px;white-space:nowrap}
.line{width:100%}
.line code{background:transparent;padding:0}
pre.code-block{background:#161b22;border:1px solid #30363d;border-radius:6px;padding:1rem;overflow-x:auto;font-size:.85rem;line-height:1.5}
ul{list-style:none;padding-left:0}
li{padding:.25rem 0}
footer{text-align:center;padding:2rem 1rem;color:#8b949e;font-size:.85rem;border-top:1px solid #30363d;margin-top:2rem}
footer a{color:#58a6ff}
a.raw-link{display:inline-block;padding:.15em .5em;border:1px solid #30363d;border-radius:4px;font-size:.8rem;color:#8b949e}
a.raw-link:hover{color:#f0f6fc;border-color:#58a6ff;text-decoration:none}
form label{display:block;margin:.75rem 0 .25rem;color:#c9d1d9;font-weight:600;font-size:.9rem}
form input[type=text],form textarea{width:100%;padding:.5rem;background:#0d1117;color:#c9d1d9;border:1px solid #30363d;border-radius:4px;font-family:inherit;font-size:.9rem}
form input[type=text]:focus,form textarea:focus{outline:none;border-color:#58a6ff}
.form-actions{margin-top:.75rem;text-align:right}
button{padding:.5rem 1rem;background:#238636;color:#fff;border:none;border-radius:4px;font-size:.9rem;cursor:pointer}
button:hover{background:#2ea043}
.btn{display:inline-block;padding:.4rem .8rem;background:#238636;color:#fff;border-radius:4px;font-size:.85rem}
.btn:hover{background:#2ea043;text-decoration:none}
.error{background:#3d1a1a;border:1px solid #da3633;border-radius:4px;padding:.75rem;margin:.5rem 0;color:#f85149}
@media (max-width: 768px) {
    main{padding:0.75rem}
    table{font-size:0.85rem}
}
"#;
