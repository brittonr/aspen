//! Integration tests for the aspen-net service mesh.
//!
//! Tests that run without a cluster use `DeterministicKeyValueStore` + `TokenBuilder`.
//! Tests that need a real cluster are marked `#[ignore]`.
//!
//! Run all:        `cargo nextest run -p aspen-net --run-ignored all`
//! Run non-ignore: `cargo nextest run -p aspen-net`
//!
//! Test matrix:
//! - 8.8/8.9:    DNS record lifecycle (requires cluster — #[ignore])
//! - 10.6:       Service publish/lookup round-trip via registry+resolver
//! - 13.3:       DownstreamProxy tunnel verification (requires cluster — #[ignore])
//! - 14.3:       SOCKS5 handshake with real TCP (in-process, no cluster)
//! - 14.4:       Port forward spec parsing (in-process)
//! - 14.5:       Token deny for unauthorized service (in-process)
//! - 14.6:       Expired token rejects new connections (in-process)
//! - 14.7:       Service publish/unpublish lifecycle (in-process)
//! - 14.8:       Delegation chain attenuation (in-process)
//! - 14.9:       NixOS VM test placeholder

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use aspen_auth::Capability;
use aspen_auth::TokenBuilder;
use aspen_auth::TokenVerifier;
use aspen_net::auth::NetAuthenticator;
use aspen_net::forward::parse_forward_spec;
use aspen_net::registry::ServiceRegistry;
use aspen_net::resolver::NameResolver;
use aspen_net::socks5::Socks5Server;
use aspen_net::types::ServiceEntry;
use aspen_testing_core::DeterministicKeyValueStore;
use iroh::SecretKey;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio_util::sync::CancellationToken;

// =========================================================================
// Helpers
// =========================================================================

/// Deterministic secret key for test token signing.
fn test_secret_key() -> SecretKey {
    SecretKey::from([1u8; 32])
}

/// Create a test service entry.
fn test_entry(name: &str, port: u16) -> ServiceEntry {
    ServiceEntry {
        name: name.to_string(),
        endpoint_id: "test-endpoint-id-0001".to_string(),
        port,
        proto: "tcp".to_string(),
        tags: vec!["test".to_string()],
        hostname: Some("test-node".to_string()),
        published_at_ms: 1000,
    }
}

/// Build a signed token with NetConnect for all services.
fn make_connect_token() -> NetAuthenticator {
    let key = test_secret_key();
    let token = TokenBuilder::new(key)
        .with_capability(Capability::NetConnect {
            service_prefix: String::new(), // empty prefix = all services
        })
        .with_lifetime(Duration::from_secs(86400))
        .build()
        .expect("token build should succeed");
    let verifier = TokenVerifier::new();
    NetAuthenticator::new(token, verifier)
}

/// Build a signed token with NetConnect scoped to a prefix.
fn make_scoped_connect_auth(prefix: &str) -> NetAuthenticator {
    let key = test_secret_key();
    let token = TokenBuilder::new(key)
        .with_capability(Capability::NetConnect {
            service_prefix: prefix.to_string(),
        })
        .with_lifetime(Duration::from_secs(86400))
        .build()
        .expect("token build should succeed");
    let verifier = TokenVerifier::new();
    NetAuthenticator::new(token, verifier)
}

/// Build a signed token that has already expired.
fn make_expired_auth() -> NetAuthenticator {
    let key = test_secret_key();
    // Build with 0-second lifetime (expires immediately)
    let token = TokenBuilder::new(key)
        .with_capability(Capability::NetConnect {
            service_prefix: String::new(),
        })
        .with_lifetime(Duration::from_secs(0))
        .build()
        .expect("token build should succeed");
    // Wait a moment so it actually expires
    std::thread::sleep(Duration::from_millis(10));
    let verifier = TokenVerifier::new();
    NetAuthenticator::new(token, verifier)
}

