use std::fmt;
use std::mem;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as SyncMutex};
use std::time::Duration;

use anyhow::{Context, Result};
use ractor::{Actor, ActorRef, MessagingErr};
use ractor_cluster::node::NodeConnectionMode;
use ractor_cluster::{
    ClientConnectErr, ClusterBidiStream, IncomingEncryptionMode, NodeEventSubscription,
    NodeServer as RactorNodeServer, NodeServerMessage, client_connect, client_connect_external,
};
use tokio::net::ToSocketAddrs;
use tokio::sync::{Mutex as AsyncMutex, Notify};
use tokio::task::JoinHandle;

/// Controls how the node server should behave while running in deterministic
/// simulations.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeterministicClusterConfig {
    pub simulation_seed: Option<u64>,
}

/// Configuration for the local `ractor_cluster::NodeServer`.
#[derive(Debug, Clone)]
pub struct NodeServerConfig {
    label: String,
    host: String,
    port: u16,
    cookie: String,
    encryption: Option<IncomingEncryptionMode>,
    connection_mode: NodeConnectionMode,
    determinism: Option<DeterministicClusterConfig>,
}

impl NodeServerConfig {
    pub fn new(
        label: impl Into<String>,
        host: impl Into<String>,
        port: u16,
        cookie: impl Into<String>,
    ) -> Self {
        Self {
            label: label.into(),
            host: host.into(),
            port,
            cookie: cookie.into(),
            encryption: None,
            connection_mode: NodeConnectionMode::Transitive,
            determinism: None,
        }
    }

    pub fn with_encryption(mut self, encryption: IncomingEncryptionMode) -> Self {
        self.encryption = Some(encryption);
        self
    }

    pub fn with_connection_mode(mut self, mode: NodeConnectionMode) -> Self {
        self.connection_mode = mode;
        self
    }

    pub fn with_determinism(mut self, config: DeterministicClusterConfig) -> Self {
        self.determinism = Some(config);
        self
    }

    pub async fn launch(self) -> Result<NodeServerHandle> {
        let server = RactorNodeServer::new(
            self.port,
            self.cookie.clone(),
            self.label.clone(),
            self.host.clone(),
            self.encryption.clone(),
            Some(self.connection_mode),
        );
        let actor_name = format!("node-server-{}", self.label);
        let (actor_ref, join_handle) = Actor::spawn(Some(actor_name.into()), server, ())
            .await
            .context("failed to spawn node server")?;

        Ok(NodeServerHandle {
            inner: Arc::new(NodeServerInner {
                actor: actor_ref,
                join_handle: AsyncMutex::new(Some(join_handle)),
                label: self.label,
                host: self.host,
                port: self.port,
                cookie: self.cookie,
                connection_mode: self.connection_mode,
                determinism: self.determinism,
                subscriptions: SyncMutex::new(Vec::new()),
            }),
        })
    }
}

struct NodeServerInner {
    actor: ActorRef<NodeServerMessage>,
    join_handle: AsyncMutex<Option<JoinHandle<()>>>,
    label: String,
    host: String,
    port: u16,
    cookie: String,
    connection_mode: NodeConnectionMode,
    determinism: Option<DeterministicClusterConfig>,
    subscriptions: SyncMutex<Vec<String>>,
}

impl fmt::Debug for NodeServerInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NodeServerInner")
            .field("label", &self.label)
            .field("host", &self.host)
            .field("port", &self.port)
            .field("cookie", &"<hidden>")
            .field("connection_mode", &self.connection_mode)
            .finish()
    }
}

/// Handle for a launched node server.
pub struct NodeServerHandle {
    inner: Arc<NodeServerInner>,
}

impl Clone for NodeServerHandle {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl fmt::Debug for NodeServerHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NodeServerHandle")
            .field("label", &self.inner.label)
            .field("addr", &self.addr())
            .finish()
    }
}

impl NodeServerHandle {
    pub fn label(&self) -> &str {
        &self.inner.label
    }

    pub fn cookie(&self) -> &str {
        &self.inner.cookie
    }

    pub fn determinism(&self) -> Option<&DeterministicClusterConfig> {
        self.inner.determinism.as_ref()
    }

    pub fn addr(&self) -> SocketAddr {
        let ip: IpAddr = self
            .inner
            .host
            .parse()
            .unwrap_or_else(|_| IpAddr::V4(Ipv4Addr::UNSPECIFIED));
        SocketAddr::new(ip, self.inner.port)
    }

    pub fn actor(&self) -> ActorRef<NodeServerMessage> {
        self.inner.actor.clone()
    }

