//! HTTP/1.1 server implementing the Nix binary cache protocol.
//!
//! Routes:
//! - `GET /nix-cache-info` — cache metadata
//! - `GET /{hash}.narinfo` — store path metadata
//! - `GET /nar/{blob_hash}.nar` — NAR archive download

use std::net::SocketAddr;
use std::sync::Arc;

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use bytes::Bytes;
use http_body_util::Full;
use hyper::Request;
use hyper::Response;
use hyper::StatusCode;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use crate::GatewayState;

// Tiger Style: explicit bounds
/// Maximum concurrent HTTP requests.
const MAX_CONCURRENT_REQUESTS: usize = 100;
/// Maximum NAR size to serve (1 GB).
const MAX_NAR_STREAM_SIZE: u64 = 1_073_741_824;
/// Cache priority for substituter ordering.
/// Lower = higher priority. cache.nixos.org defaults to 40.
/// We use 30 so the cluster cache is preferred when available,
/// falling back to cache.nixos.org for paths we don't have.
const CACHE_PRIORITY: u32 = 30;

/// Run the HTTP server.
pub async fn run(state: Arc<GatewayState>, addr: SocketAddr) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS));

    info!(%addr, "nix cache gateway listening");
    info!(
        "configure nix: --substituters http://{addr} --trusted-public-keys \"{}\"",
        state.signing_key.to_nix_public_key()
    );

    loop {
        tokio::select! {
            accept = listener.accept() => {
                let (stream, peer_addr) = accept?;
                let state = state.clone();
                let semaphore = semaphore.clone();

                tokio::spawn(async move {
                    let io = TokioIo::new(stream);

                    let service = service_fn(move |req| {
                        let state = state.clone();
                        let semaphore = semaphore.clone();
                        async move {
                            // Enforce concurrency limit
                            let _permit = match semaphore.try_acquire() {
                                Ok(p) => p,
                                Err(_) => {
                                    warn!(%peer_addr, "request rejected: concurrency limit reached");
                                    return Ok(response(StatusCode::SERVICE_UNAVAILABLE, "too many requests"));
                                }
                            };

                            handle_request(req, &state).await
                        }
                    });

                    if let Err(e) = http1::Builder::new()
                        .serve_connection(io, service)
                        .await
                    {
                        debug!(%peer_addr, error = %e, "connection error");
                    }
                });
            }
            _ = tokio::signal::ctrl_c() => {
                info!("shutting down");
                return Ok(());
            }
        }
    }
}

/// Route an incoming HTTP request.
async fn handle_request(req: Request<Incoming>, state: &GatewayState) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let path = req.uri().path().to_string();
    debug!(path = %path, "incoming request");

    let result = if path == "/nix-cache-info" {
        Ok(handle_cache_info())
    } else if let Some(hash) = path.strip_prefix('/').and_then(|p| p.strip_suffix(".narinfo")) {
        handle_narinfo(state, hash).await
    } else if let Some(blob_hash) = path.strip_prefix("/nar/").and_then(|p| p.strip_suffix(".nar")) {
        handle_nar_download(state, blob_hash).await
    } else {
        Ok(response(StatusCode::NOT_FOUND, "not found"))
    };

    match result {
        Ok(resp) => Ok(resp),
        Err(e) => {
            error!(path = %path, error = %e, "internal error");
            Ok(response(StatusCode::INTERNAL_SERVER_ERROR, "internal error"))
        }
    }
}