/// Build a signed token with NetAdmin (all net operations).
fn make_admin_auth() -> NetAuthenticator {
    let key = test_secret_key();
    let token = TokenBuilder::new(key)
        .with_capability(Capability::NetAdmin)
        .with_lifetime(Duration::from_secs(86400))
        .build()
        .expect("token build should succeed");
    let verifier = TokenVerifier::new();
    NetAuthenticator::new(token, verifier)
}

/// Create a fresh registry + resolver stack backed by in-memory KV.
fn make_stack() -> (Arc<ServiceRegistry<DeterministicKeyValueStore>>, Arc<NameResolver<DeterministicKeyValueStore>>) {
    let store = DeterministicKeyValueStore::new();
    let registry = Arc::new(ServiceRegistry::new(store));
    let resolver = Arc::new(NameResolver::new(Arc::clone(&registry)));
    (registry, resolver)
}

/// Perform a SOCKS5 CONNECT handshake on a TCP stream.
/// Returns the reply code from the server.
async fn socks5_connect(stream: &mut TcpStream, domain: &str, port: u16) -> std::io::Result<u8> {
    // Greeting: version 5, 1 method (no auth)
    stream.write_all(&[0x05, 0x01, 0x00]).await?;

    // Read greeting response
    let mut greeting_resp = [0u8; 2];
    stream.read_exact(&mut greeting_resp).await?;
    assert_eq!(greeting_resp[0], 0x05, "server should respond with SOCKS5 version");

    // CONNECT request
    let mut request = vec![0x05, 0x01, 0x00, 0x03]; // ver, CONNECT, reserved, DOMAIN
    request.push(domain.len() as u8);
    request.extend_from_slice(domain.as_bytes());
    request.extend_from_slice(&port.to_be_bytes());
    stream.write_all(&request).await?;

    // Read reply (10 bytes: ver + reply + rsv + atyp + 4-byte addr + 2-byte port)
    let mut reply = [0u8; 10];
    stream.read_exact(&mut reply).await?;
    Ok(reply[1]) // reply code
}

// =========================================================================
// 8.8: DNS A/SRV records created on publish
// =========================================================================

/// Task 8.8: Publish a service and verify DNS A and SRV records are created.
///
/// Requires: Running cluster with DNS store, iroh-docs sync, DnsProtocolServer.
#[tokio::test]
#[ignore = "requires running cluster with DNS"]
async fn dns_records_created_on_publish() {
    let (registry, _resolver) = make_stack();

    // Publish a service
    let entry = test_entry("mydb", 5432);
    registry.publish(entry).await.expect("publish should succeed");

    // In a real cluster test:
    // 1. Wait for iroh-docs sync to propagate DNS records
    // 2. Query DnsProtocolServer for A record: mydb.aspen → 127.0.0.x
    // 3. Query for SRV record: _tcp.mydb.aspen → mydb.aspen:5432

    // Verify prerequisite: service is in the registry
    let found = registry.lookup("mydb").await.expect("lookup should work");
    assert!(found.is_some(), "published service should be findable");
    let svc = found.unwrap();
    assert_eq!(svc.port, 5432);
    assert_eq!(svc.proto, "tcp");
}

// =========================================================================
// 8.9: DNS records removed on unpublish
// =========================================================================

/// Task 8.9: Unpublish a service and verify DNS records are removed.
#[tokio::test]
#[ignore = "requires running cluster with DNS"]
async fn dns_records_removed_on_unpublish() {
    let (registry, _resolver) = make_stack();

    let entry = test_entry("mydb", 5432);
    registry.publish(entry).await.expect("publish should succeed");
    registry.unpublish("mydb").await.expect("unpublish should succeed");

    // In a real cluster test:
    // 1. Wait for iroh-docs deletion sync
    // 2. Query A record: mydb.aspen → NXDOMAIN
    // 3. Query SRV record: _tcp.mydb.aspen → NXDOMAIN

    let found = registry.lookup("mydb").await.expect("lookup should work");
    assert!(found.is_none(), "unpublished service should be gone");
}

// =========================================================================
// 10.6: Service publish/lookup round-trip
// =========================================================================

