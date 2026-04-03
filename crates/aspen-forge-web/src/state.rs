//! Shared application state for the forge web frontend.

use anyhow::Context;
use anyhow::Result;
use aspen_client::AspenClient;
use aspen_client_api::CapabilityHint;
use aspen_client_api::messages::CiGetJobLogsResponse;
use aspen_client_api::messages::CiGetJobOutputResponse;
use aspen_client_api::messages::CiGetStatusResponse;
use aspen_client_api::messages::CiListRunsResponse;
use aspen_client_api::messages::CiRunInfo;
use aspen_client_api::messages::ClientRpcRequest;
use aspen_client_api::messages::ClientRpcResponse;
use aspen_client_api::messages::ClusterStateResponse;
use aspen_client_api::messages::HealthResponse;
use aspen_forge_protocol::ForgeBlobResultResponse;
use aspen_forge_protocol::ForgeCommitInfo;
use aspen_forge_protocol::ForgeIssueInfo;
use aspen_forge_protocol::ForgeMergeCheckResultResponse;
use aspen_forge_protocol::ForgePatchInfo;
use aspen_forge_protocol::ForgeRefInfo;
use aspen_forge_protocol::ForgeRepoInfo;
use aspen_forge_protocol::ForgeTreeEntry;

/// Error returned when the cluster doesn't have the forge app loaded.
#[derive(Debug)]
pub struct ForgeUnavailableError {
    pub message: String,
    pub hints: Vec<CapabilityHint>,
}

impl std::fmt::Display for ForgeUnavailableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ForgeUnavailableError {}

/// Convert a `ClientRpcResponse` that should have been a specific variant into
/// an error. Returns a `ForgeUnavailableError` for `CapabilityUnavailable`,
/// or a generic anyhow error for anything else unexpected.
fn unexpected_response(resp: ClientRpcResponse) -> anyhow::Error {
    match resp {
        ClientRpcResponse::CapabilityUnavailable(cap) => ForgeUnavailableError {
            message: cap.message,
            hints: cap.hints,
        }
        .into(),
        ClientRpcResponse::Error(e) => {
            anyhow::anyhow!("cluster error [{}]: {}", e.code, e.message)
        }
        other => anyhow::anyhow!("unexpected response from cluster: {:?}", std::mem::discriminant(&other)),
    }
}

/// Kind of file change in a diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffKind {
    Added,
    Modified,
    Deleted,
}

/// A single file that changed between two trees.
#[derive(Debug, Clone)]
pub struct FileDiff {
    pub path: String,
    pub kind: DiffKind,
    pub old_hash: Option<String>,
    pub new_hash: Option<String>,
}

/// A line in a unified diff output.
#[derive(Debug, Clone)]
pub enum DiffLine {
    /// Hunk header or complete hunk (from `similar`'s unified diff formatter).
    Hunk(String),
}

/// A single match within a file from code search.
#[derive(Debug, Clone)]
pub struct SearchMatch {
    /// 1-indexed line number of the match.
    pub line_number: usize,
    /// The matching line content.
    pub line: String,
    /// Context lines before the match.
    pub context_before: Vec<(usize, String)>,
    /// Context lines after the match.
    pub context_after: Vec<(usize, String)>,
}

/// A file that contained search matches.
#[derive(Debug, Clone)]
pub struct SearchFileResult {
    /// Path relative to repo root.
    pub path: String,
    /// Matches within this file.
    pub matches: Vec<SearchMatch>,
}

/// Overall search results.
#[derive(Debug, Clone)]
pub struct SearchResults {
    /// Files with matches.
    pub files: Vec<SearchFileResult>,
    /// Total match count (may exceed files.matches if capped).
    pub total_matches: usize,
    /// Whether the search was truncated.
    pub truncated: bool,
    /// Number of files examined.
    pub files_examined: usize,
}

/// Lightweight representation of a commit CI status for display purposes.
///
/// Deserialized from `forge:status:{repo}:{commit}:{context}` KV entries
/// written by `ForgeStatusReporter`.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CommitStatusEntry {
    /// Context string identifying the check (e.g., "ci/pipeline").
    pub context: String,
    /// Current state: "pending", "success", "failure", "error".
    pub state: String,
    /// Human-readable description.
    pub description: String,
    /// Pipeline run ID that produced this status.
    pub pipeline_run_id: Option<String>,
}

/// Shared application state containing the client connection.
#[derive(Clone)]
pub struct AppState {
    client: AspenClient,
}

