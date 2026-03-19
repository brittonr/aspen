//! Per-request bridge between hyper HTTP/1.1 and h3 streams.
//!
//! Receives a hyper request, opens an h3 stream on the existing QUIC
//! connection, forwards method/path/headers/body, and streams the h3
//! response body back chunk-by-chunk without buffering.

use std::convert::Infallible;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;

use bytes::Buf;
use bytes::Bytes;
use http_body::Frame;
use http_body_util::BodyExt;
use http_body_util::Full;
use hyper::Request;
use hyper::Response;
use hyper::StatusCode;
use tokio::sync::mpsc;
use tracing::debug;
use tracing::warn;

use crate::connection::ConnectionPool;

/// Tiger Style: max bytes the streaming body reader will forward before
/// killing the h3 stream. Prevents a misbehaving upstream from filling
/// memory on the proxy.
const MAX_RESPONSE_BODY_BYTES: usize = 256 * 1024 * 1024;

/// Tiger Style: channel depth for streaming body chunks. Provides
/// backpressure — if the TCP client is slow, the h3 reader blocks
/// after this many chunks are buffered.
const BODY_CHANNEL_DEPTH: usize = 32;

/// Tiger Style: max request body we'll collect before rejecting (16 MB).
const MAX_REQUEST_BODY_BYTES: usize = 16 * 1024 * 1024;

/// Body type returned to hyper. Either a fixed error page or a
/// channel-backed stream of h3 data frames.
pub type ProxyBody = http_body_util::Either<Full<Bytes>, StreamingBody>;

/// Streaming response body backed by an mpsc channel. An h3 reader
/// task sends data frames into the channel; hyper polls the receiver
/// to write them to the TCP socket.
pub struct StreamingBody {
    rx: mpsc::Receiver<Result<Bytes, StreamingBodyError>>,
}

/// Error surfaced through the body stream when the h3 reader fails.
#[derive(Debug)]
pub struct StreamingBodyError(String);

impl std::fmt::Display for StreamingBodyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for StreamingBodyError {}

