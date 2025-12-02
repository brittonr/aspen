use std::collections::HashMap;
use std::fmt::Display;
use std::future::Future;

use anyhow::Context;
use openraft::{BasicNode, OptionalSend, RaftTypeConfig, Snapshot, StorageError};
use openraft::error::{
    Infallible, InstallSnapshotError, NetworkError, RPCError, RaftError, RemoteError,
    ReplicationClosed, StreamingError, Unreachable,
};
use openraft::network::{RPCOption, RaftNetwork, RaftNetworkFactory, v2::RaftNetworkV2};
use openraft::raft::{
    AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest, InstallSnapshotResponse,
    SnapshotResponse, VoteRequest, VoteResponse,
};
use openraft::type_config::alias::VoteOf;
use reqwest::Client;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeek, AsyncWrite};
use tokio::select;
use tracing::warn;

use crate::cluster::IrohEndpointManager;
use crate::raft::rpc::{
    RaftAppendEntriesRequest, RaftRpcProtocol, RaftRpcResponse, RaftSnapshotRequest, RaftVoteRequest,
};
use crate::raft::types::{AppTypeConfig, NodeId};
use std::sync::Arc;

/// Maximum size for RPC messages (10 MB).
///
/// Tiger Style: Fixed limit to prevent unbounded memory use.
const MAX_RPC_MESSAGE_SIZE: usize = 10 * 1024 * 1024;

/// HTTP-based Raft network factory modeled after the upstream memstore example.
pub struct HttpRaftNetworkFactory {
    client: Client,
}

impl HttpRaftNetworkFactory {
    pub fn new() -> Self {
        let client = Client::builder().no_proxy().build().expect("http client");
        Self { client }
    }
}

impl<C> RaftNetworkFactory<C> for HttpRaftNetworkFactory
where
    C: RaftTypeConfig<Node = BasicNode>,
    <C as RaftTypeConfig>::SnapshotData: AsyncRead + AsyncWrite + AsyncSeek + Unpin,
{
    type Network = HttpRaftNetwork<C>;

    #[tracing::instrument(level = "debug", skip_all)]
    async fn new_client(&mut self, target: C::NodeId, node: &BasicNode) -> Self::Network {
        HttpRaftNetwork {
            addr: node.addr.clone(),
            client: self.client.clone(),
            target,
        }
    }
}

pub struct HttpRaftNetwork<C>
where
    C: RaftTypeConfig,
{
    addr: String,
    client: Client,
    target: C::NodeId,
}

impl<C> HttpRaftNetwork<C>
where
    C: RaftTypeConfig,
{
    async fn request<Req, Resp, Err>(
        &mut self,
        path: impl Display,
        req: Req,
    ) -> Result<Result<Resp, Err>, RPCError<C>>
    where
        Req: Serialize + 'static,
        Resp: Serialize + DeserializeOwned,
        Err: std::error::Error + Serialize + DeserializeOwned,
    {
        let url = format!("http://{}/raft/{}", self.addr, path);
        let resp = self
            .client
            .post(url.clone())
            .json(&req)
            .send()
            .await
            .map_err(|err| {
                if err.is_connect() {
                    RPCError::Unreachable(Unreachable::new(&err))
                } else {
                    RPCError::Network(NetworkError::new(&err))
                }
            })?;

        let res: Result<Resp, Err> = resp.json().await.map_err(|err| NetworkError::new(&err))?;
        Ok(res)
    }
}

#[allow(clippy::blocks_in_conditions)]
impl<C> RaftNetwork<C> for HttpRaftNetwork<C>
where
    C: RaftTypeConfig,
{
    #[tracing::instrument(level = "debug", skip_all, err(Debug))]
    async fn append_entries(
        &mut self,
        req: AppendEntriesRequest<C>,
        _option: RPCOption,
    ) -> Result<AppendEntriesResponse<C>, RPCError<C, RaftError<C>>> {
        let res = self
            .request::<_, _, Infallible>("append", req)
            .await
            .map_err(RPCError::with_raft_error)?;
        Ok(res.unwrap())
    }

    #[tracing::instrument(level = "debug", skip_all, err(Debug))]
    async fn install_snapshot(
        &mut self,
        req: InstallSnapshotRequest<C>,
        _option: RPCOption,
    ) -> Result<InstallSnapshotResponse<C>, RPCError<C, RaftError<C, InstallSnapshotError>>> {
        let res = self
            .request("snapshot", req)
            .await
            .map_err(RPCError::with_raft_error)?;
        match res {
            Ok(resp) => Ok(resp),
            Err(err) => Err(RPCError::RemoteError(RemoteError::new(
                self.target.clone(),
                RaftError::APIError(err),
            ))),
        }
    }

    #[tracing::instrument(level = "debug", skip_all, err(Debug))]
    async fn vote(
        &mut self,
        req: VoteRequest<C>,
        _option: RPCOption,
    ) -> Result<VoteResponse<C>, RPCError<C, RaftError<C>>> {
        let res = self
            .request::<_, _, Infallible>("vote", req)
            .await
            .map_err(RPCError::with_raft_error)?;
        Ok(res.unwrap())
    }
}

