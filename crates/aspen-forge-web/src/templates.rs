//! HTML templates using maud for the Forge web frontend.

use aspen_client_api::messages::CiArtifactInfo;
use aspen_client_api::messages::CiGetJobLogsResponse;
use aspen_client_api::messages::CiGetJobOutputResponse;
use aspen_client_api::messages::CiGetStatusResponse;
use aspen_client_api::messages::CiJobInfo;
use aspen_client_api::messages::CiListArtifactsResponse;
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
    render_repo_tabs(repo, active)
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
            let suffix_start = npub.len().saturating_sub(4);
            debug_assert!(suffix_start < npub.len());
            return format!("npub:{}…{}", &npub[..8], &npub[suffix_start..]);
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
        html lang="en" data-theme="phosphor" {
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

pub struct RepoOverviewParams<'a> {
    pub branches: &'a [ForgeRefInfo],
    pub recent_commits: &'a [ForgeCommitInfo],
    pub readme_html: Option<&'a str>,
    pub ticket: &'a str,
    pub ci_status: Option<&'a CiRunInfo>,
    pub branch_ci: &'a std::collections::HashMap<String, String>,
}

/// Repository overview page.
pub fn repo_overview(repo: &ForgeRepoInfo, params: &RepoOverviewParams<'_>) -> Markup {
    debug_assert!(!params.ticket.is_empty());
    debug_assert!(params.branch_ci.keys().all(|branch_name| !branch_name.is_empty()));
    let clone_cmd = format!("git clone aspen://<cluster-ticket>/{} {}", repo.id, repo.name);
    base_layout(&repo.name, html! {
        h1 {
            (&repo.name)
            @if let Some(run) = params.ci_status {
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

        @if let Some(html) = params.readme_html {
            div.readme {
                h2 { "📖 README" }
                div.markdown { (PreEscaped(html)) }
            }
        }

        h2 { "Branches" }
        @if params.branches.is_empty() {
            p.muted { "No branches." }
        } @else {
            ul {
                @for b in params.branches {
                    li {
                        @if let Some(status) = params.branch_ci.get(&b.name) {
                            span class=(branch_ci_dot_class(status)) { "" }
                            " "
                        }
                        a href=(format!("/{}/tree/{}", repo.id, b.name)) { (&b.name) }
                        " → " code { (short_hash(&b.hash)) }
                    }
                }
            }
        }

        h2 { "Recent Commits" }
        @if params.recent_commits.is_empty() {
            p.muted { "No commits." }
        } @else {
            (commit_table(repo, params.recent_commits))
        }
    })
}

pub struct FileBrowserParams<'a> {
    pub ref_name: &'a str,
    pub path: &'a str,
    pub entries: &'a [ForgeTreeEntry],
}

/// File browser page (directory listing).
pub fn file_browser(repo: &ForgeRepoInfo, params: &FileBrowserParams<'_>) -> Markup {
    base_layout(&format!("{} — {}", repo.name, if params.path.is_empty() { "/" } else { params.path }), html! {
        h1 {
            a href=(format!("/{}", repo.id)) { (&repo.name) }
            " / "
            a href=(format!("/{}/tree/{}", repo.id, params.ref_name)) { code { (params.ref_name) } }
            @if !params.path.is_empty() {
                (breadcrumb_path(repo, params.ref_name, params.path))
            }
        }

        (repo_tabs(repo, "tree"))

        table.file-list {
            @if !params.path.is_empty() {
                tr {
                    td { a href=(parent_link(&TreeLinkContext { repo, ref_name: params.ref_name, path: params.path })) { "📁 .." } }
                    td {}
                }
            }
            @for entry in params.entries {
                tr {
                    td {
                        @let link = entry_link(&TreeLinkContext { repo, ref_name: params.ref_name, path: params.path }, &entry.name, entry.mode);
                        a href=(link) { (file_icon(entry.mode)) " " (&entry.name) }
                    }
                    td.hash { code { (short_hash(&entry.hash)) } }
                }
            }
        }
    })
}

pub struct FileViewParams<'a> {
    pub ref_name: &'a str,
    pub path: &'a str,
    pub content: Option<&'a [u8]>,
    pub size_bytes: u64,
}

