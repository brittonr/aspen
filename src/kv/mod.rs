pub mod client;
pub mod types;

use std::path::PathBuf;

use anyhow::Result;
use iroh::{EndpointAddr, EndpointId, SecretKey};

pub use self::client::KvClient;
pub use self::types::NodeId;

/// Builds a deterministic KV service surface for integration tests and examples.
pub struct KvServiceBuilder {
    node_id: NodeId,
    data_dir: PathBuf,
    peers: Vec<(NodeId, EndpointAddr)>,
}

impl KvServiceBuilder {
    pub fn new(node_id: NodeId, data_dir: impl Into<PathBuf>) -> Self {
        Self {
            node_id,
            data_dir: data_dir.into(),
            peers: Vec::new(),
        }
    }

    pub fn with_peers(mut self, peers: impl IntoIterator<Item = (NodeId, EndpointAddr)>) -> Self {
        self.peers.extend(peers);
        self
    }

    pub fn add_peer(mut self, peer: (NodeId, EndpointAddr)) -> Self {
        self.peers.push(peer);
        self
    }

    pub async fn start(self) -> Result<KvService> {
        let endpoint_id = generate_endpoint_id(self.node_id);
        let endpoint_addr = EndpointAddr::new(endpoint_id);
        Ok(KvService {
            node: KvNode {
                id: self.node_id,
                data_dir: self.data_dir,
            },
            endpoint_addr,
            peers: self.peers,
        })
    }
}

fn generate_endpoint_id(node_id: NodeId) -> EndpointId {
    let mut seed = [0u8; 32];
    seed[..8].copy_from_slice(&u64::from(node_id).to_le_bytes());
    SecretKey::from(seed).public()
}

/// Handle returned by [`KvServiceBuilder::start`].
pub struct KvService {
    node: KvNode,
    endpoint_addr: EndpointAddr,
    peers: Vec<(NodeId, EndpointAddr)>,
}

impl KvService {
    pub fn node(&self) -> &KvNode {
        &self.node
    }

    pub fn endpoint_addr(&self) -> &EndpointAddr {
        &self.endpoint_addr
    }

    pub fn peers(&self) -> &[(NodeId, EndpointAddr)] {
        &self.peers
    }
}

/// Basic information describing a KV node.
pub struct KvNode {
    pub id: NodeId,
    pub data_dir: PathBuf,
}