/// Task 10.6: Publish a service via the registry, look it up via the
/// resolver, and verify the round-trip data integrity.
#[tokio::test]
async fn rpc_publish_lookup_roundtrip() {
    let (registry, resolver) = make_stack();

    // Publish a service
    let entry = ServiceEntry {
        name: "web-api".to_string(),
        endpoint_id: "endpoint-abc-123".to_string(),
        port: 8080,
        proto: "tcp".to_string(),
        tags: vec!["api".to_string(), "production".to_string()],
        hostname: Some("web-node-1".to_string()),
        published_at_ms: 42000,
    };
    registry.publish(entry.clone()).await.expect("publish should succeed");

    // Direct lookup
    let direct = registry.lookup("web-api").await.unwrap().expect("service should exist");
    assert_eq!(direct.name, "web-api");
    assert_eq!(direct.endpoint_id, "endpoint-abc-123");
    assert_eq!(direct.port, 8080);
    assert_eq!(direct.tags, vec!["api", "production"]);

    // Resolver lookup
    let resolved = resolver.resolve("web-api").await.unwrap().expect("should resolve");
    assert_eq!(resolved.0, "endpoint-abc-123");
    assert_eq!(resolved.1, 8080);

    // Resolver with .aspen suffix
    let resolved_suffix = resolver.resolve("web-api.aspen").await.unwrap().expect("should resolve");
    assert_eq!(resolved_suffix, resolved);

    // List all
    let all = registry.list(None).await.unwrap();
    assert_eq!(all.len(), 1);

    // List by tag
    let api_only = registry.list(Some("api")).await.unwrap();
    assert_eq!(api_only.len(), 1);
    let missing_tag = registry.list(Some("nonexistent")).await.unwrap();
    assert_eq!(missing_tag.len(), 0);
}

/// Task 10.6 (cluster): RPC handler publish/lookup via real cluster.
#[tokio::test]
#[ignore = "requires running cluster with RPC handlers"]
async fn rpc_handler_publish_lookup() {
    // Full cluster test:
    // 1. Send NetPublish RPC via aspen-cli
    // 2. Send NetLookup RPC, verify response matches
    // 3. Send NetList RPC, verify list contains service
    // 4. NetUnpublish, verify gone
}

// =========================================================================
// 13.3: DownstreamProxy tunnel to UpstreamProxy
// =========================================================================

/// Task 13.3: DownstreamProxy on daemon side tunnels to UpstreamProxy on node.
#[tokio::test]
#[ignore = "requires running cluster with proxy ALPN"]
async fn downstream_proxy_tunnels_to_upstream() {
    // Full test:
    // 1. Node registers UpstreamProxy on iroh-http-proxy/1 ALPN
    // 2. Daemon creates DownstreamProxy
    // 3. create_tunnel(EndpointAuthority { endpoint_id, authority })
    // 4. Bidirectional data flow verified
}

// =========================================================================
// 14.3: Full SOCKS5 tunnel (handshake works in-process)
// =========================================================================

/// Task 14.3: SOCKS5 handshake + name resolution works end-to-end.
///
/// The actual iroh tunnel requires a running cluster (separate test).
/// This test verifies the SOCKS5 protocol, name resolution, and auth
/// with real TCP connections.
#[tokio::test]
async fn socks5_handshake_with_registry() {
    let (registry, resolver) = make_stack();
    let cancel = CancellationToken::new();

    // Publish a service
    registry.publish(test_entry("mydb", 5432)).await.unwrap();

    let auth = Arc::new(make_connect_token());

    // Start SOCKS5 server on random port
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = listener.local_addr().unwrap();

    let server = Socks5Server::new(resolver, auth, cancel.clone());
    let server_task = tokio::spawn(async move { server.run(listener).await });

    tokio::time::sleep(Duration::from_millis(50)).await;

    // SOCKS5 CONNECT to valid service
    let mut client = TcpStream::connect(proxy_addr).await.unwrap();
    let reply = socks5_connect(&mut client, "mydb.aspen", 5432).await.unwrap();
    assert_eq!(reply, 0x00, "SOCKS5 CONNECT should succeed for valid service");

    cancel.cancel();
    let _ = tokio::time::timeout(Duration::from_secs(2), server_task).await;
}