impl http_body::Body for StreamingBody {
    type Data = Bytes;
    type Error = StreamingBodyError;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match self.rx.poll_recv(cx) {
            Poll::Ready(Some(Ok(chunk))) => Poll::Ready(Some(Ok(Frame::data(chunk)))),
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Handle one HTTP/1.1 request by forwarding it over h3 to the iroh endpoint.
pub async fn handle_request(
    pool: Arc<ConnectionPool>,
    req: Request<hyper::body::Incoming>,
) -> Result<Response<ProxyBody>, Infallible> {
    match forward_request(&pool, req).await {
        Ok(resp) => Ok(resp),
        Err(e) => {
            warn!(error = %e, "proxy request failed");
            Ok(bad_gateway(&e.to_string()))
        }
    }
}

/// Forward a single request through the h3 connection.
///
/// Returns as soon as the h3 response headers arrive. The response
/// body is streamed via a background task.
async fn forward_request(
    pool: &ConnectionPool,
    req: Request<hyper::body::Incoming>,
) -> anyhow::Result<Response<ProxyBody>> {
    let (parts, body) = req.into_parts();

    let uri = match parts.uri.path_and_query() {
        Some(pq) => pq.as_str().to_string(),
        None => "/".to_string(),
    };

    let method = parts.method.clone();
    let has_body = !matches!(method, hyper::Method::GET | hyper::Method::HEAD);

    debug!(%method, %uri, "forwarding request");

    // Get an h3 sender from the pool
    let (mut sender, timeout) = match pool.get_sender().await {
        Ok(s) => s,
        Err(e) => return Err(anyhow::anyhow!("failed to get h3 connection: {e}")),
    };

    // Build h3 request — forward method, path, and selected headers
    let mut h3_req_builder = http::Request::builder().method(method).uri(&uri);

    for (name, value) in &parts.headers {
        match name.as_str() {
            "host" | "connection" | "transfer-encoding" | "upgrade" => continue,
            _ => {
                h3_req_builder = h3_req_builder.header(name, value);
            }
        }
    }

    h3_req_builder = h3_req_builder.header("host", "aspen-proxy");

    let h3_req = h3_req_builder.body(())?;

    // Send request on a new h3 stream
    let send_result = tokio::time::timeout(timeout, sender.send_request(h3_req)).await;

    let mut stream = match send_result {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            pool.invalidate().await;
            return Err(anyhow::anyhow!("h3 send_request failed: {e}"));
        }
        Err(_) => {
            pool.invalidate().await;
            return Err(anyhow::anyhow!("h3 send_request timed out"));
        }
    };

    // Forward the request body for POST/PUT/PATCH/DELETE
    if has_body {
        let collected = body.collect().await.map_err(|e| anyhow::anyhow!("read request body: {e}"))?;
        let body_bytes = collected.to_bytes();
        if body_bytes.len() > MAX_REQUEST_BODY_BYTES {
            return Err(anyhow::anyhow!("request body exceeds {} bytes", MAX_REQUEST_BODY_BYTES));
        }
        if !body_bytes.is_empty() {
            stream.send_data(body_bytes).await.map_err(|e| anyhow::anyhow!("send request body: {e}"))?;
        }
    }

    stream.finish().await?;

    // Receive h3 response headers
    let h3_resp = stream.recv_response().await?;
    let status = h3_resp.status();

    debug!(%status, "received h3 response");

    // Build the TCP response headers immediately — body streams in background
    let mut resp_builder = Response::builder().status(status);
    for (name, value) in h3_resp.headers() {
        resp_builder = resp_builder.header(name, value);
    }

    // Spawn a task that reads h3 data frames into a channel
    let (tx, rx) = mpsc::channel(BODY_CHANNEL_DEPTH);

    tokio::spawn(async move {
        let mut total_bytes: usize = 0;
        loop {
            match stream.recv_data().await {
                Ok(Some(chunk)) => {
                    let data = Bytes::copy_from_slice(chunk.chunk());
                    total_bytes = total_bytes.saturating_add(data.len());
                    if total_bytes > MAX_RESPONSE_BODY_BYTES {
                        let _ = tx
                            .send(Err(StreamingBodyError(format!(
                                "response body exceeds {MAX_RESPONSE_BODY_BYTES} bytes"
                            ))))
                            .await;
                        stream.stop_sending(h3::error::Code::H3_EXCESSIVE_LOAD);
                        return;
                    }
                    if tx.send(Ok(data)).await.is_err() {
                        // TCP client disconnected — stop reading from h3
                        stream.stop_sending(h3::error::Code::H3_REQUEST_CANCELLED);
                        return;
                    }
                }
                Ok(None) => return, // body complete
                Err(e) => {
                    let _ = tx.send(Err(StreamingBodyError(format!("h3 recv_data: {e}")))).await;
                    return;
                }
            }
        }
    });

    let body = StreamingBody { rx };
    let resp = resp_builder.body(http_body_util::Either::Right(body))?;
    Ok(resp)
}

/// Return a 502 Bad Gateway response.
fn bad_gateway(msg: &str) -> Response<ProxyBody> {
    let body = Full::new(Bytes::from(msg.to_string()));
    Response::builder()
        .status(StatusCode::BAD_GATEWAY)
        .header("content-type", "text/plain")
        .body(http_body_util::Either::Left(body))
        .unwrap_or_else(|_| Response::new(http_body_util::Either::Left(Full::new(Bytes::from("bad gateway")))))
}

#[cfg(test)]
mod tests {
    use http_body_util::BodyExt;

    use super::*;

    /// StreamingBody yields data frames from the channel in order.
    #[tokio::test]
    async fn streaming_body_delivers_chunks() {
        let (tx, rx) = mpsc::channel(4);
        let body = StreamingBody { rx };

        tx.send(Ok(Bytes::from("hello "))).await.unwrap();
        tx.send(Ok(Bytes::from("world"))).await.unwrap();
        drop(tx);

        let collected = body.collect().await.unwrap().to_bytes();
        assert_eq!(collected, "hello world");
    }

    /// StreamingBody propagates errors from the reader task.
    #[tokio::test]
    async fn streaming_body_propagates_error() {
        let (tx, rx) = mpsc::channel(4);
        let body = StreamingBody { rx };

        tx.send(Ok(Bytes::from("partial"))).await.unwrap();
        tx.send(Err(StreamingBodyError("h3 broke".into()))).await.unwrap();
        drop(tx);

        let err = body.collect().await.unwrap_err();
        assert_eq!(err.to_string(), "h3 broke");
    }

    /// StreamingBody completes immediately when channel is closed empty.
    #[tokio::test]
    async fn streaming_body_empty() {
        let (tx, rx) = mpsc::channel::<Result<Bytes, StreamingBodyError>>(4);
        drop(tx);
        let body = StreamingBody { rx };

        let collected = body.collect().await.unwrap().to_bytes();
        assert!(collected.is_empty());
    }

    /// bad_gateway returns 502 with the given message.
    #[test]
    fn bad_gateway_response() {
        let resp = bad_gateway("upstream died");
        assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
        assert_eq!(resp.headers().get("content-type").unwrap(), "text/plain");
    }
}
