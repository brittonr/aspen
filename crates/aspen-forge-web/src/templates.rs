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

/// Tiger Style: max markdown input size (1 MB) to bound rendering time.
pub(crate) const MAX_MARKDOWN_BYTES: usize = 1024 * 1024;

const LOGIN_JS: &str = r#"
(async function() {
  var msg = document.getElementById('login-msg');
  if (!window.nostr) {
    msg.textContent = 'No Nostr extension detected. Install nos2x, Alby, or another NIP-07 extension.';
    return;
  }
  try {
    msg.textContent = 'Getting your public key…';
    var pubkey = await window.nostr.getPublicKey();
    msg.textContent = 'Requesting challenge for ' + pubkey.substring(0, 12) + '…';

    var resp = await fetch('/login/challenge?npub=' + pubkey);
    var data = await resp.json();
    if (data.error) { msg.textContent = 'Error: ' + data.error; return; }

    msg.textContent = 'Please approve the signature in your extension…';
    var event = { kind: 27235, created_at: Math.floor(Date.now()/1000),
      tags: [['challenge', data.challenge_hex]], content: data.challenge_id, pubkey: pubkey };
    var signed = await window.nostr.signEvent(event);

    document.getElementById('form-npub').value = pubkey;
    document.getElementById('form-challenge-id').value = data.challenge_id;
    document.getElementById('form-sig').value = signed.sig;
    document.getElementById('login-form').submit();
  } catch(e) {
    msg.textContent = 'Login failed: ' + e.message;
  }
})();
"#;

const REPO_FILTER_JS: &str = r#"
document.getElementById('repo-filter').addEventListener('input', function() {
  var q = this.value.toLowerCase();
  var cards = document.querySelectorAll('.repo-card');
  var visible = 0;
  cards.forEach(function(c) {
    var show = !q || c.dataset.name.indexOf(q) !== -1;
    c.style.display = show ? '' : 'none';
    if (show) visible++;
  });
  document.getElementById('repo-no-match').style.display = visible ? 'none' : '';
});
"#;

/// Render markdown to HTML.
pub fn render_markdown(source: &str) -> String {
    let input = if source.len() > MAX_MARKDOWN_BYTES {
        &source[..MAX_MARKDOWN_BYTES]
    } else {
        source
    };
    let opts = pulldown_cmark::Options::ENABLE_TABLES
        | pulldown_cmark::Options::ENABLE_STRIKETHROUGH
        | pulldown_cmark::Options::ENABLE_TASKLISTS;
    let parser = pulldown_cmark::Parser::new_ext(input, opts);
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, parser);
    html
}

/// Repo navigation tabs with optional search box.
fn repo_tabs(repo: &ForgeRepoInfo, active: &str) -> Markup {
    let tabs = [
        ("Overview", format!("/{}", repo.id), "overview"),
        ("Files", format!("/{}/tree", repo.id), "tree"),
        ("Commits", format!("/{}/commits", repo.id), "commits"),
        ("Issues", format!("/{}/issues", repo.id), "issues"),
        ("Patches", format!("/{}/patches", repo.id), "patches"),
    ];
    html! {
        div.tabs {
            @for (label, href, key) in &tabs {
                @if *key == active {
                    a.active href=(href) { (label) }
                } @else {
                    a href=(href) { (label) }
                }
            }
            form.tabs-search method="get" action=(format!("/{}/search", repo.id)) {
                input type="text" name="q" placeholder="Search…" {}
            }
        }
    }
}

