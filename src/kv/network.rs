use std::collections::HashMap;
use std::io;
use std::sync::Arc;

use async_trait::async_trait;
use bincode;
use iroh::endpoint::{Connection, RecvStream, SendStream};
use iroh::protocol::{AcceptError, ProtocolHandler, Router};
use iroh::{Endpoint, EndpointAddr};
use openraft::BasicNode;
use openraft::error::{InstallSnapshotError, NetworkError, RPCError, RaftError, RemoteError};
use openraft::network::{RPCOption, RaftNetwork, RaftNetworkFactory};
use openraft::raft::{
    AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest, InstallSnapshotResponse,
    VoteRequest, VoteResponse,
};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tokio::sync::{Mutex, RwLock};

use crate::kv::error::Error;

#[async_trait]
pub trait KvNetworkRuntime: RaftNetworkFactory<TypeConfig> + Clone + Send + Sync + 'static {
    type Addr: Clone + Send + Sync + 'static;

    async fn start(&self, raft: openraft::Raft<TypeConfig>) -> Result<(), Error>;

    async fn register_peer(&self, node_id: NodeId, addr: Self::Addr);

    fn local_addr(&self) -> Self::Addr;
}
use crate::kv::types::{NodeId, TypeConfig};

const ALPN: &[u8] = b"aspen/raft/0";

#[derive(Clone)]
pub struct KvNetworkFactory {
    inner: Arc<KvNetworkInner>,
}

struct KvNetworkInner {
    endpoint: Endpoint,
    peers: RwLock<HashMap<NodeId, EndpointAddr>>,
    router: Mutex<Option<Router>>,
}

impl KvNetworkFactory {
    pub async fn bind() -> Result<Self, Error> {
        let endpoint = Endpoint::bind().await.map_err(|e| Error::Raft {
            message: e.to_string(),
        })?;
        Ok(Self {
            inner: Arc::new(KvNetworkInner {
                endpoint,
                peers: RwLock::new(HashMap::new()),
                router: Mutex::new(None),
            }),
        })
    }

    pub async fn start(&self, raft: openraft::Raft<TypeConfig>) -> Result<(), Error> {
        let mut router = self.inner.router.lock().await;
        if router.is_some() {
            return Ok(());
        }
        let protocol = RaftProtocol { raft };
        let endpoint = self.inner.endpoint.clone();
        let handle = Router::builder(endpoint).accept(ALPN, protocol).spawn();
        handle.endpoint().online().await;
        *router = Some(handle);
        Ok(())
    }

    pub async fn register_peer(&self, node_id: NodeId, addr: EndpointAddr) {
        self.inner.peers.write().await.insert(node_id, addr);
    }

    pub fn local_addr(&self) -> EndpointAddr {
        self.inner.endpoint.addr()
    }

    async fn lookup(&self, node_id: NodeId) -> Option<EndpointAddr> {
        self.inner.peers.read().await.get(&node_id).cloned()
    }
}

#[async_trait]
impl KvNetworkRuntime for KvNetworkFactory {
    type Addr = EndpointAddr;

    async fn start(&self, raft: openraft::Raft<TypeConfig>) -> Result<(), Error> {
        KvNetworkFactory::start(self, raft).await
    }

    async fn register_peer(&self, node_id: NodeId, addr: Self::Addr) {
        KvNetworkFactory::register_peer(self, node_id, addr).await;
    }

    fn local_addr(&self) -> Self::Addr {
        KvNetworkFactory::local_addr(self)
    }
}

impl RaftNetworkFactory<TypeConfig> for KvNetworkFactory {
    type Network = KvNetwork;

    async fn new_client(&mut self, target: NodeId, _node: &BasicNode) -> Self::Network {
        let addr = self.lookup(target).await;
        KvNetwork {
            endpoint: self.inner.endpoint.clone(),
            target,
            addr,
        }
    }
}

pub struct KvNetwork {
    endpoint: Endpoint,
    target: NodeId,
    addr: Option<EndpointAddr>,
}

impl KvNetwork {
    async fn send(&self, message: &NetworkMessage) -> io::Result<NetworkResponse> {
        let addr = self
            .addr
            .clone()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "peer address unknown"))?;
        let connection = self
            .endpoint
            .connect(addr, ALPN)
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        let (mut send, mut recv) = connection
            .open_bi()
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        let payload = bincode::serialize(message)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        write_frame(&mut send, &payload).await?;
        send.finish()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        let response = read_frame(&mut recv).await?;
        let decoded: NetworkResponse = bincode::deserialize(&response)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        Ok(decoded)
    }
}

