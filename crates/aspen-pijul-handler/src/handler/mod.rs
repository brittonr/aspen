//! Pijul (patch-based VCS) request handler.
//!
//! Handles all Pijul* operations for patch-based version control.
//!
//! ## Operations (15 total)
//!
//! ### Repository Operations (3)
//! - PijulRepoInit: Create a new Pijul repository
//! - PijulRepoList: List all repositories
//! - PijulRepoInfo: Get repository details
//!
//! ### Channel Operations (5)
//! - PijulChannelList: List channels in a repository
//! - PijulChannelCreate: Create a new channel
//! - PijulChannelDelete: Delete a channel
//! - PijulChannelFork: Fork a channel
//! - PijulChannelInfo: Get channel details
//!
//! ### Change Operations (7)
//! - PijulRecord: Record changes (requires local filesystem - returns NOT_IMPLEMENTED)
//! - PijulApply: Apply a change to a channel
//! - PijulUnrecord: Remove a change from a channel
//! - PijulLog: Get change log for a channel
//! - PijulCheckout: Checkout to working directory (requires local filesystem - returns
//!   NOT_IMPLEMENTED)
//! - PijulShow: Show change details
//! - PijulBlame: Show file attribution

mod change;
mod channel;
mod helpers;
mod repo;

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::RequestHandler;

/// Type alias for the PijulStore with concrete types.
pub(crate) type PijulStoreRef =
    std::sync::Arc<aspen_pijul::PijulStore<aspen_blob::IrohBlobStore, dyn aspen_core::KeyValueStore>>;

/// Handler for Pijul operations.
pub struct PijulHandler;

