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
