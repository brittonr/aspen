use std::env;
use std::error::Error;
use std::path::PathBuf;

use aspen::NodeBuilder;
use aspen::node::types::NodeId;

const ID_ENV: &str = "ASPEN_NODE_ID";
const DATA_DIR_ENV: &str = "ASPEN_NODE_DATA_DIR";
const PEERS_ENV: &str = "ASPEN_NODE_PEERS";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = EnvConfig::from_env()?;
    let peer_count = config.peer_count;
    let node = config.into_builder().start().await?;
    println!(
        "Node {} ready at {:?} with {} configured peers",
        node.node_id(),
        node.endpoint_addr(),
        peer_count
    );
    println!("Client handle acquired for node {}", node.node_id());

    Ok(())
}

struct EnvConfig {
    node_id: NodeId,
    data_dir: PathBuf,
    peers: Vec<String>,
    peer_count: usize,
}

impl EnvConfig {
    fn from_env() -> Result<Self, Box<dyn Error>> {
        let node_id: NodeId = env::var(ID_ENV)?.parse()?;
        let data_dir = PathBuf::from(env::var(DATA_DIR_ENV)?);
        let peers = parse_peers(env::var(PEERS_ENV).ok())?;
        let peer_count = peers.len();
        Ok(Self {
            node_id,
            data_dir,
            peers,
            peer_count,
        })
    }

    fn into_builder(self) -> NodeBuilder {
        NodeBuilder::new(self.node_id, self.data_dir).with_peers(self.peers)
    }
}

fn parse_peers(raw: Option<String>) -> Result<Vec<String>, Box<dyn Error>> {
    let mut peers = Vec::new();
    if let Some(raw) = raw {
        for spec in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            peers.push(spec.to_string());
        }
    }
    Ok(peers)
}
