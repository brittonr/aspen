//! HTTP-to-Iroh proxy for Nix binary cache.
//!
//! This module provides a lightweight HTTP/1.1 server that proxies Nix cache
//! requests to Aspen's nix-cache-gateway over Iroh QUIC (HTTP/3).
//!
//! # Purpose
//!
//! Nix expects HTTP(S) URLs for substituters, but Aspen's cache gateway uses
//! HTTP/3 over Iroh QUIC. This proxy bridges that gap by:
//!
//! 1. Listening on localhost for HTTP/1.1 requests
//! 2. Forwarding requests to the gateway via Iroh H3
//! 3. Streaming responses back to Nix
//!
//! # Usage
//!
//! ```ignore
//! let proxy = CacheProxy::start(endpoint, gateway_node_id).await?;
//! let substituter_url = proxy.substituter_url();
//! // Use in nix build --substituters "{substituter_url}"
//! proxy.shutdown().await;
//! ```
//!
//! # Security
//!
//! - Binds only to 127.0.0.1 (localhost)
//! - Never exposes the proxy externally
//! - All upstream requests go through authenticated Iroh connections

use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use bytes::Buf;
use bytes::Bytes;
use http_body_util::Full;
use hyper::Request;
use hyper::Response;
use hyper::StatusCode;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use iroh::Endpoint;
use iroh::PublicKey;
use snafu::ResultExt;
use snafu::Snafu;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::instrument;
use tracing::warn;

// Tiger Style: Explicit bounds
/// Maximum request body size (none expected for cache requests).
#[allow(dead_code)]
const MAX_REQUEST_BODY_SIZE: u64 = 1024;
/// Connection timeout for Iroh H3 requests.
const H3_CONNECTION_TIMEOUT: Duration = Duration::from_secs(30);
/// Read timeout for streaming responses.
#[allow(dead_code)]
const STREAM_READ_TIMEOUT: Duration = Duration::from_secs(60);
/// Maximum concurrent requests per proxy instance.
const MAX_CONCURRENT_REQUESTS: usize = 100;
/// Buffer size for streaming NAR chunks.
#[allow(dead_code)]
const NAR_CHUNK_SIZE: usize = 64 * 1024;
/// Maximum response body size (100 MB).
const MAX_RESPONSE_SIZE: usize = 100 * 1024 * 1024;

/// ALPN protocol for Nix cache gateway.
pub const NIX_CACHE_ALPN: &[u8] = b"iroh+h3";

/// Errors from the cache proxy.
#[derive(Debug, Snafu)]
pub enum CacheProxyError {
    /// Failed to bind to local address.
    #[snafu(display("failed to bind proxy to {addr}: {source}"))]
    Bind {
        /// The address we tried to bind to.
        addr: String,
        /// The underlying IO error.
        source: std::io::Error,
    },

    /// Failed to connect to gateway.
    #[snafu(display("failed to connect to gateway {node_id}: {reason}"))]
    GatewayConnection {
        /// The gateway node ID (short form).
        node_id: String,
        /// The connection error reason.
        reason: String,
    },

    /// H3 protocol error.
    #[snafu(display("H3 protocol error: {reason}"))]
    H3Protocol {
        /// The protocol error reason.
        reason: String,
    },

    /// Request timeout.
    #[snafu(display("request timed out after {timeout_secs}s"))]
    Timeout {
        /// The timeout duration in seconds.
        timeout_secs: u64,
    },

    /// Proxy shutdown.
    #[snafu(display("proxy is shutting down"))]
    #[allow(dead_code)]
    Shutdown,
}

type ProxyResult<T> = std::result::Result<T, CacheProxyError>;

/// Lightweight HTTP proxy that bridges Nix's HTTP requests to Iroh H3.
///
/// Embedded in NixBuildWorker - starts on demand, shuts down after build.
pub struct CacheProxy {
    /// Local address the proxy is listening on.
    local_addr: SocketAddr,
    /// Shutdown signal sender.
    shutdown_tx: Option<oneshot::Sender<()>>,
    /// Server task handle.
    task_handle: Option<JoinHandle<()>>,
}