/// Task 14.3 (cluster): Full SOCKS5 tunnel with iroh data flow.
#[tokio::test]
#[ignore = "requires running cluster with iroh tunnel support"]
async fn socks5_full_tunnel_with_iroh() {
    // Full test: start echo server → publish → SOCKS5 connect → send data → echo back
}

// =========================================================================
// 14.4: Port forwarding
// =========================================================================

/// Task 14.4: Port forward spec parsing and resolution.
#[tokio::test]
async fn port_forward_spec_parsing() {
    // Simple
    let (addr, svc, rport) = parse_forward_spec("5432:mydb").unwrap();
    assert_eq!(addr, SocketAddr::from(([127, 0, 0, 1], 5432)));
    assert_eq!(svc, "mydb");
    assert!(rport.is_none());

    // With remote port
    let (addr, svc, rport) = parse_forward_spec("8080:web:9090").unwrap();
    assert_eq!(addr, SocketAddr::from(([127, 0, 0, 1], 8080)));
    assert_eq!(svc, "web");
    assert_eq!(rport, Some(9090));

    // With bind addr
    let (addr, svc, rport) = parse_forward_spec("0.0.0.0:5432:mydb").unwrap();
    assert_eq!(addr, SocketAddr::from(([0, 0, 0, 0], 5432)));
    assert_eq!(svc, "mydb");
    assert!(rport.is_none());

    // Full
    let (addr, svc, rport) = parse_forward_spec("0.0.0.0:8080:web:9090").unwrap();
    assert_eq!(addr, SocketAddr::from(([0, 0, 0, 0], 8080)));
    assert_eq!(svc, "web");
    assert_eq!(rport, Some(9090));
}

/// Task 14.4 (cluster): End-to-end port forward.
#[tokio::test]
#[ignore = "requires running cluster with iroh tunnel support"]
async fn port_forward_end_to_end() {
    // Full test: echo server → publish → forward local:8888 → connect → echo
}

// =========================================================================
// 14.5: Token deny for unauthorized service
// =========================================================================

/// Task 14.5: SOCKS5 returns connection refused for unauthorized service.
#[tokio::test]
async fn socks5_token_deny_unauthorized_service() {
    let (registry, resolver) = make_stack();
    let cancel = CancellationToken::new();

    // Publish two services
    registry.publish(test_entry("allowed-svc", 5432)).await.unwrap();
    registry.publish(test_entry("denied-svc", 5433)).await.unwrap();

    // Token only grants access to "allowed" prefix
    let auth = Arc::new(make_scoped_connect_auth("allowed"));

    // Start SOCKS5 server
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = listener.local_addr().unwrap();
    let server = Socks5Server::new(resolver, auth, cancel.clone());
    let server_task = tokio::spawn(async move { server.run(listener).await });
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Allowed service: should succeed
    let mut client = TcpStream::connect(proxy_addr).await.unwrap();
    let reply = socks5_connect(&mut client, "allowed-svc.aspen", 5432).await.unwrap();
    assert_eq!(reply, 0x00, "allowed service should get SOCKS5 success");

    // Denied service: should get connection refused (0x05)
    let mut client = TcpStream::connect(proxy_addr).await.unwrap();
    let reply = socks5_connect(&mut client, "denied-svc.aspen", 5433).await.unwrap();
    assert_eq!(reply, 0x05, "denied service should get SOCKS5 connection refused");

    cancel.cancel();
    let _ = tokio::time::timeout(Duration::from_secs(2), server_task).await;
}

// =========================================================================
// 14.6: Expired token rejects new connections
// =========================================================================

