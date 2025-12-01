use std::env;
use std::error::Error;
use std::path::PathBuf;

use aspen::KvServiceBuilder;
use aspen::kv::types::NodeId;
use iroh::{EndpointAddr, EndpointId};
use std::str::FromStr;

const ID_ENV: &str = "ASPEN_KV_NODE_ID";
const DATA_DIR_ENV: &str = "ASPEN_KV_DATA_DIR";
const PEERS_ENV: &str = "ASPEN_KV_PEERS";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = EnvConfig::from_env()?;
    let peer_count = config.peer_count;
    let service = config.into_builder().start().await?;
    println!(
        "KvNode {} ready at {:?} with {} configured peers",
        service.node().id,
        service.endpoint_addr(),
        peer_count
    );
    println!("Client handle acquired for node {}", service.node().id);

    Ok(())
}

struct EnvConfig {
    node_id: NodeId,
    data_dir: PathBuf,
    peers: Vec<(NodeId, EndpointAddr)>,
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

    fn into_builder(self) -> KvServiceBuilder {
        KvServiceBuilder::new(self.node_id, self.data_dir).with_peers(self.peers)
    }
}

fn parse_peers(raw: Option<String>) -> Result<Vec<(NodeId, EndpointAddr)>, Box<dyn Error>> {
    let mut peers = Vec::new();
    if let Some(raw) = raw {
        for spec in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            let (id_str, addr_str) = spec
                .split_once('=')
                .ok_or_else(|| format!("invalid peer spec '{spec}', expected id=addr"))?;
            let node_id: NodeId = id_str.parse()?;
            let addr = parse_endpoint_addr(addr_str)?;
            peers.push((node_id, addr));
        }
    }
    Ok(peers)
}

fn parse_endpoint_addr(input: &str) -> Result<EndpointAddr, Box<dyn Error>> {
    if let Ok(addr) = serde_json::from_str::<EndpointAddr>(input) {
        return Ok(addr);
    }
    let endpoint_id = EndpointId::from_str(input)?;
    Ok(EndpointAddr::new(endpoint_id))
}
