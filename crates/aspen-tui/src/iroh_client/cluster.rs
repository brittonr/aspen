//! Cluster-related RPC methods for IrohClient.

use anyhow::Result;
use aspen_client::AddLearnerResultResponse;
use aspen_client::ChangeMembershipResultResponse;
use aspen_client::CheckpointWalResultResponse;
use aspen_client::ClientRpcRequest;
use aspen_client::ClientRpcResponse;
use aspen_client::ClusterStateResponse;
use aspen_client::ClusterTicketResponse;
use aspen_client::HealthResponse;
use aspen_client::InitResultResponse;
use aspen_client::MetricsResponse;
use aspen_client::NodeInfoResponse;
use aspen_client::PromoteLearnerResultResponse;
use aspen_client::RaftMetricsResponse;
use aspen_client::SnapshotResultResponse;

use super::IrohClient;

impl IrohClient {
    /// Get health status from the node.
    pub async fn get_health(&self) -> Result<HealthResponse> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::GetHealth).await?;

        match response {
            ClientRpcResponse::Health(health) => Ok(health),
            _ => anyhow::bail!("unexpected response type for GetHealth"),
        }
    }

    /// Get Raft metrics from the node.
    pub async fn get_raft_metrics(&self) -> Result<RaftMetricsResponse> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::GetRaftMetrics).await?;

        match response {
            ClientRpcResponse::RaftMetrics(metrics) => Ok(metrics),
            _ => anyhow::bail!("unexpected response type for GetRaftMetrics"),
        }
    }

    /// Get the current leader node ID.
    pub async fn get_leader(&self) -> Result<Option<u64>> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::GetLeader).await?;

        match response {
            ClientRpcResponse::Leader(leader) => Ok(leader),
            _ => anyhow::bail!("unexpected response type for GetLeader"),
        }
    }

    /// Get node information.
    pub async fn get_node_info(&self) -> Result<NodeInfoResponse> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::GetNodeInfo).await?;

        match response {
            ClientRpcResponse::NodeInfo(info) => Ok(info),
            _ => anyhow::bail!("unexpected response type for GetNodeInfo"),
        }
    }

    /// Get cluster ticket for joining.
    pub async fn get_cluster_ticket(&self) -> Result<ClusterTicketResponse> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::GetClusterTicket).await?;

        match response {
            ClientRpcResponse::ClusterTicket(ticket) => Ok(ticket),
            _ => anyhow::bail!("unexpected response type for GetClusterTicket"),
        }
    }

    /// Initialize a new cluster.
    pub async fn init_cluster(&self) -> Result<InitResultResponse> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::InitCluster).await?;

        match response {
            ClientRpcResponse::InitResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for InitCluster"),
        }
    }

    /// Send a ping to check connectivity.
    pub async fn ping(&self) -> Result<()> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::Ping).await?;

        match response {
            ClientRpcResponse::Pong => Ok(()),
            _ => anyhow::bail!("unexpected response type for Ping"),
        }
    }

    /// Get cluster state with all known nodes.
    pub async fn get_cluster_state(&self) -> Result<ClusterStateResponse> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::GetClusterState).await?;

        match response {
            ClientRpcResponse::ClusterState(state) => Ok(state),
            _ => anyhow::bail!("unexpected response type for GetClusterState"),
        }
    }

    /// Get Prometheus-format metrics from the node.
    pub async fn get_metrics(&self) -> Result<MetricsResponse> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::GetMetrics).await?;

        match response {
            ClientRpcResponse::Metrics(metrics) => Ok(metrics),
            _ => anyhow::bail!("unexpected response type for GetMetrics"),
        }
    }

    /// Add a learner node to the cluster.
    pub async fn add_learner(&self, node_id: u64, addr: String) -> Result<AddLearnerResultResponse> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::AddLearner { node_id, addr }).await?;

        match response {
            ClientRpcResponse::AddLearnerResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for AddLearner"),
        }
    }

    /// Change cluster membership.
    pub async fn change_membership(&self, members: Vec<u64>) -> Result<ChangeMembershipResultResponse> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::ChangeMembership { members }).await?;

        match response {
            ClientRpcResponse::ChangeMembershipResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for ChangeMembership"),
        }
    }

    /// Promote a learner node to voter.
    pub async fn promote_learner(
        &self,
        learner_id: u64,
        replace_node: Option<u64>,
        is_force: bool,
    ) -> Result<PromoteLearnerResultResponse> {
        let response = self
            .send_rpc_with_retry(ClientRpcRequest::PromoteLearner {
                learner_id,
                replace_node,
                is_force,
            })
            .await?;

        match response {
            ClientRpcResponse::PromoteLearnerResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for PromoteLearner"),
        }
    }

    /// Trigger a Raft snapshot.
    pub async fn trigger_snapshot(&self) -> Result<SnapshotResultResponse> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::TriggerSnapshot).await?;

        match response {
            ClientRpcResponse::SnapshotResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for TriggerSnapshot"),
        }
    }

    /// Force a SQLite WAL checkpoint.
    pub async fn checkpoint_wal(&self) -> Result<CheckpointWalResultResponse> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::CheckpointWal).await?;

        match response {
            ClientRpcResponse::CheckpointWalResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for CheckpointWal"),
        }
    }
}
