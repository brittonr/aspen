use std::fs;
use std::path::Path;
use std::sync::Arc;

use openraft::Config;
use openraft::Raft;

use crate::error::Result as AspenResult;
use crate::kv::client::KvClient;
use crate::kv::error::Error as KvError;
use crate::kv::network::{KvNetworkFactory, KvNetworkRuntime};
use crate::kv::store::{KvLogStore, KvStateMachine, new_storage};
use crate::kv::types::{NodeId, TypeConfig};

pub type KvNode = GenericKvNode<KvNetworkFactory>;

pub struct GenericKvNode<N>
where
    N: KvNetworkRuntime,
{
    pub id: NodeId,
    pub raft: Raft<TypeConfig>,
    pub state_machine: KvStateMachine,
    pub log_store: KvLogStore,
    pub network: N,
}

impl KvNode {
    pub async fn start<P: AsRef<Path>>(id: NodeId, data_dir: P) -> AspenResult<Self> {
        let network = KvNetworkFactory::bind().await?;
        let config = default_config()?;
        GenericKvNode::start_with_network(id, data_dir, network, config).await
    }
}

impl<N> GenericKvNode<N>
where
    N: KvNetworkRuntime,
{
    pub async fn start_with_network<P: AsRef<Path>>(
        id: NodeId,
        data_dir: P,
        network: N,
        config: Arc<Config>,
    ) -> AspenResult<Self> {
        let data_dir = data_dir.as_ref();
        if !data_dir.exists() {
            fs::create_dir_all(data_dir).map_err(|e| KvError::Io { source: e })?;
        }
        let db_path = data_dir.join(DEFAULT_STORE_FILE);
        let db_str = db_path.to_str().ok_or_else(|| KvError::Raft {
            message: "invalid path".into(),
        })?;
        let (log_store, state_machine) = new_storage(db_str).await?;

        let raft = Raft::new(
            id,
            config,
            network.clone(),
            log_store.clone(),
            state_machine.clone(),
        )
        .await
        .map_err(|e| KvError::Raft {
            message: e.to_string(),
        })?;
        network.start(raft.clone()).await?;

        Ok(Self {
            id,
            raft,
            state_machine,
            log_store,
            network,
        })
    }

    pub fn endpoint_addr(&self) -> N::Addr {
        self.network.local_addr()
    }

    pub async fn register_peer(&self, node_id: NodeId, addr: N::Addr) {
        self.network.register_peer(node_id, addr).await;
    }

    pub fn client(&self) -> KvClient {
        KvClient::new(self.raft.clone(), self.state_machine.clone())
    }
}

const DEFAULT_STORE_FILE: &str = "kv.redb";

pub(crate) fn default_config() -> Result<Arc<Config>, KvError> {
    let config = Config::build(&["aspen-kv"]).map_err(|e| KvError::Raft {
        message: e.to_string(),
    })?;
    Ok(Arc::new(config))
}