impl AppState {
    /// Create new app state with an established client connection.
    pub fn new(client: AspenClient) -> Self {
        Self { client }
    }

    /// Get reference to the underlying client.
    pub fn client(&self) -> &AspenClient {
        &self.client
    }

    /// Get the cluster ticket as a string (for clone URLs).
    pub fn ticket_str(&self) -> String {
        self.client.ticket().serialize()
    }

    /// List repositories.
    pub async fn list_repos(&self) -> Result<Vec<ForgeRepoInfo>> {
        let resp = self
            .client
            .send(ClientRpcRequest::ForgeListRepos {
                limit: Some(100),
                offset: None,
            })
            .await
            .context("list repos")?;
        match resp {
            ClientRpcResponse::ForgeRepoListResult(r) => Ok(r.repos),
            other => Err(unexpected_response(other)),
        }
    }

    /// Get repository info.
    pub async fn get_repo(&self, repo_id: &str) -> Result<ForgeRepoInfo> {
        let resp = self
            .client
            .send(ClientRpcRequest::ForgeGetRepo {
                repo_id: repo_id.into(),
            })
            .await
            .context("get repo")?;
        match resp {
            ClientRpcResponse::ForgeRepoResult(r) => r.repo.ok_or_else(|| anyhow::anyhow!("repo not found")),
            other => Err(unexpected_response(other)),
        }
    }

    /// List branches.
    pub async fn list_branches(&self, repo_id: &str) -> Result<Vec<ForgeRefInfo>> {
        let resp = self
            .client
            .send(ClientRpcRequest::ForgeListBranches {
                repo_id: repo_id.into(),
            })
            .await
            .context("list branches")?;
        match resp {
            ClientRpcResponse::ForgeRefListResult(r) => Ok(r.refs),
            other => Err(unexpected_response(other)),
        }
    }

    /// Get commit log.
    pub async fn get_log(
        &self,
        repo_id: &str,
        ref_name: Option<String>,
        limit: Option<u32>,
    ) -> Result<Vec<ForgeCommitInfo>> {
        let resp = self
            .client
            .send(ClientRpcRequest::ForgeLog {
                repo_id: repo_id.into(),
                ref_name,
                limit,
            })
            .await
            .context("get log")?;
        match resp {
            ClientRpcResponse::ForgeLogResult(r) => Ok(r.commits),
            other => Err(unexpected_response(other)),
        }
    }

    /// Resolve a ref name to its commit hash.
    pub async fn resolve_ref(&self, repo_id: &str, ref_name: &str) -> Result<String> {
        let resp = self
            .client
            .send(ClientRpcRequest::ForgeGetRef {
                repo_id: repo_id.into(),
                ref_name: ref_name.into(),
            })
            .await
            .context("get ref")?;
        match resp {
            ClientRpcResponse::ForgeRefResult(r) => {
                r.ref_info.map(|ri| ri.hash).ok_or_else(|| anyhow::anyhow!("ref not found"))
            }
            other => Err(unexpected_response(other)),
        }
    }

    /// Get tree entries.
    pub async fn get_tree(&self, hash: &str) -> Result<Vec<ForgeTreeEntry>> {
        let resp = self.client.send(ClientRpcRequest::ForgeGetTree { hash: hash.into() }).await.context("get tree")?;
        match resp {
            ClientRpcResponse::ForgeTreeResult(r) => r.entries.ok_or_else(|| anyhow::anyhow!("tree not found")),
            other => Err(unexpected_response(other)),
        }
    }

    /// Get blob content and size.
    pub async fn get_blob(&self, hash: &str) -> Result<ForgeBlobResultResponse> {
        let resp = self.client.send(ClientRpcRequest::ForgeGetBlob { hash: hash.into() }).await.context("get blob")?;
        match resp {
            ClientRpcResponse::ForgeBlobResult(r) => Ok(r),
            other => Err(unexpected_response(other)),
        }
    }

    /// Get commit info.
    pub async fn get_commit(&self, hash: &str) -> Result<ForgeCommitInfo> {
        let resp = self
            .client
            .send(ClientRpcRequest::ForgeGetCommit { hash: hash.into() })
            .await
            .context("get commit")?;
        match resp {
            ClientRpcResponse::ForgeCommitResult(r) => r.commit.ok_or_else(|| anyhow::anyhow!("commit not found")),
            other => Err(unexpected_response(other)),
        }
    }

