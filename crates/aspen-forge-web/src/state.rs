//! Shared application state for the forge web frontend.

use anyhow::Context;
use anyhow::Result;
use aspen_client::AspenClient;
use aspen_client_api::messages::ClientRpcRequest;
use aspen_client_api::messages::ClientRpcResponse;
use aspen_forge_protocol::ForgeBlobResultResponse;
use aspen_forge_protocol::ForgeCommitInfo;
use aspen_forge_protocol::ForgeIssueInfo;
use aspen_forge_protocol::ForgePatchInfo;
use aspen_forge_protocol::ForgeRefInfo;
use aspen_forge_protocol::ForgeRepoInfo;
use aspen_forge_protocol::ForgeTreeEntry;

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
            other => Err(anyhow::anyhow!("unexpected response: {other:?}")),
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
            other => Err(anyhow::anyhow!("unexpected response: {other:?}")),
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
            other => Err(anyhow::anyhow!("unexpected response: {other:?}")),
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
            other => Err(anyhow::anyhow!("unexpected response: {other:?}")),
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
            other => Err(anyhow::anyhow!("unexpected response: {other:?}")),
        }
    }

    /// Get tree entries.
    pub async fn get_tree(&self, hash: &str) -> Result<Vec<ForgeTreeEntry>> {
        let resp = self.client.send(ClientRpcRequest::ForgeGetTree { hash: hash.into() }).await.context("get tree")?;
        match resp {
            ClientRpcResponse::ForgeTreeResult(r) => r.entries.ok_or_else(|| anyhow::anyhow!("tree not found")),
            other => Err(anyhow::anyhow!("unexpected response: {other:?}")),
        }
    }

    /// Get blob content and size.
    pub async fn get_blob(&self, hash: &str) -> Result<ForgeBlobResultResponse> {
        let resp = self.client.send(ClientRpcRequest::ForgeGetBlob { hash: hash.into() }).await.context("get blob")?;
        match resp {
            ClientRpcResponse::ForgeBlobResult(r) => Ok(r),
            other => Err(anyhow::anyhow!("unexpected response: {other:?}")),
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
            other => Err(anyhow::anyhow!("unexpected response: {other:?}")),
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
            other => Err(anyhow::anyhow!("unexpected response: {other:?}")),
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
            other => Err(anyhow::anyhow!("unexpected response: {other:?}")),
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
            other => Err(anyhow::anyhow!("unexpected response: {other:?}")),
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
            other => Err(anyhow::anyhow!("unexpected response: {other:?}")),
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
            other => Err(anyhow::anyhow!("unexpected response: {other:?}")),
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
            other => Err(anyhow::anyhow!("unexpected response: {other:?}")),
        }
    }
}
