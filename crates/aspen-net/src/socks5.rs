//! SOCKS5 proxy server (RFC 1928).
//!
//! A minimal SOCKS5 implementation that resolves `*.aspen` service names
//! via the registry and creates CONNECT tunnels through iroh QUIC.
//! Only supports CONNECT command with domain name address type.

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;

use aspen_traits::KeyValueStore;
use snafu::ResultExt;
use snafu::Snafu;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::time;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::auth::NetAuthError;
use crate::auth::NetAuthenticator;
use crate::constants::MAX_SOCKS5_CONNECTIONS;
use crate::constants::SOCKS5_HANDSHAKE_TIMEOUT_SECS;
use crate::resolver::NameResolver;
use crate::resolver::ResolverError;
use crate::tunnel::TunnelError;

/// SOCKS5 protocol constants.
const SOCKS5_VERSION: u8 = 0x05;
const CMD_CONNECT: u8 = 0x01;
const ATYP_DOMAIN: u8 = 0x03;
const ATYP_IPV4: u8 = 0x01;
const ATYP_IPV6: u8 = 0x04;

/// SOCKS5 reply codes.
const REPLY_SUCCESS: u8 = 0x00;
#[allow(dead_code)]
const REPLY_GENERAL_FAILURE: u8 = 0x01;
const REPLY_HOST_UNREACHABLE: u8 = 0x04;
const REPLY_CONNECTION_REFUSED: u8 = 0x05;
const REPLY_CMD_NOT_SUPPORTED: u8 = 0x07;

/// Errors from SOCKS5 server.
#[derive(Debug, Snafu)]
pub enum Socks5Error {
    /// Connection limit reached.
    #[snafu(display("connection limit reached ({max})"))]
    ConnectionLimitReached { max: u32 },

    /// I/O error.
    #[snafu(display("io error: {source}"))]
    Io { source: std::io::Error },

    /// Protocol error.
    #[snafu(display("socks5 protocol error: {reason}"))]
    Protocol { reason: String },

    /// Resolver error.
    #[snafu(display("resolver error: {source}"))]
    Resolver { source: ResolverError },

    /// Auth error.
    #[snafu(display("auth error: {source}"))]
    Auth { source: NetAuthError },

    /// Tunnel error.
    #[snafu(display("tunnel error: {source}"))]
    Tunnel { source: TunnelError },
}

struct ConnectionContext<'a, S: KeyValueStore> {
    resolver: &'a NameResolver<S>,
    auth: &'a NetAuthenticator,
    endpoint: &'a iroh::Endpoint,
    cancel: tokio_util::sync::CancellationToken,
}

/// SOCKS5 proxy server.
///
/// Accepts TCP connections, performs SOCKS5 handshake, resolves
/// `*.aspen` names, and tunnels traffic through iroh QUIC.
pub struct Socks5Server<S: KeyValueStore> {
    resolver: Arc<NameResolver<S>>,
    authenticator: Arc<NetAuthenticator>,
    endpoint: Arc<iroh::Endpoint>,
    active_connections: Arc<AtomicU32>,
    cancel: tokio_util::sync::CancellationToken,
}

impl<S: KeyValueStore + 'static> Socks5Server<S> {
    /// Create a new SOCKS5 server with an iroh endpoint for tunneling.
    pub fn new(
        resolver: Arc<NameResolver<S>>,
        authenticator: Arc<NetAuthenticator>,
        endpoint: Arc<iroh::Endpoint>,
        cancel: tokio_util::sync::CancellationToken,
    ) -> Self {
        Self {
            resolver,
            authenticator,
            endpoint,
            active_connections: Arc::new(AtomicU32::new(0)),
            cancel,
        }
    }

    /// Run the SOCKS5 server, accepting connections on the given listener.
    pub async fn run(&self, listener: TcpListener) -> Result<(), Socks5Error> {
        info!("SOCKS5 proxy listening on {}", listener.local_addr().context(IoSnafu)?);

        loop {
            tokio::select! {
                _ = self.cancel.cancelled() => {
                    info!("SOCKS5 server shutting down");
                    break;
                }
                accept = listener.accept() => {
                    let (stream, addr) = accept.context(IoSnafu)?;

                    // Check connection limit
                    let count = self.active_connections.load(Ordering::Relaxed);
                    if count >= MAX_SOCKS5_CONNECTIONS {
                        warn!("SOCKS5 connection limit reached ({count}), rejecting");
                        drop(stream);
                        continue;
                    }

                    self.active_connections.fetch_add(1, Ordering::Relaxed);
                    let resolver = Arc::clone(&self.resolver);
                    let auth = Arc::clone(&self.authenticator);
                    let endpoint = Arc::clone(&self.endpoint);
                    let counter = Arc::clone(&self.active_connections);
                    let cancel = self.cancel.clone();

                    tokio::spawn(async move {
                        let context = ConnectionContext {
                            resolver: resolver.as_ref(),
                            auth: auth.as_ref(),
                            endpoint: endpoint.as_ref(),
                            cancel,
                        };
                        if let Err(e) = handle_connection(stream, addr, context).await {
                            debug!("SOCKS5 connection from {addr} error: {e}");
                        }
                        counter.fetch_sub(1, Ordering::Relaxed);
                    });
                }
            }
        }

        Ok(())
    }

    /// Get the current number of active connections.
    pub fn active_connection_count(&self) -> u32 {
        self.active_connections.load(Ordering::Relaxed)
    }
}

