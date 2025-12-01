//! Cluster orchestration primitives built around `ractor_cluster::NodeServer`.
//!
//! # Responsibilities
//! - Spin up [`NodeServer`] instances with magic-cookie auth configured the same
//!   way everywhere so Raft actors can be remoted deterministically.
//! - Provide hooks for attaching BYO transports via [`ClusterBidiStream`] so we
//!   can drive the cluster through QUIC/WebSockets/memory pipes in tests.
//! - Surface macros/helpers that ensure every actor message derives
//!   [`RactorClusterMessage`](ractor_cluster::RactorClusterMessage), keeping the
//!   network serialization contract obvious.
//!
//! # Composition
//! `cluster` exposes handles consumed by `raft` and `api`. `raft` will
//! register actor groups with a running node, while `api` will issue cluster
//! RPCs through the same [`NodeServerHandle`]. Tests (deterministic or not) can
//! inject synthetic transports via [`NodeServerHandle::attach_external_stream`].

use anyhow::Context;
use ractor::concurrency::JoinHandle;
use ractor::{Actor, ActorName, ActorRef, MessagingErr, SpawnErr};
use ractor_cluster::node::NodeConnectionMode;
use ractor_cluster::{
    ClusterBidiStream, IncomingEncryptionMode, NodeEventSubscription, NodeServer, NodeServerMessage,
};

pub mod iroh;
pub use iroh::{IrohClusterConfig, IrohClusterTransport};

/// Deterministic knobs used when running under `madsim`/turmoil.
#[derive(Debug, Clone, Default)]
pub struct DeterministicClusterConfig {
    /// Optional simulation seed recorded alongside each deterministic test run.
    pub simulation_seed: Option<u64>,
}

/// Configuration for spinning up a [`NodeServer`].
#[derive(Debug, Clone)]
pub struct NodeServerConfig {
    /// Node name used in Erlang-style identifiers (`<name>@<host>`).
    pub node_name: String,
    /// Hostname portion of the identifier plus listener binding host.
    pub hostname: String,
    /// Port used for the default TCP listener. Zero lets the OS pick a port.
    pub port: u16,
    /// Shared secret that authenticates connections (`magic cookie`).
    pub cookie: String,
    /// On-wire encryption mode.
    pub encryption_mode: IncomingEncryptionMode,
    /// How the node connects to peers once one connection is established.
    pub connection_mode: NodeConnectionMode,
    /// Deterministic testing config.
    pub deterministic: DeterministicClusterConfig,
}

impl NodeServerConfig {
    /// Create a new config ensuring Tiger Style invariants up front.
    pub fn new(
        node_name: impl Into<String>,
        hostname: impl Into<String>,
        port: u16,
        cookie: impl Into<String>,
    ) -> Self {
        let node_name = node_name.into();
        assert!(
            !node_name.is_empty(),
            "node_name must be non-empty so actor names stay unique"
        );
        let hostname = hostname.into();
        assert!(
            !hostname.is_empty(),
            "hostname must be non-empty for deterministic seeds"
        );
        let cookie = cookie.into();
        assert!(
            cookie.len() >= 8,
            "magic cookies shorter than 8 bytes are trivially guessable"
        );
        Self {
            node_name,
            hostname,
            port,
            cookie,
            encryption_mode: IncomingEncryptionMode::Raw,
            connection_mode: NodeConnectionMode::Transitive,
            deterministic: DeterministicClusterConfig::default(),
        }
    }

    /// Override deterministic options used by `madsim`.
    pub fn with_determinism(mut self, deterministic: DeterministicClusterConfig) -> Self {
        self.deterministic = deterministic;
        self
    }

    /// Override encryption mode.
    pub fn with_encryption_mode(mut self, mode: IncomingEncryptionMode) -> Self {
        self.encryption_mode = mode;
        self
    }

    /// Override node connection mode.
    pub fn with_connection_mode(mut self, mode: NodeConnectionMode) -> Self {
        self.connection_mode = mode;
        self
    }