/// Blob (file content) page.
pub fn file_view(repo: &ForgeRepoInfo, params: &FileViewParams<'_>) -> Markup {
    debug_assert!(!params.ref_name.is_empty());
    debug_assert!(!params.path.is_empty());
    let is_text = params.content.as_ref().is_some_and(|c| is_likely_text(c));
    let raw_url = format!("/{}/raw/{}/{}", repo.id, params.ref_name, params.path);
    base_layout(&format!("{} — {}", repo.name, params.path), html! {
        h1 {
            a href=(format!("/{}", repo.id)) { (&repo.name) }
            " / "
            code { (params.ref_name) }
            " / " (params.path)
        }

        p.meta {
            (params.size_bytes) " bytes"
            " · "
            a.raw-link href=(raw_url) { "Raw" }
        }

        @if is_text {
            @if let Some(bytes) = params.content {
                pre.code-block {
                    table.code-table {
                        @let text = String::from_utf8_lossy(bytes);
                        @for (line_no, line) in text.lines().enumerate() {
                            @let display_line_no = checked_line_number(line_no);
                            tr {
                                td.ln id=(format!("L{}", display_line_no)) { (display_line_no) }
                                td.line { code { (line) } }
                            }
                        }
                    }
                }
            }
        } @else {
            p.muted {
                "Binary file (" (params.size_bytes) " bytes) · "
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
    ci_statuses: &[crate::state::CommitStatusEntry],
) -> Markup {
    use crate::state::DiffKind;
    use crate::state::DiffLine;

    debug_assert!(!commit.hash.is_empty());
    debug_assert!(file_diffs.iter().all(|(file, _)| !file.path.is_empty()));
    base_layout(&format!("{} — {}", repo.name, short_hash(&commit.hash)), html! {
        h1 {
            a href=(format!("/{}", repo.id)) { (&repo.name) }
            " / commit"
        }

        @if !ci_statuses.is_empty() {
            (commit_status_badges(&repo.id, ci_statuses))
        }

        div.card {
            p { strong { (first_line(&commit.message)) } }
            @let rest = commit_message_rest(&commit.message);
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

struct NewIssueFormDraft<'a> {
    error: Option<&'a str>,
    title: &'a str,
    body: &'a str,
}

/// New issue form page.
pub fn new_issue_form(repo: &ForgeRepoInfo) -> Markup {
    new_issue_form_inner(repo, &NewIssueFormDraft {
        error: None,
        title: "",
        body: "",
    })
}

/// New issue form with validation error.
pub fn new_issue_form_with_error(repo: &ForgeRepoInfo, error: &str, title: &str, body: &str) -> Markup {
    new_issue_form_inner(repo, &NewIssueFormDraft {
        error: Some(error),
        title,
        body,
    })
}

fn new_issue_form_inner(repo: &ForgeRepoInfo, draft: &NewIssueFormDraft<'_>) -> Markup {
    base_layout(&format!("{} — New Issue", repo.name), html! {
        p { a href=(format!("/{}/issues", repo.id)) { "← Issues" } }
        h1 { "New Issue" }

        @if let Some(msg) = draft.error {
            div.error { (msg) }
        }

        form.card method="POST" action=(format!("/{}/issues/new", repo.id)) {
            label for="title" { "Title" }
            input type="text" name="title" id="title" value=(draft.title) required
                placeholder="Issue title" {}

            label for="body" { "Description" }
            textarea name="body" id="body" rows="8" placeholder="Describe the issue…" { (draft.body) }

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
            let elapsed_ms = end.saturating_sub(start);
            let secs = elapsed_ms / 1000;
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
    debug_assert_eq!(repo_id.is_some(), repo_name.is_some());
    debug_assert!(!active_filter.contains('&'));
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
                        th { "Duration" }
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
                            td.meta { (format_duration_ms(Some(run.created_at_ms), run.completed_at_ms)) }
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
    debug_assert!(!repo_id.is_empty());
    debug_assert!(!repo_name.is_empty());
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
        div.ci-actions {
            @if is_active {
                form.inline method="POST" action=(format!("/{repo_id}/ci/{run_id}/cancel")) {
                    button.btn-cancel type="submit" { "✕ Cancel" }
                }
            } @else {
                form.inline method="POST" action=(format!("/{repo_id}/ci/{run_id}/retrigger")) {
                    button.btn type="submit" { "↻ Re-trigger" }
                }
            }
            @if status_resp.commit_hash.is_some() {
                a.btn href=(format!("/{repo_id}/ci/{run_id}/config")) { "View .aspen/ci.ncl" }
            }
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
            @if let (Some(created), Some(completed)) = (status_resp.created_at_ms, status_resp.completed_at_ms) {
                " · Duration: " (format_duration_ms(Some(created), Some(completed)))
            }
        }
        @if let Some(ref err) = status_resp.error {
            div.ci-error-callout { strong { "Error: " } (err) }
        }
        (stage_timeline(&status_resp.stages))
        @for stage in &status_resp.stages {
            (stage_section(&PipelineJobContext { repo_id, run_id }, stage))
        }
        @if status_resp.stages.is_empty() {
            p.muted { "No stages defined for this pipeline." }
        }
    })
}

/// Parameters for rendering the CI Nickel config used by a run.
pub struct CiConfigViewParams<'a> {
    pub repo_id: &'a str,
    pub repo_name: &'a str,
    pub run_id: &'a str,
    pub ref_name: Option<&'a str>,
    pub commit_hash: Option<&'a str>,
    pub path: &'a str,
    pub source: &'a str,
    pub size_bytes: u64,
    pub is_truncated: bool,
}

/// CI Nickel config viewer page.
pub fn ci_config_viewer(params: &CiConfigViewParams<'_>) -> Markup {
    debug_assert!(!params.repo_id.is_empty());
    debug_assert!(!params.run_id.is_empty());
    debug_assert!(!params.path.is_empty());
    let title = format!("{} — CI config", params.repo_name);
    let run_url = format!("/{}/ci/{}", params.repo_id, params.run_id);

    base_layout(&title, html! {
        h1 {
            a href=(format!("/{}", params.repo_id)) { (params.repo_name) }
            " / CI config"
        }
        p.meta {
            "Run: " code { (params.run_id) }
            @if let Some(ref_name) = params.ref_name {
                " · Ref: " code { (ref_name) }
            }
            @if let Some(commit_hash) = params.commit_hash {
                " · Commit: " code { (short_hash(commit_hash)) }
            }
            " · Path: " code { (params.path) }
            " · " (params.size_bytes) " bytes"
        }
        p { a href=(run_url) { "← Back to CI run" } }
        @if params.is_truncated {
            div.ci-error-callout {
                strong { "Truncated: " }
                "showing first " (params.source.len()) " bytes."
            }
        }
        pre.code-block {
            table.code-table {
                @for (line_no, line) in params.source.lines().enumerate() {
                    @let display_line_no = checked_line_number(line_no);
                    tr {
                        td.ln id=(format!("L{}", display_line_no)) { (display_line_no) }
                        td.line { code { (line) } }
                    }
                }
            }
        }
    })
}

/// Horizontal stage progress bar.
fn stage_timeline(stages: &[CiStageInfo]) -> Markup {
    if stages.is_empty() {
        return html! {};
    }
    html! {
        div.stage-timeline {
            @for stage in stages {
                div class=(format!("stage-segment stage-seg-{}", stage_segment_status(&stage.status))) {
                    div.stage-seg-name { (&stage.name) }
                    div.stage-seg-status {
                        (stage_segment_icon(&stage.status))
                        " "
                        (status_label(&stage.status))
                    }
                }
            }
        }
    }
}

/// Map a status string to a CSS modifier for segment coloring.
fn stage_segment_status(status: &str) -> &'static str {
    match status {
        "succeeded" => "succeeded",
        "failed" => "failed",
        "running" => "running",
        "cancelled" => "cancelled",
        _ => "pending",
    }
}

/// Icon for a stage segment status.
fn stage_segment_icon(status: &str) -> &'static str {
    match status {
        "succeeded" => "\u{2713}",
        "failed" => "\u{2717}",
        "running" => "\u{25cf}",
        "cancelled" => "\u{25cb}",
        _ => "\u{25cb}",
    }
}

/// Render a single stage section with its jobs.
struct PipelineJobContext<'a> {
    repo_id: &'a str,
    run_id: &'a str,
}

fn stage_section(context: &PipelineJobContext<'_>, stage: &CiStageInfo) -> Markup {
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
                        (job_row(context, job))
                    }
                }
            }
        }
    }
}

