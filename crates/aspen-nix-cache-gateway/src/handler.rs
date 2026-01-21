//! Iroh protocol handler for the Nix cache HTTP/3 gateway.
//!
//! Implements `iroh::protocol::ProtocolHandler` for integration with Iroh's Router.

use std::fmt;
use std::future::Future;
use std::sync::Arc;

use aspen_blob::BlobStore;
use aspen_cache::CacheIndex;
use bytes::Bytes;
use h3::quic::BidiStream;
use h3::server::RequestStream;
use http::Request;
use iroh::endpoint::Connection;
use iroh::protocol::{AcceptError, ProtocolHandler};
use tokio::sync::Semaphore;
use tracing::{debug, error, info, instrument, warn};

use crate::config::NixCacheGatewayConfig;
use crate::signing::NarinfoSigner;
use crate::streaming::NarStreamingHandler;

/// Protocol handler for Nix cache HTTP/3 gateway.
///
/// Integrates with Iroh's Router to handle connections using the `iroh+h3` ALPN.
pub struct NixCacheProtocolHandler<I, B>
where
    I: CacheIndex + Send + Sync + 'static,
    B: BlobStore + Send + Sync + 'static,
{
    /// The streaming handler for NAR requests.
    streaming_handler: Arc<NarStreamingHandler<I, B>>,
    /// Semaphore for connection limiting.
    connection_semaphore: Arc<Semaphore>,
}

impl<I, B> fmt::Debug for NixCacheProtocolHandler<I, B>
where
    I: CacheIndex + Send + Sync + 'static,
    B: BlobStore + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NixCacheProtocolHandler")
            .field("max_connections", &self.connection_semaphore.available_permits())
            .finish()
    }
}

impl<I, B> NixCacheProtocolHandler<I, B>
where
    I: CacheIndex + Send + Sync + 'static,
    B: BlobStore + Send + Sync + 'static,
{
    /// Create a new protocol handler.
    pub fn new(
        config: NixCacheGatewayConfig,
        cache_index: Arc<I>,
        blob_store: Arc<B>,
        signer: Option<NarinfoSigner>,
    ) -> Self {
        let max_connections = config.max_connections;
        let streaming_handler = Arc::new(NarStreamingHandler::new(config, cache_index, blob_store, signer));

        Self {
            streaming_handler,
            connection_semaphore: Arc::new(Semaphore::new(max_connections as usize)),
        }
    }

    /// Create with default connection limit.
    pub fn with_defaults(cache_index: Arc<I>, blob_store: Arc<B>) -> Self {
        Self::new(NixCacheGatewayConfig::default(), cache_index, blob_store, None)
    }

    /// Get the streaming handler.
    pub fn streaming_handler(&self) -> &NarStreamingHandler<I, B> {
        &self.streaming_handler
    }

    /// Handle a single HTTP/3 connection.
    async fn handle_connection(&self, conn: Connection) {
        let remote_id = conn.remote_id().fmt_short().to_string();
        debug!(remote = %remote_id, "new HTTP/3 connection");

        // Wrap connection with h3-iroh (not async)
        let h3_conn = h3_iroh::Connection::new(conn);

        // Create h3 server connection
        let mut server_conn = match h3::server::Connection::new(h3_conn).await {
            Ok(c) => c,
            Err(e) => {
                warn!(remote = %remote_id, error = %e, "failed to create h3 server");
                return;
            }
        };

        // Accept and handle requests
        loop {
            match server_conn.accept().await {
                Ok(Some(req_resolver)) => {
                    // Resolve the request to get (request, stream)
                    let (request, stream) = match req_resolver.resolve_request().await {
                        Ok((req, stream)) => (req, stream),
                        Err(e) => {
                            error!(error = %e, "failed to resolve request");
                            continue;
                        }
                    };

                    let handler = Arc::clone(&self.streaming_handler);
                    tokio::spawn(async move {
                        if let Err(e) = handle_request(handler, request, stream).await {
                            warn!(error = %e, "request handling failed");
                        }
                    });
                }
                Ok(None) => {
                    // Connection closed gracefully
                    debug!(remote = %remote_id, "connection closed");
                    break;
                }
                Err(e) => {
                    warn!(remote = %remote_id, error = %e, "error accepting request");
                    break;
                }
            }
        }
    }
}

