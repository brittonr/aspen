//! MadsimNetworkFactory - creates MadsimRaftNetwork instances per target node.

use std::sync::Arc;

use openraft::network::RaftNetworkFactory;

use super::FailureInjector;
use super::MadsimRaftRouter;
use super::network::MadsimRaftNetwork;
use crate::types::AppTypeConfig;
use crate::types::NodeId;
use crate::types::RaftMemberInfo;

/// Madsim-compatible Raft network factory for deterministic simulation.
///
/// Creates MadsimRaftNetwork instances for each target node. Unlike IrpcRaftNetworkFactory
/// which uses Iroh EndpointAddr for peer discovery, MadsimNetworkFactory uses madsim's
/// TCP addresses (e.g., "127.0.0.1:26001") for deterministic network simulation.
#[derive(Clone)]
pub struct MadsimNetworkFactory {
    /// Source node ID for this factory.
    pub(crate) source_node_id: NodeId,
    /// Router managing all nodes in the simulation.
    router: Arc<MadsimRaftRouter>,
    /// Failure injector for chaos testing.
    failure_injector: Arc<FailureInjector>,
}

impl MadsimNetworkFactory {
    /// Create a new madsim network factory for the given source node.
    pub fn new(source_node_id: NodeId, router: Arc<MadsimRaftRouter>, failure_injector: Arc<FailureInjector>) -> Self {
        Self {
            source_node_id,
            router,
            failure_injector,
        }
    }
}

impl RaftNetworkFactory<AppTypeConfig> for MadsimNetworkFactory {
    type Network = MadsimRaftNetwork;

    #[tracing::instrument(level = "debug", skip_all)]
    async fn new_client(&mut self, target: NodeId, _node: &RaftMemberInfo) -> Self::Network {
        MadsimRaftNetwork::new(self.source_node_id, target, self.router.clone(), self.failure_injector.clone())
    }
}