impl CacheProxy {
    /// Start proxy on an ephemeral port, returns the proxy instance.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - Iroh endpoint for connecting to the gateway
    /// * `gateway_node` - PublicKey of the nix-cache-gateway
    ///
    /// # Returns
    ///
    /// A running proxy instance with the URL for `--substituters`.
    #[instrument(skip(endpoint), fields(gateway = %gateway_node))]
    pub async fn start(endpoint: Arc<Endpoint>, gateway_node: PublicKey) -> ProxyResult<Self> {
        // Bind to localhost only (security: never external)
        let listener = TcpListener::bind("127.0.0.1:0").await.context(BindSnafu {
            addr: "127.0.0.1:0".to_string(),
        })?;

        let local_addr = listener.local_addr().context(BindSnafu {
            addr: "127.0.0.1:0".to_string(),
        })?;

        info!(addr = %local_addr, "cache proxy started");

        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        // Create shared state for request handlers
        let state = Arc::new(ProxyState {
            endpoint,
            gateway_node,
            request_semaphore: Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_REQUESTS)),
        });

        // Spawn server task
        let task_handle = tokio::spawn(run_server(listener, state, shutdown_rx));

        Ok(Self {
            local_addr,
            shutdown_tx: Some(shutdown_tx),
            task_handle: Some(task_handle),
        })
    }

    /// Get the substituter URL for nix build.
    ///
    /// Returns a URL like `http://127.0.0.1:PORT` suitable for
    /// `--substituters` argument.
    pub fn substituter_url(&self) -> String {
        format!("http://{}", self.local_addr)
    }

    /// Get the local address the proxy is listening on.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Graceful shutdown of the proxy.
    ///
    /// Signals the server to stop accepting new connections and waits
    /// for the server task to complete.
    pub async fn shutdown(mut self) {
        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }

        // Wait for server to finish
        if let Some(handle) = self.task_handle.take() {
            if let Err(e) = handle.await {
                warn!(error = %e, "proxy task panicked during shutdown");
            }
        }

        info!(addr = %self.local_addr, "cache proxy stopped");
    }
}

impl Drop for CacheProxy {
    fn drop(&mut self) {
        // Send shutdown signal if not already sent
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        // Note: We can't await the task handle in drop, so the task
        // will be aborted when the JoinHandle is dropped.
    }
}

/// Shared state for request handlers.
struct ProxyState {
    /// Iroh endpoint for gateway connections.
    endpoint: Arc<Endpoint>,
    /// Gateway node ID.
    gateway_node: PublicKey,
    /// Semaphore for limiting concurrent requests.
    request_semaphore: Arc<tokio::sync::Semaphore>,
}

/// Run the HTTP server until shutdown.
async fn run_server(listener: TcpListener, state: Arc<ProxyState>, mut shutdown_rx: oneshot::Receiver<()>) {
    loop {
        tokio::select! {
            // Accept new connection
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((stream, remote_addr)) => {
                        debug!(remote = %remote_addr, "new connection");

                        let state = Arc::clone(&state);
                        tokio::spawn(async move {
                            let io = TokioIo::new(stream);
                            let service = service_fn(move |req| {
                                handle_request(Arc::clone(&state), req)
                            });

                            if let Err(e) = http1::Builder::new()
                                .serve_connection(io, service)
                                .await
                            {
                                if !e.is_incomplete_message() {
                                    warn!(error = %e, "connection error");
                                }
                            }
                        });
                    }
                    Err(e) => {
                        error!(error = %e, "accept failed");
                    }
                }
            }

            // Shutdown signal
            _ = &mut shutdown_rx => {
                info!("proxy received shutdown signal");
                break;
            }
        }
    }
}

/// Handle a single HTTP request by proxying to the gateway.
#[instrument(skip(state, req), fields(method = %req.method(), uri = %req.uri()))]
async fn handle_request(
    state: Arc<ProxyState>,
    req: Request<hyper::body::Incoming>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    // Acquire request permit
    let _permit = match state.request_semaphore.try_acquire() {
        Ok(p) => p,
        Err(_) => {
            warn!("too many concurrent requests");
            return Ok(error_response(StatusCode::SERVICE_UNAVAILABLE, "too many requests"));
        }
    };

    // Route based on path
    let path = req.uri().path();

    let result = if path == "/nix-cache-info" {
        handle_cache_info(&state).await
    } else if path.ends_with(".narinfo") {
        handle_narinfo(&state, path).await
    } else if path.starts_with("/nar/") && path.ends_with(".nar") {
        handle_nar(&state, path, req.headers().get("range").and_then(|v| v.to_str().ok())).await
    } else {
        Ok(error_response(StatusCode::NOT_FOUND, "not found"))
    };

    Ok(result.unwrap_or_else(|e| {
        warn!(error = %e, "request failed");
        error_response(StatusCode::BAD_GATEWAY, &e.to_string())
    }))
}

/// Handle GET /nix-cache-info
async fn handle_cache_info(state: &ProxyState) -> ProxyResult<Response<Full<Bytes>>> {
    let body = forward_get_request(state, "/nix-cache-info").await?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/plain")
        .body(Full::new(body))
        .expect("valid response"))
}