impl<I, B> ProtocolHandler for NixCacheProtocolHandler<I, B>
where
    I: CacheIndex + Send + Sync + 'static,
    B: BlobStore + Send + Sync + 'static,
{
    fn accept(&self, conn: Connection) -> impl Future<Output = Result<(), AcceptError>> + Send {
        let handler = self.clone();
        let semaphore = Arc::clone(&self.connection_semaphore);

        async move {
            // Acquire connection permit
            let permit = match semaphore.try_acquire() {
                Ok(p) => p,
                Err(_) => {
                    info!("rejecting connection: max connections reached");
                    return Err(AcceptError::from_err(std::io::Error::new(
                        std::io::ErrorKind::ConnectionRefused,
                        "max connections reached",
                    )));
                }
            };

            // Handle the connection (permit is dropped when this completes)
            handler.handle_connection(conn).await;
            drop(permit);

            Ok(())
        }
    }

    fn shutdown(&self) -> impl Future<Output = ()> + Send {
        async move {
            info!("Nix cache gateway shutting down");
        }
    }
}

impl<I, B> Clone for NixCacheProtocolHandler<I, B>
where
    I: CacheIndex + Send + Sync + 'static,
    B: BlobStore + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            streaming_handler: Arc::clone(&self.streaming_handler),
            connection_semaphore: Arc::clone(&self.connection_semaphore),
        }
    }
}

/// Handle a single HTTP/3 request.
#[instrument(skip(handler, request, stream), fields(
    method = %request.method(),
    path = %request.uri().path()
))]
async fn handle_request<I, B, T>(
    handler: Arc<NarStreamingHandler<I, B>>,
    request: Request<()>,
    mut stream: RequestStream<T, Bytes>,
) -> crate::Result<()>
where
    I: CacheIndex + Send + Sync + 'static,
    B: BlobStore + Send + Sync + 'static,
    T: BidiStream<Bytes>,
{
    use crate::endpoints::{cache_info, narinfo};

    let method = request.method();
    let path = request.uri().path();

    debug!(%method, %path, "handling request");

    match (method, path) {
        // GET /nix-cache-info
        (&http::Method::GET, "/nix-cache-info") => {
            let response = cache_info::handle_cache_info(handler.config());
            send_response(&mut stream, response).await?;
        }

        // GET /{hash}.narinfo
        (&http::Method::GET, path) if path.ends_with(".narinfo") => {
            match narinfo::extract_store_hash(path) {
                Some(store_hash) => {
                    match narinfo::handle_narinfo(store_hash, handler.cache_index(), handler.signer()).await {
                        Ok(response) => {
                            send_response(&mut stream, response).await?;
                        }
                        Err(e) => {
                            send_error(&mut stream, e.status_code(), &e.to_string()).await?;
                        }
                    }
                }
                None => {
                    send_error(&mut stream, http::StatusCode::BAD_REQUEST, "invalid narinfo path").await?;
                }
            }
        }

        // HEAD /{hash}.narinfo
        (&http::Method::HEAD, path) if path.ends_with(".narinfo") => {
            match narinfo::extract_store_hash(path) {
                Some(store_hash) => {
                    match narinfo::handle_narinfo(store_hash, handler.cache_index(), handler.signer()).await {
                        Ok(response) => {
                            // HEAD: send headers only, no body
                            let headers_only = http::Response::builder()
                                .status(response.status())
                                .header("Content-Type", "text/x-nix-narinfo")
                                .header("Content-Length", response.body().len())
                                .body(String::new())
                                .expect("valid response");
                            send_response(&mut stream, headers_only).await?;
                        }
                        Err(e) => {
                            send_error(&mut stream, e.status_code(), "").await?;
                        }
                    }
                }
                None => {
                    send_error(&mut stream, http::StatusCode::BAD_REQUEST, "").await?;
                }
            }
        }

        // GET /nar/{hash}.nar
        (&http::Method::GET, path) if path.starts_with("/nar/") && path.ends_with(".nar") => {
            let range_header = request
                .headers()
                .get(http::header::RANGE)
                .and_then(|v| v.to_str().ok());

            match handler.prepare_download(path, range_header).await {
                Ok(download) => {
                    // Send headers first
                    let headers = download.response_headers();
                    let h3_headers = http::Response::builder()
                        .status(headers.status())
                        .header("Content-Type", "application/x-nix-nar")
                        .header("Accept-Ranges", "bytes")
                        .header("Content-Length", download.transfer_size())
                        .body(())
                        .expect("valid headers");

                    stream
                        .send_response(h3_headers)
                        .await
                        .map_err(|e| crate::NixCacheError::StreamError {
                            message: e.to_string(),
                        })?;

                    // Stream body chunks
                    let result = stream_nar_data(&handler, &download, &mut stream).await;

                    // Finish stream
                    if let Err(e) = stream.finish().await {
                        warn!(error = %e, "failed to finish stream");
                    }

                    if let Err(e) = result {
                        // Already sent headers, can't send error response
                        warn!(error = %e, "streaming error after headers sent");
                    }
                }
                Err(e) => {
                    send_error(&mut stream, e.status_code(), &e.to_string()).await?;
                }
            }
        }

        // HEAD /nar/{hash}.nar
        (&http::Method::HEAD, path) if path.starts_with("/nar/") && path.ends_with(".nar") => {
            match handler.prepare_download(path, None).await {
                Ok(download) => {
                    let headers = http::Response::builder()
                        .status(http::StatusCode::OK)
                        .header("Content-Type", "application/x-nix-nar")
                        .header("Accept-Ranges", "bytes")
                        .header("Content-Length", download.content_length)
                        .body(String::new())
                        .expect("valid headers");
                    send_response(&mut stream, headers).await?;
                }
                Err(e) => {
                    send_error(&mut stream, e.status_code(), "").await?;
                }
            }
        }

        // 404 for everything else
        _ => {
            send_error(&mut stream, http::StatusCode::NOT_FOUND, "not found").await?;
        }
    }

    Ok(())
}

