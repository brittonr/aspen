/// In-memory Raft network implementation for deterministic testing.
///
/// Provides `RaftNetworkFactory` and `RaftNetworkV2` implementations that route
/// all RPCs through the `InnerRouter`'s simulated network layer.
use std::sync::Arc;

use aspen_raft::types::AppTypeConfig;
use aspen_raft::types::NodeId;
use aspen_raft::types::RaftMemberInfo;
use openraft::alias::VoteOf;
use openraft::error::RPCError;
use openraft::error::ReplicationClosed;
use openraft::error::StreamingError;
use openraft::network::RPCOption;
use openraft::network::v2::RaftNetworkV2;
use openraft::raft::AppendEntriesRequest;
use openraft::raft::AppendEntriesResponse;
use openraft::raft::SnapshotResponse;
use openraft::raft::VoteRequest;
use openraft::raft::VoteResponse;

use super::inner::InnerRouter;

/// Network factory for in-memory Raft nodes. Routes RPCs through the router's
/// simulated network layer with configurable delays and failures.
#[derive(Clone)]
pub(crate) struct InMemoryNetworkFactory {
    source: NodeId,
    router: Arc<InnerRouter>,
}

impl InMemoryNetworkFactory {
    pub(crate) fn new(source: NodeId, router: Arc<InnerRouter>) -> Self {
        Self { source, router }
    }
}

impl openraft::network::RaftNetworkFactory<AppTypeConfig> for InMemoryNetworkFactory {
    type Network = InMemoryNetwork;

    async fn new_client(&mut self, target: NodeId, _node: &RaftMemberInfo) -> Self::Network {
        InMemoryNetwork {
            source: self.source,
            target,
            router: self.router.clone(),
        }
    }
}

/// In-memory network client that routes RPCs through the router.
pub(crate) struct InMemoryNetwork {
    source: NodeId,
    target: NodeId,
    router: Arc<InnerRouter>,
}

impl RaftNetworkV2<AppTypeConfig> for InMemoryNetwork {
    async fn append_entries(
        &mut self,
        rpc: AppendEntriesRequest<AppTypeConfig>,
        _option: RPCOption,
    ) -> Result<AppendEntriesResponse<AppTypeConfig>, RPCError<AppTypeConfig>> {
        self.router.send_append_entries(self.source, self.target, rpc).await
    }

    async fn vote(
        &mut self,
        rpc: VoteRequest<AppTypeConfig>,
        _option: RPCOption,
    ) -> Result<VoteResponse<AppTypeConfig>, RPCError<AppTypeConfig>> {
        self.router.send_vote(self.source, self.target, rpc).await
    }

    async fn full_snapshot(
        &mut self,
        vote: VoteOf<AppTypeConfig>,
        snapshot: openraft::Snapshot<AppTypeConfig>,
        _cancel: impl std::future::Future<Output = ReplicationClosed> + openraft::OptionalSend + 'static,
        _option: RPCOption,
    ) -> Result<SnapshotResponse<AppTypeConfig>, StreamingError<AppTypeConfig>> {
        self.router.send_snapshot(self.source, self.target, vote, snapshot).await
    }
}
