//! Trust share-collection protocol handler.
//!
//! Serves `GetShare` requests over the trust ALPN during membership-driven
//! secret rotation.

use std::sync::Arc;

use anyhow::Context;
use aspen_trust::protocol::ShareResponse;
use aspen_trust::protocol::TrustRequest;
use aspen_trust::protocol::TrustResponse;
use async_trait::async_trait;
use iroh::EndpointId;
use iroh::endpoint::Connection;
use iroh::protocol::AcceptError;
use iroh::protocol::ProtocolHandler;
use tokio_util::task::TaskTracker;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::ConnectionManager;
use crate::StreamManager;
use crate::handle_connection_streams;

const MAX_TRUST_CONNECTIONS: u32 = 64;
const MAX_TRUST_STREAMS_PER_CONNECTION: u32 = 4;
const MAX_TRUST_MESSAGE_SIZE: usize = 4096;

/// Source of trust shares for incoming `GetShare` requests.
#[async_trait]
pub trait TrustShareProvider: Send + Sync {
    /// Return the local share for `epoch` if `requester` is authorized.
    async fn get_share(&self, requester: EndpointId, epoch: u64) -> anyhow::Result<Option<ShareResponse>>;
}

/// Protocol handler for trust share collection over Iroh QUIC.
pub struct TrustProtocolHandler {
    provider: Arc<dyn TrustShareProvider>,
    connections: ConnectionManager,
}

impl std::fmt::Debug for TrustProtocolHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrustProtocolHandler")
            .field("max_connections", &self.connections.max_connections())
            .field("max_streams_per_connection", &self.connections.max_streams_per_connection())
            .finish_non_exhaustive()
    }
}

impl TrustProtocolHandler {
    /// Create a new trust protocol handler.
    pub fn new(provider: Arc<dyn TrustShareProvider>) -> Self {
        Self {
            provider,
            connections: ConnectionManager::new(MAX_TRUST_CONNECTIONS, MAX_TRUST_STREAMS_PER_CONNECTION),
        }
    }
}

impl ProtocolHandler for TrustProtocolHandler {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let remote = connection.remote_id();
        let Some(_permit) = self.connections.try_acquire_connection() else {
            warn!(remote = %remote, max = self.connections.max_connections(), "trust connection limit reached");
            return Err(AcceptError::from_err(std::io::Error::other("trust connection limit reached")));
        };

        debug!(remote = %remote, "accepted trust protocol connection");
        let tracker = TaskTracker::new();
        let provider = self.provider.clone();
        let result = handle_connection_streams(
            connection,
            StreamManager::new(self.connections.max_streams_per_connection()),
            &tracker,
            move |recv, send| {
                let provider = provider.clone();
                async move { handle_trust_stream(remote, recv, send, provider).await }
            },
        )
        .await;
        tracker.close();
        tracker.wait().await;
        result.map_err(|err| AcceptError::from_err(std::io::Error::other(err.to_string())))
    }

    async fn shutdown(&self) {
        info!("trust protocol handler shutting down");
        self.connections.shutdown();
    }
}

async fn handle_trust_stream(
    remote: EndpointId,
    mut recv: iroh::endpoint::RecvStream,
    mut send: iroh::endpoint::SendStream,
    provider: Arc<dyn TrustShareProvider>,
) -> anyhow::Result<()> {
    let buffer = recv.read_to_end(MAX_TRUST_MESSAGE_SIZE).await.context("failed to read trust request")?;
    let request: TrustRequest = postcard::from_bytes(&buffer).context("failed to deserialize trust request")?;

    match request {
        TrustRequest::GetShare(request) => {
            let Some(response) = provider.get_share(remote, request.epoch).await? else {
                warn!(remote = %remote, epoch = request.epoch, "trust share request rejected");
                anyhow::bail!("trust share request rejected");
            };
            let bytes =
                postcard::to_stdvec(&TrustResponse::Share(response)).context("failed to serialize trust response")?;
            send.write_all(&bytes).await.context("failed to write trust response")?;
            send.finish().context("failed to finish trust response")?;
            Ok(())
        }
    }
}