/// Handle GET /nix-cache-info
fn handle_cache_info() -> Response<Full<Bytes>> {
    let body = format!("StoreDir: /nix/store\nWantMassQuery: 1\nPriority: {CACHE_PRIORITY}\n");
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/x-nix-cache-info")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

/// Handle GET /{hash}.narinfo
async fn handle_narinfo(state: &GatewayState, hash: &str) -> anyhow::Result<Response<Full<Bytes>>> {
    // Validate hash format
    if hash.is_empty() || hash.len() > 64 || !hash.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Ok(response(StatusCode::BAD_REQUEST, "invalid store hash"));
    }

    // Query cache via RPC
    let resp = state
        .client
        .send(ClientRpcRequest::CacheQuery {
            store_hash: hash.to_string(),
        })
        .await?;

    let entry = match resp {
        ClientRpcResponse::CacheQueryResult(r) => {
            if let Some(err) = r.error {
                warn!(hash, error = %err, "cache query error");
                return Ok(response(StatusCode::INTERNAL_SERVER_ERROR, "cache error"));
            }
            match r.entry {
                Some(e) => e,
                None => return Ok(response(StatusCode::NOT_FOUND, "not found")),
            }
        }
        ClientRpcResponse::Error(e) => {
            warn!(hash, code = %e.code, message = %e.message, "RPC error");
            return Ok(response(StatusCode::INTERNAL_SERVER_ERROR, "rpc error"));
        }
        _ => return Ok(response(StatusCode::INTERNAL_SERVER_ERROR, "unexpected response")),
    };

    // Build a CacheEntry for narinfo rendering
    let mut cache_entry = aspen_cache::CacheEntry::new(
        entry.store_path,
        entry.store_hash,
        entry.blob_hash,
        entry.nar_size,
        entry.nar_hash,
        entry.created_at_ms,
        entry.created_by_node,
    );
    if !entry.references.is_empty()
        && let Ok(e) = cache_entry.clone().with_references(entry.references)
    {
        cache_entry = e;
    }
    if let Some(deriver) = entry.deriver
        && let Ok(e) = cache_entry.clone().with_deriver(Some(deriver))
    {
        cache_entry = e;
    }

    // Sign the narinfo
    let fingerprint = cache_entry.fingerprint();
    let signature = state.signing_key.sign_fingerprint(&fingerprint);
    let narinfo = cache_entry.to_narinfo(Some(&signature));

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/x-nix-narinfo")
        .body(Full::new(Bytes::from(narinfo)))
        .unwrap())
}

/// Handle GET /nar/{blob_hash}.nar
async fn handle_nar_download(state: &GatewayState, blob_hash: &str) -> anyhow::Result<Response<Full<Bytes>>> {
    // Validate blob hash format
    if blob_hash.is_empty() || blob_hash.len() > 64 || !blob_hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(response(StatusCode::BAD_REQUEST, "invalid blob hash"));
    }

    // Download blob data via GetBlob RPC
    let resp = state
        .client
        .send(ClientRpcRequest::GetBlob {
            hash: blob_hash.to_string(),
        })
        .await;

    match resp {
        Ok(ClientRpcResponse::GetBlobResult(r)) => {
            if let Some(err) = r.error {
                warn!(blob_hash, error = %err, "blob get error");
                return Ok(response(StatusCode::NOT_FOUND, "blob not found"));
            }
            if let Some(data) = r.data {
                let size = data.len() as u64;
                if size > MAX_NAR_STREAM_SIZE {
                    return Ok(response(StatusCode::PAYLOAD_TOO_LARGE, "NAR exceeds maximum size"));
                }
                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "application/x-nix-nar")
                    .header("Content-Length", size.to_string())
                    .body(Full::new(Bytes::from(data)))
                    .unwrap())
            } else {
                Ok(response(StatusCode::NOT_FOUND, "blob not found"))
            }
        }
        Ok(ClientRpcResponse::Error(e)) => {
            debug!(blob_hash, code = %e.code, message = %e.message, "blob error");
            Ok(response(StatusCode::NOT_FOUND, "not found"))
        }
        Ok(_) => Ok(response(StatusCode::INTERNAL_SERVER_ERROR, "unexpected response")),
        Err(e) => {
            warn!(blob_hash, error = %e, "blob fetch failed");
            Ok(response(StatusCode::INTERNAL_SERVER_ERROR, "fetch failed"))
        }
    }
}

/// Build a simple text response.
fn response(status: StatusCode, body: &str) -> Response<Full<Bytes>> {
    Response::builder().status(status).body(Full::new(Bytes::from(body.to_string()))).unwrap()
}