    /// List issues.
    pub async fn list_issues(&self, repo_id: &str) -> Result<Vec<ForgeIssueInfo>> {
        let resp = self
            .client
            .send(ClientRpcRequest::ForgeListIssues {
                repo_id: repo_id.into(),
                state: None,
                limit: Some(50),
            })
            .await
            .context("list issues")?;
        match resp {
            ClientRpcResponse::ForgeIssueListResult(r) => Ok(r.issues),
            other => Err(unexpected_response(other)),
        }
    }

    /// Get issue detail with comments.
    pub async fn get_issue_with_comments(
        &self,
        repo_id: &str,
        issue_id: &str,
    ) -> Result<(ForgeIssueInfo, Vec<aspen_forge_protocol::ForgeCommentInfo>)> {
        let resp = self
            .client
            .send(ClientRpcRequest::ForgeGetIssue {
                repo_id: repo_id.into(),
                issue_id: issue_id.into(),
            })
            .await
            .context("get issue")?;
        match resp {
            ClientRpcResponse::ForgeIssueResult(r) => {
                let issue = r.issue.ok_or_else(|| anyhow::anyhow!("issue not found"))?;
                let comments = r.comments.unwrap_or_default();
                Ok((issue, comments))
            }
            other => Err(unexpected_response(other)),
        }
    }

    /// List patches.
    pub async fn list_patches(&self, repo_id: &str) -> Result<Vec<ForgePatchInfo>> {
        let resp = self
            .client
            .send(ClientRpcRequest::ForgeListPatches {
                repo_id: repo_id.into(),
                state: None,
                limit: Some(50),
            })
            .await
            .context("list patches")?;
        match resp {
            ClientRpcResponse::ForgePatchListResult(r) => Ok(r.patches),
            other => Err(unexpected_response(other)),
        }
    }

    /// Create an issue.
    pub async fn create_issue(&self, repo_id: &str, title: &str, body: &str, labels: Vec<String>) -> Result<()> {
        let resp = self
            .client
            .send(ClientRpcRequest::ForgeCreateIssue {
                repo_id: repo_id.into(),
                title: title.into(),
                body: body.into(),
                labels,
            })
            .await
            .context("create issue")?;
        match resp {
            ClientRpcResponse::ForgeOperationResult(r) if r.is_success => Ok(()),
            ClientRpcResponse::ForgeOperationResult(r) => {
                Err(anyhow::anyhow!(r.error.unwrap_or_else(|| "unknown error".into())))
            }
            other => Err(unexpected_response(other)),
        }
    }

    /// Close an issue.
    pub async fn close_issue(&self, repo_id: &str, issue_id: &str) -> Result<()> {
        let resp = self
            .client
            .send(ClientRpcRequest::ForgeCloseIssue {
                repo_id: repo_id.into(),
                issue_id: issue_id.into(),
                reason: None,
            })
            .await
            .context("close issue")?;
        match resp {
            ClientRpcResponse::ForgeOperationResult(r) if r.is_success => Ok(()),
            ClientRpcResponse::ForgeOperationResult(r) => {
                Err(anyhow::anyhow!(r.error.unwrap_or_else(|| "unknown error".into())))
            }
            other => Err(unexpected_response(other)),
        }
    }

    /// Reopen a closed issue.
    pub async fn reopen_issue(&self, repo_id: &str, issue_id: &str) -> Result<()> {
        let resp = self
            .client
            .send(ClientRpcRequest::ForgeReopenIssue {
                repo_id: repo_id.into(),
                issue_id: issue_id.into(),
            })
            .await
            .context("reopen issue")?;
        match resp {
            ClientRpcResponse::ForgeOperationResult(r) if r.is_success => Ok(()),
            ClientRpcResponse::ForgeOperationResult(r) => {
                Err(anyhow::anyhow!(r.error.unwrap_or_else(|| "unknown error".into())))
            }
            other => Err(unexpected_response(other)),
        }
    }

    /// Add a comment to an issue.
    pub async fn comment_issue(&self, repo_id: &str, issue_id: &str, body: &str) -> Result<()> {
        let resp = self
            .client
            .send(ClientRpcRequest::ForgeCommentIssue {
                repo_id: repo_id.into(),
                issue_id: issue_id.into(),
                body: body.into(),
            })
            .await
            .context("comment issue")?;
        match resp {
            ClientRpcResponse::ForgeOperationResult(r) if r.is_success => Ok(()),
            ClientRpcResponse::ForgeOperationResult(r) => {
                Err(anyhow::anyhow!(r.error.unwrap_or_else(|| "unknown error".into())))
            }
            other => Err(unexpected_response(other)),
        }
    }