impl RaftNetwork<TypeConfig> for KvNetwork {
    async fn append_entries(
        &mut self,
        req: AppendEntriesRequest<TypeConfig>,
        _option: RPCOption,
    ) -> Result<AppendEntriesResponse<NodeId>, RPCError<NodeId, BasicNode, RaftError<NodeId>>> {
        let response = self
            .send(&NetworkMessage::AppendEntries(req))
            .await
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;
        match response {
            NetworkResponse::AppendEntries(result) => {
                result.map_err(|err| RPCError::RemoteError(RemoteError::new(self.target, err)))
            }
            _ => Err(RPCError::Network(invalid_response_error())),
        }
    }

    async fn install_snapshot(
        &mut self,
        req: InstallSnapshotRequest<TypeConfig>,
        _option: RPCOption,
    ) -> Result<
        InstallSnapshotResponse<NodeId>,
        RPCError<NodeId, BasicNode, RaftError<NodeId, InstallSnapshotError>>,
    > {
        let response = self
            .send(&NetworkMessage::InstallSnapshot(req))
            .await
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;
        match response {
            NetworkResponse::InstallSnapshot(result) => {
                result.map_err(|err| RPCError::RemoteError(RemoteError::new(self.target, err)))
            }
            _ => Err(RPCError::Network(invalid_response_error())),
        }
    }

    async fn vote(
        &mut self,
        req: VoteRequest<NodeId>,
        _option: RPCOption,
    ) -> Result<VoteResponse<NodeId>, RPCError<NodeId, BasicNode, RaftError<NodeId>>> {
        let response = self
            .send(&NetworkMessage::Vote(req))
            .await
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;
        match response {
            NetworkResponse::Vote(result) => {
                result.map_err(|err| RPCError::RemoteError(RemoteError::new(self.target, err)))
            }
            _ => Err(RPCError::Network(invalid_response_error())),
        }
    }
}

#[derive(Serialize, Deserialize)]
enum NetworkMessage {
    AppendEntries(AppendEntriesRequest<TypeConfig>),
    InstallSnapshot(InstallSnapshotRequest<TypeConfig>),
    Vote(VoteRequest<NodeId>),
}

#[derive(Serialize, Deserialize)]
enum NetworkResponse {
    AppendEntries(Result<AppendEntriesResponse<NodeId>, RaftError<NodeId>>),
    InstallSnapshot(
        Result<InstallSnapshotResponse<NodeId>, RaftError<NodeId, InstallSnapshotError>>,
    ),
    Vote(Result<VoteResponse<NodeId>, RaftError<NodeId>>),
}

#[derive(Clone)]
struct RaftProtocol {
    raft: openraft::Raft<TypeConfig>,
}

impl std::fmt::Debug for RaftProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RaftProtocol").finish()
    }
}

impl ProtocolHandler for RaftProtocol {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let (mut send, mut recv) = connection.accept_bi().await?;
        let payload = read_frame(&mut recv)
            .await
            .map_err(|e| AcceptError::from_err(e))?;
        let message: NetworkMessage =
            bincode::deserialize(&payload).map_err(|e| AcceptError::from_err(e))?;
        let response = match message {
            NetworkMessage::AppendEntries(req) => {
                let res = self.raft.append_entries(req).await;
                NetworkResponse::AppendEntries(res)
            }
            NetworkMessage::InstallSnapshot(req) => {
                let res = self.raft.install_snapshot(req).await;
                NetworkResponse::InstallSnapshot(res)
            }
            NetworkMessage::Vote(req) => {
                let res = self.raft.vote(req).await;
                NetworkResponse::Vote(res)
            }
        };
        let bytes = bincode::serialize(&response).map_err(|e| AcceptError::from_err(e))?;
        write_frame(&mut send, &bytes)
            .await
            .map_err(|e| AcceptError::from_err(e))?;
        send.finish().map_err(|e| AcceptError::from_err(e))?;
        Ok(())
    }
}

async fn write_frame(stream: &mut SendStream, data: &[u8]) -> io::Result<()> {
    let len = data.len() as u32;
    stream.write_all(&len.to_be_bytes()).await?;
    stream.write_all(data).await?;
    stream.flush().await?;
    Ok(())
}

async fn read_frame(stream: &mut RecvStream) -> io::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    stream
        .read_exact(&mut len_buf)
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut data = vec![0u8; len];
    stream
        .read_exact(&mut data)
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    Ok(data)
}

fn invalid_response_error() -> NetworkError {
    let err = io::Error::new(io::ErrorKind::InvalidData, "invalid response");
    NetworkError::new(&err)
}
