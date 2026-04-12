//! Trust share collection client over Iroh QUIC.
//!
//! Used by the Raft leader to request old-epoch shares from current voters
//! before proposing a trust reconfiguration entry.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use aspen_transport::TRUST_ALPN;
use aspen_trust::protocol::GetShareRequest;
use aspen_trust::protocol::ShareResponse;
use aspen_trust::protocol::TrustRequest;
use aspen_trust::protocol::TrustResponse;
use async_trait::async_trait;

const TRUST_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const TRUST_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_TRUST_MESSAGE_SIZE: usize = 4096;

/// Error returned when a trust share request reveals this node has been expunged.
#[derive(Debug, Clone)]
pub struct ExpungedByPeer {
    /// The epoch at which this node was removed.
    pub epoch: u64,
}

impl std::fmt::Display for ExpungedByPeer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "this node has been expunged from the cluster at epoch {}", self.epoch)
    }
}

impl std::error::Error for ExpungedByPeer {}

/// Client used by the trust reconfiguration coordinator to fetch shares.
#[async_trait]
pub trait TrustShareClient: Send + Sync {
    /// Request a share for `epoch` from the node listening at `target`.
    async fn get_share(&self, target: iroh::EndpointAddr, epoch: u64) -> anyhow::Result<ShareResponse>;

    /// Send an expungement notification to `target` telling it that it was
    /// removed at the given `epoch`. Fire-and-forget: best effort, errors logged.
    async fn send_expunged(&self, target: iroh::EndpointAddr, epoch: u64) -> anyhow::Result<()>;
}

/// Iroh QUIC implementation of `TrustShareClient`.
#[derive(Debug)]
pub struct IrohTrustShareClient {
    endpoint: Arc<iroh::Endpoint>,
}

impl IrohTrustShareClient {
    /// Create a new Iroh-backed trust share client.
    pub fn new(endpoint: Arc<iroh::Endpoint>) -> Self {
        Self { endpoint }
    }
}

#[async_trait]
impl TrustShareClient for IrohTrustShareClient {
    async fn send_expunged(&self, target: iroh::EndpointAddr, epoch: u64) -> anyhow::Result<()> {
        let connection = tokio::time::timeout(TRUST_CONNECT_TIMEOUT, self.endpoint.connect(target, TRUST_ALPN))
            .await
            .context("trust connect timeout")?
            .context("failed to connect to trust endpoint")?;

        let (mut send, _recv) = tokio::time::timeout(TRUST_REQUEST_TIMEOUT, connection.open_bi())
            .await
            .context("trust stream open timeout")?
            .context("failed to open trust stream")?;

        let request = TrustRequest::Expunged { epoch };
        let request_bytes = postcard::to_stdvec(&request).context("failed to serialize expunged notification")?;
        send.write_all(&request_bytes).await.context("failed to write expunged notification")?;
        send.finish().context("failed to finish expunged notification")?;
        Ok(())
    }

    async fn get_share(&self, target: iroh::EndpointAddr, epoch: u64) -> anyhow::Result<ShareResponse> {
        let connection = tokio::time::timeout(TRUST_CONNECT_TIMEOUT, self.endpoint.connect(target, TRUST_ALPN))
            .await
            .context("trust connect timeout")?
            .context("failed to connect to trust endpoint")?;

        let (mut send, mut recv) = tokio::time::timeout(TRUST_REQUEST_TIMEOUT, connection.open_bi())
            .await
            .context("trust stream open timeout")?
            .context("failed to open trust stream")?;

        let request = TrustRequest::GetShare(GetShareRequest { epoch });
        let request_bytes = postcard::to_stdvec(&request).context("failed to serialize trust request")?;
        send.write_all(&request_bytes).await.context("failed to write trust request")?;
        send.finish().context("failed to finish trust request")?;

        let response_bytes = tokio::time::timeout(TRUST_REQUEST_TIMEOUT, recv.read_to_end(MAX_TRUST_MESSAGE_SIZE))
            .await
            .context("trust response timeout")?
            .context("failed to read trust response")?;
        let response: TrustResponse =
            postcard::from_bytes(&response_bytes).context("failed to deserialize trust response")?;

        match response {
            TrustResponse::Share(share) => Ok(share),
            TrustResponse::Expunged { epoch } => Err(ExpungedByPeer { epoch }.into()),
        }
    }
}
