// SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use iroh::endpoint::presets;
use iroh::endpoint::Connection;
use iroh::endpoint::RecvStream;
use iroh::endpoint::SendStream;
use iroh::Endpoint;
use iroh::EndpointAddr;
use molten_batch_application::*;
use molten_batch_core::*;
use molten_orchestrator_codec as codec;
use tokio::sync::watch;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::protocol::*;

pub async fn bind_endpoint(key: iroh::SecretKey, bind: std::net::SocketAddr) -> Result<Endpoint, Error> {
    Endpoint::builder(presets::Minimal)
        .clear_ip_transports()
        .bind_addr(bind)
        .map_err(crate::invalid)?
        .secret_key(key)
        .alpns(vec![ALPN.to_vec()])
        .bind()
        .await
        .map_err(crate::invalid)
}
pub fn endpoint_id(endpoint: &Endpoint) -> Id {
    Id(*endpoint.id().as_bytes())
}
struct ConnectionGuard(Connection);
impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.0.close(0u32.into(), b"request finished");
    }
}
async fn send_frame(send: &mut SendStream, bytes: &[u8]) -> Result<(), Error> {
    if bytes.len() > MAX_FRAME_BYTES {
        return Err(Error::Bound);
    }
    send.write_all(&(bytes.len() as u64).to_be_bytes()).await.map_err(|_| Error::Unknown)?;
    send.write_all(bytes).await.map_err(|_| Error::Unknown)?;
    send.finish().map_err(|_| Error::Unknown)
}
async fn receive_frame(recv: &mut RecvStream) -> Result<Vec<u8>, Error> {
    let mut length = [0_u8; 8];
    recv.read_exact(&mut length).await.map_err(|_| Error::Unknown)?;
    let size = u64::from_be_bytes(length);
    if size == 0 || size > MAX_FRAME_BYTES as u64 {
        return Err(Error::Bound);
    }
    let bytes = recv.read_to_end(size as usize + 1).await.map_err(|_| Error::Unknown)?;
    if bytes.len() != size as usize {
        return Err(Error::Invalid("frame length/trailing bytes".into()));
    }
    Ok(bytes)
}
/// Exactly one RPC attempt. Caller owns idempotency/reconciliation; there is no
/// hidden reconnect-and-resubmit policy in this transport adapter.
pub async fn rpc(
    endpoint: &Endpoint,
    target: &PeerAddress,
    request: &Request,
    deadline: Duration,
) -> Result<Reply, Error> {
    let bytes = codec::frame(REQUEST_SCHEMA, request).map_err(crate::invalid)?;
    let remote: iroh::EndpointId = target.id.hex().parse().map_err(crate::invalid)?;
    let address = EndpointAddr::new(remote).with_ip_addr(target.address);
    let reply = tokio::time::timeout(deadline, async {
        let connection = endpoint.connect(address, ALPN).await.map_err(|_| Error::Unavailable)?;
        if Id(*connection.remote_id().as_bytes()) != target.id {
            return Err(Error::Denied);
        }
        let guard = ConnectionGuard(connection);
        let (mut send, mut recv) = guard.0.open_bi().await.map_err(|_| Error::Unknown)?;
        send_frame(&mut send, &bytes).await?;
        let bytes = receive_frame(&mut recv).await?;
        codec::unframe::<Reply>(REPLY_SCHEMA, &bytes).map_err(crate::invalid)
    })
    .await
    .map_err(|_| Error::Unknown)??;
    match reply {
        Reply::Error(code) => Err(match code.as_str() {
            "denied" => Error::Denied,
            "conflict" => Error::Conflict,
            "unknown-reconcile" => Error::Unknown,
            "unavailable" => Error::Unavailable,
            "bound-exceeded" => Error::Bound,
            "unsupported" => Error::Unsupported,
            _ => Error::Invalid("remote rejected request".into()),
        }),
        other => Ok(other),
    }
}
#[async_trait]
pub trait RequestHandler: Send + Sync {
    fn permits(&self, peer: Id) -> bool;
    async fn handle(&self, peer: Id, request: Request) -> Reply;
}
/// Bounded one-request-per-connection ingress. Peer admission precedes framing
/// and parsing. The key allowlist is configured separately from connectivity.
pub async fn serve(
    endpoint: Endpoint,
    handler: Arc<dyn RequestHandler>,
    deadline: Duration,
    mut stop: watch::Receiver<bool>,
) -> Result<(), Error> {
    let permits = Arc::new(Semaphore::new(8));
    let mut tasks = JoinSet::new();
    loop {
        tokio::select! {
            changed=stop.changed()=>{if changed.is_err()||*stop.borrow(){break;}},
            _=tasks.join_next(),if !tasks.is_empty()=>{},
            incoming=endpoint.accept()=>{
                let Some(incoming)=incoming else{break;};
                let Ok(permit)=permits.clone().try_acquire_owned() else{drop(incoming);continue;};
                let handler=handler.clone();
                tasks.spawn(async move {
                    let _permit=permit;
                    let result=tokio::time::timeout(deadline,async {
                        let connection=incoming.await.map_err(|_|Error::Unavailable)?;
                        let peer=Id(*connection.remote_id().as_bytes());
                        if !handler.permits(peer){connection.close(1u32.into(),b"denied");return Err(Error::Denied);}
                        let guard=ConnectionGuard(connection);
                        let (mut send,mut recv)=guard.0.accept_bi().await.map_err(|_|Error::Unknown)?;
                        let raw=receive_frame(&mut recv).await?;
                        let request=codec::unframe::<Request>(REQUEST_SCHEMA,&raw).map_err(crate::invalid)?;
                        let reply=handler.handle(peer,request).await;
                        send_frame(&mut send,&codec::frame(REPLY_SCHEMA,&reply).map_err(crate::invalid)?).await?;
                        // Do not close before the caller can acknowledge receipt.
                        guard.0.closed().await;Ok::<(),Error>(())
                    }).await;
                    let _=result; // Per-request failure never fabricates semantic success.
                });
            }
        }
    }
    endpoint.close().await;
    while tasks.join_next().await.is_some() {}
    Ok(())
}
pub struct IrohWorkers {
    endpoint: Endpoint,
    peers: BTreeMap<Id, PeerAddress>,
    deadline: Duration,
}
impl IrohWorkers {
    pub fn new(endpoint: Endpoint, peers: Vec<PeerAddress>, deadline: Duration) -> Result<Self, Error> {
        let mut map = BTreeMap::new();
        for p in peers {
            if map.insert(p.id, p).is_some() {
                return Err(Error::Invalid("duplicate peer".into()));
            }
        }
        if map.len() > MAX_NODES {
            return Err(Error::Bound);
        }
        Ok(Self {
            endpoint,
            peers: map,
            deadline,
        })
    }
    async fn call(&self, node: Id, request: Request) -> Result<Reply, Error> {
        let peer = self.peers.get(&node).ok_or(Error::Denied)?;
        rpc(&self.endpoint, peer, &request, self.deadline).await
    }
    async fn observation(&self, r: &ExecutionRequest, request: Request) -> Result<WorkerObservation, Error> {
        match self.call(r.fence.node, request).await? {
            Reply::Observation(o) => Ok(o.into()),
            _ => Err(Error::Invalid("observation reply shape".into())),
        }
    }
}
#[async_trait]
impl WorkerClient for IrohWorkers {
    async fn info(&self, node: Id) -> Result<WorkerInfo, Error> {
        match self.call(node, Request::Info).await? {
            Reply::Info(i) => Ok(i.into()),
            _ => Err(Error::Invalid("info reply".into())),
        }
    }
    async fn put(&self, node: Id, id: Id, c: Content) -> Result<(), Error> {
        match self
            .call(node, Request::Put {
                id,
                content: WireContent::from_content(c)?,
            })
            .await?
        {
            Reply::Id(got) if got == id => Ok(()),
            _ => Err(Error::Invalid("put reply".into())),
        }
    }
    async fn get(&self, node: Id, id: Id) -> Result<Content, Error> {
        match self.call(node, Request::Get { id }).await? {
            Reply::Content(c) => {
                let c = c.content()?;
                if crate::content_id(&c)? != id {
                    return Err(Error::Invalid("get reply identity".into()));
                }
                Ok(c)
            }
            _ => Err(Error::Invalid("get reply".into())),
        }
    }
    async fn start(&self, r: ExecutionRequest) -> Result<WorkerObservation, Error> {
        self.observation(&r, Request::Start { request: r.clone() }).await
    }
    async fn observe(&self, r: &ExecutionRequest) -> Result<WorkerObservation, Error> {
        self.observation(r, Request::Observe { request: r.clone() }).await
    }
    async fn cancel(&self, r: &ExecutionRequest) -> Result<WorkerObservation, Error> {
        self.observation(r, Request::CancelAttempt { request: r.clone() }).await
    }
}