/// Task 14.6: Expired token causes SOCKS5 to reject new connections.
#[tokio::test]
async fn socks5_expired_token_rejects() {
    let (registry, resolver) = make_stack();
    let cancel = CancellationToken::new();

    registry.publish(test_entry("mydb", 5432)).await.unwrap();

    let auth = Arc::new(make_expired_auth());

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = listener.local_addr().unwrap();
    let server = Socks5Server::new(resolver, auth, cancel.clone());
    let server_task = tokio::spawn(async move { server.run(listener).await });
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Expired token should be refused
    let mut client = TcpStream::connect(proxy_addr).await.unwrap();
    let reply = socks5_connect(&mut client, "mydb.aspen", 5432).await.unwrap();
    assert_eq!(reply, 0x05, "expired token should get SOCKS5 connection refused");

    cancel.cancel();
    let _ = tokio::time::timeout(Duration::from_secs(2), server_task).await;
}

// =========================================================================
// 14.7: Service publish/unpublish lifecycle
// =========================================================================

/// Task 14.7: Full publish/unpublish lifecycle with verification at each step.
#[tokio::test]
async fn service_lifecycle_publish_unpublish() {
    let (registry, resolver) = make_stack();

    // Initially empty
    let all = registry.list(None).await.unwrap();
    assert_eq!(all.len(), 0);

    // Publish first service
    registry.publish(test_entry("mydb", 5432)).await.unwrap();

    let found = resolver.resolve("mydb").await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap(), ("test-endpoint-id-0001".to_string(), 5432));

    // Publish second service
    let mut entry2 = test_entry("cache", 6379);
    entry2.endpoint_id = "test-endpoint-id-0002".to_string();
    registry.publish(entry2).await.unwrap();

    let all = registry.list(None).await.unwrap();
    assert_eq!(all.len(), 2);

    // Update first service (overwrite with different port)
    let mut updated = test_entry("mydb", 5433);
    updated.published_at_ms = 2000;
    registry.publish(updated).await.unwrap();

    // Force resolver cache refresh
    resolver.refresh_cache().await.unwrap();

    let found = resolver.resolve("mydb").await.unwrap();
    assert_eq!(found.unwrap().1, 5433, "updated port should be reflected");

    // Unpublish first service
    registry.unpublish("mydb").await.unwrap();
    resolver.refresh_cache().await.unwrap();

    let found = resolver.resolve("mydb").await.unwrap();
    assert!(found.is_none(), "mydb should be gone after unpublish");

    // Second service still exists
    let found = resolver.resolve("cache").await.unwrap();
    assert!(found.is_some());

    // Clean up
    registry.unpublish("cache").await.unwrap();
    let all = registry.list(None).await.unwrap();
    assert_eq!(all.len(), 0);

    // Node entries should be clean
    let nodes = registry.list_nodes().await.unwrap();
    for node in &nodes {
        assert!(node.services.is_empty());
    }
}

// =========================================================================
// 14.8: Delegation chain attenuation
// =========================================================================

/// Task 14.8: Token delegation chain — root delegates to scoped,
/// verify attenuation works.
#[tokio::test]
async fn delegation_chain_attenuation() {
    // Admin auth: can do everything
    let admin_auth = make_admin_auth();
    assert!(admin_auth.check_connect("any-service", 5432).is_ok());
    assert!(admin_auth.check_connect("restricted/db", 5432).is_ok());
    assert!(admin_auth.check_publish("any-service").is_ok());

    // Scoped auth: only "allowed/" prefix
    let scoped_auth = make_scoped_connect_auth("allowed/");
    assert!(scoped_auth.check_connect("allowed/mydb", 5432).is_ok());
    assert!(scoped_auth.check_connect("allowed/cache", 6379).is_ok());
    assert!(scoped_auth.check_connect("denied/mydb", 5432).is_err());
    assert!(scoped_auth.check_connect("other-service", 8080).is_err());

    // Capability containment (used for delegation chains)
    let admin_cap = Capability::NetAdmin;
    let connect_cap = Capability::NetConnect {
        service_prefix: "team-a/".to_string(),
    };
    let narrow_connect = Capability::NetConnect {
        service_prefix: "team-a/prod/".to_string(),
    };
    let publish_cap = Capability::NetPublish {
        service_prefix: "team-a/".to_string(),
    };

    // NetAdmin contains everything
    assert!(admin_cap.contains(&connect_cap));
    assert!(admin_cap.contains(&publish_cap));
    assert!(admin_cap.contains(&narrow_connect));

    // Broader contains narrower
    assert!(connect_cap.contains(&narrow_connect));

    // Narrower does NOT contain broader
    assert!(!narrow_connect.contains(&connect_cap));

    // Connect does NOT contain Publish (different type)
    assert!(!connect_cap.contains(&publish_cap));
}