/// Render an author's display name.
///
/// Priority: npub (truncated bech32-style) > author_name > ed25519 key (truncated).
fn author_display(commit: &ForgeCommitInfo) -> String {
    if let Some(ref npub) = commit.author_npub {
        // Show truncated npub: first 8 + last 4 chars
        if npub.len() > 16 {
            format!("npub:{}…{}", &npub[..8], &npub[npub.len() - 4..])
        } else {
            format!("npub:{npub}")
        }
    } else if !commit.author_name.is_empty() {
        commit.author_name.clone()
    } else if let Some(ref key) = commit.author_key {
        short_hash(key).to_string()
    } else {
        "unknown".to_string()
    }
}

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
                nav {
                    a href="/" { "🌲 Aspen Forge" }
                    a.nav-login href="/login" { "Login" }
                }
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
            input #repo-filter type="text" placeholder="Filter repositories…" autocomplete="off" {}
            p #repo-no-match.muted style="display:none" { "No matching repositories." }
            @for repo in repos {
                div.card.repo-card data-name=(repo.name.to_lowercase()) {
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
            script { (PreEscaped(REPO_FILTER_JS)) }
        }
    })
}

/// Repository overview page.
pub fn repo_overview(
    repo: &ForgeRepoInfo,
    branches: &[ForgeRefInfo],
    recent_commits: &[ForgeCommitInfo],
    readme_html: Option<&str>,
    ticket: &str,
) -> Markup {
    let clone_cmd = format!("git clone aspen://{}/{} {}", ticket, repo.id, repo.name);
    base_layout(&repo.name, html! {
        h1 { (&repo.name) }
        @if let Some(ref desc) = repo.description {
            p.muted { (desc) }
        }

        div.clone-box {
            span.muted { "Clone: " }
            code { (clone_cmd) }
        }

        (repo_tabs(repo, "overview"))

        @if let Some(html) = readme_html {
            div.readme {
                h2 { "📖 README" }
                div.markdown { (PreEscaped(html)) }
            }
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

        (repo_tabs(repo, "tree"))

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
                                td.ln id=(format!("L{}", line_no + 1)) { (line_no + 1) }
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

/// Commit detail page with diff.
pub fn commit_detail(
    repo: &ForgeRepoInfo,
    commit: &ForgeCommitInfo,
    file_diffs: &[(&crate::state::FileDiff, Vec<crate::state::DiffLine>)],
    truncated: bool,
) -> Markup {
    use crate::state::DiffKind;
    use crate::state::DiffLine;

    base_layout(&format!("{} — {}", repo.name, short_hash(&commit.hash)), html! {
        h1 {
            a href=(format!("/{}", repo.id)) { (&repo.name) }
            " / commit"
        }

        div.card {
            p { strong { (first_line(&commit.message)) } }
            @let rest = commit.message.lines().skip(1).collect::<Vec<_>>().join("\n").trim().to_string();
            @if !rest.is_empty() {
                pre.code-block { (rest) }
            }
            p.meta {
                (author_display(commit))
                " · " (format_time(commit.timestamp_ms))
            }
            p.commit-meta {
                span.hash { "commit " code { (&commit.hash) } }
            }
            @if !commit.parents.is_empty() {
                p.meta {
                    "parent "
                    @for (i, parent) in commit.parents.iter().enumerate() {
                        @if i > 0 { ", " }
                        a href=(format!("/{}/commit/{}", repo.id, parent)) { code { (short_hash(parent)) } }
                    }
                }
            }
        }

        p.meta { (file_diffs.len()) " files changed" }

        @for (file, lines) in file_diffs {
            div.diff-file {
                div.diff-file-header {
                    (&file.path)
                    @match file.kind {
                        DiffKind::Added => { span.badge.added { "added" } },
                        DiffKind::Deleted => { span.badge.deleted { "deleted" } },
                        DiffKind::Modified => { span.badge.modified { "modified" } },
                    }
                }
                @if lines.is_empty() {
                    p.diff-limit { "No content changes" }
                } @else {
                    table.diff {
                        @for line in lines {
                            @match line {
                                DiffLine::Hunk(text) => {
                                    @for raw_line in text.lines() {
                                        @if raw_line.starts_with("@@") {
                                            tr.hunk { td colspan="2" { (raw_line) } }
                                        } @else if raw_line.starts_with('+') {
                                            tr.add { td.ln {} td { (raw_line) } }
                                        } @else if raw_line.starts_with('-') {
                                            tr.del { td.ln {} td { (raw_line) } }
                                        } @else {
                                            tr { td.ln {} td { (raw_line) } }
                                        }
                                    }
                                },
                            }
                        }
                    }
                }
            }
        }

        @if truncated {
            p.diff-limit { "Too many files changed — showing first 50" }
        }

        p { a href=(format!("/{}/commits", repo.id)) { "← Back to commits" } }
    })
}

/// Commit log page.
pub fn commit_log(repo: &ForgeRepoInfo, commits: &[ForgeCommitInfo]) -> Markup {
    base_layout(&format!("{} — Commits", repo.name), html! {
        h1 { a href=(format!("/{}", repo.id)) { (&repo.name) } " — Commits" }

        (repo_tabs(repo, "commits"))

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

        (repo_tabs(repo, "issues"))

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
        div.issue-header {
            span class={ "badge " (if issue.state == "open" { "open" } else { "closed" }) } {
                (&issue.state)
            }
            @if issue.state == "open" {
                form.inline method="POST" action=(format!("/{}/issues/{}/close", repo.id, issue.id)) {
                    button.btn-secondary type="submit" { "Close Issue" }
                }
            } @else {
                form.inline method="POST" action=(format!("/{}/issues/{}/reopen", repo.id, issue.id)) {
                    button.btn-secondary type="submit" { "Reopen Issue" }
                }
            }
        }
        div.card { div.markdown { (PreEscaped(render_markdown(&issue.body))) } }

        h2 { "Comments (" (comments.len()) ")" }
        @for c in comments {
            div.card {
                p.meta { code { (short_hash(&c.author)) } " · " (format_time(c.timestamp_ms)) }
                div.markdown { (PreEscaped(render_markdown(&c.body))) }
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

        (repo_tabs(repo, "patches"))

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
            div.card { div.markdown { (PreEscaped(render_markdown(&patch.description))) } }
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

/// Code search results page.
pub fn search_results(repo: &ForgeRepoInfo, query: &str, results: Option<&crate::state::SearchResults>) -> Markup {
    base_layout(&format!("{} — Search", repo.name), html! {
        h1 {
            a href=(format!("/{}", repo.id)) { (&repo.name) }
            " — Search"
        }

        (repo_tabs(repo, "search"))

        form method="get" action=(format!("/{}/search", repo.id)) {
            input.search-input type="text" name="q" value=(query)
                placeholder="Search code…" autocomplete="off" autofocus {}
        }

        @if query.len() < 2 && !query.is_empty() {
            p.muted { "Enter at least 2 characters." }
        } @else if let Some(res) = results {
            @if res.files.is_empty() {
                p.muted { "No results for "" code { (query) } """ }
            } @else {
                p.meta {
                    (res.total_matches) " matches in " (res.files.len()) " files"
                    " (" (res.files_examined) " files searched)"
                    @if res.truncated { " — results truncated" }
                }
                @for file in &res.files {
                    div.search-file {
                        div.search-file-header {
                            a href=(format!("/{}/blob/{}/{}", repo.id, repo.default_branch, file.path)) {
                                (&file.path)
                            }
                        }
                        table.search-matches {
                            @for m in &file.matches {
                                @for (ln, line) in &m.context_before {
                                    tr.ctx {
                                        td.ln { (ln) }
                                        td { code { (line) } }
                                    }
                                }
                                tr.hit {
                                    td.ln {
                                        a href=(format!("/{}/blob/{}/{}#L{}",
                                            repo.id, repo.default_branch, file.path, m.line_number)) {
                                            (m.line_number)
                                        }
                                    }
                                    td { code { (&m.line) } }
                                }
                                @for (ln, line) in &m.context_after {
                                    tr.ctx {
                                        td.ln { (ln) }
                                        td { code { (line) } }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    })
}

/// Login page with NIP-07 browser extension support.
pub fn login_page() -> Markup {
    base_layout("Login", html! {
        h1 { "Login with Nostr" }

        div #login-status.card {
            p #login-msg { "Checking for Nostr extension…" }
        }

        // Hidden form submitted by JS after signing
        form #login-form method="post" action="/login/verify" style="display:none" {
            input type="hidden" name="npub" #form-npub {}
            input type="hidden" name="challenge_id" #form-challenge-id {}
            input type="hidden" name="signature" #form-sig {}
        }

        script { (PreEscaped(LOGIN_JS)) }

        noscript {
            div.card {
                p { "JavaScript is required for NIP-07 login." }
                p.muted { "Your Nostr key never leaves your browser — the extension signs locally." }
            }
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

/// Error page shown when the forge app is not loaded on the cluster.
pub fn forge_unavailable(message: &str, hints: &[aspen_client_api::CapabilityHint]) -> Markup {
    base_layout("Forge Unavailable", html! {
        h1 { "Forge Unavailable" }
        p { (message) }
        p.muted {
            "The cluster this web frontend is connected to does not have the "
            code { "forge" }
            " app loaded. Start the node with "
            code { "--features forge,blob" }
            " and ensure blob storage is enabled."
        }
        @if !hints.is_empty() {
            h2 { "Clusters with Forge" }
            p.muted { "These discovered clusters have the forge app:" }
            ul {
                @for hint in hints {
                    li {
                        strong { (&hint.name) }
                        " — " code { (&hint.cluster_key) }
                        @if let Some(ref ver) = hint.app_version {
                            " (v" (ver) ")"
                        }
                    }
                }
            }
        }
    })
}

// ── Helpers ──────────────────────────────────────────────────────────

fn commit_table(repo: &ForgeRepoInfo, commits: &[ForgeCommitInfo]) -> Markup {
    html! {
        table.commits {
            @for c in commits {
                tr {
                    td.hash {
                        a href=(format!("/{}/commit/{}", repo.id, c.hash)) {
                            code { (short_hash(&c.hash)) }
                        }
                    }
                    td { (first_line(&c.message)) }
                    td.meta { (author_display(c)) }
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
nav{display:flex;justify-content:space-between;align-items:center}
nav a{color:#f0f6fc;font-weight:600;font-size:1.1rem}
.nav-login{font-weight:400 !important;font-size:.9rem !important;color:#8b949e !important}
.nav-login:hover{color:#58a6ff !important}
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
.issue-header{display:flex;align-items:center;gap:.75rem;margin:.5rem 0}
form.inline{display:inline}
.btn-secondary{padding:.35rem .7rem;background:transparent;color:#c9d1d9;border:1px solid #30363d;border-radius:4px;font-size:.8rem;cursor:pointer}
.btn-secondary:hover{border-color:#da3633;color:#f85149}
.readme{background:#161b22;border:1px solid #30363d;border-radius:6px;padding:1.5rem;margin:1rem 0}
.readme h2{margin-top:0}
.markdown h1,.markdown h2,.markdown h3{color:#f0f6fc;border-bottom:1px solid #21262d;padding-bottom:.3em;margin:1.2em 0 .5em}
.markdown h1{font-size:1.4rem}
.markdown h2{font-size:1.2rem}
.markdown h3{font-size:1rem}
.markdown p{margin:.5em 0}
.markdown pre{background:#0d1117;border:1px solid #30363d;border-radius:4px;padding:.8rem;overflow-x:auto;font-size:.85rem}
.markdown code{font-size:.85em;background:#0d1117;padding:.1em .3em;border-radius:3px}
.markdown pre code{background:transparent;padding:0}
.markdown blockquote{border-left:3px solid #30363d;padding-left:.8rem;color:#8b949e;margin:.5em 0}
.markdown ul,.markdown ol{padding-left:1.5em;margin:.5em 0}
.markdown table{border-collapse:collapse;margin:.5em 0}
.markdown th,.markdown td{border:1px solid #30363d;padding:.4rem .6rem}
.markdown th{background:#161b22}
.markdown img{max-width:100%}
.markdown hr{border:none;border-top:1px solid #30363d;margin:1em 0}
.markdown input[type=checkbox]{margin-right:.4em}
#repo-filter{width:100%;padding:.5rem;background:#0d1117;color:#c9d1d9;border:1px solid #30363d;border-radius:6px;font-size:.9rem;margin:.75rem 0}
#repo-filter:focus{outline:none;border-color:#58a6ff}
.clone-box{background:#161b22;border:1px solid #30363d;border-radius:6px;padding:.5rem .75rem;margin:.75rem 0;font-size:.85rem;overflow-x:auto;white-space:nowrap}
.clone-box code{user-select:all;cursor:text}
.diff-file{border:1px solid #30363d;border-radius:6px;margin:.75rem 0;overflow:hidden}
.diff-file-header{background:#161b22;padding:.5rem .75rem;border-bottom:1px solid #30363d;font-weight:600;font-size:.9rem}
.diff-file-header .badge{margin-left:.5rem}
.badge.added{background:#238636;color:#fff}
.badge.deleted{background:#da3633;color:#fff}
.badge.modified{background:#1f6feb;color:#fff}
table.diff{width:100%;border-collapse:collapse;font-size:.82rem;line-height:1.45}
table.diff td{padding:0 .6rem;border:none;white-space:pre;font-family:'JetBrains Mono',monospace}
table.diff .ln{color:#484f58;text-align:right;user-select:none;width:1px;padding:0 .4rem}
table.diff tr.add{background:#0d2818}
table.diff tr.add td{color:#aff5b4}
table.diff tr.del{background:#2d1115}
table.diff tr.del td{color:#ffa198}
table.diff tr.hunk td{color:#58a6ff;background:#0d1117;padding:.2rem .6rem}
.diff-limit{padding:.75rem;color:#8b949e;font-style:italic;text-align:center}
.search-input{width:100%;padding:.5rem;background:#0d1117;color:#c9d1d9;border:1px solid #30363d;border-radius:6px;font-size:.9rem;margin:.5rem 0}
.search-input:focus{outline:none;border-color:#58a6ff}
.search-file{border:1px solid #30363d;border-radius:6px;margin:.75rem 0;overflow:hidden}
.search-file-header{background:#161b22;padding:.4rem .75rem;border-bottom:1px solid #30363d;font-size:.9rem}
table.search-matches{width:100%;border-collapse:collapse;font-size:.82rem;line-height:1.45}
table.search-matches td{padding:0 .6rem;border:none;white-space:pre;font-family:'JetBrains Mono',monospace}
table.search-matches .ln{color:#484f58;text-align:right;user-select:none;width:1px;padding:0 .4rem}
table.search-matches .ln a{color:inherit;text-decoration:none}
table.search-matches .ln a:hover{color:#58a6ff}
table.search-matches tr.hit{background:#2a1f00}
table.search-matches tr.hit td{color:#e3b341}
table.search-matches tr.ctx td{color:#8b949e}
.tabs-search{margin-left:auto}
.tabs-search input{padding:.25rem .5rem;background:#0d1117;color:#c9d1d9;border:1px solid #30363d;border-radius:4px;font-size:.8rem;width:10rem}
.tabs-search input:focus{outline:none;border-color:#58a6ff;width:14rem}
.commit-meta{margin:.75rem 0}
.commit-meta .hash{font-size:.9rem;color:#8b949e}
@media (max-width: 768px) {
    main{padding:0.75rem}
    table{font-size:0.85rem}
}
"#;