/// Stream NAR data directly to avoid closure lifetime issues.
async fn stream_nar_data<I, B, T>(
    handler: &NarStreamingHandler<I, B>,
    download: &crate::endpoints::nar::NarDownload,
    stream: &mut RequestStream<T, Bytes>,
) -> crate::Result<()>
where
    I: CacheIndex + Send + Sync + 'static,
    B: BlobStore + Send + Sync + 'static,
    T: BidiStream<Bytes>,
{
    use std::io::SeekFrom;
    use std::time::Instant;
    use tokio::io::{AsyncReadExt, AsyncSeekExt};

    use crate::constants::MAX_STREAM_DURATION;

    let start_time = Instant::now();
    let chunk_size = handler.config().nar_chunk_size_bytes as usize;

    // Get streaming reader
    let mut reader = handler
        .blob_store()
        .reader(&download.blob_hash)
        .await
        .map_err(|e| crate::NixCacheError::BlobStore {
            message: e.to_string(),
        })?
        .ok_or_else(|| crate::NixCacheError::BlobNotAvailable {
            blob_hash: download.blob_hash.to_string(),
        })?;

    // Seek to range start if partial
    let (transfer_start, transfer_end) = match download.range {
        Some((start, end)) => {
            reader.seek(SeekFrom::Start(start)).await.map_err(|e| crate::NixCacheError::BlobStore {
                message: format!("seek error: {}", e),
            })?;
            (start, end)
        }
        None => (0, download.content_length - 1),
    };

    let transfer_size = transfer_end - transfer_start + 1;
    let mut bytes_sent: u64 = 0;
    let mut buf = vec![0u8; chunk_size];

    while bytes_sent < transfer_size {
        // Check timeout
        if start_time.elapsed() > MAX_STREAM_DURATION {
            return Err(crate::NixCacheError::Timeout {
                elapsed_ms: start_time.elapsed().as_millis() as u64,
            });
        }

        // Calculate how much to read
        let remaining = (transfer_size - bytes_sent) as usize;
        let read_size = remaining.min(chunk_size);

        // Read chunk from blob store
        let bytes_read = reader.read(&mut buf[..read_size]).await.map_err(|e| crate::NixCacheError::BlobStore {
            message: format!("read error: {}", e),
        })?;

        if bytes_read == 0 {
            // Unexpected EOF
            warn!(bytes_sent, transfer_size, "unexpected EOF during streaming");
            break;
        }

        // Send chunk via h3 (this is where h3 backpressure happens)
        let chunk = Bytes::copy_from_slice(&buf[..bytes_read]);
        stream.send_data(chunk).await.map_err(|e| crate::NixCacheError::StreamError {
            message: e.to_string(),
        })?;

        bytes_sent += bytes_read as u64;
    }

    info!(bytes_sent, partial = download.is_partial(), "NAR streaming complete");

    Ok(())
}

/// Send an HTTP response.
async fn send_response<T>(
    stream: &mut RequestStream<T, Bytes>,
    response: http::Response<String>,
) -> crate::Result<()>
where
    T: BidiStream<Bytes>,
{
    let (parts, body) = response.into_parts();

    // Send headers
    let headers = http::Response::from_parts(parts, ());
    stream.send_response(headers).await.map_err(|e| crate::NixCacheError::StreamError {
        message: e.to_string(),
    })?;

    // Send body if not empty
    if !body.is_empty() {
        stream
            .send_data(Bytes::from(body))
            .await
            .map_err(|e| crate::NixCacheError::StreamError {
                message: e.to_string(),
            })?;
    }

    // Finish stream
    stream.finish().await.map_err(|e| crate::NixCacheError::StreamError {
        message: e.to_string(),
    })?;

    Ok(())
}

/// Send an error response.
async fn send_error<T>(
    stream: &mut RequestStream<T, Bytes>,
    status: http::StatusCode,
    message: &str,
) -> crate::Result<()>
where
    T: BidiStream<Bytes>,
{
    let response = http::Response::builder()
        .status(status)
        .header("Content-Type", "text/plain")
        .body(message.to_string())
        .expect("valid response");

    send_response(stream, response).await
}
