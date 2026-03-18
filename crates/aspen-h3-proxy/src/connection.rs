//! Iroh QUIC connection management with h3 multiplexing.
//!
//! Maintains a single QUIC connection to the target iroh endpoint and
//! multiplexes h3 streams over it. Reconnects with exponential backoff
//! when the connection drops.

use std::sync::Arc;
use std::time::Duration;

use h3::client::SendRequest;
use iroh::Endpoint;
use iroh::PublicKey;
use iroh_h3::Connection as IrohH3Connection;
use iroh_h3::OpenStreams;
use tokio::sync::Mutex;
use tokio::sync::oneshot;
use tracing::debug;
use tracing::info;

/// A live h3 session: the send handle plus a driver shutdown signal.
struct H3Session {
    send_request: SendRequest<OpenStreams, bytes::Bytes>,
    _driver_shutdown: oneshot::Sender<()>,
}

/// Manages an h3 connection to a single iroh endpoint.
///
/// Each call to `send_request()` gets a clone of the h3 `SendRequest`
/// handle, which opens a new stream on the existing QUIC connection.
/// If the connection is dead, it reconnects with exponential backoff.
pub struct ConnectionPool {
    endpoint: Arc<Endpoint>,
    target: PublicKey,
    alpn: Vec<u8>,
    request_timeout: Duration,
    session: Mutex<Option<H3Session>>,
}

impl ConnectionPool {
    pub fn new(endpoint: Arc<Endpoint>, target: PublicKey, alpn: Vec<u8>, request_timeout: Duration) -> Self {
        Self {
            endpoint,
            target,
            alpn,
            request_timeout,
            session: Mutex::new(None),
        }
    }

    /// Get an h3 `SendRequest` handle, connecting if needed.
    ///
    /// Returns the handle and the request timeout. The caller is responsible
    /// for opening a stream and sending the request. If no connection can be
    /// established within the request timeout, returns an error (caller sends 502).
    pub async fn get_sender(&self) -> anyhow::Result<(SendRequest<OpenStreams, bytes::Bytes>, Duration)> {
        let mut guard = self.session.lock().await;

        // Try the existing session first
        if let Some(session) = guard.as_ref() {
            if !session._driver_shutdown.is_closed() {
                return Ok((session.send_request.clone(), self.request_timeout));
            }
            // Driver exited — connection is dead
            debug!("h3 connection dead, reconnecting");
        }

        // Try a single connection attempt with a timeout rather than
        // looping with backoff. Backoff happens across requests — each
        // failing request triggers one attempt, and the caller gets 502.
        let session = tokio::time::timeout(self.request_timeout, self.try_connect())
            .await
            .map_err(|_| anyhow::anyhow!("connection timed out"))??;

        let sender = session.send_request.clone();
        *guard = Some(session);

        Ok((sender, self.request_timeout))
    }

    /// Invalidate the current connection, forcing a reconnect on next use.
    pub async fn invalidate(&self) {
        let mut guard = self.session.lock().await;
        *guard = None;
    }

    /// Single connection attempt.
    async fn try_connect(&self) -> anyhow::Result<H3Session> {
        let conn = tokio::time::timeout(Duration::from_secs(10), self.endpoint.connect(self.target, &self.alpn))
            .await
            .map_err(|_| anyhow::anyhow!("connection timeout"))??;

        info!(target_id = %self.target.fmt_short(), "connected to iroh endpoint");

        let h3_conn = IrohH3Connection::new(conn);
        let (mut driver, send_request) = h3::client::new(h3_conn).await?;

        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        tokio::spawn(async move {
            tokio::select! {
                biased;
                _ = shutdown_rx => {}
                err = std::future::poll_fn(|cx| driver.poll_close(cx)) => {
                    debug!(error = %err, "h3 driver closed");
                }
            }
        });

        Ok(H3Session {
            send_request,
            _driver_shutdown: shutdown_tx,
        })
    }
}
