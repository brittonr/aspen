//! TCP port forwarding.
//!
//! Maps a local TCP port to a named remote service through iroh QUIC.
//! Wraps `DownstreamProxy` with `ProxyMode::Tcp`.

use std::net::SocketAddr;

use aspen_traits::KeyValueStore;
use snafu::Snafu;
use tracing::info;

use crate::auth::NetAuthError;
use crate::auth::NetAuthenticator;
use crate::resolver::NameResolver;
use crate::resolver::ResolverError;

/// Errors from port forwarding.
#[derive(Debug, Snafu)]
pub enum ForwardError {
    /// Service not found in registry.
    #[snafu(display("service '{name}' not found"))]
    ServiceNotFound { name: String },

    /// Authorization failed.
    #[snafu(display("auth error: {source}"))]
    Auth { source: NetAuthError },

    /// Resolver error.
    #[snafu(display("resolver error: {source}"))]
    Resolver { source: ResolverError },

    /// Invalid forward spec.
    #[snafu(display("invalid forward spec '{spec}': {reason}"))]
    InvalidSpec { spec: String, reason: String },

    /// I/O error (e.g., port already in use).
    #[snafu(display("io error: {source}"))]
    Io { source: std::io::Error },
}

/// Resolved forward target.
#[derive(Debug, Clone)]
pub struct ForwardTarget {
    /// Local address to bind.
    pub local_addr: SocketAddr,
    /// Resolved service name.
    pub service_name: String,
    /// Remote iroh endpoint ID.
    pub endpoint_id: String,
    /// Remote port.
    pub remote_port: u16,
}

/// Resolve and authorize a port forward.
///
/// Looks up the service, checks the token, and returns the resolved target.
/// The actual TCP listener + tunnel is created by the daemon with `DownstreamProxy`.
pub async fn resolve_forward<S: KeyValueStore + 'static>(
    local_addr: SocketAddr,
    service_name: &str,
    remote_port: Option<u16>,
    resolver: &NameResolver<S>,
    auth: &NetAuthenticator,
) -> Result<ForwardTarget, ForwardError> {
    // Resolve service name
    let (endpoint_id, registered_port) = resolver
        .resolve(service_name)
        .await
        .map_err(|e| ForwardError::Resolver { source: e })?
        .ok_or_else(|| ForwardError::ServiceNotFound {
            name: service_name.to_string(),
        })?;

    let port = remote_port.unwrap_or(registered_port);

    // Check authorization
    auth.check_connect(service_name, port).map_err(|e| ForwardError::Auth { source: e })?;

    info!("Forward {local_addr} -> {service_name} (endpoint={endpoint_id}, port={port})");

    Ok(ForwardTarget {
        local_addr,
        service_name: service_name.to_string(),
        endpoint_id,
        remote_port: port,
    })
}

#[derive(Clone, Copy)]
struct PortSpec<'a> {
    value: &'a str,
    spec: &'a str,
}

/// Parse a forward specification string.
///
/// Format: `[bind_addr:]local_port:service[:remote_port]`
///
/// Examples:
/// - `5432:mydb` → bind 127.0.0.1:5432, service "mydb", default remote port
/// - `8080:web:9090` → bind 127.0.0.1:8080, service "web", remote port 9090
/// - `0.0.0.0:5432:mydb` → bind all interfaces on 5432, service "mydb"
pub fn parse_forward_spec(spec: &str) -> Result<(SocketAddr, String, Option<u16>), ForwardError> {
    let parts: Vec<&str> = spec.split(':').collect();

    match parts.len() {
        // local_port:service
        2 => {
            let local_port = parse_port(PortSpec { value: parts[0], spec })?;
            let service = parts[1].to_string();
            let addr = SocketAddr::from(([127, 0, 0, 1], local_port));
            Ok((addr, service, None))
        }
        // local_port:service:remote_port  OR  bind_addr:local_port:service
        3 => {
            // Try local_port:service:remote_port first
            if let Ok(local_port) = parts[0].parse::<u16>()
                && let Ok(remote_port) = parts[2].parse::<u16>()
            {
                let service = parts[1].to_string();
                let addr = SocketAddr::from(([127, 0, 0, 1], local_port));
                return Ok((addr, service, Some(remote_port)));
            }
            // Try bind_addr:local_port:service
            let bind_addr: std::net::IpAddr = parts[0].parse().map_err(|_| ForwardError::InvalidSpec {
                spec: spec.to_string(),
                reason: format!("'{}' is not a valid IP address or port", parts[0]),
            })?;
            let local_port = parse_port(PortSpec { value: parts[1], spec })?;
            let service = parts[2].to_string();
            let addr = SocketAddr::new(bind_addr, local_port);
            Ok((addr, service, None))
        }
        // bind_addr:local_port:service:remote_port
        4 => {
            let bind_addr: std::net::IpAddr = parts[0].parse().map_err(|_| ForwardError::InvalidSpec {
                spec: spec.to_string(),
                reason: format!("'{}' is not a valid IP address", parts[0]),
            })?;
            let local_port = parse_port(PortSpec { value: parts[1], spec })?;
            let service = parts[2].to_string();
            let remote_port = parse_port(PortSpec { value: parts[3], spec })?;
            let addr = SocketAddr::new(bind_addr, local_port);
            Ok((addr, service, Some(remote_port)))
        }
        _ => Err(ForwardError::InvalidSpec {
            spec: spec.to_string(),
            reason: "expected [bind_addr:]local_port:service[:remote_port]".to_string(),
        }),
    }
}

fn parse_port(input: PortSpec<'_>) -> Result<u16, ForwardError> {
    input.value.parse().map_err(|_| ForwardError::InvalidSpec {
        spec: input.spec.to_string(),
        reason: format!("'{}' is not a valid port number", input.value),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_spec() {
        let (addr, service, remote_port) = parse_forward_spec("5432:mydb").unwrap();
        assert_eq!(addr, SocketAddr::from(([127, 0, 0, 1], 5432)));
        assert_eq!(service, "mydb");
        assert!(remote_port.is_none());
    }

    #[test]
    fn parse_with_remote_port() {
        let (addr, service, remote_port) = parse_forward_spec("8080:web:9090").unwrap();
        assert_eq!(addr, SocketAddr::from(([127, 0, 0, 1], 8080)));
        assert_eq!(service, "web");
        assert_eq!(remote_port, Some(9090));
    }

    #[test]
    fn parse_with_bind_addr() {
        let (addr, service, remote_port) = parse_forward_spec("0.0.0.0:5432:mydb").unwrap();
        assert_eq!(addr, SocketAddr::from(([0, 0, 0, 0], 5432)));
        assert_eq!(service, "mydb");
        assert!(remote_port.is_none());
    }

    #[test]
    fn parse_full_spec() {
        let (addr, service, remote_port) = parse_forward_spec("0.0.0.0:8080:web:9090").unwrap();
        assert_eq!(addr, SocketAddr::from(([0, 0, 0, 0], 8080)));
        assert_eq!(service, "web");
        assert_eq!(remote_port, Some(9090));
    }

    #[test]
    fn parse_invalid_spec() {
        assert!(parse_forward_spec("").is_err());
        assert!(parse_forward_spec("noport").is_err());
        assert!(parse_forward_spec("a:b:c:d:e").is_err());
    }
}
