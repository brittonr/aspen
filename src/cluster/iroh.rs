//! Iroh-backed transport for `NodeServer`.
//!
//! This module wires [`ractor_cluster::NodeServer`] to [`iroh`] so actors can
//! communicate over QUIC/endpoint IDs instead of raw TCP sockets. It uses the
//! “bring your own transport” hook offered by [`NodeServerMessage`] plus the
//! [`ClusterBidiStream`] trait.

use std::fmt;
use std::sync::Arc;

use anyhow::Context;
use iroh::endpoint::{Connection, Endpoint};
use iroh::protocol::{AcceptError, ProtocolHandler, Router};
use iroh::{EndpointAddr, EndpointId, RelayMode, SecretKey};
use ractor::ActorRef;
use ractor_cluster::client_connect_external;
use ractor_cluster::{ClusterBidiStream, NodeServerMessage};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::OnceCell;
use tracing::info;

use crate::cluster::NodeServerHandle;

const ASPEN_CLUSTER_ALPN: &[u8] = b"/aspen/cluster/1";

/// Configuration for the Iroh transport.
#[derive(Clone, Debug)]
pub struct IrohClusterConfig {
    /// Optional static secret key (otherwise a new key is generated).
    pub secret_key: Option<SecretKey>,
    /// Relay configuration (defaults to disabled).
    pub relay_mode: RelayMode,
}

impl Default for IrohClusterConfig {
    fn default() -> Self {
        Self {
            secret_key: None,
            relay_mode: RelayMode::Disabled,
        }
    }
}

/// Handle that keeps the Iroh endpoint and router alive.
pub struct IrohClusterTransport {
    router: Router,
    node_server: ActorRef<NodeServerMessage>,
    shutdown_once: OnceCell<()>,
}

impl fmt::Debug for IrohClusterTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IrohClusterTransport")
            .field("endpoint_id", &self.endpoint_id())
            .finish()
    }
}

impl IrohClusterTransport {
    /// Start a router that accepts `/aspen/cluster/1` QUIC streams and forwards them to the node server.
    pub async fn spawn(
        node_server: &NodeServerHandle,
        config: IrohClusterConfig,
    ) -> anyhow::Result<Self> {
        let mut builder = Endpoint::builder().relay_mode(config.relay_mode);
        if let Some(secret) = config.secret_key {
            builder = builder.secret_key(secret);
        }
        let endpoint = builder.bind().await.context("bind iroh endpoint")?;
        let handler = ClusterProtocolHandler {
            node_server: node_server.actor().clone(),
        };
        let router = Router::builder(endpoint)
            .accept(ASPEN_CLUSTER_ALPN, Arc::new(handler))
            .spawn();
        Ok(Self {
            router,
            node_server: node_server.actor().clone(),
            shutdown_once: OnceCell::new(),
        })
    }

    /// Local endpoint identifier.
    pub fn endpoint_id(&self) -> EndpointId {
        self.router.endpoint().id()
    }

    /// Local endpoint address (contains the ID plus relay hints).
    pub fn endpoint_addr(&self) -> EndpointAddr {
        self.router.endpoint().addr()
    }

    /// Connect to a peer endpoint and attach the resulting stream to the node server.
    pub async fn connect(&self, peer: EndpointAddr) -> anyhow::Result<()> {
        let connection = self
            .router
            .endpoint()
            .connect(peer.clone(), ASPEN_CLUSTER_ALPN)
            .await
            .with_context(|| format!("connect to iroh peer {peer:?}"))?;
        let (send, recv) = connection
            .open_bi()
            .await
            .context("open cluster stream over iroh")?;
        let stream = IrohClusterStream::new(connection.remote_id(), send, recv);
        client_connect_external(
            &self.node_server,
            Box::new(stream) as Box<dyn ClusterBidiStream>,
        )
        .await
        .context("attach iroh transport to NodeServer")?;
        Ok(())
    }

    /// Underlying `iroh::Endpoint` for advanced callers.
    pub fn endpoint(&self) -> &Endpoint {
        self.router.endpoint()
    }

    /// Wait until the endpoint reports being online (relay registered, addresses known).
    pub async fn wait_until_online(&self) {
        self.router.endpoint().online().await;
    }

    /// Shut down the router and endpoint.
    pub async fn shutdown(self) -> anyhow::Result<()> {
        if self.shutdown_once.get().is_none() {
            self.router
                .shutdown()
                .await
                .context("shutdown iroh router")?;
            let _ = self.shutdown_once.set(());
        }
        Ok(())
    }
}

#[derive(Clone)]
struct ClusterProtocolHandler {
    node_server: ActorRef<NodeServerMessage>,
}

