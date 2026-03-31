//! HTML templates using maud for the Forge web frontend.

use aspen_client_api::messages::CiGetJobLogsResponse;
use aspen_client_api::messages::CiGetStatusResponse;
use aspen_client_api::messages::CiJobInfo;
use aspen_client_api::messages::CiRunInfo;
use aspen_client_api::messages::CiStageInfo;
use aspen_client_api::messages::ClusterStateResponse;
use aspen_client_api::messages::HealthResponse;
use aspen_forge_protocol::ForgeCommitInfo;
use aspen_forge_protocol::ForgeIssueInfo;
use aspen_forge_protocol::ForgeMergeCheckResultResponse;
use aspen_forge_protocol::ForgePatchInfo;
use aspen_forge_protocol::ForgeRefInfo;
use aspen_forge_protocol::ForgeRepoInfo;
use aspen_forge_protocol::ForgeTreeEntry;
use maud::DOCTYPE;
use maud::Markup;
use maud::PreEscaped;
use maud::html;

use crate::state::DiffKind;
use crate::state::FileDiff;

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
        ("CI", format!("/{}/ci", repo.id), "ci"),
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
/// Priority: resolved display name > npub (truncated) > author_name > ed25519 key (truncated).
fn author_display(commit: &ForgeCommitInfo) -> String {
    if let Some(ref name) = commit.author_display_name {
        return name.clone();
    }
    if let Some(ref npub) = commit.author_npub {
        if npub.len() > 16 {
            return format!("npub:{}…{}", &npub[..8], &npub[npub.len() - 4..]);
        }
        return format!("npub:{npub}");
    }
    if !commit.author_name.is_empty() {
        return commit.author_name.clone();
    }
    if let Some(ref key) = commit.author_key {
        return short_hash(key).to_string();
    }
    "unknown".to_string()
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
                    a.nav-link href="/ci" { "CI" }
                    a.nav-link href="/cluster" { "Cluster" }
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
    ci_status: Option<&CiRunInfo>,
) -> Markup {
    let clone_cmd = format!("git clone aspen://{}/{} {}", ticket, repo.id, repo.name);
    base_layout(&repo.name, html! {
        h1 {
            (&repo.name)
            @if let Some(run) = ci_status {
                " " (ci_status_badge(&repo.id, run))
            }
        }
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
    patch_detail_full(repo, patch, None, &[])
}

/// Full patch detail page with merge check, actions, and diff.
pub fn patch_detail_full(
    repo: &ForgeRepoInfo,
    patch: &ForgePatchInfo,
    merge_check: Option<&ForgeMergeCheckResultResponse>,
    diff: &[FileDiff],
) -> Markup {
    base_layout(&format!("{} — {}", repo.name, patch.title), html! {
        p { a href=(format!("/{}/patches", repo.id)) { "← Patches" } }
        h1 { (&patch.title) }
        span class={ "badge " (match patch.state.as_str() {
            "open" => "open",
            "merged" => "merged",
            _ => "closed",
        }) } {
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

        // Merge status banner and actions (open patches only)
        @if patch.state == "open" {
            (merge_status_banner(repo, patch, merge_check))
        }

        // File diff section
        @if !diff.is_empty() {
            h2 { "Changed files (" (diff.len()) ")" }
            table.diff-table {
                thead { tr { th { "Status" } th { "Path" } } }
                tbody {
                    @for file in diff {
                        tr {
                            td {
                                span class={ "diff-badge " (match file.kind {
                                    DiffKind::Added => "added",
                                    DiffKind::Modified => "modified",
                                    DiffKind::Deleted => "deleted",
                                }) } {
                                    (match file.kind {
                                        DiffKind::Added => "A",
                                        DiffKind::Modified => "M",
                                        DiffKind::Deleted => "D",
                                    })
                                }
                            }
                            td { code { (&file.path) } }
                        }
                    }
                }
            }
        } @else if patch.state == "open" {
            p.muted { "No file changes detected." }
        }
    })
}

/// Merge status banner with action buttons.
fn merge_status_banner(
    repo: &ForgeRepoInfo,
    patch: &ForgePatchInfo,
    merge_check: Option<&ForgeMergeCheckResultResponse>,
) -> Markup {
    html! {
        div.merge-section {
            @if let Some(check) = merge_check {
                // Status banner
                div class={ "merge-banner " (if check.mergeable { "merge-ready" }
                    else if !check.conflicts.is_empty() { "merge-conflict" }
                    else { "merge-blocked" }) } {

                    @if check.mergeable {
                        strong { "✓ Ready to merge" }
                    } @else if !check.conflicts.is_empty() {
                        strong { "✗ Merge conflicts" }
                        ul {
                            @for path in &check.conflicts {
                                li { code { (path) } }
                            }
                        }
                    } @else if let Some(reason) = &check.protection_reason {
                        strong { "⏳ Blocked" }
                        " — " (reason)
                    } @else {
                        strong { "✗ Cannot merge" }
                    }
                }

                // Action buttons
                div.merge-actions {
                    // Approve button
                    form method="POST"
                        action=(format!("/{}/patches/{}/approve", repo.id, patch.id))
                        style="display:inline" {
                        button type="submit" class="btn-approve" { "Approve" }
                    }

                    // Merge button with strategy
                    @if check.mergeable {
                        form method="POST"
                            action=(format!("/{}/patches/{}/merge", repo.id, patch.id))
                            style="display:inline" {
                            select name="strategy" {
                                option value="merge" selected { "Merge commit" }
                                @if check.available_strategies.iter().any(|s| s == "fast-forward") {
                                    option value="fast-forward" { "Fast-forward" }
                                }
                                option value="squash" { "Squash" }
                            }
                            " "
                            button type="submit" class="btn-merge" { "Merge" }
                        }
                    } @else {
                        button type="button" class="btn-merge" disabled { "Merge" }
                    }
                }
            } @else {
                div class="merge-banner merge-blocked" {
                    "Merge check unavailable"
                }

                // Still show approve button
                div.merge-actions {
                    form method="POST"
                        action=(format!("/{}/patches/{}/approve", repo.id, patch.id))
                        style="display:inline" {
                        button type="submit" class="btn-approve" { "Approve" }
                    }
                }
            }
        }
    }
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

// ── CI Templates ─────────────────────────────────────────────────────

/// Convert ANSI escape codes to HTML. Returns safe HTML.
fn ansi_to_html_safe(input: &str) -> String {
    ansi_to_html::convert(input).unwrap_or_else(|_| maud::html! { (input) }.into_string())
}

/// CSS class for a pipeline status badge.
fn status_badge_class(status: &str) -> &'static str {
    match status {
        "succeeded" => "badge ci-succeeded",
        "failed" => "badge ci-failed",
        "running" => "badge ci-running",
        "pending" => "badge ci-pending",
        "cancelled" => "badge ci-cancelled",
        _ => "badge ci-pending",
    }
}

/// Human-friendly status label.
fn status_label(status: &str) -> &str {
    match status {
        "succeeded" => "passed",
        "failed" => "failed",
        "running" => "running",
        "pending" => "pending",
        "cancelled" => "cancelled",
        other => other,
    }
}

/// Format a duration from start/end timestamps.
fn format_duration_ms(started_ms: Option<u64>, ended_ms: Option<u64>) -> String {
    match (started_ms, ended_ms) {
        (Some(start), Some(end)) if end >= start => {
            let secs = (end - start) / 1000;
            if secs < 60 {
                format!("{secs}s")
            } else {
                format!("{}m {}s", secs / 60, secs % 60)
            }
        }
        (Some(_), None) => "running…".to_string(),
        _ => "-".to_string(),
    }
}

/// CI pipeline list page (all repos or per-repo).
pub fn pipeline_list(
    runs: &[CiRunInfo],
    repo_id: Option<&str>,
    repo_name: Option<&str>,
    active_filter: &str,
) -> Markup {
    let title = match repo_name {
        Some(name) => format!("{name} — CI Pipelines"),
        None => "CI Pipelines".to_string(),
    };
    let base_path = match repo_id {
        Some(id) => format!("/{id}/ci"),
        None => "/ci".to_string(),
    };
    base_layout(&title, html! {
        @if let Some(name) = repo_name {
            @if let Some(id) = repo_id {
                (repo_tabs_with_ci(&ForgeRepoInfo { id: id.to_string(), name: name.to_string(), ..dummy_repo() }, "ci"))
            }
        }
        h1 { (title) }
        div.ci-filters {
            @for (label, filter_val) in [("All", ""), ("Running", "running"), ("Passed", "succeeded"), ("Failed", "failed")] {
                @if *filter_val == *active_filter {
                    span.ci-filter-active { (label) }
                } @else if filter_val.is_empty() {
                    a href=(&base_path) { (label) }
                } @else {
                    a href=(format!("{base_path}?status={filter_val}")) { (label) }
                }
            }
        }
        @if runs.is_empty() {
            p.muted { "No pipeline runs yet." }
        } @else {
            table.ci-runs {
                thead {
                    tr {
                        th { "Status" }
                        th { "Pipeline" }
                        th { "Ref" }
                        th { "Created" }
                    }
                }
                tbody {
                    @for run in runs {
                        tr {
                            td {
                                span class=(status_badge_class(&run.status)) { (status_label(&run.status)) }
                            }
                            td {
                                a href=(format!("/{}/ci/{}", run.repo_id, run.run_id)) {
                                    code { (short_hash(&run.run_id)) }
                                }
                            }
                            td { code { (&run.ref_name) } }
                            td.meta { (format_time(run.created_at_ms)) }
                        }
                    }
                }
            }
        }
    })
}

/// CI pipeline run detail page with stages and jobs.
pub fn pipeline_detail(repo_id: &str, repo_name: &str, status_resp: &CiGetStatusResponse) -> Markup {
    let run_id = status_resp.run_id.as_deref().unwrap_or("unknown");
    let status = status_resp.status.as_deref().unwrap_or("unknown");
    let is_active = status == "running" || status == "pending";
    let title = format!("{repo_name} — Pipeline {}", short_hash(run_id));

    base_layout(&title, html! {
        (repo_tabs_with_ci(&ForgeRepoInfo { id: repo_id.to_string(), name: repo_name.to_string(), ..dummy_repo() }, "ci"))
        @if is_active {
            meta http-equiv="refresh" content="5";
        }
        h1 {
            "Pipeline " code { (short_hash(run_id)) }
            " "
            span class=(status_badge_class(status)) { (status_label(status)) }
        }
        div.commit-meta {
            @if let Some(ref ref_name) = status_resp.ref_name {
                "Ref: " code { (ref_name) }
            }
            @if let Some(ref commit) = status_resp.commit_hash {
                " · Commit: " code { (short_hash(commit)) }
            }
            @if let Some(created) = status_resp.created_at_ms {
                " · Created: " (format_time(created))
            }
        }
        @for stage in &status_resp.stages {
            (stage_section(repo_id, run_id, stage))
        }
        @if status_resp.stages.is_empty() {
            p.muted { "No stages defined for this pipeline." }
        }
    })
}

/// Render a single stage section with its jobs.
fn stage_section(repo_id: &str, run_id: &str, stage: &CiStageInfo) -> Markup {
    html! {
        div.ci-stage {
            h3.ci-stage-header {
                span class=(status_badge_class(&stage.status)) { (status_label(&stage.status)) }
                " "
                (&stage.name)
            }
            table.ci-jobs {
                thead {
                    tr {
                        th { "Status" }
                        th { "Job" }
                        th { "Duration" }
                        th { "" }
                    }
                }
                tbody {
                    @for job in &stage.jobs {
                        (job_row(repo_id, run_id, job))
                    }
                }
            }
        }
    }
}

/// Render a single job row.
fn job_row(repo_id: &str, run_id: &str, job: &CiJobInfo) -> Markup {
    html! {
        tr {
            td {
                span class=(status_badge_class(&job.status)) { (status_label(&job.status)) }
            }
            td { (&job.name) }
            td.meta { (format_duration_ms(job.started_at_ms, job.ended_at_ms)) }
            td {
                a href=(format!("/{repo_id}/ci/{run_id}/{}", job.id)) { "logs →" }
            }
        }
    }
}

/// Parameters for rendering a CI job log page.
pub struct JobLogParams<'a> {
    pub repo_id: &'a str,
    pub repo_name: &'a str,
    pub run_id: &'a str,
    pub job_id: &'a str,
    pub job_name: &'a str,
    pub job_status: &'a str,
    pub job_started_ms: Option<u64>,
    pub job_ended_ms: Option<u64>,
    pub logs_resp: &'a CiGetJobLogsResponse,
    pub start_index: u32,
}

/// CI job log viewer page.
pub fn job_log_viewer(params: &JobLogParams<'_>) -> Markup {
    let is_active = params.job_status == "running";
    let title = format!("{} — {} logs", params.repo_name, params.job_name);

    let mut log_html = String::new();
    for chunk in &params.logs_resp.chunks {
        log_html.push_str(&ansi_to_html_safe(&chunk.content));
    }

    base_layout(&title, html! {
        (repo_tabs_with_ci(&ForgeRepoInfo { id: params.repo_id.to_string(), name: params.repo_name.to_string(), ..dummy_repo() }, "ci"))
        @if is_active {
            meta http-equiv="refresh" content="5";
        }
        div.ci-job-header {
            h1 {
                (params.job_name)
                " "
                span class=(status_badge_class(params.job_status)) { (status_label(params.job_status)) }
            }
            div.meta {
                "Duration: " (format_duration_ms(params.job_started_ms, params.job_ended_ms))
                " · "
                a href=(format!("/{}/ci/{}", params.repo_id, params.run_id)) { "← Back to pipeline" }
            }
        }
        pre.log-viewer {
            (PreEscaped(&log_html))
        }
        @if params.logs_resp.has_more {
            div.ci-load-more {
                a href=(format!("/{}/ci/{}/{}?start={}", params.repo_id, params.run_id, params.job_id, params.start_index + params.logs_resp.chunks.len() as u32)) {
                    "Load more…"
                }
            }
        }
    })
}

/// CI status badge for the repo overview page.
pub fn ci_status_badge(repo_id: &str, run: &CiRunInfo) -> Markup {
    let label = match run.status.as_str() {
        "succeeded" => "CI: passing",
        "failed" => "CI: failing",
        "running" => "CI: running",
        _ => "CI: pending",
    };
    html! {
        a.ci-badge-link href=(format!("/{repo_id}/ci/{}", run.run_id)) {
            span class=(status_badge_class(&run.status)) { (label) }
        }
    }
}

/// CI unavailable page.
pub fn ci_unavailable() -> Markup {
    base_layout("CI Unavailable", html! {
        h1 { "CI Unavailable" }
        div.card {
            p { "CI pipelines are not available on this cluster." }
            p.muted {
                "The cluster node needs to be started with CI features enabled."
                br;
                "Try: " code { "--features ci" }
            }
        }
    })
}

// ── Cluster Templates ────────────────────────────────────────────────

/// Cluster overview page.
pub fn cluster_overview(health: &HealthResponse, cluster: &ClusterStateResponse) -> Markup {
    let total_nodes = cluster.nodes.len();
    let leader_id = cluster.leader_id;

    base_layout("Cluster Overview", html! {
        meta http-equiv="refresh" content="10";
        h1 { "Cluster Overview" }
        div.cluster-summary {
            div.cluster-stat {
                span.cluster-stat-value { (total_nodes) }
                span.cluster-stat-label { "nodes" }
            }
            div.cluster-stat {
                span.cluster-stat-value { (health.status) }
                span.cluster-stat-label { "health" }
            }
            @if let Some(lid) = leader_id {
                div.cluster-stat {
                    span.cluster-stat-value { (lid) }
                    span.cluster-stat-label { "leader" }
                }
            }
            div.cluster-stat {
                span.cluster-stat-value { (format_uptime_secs(health.uptime_seconds)) }
                span.cluster-stat-label { "uptime" }
            }
        }
        table.cluster-nodes {
            thead {
                tr {
                    th { "Node" }
                    th { "Role" }
                    th { "Health" }
                }
            }
            tbody {
                @for node in &cluster.nodes {
                    tr {
                        td { "Node " (node.node_id) }
                        td {
                            @if node.is_leader {
                                span.badge.cluster-leader { "leader" }
                            } @else if node.is_learner {
                                span.badge.cluster-learner { "learner" }
                            } @else {
                                span.badge.cluster-follower { "follower" }
                            }
                        }
                        td {
                            span.badge.cluster-healthy { "healthy" }
                        }
                    }
                }
            }
        }
    })
}

/// Format uptime from seconds into human-readable.
fn format_uptime_secs(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

/// Create a minimal dummy ForgeRepoInfo for tab rendering.
fn dummy_repo() -> ForgeRepoInfo {
    ForgeRepoInfo {
        id: String::new(),
        name: String::new(),
        description: None,
        default_branch: "main".to_string(),
        delegates: vec![],
        threshold_delegates: 0,
        created_at_ms: 0,
    }
}

/// Repo tabs variant that includes the CI tab.
fn repo_tabs_with_ci(repo: &ForgeRepoInfo, active: &str) -> Markup {
    let tabs = [
        ("Overview", format!("/{}", repo.id), "overview"),
        ("Files", format!("/{}/tree", repo.id), "tree"),
        ("Commits", format!("/{}/commits", repo.id), "commits"),
        ("Issues", format!("/{}/issues", repo.id), "issues"),
        ("Patches", format!("/{}/patches", repo.id), "patches"),
        ("CI", format!("/{}/ci", repo.id), "ci"),
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
.nav-link{font-weight:400 !important;font-size:.9rem !important;color:#8b949e !important;margin-left:1rem}
.nav-link:hover{color:#58a6ff !important}
.badge.ci-succeeded{background:#238636;color:#fff}
.badge.ci-failed{background:#da3633;color:#fff}
.badge.ci-running{background:#9e6a03;color:#fff}
.badge.ci-pending{background:#484f58;color:#c9d1d9}
.badge.ci-cancelled{background:#484f58;color:#8b949e}
.ci-filters{display:flex;gap:1rem;margin:.75rem 0}
.ci-filters a,.ci-filter-active{padding:.25rem .6rem;border-radius:4px;font-size:.85rem}
.ci-filter-active{background:#30363d;color:#f0f6fc;font-weight:600}
table.ci-runs{width:100%;border-collapse:collapse}
table.ci-runs th{text-align:left;padding:.4rem .6rem;border-bottom:2px solid #30363d;font-size:.85rem;color:#8b949e}
table.ci-runs td{padding:.4rem .6rem;border-bottom:1px solid #21262d}
table.ci-runs tr:hover{background:#161b22}
.ci-stage{border:1px solid #30363d;border-radius:6px;margin:.75rem 0;overflow:hidden}
.ci-stage-header{background:#161b22;padding:.5rem .75rem;border-bottom:1px solid #30363d;display:flex;align-items:center;gap:.5rem}
table.ci-jobs{width:100%;border-collapse:collapse}
table.ci-jobs th{text-align:left;padding:.3rem .6rem;font-size:.8rem;color:#8b949e}
table.ci-jobs td{padding:.3rem .6rem;border-bottom:1px solid #21262d}
table.ci-jobs tr:hover{background:#0d1117}
.ci-job-header{margin:.75rem 0}
.log-viewer{background:#0d1117;border:1px solid #30363d;border-radius:6px;padding:1rem;overflow-x:auto;font-family:'JetBrains Mono',monospace;font-size:.8rem;line-height:1.5;color:#c9d1d9;white-space:pre-wrap;word-break:break-all;max-height:80vh;overflow-y:auto}
.ci-load-more{text-align:center;padding:.75rem}
.ci-badge-link{text-decoration:none}
.ci-badge-link:hover{text-decoration:none}
.cluster-summary{display:flex;gap:1.5rem;margin:1rem 0;flex-wrap:wrap}
.cluster-stat{background:#161b22;border:1px solid #30363d;border-radius:6px;padding:.75rem 1.25rem;text-align:center;min-width:100px}
.cluster-stat-value{display:block;font-size:1.5rem;font-weight:700;color:#f0f6fc}
.cluster-stat-label{display:block;font-size:.8rem;color:#8b949e}
table.cluster-nodes{width:100%;border-collapse:collapse;margin:.75rem 0}
table.cluster-nodes th{text-align:left;padding:.4rem .6rem;border-bottom:2px solid #30363d;font-size:.85rem;color:#8b949e}
table.cluster-nodes td{padding:.4rem .6rem;border-bottom:1px solid #21262d}
.badge.cluster-leader{background:#1f6feb;color:#fff}
.badge.cluster-follower{background:#30363d;color:#c9d1d9}
.badge.cluster-learner{background:#9e6a03;color:#fff}
.badge.cluster-healthy{background:#238636;color:#fff}
.badge.cluster-unreachable{background:#da3633;color:#fff}
@media (max-width: 768px) {
    main{padding:0.75rem}
    table{font-size:0.85rem}
    .cluster-summary{flex-direction:column}
}
"#;