/// IRPC-based Raft network factory for Iroh P2P transport.
///
/// Tiger Style: Fixed peer map, explicit endpoint management.
pub struct IrpcRaftNetworkFactory {
    endpoint_manager: Arc<IrohEndpointManager>,
    /// Map of NodeId to Iroh EndpointAddr for peer discovery.
    /// Populated from CLI args or config during bootstrap.
    peer_addrs: HashMap<NodeId, iroh::EndpointAddr>,
}

impl IrpcRaftNetworkFactory {
    pub fn new(
        endpoint_manager: Arc<IrohEndpointManager>,
        peer_addrs: HashMap<NodeId, iroh::EndpointAddr>,
    ) -> Self {
        Self {
            endpoint_manager,
            peer_addrs,
        }
    }
}

impl RaftNetworkFactory<AppTypeConfig> for IrpcRaftNetworkFactory {
    type Network = IrpcRaftNetwork;

    #[tracing::instrument(level = "debug", skip_all)]
    async fn new_client(&mut self, target: NodeId, _node: &BasicNode) -> Self::Network {
        // Look up peer's Iroh address
        let peer_addr = self.peer_addrs.get(&target).cloned();

        IrpcRaftNetwork {
            endpoint_manager: Arc::clone(&self.endpoint_manager),
            peer_addr,
            target,
        }
    }
}

/// IRPC-based Raft network client for a single peer.
///
/// Tiger Style:
/// - Explicit error handling for connection failures
/// - Fail fast if peer address is missing
/// - Bounded retries handled by IRPC/Iroh layers
pub struct IrpcRaftNetwork {
    endpoint_manager: Arc<IrohEndpointManager>,
    peer_addr: Option<iroh::EndpointAddr>,
    target: NodeId,
}

impl IrpcRaftNetwork {
    /// Send an RPC request to the peer and wait for response.
    ///
    /// Tiger Style: Fail fast if peer address is unknown.
    ///
    /// This implements a simple request-response pattern:
    /// 1. Serialize the protocol enum (request without channels)
    /// 2. Send it over Iroh bidirectional stream
    /// 3. Wait for response on the same stream
    /// 4. Deserialize and return the response
    async fn send_rpc(&self, request: RaftRpcProtocol) -> anyhow::Result<RaftRpcResponse> {
        let peer_addr = self
            .peer_addr
            .as_ref()
            .context("peer address not found in peer map")?;

        let endpoint = self.endpoint_manager.endpoint();

        // Open a connection to the peer
        let connection = endpoint
            .connect(peer_addr.clone(), b"raft-rpc")
            .await
            .context("failed to connect to peer")?;

        // Open bidirectional stream
        let (mut send_stream, mut recv_stream) = connection
            .open_bi()
            .await
            .context("failed to open bidirectional stream")?;

        // Serialize and send the request
        let serialized = postcard::to_stdvec(&request).context("failed to serialize RPC request")?;
        send_stream
            .write_all(&serialized)
            .await
            .context("failed to write RPC request to stream")?;
        send_stream
            .finish()
            .context("failed to finish send stream")?;

        // Read response (with 10 MB size limit)
        let response_buf = recv_stream
            .read_to_end(MAX_RPC_MESSAGE_SIZE)
            .await
            .context("failed to read RPC response")?;

        // Deserialize response
        let response: RaftRpcResponse = postcard::from_bytes(&response_buf)
            .context("failed to deserialize RPC response")?;

        Ok(response)
    }
}