/// Render a single job row.
fn job_row(context: &PipelineJobContext<'_>, job: &CiJobInfo) -> Markup {
    html! {
        tr {
            td {
                span class=(status_badge_class(&job.status)) { (status_label(&job.status)) }
            }
            td {
                (&job.name)
                @if let Some(ref err) = job.error {
                    div.ci-job-error { (err) }
                }
            }
            td.meta { (format_duration_ms(job.started_at_ms, job.ended_at_ms)) }
            td {
                a href=(format!("/{}/ci/{}/{}", context.repo_id, context.run_id, job.id)) { "logs →" }
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
    debug_assert!(!params.repo_id.is_empty());
    debug_assert!(!params.job_id.is_empty());
    let is_active = params.job_status == "running";
    let title = format!("{} — {} logs", params.repo_name, params.job_name);

    // Build numbered log lines from chunks
    let log_line_count_bound = params.logs_resp.chunks.iter().map(|chunk| chunk.content.lines().count()).sum::<usize>();
    let mut log_lines: Vec<String> = Vec::with_capacity(log_line_count_bound);
    for chunk in &params.logs_resp.chunks {
        for line in chunk.content.split('\n') {
            log_lines.push(ansi_to_html_safe(line));
        }
    }
    // Remove trailing empty line from final newline
    if log_lines.last().is_some_and(|l| l.is_empty()) {
        log_lines.pop();
    }

    let full_url = format!("/{}/ci/{}/{}?full=1", params.repo_id, params.run_id, params.job_id);

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
                " · "
                a href=(&full_url) { "View full output" }
            }
        }
        div.log-viewer {
            table.log-table {
                @for (i, line_html) in log_lines.iter().enumerate() {
                    tr {
                        td.log-line-number { (checked_line_number(i)) }
                        td.log-line { code { (PreEscaped(line_html)) } }
                    }
                }
            }
        }
        @if params.logs_resp.has_more {
            div.ci-load-more {
                a href=(format!("/{}/ci/{}/{}?start={}", params.repo_id, params.run_id, params.job_id, next_log_start_index(params.start_index, params.logs_resp.chunks.len()))) {
                    "Load more…"
                }
            }
        }
    })
}