    /// Fetch README content from a repo's default branch (if present).
    ///
    /// Resolves default branch → commit → root tree, scans for a
    /// README file (case-insensitive), and returns the blob content.
    pub async fn get_readme(&self, repo: &ForgeRepoInfo) -> Option<Vec<u8>> {
        let commit_hash = self.resolve_ref(&repo.id, &repo.default_branch).await.ok()?;
        let commit = self.get_commit(&commit_hash).await.ok()?;
        let entries = self.get_tree(&commit.tree).await.ok()?;

        // Look for README variants (case-insensitive).
        let readme_entry = entries.iter().find(|e| {
            let lower = e.name.to_ascii_lowercase();
            lower == "readme.md" || lower == "readme" || lower == "readme.txt" || lower == "readme.rst"
        })?;

        let blob = self.get_blob(&readme_entry.hash).await.ok()?;
        blob.content
    }

    // ── Diff computation ──────────────────────────────────────────

    /// Max files to diff per commit.
    pub const MAX_DIFF_FILES: usize = 50;
    /// Max blob size for inline diff (256KB).
    const MAX_DIFF_BLOB_BYTES: u64 = 256 * 1024;

    /// Compare two trees and return a list of changed files.
    ///
    /// Recursively walks both trees, comparing entries by name.
    /// Skips subtrees with matching hashes (no changes underneath).
    pub async fn diff_trees(&self, old_tree: Option<&str>, new_tree: &str) -> Result<Vec<FileDiff>> {
        let mut result = Vec::new();
        self.diff_trees_recursive(old_tree, new_tree, "", &mut result).await?;
        Ok(result)
    }

    #[async_recursion::async_recursion]
    async fn diff_trees_recursive(
        &self,
        old_tree: Option<&str>,
        new_tree: &str,
        prefix: &str,
        result: &mut Vec<FileDiff>,
    ) -> Result<()> {
        if result.len() >= Self::MAX_DIFF_FILES {
            return Ok(());
        }

        let new_entries = self.get_tree(new_tree).await?;
        let old_entries = match old_tree {
            Some(h) => self.get_tree(h).await.unwrap_or_default(),
            None => vec![],
        };

        let old_map: std::collections::HashMap<&str, &ForgeTreeEntry> =
            old_entries.iter().map(|e| (e.name.as_str(), e)).collect();
        let new_map: std::collections::HashMap<&str, &ForgeTreeEntry> =
            new_entries.iter().map(|e| (e.name.as_str(), e)).collect();

        // Added or modified entries
        for entry in &new_entries {
            if result.len() >= Self::MAX_DIFF_FILES {
                break;
            }
            let path = if prefix.is_empty() {
                entry.name.clone()
            } else {
                format!("{prefix}/{}", entry.name)
            };
            let is_dir = entry.mode == 0o40000;

            match old_map.get(entry.name.as_str()) {
                Some(old) if old.hash == entry.hash => {} // unchanged
                Some(old) if is_dir && old.mode == 0o40000 => {
                    // Both are directories with different hashes — recurse
                    self.diff_trees_recursive(Some(&old.hash), &entry.hash, &path, result).await?;
                }
                Some(old) if !is_dir => {
                    result.push(FileDiff {
                        path,
                        kind: DiffKind::Modified,
                        old_hash: Some(old.hash.clone()),
                        new_hash: Some(entry.hash.clone()),
                    });
                }
                Some(_) => {} // mode change (dir→file etc), treat as modified
                None if is_dir => {
                    self.diff_trees_recursive(None, &entry.hash, &path, result).await?;
                }
                None => {
                    result.push(FileDiff {
                        path,
                        kind: DiffKind::Added,
                        old_hash: None,
                        new_hash: Some(entry.hash.clone()),
                    });
                }
            }
        }

        // Deleted entries (in old but not new)
        for entry in &old_entries {
            if result.len() >= Self::MAX_DIFF_FILES {
                break;
            }
            if new_map.contains_key(entry.name.as_str()) {
                continue;
            }
            let path = if prefix.is_empty() {
                entry.name.clone()
            } else {
                format!("{prefix}/{}", entry.name)
            };
            if entry.mode == 0o40000 {
                // Deleted directory — list all files as deleted
                self.collect_deleted_tree(&entry.hash, &path, result).await?;
            } else {
                result.push(FileDiff {
                    path,
                    kind: DiffKind::Deleted,
                    old_hash: Some(entry.hash.clone()),
                    new_hash: None,
                });
            }
        }
        Ok(())
    }