#[allow(clippy::blocks_in_conditions)]
impl RaftNetworkV2<AppTypeConfig> for IrpcRaftNetwork {
    #[tracing::instrument(level = "debug", skip_all, err(Debug))]
    async fn append_entries(
        &mut self,
        rpc: AppendEntriesRequest<AppTypeConfig>,
        _option: RPCOption,
    ) -> Result<AppendEntriesResponse<AppTypeConfig>, RPCError<AppTypeConfig>> {
        let request = RaftAppendEntriesRequest { request: rpc };
        let protocol = RaftRpcProtocol::AppendEntries(request);

        // Send the RPC and get response
        let response = self.send_rpc(protocol).await.map_err(|err| {
            warn!(target_node = %self.target, error = %err, "failed to send append_entries RPC");
            let err_str = err.to_string();
            RPCError::Unreachable(Unreachable::new(&std::io::Error::new(
                std::io::ErrorKind::Other,
                err_str,
            )))
        })?;

        // Extract result from response
        match response {
            RaftRpcResponse::AppendEntries(result) => Ok(result),
            _ => Err(RPCError::Network(NetworkError::new(&std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "unexpected response type for append_entries",
            )))),
        }
    }

    #[tracing::instrument(level = "debug", skip_all, err(Debug))]
    async fn vote(
        &mut self,
        rpc: VoteRequest<AppTypeConfig>,
        _option: RPCOption,
    ) -> Result<VoteResponse<AppTypeConfig>, RPCError<AppTypeConfig>> {
        let request = RaftVoteRequest { request: rpc };
        let protocol = RaftRpcProtocol::Vote(request);

        // Send the RPC and get response
        let response = self.send_rpc(protocol).await.map_err(|err| {
            warn!(target_node = %self.target, error = %err, "failed to send vote RPC");
            let err_str = err.to_string();
            RPCError::Unreachable(Unreachable::new(&std::io::Error::new(
                std::io::ErrorKind::Other,
                err_str,
            )))
        })?;

        // Extract result from response
        match response {
            RaftRpcResponse::Vote(result) => Ok(result),
            _ => Err(RPCError::Network(NetworkError::new(&std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "unexpected response type for vote",
            )))),
        }
    }

    #[tracing::instrument(level = "debug", skip_all, err(Debug))]
    async fn full_snapshot(
        &mut self,
        vote: VoteOf<AppTypeConfig>,
        snapshot: Snapshot<AppTypeConfig>,
        cancel: impl Future<Output = ReplicationClosed> + OptionalSend + 'static,
        _option: RPCOption,
    ) -> Result<SnapshotResponse<AppTypeConfig>, StreamingError<AppTypeConfig>> {
        // Read snapshot data into bytes
        let mut snapshot_data = Vec::new();
        let mut snapshot_reader = snapshot.snapshot;
        snapshot_reader
            .read_to_end(&mut snapshot_data)
            .await
            .map_err(|err| {
                StreamingError::StorageError(StorageError::read_snapshot(
                    Some(snapshot.meta.signature()),
                    &err,
                ))
            })?;

        let request = RaftSnapshotRequest {
            vote,
            snapshot_meta: snapshot.meta,
            snapshot_data,
        };
        let protocol = RaftRpcProtocol::InstallSnapshot(request);

        // Send the RPC with cancellation support
        let response = select! {
            send_result = self.send_rpc(protocol) => {
                send_result.map_err(|err| {
                    warn!(target_node = %self.target, error = %err, "failed to send snapshot RPC");
                    let err_str = err.to_string();
                    StreamingError::Unreachable(Unreachable::new(&std::io::Error::new(
                        std::io::ErrorKind::Other,
                        err_str,
                    )))
                })?
            }
            closed = cancel => {
                warn!(target_node = %self.target, "snapshot transmission cancelled");
                return Err(StreamingError::Closed(closed));
            }
        };

        // Extract result from response
        match response {
            RaftRpcResponse::InstallSnapshot(result) => {
                // Handle remote RaftError as StorageError since snapshot installation failed
                result.map_err(|raft_err| {
                    StreamingError::StorageError(StorageError::read_snapshot(
                        None,
                        &raft_err,
                    ))
                })
            }
            _ => Err(StreamingError::Unreachable(Unreachable::new(&std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "unexpected response type for install_snapshot",
            )))),
        }
    }
}