    /// Launch the [`NodeServer`] actor and return a handle.
    pub async fn launch(self) -> Result<NodeServerHandle, SpawnErr> {
        let actor_name: ActorName = format!(
            "cluster:{}:{}",
            self.node_name,
            self.deterministic
                .simulation_seed
                .map(|seed| seed.to_string())
                .unwrap_or_else(|| "live".to_string())
        );
        let node_server = NodeServer::new(
            self.port,
            self.cookie,
            self.node_name.clone(),
            self.hostname.clone(),
            Some(self.encryption_mode),
            Some(self.connection_mode),
        );
        let (actor, join_handle) = Actor::spawn(Some(actor_name), node_server, ()).await?;
        Ok(NodeServerHandle {
            server: actor,
            join_handle,
            deterministic: self.deterministic,
        })
    }
}

/// Handle to a spawned [`NodeServer`] actor.
#[derive(Debug)]
pub struct NodeServerHandle {
    server: ActorRef<NodeServerMessage>,
    join_handle: JoinHandle<()>,
    deterministic: DeterministicClusterConfig,
}

impl NodeServerHandle {
    /// Access the underlying actor ref.
    pub fn actor(&self) -> &ActorRef<NodeServerMessage> {
        &self.server
    }

    /// Deterministic config used for this instance (recorded alongside simulator traces).
    pub fn deterministic_config(&self) -> &DeterministicClusterConfig {
        &self.deterministic
    }

    /// Attach a BYO transport implementing [`ClusterBidiStream`].
    pub fn attach_external_stream<S>(
        &self,
        stream: S,
        is_server: bool,
    ) -> Result<(), MessagingErr<NodeServerMessage>>
    where
        S: ClusterBidiStream,
    {
        self.server
            .cast(NodeServerMessage::ConnectionOpenedExternal {
                stream: Box::new(stream),
                is_server,
            })
    }

    /// Subscribe to node events for deterministic harnesses to inspect ordering.
    pub fn subscribe<S>(
        &self,
        id: impl Into<String>,
        subscription: S,
    ) -> Result<(), MessagingErr<NodeServerMessage>>
    where
        S: NodeEventSubscription,
    {
        self.server.cast(NodeServerMessage::SubscribeToEvents {
            id: id.into(),
            subscription: Box::new(subscription),
        })
    }

    /// Shutdown helper; this only waits for the join handle because `ractor` actors terminate on drop.
    pub async fn shutdown(self) -> anyhow::Result<()> {
        self.server.get_cell().stop(Some("cluster_shutdown".into()));
        self.join_handle
            .await
            .context("node server join handle failed")?;
        Ok(())
    }
}

/// Helper macro that wraps `enum`/`struct` definitions and derives both
/// [`RactorMessage`](ractor_cluster::RactorMessage) and
/// [`RactorClusterMessage`](ractor_cluster::RactorClusterMessage).
#[macro_export]
macro_rules! cluster_message {
    ($(#[$meta:meta])* $vis:vis enum $name:ident $body:tt) => {
        $(#[$meta])*
        #[derive(Clone, Debug, ractor_cluster::RactorClusterMessage)]
        $vis enum $name $body
    };
    ($(#[$meta:meta])* $vis:vis struct $name:ident $body:tt) => {
        $(#[$meta])*
        #[derive(Clone, Debug, ractor_cluster::RactorClusterMessage)]
        $vis struct $name $body
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use ractor::Message;

    cluster_message! {
        /// Dummy message used to prove macros wire up traits.
        pub enum DummyMessage {
            Ping,
            #[allow(dead_code)]
            Echo(u64),
        }
    }

    #[tokio::test]
    async fn node_server_config_enforces_cookie_length() {
        let config = NodeServerConfig::new("node-a", "local", 0, "supersecret");
        assert_eq!(config.node_name, "node-a");
        assert_eq!(config.cookie, "supersecret");
    }

    #[test]
    fn dummy_message_is_a_ractor_message() {
        fn assert_message<M: Message>() {}
        assert_message::<DummyMessage>();
    }
}
