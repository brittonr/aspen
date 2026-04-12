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

        let (mut send, mut recv) = tokio::time::timeout(TRUST_REQUEST_TIMEOUT, connection.open_bi())
            .await
            .context("trust stream open timeout")?
            .context("failed to open trust stream")?;

        let request = TrustRequest::Expunged { epoch };
        let request_bytes = postcard::to_stdvec(&request).context("failed to serialize expunged notification")?;
        send.write_all(&request_bytes).await.context("failed to write expunged notification")?;
        send.finish().context("failed to finish expunged notification")?;

        let response_bytes = tokio::time::timeout(TRUST_REQUEST_TIMEOUT, recv.read_to_end(MAX_TRUST_MESSAGE_SIZE))
            .await
            .context("expunged notification acknowledgement timeout")?
            .context("failed to read expunged notification acknowledgement")?;
        let response: TrustResponse = postcard::from_bytes(&response_bytes)
            .context("failed to deserialize expunged notification acknowledgement")?;
        match response {
            TrustResponse::Expunged { epoch: ack_epoch } if ack_epoch == epoch => Ok(()),
            TrustResponse::Expunged { epoch: ack_epoch } => {
                anyhow::bail!("expunged notification acknowledged wrong epoch: expected {epoch}, got {ack_epoch}")
            }
            other => anyhow::bail!("unexpected expunged notification acknowledgement: {other:?}"),
        }
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::AtomicU64;
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    use aspen_transport::TRUST_ALPN;
    use aspen_transport::TrustProtocolHandler;
    use aspen_transport::TrustShareProvider;
    use iroh::Endpoint;
    use iroh::EndpointId;
    use iroh::protocol::Router;
    use tokio::sync::Notify;
    use tokio::task::JoinHandle;

    use super::*;

    #[derive(Debug)]
    struct RecordingProvider {
        expunged_epoch: AtomicU64,
        entered_on_expunged: Notify,
        release_on_expunged: Notify,
        is_expunged: AtomicBool,
    }

    #[async_trait]
    impl TrustShareProvider for RecordingProvider {
        async fn get_share(&self, _requester: EndpointId, _epoch: u64) -> anyhow::Result<Option<TrustResponse>> {
            Ok(None)
        }

        fn is_expunged(&self) -> bool {
            self.is_expunged.load(Ordering::Acquire)
        }

        async fn on_expunged(&self, _from: EndpointId, epoch: u64) -> anyhow::Result<()> {
            self.entered_on_expunged.notify_one();
            self.release_on_expunged.notified().await;
            self.expunged_epoch.store(epoch, Ordering::Release);
            self.is_expunged.store(true, Ordering::Release);
            Ok(())
        }
    }

    async fn make_endpoint() -> Endpoint {
        Endpoint::builder(iroh::endpoint::presets::N0)
            .clear_address_lookup()
            .bind()
            .await
            .expect("bind endpoint")
    }

    #[tokio::test]
    async fn send_expunged_waits_for_peer_processing() {
        let server_ep = make_endpoint().await;
        let provider = Arc::new(RecordingProvider {
            expunged_epoch: AtomicU64::new(0),
            entered_on_expunged: Notify::new(),
            release_on_expunged: Notify::new(),
            is_expunged: AtomicBool::new(false),
        });
        let _router = Router::builder(server_ep.clone())
            .accept(TRUST_ALPN, TrustProtocolHandler::new(provider.clone()))
            .spawn();

        let client_ep = make_endpoint().await;
        let client = IrohTrustShareClient::new(Arc::new(client_ep.clone()));

        let send_task: JoinHandle<anyhow::Result<()>> = tokio::spawn({
            let server_addr = server_ep.addr();
            async move { client.send_expunged(server_addr, 7).await }
        });

        tokio::time::timeout(Duration::from_secs(2), provider.entered_on_expunged.notified())
            .await
            .expect("peer should begin processing expungement notification");
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(!send_task.is_finished(), "send_expunged returned before peer processing finished");
        assert_eq!(provider.expunged_epoch.load(Ordering::Acquire), 0);
        assert!(!provider.is_expunged.load(Ordering::Acquire));

        provider.release_on_expunged.notify_one();
        send_task.await.expect("send task should join").expect("send expunged notification");
        assert_eq!(provider.expunged_epoch.load(Ordering::Acquire), 7);
        assert!(provider.is_expunged.load(Ordering::Acquire));

        client_ep.close().await;
        server_ep.close().await;
    }
}
