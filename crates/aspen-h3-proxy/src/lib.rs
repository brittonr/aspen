//! TCP-to-iroh-h3 reverse proxy.
//!
//! Bridges HTTP/1.1 clients (browsers, curl, nix) to iroh endpoints serving
//! HTTP/3 over QUIC. Each incoming TCP request is forwarded as an h3 stream
//! to the configured iroh endpoint + ALPN.
//!
//! # Library Usage
//!
//! ```ignore
//! use aspen_h3_proxy::{H3Proxy, ProxyConfig};
//!
//! let config = ProxyConfig {
//!     bind_addr: "127.0.0.1".into(),
//!     port: 8080,
//!     endpoint_id: target_id,
//!     alpn: b"aspen/forge-web/1".to_vec(),
//!     ..Default::default()
//! };
//! let proxy = H3Proxy::new(config);
//! proxy.run().await?;
//! ```

pub mod bridge;
pub mod config;
pub mod connection;

use std::net::SocketAddr;
use std::sync::Arc;

pub use config::ProxyConfig;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tracing::error;
use tracing::info;

use crate::connection::ConnectionPool;

/// Tiger Style: max concurrent TCP connections the proxy accepts.
const MAX_CONCURRENT_CONNECTIONS: usize = 500;

/// TCP-to-iroh-h3 reverse proxy.
///
/// Listens for HTTP/1.1 on TCP and forwards each request as an HTTP/3
/// stream to the configured iroh endpoint.
pub struct H3Proxy {
    config: ProxyConfig,
}

impl H3Proxy {
    /// Create a new proxy with the given configuration.
    pub fn new(config: ProxyConfig) -> Self {
        Self { config }
    }

    /// Run the proxy until shutdown (ctrl-c or cancellation).
    ///
    /// Binds a TCP listener and forwards all incoming HTTP/1.1 requests
    /// to the iroh endpoint over h3.
    pub async fn run(&self) -> anyhow::Result<()> {
        let addr: SocketAddr = format!("{}:{}", self.config.bind_addr, self.config.port).parse()?;
        let listener = TcpListener::bind(addr).await?;
        let local_addr = listener.local_addr()?;

        info!(addr = %local_addr, alpn = %String::from_utf8_lossy(&self.config.alpn), "h3 proxy listening");

        let endpoint = iroh::Endpoint::builder(iroh::endpoint::presets::N0).bind().await?;

        let pool = Arc::new(ConnectionPool::new(
            Arc::new(endpoint),
            self.config.endpoint_id,
            self.config.alpn.clone(),
            self.config.request_timeout,
        ));

        let semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_CONNECTIONS));
        let shutdown = tokio::signal::ctrl_c();
        tokio::pin!(shutdown);

        loop {
            tokio::select! {
                biased;
                _ = &mut shutdown => {
                    info!("proxy shutting down");
                    break;
                }
                accept = listener.accept() => {
                    match accept {
                        Ok((stream, remote)) => {
                            let pool = Arc::clone(&pool);
                            let permit = match semaphore.clone().try_acquire_owned() {
                                Ok(p) => p,
                                Err(_) => {
                                    tracing::warn!(remote = %remote, "too many connections, rejecting");
                                    continue;
                                }
                            };

                            tokio::spawn(async move {
                                let _permit = permit;
                                let io = TokioIo::new(stream);
                                let pool = Arc::clone(&pool);
                                let svc = service_fn(move |req| {
                                    bridge::handle_request(Arc::clone(&pool), req)
                                });

                                if let Err(e) = http1::Builder::new()
                                    .serve_connection(io, svc)
                                    .await
                                    && !e.is_incomplete_message()
                                {
                                    tracing::warn!(error = %e, "connection error");
                                }
                            });
                        }
                        Err(e) => {
                            error!(error = %e, "accept failed");
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Get the bind address this proxy will use.
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.config.bind_addr, self.config.port)
    }
}
