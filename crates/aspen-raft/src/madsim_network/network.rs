//! MadsimRaftNetwork - RaftNetworkV2 implementation using deterministic TCP.

use std::sync::Arc;

use openraft::OptionalSend;
use openraft::Snapshot;
use openraft::error::NetworkError;
use openraft::error::RPCError;
use openraft::error::ReplicationClosed;
use openraft::error::StreamingError;
use openraft::error::Unreachable;
use openraft::network::RPCOption;
use openraft::network::v2::RaftNetworkV2;
use openraft::raft::AppendEntriesRequest;
use openraft::raft::AppendEntriesResponse;
use openraft::raft::SnapshotResponse;
use openraft::raft::VoteRequest;
use openraft::raft::VoteResponse;
use openraft::type_config::alias::VoteOf;

use super::FailureInjector;
use super::MadsimRaftRouter;
use crate::types::AppTypeConfig;
use crate::types::NodeId;

/// Madsim-compatible Raft network implementation using deterministic TCP.
///
/// Implements OpenRaft's RaftNetworkV2 trait to send vote, append_entries, and snapshot
/// RPCs over madsim's deterministic network layer. All network operations are reproducible
/// given the same seed, enabling automated detection of distributed systems bugs.
pub struct MadsimRaftNetwork {
    source: NodeId,
    target: NodeId,
    router: Arc<MadsimRaftRouter>,
    failure_injector: Arc<FailureInjector>,
}

impl MadsimRaftNetwork {
    pub(super) fn new(
        source: NodeId,
        target: NodeId,
        router: Arc<MadsimRaftRouter>,
        failure_injector: Arc<FailureInjector>,
    ) -> Self {
        Self {
            source,
            target,
            router,
            failure_injector,
        }
    }

    /// Check if the failure injector should drop this message.
    ///
    /// Returns Err(RPCError::Unreachable) if the message should be dropped,
    /// otherwise Ok(()).
    async fn check_failure_injection(&self) -> Result<(), RPCError<AppTypeConfig>> {
        if self.failure_injector.should_drop_message(self.source, self.target) {
            return Err(RPCError::Unreachable(Unreachable::new(&std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                "failure injector dropped message",
            ))));
        }
        Ok(())
    }

    /// Apply network delay if configured by the failure injector.
    async fn apply_network_delay(&self) {
        if let Some(delay) = self.failure_injector.get_network_delay(self.source, self.target) {
            madsim::time::sleep(delay).await;
        }
    }
}

impl RaftNetworkV2<AppTypeConfig> for MadsimRaftNetwork {
    async fn append_entries(
        &mut self,
        rpc: AppendEntriesRequest<AppTypeConfig>,
        _option: RPCOption,
    ) -> Result<AppendEntriesResponse<AppTypeConfig>, RPCError<AppTypeConfig>> {
        self.check_failure_injection().await?;
        self.apply_network_delay().await;

        self.router.send_append_entries(self.source, self.target, rpc).await
    }

    async fn vote(
        &mut self,
        rpc: VoteRequest<AppTypeConfig>,
        _option: RPCOption,
    ) -> Result<VoteResponse<AppTypeConfig>, RPCError<AppTypeConfig>> {
        self.check_failure_injection().await?;
        self.apply_network_delay().await;

        self.router.send_vote(self.source, self.target, rpc).await
    }

    async fn full_snapshot(
        &mut self,
        vote: VoteOf<AppTypeConfig>,
        snapshot: Snapshot<AppTypeConfig>,
        _cancel: impl std::future::Future<Output = ReplicationClosed> + OptionalSend + 'static,
        _option: RPCOption,
    ) -> Result<SnapshotResponse<AppTypeConfig>, StreamingError<AppTypeConfig>> {
        // Check failure injection first
        if let Err(_rpc_err) = self.check_failure_injection().await {
            return Err(StreamingError::Network(NetworkError::new(&std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                "failure injector dropped snapshot message",
            ))));
        }
        self.apply_network_delay().await;

        self.router.send_snapshot(self.source, self.target, vote, snapshot).await
    }
}
