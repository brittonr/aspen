//! Per-request bridge between hyper HTTP/1.1 and h3 streams.
//!
//! Receives a hyper request, opens an h3 stream on the existing QUIC
//! connection, forwards method/path/headers/body, and streams the h3
//! response back without buffering the entire body.

use std::convert::Infallible;
use std::sync::Arc;

use bytes::Buf;
use bytes::Bytes;
use http_body_util::Full;
use hyper::Request;
use hyper::Response;
use hyper::StatusCode;
use tracing::debug;
use tracing::warn;

use crate::connection::ConnectionPool;

/// Tiger Style: max response body (256 MB) to prevent unbounded memory.
const MAX_RESPONSE_BODY: usize = 256 * 1024 * 1024;

/// Handle one HTTP/1.1 request by forwarding it over h3 to the iroh endpoint.
pub async fn handle_request(
    pool: Arc<ConnectionPool>,
    req: Request<hyper::body::Incoming>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    match forward_request(&pool, req).await {
        Ok(resp) => Ok(resp),
        Err(e) => {
            warn!(error = %e, "proxy request failed");
            Ok(bad_gateway(&e.to_string()))
        }
    }
}

/// Forward a single request through the h3 connection.
async fn forward_request(
    pool: &ConnectionPool,
    req: Request<hyper::body::Incoming>,
) -> anyhow::Result<Response<Full<Bytes>>> {
    let (parts, _body) = req.into_parts();

    // Build the URI path with query string
    let uri = match parts.uri.path_and_query() {
        Some(pq) => pq.as_str().to_string(),
        None => "/".to_string(),
    };

    let method = parts.method.clone();

    debug!(%method, %uri, "forwarding request");

    // Get an h3 sender from the pool
    let (mut sender, timeout) = match pool.get_sender().await {
        Ok(s) => s,
        Err(e) => return Err(anyhow::anyhow!("failed to get h3 connection: {e}")),
    };

    // Build h3 request — forward method, path, and selected headers
    let mut h3_req_builder = http::Request::builder().method(method.clone()).uri(&uri);

    // Forward safe hop-by-hop headers
    for (name, value) in &parts.headers {
        match name.as_str() {
            // Skip hop-by-hop headers that don't apply across the bridge
            "host" | "connection" | "transfer-encoding" | "upgrade" => continue,
            _ => {
                h3_req_builder = h3_req_builder.header(name, value);
            }
        }
    }

    // Always set a host header for the h3 side
    h3_req_builder = h3_req_builder.header("host", "aspen-proxy");

    let h3_req = h3_req_builder.body(())?;

    // Send request on a new h3 stream
    let send_result = tokio::time::timeout(timeout, sender.send_request(h3_req)).await;

    let mut stream = match send_result {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            // Stream error — invalidate the connection so next request reconnects
            pool.invalidate().await;
            return Err(anyhow::anyhow!("h3 send_request failed: {e}"));
        }
        Err(_) => {
            pool.invalidate().await;
            return Err(anyhow::anyhow!("h3 send_request timed out"));
        }
    };

    // Finish sending (no request body for GET — extend for POST later)
    stream.finish().await?;

    // Receive h3 response headers
    let h3_resp = stream.recv_response().await?;
    let status = h3_resp.status();

    debug!(%status, "received h3 response");

    // Collect response body (streaming into a buffer)
    // TODO: switch to StreamBody for true streaming once hyper body type is flexible
    let mut body = Vec::new();
    while let Some(chunk) = stream.recv_data().await? {
        body.extend_from_slice(chunk.chunk());
        if body.len() > MAX_RESPONSE_BODY {
            return Err(anyhow::anyhow!("response body exceeds {MAX_RESPONSE_BODY} bytes"));
        }
    }

    // Build the TCP response, forwarding status + headers
    let mut resp_builder = Response::builder().status(status);
    for (name, value) in h3_resp.headers() {
        resp_builder = resp_builder.header(name, value);
    }

    let resp = resp_builder.body(Full::new(Bytes::from(body)))?;
    Ok(resp)
}

/// Return a 502 Bad Gateway response.
fn bad_gateway(msg: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::BAD_GATEWAY)
        .header("content-type", "text/plain")
        .body(Full::new(Bytes::from(msg.to_string())))
        .unwrap_or_else(|_| Response::new(Full::new(Bytes::from("bad gateway"))))
}