    /// Collect all files in a tree as deleted entries.
    #[async_recursion::async_recursion]
    async fn collect_deleted_tree(&self, tree_hash: &str, prefix: &str, result: &mut Vec<FileDiff>) -> Result<()> {
        if result.len() >= Self::MAX_DIFF_FILES {
            return Ok(());
        }
        let entries = self.get_tree(tree_hash).await.unwrap_or_default();
        for entry in &entries {
            if result.len() >= Self::MAX_DIFF_FILES {
                break;
            }
            let path = format!("{prefix}/{}", entry.name);
            if entry.mode == 0o40000 {
                self.collect_deleted_tree(&entry.hash, &path, result).await?;
            } else {
                result.push(FileDiff {
                    path,
                    kind: DiffKind::Deleted,
                    old_hash: Some(entry.hash.clone()),
                    new_hash: None,
                });
            }
        }
        Ok(())
    }

    /// Compute a unified diff between two blobs (by hash).
    pub async fn compute_file_diff(&self, file: &FileDiff) -> Vec<DiffLine> {
        let old_text = match &file.old_hash {
            Some(h) => self.get_text_blob(h).await,
            None => Some(String::new()),
        };
        let new_text = match &file.new_hash {
            Some(h) => self.get_text_blob(h).await,
            None => Some(String::new()),
        };

        let (Some(old), Some(new)) = (old_text, new_text) else {
            return vec![DiffLine::Hunk("Binary or large file — diff not shown".into())];
        };

        let diff = similar::TextDiff::from_lines(&old, &new);
        let mut lines = Vec::new();
        for hunk in diff.unified_diff().context_radius(3).iter_hunks() {
            lines.push(DiffLine::Hunk(format!("{hunk}")));
        }
        if lines.is_empty() && file.kind == DiffKind::Modified {
            lines.push(DiffLine::Hunk("No visible changes".into()));
        }
        lines
    }

    /// Fetch a blob as text if it's small enough and looks like text.
    async fn get_text_blob(&self, hash: &str) -> Option<String> {
        let blob = self.get_blob(hash).await.ok()?;
        let size = blob.size.unwrap_or(0);
        if size > Self::MAX_DIFF_BLOB_BYTES {
            return None;
        }
        let content = blob.content?;
        String::from_utf8(content).ok()
    }

    // ── Code search ──────────────────────────────────────────────

    /// Max files to walk during a search.
    const MAX_SEARCH_FILES: usize = 500;
    /// Max blob size for searching (256KB).
    const MAX_SEARCH_BLOB_BYTES: u64 = 256 * 1024;
    /// Max result matches to return.
    const MAX_SEARCH_MATCHES: usize = 50;
    /// Context lines above/below a match.
    const SEARCH_CONTEXT: usize = 2;

    /// Search file contents at HEAD for a substring.
    pub async fn search_code(&self, repo_id: &str, query: &str) -> Result<SearchResults> {
        let query_lower = query.to_lowercase();
        let commit_hash = self.resolve_ref(repo_id, "main").await?;
        let commit = self.get_commit(&commit_hash).await?;

        let mut files_to_search = Vec::new();
        self.collect_search_files(&commit.tree, "", &mut files_to_search).await?;

        let mut results = SearchResults {
            files: Vec::new(),
            total_matches: 0,
            truncated: files_to_search.len() >= Self::MAX_SEARCH_FILES,
            files_examined: files_to_search.len(),
        };

        for (path, hash) in &files_to_search {
            if results.total_matches >= Self::MAX_SEARCH_MATCHES {
                results.truncated = true;
                break;
            }
            let remaining = Self::MAX_SEARCH_MATCHES - results.total_matches;
            if let Some(file_result) = self.search_blob(path, hash, &query_lower, remaining).await {
                results.total_matches += file_result.matches.len();
                results.files.push(file_result);
            }
        }

        Ok(results)
    }

    /// Recursively collect (path, blob_hash) for all files in a tree.
    #[async_recursion::async_recursion]
    async fn collect_search_files(&self, tree_hash: &str, prefix: &str, out: &mut Vec<(String, String)>) -> Result<()> {
        if out.len() >= Self::MAX_SEARCH_FILES {
            return Ok(());
        }
        let entries = self.get_tree(tree_hash).await?;
        for entry in &entries {
            if out.len() >= Self::MAX_SEARCH_FILES {
                break;
            }
            let path = if prefix.is_empty() {
                entry.name.clone()
            } else {
                format!("{prefix}/{}", entry.name)
            };
            if entry.mode == 0o40000 {
                self.collect_search_files(&entry.hash, &path, out).await?;
            } else {
                out.push((path, entry.hash.clone()));
            }
        }
        Ok(())
    }