#[async_trait::async_trait]
impl RequestHandler for PijulHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::PijulRepoInit { .. }
                | ClientRpcRequest::PijulRepoList { .. }
                | ClientRpcRequest::PijulRepoInfo { .. }
                | ClientRpcRequest::PijulChannelList { .. }
                | ClientRpcRequest::PijulChannelCreate { .. }
                | ClientRpcRequest::PijulChannelDelete { .. }
                | ClientRpcRequest::PijulChannelFork { .. }
                | ClientRpcRequest::PijulChannelInfo { .. }
                | ClientRpcRequest::PijulRecord { .. }
                | ClientRpcRequest::PijulApply { .. }
                | ClientRpcRequest::PijulUnrecord { .. }
                | ClientRpcRequest::PijulLog { .. }
                | ClientRpcRequest::PijulCheckout { .. }
                | ClientRpcRequest::PijulShow { .. }
                | ClientRpcRequest::PijulBlame { .. }
        )
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        // Check if Pijul feature is available
        let pijul_store = match &ctx.pijul_store {
            Some(store) => store,
            None => {
                return Ok(ClientRpcResponse::error("PIJUL_UNAVAILABLE", "Pijul feature not configured on this node"));
            }
        };

        match request {
            // Repository Operations
            ClientRpcRequest::PijulRepoInit {
                name,
                description,
                default_channel,
            } => repo::handle_repo_init(pijul_store, name, description, default_channel).await,

            ClientRpcRequest::PijulRepoList { limit } => repo::handle_repo_list(pijul_store, limit).await,

            ClientRpcRequest::PijulRepoInfo { repo_id } => repo::handle_repo_info(pijul_store, repo_id).await,

            // Channel Operations
            ClientRpcRequest::PijulChannelList { repo_id } => channel::handle_channel_list(pijul_store, repo_id).await,

            ClientRpcRequest::PijulChannelCreate { repo_id, name } => {
                channel::handle_channel_create(pijul_store, repo_id, name).await
            }

            ClientRpcRequest::PijulChannelDelete { repo_id, name } => {
                channel::handle_channel_delete(pijul_store, repo_id, name).await
            }

            ClientRpcRequest::PijulChannelFork {
                repo_id,
                source,
                target,
            } => channel::handle_channel_fork(pijul_store, repo_id, source, target).await,

            ClientRpcRequest::PijulChannelInfo { repo_id, name } => {
                channel::handle_channel_info(pijul_store, repo_id, name).await
            }

            // Change Operations
            ClientRpcRequest::PijulRecord { .. } => {
                // Recording requires local filesystem access - complex to implement remotely
                Ok(ClientRpcResponse::error(
                    "NOT_IMPLEMENTED",
                    "Recording changes requires local filesystem access. Use local pijul tools.",
                ))
            }

            ClientRpcRequest::PijulApply {
                repo_id,
                channel,
                change_hash,
            } => change::handle_apply(pijul_store, repo_id, channel, change_hash).await,

            ClientRpcRequest::PijulUnrecord {
                repo_id,
                channel,
                change_hash,
            } => change::handle_unrecord(pijul_store, repo_id, channel, change_hash).await,

            ClientRpcRequest::PijulLog {
                repo_id,
                channel,
                limit,
            } => change::handle_log(pijul_store, repo_id, channel, limit).await,

            ClientRpcRequest::PijulCheckout { .. } => {
                // Checkout requires local filesystem access - complex to implement remotely
                Ok(ClientRpcResponse::error(
                    "NOT_IMPLEMENTED",
                    "Checkout requires local filesystem access. Use local pijul tools.",
                ))
            }

            ClientRpcRequest::PijulShow { repo_id, change_hash } => {
                change::handle_show(pijul_store, repo_id, change_hash).await
            }

            ClientRpcRequest::PijulBlame { repo_id, channel, path } => {
                change::handle_blame(pijul_store, repo_id, channel, path).await
            }

            _ => Err(anyhow::anyhow!("request not handled by PijulHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "PijulHandler"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_handle_repo_init() {
        let handler = PijulHandler;
        assert!(handler.can_handle(&ClientRpcRequest::PijulRepoInit {
            name: "my-repo".to_string(),
            description: Some("A test repo".to_string()),
            default_channel: "main".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_repo_list() {
        let handler = PijulHandler;
        assert!(handler.can_handle(&ClientRpcRequest::PijulRepoList { limit: 100 }));
    }

    #[test]
    fn test_can_handle_repo_info() {
        let handler = PijulHandler;
        assert!(handler.can_handle(&ClientRpcRequest::PijulRepoInfo {
            repo_id: "abc123".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_channel_list() {
        let handler = PijulHandler;
        assert!(handler.can_handle(&ClientRpcRequest::PijulChannelList {
            repo_id: "abc123".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_channel_create() {
        let handler = PijulHandler;
        assert!(handler.can_handle(&ClientRpcRequest::PijulChannelCreate {
            repo_id: "abc123".to_string(),
            name: "feature-branch".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_apply() {
        let handler = PijulHandler;
        assert!(handler.can_handle(&ClientRpcRequest::PijulApply {
            repo_id: "abc123".to_string(),
            channel: "main".to_string(),
            change_hash: "deadbeef".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_log() {
        let handler = PijulHandler;
        assert!(handler.can_handle(&ClientRpcRequest::PijulLog {
            repo_id: "abc123".to_string(),
            channel: "main".to_string(),
            limit: 50,
        }));
    }

    #[test]
    fn test_can_handle_show() {
        let handler = PijulHandler;
        assert!(handler.can_handle(&ClientRpcRequest::PijulShow {
            repo_id: "abc123".to_string(),
            change_hash: "deadbeef".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_channel_delete() {
        let handler = PijulHandler;
        assert!(handler.can_handle(&ClientRpcRequest::PijulChannelDelete {
            repo_id: "abc123".to_string(),
            name: "old-branch".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_channel_fork() {
        let handler = PijulHandler;
        assert!(handler.can_handle(&ClientRpcRequest::PijulChannelFork {
            repo_id: "abc123".to_string(),
            source: "main".to_string(),
            target: "feature".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_unrecord() {
        let handler = PijulHandler;
        assert!(handler.can_handle(&ClientRpcRequest::PijulUnrecord {
            repo_id: "abc123".to_string(),
            channel: "main".to_string(),
            change_hash: "deadbeef".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_blame() {
        let handler = PijulHandler;
        assert!(handler.can_handle(&ClientRpcRequest::PijulBlame {
            repo_id: "abc123".to_string(),
            channel: "main".to_string(),
            path: "src/main.rs".to_string(),
        }));
    }

    #[test]
    fn test_rejects_unrelated_requests() {
        let handler = PijulHandler;

        assert!(!handler.can_handle(&ClientRpcRequest::Ping));
        assert!(!handler.can_handle(&ClientRpcRequest::GetHealth));
        assert!(!handler.can_handle(&ClientRpcRequest::InitCluster));
        assert!(!handler.can_handle(&ClientRpcRequest::GetClusterState));
        assert!(!handler.can_handle(&ClientRpcRequest::ReadKey {
            key: "test".to_string(),
        }));
        assert!(!handler.can_handle(&ClientRpcRequest::WriteKey {
            key: "test".to_string(),
            value: vec![1, 2, 3],
        }));
    }

    #[test]
    fn test_handler_name() {
        let handler = PijulHandler;
        assert_eq!(handler.name(), "PijulHandler");
    }
}