impl fmt::Debug for ClusterProtocolHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClusterProtocolHandler").finish()
    }
}

impl ProtocolHandler for ClusterProtocolHandler {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let remote = connection.remote_id();
        let (send, recv) = connection.accept_bi().await?;
        let stream = IrohClusterStream::new(remote, send, recv);
        self.node_server
            .cast(NodeServerMessage::ConnectionOpenedExternal {
                stream: Box::new(stream) as Box<dyn ClusterBidiStream>,
                is_server: true,
            })
            .map_err(AcceptError::from_err)?;
        info!(remote = %remote, "accepted iroh cluster stream");
        Ok(())
    }

    async fn shutdown(&self) {
        info!("iroh cluster handler shutting down");
    }
}

struct IrohClusterStream {
    peer_label: String,
    recv: iroh::endpoint::RecvStream,
    send: iroh::endpoint::SendStream,
}

impl IrohClusterStream {
    fn new(
        peer_id: EndpointId,
        send: iroh::endpoint::SendStream,
        recv: iroh::endpoint::RecvStream,
    ) -> Self {
        Self {
            peer_label: format!("iroh:{peer_id}"),
            recv,
            send,
        }
    }
}

impl ClusterBidiStream for IrohClusterStream {
    fn split(self: Box<Self>) -> (ractor_cluster::BoxRead, ractor_cluster::BoxWrite) {
        (
            Box::new(IrohRead(self.recv)) as ractor_cluster::BoxRead,
            Box::new(IrohWrite(self.send)) as ractor_cluster::BoxWrite,
        )
    }

    fn peer_label(&self) -> Option<String> {
        Some(self.peer_label.clone())
    }

    fn local_label(&self) -> Option<String> {
        Some("iroh:local".to_string())
    }
}

struct IrohRead(iroh::endpoint::RecvStream);
struct IrohWrite(iroh::endpoint::SendStream);

impl AsyncRead for IrohRead {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match std::pin::Pin::new(&mut self.0).poll_read(cx, buf) {
            std::task::Poll::Ready(Ok(())) => std::task::Poll::Ready(Ok(())),
            std::task::Poll::Ready(Err(err)) => {
                std::task::Poll::Ready(Err(std::io::Error::new(std::io::ErrorKind::Other, err)))
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

impl AsyncWrite for IrohWrite {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        match std::pin::Pin::new(&mut self.0).poll_write(cx, buf) {
            std::task::Poll::Ready(Ok(len)) => std::task::Poll::Ready(Ok(len)),
            std::task::Poll::Ready(Err(err)) => {
                std::task::Poll::Ready(Err(std::io::Error::new(std::io::ErrorKind::Other, err)))
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match std::pin::Pin::new(&mut self.0).poll_flush(cx) {
            std::task::Poll::Ready(Ok(())) => std::task::Poll::Ready(Ok(())),
            std::task::Poll::Ready(Err(err)) => {
                std::task::Poll::Ready(Err(std::io::Error::new(std::io::ErrorKind::Other, err)))
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match std::pin::Pin::new(&mut self.0).poll_shutdown(cx) {
            std::task::Poll::Ready(Ok(())) => std::task::Poll::Ready(Ok(())),
            std::task::Poll::Ready(Err(err)) => {
                std::task::Poll::Ready(Err(std::io::Error::new(std::io::ErrorKind::Other, err)))
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cluster::NodeServerConfig;
    use tokio::time::{Duration, sleep};

    #[tokio::test]
    async fn connect_two_nodes_over_iroh() {
        tracing_subscriber::fmt::try_init().ok();
        let cfg_a = NodeServerConfig::new("node-a", "127.0.0.1", 0, "cookie-a");
        let cfg_b = NodeServerConfig::new("node-b", "127.0.0.1", 0, "cookie-a");
        let node_a = cfg_a.launch().await.expect("launch node a");
        let node_b = cfg_b.launch().await.expect("launch node b");

        let iroh_a = IrohClusterTransport::spawn(&node_a, IrohClusterConfig::default())
            .await
            .unwrap();
        let iroh_b = IrohClusterTransport::spawn(&node_b, IrohClusterConfig::default())
            .await
            .unwrap();

        iroh_a
            .connect(iroh_b.endpoint_addr())
            .await
            .expect("connect over iroh");

        // Give NodeServer time to finish session wiring.
        sleep(Duration::from_millis(200)).await;

        iroh_a.shutdown().await.unwrap();
        iroh_b.shutdown().await.unwrap();
        node_a.shutdown().await.unwrap();
        node_b.shutdown().await.unwrap();
    }
}