    /// Search a single blob for matches, returning up to `max_matches`.
    async fn search_blob(
        &self,
        path: &str,
        hash: &str,
        query_lower: &str,
        max_matches: usize,
    ) -> Option<SearchFileResult> {
        let blob = self.get_blob(hash).await.ok()?;
        if blob.size.unwrap_or(0) > Self::MAX_SEARCH_BLOB_BYTES {
            return None;
        }
        let content = blob.content?;
        let text = String::from_utf8(content).ok()?;
        let lines: Vec<&str> = text.lines().collect();

        let mut matches = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            if matches.len() >= max_matches {
                break;
            }
            if line.to_lowercase().contains(query_lower) {
                let ctx_before: Vec<(usize, String)> = lines[i.saturating_sub(Self::SEARCH_CONTEXT)..i]
                    .iter()
                    .enumerate()
                    .map(|(j, l)| (i.saturating_sub(Self::SEARCH_CONTEXT) + j + 1, l.to_string()))
                    .collect();
                let ctx_after: Vec<(usize, String)> = lines[i + 1..lines.len().min(i + 1 + Self::SEARCH_CONTEXT)]
                    .iter()
                    .enumerate()
                    .map(|(j, l)| (i + 2 + j, l.to_string()))
                    .collect();
                matches.push(SearchMatch {
                    line_number: i + 1,
                    line: line.to_string(),
                    context_before: ctx_before,
                    context_after: ctx_after,
                });
            }
        }