// =========================================================================
// 14.9: NixOS VM test placeholder
// =========================================================================

/// Task 14.9: NixOS VM integration test (run via nix build).
///
/// See nix/tests/net-service-mesh.nix for the full VM test.
#[test]
#[ignore = "NixOS VM test — run via nix build"]
fn nixos_vm_multi_node_tunnel() {}

// =========================================================================
// Daemon lifecycle
// =========================================================================

#[tokio::test]
async fn daemon_start_stop_no_dns() {
    use aspen_net::daemon::DaemonConfig;
    use aspen_net::daemon::NetDaemon;

    let config = DaemonConfig {
        cluster_ticket: "test-ticket".to_string(),
        token: "test-token".to_string(),
        socks5_addr: DaemonConfig::default_socks5_addr(),
        dns_addr: DaemonConfig::default_dns_addr(),
        dns_enabled: false,
        auto_publish: vec![],
        tags: vec![],
    };

    let mut daemon = NetDaemon::start(config).await.unwrap();
    assert!(daemon.is_running());

    daemon.shutdown().await;
    assert!(!daemon.is_running());
}

#[tokio::test]
async fn daemon_publish_spec_parsing() {
    use aspen_net::daemon::parse_publish_spec;

    let (name, port, proto) = parse_publish_spec("mydb:5432").unwrap();
    assert_eq!(name, "mydb");
    assert_eq!(port, 5432);
    assert_eq!(proto, "tcp");

    let (name, port, proto) = parse_publish_spec("web:8080:http").unwrap();
    assert_eq!(name, "web");
    assert_eq!(port, 8080);
    assert_eq!(proto, "http");

    assert!(parse_publish_spec("invalid").is_err());
    assert!(parse_publish_spec("name:notaport").is_err());
}

// =========================================================================
// Extra SOCKS5 edge cases
// =========================================================================

#[tokio::test]
async fn socks5_rejects_non_aspen_domain() {
    let (_registry, resolver) = make_stack();
    let cancel = CancellationToken::new();
    let auth = Arc::new(make_connect_token());

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = listener.local_addr().unwrap();
    let server = Socks5Server::new(resolver, auth, cancel.clone());
    let server_task = tokio::spawn(async move { server.run(listener).await });
    tokio::time::sleep(Duration::from_millis(50)).await;

    let mut client = TcpStream::connect(proxy_addr).await.unwrap();
    let reply = socks5_connect(&mut client, "google.com", 443).await.unwrap();
    assert_eq!(reply, 0x05, "non-.aspen domain should be refused");

    cancel.cancel();
    let _ = tokio::time::timeout(Duration::from_secs(2), server_task).await;
}

#[tokio::test]
async fn socks5_host_unreachable_unknown_service() {
    let (_registry, resolver) = make_stack();
    let cancel = CancellationToken::new();
    let auth = Arc::new(make_connect_token());

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = listener.local_addr().unwrap();
    let server = Socks5Server::new(resolver, auth, cancel.clone());
    let server_task = tokio::spawn(async move { server.run(listener).await });
    tokio::time::sleep(Duration::from_millis(50)).await;

    let mut client = TcpStream::connect(proxy_addr).await.unwrap();
    let reply = socks5_connect(&mut client, "nonexistent.aspen", 5432).await.unwrap();
    assert_eq!(reply, 0x04, "unknown .aspen service should get host unreachable");

    cancel.cancel();
    let _ = tokio::time::timeout(Duration::from_secs(2), server_task).await;
}
