//! Issues sub-handler (create, list, get, comment, close, reopen).

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_rpc_core::ClientProtocolContext;

use super::handlers::cob::issues::*;

pub(crate) struct IssuesSubHandler;

impl IssuesSubHandler {
    pub(crate) fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::ForgeCreateIssue { .. }
                | ClientRpcRequest::ForgeListIssues { .. }
                | ClientRpcRequest::ForgeGetIssue { .. }
                | ClientRpcRequest::ForgeCommentIssue { .. }
                | ClientRpcRequest::ForgeCloseIssue { .. }
                | ClientRpcRequest::ForgeReopenIssue { .. }
        )
    }

    pub(crate) async fn handle(
        &self,
        request: ClientRpcRequest,
        _ctx: &ClientProtocolContext,
        forge_node: &super::handlers::ForgeNodeRef,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::ForgeCreateIssue {
                repo_id,
                title,
                body,
                labels,
            } => handle_create_issue(forge_node, repo_id, title, body, labels).await,

            ClientRpcRequest::ForgeListIssues { repo_id, state, limit } => {
                handle_list_issues(forge_node, repo_id, state, limit).await
            }

            ClientRpcRequest::ForgeGetIssue { repo_id, issue_id } => {
                handle_get_issue(forge_node, repo_id, issue_id).await
            }

            ClientRpcRequest::ForgeCommentIssue {
                repo_id,
                issue_id,
                body,
            } => handle_comment_issue(forge_node, repo_id, issue_id, body).await,

            ClientRpcRequest::ForgeCloseIssue {
                repo_id,
                issue_id,
                reason,
            } => handle_close_issue(forge_node, repo_id, issue_id, reason).await,

            ClientRpcRequest::ForgeReopenIssue { repo_id, issue_id } => {
                handle_reopen_issue(forge_node, repo_id, issue_id).await
            }

            _ => Err(anyhow::anyhow!("request not handled by IssuesSubHandler")),
        }
    }
}