/// Handle a single SOCKS5 connection.
async fn handle_connection<S: KeyValueStore + 'static>(
    mut stream: TcpStream,
    addr: SocketAddr,
    context: ConnectionContext<'_, S>,
) -> Result<(), Socks5Error> {
    // Wrap handshake in a timeout
    let handshake_result = time::timeout(
        std::time::Duration::from_secs(SOCKS5_HANDSHAKE_TIMEOUT_SECS),
        perform_handshake(&mut stream, addr, context.resolver, context.auth),
    )
    .await;

    let (domain, _port, endpoint_id, remote_port) = match handshake_result {
        Ok(Ok(result)) => result,
        Ok(Err(e)) => return Err(e),
        Err(_) => {
            debug!("SOCKS5 handshake timeout from {addr}");
            return Err(Socks5Error::Protocol {
                reason: "handshake timeout".to_string(),
            });
        }
    };
    debug_assert!(domain.ends_with(".aspen"), "validated SOCKS5 domain must end with .aspen");
    debug_assert!(!endpoint_id.is_empty(), "resolved SOCKS5 endpoint ID must not be empty");

    // Parse the endpoint ID and build an EndpointAddr for iroh connection
    let remote_id: iroh::EndpointId = match endpoint_id.parse() {
        Ok(id) => id,
        Err(e) => {
            warn!("SOCKS5 invalid endpoint id '{endpoint_id}': {e}");
            send_reply(&mut stream, REPLY_HOST_UNREACHABLE).await?;
            return Err(Socks5Error::Protocol {
                reason: format!("invalid endpoint id '{endpoint_id}': {e}"),
            });
        }
    };
    let remote_addr = iroh::EndpointAddr::new(remote_id);

    // Open the tunnel through iroh QUIC
    let (mut send, mut recv) = match crate::tunnel::open_tunnel(context.endpoint, remote_addr, remote_port).await {
        Ok(streams) => streams,
        Err(e) => {
            warn!(
                addr = %addr,
                domain = %domain,
                error = %e,
                "SOCKS5 tunnel connection failed"
            );
            send_reply(&mut stream, REPLY_HOST_UNREACHABLE).await?;
            return Err(Socks5Error::Tunnel { source: e });
        }
    };

    // Send success response — tunnel is established
    send_reply(&mut stream, REPLY_SUCCESS).await?;

    debug!("SOCKS5 tunnel established: {addr} -> {domain}:{remote_port} (endpoint={endpoint_id})");

    // Bidirectional copy between TCP socket and QUIC stream
    let (mut tcp_read, mut tcp_write) = stream.split();

    tokio::select! {
        _ = context.cancel.cancelled() => {
            debug!("SOCKS5 tunnel cancelled: {addr} -> {domain}");
        }
        result = async {
            let c2s = tokio::io::copy(&mut tcp_read, &mut send);
            let s2c = tokio::io::copy(&mut recv, &mut tcp_write);
            tokio::try_join!(c2s, s2c)
        } => {
            match result {
                Ok((c2s, s2c)) => {
                    debug!("SOCKS5 tunnel closed: {addr} -> {domain} (c2s={c2s}, s2c={s2c})");
                }
                Err(e) => {
                    debug!("SOCKS5 tunnel error: {addr} -> {domain}: {e}");
                }
            }
        }
    }

    // Best-effort shutdown
    if let Err(error) = send.finish() {
        debug!(addr = %addr, error = ?error, "SOCKS5 tunnel finish failed");
    }

    Ok(())
}

