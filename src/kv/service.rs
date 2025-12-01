use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use openraft::Config;

use crate::error::Result;
use crate::kv::client::KvClient;
use crate::kv::network::{KvNetworkFactory, KvNetworkRuntime};
use crate::kv::node::{GenericKvNode, default_config};
use crate::kv::types::NodeId;

pub type KvServiceBuilder = GenericKvServiceBuilder<KvNetworkFactory>;
pub type KvService = GenericKvService<KvNetworkFactory>;

/// Builder for bootstrapping a [`GenericKvNode`] from configuration-style inputs.
///
/// The builder keeps peer metadata so that each node registers known neighbors
/// immediately after the Raft instance is booted.
pub struct GenericKvServiceBuilder<N>
where
    N: KvNetworkRuntime,
{
    node_id: NodeId,
    data_dir: PathBuf,
    peers: Vec<(NodeId, N::Addr)>,
    config: Option<Arc<Config>>,
    _marker: PhantomData<N>,
}

impl<N> GenericKvServiceBuilder<N>
where
    N: KvNetworkRuntime,
{
    pub fn new(node_id: NodeId, data_dir: impl Into<PathBuf>) -> Self {
        Self {
            node_id,
            data_dir: data_dir.into(),
            peers: Vec::new(),
            config: None,
            _marker: PhantomData,
        }
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    pub fn peers(&self) -> &[(NodeId, N::Addr)] {
        &self.peers
    }

    pub fn with_peer(mut self, node_id: NodeId, addr: N::Addr) -> Self {
        self.peers.push((node_id, addr));
        self
    }

    pub fn with_peers<I>(mut self, peers: I) -> Self
    where
        I: IntoIterator<Item = (NodeId, N::Addr)>,
    {
        self.peers.extend(peers);
        self
    }

    pub fn with_config(mut self, config: Arc<Config>) -> Self {
        self.config = Some(config);
        self
    }

    pub async fn start_with_network(self, network: N) -> Result<GenericKvService<N>> {
        let config = match self.config {
            Some(cfg) => cfg,
            None => default_config()?,
        };
        let node = GenericKvNode::start_with_network(self.node_id, &self.data_dir, network, config)
            .await?;
        for (peer_id, addr) in &self.peers {
            node.register_peer(*peer_id, addr.clone()).await;
        }
        Ok(GenericKvService { node })
    }
}

impl GenericKvServiceBuilder<KvNetworkFactory> {
    pub async fn start(self) -> Result<KvService> {
        let network = KvNetworkFactory::bind().await?;
        self.start_with_network(network).await
    }
}

/// Convenience wrapper around [`GenericKvNode`] that exposes the client handle and
/// network metadata for reuse.
pub struct GenericKvService<N>
where
    N: KvNetworkRuntime,
{
    node: GenericKvNode<N>,
}

impl<N> GenericKvService<N>
where
    N: KvNetworkRuntime,
{
    pub fn node(&self) -> &GenericKvNode<N> {
        &self.node
    }

    pub fn into_node(self) -> GenericKvNode<N> {
        self.node
    }

    pub fn client(&self) -> KvClient {
        self.node.client()
    }

    pub fn endpoint_addr(&self) -> N::Addr {
        self.node.endpoint_addr()
    }
}
