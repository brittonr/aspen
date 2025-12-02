use std::fmt::Display;

use openraft::BasicNode;
use openraft::RaftTypeConfig;
use openraft::error::{
    Infallible, InstallSnapshotError, NetworkError, RPCError, RaftError, RemoteError, Unreachable,
};
use openraft::network::{RPCOption, RaftNetwork, RaftNetworkFactory};
use openraft::raft::{
    AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest, InstallSnapshotResponse,
    VoteRequest, VoteResponse,
};
use reqwest::Client;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::io::{AsyncRead, AsyncSeek, AsyncWrite};

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