/// Perform the SOCKS5 handshake: greeting, auth negotiation, CONNECT request.
async fn perform_handshake<S: KeyValueStore + 'static>(
    stream: &mut TcpStream,
    addr: SocketAddr,
    resolver: &NameResolver<S>,
    auth: &NetAuthenticator,
) -> Result<(String, u16, String, u16), Socks5Error> {
    // === Greeting ===
    let version = read_u8(stream).await?;
    if version != SOCKS5_VERSION {
        return Err(Socks5Error::Protocol {
            reason: format!("unsupported version: {version}"),
        });
    }

    let nmethods = read_u8(stream).await?;
    let mut methods = vec![0u8; usize::from(nmethods)];
    stream.read_exact(&mut methods).await.context(IoSnafu)?;

    // Reply: no auth required
    stream.write_all(&[SOCKS5_VERSION, 0x00]).await.context(IoSnafu)?;

    // === CONNECT request ===
    let version = read_u8(stream).await?;
    if version != SOCKS5_VERSION {
        return Err(Socks5Error::Protocol {
            reason: format!("request version mismatch: {version}"),
        });
    }

    let cmd = read_u8(stream).await?;
    let _reserved = read_u8(stream).await?;
    let atyp = read_u8(stream).await?;

    // Only support CONNECT command
    if cmd != CMD_CONNECT {
        send_reply(stream, REPLY_CMD_NOT_SUPPORTED).await?;
        return Err(Socks5Error::Protocol {
            reason: format!("unsupported command: {cmd} (only CONNECT supported)"),
        });
    }

    // Only support domain name address type
    if atyp == ATYP_IPV4 || atyp == ATYP_IPV6 {
        send_reply(stream, REPLY_CONNECTION_REFUSED).await?;
        return Err(Socks5Error::Protocol {
            reason: "IPv4/IPv6 addresses not supported, use domain names".to_string(),
        });
    }

    if atyp != ATYP_DOMAIN {
        send_reply(stream, REPLY_CONNECTION_REFUSED).await?;
        return Err(Socks5Error::Protocol {
            reason: format!("unsupported address type: {atyp}"),
        });
    }

    // Read domain name
    let domain_len = usize::from(read_u8(stream).await?);
    let mut domain_bytes = vec![0u8; domain_len];
    stream.read_exact(&mut domain_bytes).await.context(IoSnafu)?;
    debug_assert_eq!(domain_bytes.len(), domain_len, "SOCKS5 domain buffer length must match declared length");
    let domain = String::from_utf8_lossy(&domain_bytes).to_string();

    // Read port
    let requested_endpoint = read_u16(stream).await?;

    debug!("SOCKS5 CONNECT from {addr}: {domain}:{requested_endpoint}");

    // Reject non-.aspen domains
    if !domain.ends_with(".aspen") {
        warn!("SOCKS5 rejected non-.aspen domain: {domain}");
        send_reply(stream, REPLY_CONNECTION_REFUSED).await?;
        return Err(Socks5Error::Protocol {
            reason: format!("non-.aspen destination not supported: {domain}"),
        });
    }

    // Resolve service name (strips .aspen suffix)
    let service_name = domain.strip_suffix(".aspen").unwrap_or(&domain);
    let resolved = resolver.resolve(service_name).await.context(ResolverSnafu)?;

    let (endpoint_id, _registered_port) = match resolved {
        Some(r) => r,
        None => {
            send_reply(stream, REPLY_HOST_UNREACHABLE).await?;
            return Err(Socks5Error::Protocol {
                reason: format!("service not found: {service_name}"),
            });
        }
    };

    // Token authorization check uses client-specified port (per spec: explicit CONNECT override)
    if let Err(e) = auth.check_connect(service_name, requested_endpoint) {
        send_reply(stream, REPLY_CONNECTION_REFUSED).await?;
        return Err(Socks5Error::Auth { source: e });
    }

    Ok((domain, requested_endpoint, endpoint_id, requested_endpoint))
}

/// Send a SOCKS5 reply.
async fn send_reply(stream: &mut TcpStream, reply_code: u8) -> Result<(), Socks5Error> {
    let response = [
        SOCKS5_VERSION,
        reply_code,
        0x00, // reserved
        0x01, // IPv4
        0,
        0,
        0,
        0, // 0.0.0.0
        0,
        0, // port 0
    ];
    stream.write_all(&response).await.context(IoSnafu)?;
    stream.flush().await.context(IoSnafu)?;
    Ok(())
}

/// Read a single byte from the stream.
async fn read_u8(stream: &mut TcpStream) -> Result<u8, Socks5Error> {
    let mut buf = [0u8; 1];
    stream.read_exact(&mut buf).await.context(IoSnafu)?;
    Ok(buf[0])
}

/// Read a big-endian u16 from the stream.
async fn read_u16(stream: &mut TcpStream) -> Result<u16, Socks5Error> {
    let mut buf = [0u8; 2];
    stream.read_exact(&mut buf).await.context(IoSnafu)?;
    Ok(u16::from_be_bytes(buf))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socks5_greeting_bytes() {
        let greeting = [0x05, 0x01, 0x00];
        assert_eq!(greeting[0], SOCKS5_VERSION);
        assert_eq!(greeting[1], 1);
        assert_eq!(greeting[2], 0x00);
    }

    #[test]
    fn socks5_connect_request_bytes() {
        let header = [0x05, CMD_CONNECT, 0x00, ATYP_DOMAIN];
        assert_eq!(header[0], SOCKS5_VERSION);
        assert_eq!(header[1], CMD_CONNECT);
        assert_eq!(header[3], ATYP_DOMAIN);
    }

    #[test]
    fn domain_extraction() {
        let domain = "mydb.aspen";
        let service_name = domain.strip_suffix(".aspen").unwrap();
        assert_eq!(service_name, "mydb");
    }

    #[test]
    fn reply_codes() {
        assert_eq!(REPLY_SUCCESS, 0x00);
        assert_eq!(REPLY_GENERAL_FAILURE, 0x01);
        assert_eq!(REPLY_HOST_UNREACHABLE, 0x04);
        assert_eq!(REPLY_CONNECTION_REFUSED, 0x05);
        assert_eq!(REPLY_CMD_NOT_SUPPORTED, 0x07);
    }
}