    pub fn subscribe(
        &self,
        id: impl Into<String>,
        subscription: Box<dyn NodeEventSubscription>,
    ) -> Result<()> {
        let sub_id = id.into();
        self.inner
            .actor
            .cast(NodeServerMessage::SubscribeToEvents {
                id: sub_id.clone(),
                subscription,
            })?;
        self.inner
            .subscriptions
            .lock()
            .expect("subscriptions poisoned")
            .push(sub_id);
        Ok(())
    }

    pub fn unsubscribe(&self, id: &str) -> Result<()> {
        self.inner
            .actor
            .cast(NodeServerMessage::UnsubscribeToEvents(id.to_string()))?;
        let mut guard = self
            .inner
            .subscriptions
            .lock()
            .expect("subscriptions poisoned");
        if let Some(pos) = guard.iter().position(|existing| existing == id) {
            guard.remove(pos);
        }
        Ok(())
    }

    pub async fn client_connect<T>(&self, address: T) -> Result<(), ClientConnectErr>
    where
        T: ToSocketAddrs,
    {
        client_connect(&self.inner.actor, address).await
    }

    pub async fn client_connect_external(
        &self,
        stream: Box<dyn ClusterBidiStream>,
    ) -> Result<(), ClientConnectErr> {
        client_connect_external(&self.inner.actor, stream).await
    }

    pub fn attach_external_stream(
        &self,
        stream: Box<dyn ClusterBidiStream>,
        is_server: bool,
    ) -> Result<(), MessagingErr<NodeServerMessage>> {
        self.inner
            .actor
            .cast(NodeServerMessage::ConnectionOpenedExternal { stream, is_server })
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.inner.actor.stop(Some("node-server-shutdown".into()));
        if let Some(join) = self.inner.join_handle.lock().await.take() {
            join.await
                .context("node server task aborted while shutting down")?;
        }

        // best-effort removal of outstanding subscriptions
        let subs = {
            let mut guard = self
                .inner
                .subscriptions
                .lock()
                .expect("subscriptions poisoned");
            mem::take(&mut *guard)
        };
        for id in subs {
            let _ = self
                .inner
                .actor
                .cast(NodeServerMessage::UnsubscribeToEvents(id));
        }

        Ok(())
    }
}

/// Control surface for the fake Iroh-backed transport.
#[derive(Debug, Clone)]
pub struct IrohClusterConfig {
    pub metrics_label: Option<String>,
    pub startup_delay: Duration,
}

impl Default for IrohClusterConfig {
    fn default() -> Self {
        Self {
            metrics_label: None,
            startup_delay: Duration::from_millis(25),
        }
    }
}

struct IrohEndpointInner {
    metrics_snapshot: SyncMutex<String>,
}

/// Minimal endpoint handle that exposes a metrics snapshot.
#[derive(Clone)]
pub struct IrohEndpoint {
    label: String,
    inner: Arc<IrohEndpointInner>,
}

impl IrohEndpoint {
    fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            inner: Arc::new(IrohEndpointInner {
                metrics_snapshot: SyncMutex::new(String::new()),
            }),
        }
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn metrics_snapshot(&self) -> String {
        self.inner
            .metrics_snapshot
            .lock()
            .expect("poisoned metrics")
            .clone()
    }

    fn update_metrics(&self, snapshot: String) {
        let mut guard = self
            .inner
            .metrics_snapshot
            .lock()
            .expect("poisoned metrics");
        *guard = snapshot;
    }
}

/// Simulated transport that keeps enough API surface alive for the tests.
pub struct IrohClusterTransport {
    endpoint: IrohEndpoint,
    online: Arc<Notify>,
    is_shutdown: Arc<AtomicBool>,
}

impl IrohClusterTransport {
    pub async fn spawn(server: &NodeServerHandle, config: IrohClusterConfig) -> Result<Self> {
        let endpoint = IrohEndpoint::new(
            config
                .metrics_label
                .clone()
                .unwrap_or_else(|| server.label().to_string()),
        );
        endpoint.update_metrics(format!(
            "# TYPE magicsock_recv_datagrams counter\nmagicsock_recv_datagrams{{node=\"{}\"}} {}\n# EOF\n",
            endpoint.label(),
            server
                .determinism()
                .and_then(|d| d.simulation_seed)
                .unwrap_or_default()
        ));
        let notify = Arc::new(Notify::new());
        let notify_clone = notify.clone();
        tokio::spawn(async move {
            tokio::time::sleep(config.startup_delay).await;
            notify_clone.notify_waiters();
        });
        Ok(Self {
            endpoint,
            online: notify,
            is_shutdown: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn endpoint(&self) -> IrohEndpoint {
        self.endpoint.clone()
    }

    pub async fn wait_until_online(&self) {
        self.online.notified().await;
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.is_shutdown.store(true, Ordering::SeqCst);
        Ok(())
    }
}