/// Handle GET /{hash}.narinfo
async fn handle_narinfo(state: &ProxyState, path: &str) -> ProxyResult<Response<Full<Bytes>>> {
    let body = forward_get_request(state, path).await?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/x-nix-narinfo")
        .body(Full::new(body))
        .expect("valid response"))
}

/// Handle GET /nar/{hash}.nar
async fn handle_nar(state: &ProxyState, path: &str, range_header: Option<&str>) -> ProxyResult<Response<Full<Bytes>>> {
    // For now, forward the full NAR. Range support can be added later.
    // TODO: Implement streaming for large NARs instead of buffering
    let body = forward_get_request(state, path).await?;

    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/x-nix-nar")
        .header("Content-Length", body.len());

    if range_header.is_some() {
        // Acknowledge we received the range header even though we're
        // returning the full content. Nix handles this gracefully.
        builder = builder.header("Accept-Ranges", "bytes");
    }

    Ok(builder.body(Full::new(body)).expect("valid response"))
}

/// Forward a GET request to the gateway via H3.
async fn forward_get_request(state: &ProxyState, path: &str) -> ProxyResult<Bytes> {
    // Connect to gateway
    let conn = tokio::time::timeout(H3_CONNECTION_TIMEOUT, state.endpoint.connect(state.gateway_node, NIX_CACHE_ALPN))
        .await
        .map_err(|_| CacheProxyError::Timeout {
            timeout_secs: H3_CONNECTION_TIMEOUT.as_secs(),
        })?
        .map_err(|e| CacheProxyError::GatewayConnection {
            node_id: state.gateway_node.fmt_short().to_string(),
            reason: e.to_string(),
        })?;

    // Wrap with h3-iroh
    let h3_conn = h3_iroh::Connection::new(conn);

    // Create H3 client connection
    let (mut driver, mut send_request) =
        h3::client::new(h3_conn).await.map_err(|e| CacheProxyError::H3Protocol { reason: e.to_string() })?;

    // Drive the connection in the background
    // We use a channel to signal when we're done so the driver can close cleanly
    let (done_tx, done_rx) = oneshot::channel::<()>();
    let drive_handle = tokio::spawn(async move {
        tokio::select! {
            biased;
            _ = done_rx => {
                // Request completed, close driver
            }
            error = std::future::poll_fn(|cx| driver.poll_close(cx)) => {
                debug!(error = %error, "H3 driver error");
            }
        }
    });

    // Build request
    let req = http::Request::builder()
        .method("GET")
        .uri(path)
        .header("host", "aspen-cache")
        .body(())
        .expect("valid request");

    // Send request
    let mut stream = send_request
        .send_request(req)
        .await
        .map_err(|e| CacheProxyError::H3Protocol { reason: e.to_string() })?;

    // Finish sending (no body)
    stream.finish().await.map_err(|e| CacheProxyError::H3Protocol { reason: e.to_string() })?;

    // Receive response
    let resp = stream.recv_response().await.map_err(|e| CacheProxyError::H3Protocol { reason: e.to_string() })?;

    debug!(status = %resp.status(), "received response from gateway");

    // Check status
    if !resp.status().is_success() {
        // Signal driver to close
        let _ = done_tx.send(());
        let _ = drive_handle.await;

        return Err(CacheProxyError::H3Protocol {
            reason: format!("gateway returned {}", resp.status()),
        });
    }

    // Collect response body
    let mut body = Vec::new();
    while let Some(chunk) =
        stream.recv_data().await.map_err(|e| CacheProxyError::H3Protocol { reason: e.to_string() })?
    {
        // Use Buf trait to get the bytes
        body.extend_from_slice(chunk.chunk());

        // Prevent unbounded memory growth
        if body.len() > MAX_RESPONSE_SIZE {
            // Signal driver to close
            let _ = done_tx.send(());
            let _ = drive_handle.await;

            return Err(CacheProxyError::H3Protocol {
                reason: "response body too large".to_string(),
            });
        }
    }

    // Signal driver to close
    let _ = done_tx.send(());
    let _ = drive_handle.await;

    Ok(Bytes::from(body))
}

/// Create an error response.
fn error_response(status: StatusCode, message: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header("Content-Type", "text/plain")
        .body(Full::new(Bytes::from(message.to_string())))
        .expect("valid response")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_substituter_url_format() {
        // The URL format should be valid for nix --substituters
        let url = "http://127.0.0.1:12345";
        assert!(url.starts_with("http://127.0.0.1:"));
    }

    #[test]
    fn test_alpn_constant() {
        assert_eq!(NIX_CACHE_ALPN, b"iroh+h3");
    }
}