/// CSS class for a branch CI status dot.
fn branch_ci_dot_class(status: &str) -> &'static str {
    match status {
        "succeeded" | "success" => "branch-ci-dot branch-ci-passed",
        "failed" | "failure" => "branch-ci-dot branch-ci-failed",
        "running" => "branch-ci-dot branch-ci-running",
        _ => "branch-ci-dot branch-ci-pending",
    }
}

/// Format bytes into human-readable size.
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB.saturating_mul(KB);
    const GB: u64 = MB.saturating_mul(KB);
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Render an artifacts section for a job page.
fn artifacts_section(artifacts: &[CiArtifactInfo]) -> Markup {
    html! {
        div.artifacts-section {
            h3 { "Artifacts" }
            table.artifacts-table {
                thead {
                    tr {
                        th { "Name" }
                        th { "Size" }
                        th { "Type" }
                        th { "Download" }
                    }
                }
                tbody {
                    @for artifact in artifacts {
                        tr.artifact-row {
                            td { (&artifact.name) }
                            td.meta { (format_bytes(artifact.size_bytes)) }
                            td.meta { (&artifact.content_type) }
                            td {
                                code.artifact-cli { "aspen-cli ci artifact " (&artifact.blob_hash) }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// CI job log viewer with optional artifact section.
pub fn job_log_viewer_with_artifacts(
    params: &JobLogParams<'_>,
    artifacts_resp: Option<&CiListArtifactsResponse>,
) -> Markup {
    let log_markup = job_log_viewer(params);
    // If no artifacts, just return the log viewer
    let has_artifacts = artifacts_resp.is_some_and(|a| a.is_success && !a.artifacts.is_empty());
    if !has_artifacts {
        return log_markup;
    }
    // Inject artifacts section before </main> in the rendered page.
    let mut html = log_markup.into_string();
    if let Some(resp) = artifacts_resp {
        let artifacts_html = artifacts_section(&resp.artifacts).into_string();
        if let Some(pos) = html.rfind("</main>") {
            html.insert_str(pos, &artifacts_html);
        }
    }
    PreEscaped(html)
}

/// Max bytes of full output to display (1 MB).
const MAX_FULL_OUTPUT_BYTES: usize = 1024 * 1024;

/// Parameters for rendering a CI job full output page.
pub struct JobFullOutputParams<'a> {
    pub repo_id: &'a str,
    pub repo_name: &'a str,
    pub run_id: &'a str,
    pub job_id: &'a str,
    pub job_name: &'a str,
    pub job_status: &'a str,
    pub job_started_ms: Option<u64>,
    pub job_ended_ms: Option<u64>,
    pub output_resp: &'a CiGetJobOutputResponse,
}

/// CI job full output viewer page.
pub fn job_full_output_viewer(params: &JobFullOutputParams<'_>) -> Markup {
    debug_assert!(!params.repo_id.is_empty());
    debug_assert!(!params.job_id.is_empty());
    let is_active = params.job_status == "running";
    let title = format!("{} — {} output", params.repo_name, params.job_name);
    let chunked_url = format!("/{}/ci/{}/{}", params.repo_id, params.run_id, params.job_id);

    let stdout = params.output_resp.stdout.as_deref().unwrap_or("");
    let stderr = params.output_resp.stderr.as_deref().unwrap_or("");
    let total_bytes = stdout.len().saturating_add(stderr.len());
    let is_truncated = total_bytes > MAX_FULL_OUTPUT_BYTES;

    // Truncate if needed
    let stdout_display = if stdout.len() > MAX_FULL_OUTPUT_BYTES {
        &stdout[..MAX_FULL_OUTPUT_BYTES]
    } else {
        stdout
    };
    let stderr_budget = MAX_FULL_OUTPUT_BYTES.saturating_sub(stdout_display.len());
    let stderr_display = if stderr.len() > stderr_budget {
        &stderr[..stderr_budget]
    } else {
        stderr
    };

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
                " · "
                a href=(&chunked_url) { "View chunked logs" }
            }
        }
        @if !stdout_display.is_empty() {
            h3 { "stdout" }
            pre.log-viewer.log-stdout {
                (PreEscaped(ansi_to_html_safe(stdout_display)))
            }
        }
        @if !stderr_display.is_empty() {
            h3 { "stderr" }
            pre.log-viewer.log-stderr {
                (PreEscaped(ansi_to_html_safe(stderr_display)))
            }
        }
        @if stdout_display.is_empty() && stderr_display.is_empty() {
            p.muted { "No output captured." }
        }
        @if is_truncated {
            div.ci-load-more {
                "Output truncated at 1 MB. Use "
                code { "aspen-cli ci output " (params.run_id) " " (params.job_id) }
                " for full output."
            }
        }
    })
}

/// Render CI status badges for a commit.
fn commit_status_badges(repo_id: &str, statuses: &[crate::state::CommitStatusEntry]) -> Markup {
    html! {
        div.commit-ci-badges {
            @for st in statuses {
                @let badge_class = match st.state.as_str() {
                    "success" => "badge ci-succeeded",
                    "failure" => "badge ci-failed",
                    "error" => "badge ci-failed",
                    "pending" => "badge ci-running",
                    _ => "badge ci-pending",
                };
                @let label = match st.state.as_str() {
                    "success" => "passed",
                    "failure" => "failed",
                    "error" => "error",
                    "pending" => "pending",
                    _ => "unknown",
                };
                @if let Some(ref run_id) = st.pipeline_run_id {
                    a.ci-badge-link href=(format!("/{repo_id}/ci/{run_id}")) {
                        span class=(badge_class) { (&st.context) ": " (label) }
                    }
                } @else {
                    span class=(badge_class) { (&st.context) ": " (label) }
                }
                " "
            }
        }
    }
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
    debug_assert!(cluster.nodes.iter().all(|node| node.node_id > 0));
    debug_assert!(health.node_id > 0);
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
        backends: vec![],
        backend_routes: vec![],
    }
}

/// Repo tabs variant that includes the CI tab.
fn repo_tabs_with_ci(repo: &ForgeRepoInfo, active: &str) -> Markup {
    render_repo_tabs(repo, active)
}

// ── Helpers ──────────────────────────────────────────────────────────

fn render_repo_tabs(repo: &ForgeRepoInfo, active: &str) -> Markup {
    debug_assert!(!active.is_empty());
    debug_assert!(repo.id.is_empty() || !repo.name.is_empty());
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

fn checked_line_number(line_index: usize) -> usize {
    debug_assert!(line_index < usize::MAX);
    line_index.saturating_add(1)
}

fn next_log_start_index(start_index: u32, chunk_count: usize) -> u32 {
    let chunk_count_u32 = u32::try_from(chunk_count).unwrap_or(u32::MAX);
    start_index.saturating_add(chunk_count_u32)
}

fn commit_message_rest(message: &str) -> String {
    message.lines().skip(1).collect::<Vec<_>>().join("\n").trim().to_string()
}

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

struct TreeLinkContext<'a> {
    repo: &'a ForgeRepoInfo,
    ref_name: &'a str,
    path: &'a str,
}

fn entry_link(context: &TreeLinkContext<'_>, name: &str, mode: u32) -> String {
    let full = if context.path.is_empty() {
        name.to_string()
    } else {
        format!("{}/{}", context.path, name)
    };
    if is_dir(mode) {
        format!("/{}/tree/{}/{}", context.repo.id, context.ref_name, full)
    } else {
        format!("/{}/blob/{}/{}", context.repo.id, context.ref_name, full)
    }
}

fn parent_link(context: &TreeLinkContext<'_>) -> String {
    match context.path.rsplit_once('/') {
        Some((parent, _)) => format!("/{}/tree/{}/{}", context.repo.id, context.ref_name, parent),
        None => format!("/{}/tree/{}", context.repo.id, context.ref_name),
    }
}

fn is_likely_text(data: &[u8]) -> bool {
    let check_len = data.len().min(8192);
    let nul_count = data[..check_len].iter().filter(|&&b| b == 0).count();
    nul_count == 0
}

fn breadcrumb_path(repo: &ForgeRepoInfo, ref_name: &str, path: &str) -> Markup {
    let segments: Vec<&str> = path.split('/').collect();
    let mut cumulative_paths = Vec::with_capacity(segments.len());
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
:root,
[data-theme="phosphor"]{
  --bg:#0f1011;
  --bg-alt:#181a1c;
  --bg-deep:#070708;
  --fg:#e8e8ea;
  --fg-alt:#b4b6ba;
  --muted:#70737a;
  --border:#2a2d31;
  --border-strong:#4a4d52;
  --accent:#6bff9e;
  --accent-dim:#2e5a3c;
  --warn:#ffb000;
  --danger:#ff6b7a;
  --success:#6bff9e;
  --info:#9ad7ff;
  --inverse-bg:#e8e8ea;
  --inverse-fg:#0f1011;
  --code-bg:#070708;
  --code-fg:#e8e8ea;
  --font-display:'DM Serif Display','Times New Roman',Georgia,serif;
  --font-prose:'DM Sans','Inter',-apple-system,BlinkMacSystemFont,'Segoe UI',system-ui,sans-serif;
  --font-mono:'SF Mono','Monaco','Menlo','Inconsolata','Fira Mono',ui-monospace,monospace;
  --fs-xs:0.7rem;
  --fs-sm:0.8rem;
  --fs-base:0.92rem;
  --fs-body:1rem;
  --fs-h3:1.1rem;
  --fs-h2:1.35rem;
  --fs-h1:1.75rem;
  --lh-normal:1.6;
  --lh-code:1.5;
  --space-1:0.25rem;
  --space-2:0.5rem;
  --space-3:0.75rem;
  --space-4:1rem;
  --space-5:1.5rem;
  --space-6:2rem;
  --container-max:68rem;
  --border-w:2px;
  --border-hard:3px;
  --radius:0;
  --shadow-sm:2px 2px 0 0 var(--fg);
  --shadow-md:4px 4px 0 0 var(--fg);
  --focus-ring:0 0 0 2px var(--accent-dim),0 0 0 4px var(--accent);
  --dur-fast:0.12s;
}
*{box-sizing:border-box;margin:0;padding:0}
html{background:var(--bg-deep);color:var(--fg)}
body{font-family:var(--font-prose);background:var(--bg);color:var(--fg);font-size:var(--fs-body);line-height:var(--lh-normal);min-height:100vh;-webkit-font-smoothing:antialiased}
a{color:var(--fg);text-decoration:none;border-bottom:1px solid transparent}
a:hover{color:var(--accent);border-bottom-color:var(--accent);text-decoration:none}
code,kbd,samp{font-family:var(--font-mono)}
code{font-size:0.88em;background:var(--code-bg);color:var(--code-fg);padding:0.1em 0.35em;border:var(--border-w) solid var(--border);border-radius:var(--radius)}
nav{background:var(--bg);border-bottom:var(--border-w) solid var(--fg);display:flex;align-items:center;gap:var(--space-4);padding:var(--space-3) var(--space-5);font-family:var(--font-mono);text-transform:uppercase;letter-spacing:0.12em;position:sticky;top:0;z-index:10}
nav a:first-child{margin-right:auto;color:var(--fg);font-weight:700;font-size:var(--fs-sm);border:var(--border-w) solid var(--fg);padding:var(--space-2) var(--space-3);box-shadow:var(--shadow-sm);background:var(--bg-deep)}
.nav-link,.nav-login{font-weight:600 !important;font-size:var(--fs-xs) !important;color:var(--fg-alt) !important;margin-left:0;border-bottom:none}
.nav-link:hover,.nav-login:hover{color:var(--accent) !important}
main{max-width:var(--container-max);margin:0 auto;padding:var(--space-5)}
h1,h2,h3{color:var(--fg);margin:var(--space-4) 0 var(--space-2);letter-spacing:-0.015em}
h1{font-family:var(--font-display);font-size:var(--fs-h1);line-height:1.1}
h2{font-family:var(--font-display);font-size:var(--fs-h2)}
h3{font-size:var(--fs-h3)}
.muted,.meta{color:var(--muted);font-size:var(--fs-sm)}
.card,.readme,.cluster-stat{background:var(--bg-alt);border:var(--border-w) solid var(--border);border-radius:var(--radius);padding:var(--space-4);margin:var(--space-3) 0;box-shadow:var(--shadow-sm)}
.card:hover{border-color:var(--border-strong);box-shadow:var(--shadow-md)}
.tabs{display:flex;gap:0;border-bottom:var(--border-w) solid var(--fg);margin:var(--space-4) 0;font-family:var(--font-mono);text-transform:uppercase;letter-spacing:0.1em;overflow-x:auto}
.tabs a{padding:var(--space-2) var(--space-4);color:var(--fg-alt);border:var(--border-w) solid transparent;border-bottom:none;font-size:var(--fs-xs)}
.tabs a.active{color:var(--inverse-fg);background:var(--inverse-bg);border-color:var(--fg);font-weight:700}
.tabs a:hover{color:var(--accent);text-decoration:none}
.badge{display:inline-block;padding:0.15em 0.55em;border:var(--border-w) solid var(--border);border-radius:var(--radius);font-family:var(--font-mono);font-size:var(--fs-xs);font-weight:700;text-transform:uppercase;letter-spacing:0.08em}
.badge.open,.badge.added,.badge.ci-succeeded,.badge.cluster-healthy{background:var(--success);color:var(--inverse-fg);border-color:var(--success)}
.badge.closed,.badge.deleted,.badge.ci-failed,.badge.cluster-unreachable{background:var(--danger);color:var(--bg-deep);border-color:var(--danger)}
.badge.modified,.badge.cluster-leader{background:var(--info);color:var(--bg-deep);border-color:var(--info)}
.badge.ci-running,.badge.cluster-learner{background:var(--warn);color:var(--bg-deep);border-color:var(--warn)}
.badge.ci-pending,.badge.ci-cancelled,.badge.cluster-follower{background:var(--bg-alt);color:var(--fg-alt);border-color:var(--border-strong)}
table{width:100%;border-collapse:collapse;background:var(--bg)}
th{font-family:var(--font-mono);text-transform:uppercase;letter-spacing:0.1em;color:var(--fg-alt);font-size:var(--fs-xs);text-align:left}
table td,table th{padding:var(--space-2) var(--space-3);border-bottom:var(--border-w) solid var(--border)}
table tr:hover{background:var(--bg-alt)}
table.file-list td:first-child{width:70%}
table.commits td:first-child{width:7rem}
.hash{color:var(--muted);text-align:right;font-family:var(--font-mono)}
.ln,.log-line-number{color:var(--muted);text-align:right;user-select:none;width:1px;white-space:nowrap;font-family:var(--font-mono)}
.line,.log-line{width:100%}
.line code,.log-line code{background:transparent;border:none;padding:0}
pre.code-block,.log-viewer{background:var(--code-bg);color:var(--code-fg);border:var(--border-w) solid var(--fg);border-radius:var(--radius);padding:var(--space-4);overflow:auto;font-family:var(--font-mono);font-size:var(--fs-sm);line-height:var(--lh-code);box-shadow:var(--shadow-sm)}
table.code-table,table.log-table{border:none;background:transparent}
table.code-table td,table.log-table td{border:none;padding:0 var(--space-2);vertical-align:top}
ul{list-style:none;padding-left:0}
li{padding:var(--space-1) 0}
footer{text-align:center;padding:var(--space-6) var(--space-4);color:var(--muted);font-size:var(--fs-sm);border-top:var(--border-w) solid var(--fg);margin-top:var(--space-6);font-family:var(--font-mono)}
a.raw-link,.btn,button,.btn-secondary,.btn-cancel{display:inline-block;padding:var(--space-2) var(--space-3);border:var(--border-w) solid var(--fg);border-radius:var(--radius);font-family:var(--font-mono);font-size:var(--fs-sm);font-weight:700;cursor:pointer;background:var(--inverse-bg);color:var(--inverse-fg);box-shadow:var(--shadow-sm);text-transform:uppercase;letter-spacing:0.08em}
a.raw-link:hover,.btn:hover,button:hover{transform:translate(1px,1px);box-shadow:none;color:var(--inverse-fg);text-decoration:none;border-color:var(--fg)}
.btn-secondary{background:transparent;color:var(--fg);box-shadow:none}
.btn-secondary:hover{color:var(--warn);border-color:var(--warn)}
.btn-cancel{background:transparent;color:var(--danger);border-color:var(--danger);box-shadow:none}
.btn-cancel:hover{background:var(--danger);color:var(--bg-deep)}
.error,.ci-error-callout{background:#2d1115;border:var(--border-w) solid var(--danger);border-radius:var(--radius);padding:var(--space-3) var(--space-4);margin:var(--space-3) 0;color:#ffa198;box-shadow:var(--shadow-sm)}
form label{display:block;margin:var(--space-3) 0 var(--space-1);color:var(--fg);font-weight:700;font-family:var(--font-mono);font-size:var(--fs-sm);text-transform:uppercase;letter-spacing:0.08em}
form input[type=text],form textarea,#repo-filter,.search-input,.tabs-search input{width:100%;padding:var(--space-2);background:var(--bg-deep);color:var(--fg);border:var(--border-w) solid var(--border);border-radius:var(--radius);font-family:inherit;font-size:var(--fs-base)}
form input[type=text]:focus,form textarea:focus,#repo-filter:focus,.search-input:focus,.tabs-search input:focus{outline:none;border-color:var(--accent);box-shadow:var(--focus-ring)}
.form-actions{margin-top:var(--space-3);text-align:right}
.issue-header,.ci-actions,.commit-ci-badges,.ci-filters,.cluster-summary{display:flex;gap:var(--space-3);flex-wrap:wrap;align-items:center;margin:var(--space-3) 0}
form.inline{display:inline}
.markdown h1,.markdown h2,.markdown h3{color:var(--fg);border-bottom:var(--border-w) solid var(--border);padding-bottom:var(--space-2);margin:1.2em 0 0.5em}
.markdown h1{font-size:1.45rem}.markdown h2{font-size:1.25rem}.markdown h3{font-size:1rem}
.markdown p{margin:0.6em 0}.markdown pre{background:var(--code-bg);border:var(--border-w) solid var(--border);padding:var(--space-3);overflow-x:auto}.markdown code{font-size:0.85em}.markdown pre code{background:transparent;border:none;padding:0}.markdown blockquote{border-left:var(--border-hard) solid var(--accent);padding-left:var(--space-3);color:var(--fg-alt);margin:0.5em 0}.markdown ul,.markdown ol{padding-left:var(--space-5);margin:0.5em 0;list-style:disc}.markdown table{margin:0.5em 0}.markdown th,.markdown td{border:var(--border-w) solid var(--border);padding:var(--space-2) var(--space-3)}.markdown th{background:var(--bg-alt)}.markdown img{max-width:100%}.markdown hr{border:none;border-top:var(--border-w) solid var(--border);margin:1em 0}.markdown input[type=checkbox]{margin-right:var(--space-2)}
.clone-box,.diff-file,.search-file,.ci-stage,.stage-timeline{background:var(--bg-alt);border:var(--border-w) solid var(--border);border-radius:var(--radius);margin:var(--space-3) 0;overflow:hidden;box-shadow:var(--shadow-sm)}
.clone-box{padding:var(--space-2) var(--space-3);font-size:var(--fs-sm);overflow-x:auto;white-space:nowrap}.clone-box code{user-select:all;cursor:text}
.diff-file-header,.search-file-header,.ci-stage-header{background:var(--bg-deep);padding:var(--space-2) var(--space-3);border-bottom:var(--border-w) solid var(--border);font-weight:700;font-size:var(--fs-base);font-family:var(--font-mono)}
table.diff,table.search-matches{font-size:var(--fs-sm);line-height:1.45;background:var(--code-bg)}
table.diff td,table.search-matches td{padding:0 var(--space-2);border:none;white-space:pre;font-family:var(--font-mono)}
table.diff tr.add{background:#0d2818}table.diff tr.add td{color:#aff5b4}table.diff tr.del{background:#2d1115}table.diff tr.del td{color:#ffa198}table.diff tr.hunk td{color:var(--accent);background:var(--bg-deep);padding:var(--space-1) var(--space-2)}
.diff-limit{padding:var(--space-3);color:var(--muted);font-style:italic;text-align:center}
table.search-matches .ln a{color:inherit;text-decoration:none}table.search-matches .ln a:hover{color:var(--accent)}table.search-matches tr.hit{background:#2a1f00}table.search-matches tr.hit td{color:#e3b341}table.search-matches tr.ctx td{color:var(--muted)}
.tabs-search{margin-left:auto}.tabs-search input{font-size:var(--fs-xs);width:10rem;transition:width var(--dur-fast)}.tabs-search input:focus{width:14rem}
.commit-meta,.ci-job-header{margin:var(--space-3) 0}.commit-meta .hash{font-size:var(--fs-base);color:var(--muted)}
.ci-filters a,.ci-filter-active{padding:var(--space-1) var(--space-3);border:var(--border-w) solid var(--border);font-family:var(--font-mono);font-size:var(--fs-xs);text-transform:uppercase;letter-spacing:0.08em}.ci-filter-active{background:var(--inverse-bg);color:var(--inverse-fg);font-weight:700;border-color:var(--fg)}
table.ci-runs,table.ci-jobs,.artifacts-table,table.cluster-nodes{width:100%;border-collapse:collapse}.ci-job-error{color:var(--danger);font-size:var(--fs-sm);margin-top:var(--space-1)}
.log-viewer{padding:0;max-height:80vh}.log-stdout{border-color:var(--border)}.log-stderr{background:#1a0d0d;border-color:var(--danger)}.artifacts-section{margin:var(--space-4) 0}.artifact-cli{font-size:var(--fs-xs);color:var(--muted);user-select:all}.ci-load-more{text-align:center;padding:var(--space-3)}
.stage-timeline{display:flex;gap:0}.stage-segment{flex:1;padding:var(--space-3);text-align:center;min-width:0;border-right:var(--border-w) solid var(--border)}.stage-segment:last-child{border-right:none}.stage-seg-name{font-weight:700;font-size:var(--fs-sm);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}.stage-seg-status{font-family:var(--font-mono);font-size:var(--fs-xs);margin-top:var(--space-1);text-transform:uppercase;letter-spacing:0.08em}.stage-seg-succeeded{background:#0d2818;color:#aff5b4}.stage-seg-failed{background:#2d1115;color:#ffa198}.stage-seg-running{background:#2a1f00;color:#e3b341}.stage-seg-pending,.stage-seg-cancelled{background:var(--bg-alt);color:var(--muted)}
.ci-badge-link{text-decoration:none;border-bottom:none}.ci-badge-link:hover{text-decoration:none;border-bottom:none}.branch-ci-dot{display:inline-block;width:0.55rem;height:0.55rem;border-radius:50%;vertical-align:middle;border:1px solid var(--fg)}.branch-ci-passed{background:var(--success)}.branch-ci-failed{background:var(--danger)}.branch-ci-running{background:var(--warn)}.branch-ci-pending{background:var(--muted)}
.cluster-stat{text-align:center;min-width:7rem}.cluster-stat-value{display:block;font-size:1.6rem;font-weight:700;color:var(--fg);font-family:var(--font-mono)}.cluster-stat-label{display:block;font-size:var(--fs-sm);color:var(--muted);font-family:var(--font-mono);text-transform:uppercase;letter-spacing:0.08em}
@media (max-width: 768px){nav{position:static;align-items:flex-start;flex-wrap:wrap;padding:var(--space-3)}main{padding:var(--space-3)}table{font-size:var(--fs-sm)}.cluster-summary{flex-direction:column;align-items:stretch}.stage-timeline{flex-direction:column}.stage-segment{border-right:none;border-bottom:var(--border-w) solid var(--border)}}
"#;
