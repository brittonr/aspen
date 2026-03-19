//! HTTP/3 protocol handler for the forge web frontend.
//!
//! Implements `ProtocolHandler` directly using `h3` + `iroh-h3`,
//! no axum dependency needed.

use std::sync::Arc;

use bytes::Buf;
use bytes::Bytes;
use h3::server::RequestResolver;
use http::Response;
use iroh::protocol::AcceptError;
use iroh::protocol::ProtocolHandler;
use iroh_h3::Connection as IrohH3Connection;
use tracing::debug;
use tracing::warn;

use crate::routes;
use crate::state::AppState;

type H3Conn = h3::server::Connection<IrohH3Connection, Bytes>;

/// HTTP/3 handler that serves the forge web UI.
pub struct ForgeH3Handler {
    state: Arc<AppState>,
}

impl std::fmt::Debug for ForgeH3Handler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ForgeH3Handler").finish()
    }
}

impl ForgeH3Handler {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

impl ProtocolHandler for ForgeH3Handler {
    async fn accept(&self, conn: iroh::endpoint::Connection) -> Result<(), AcceptError> {
        let h3_conn = IrohH3Connection::new(conn);
        let mut server = H3Conn::new(h3_conn).await.map_err(AcceptError::from_err)?;

        while let Some(resolver) = server.accept().await.map_err(AcceptError::from_err)? {
            let state = self.state.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_request(state, resolver).await {
                    warn!("request error: {e:#}");
                }
            });
        }

        Ok(())
    }
}

/// Handle a single HTTP/3 request.
async fn handle_request(
    state: Arc<AppState>,
    resolver: RequestResolver<IrohH3Connection, Bytes>,
) -> anyhow::Result<()> {
    let (req, mut stream) = resolver.resolve_request().await?;

    let (parts, _body) = req.into_parts();
    let method = parts.method.clone();
    let path = parts.uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/");

    debug!(%method, %path, "forge-web request");

    // Read POST body (bounded to 64 KB for form submissions).
    let body_bytes = if method == http::Method::POST {
        let mut buf = Vec::new();
        while let Some(chunk) = stream.recv_data().await? {
            buf.extend_from_slice(chunk.chunk());
            if buf.len() > 64 * 1024 {
                break;
            }
        }
        Some(Bytes::from(buf))
    } else {
        None
    };

    let route_resp = match method {
        ref m if *m == http::Method::GET => routes::dispatch(&state, path, None).await,
        ref m if *m == http::Method::POST => routes::dispatch(&state, path, body_bytes.as_ref()).await,
        _ => routes::method_not_allowed(),
    };

    let resp = Response::builder()
        .status(route_resp.status())
        .header("content-type", route_resp.content_type())
        .body(())
        .map_err(|e| anyhow::anyhow!("response build error: {e}"))?;

    stream.send_response(resp).await?;
    stream.send_data(Bytes::from(route_resp.into_bytes())).await?;
    stream.finish().await?;

    Ok(())
}