        if matches.is_empty() {
            None
        } else {
            Some(SearchFileResult {
                path: path.to_string(),
                matches,
            })
        }
    }

    // ── Profile resolution ─────────────────────────────────────────

    /// Resolve an npub to a display name via the Nostr relay's profile metadata.
    pub async fn resolve_profile(&self, npub_hex: &str) -> Option<String> {
        let resp = self
            .client
            .send(ClientRpcRequest::NostrGetProfile {
                npub_hex: npub_hex.to_string(),
            })
            .await
            .ok()?;
        match resp {
            ClientRpcResponse::NostrGetProfileResult { display_name, .. } => display_name,
            _ => None,
        }
    }

    // ── Nostr auth ────────────────────────────────────────────────

    /// Request a challenge for npub authentication.
    pub async fn nostr_auth_challenge(&self, npub_hex: &str) -> Result<(String, String)> {
        let resp = self
            .client
            .send(ClientRpcRequest::NostrAuthChallenge {
                npub_hex: npub_hex.to_string(),
            })
            .await
            .context("auth challenge")?;
        match resp {
            ClientRpcResponse::NostrAuthChallengeResult {
                challenge_id,
                challenge_hex,
            } => Ok((challenge_id, challenge_hex)),
            other => Err(unexpected_response(other)),
        }
    }

    /// Verify a signed challenge and get a session token.
    pub async fn nostr_auth_verify(&self, npub_hex: &str, challenge_id: &str, signature_hex: &str) -> Result<String> {
        let resp = self
            .client
            .send(ClientRpcRequest::NostrAuthVerify {
                npub_hex: npub_hex.to_string(),
                challenge_id: challenge_id.to_string(),
                signature_hex: signature_hex.to_string(),
            })
            .await
            .context("auth verify")?;
        match resp {
            ClientRpcResponse::NostrAuthVerifyResult {
                is_success: true,
                token: Some(token),
                ..
            } => Ok(token),
            ClientRpcResponse::NostrAuthVerifyResult { error: Some(e), .. } => Err(anyhow::anyhow!("auth failed: {e}")),
            other => Err(unexpected_response(other)),
        }
    }

    /// Get patch detail.
    pub async fn get_patch(&self, repo_id: &str, patch_id: &str) -> Result<ForgePatchInfo> {
        let resp = self
            .client
            .send(ClientRpcRequest::ForgeGetPatch {
                repo_id: repo_id.into(),
                patch_id: patch_id.into(),
            })
            .await
            .context("get patch")?;
        match resp {
            ClientRpcResponse::ForgePatchResult(r) => r.patch.ok_or_else(|| anyhow::anyhow!("patch not found")),
            other => Err(unexpected_response(other)),
        }
    }

    /// Check if a patch is mergeable (dry-run, no side effects).
    pub async fn check_merge(&self, repo_id: &str, patch_id: &str) -> Result<ForgeMergeCheckResultResponse> {
        let resp = self
            .client
            .send(ClientRpcRequest::ForgeCheckMerge {
                repo_id: repo_id.into(),
                patch_id: patch_id.into(),
            })
            .await
            .context("check merge")?;
        match resp {
            ClientRpcResponse::ForgeMergeCheckResult(r) => Ok(r),
            other => Err(unexpected_response(other)),
        }
    }

    /// Merge a patch with the given strategy.
    pub async fn merge_patch(
        &self,
        repo_id: &str,
        patch_id: &str,
        strategy: Option<&str>,
    ) -> Result<ForgeMergeCheckResultResponse> {
        let resp = self
            .client
            .send(ClientRpcRequest::ForgeMergePatch {
                repo_id: repo_id.into(),
                patch_id: patch_id.into(),
                strategy: strategy.map(|s| s.to_string()),
                message: None,
            })
            .await
            .context("merge patch")?;
        match resp {
            ClientRpcResponse::ForgeMergeCheckResult(r) => Ok(r),
            other => Err(unexpected_response(other)),
        }
    }

    // ── CI operations ─────────────────────────────────────────────

    /// List CI pipeline runs.
    pub async fn list_runs(
        &self,
        repo_id: Option<&str>,
        status: Option<&str>,
        limit: Option<u32>,
    ) -> Result<CiListRunsResponse> {
        let resp = self
            .client
            .send(ClientRpcRequest::CiListRuns {
                repo_id: repo_id.map(|s| s.to_string()),
                status: status.map(|s| s.to_string()),
                limit,
            })
            .await
            .context("ci list runs")?;
        match resp {
            ClientRpcResponse::CiListRunsResult(r) => Ok(r),
            other => Err(unexpected_response(other)),
        }
    }

    /// Get CI pipeline run status with stages and jobs.
    pub async fn get_run_status(&self, run_id: &str) -> Result<CiGetStatusResponse> {
        let resp = self
            .client
            .send(ClientRpcRequest::CiGetStatus {
                run_id: run_id.to_string(),
            })
            .await
            .context("ci get status")?;
        match resp {
            ClientRpcResponse::CiGetStatusResult(r) => Ok(r),
            other => Err(unexpected_response(other)),
        }
    }

    /// Get CI job logs in chunks.
    pub async fn get_job_logs(
        &self,
        run_id: &str,
        job_id: &str,
        start_index: u32,
        limit: Option<u32>,
    ) -> Result<CiGetJobLogsResponse> {
        let resp = self
            .client
            .send(ClientRpcRequest::CiGetJobLogs {
                run_id: run_id.to_string(),
                job_id: job_id.to_string(),
                start_index,
                limit,
            })
            .await
            .context("ci get job logs")?;
        match resp {
            ClientRpcResponse::CiGetJobLogsResult(r) => Ok(r),
            other => Err(unexpected_response(other)),
        }
    }

    /// Get CI job full output.
    pub async fn get_job_output(&self, run_id: &str, job_id: &str) -> Result<CiGetJobOutputResponse> {
        let resp = self
            .client
            .send(ClientRpcRequest::CiGetJobOutput {
                run_id: run_id.to_string(),
                job_id: job_id.to_string(),
            })
            .await
            .context("ci get job output")?;
        match resp {
            ClientRpcResponse::CiGetJobOutputResult(r) => Ok(r),
            other => Err(unexpected_response(other)),
        }
    }

    /// Get latest CI status for a repo (most recent run, if any).
    pub async fn get_latest_ci_status(&self, repo_id: &str) -> Option<CiRunInfo> {
        let resp = self.list_runs(Some(repo_id), None, Some(1)).await.ok()?;
        resp.runs.into_iter().next()
    }

    /// Get commit statuses for a given repo and commit hash.
    ///
    /// Scans the `forge:status:{repo_hex}:{commit_hex}:` prefix.
    /// Returns empty on any error (commit status is supplementary info).
    pub async fn get_commit_statuses(&self, repo_id: &str, commit_hash: &str) -> Vec<CommitStatusEntry> {
        let prefix = format!("forge:status:{repo_id}:{commit_hash}:");
        let resp = self
            .client
            .send(ClientRpcRequest::ScanKeys {
                prefix,
                limit: Some(20),
                continuation_token: None,
            })
            .await;
        let resp = match resp {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };
        match resp {
            ClientRpcResponse::ScanResult(scan) => scan
                .entries
                .iter()
                .filter_map(|entry| serde_json::from_str::<CommitStatusEntry>(&entry.value).ok())
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Get latest CI status for a specific ref (branch/tag).
    ///
    /// Returns None if no CI status exists or on error (best-effort).
    pub async fn get_ref_status(&self, repo_id: &str, ref_name: &str) -> Option<CiGetStatusResponse> {
        let resp = self
            .client
            .send(ClientRpcRequest::CiGetRefStatus {
                repo_id: repo_id.to_string(),
                ref_name: ref_name.to_string(),
            })
            .await
            .ok()?;
        match resp {
            ClientRpcResponse::CiGetRefStatusResult(r) if r.was_found => Some(r),
            _ => None,
        }
    }

    /// Cancel a running CI pipeline.
    pub async fn cancel_run(&self, run_id: &str) -> Result<aspen_client_api::messages::CiCancelRunResponse> {
        let resp = self
            .client
            .send(ClientRpcRequest::CiCancelRun {
                run_id: run_id.to_string(),
                reason: None,
            })
            .await
            .context("ci cancel run")?;
        match resp {
            ClientRpcResponse::CiCancelRunResult(r) => Ok(r),
            other => Err(unexpected_response(other)),
        }
    }

    /// Re-trigger a CI pipeline for the same repo and ref.
    pub async fn retrigger_run(
        &self,
        repo_id: &str,
        ref_name: &str,
    ) -> Result<aspen_client_api::messages::CiTriggerPipelineResponse> {
        let resp = self
            .client
            .send(ClientRpcRequest::CiTriggerPipeline {
                repo_id: repo_id.to_string(),
                ref_name: ref_name.to_string(),
                commit_hash: None,
            })
            .await
            .context("ci retrigger")?;
        match resp {
            ClientRpcResponse::CiTriggerPipelineResult(r) => Ok(r),
            other => Err(unexpected_response(other)),
        }
    }

    /// List artifacts for a CI job.
    pub async fn list_artifacts(
        &self,
        job_id: &str,
        run_id: Option<&str>,
    ) -> Result<aspen_client_api::messages::CiListArtifactsResponse> {
        let resp = self
            .client
            .send(ClientRpcRequest::CiListArtifacts {
                job_id: job_id.to_string(),
                run_id: run_id.map(|s| s.to_string()),
            })
            .await
            .context("ci list artifacts")?;
        match resp {
            ClientRpcResponse::CiListArtifactsResult(r) => Ok(r),
            other => Err(unexpected_response(other)),
        }
    }

    // ── Cluster operations ────────────────────────────────────────

    /// Get node health status.
    pub async fn get_health(&self) -> Result<HealthResponse> {
        let resp = self.client.send(ClientRpcRequest::GetHealth).await.context("get health")?;
        match resp {
            ClientRpcResponse::Health(r) => Ok(r),
            other => Err(unexpected_response(other)),
        }
    }

    /// Get cluster state (all nodes, leader, roles).
    pub async fn get_cluster_state(&self) -> Result<ClusterStateResponse> {
        let resp = self.client.send(ClientRpcRequest::GetClusterState).await.context("get cluster state")?;
        match resp {
            ClientRpcResponse::ClusterState(r) => Ok(r),
            other => Err(unexpected_response(other)),
        }
    }

    /// Approve a patch at its current head.
    pub async fn approve_patch(&self, repo_id: &str, patch_id: &str) -> Result<()> {
        // Get current head for the approval
        let patch = self.get_patch(repo_id, patch_id).await?;
        let resp = self
            .client
            .send(ClientRpcRequest::ForgeApprovePatch {
                repo_id: repo_id.into(),
                patch_id: patch_id.into(),
                commit: patch.head,
                message: None,
            })
            .await
            .context("approve patch")?;
        match resp {
            ClientRpcResponse::ForgePatchResult(r) if r.is_success => Ok(()),
            ClientRpcResponse::ForgeOperationResult(r) if r.is_success => Ok(()),
            ClientRpcResponse::ForgePatchResult(r) => {
                Err(anyhow::anyhow!(r.error.unwrap_or_else(|| "approve failed".into())))
            }
            ClientRpcResponse::ForgeOperationResult(r) => {
                Err(anyhow::anyhow!(r.error.unwrap_or_else(|| "approve failed".into())))
            }
            other => Err(unexpected_response(other)),
        }
    }
}
