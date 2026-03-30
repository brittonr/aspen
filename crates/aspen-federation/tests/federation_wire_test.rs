//! Wire-level integration tests for the federation sync protocol.
//!
//! Tests the complete federation handshake, resource listing, state queries,
//! and object sync over real iroh QUIC connections. Two iroh endpoints
//! simulate two independent clusters communicating via the federation ALPN.
//!
//! # Architecture
//!
//! ```text
//! Alice Cluster                          Bob Cluster
//! ┌──────────────────────┐              ┌──────────────────────┐
//! │ iroh Endpoint        │              │ iroh Endpoint        │
//! │ + FederationProtocol │── QUIC/TLS ──│ + FederationProtocol │
//! │   Handler            │              │   Handler            │
//! │ + ClusterIdentity    │              │ + ClusterIdentity    │
//! │ + TrustManager       │              │ + TrustManager       │
//! │ + Resources          │              │ + Resources          │
//! └──────────────────────┘              └──────────────────────┘
//! ```
//!
//! # Running
//!
//! These tests require network access (iroh binds to loopback) and are
//! `#[ignore]` for CI sandboxes:
//!
//! ```sh
//! cargo nextest run -p aspen-federation \
//!     --test federation_wire_test --run-ignored all
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use aspen_core::Signature;
use aspen_federation::identity::ClusterIdentity;
use aspen_federation::resolver::FederationResourceError;
use aspen_federation::resolver::FederationResourceResolver;
use aspen_federation::resolver::FederationResourceState;
use aspen_federation::sync::FEDERATION_ALPN;
use aspen_federation::sync::FEDERATION_PROTOCOL_VERSION;
use aspen_federation::sync::FederationProtocolContext;
use aspen_federation::sync::FederationProtocolHandler;
use aspen_federation::sync::FederationRequest;
use aspen_federation::sync::FederationResponse;
use aspen_federation::sync::SyncObject;
use aspen_federation::sync::get_remote_resource_state;
use aspen_federation::sync::list_remote_resources;
use aspen_federation::sync::sync_remote_objects;
use aspen_federation::trust::TrustManager;
use aspen_federation::types::FederatedId;
use aspen_federation::types::FederationMode;
use aspen_federation::types::FederationSettings;
use async_trait::async_trait;
use iroh::Endpoint;
use iroh::EndpointAddr;
use iroh::endpoint::Connection;
use iroh::protocol::Router;
use tokio::sync::RwLock;

// ============================================================================
// Test Helpers
// ============================================================================

/// Mock resource resolver for testing object verification.
struct MockResolver {
    objects: Arc<RwLock<Vec<SyncObject>>>,
    state: Arc<RwLock<FederationResourceState>>,
}

impl MockResolver {
    /// Create a new mock resolver with the given objects.
    fn new(objects: Vec<SyncObject>) -> Self {
        Self {
            objects: Arc::new(RwLock::new(objects)),
            state: Arc::new(RwLock::new(FederationResourceState::default())),
        }
    }
}

#[async_trait]
impl FederationResourceResolver for MockResolver {
    async fn get_resource_state(
        &self,
        _fed_id: &FederatedId,
    ) -> Result<FederationResourceState, FederationResourceError> {
        Ok(self.state.read().await.clone())
    }

    async fn sync_objects(
        &self,
        _fed_id: &FederatedId,
        _want_types: &[String],
        _have_hashes: &[[u8; 32]],
        _limit: u32,
    ) -> Result<Vec<SyncObject>, FederationResourceError> {
        Ok(self.objects.read().await.clone())
    }

    async fn resource_exists(&self, _fed_id: &FederatedId) -> bool {
        true
    }
}

/// A minimal federation cluster for testing.
struct TestCluster {
    endpoint: Endpoint,
    identity: ClusterIdentity,
    trust_manager: Arc<TrustManager>,
    resource_settings: Arc<RwLock<HashMap<FederatedId, FederationSettings>>>,
    /// Keep the router alive so the protocol handler stays registered.
    _router: Router,
}

impl TestCluster {
    /// Create a new test cluster with an iroh endpoint and federation handler.
    async fn new(name: &str) -> Self {
        Self::new_with_resolver(name, None).await
    }

    /// Create a new test cluster with an optional resource resolver.
    async fn new_with_resolver(name: &str, resource_resolver: Option<Arc<dyn FederationResourceResolver>>) -> Self {
        let secret_key = iroh::SecretKey::generate(&mut rand::rng());
        let identity = ClusterIdentity::generate(name.to_string());
        let trust_manager = Arc::new(TrustManager::new());
        let resource_settings = Arc::new(RwLock::new(HashMap::new()));

        // Build endpoint first
        let endpoint = Endpoint::builder(iroh::endpoint::presets::N0)
            .secret_key(secret_key)
            .clear_address_lookup()
            .bind()
            .await
            .expect("bind endpoint");

        let hlc = Arc::new(aspen_hlc::create_hlc(name));

        let context = FederationProtocolContext {
            cluster_identity: identity.clone(),
            trust_manager: trust_manager.clone(),
            resource_settings: resource_settings.clone(),
            endpoint: Arc::new(endpoint.clone()),
            hlc,
            resource_resolver,
            session_credential: std::sync::Mutex::new(None),
        };

        let handler = FederationProtocolHandler::new(context);

        // Register the federation protocol handler via Router
        let router = Router::builder(endpoint.clone()).accept(FEDERATION_ALPN.to_vec(), handler).spawn();

        Self {
            endpoint,
            identity,
            trust_manager,
            resource_settings,
            _router: router,
        }
    }

    /// Get the cluster's public key.
    fn cluster_key(&self) -> iroh::PublicKey {
        self.identity.public_key()
    }

    /// Add a federated resource with the given mode.
    async fn add_resource(&self, fed_id: FederatedId, mode: FederationMode) {
        self.add_typed_resource(fed_id, mode, "forge:repo").await;
    }

    /// Add a federated resource with the given mode and resource type.
    async fn add_typed_resource(&self, fed_id: FederatedId, mode: FederationMode, resource_type: &str) {
        let settings = match mode {
            FederationMode::Public => FederationSettings::public(),
            FederationMode::AllowList => FederationSettings::allowlist(vec![]),
            FederationMode::Disabled => FederationSettings::disabled(),
        }
        .with_resource_type(resource_type);
        self.resource_settings.write().await.insert(fed_id, settings);
    }

    /// Get the endpoint address with loopback fixup (0.0.0.0 → 127.0.0.1).
    fn endpoint_addr(&self) -> EndpointAddr {
        let mut addr = EndpointAddr::new(self.endpoint.id());
        for socket_addr in self.endpoint.bound_sockets() {
            let fixed = if socket_addr.ip().is_unspecified() {
                std::net::SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), socket_addr.port())
            } else {
                socket_addr
            };
            addr.addrs.insert(iroh::TransportAddr::Ip(fixed));
        }
        addr
    }
}

/// Create a FederatedId from a test name.
fn test_fed_id(origin: iroh::PublicKey, name: &str) -> FederatedId {
    let local_id: [u8; 32] = blake3::hash(name.as_bytes()).into();
    FederatedId::new(origin, local_id)
}

/// Connect to a remote cluster and perform the federation handshake.
///
/// Uses `EndpointAddr` for direct addressing (no discovery needed).
async fn connect_and_handshake(
    our_endpoint: &Endpoint,
    our_identity: &ClusterIdentity,
    remote_addr: EndpointAddr,
) -> anyhow::Result<(Connection, aspen_federation::identity::SignedClusterIdentity)> {
    use aspen_federation::sync::wire::read_message;
    use aspen_federation::sync::wire::write_message;

    let connection = our_endpoint
        .connect(remote_addr, FEDERATION_ALPN)
        .await
        .map_err(|e| anyhow::anyhow!("connect: {}", e))?;

    let (mut send, mut recv) = connection.open_bi().await.map_err(|e| anyhow::anyhow!("open_bi: {}", e))?;

    let request = FederationRequest::Handshake {
        identity: our_identity.to_signed(),
        protocol_version: FEDERATION_PROTOCOL_VERSION,
        capabilities: vec!["forge".to_string()],
        credential: None,
    };
    write_message(&mut send, &request).await?;

    let response: FederationResponse = read_message(&mut recv).await?;

    match response {
        FederationResponse::Handshake {
            identity, trusted: _, ..
        } => {
            if !identity.verify() {
                anyhow::bail!("peer identity verification failed");
            }
            Ok((connection, identity))
        }
        FederationResponse::Error { code, message } => {
            anyhow::bail!("handshake failed: {} - {}", code, message)
        }
        _ => anyhow::bail!("unexpected handshake response"),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[tokio::test]
#[ignore] // Requires network access (iroh binds loopback)
async fn test_federation_handshake() {
    let alice = TestCluster::new("alice-cluster").await;
    let bob = TestCluster::new("bob-cluster").await;

    // Alice trusts Bob and vice-versa
    alice.trust_manager.add_trusted(bob.cluster_key(), "bob".to_string(), None);
    bob.trust_manager.add_trusted(alice.cluster_key(), "alice".to_string(), None);

    // Alice connects to Bob
    let result = tokio::time::timeout(
        Duration::from_secs(10),
        connect_and_handshake(&alice.endpoint, &alice.identity, bob.endpoint_addr()),
    )
    .await
    .expect("timeout")
    .expect("connect");

    let (_connection, peer_identity) = result;

    // Verify Bob's identity
    assert_eq!(peer_identity.name(), "bob-cluster");
    assert!(peer_identity.verify());
    assert_eq!(peer_identity.public_key(), bob.cluster_key());
}

#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn test_federation_handshake_bidirectional() {
    let alice = TestCluster::new("alice").await;
    let bob = TestCluster::new("bob").await;

    alice.trust_manager.add_trusted(bob.cluster_key(), "bob".to_string(), None);
    bob.trust_manager.add_trusted(alice.cluster_key(), "alice".to_string(), None);

    // Alice → Bob
    let (_, bob_id) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_and_handshake(&alice.endpoint, &alice.identity, bob.endpoint_addr()),
    )
    .await
    .expect("timeout")
    .expect("alice->bob");

    assert_eq!(bob_id.name(), "bob");

    // Bob → Alice
    let (_, alice_id) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_and_handshake(&bob.endpoint, &bob.identity, alice.endpoint_addr()),
    )
    .await
    .expect("timeout")
    .expect("bob->alice");

    assert_eq!(alice_id.name(), "alice");
}

#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn test_list_remote_resources_empty() {
    let alice = TestCluster::new("alice").await;
    let bob = TestCluster::new("bob").await;

    alice.trust_manager.add_trusted(bob.cluster_key(), "bob".to_string(), None);
    bob.trust_manager.add_trusted(alice.cluster_key(), "alice".to_string(), None);

    let (conn, _) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_and_handshake(&alice.endpoint, &alice.identity, bob.endpoint_addr()),
    )
    .await
    .expect("timeout")
    .expect("connect");

    // Bob has no resources
    let resources = tokio::time::timeout(Duration::from_secs(10), list_remote_resources(&conn, None, 100))
        .await
        .expect("timeout")
        .expect("list");

    assert!(resources.is_empty());
}

#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn test_list_remote_resources_with_public_repos() {
    let alice = TestCluster::new("alice").await;
    let bob = TestCluster::new("bob").await;

    alice.trust_manager.add_trusted(bob.cluster_key(), "bob".to_string(), None);
    bob.trust_manager.add_trusted(alice.cluster_key(), "alice".to_string(), None);

    // Add two public repos on Bob
    let repo1 = test_fed_id(bob.cluster_key(), "repo-1");
    let repo2 = test_fed_id(bob.cluster_key(), "repo-2");
    bob.add_resource(repo1, FederationMode::Public).await;
    bob.add_resource(repo2, FederationMode::Public).await;

    // Also add a disabled repo (should not appear)
    let disabled = test_fed_id(bob.cluster_key(), "private-repo");
    bob.add_resource(disabled, FederationMode::Disabled).await;

    let (conn, _) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_and_handshake(&alice.endpoint, &alice.identity, bob.endpoint_addr()),
    )
    .await
    .expect("timeout")
    .expect("connect");

    let resources = tokio::time::timeout(Duration::from_secs(10), list_remote_resources(&conn, None, 100))
        .await
        .expect("timeout")
        .expect("list");

    // Only public repos should be listed
    assert_eq!(resources.len(), 2, "disabled repos should not be listed");

    // All should be forge:repo type
    for r in &resources {
        assert_eq!(r.resource_type, "forge:repo");
        assert_eq!(r.mode, "public");
    }
}

#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn test_list_remote_resources_with_type_filter() {
    let alice = TestCluster::new("alice").await;
    let bob = TestCluster::new("bob").await;

    alice.trust_manager.add_trusted(bob.cluster_key(), "bob".to_string(), None);
    bob.trust_manager.add_trusted(alice.cluster_key(), "alice".to_string(), None);

    let repo = test_fed_id(bob.cluster_key(), "repo-1");
    bob.add_resource(repo, FederationMode::Public).await;

    let (conn, _) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_and_handshake(&alice.endpoint, &alice.identity, bob.endpoint_addr()),
    )
    .await
    .expect("timeout")
    .expect("connect");

    // Filter by existing type
    let resources =
        tokio::time::timeout(Duration::from_secs(10), list_remote_resources(&conn, Some("forge:repo"), 100))
            .await
            .expect("timeout")
            .expect("list");
    assert_eq!(resources.len(), 1);

    // Filter by non-existing type
    let resources =
        tokio::time::timeout(Duration::from_secs(10), list_remote_resources(&conn, Some("ci:pipeline"), 100))
            .await
            .expect("timeout")
            .expect("list");
    assert!(resources.is_empty());
}

#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn test_get_resource_state_not_found() {
    let alice = TestCluster::new("alice").await;
    let bob = TestCluster::new("bob").await;

    alice.trust_manager.add_trusted(bob.cluster_key(), "bob".to_string(), None);
    bob.trust_manager.add_trusted(alice.cluster_key(), "alice".to_string(), None);

    let (conn, _) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_and_handshake(&alice.endpoint, &alice.identity, bob.endpoint_addr()),
    )
    .await
    .expect("timeout")
    .expect("connect");

    let nonexistent = test_fed_id(bob.cluster_key(), "nonexistent");
    let (was_found, heads, metadata) =
        tokio::time::timeout(Duration::from_secs(10), get_remote_resource_state(&conn, &nonexistent))
            .await
            .expect("timeout")
            .expect("get state");

    assert!(!was_found);
    assert!(heads.is_empty());
    assert!(metadata.is_none());
}

#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn test_get_resource_state_found() {
    let alice = TestCluster::new("alice").await;
    let bob = TestCluster::new("bob").await;

    alice.trust_manager.add_trusted(bob.cluster_key(), "bob".to_string(), None);
    bob.trust_manager.add_trusted(alice.cluster_key(), "alice".to_string(), None);

    // Add a resource on Bob
    let repo = test_fed_id(bob.cluster_key(), "test-repo");
    bob.add_resource(repo, FederationMode::Public).await;

    let (conn, _) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_and_handshake(&alice.endpoint, &alice.identity, bob.endpoint_addr()),
    )
    .await
    .expect("timeout")
    .expect("connect");

    let (was_found, _heads, _metadata) =
        tokio::time::timeout(Duration::from_secs(10), get_remote_resource_state(&conn, &repo))
            .await
            .expect("timeout")
            .expect("get state");

    assert!(was_found, "resource should be found");
}

#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn test_sync_objects_empty_resource() {
    let alice = TestCluster::new("alice").await;
    let bob = TestCluster::new("bob").await;

    alice.trust_manager.add_trusted(bob.cluster_key(), "bob".to_string(), None);
    bob.trust_manager.add_trusted(alice.cluster_key(), "alice".to_string(), None);

    let repo = test_fed_id(bob.cluster_key(), "empty-repo");
    bob.add_resource(repo, FederationMode::Public).await;

    let (conn, _) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_and_handshake(&alice.endpoint, &alice.identity, bob.endpoint_addr()),
    )
    .await
    .expect("timeout")
    .expect("connect");

    // Sync from Bob (no resolver configured, so returns empty)
    let (objects, has_more) = tokio::time::timeout(
        Duration::from_secs(10),
        sync_remote_objects(&conn, &repo, vec!["commit".to_string()], vec![], 100, None),
    )
    .await
    .expect("timeout")
    .expect("sync");

    assert!(objects.is_empty());
    assert!(!has_more);
}

#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn test_sync_objects_not_found() {
    let alice = TestCluster::new("alice").await;
    let bob = TestCluster::new("bob").await;

    alice.trust_manager.add_trusted(bob.cluster_key(), "bob".to_string(), None);
    bob.trust_manager.add_trusted(alice.cluster_key(), "alice".to_string(), None);

    let (conn, _) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_and_handshake(&alice.endpoint, &alice.identity, bob.endpoint_addr()),
    )
    .await
    .expect("timeout")
    .expect("connect");

    // Try to sync a non-existent resource
    let nonexistent = test_fed_id(bob.cluster_key(), "no-such-repo");
    let result = tokio::time::timeout(
        Duration::from_secs(10),
        sync_remote_objects(&conn, &nonexistent, vec![], vec![], 100, None),
    )
    .await
    .expect("timeout");

    // Should return an error
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("NOT_FOUND"), "expected NOT_FOUND error, got: {}", err);
}

#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn test_multiple_streams_on_single_connection() {
    let alice = TestCluster::new("alice").await;
    let bob = TestCluster::new("bob").await;

    alice.trust_manager.add_trusted(bob.cluster_key(), "bob".to_string(), None);
    bob.trust_manager.add_trusted(alice.cluster_key(), "alice".to_string(), None);

    // Add some resources on Bob
    for i in 0..3u8 {
        let repo = test_fed_id(bob.cluster_key(), &format!("repo-{}", i));
        bob.add_resource(repo, FederationMode::Public).await;
    }

    let (conn, _) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_and_handshake(&alice.endpoint, &alice.identity, bob.endpoint_addr()),
    )
    .await
    .expect("timeout")
    .expect("connect");

    // Open multiple streams sequentially on the same connection
    for i in 0..3u8 {
        let resources = tokio::time::timeout(Duration::from_secs(5), list_remote_resources(&conn, None, 100))
            .await
            .expect("timeout")
            .unwrap_or_else(|e| panic!("list {} failed: {}", i, e));

        assert_eq!(resources.len(), 3, "stream {} should see 3 resources", i);
    }
}

#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn test_full_sync_flow() {
    // End-to-end: handshake → list → get state → sync objects
    let alice = TestCluster::new("alice").await;
    let bob = TestCluster::new("bob").await;

    alice.trust_manager.add_trusted(bob.cluster_key(), "bob".to_string(), None);
    bob.trust_manager.add_trusted(alice.cluster_key(), "alice".to_string(), None);

    // Bob has a federated repo
    let repo = test_fed_id(bob.cluster_key(), "aspen-core");
    bob.add_resource(repo, FederationMode::Public).await;

    // Step 1: Handshake
    let (conn, peer_id) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_and_handshake(&alice.endpoint, &alice.identity, bob.endpoint_addr()),
    )
    .await
    .expect("timeout")
    .expect("handshake");

    assert_eq!(peer_id.name(), "bob");
    assert!(peer_id.verify());

    // Step 2: List resources
    let resources = tokio::time::timeout(Duration::from_secs(5), list_remote_resources(&conn, Some("forge:repo"), 100))
        .await
        .expect("timeout")
        .expect("list");

    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].fed_id, repo);

    // Step 3: Get resource state
    let (was_found, _heads, _meta) =
        tokio::time::timeout(Duration::from_secs(5), get_remote_resource_state(&conn, &repo))
            .await
            .expect("timeout")
            .expect("get state");

    assert!(was_found);

    // Step 4: Sync objects (empty since no resolver)
    let (objects, has_more) = tokio::time::timeout(
        Duration::from_secs(5),
        sync_remote_objects(&conn, &repo, vec!["commit".to_string()], vec![], 100, None),
    )
    .await
    .expect("timeout")
    .expect("sync");

    assert!(objects.is_empty(), "no resolver = no objects");
    assert!(!has_more);
}

// ============================================================================
// Content Hash and Delegate Signature Verification Tests
// ============================================================================

#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn test_sync_objects_drops_bad_content_hash() {
    let alice = TestCluster::new("alice").await;

    // Create mock resolver that returns objects with mismatched hashes
    let good_data = b"this is the correct data";
    let good_hash = *blake3::hash(good_data).as_bytes();

    let bad_data = b"this is the wrong data";
    let bad_hash = *blake3::hash(b"different content").as_bytes(); // Hash doesn't match data

    let mock_objects = vec![
        // Good object: hash matches data
        SyncObject {
            object_type: "blob".to_string(),
            hash: good_hash,
            data: good_data.to_vec(),
            signature: None,
            signer: None,

            envelope_hash: None,

            origin_sha1: None,
        },
        // Bad object: hash doesn't match data
        SyncObject {
            object_type: "blob".to_string(),
            hash: bad_hash,
            data: bad_data.to_vec(),
            signature: None,
            signer: None,

            envelope_hash: None,

            origin_sha1: None,
        },
    ];

    let resolver = Arc::new(MockResolver::new(mock_objects));
    let bob = TestCluster::new_with_resolver("bob", Some(resolver)).await;

    alice.trust_manager.add_trusted(bob.cluster_key(), "bob".to_string(), None);
    bob.trust_manager.add_trusted(alice.cluster_key(), "alice".to_string(), None);

    let repo = test_fed_id(bob.cluster_key(), "test-repo");
    bob.add_resource(repo, FederationMode::Public).await;

    let (conn, _) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_and_handshake(&alice.endpoint, &alice.identity, bob.endpoint_addr()),
    )
    .await
    .expect("timeout")
    .expect("connect");

    // Sync objects without delegate verification
    let (objects, _has_more) = tokio::time::timeout(
        Duration::from_secs(10),
        sync_remote_objects(&conn, &repo, vec!["blob".to_string()], vec![], 100, None),
    )
    .await
    .expect("timeout")
    .expect("sync");

    // Only the good object should be returned (bad hash dropped)
    assert_eq!(objects.len(), 1, "bad content hash should be dropped");
    assert_eq!(objects[0].hash, good_hash);
    assert_eq!(objects[0].data, good_data);
}

#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn test_sync_objects_drops_bad_delegate_signature() {
    let alice = TestCluster::new("alice").await;

    // Create a valid delegate key
    let delegate_secret = iroh::SecretKey::generate(&mut rand::rng());
    let delegate_pub = delegate_secret.public();

    // Create an unauthorized key (not in delegates list)
    let unauthorized_secret = iroh::SecretKey::generate(&mut rand::rng());

    let data = b"signed object data";
    let hash = *blake3::hash(data).as_bytes();

    // Create FederatedId for signing
    let origin = iroh::SecretKey::generate(&mut rand::rng()).public();
    let fed_id = test_fed_id(origin, "test-repo");

    // Build the signed message (same format as verify_delegate_signature)
    let object_type = "commit";
    let timestamp_ms = 0u64;
    let mut message = Vec::new();
    message.extend_from_slice(fed_id.origin().as_bytes());
    message.extend_from_slice(fed_id.local_id());
    message.extend_from_slice(object_type.as_bytes());
    message.extend_from_slice(&hash);
    message.extend_from_slice(&timestamp_ms.to_le_bytes());

    // Sign with unauthorized key (invalid)
    let bad_sig = unauthorized_secret.sign(&message);
    let bad_signature = Signature(bad_sig.to_bytes());

    let mock_objects = vec![SyncObject {
        object_type: object_type.to_string(),
        hash,
        data: data.to_vec(),
        signature: Some(bad_signature),
        signer: Some(*unauthorized_secret.public().as_bytes()),

        envelope_hash: None,

        origin_sha1: None,
    }];

    let resolver = Arc::new(MockResolver::new(mock_objects));
    let bob = TestCluster::new_with_resolver("bob", Some(resolver)).await;

    alice.trust_manager.add_trusted(bob.cluster_key(), "bob".to_string(), None);
    bob.trust_manager.add_trusted(alice.cluster_key(), "alice".to_string(), None);

    bob.add_resource(fed_id, FederationMode::Public).await;

    let (conn, _) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_and_handshake(&alice.endpoint, &alice.identity, bob.endpoint_addr()),
    )
    .await
    .expect("timeout")
    .expect("connect");

    // Sync with delegate verification enabled
    let (objects, _has_more) = tokio::time::timeout(
        Duration::from_secs(10),
        sync_remote_objects(&conn, &fed_id, vec![object_type.to_string()], vec![], 100, Some(&[delegate_pub])),
    )
    .await
    .expect("timeout")
    .expect("sync");

    // Object with invalid signature should be dropped
    assert_eq!(objects.len(), 0, "bad delegate signature should be dropped");
}

#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn test_sync_objects_passes_valid_delegate_signature() {
    let alice = TestCluster::new("alice").await;

    // Create a valid delegate key
    let delegate_secret = iroh::SecretKey::generate(&mut rand::rng());
    let delegate_pub = delegate_secret.public();

    let data = b"properly signed object data";
    let hash = *blake3::hash(data).as_bytes();

    // Create FederatedId for signing
    let origin = iroh::SecretKey::generate(&mut rand::rng()).public();
    let fed_id = test_fed_id(origin, "test-repo");

    // Build the signed message
    let object_type = "commit";
    let timestamp_ms = 0u64;
    let mut message = Vec::new();
    message.extend_from_slice(fed_id.origin().as_bytes());
    message.extend_from_slice(fed_id.local_id());
    message.extend_from_slice(object_type.as_bytes());
    message.extend_from_slice(&hash);
    message.extend_from_slice(&timestamp_ms.to_le_bytes());

    // Sign with valid delegate key
    let valid_sig = delegate_secret.sign(&message);
    let signature = Signature(valid_sig.to_bytes());

    let mock_objects = vec![SyncObject {
        object_type: object_type.to_string(),
        hash,
        data: data.to_vec(),
        signature: Some(signature),
        signer: Some(*delegate_pub.as_bytes()),

        envelope_hash: None,

        origin_sha1: None,
    }];

    let resolver = Arc::new(MockResolver::new(mock_objects));
    let bob = TestCluster::new_with_resolver("bob", Some(resolver)).await;

    alice.trust_manager.add_trusted(bob.cluster_key(), "bob".to_string(), None);
    bob.trust_manager.add_trusted(alice.cluster_key(), "alice".to_string(), None);

    bob.add_resource(fed_id, FederationMode::Public).await;

    let (conn, _) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_and_handshake(&alice.endpoint, &alice.identity, bob.endpoint_addr()),
    )
    .await
    .expect("timeout")
    .expect("connect");

    // Sync with delegate verification enabled
    let (objects, _has_more) = tokio::time::timeout(
        Duration::from_secs(10),
        sync_remote_objects(&conn, &fed_id, vec![object_type.to_string()], vec![], 100, Some(&[delegate_pub])),
    )
    .await
    .expect("timeout")
    .expect("sync");

    // Valid signature should pass through
    assert_eq!(objects.len(), 1, "valid delegate signature should pass");
    assert_eq!(objects[0].hash, hash);
    assert_eq!(objects[0].data, data);
}

#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn test_sync_objects_accepts_unsigned_when_no_delegates() {
    let alice = TestCluster::new("alice").await;

    // Create unsigned objects
    let data1 = b"unsigned object 1";
    let hash1 = *blake3::hash(data1).as_bytes();

    let data2 = b"unsigned object 2";
    let hash2 = *blake3::hash(data2).as_bytes();

    let mock_objects = vec![
        SyncObject {
            object_type: "blob".to_string(),
            hash: hash1,
            data: data1.to_vec(),
            signature: None,
            signer: None,

            envelope_hash: None,

            origin_sha1: None,
        },
        SyncObject {
            object_type: "blob".to_string(),
            hash: hash2,
            data: data2.to_vec(),
            signature: None,
            signer: None,

            envelope_hash: None,

            origin_sha1: None,
        },
    ];

    let resolver = Arc::new(MockResolver::new(mock_objects));
    let bob = TestCluster::new_with_resolver("bob", Some(resolver)).await;

    alice.trust_manager.add_trusted(bob.cluster_key(), "bob".to_string(), None);
    bob.trust_manager.add_trusted(alice.cluster_key(), "alice".to_string(), None);

    let repo = test_fed_id(bob.cluster_key(), "test-repo");
    bob.add_resource(repo, FederationMode::Public).await;

    let (conn, _) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_and_handshake(&alice.endpoint, &alice.identity, bob.endpoint_addr()),
    )
    .await
    .expect("timeout")
    .expect("connect");

    // Sync without delegate verification (delegates = None)
    let (objects, _has_more) = tokio::time::timeout(
        Duration::from_secs(10),
        sync_remote_objects(&conn, &repo, vec!["blob".to_string()], vec![], 100, None),
    )
    .await
    .expect("timeout")
    .expect("sync");

    // All unsigned objects should pass (only hash check applies)
    assert_eq!(objects.len(), 2, "unsigned objects should pass when delegates = None");
    assert_eq!(objects[0].hash, hash1);
    assert_eq!(objects[1].hash, hash2);
}

// ============================================================================
// Pull Protocol Flow Tests
// ============================================================================

/// Test: Cold pull protocol — connect, handshake to discover cluster identity,
/// then fetch objects. Mirrors what handle_federation_pull_remote does.
#[tokio::test]
#[ignore = "requires network access (iroh socket binding)"]
async fn test_pull_cold_handshake_discovers_cluster_identity() {
    // Bob has a repo; Alice wants to pull it knowing only Bob's node ID + repo hash
    let bob_data = b"commit: initial";
    let bob_hash: [u8; 32] = blake3::hash(bob_data).into();

    let resolver = MockResolver::new(vec![SyncObject {
        object_type: "commit".to_string(),
        hash: bob_hash,
        data: bob_data.to_vec(),
        signature: None,
        signer: None,

        envelope_hash: None,

        origin_sha1: None,
    }]);

    let alice = TestCluster::new("alice-puller").await;
    let bob = TestCluster::new_with_resolver("bob-origin", Some(Arc::new(resolver))).await;

    alice.trust_manager.add_trusted(bob.cluster_key(), "bob".to_string(), None);
    bob.trust_manager.add_trusted(alice.cluster_key(), "alice".to_string(), None);

    // Bob publishes a repo
    let repo_local_id: [u8; 32] = blake3::hash(b"my-repo").into();
    let repo = FederatedId::new(bob.cluster_key(), repo_local_id);
    bob.add_resource(repo, FederationMode::Public).await;

    // Alice connects to Bob (simulating cold pull: she knows node ID but not cluster key)
    let (conn, remote_identity) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_and_handshake(&alice.endpoint, &alice.identity, bob.endpoint_addr()),
    )
    .await
    .expect("timeout")
    .expect("handshake");

    // Alice learns Bob's cluster identity key from the handshake
    let discovered_key = remote_identity.public_key();
    assert_eq!(discovered_key, bob.cluster_key());

    // Alice constructs the FederatedId using discovered key + known repo ID
    let constructed_fed_id = FederatedId::new(discovered_key, repo_local_id);
    assert_eq!(constructed_fed_id, repo);

    // Alice fetches objects
    let (objects, _has_more) = tokio::time::timeout(
        Duration::from_secs(5),
        sync_remote_objects(&conn, &constructed_fed_id, vec!["commit".to_string()], vec![], 100, None),
    )
    .await
    .expect("timeout")
    .expect("sync");

    assert_eq!(objects.len(), 1);
    assert_eq!(objects[0].hash, bob_hash);
    assert_eq!(objects[0].data, bob_data);
}

/// Test: Incremental pull — second pull sends have_hashes so server can skip known objects.
#[tokio::test]
#[ignore = "requires network access (iroh socket binding)"]
async fn test_pull_incremental_with_have_hashes() {
    // Bob has two objects; Alice already has the first one
    let data1 = b"blob: old data";
    let hash1: [u8; 32] = blake3::hash(data1).into();
    let data2 = b"blob: new data";
    let hash2: [u8; 32] = blake3::hash(data2).into();

    // Resolver that filters out objects in have_hashes
    struct IncrementalResolver {
        objects: Vec<SyncObject>,
    }

    #[async_trait]
    impl FederationResourceResolver for IncrementalResolver {
        async fn get_resource_state(
            &self,
            _fed_id: &FederatedId,
        ) -> Result<FederationResourceState, FederationResourceError> {
            Ok(FederationResourceState::default())
        }

        async fn sync_objects(
            &self,
            _fed_id: &FederatedId,
            _want_types: &[String],
            have_hashes: &[[u8; 32]],
            _limit: u32,
        ) -> Result<Vec<SyncObject>, FederationResourceError> {
            // Filter out objects the client already has
            Ok(self.objects.iter().filter(|o| !have_hashes.contains(&o.hash)).cloned().collect())
        }

        async fn resource_exists(&self, _fed_id: &FederatedId) -> bool {
            true
        }

        async fn list_resources(&self, _limit: u32) -> Vec<(FederatedId, String)> {
            vec![]
        }

        async fn import_pushed_objects(
            &self,
            _fed_id: &FederatedId,
            _objects: Vec<SyncObject>,
            _ref_updates: Vec<aspen_federation::sync::RefEntry>,
        ) -> Result<(u32, u32, u32), FederationResourceError> {
            Ok((0, 0, 0))
        }
    }

    let resolver = IncrementalResolver {
        objects: vec![
            SyncObject {
                object_type: "blob".to_string(),
                hash: hash1,
                data: data1.to_vec(),
                signature: None,
                signer: None,

                envelope_hash: None,

                origin_sha1: None,
            },
            SyncObject {
                object_type: "blob".to_string(),
                hash: hash2,
                data: data2.to_vec(),
                signature: None,
                signer: None,

                envelope_hash: None,

                origin_sha1: None,
            },
        ],
    };

    let alice = TestCluster::new("alice-inc").await;
    let bob = TestCluster::new_with_resolver("bob-inc", Some(Arc::new(resolver))).await;

    alice.trust_manager.add_trusted(bob.cluster_key(), "bob".to_string(), None);
    bob.trust_manager.add_trusted(alice.cluster_key(), "alice".to_string(), None);

    let repo = test_fed_id(bob.cluster_key(), "incremental-repo");
    bob.add_resource(repo, FederationMode::Public).await;

    let (conn, _) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_and_handshake(&alice.endpoint, &alice.identity, bob.endpoint_addr()),
    )
    .await
    .expect("timeout")
    .expect("handshake");

    // First pull: Alice has nothing → gets both objects
    let (objects_full, _) = tokio::time::timeout(
        Duration::from_secs(5),
        sync_remote_objects(&conn, &repo, vec!["blob".to_string()], vec![], 100, None),
    )
    .await
    .expect("timeout")
    .expect("full sync");

    assert_eq!(objects_full.len(), 2, "first pull should get all objects");

    // Second pull: Alice has hash1 → only gets hash2
    let (objects_inc, _) = tokio::time::timeout(
        Duration::from_secs(5),
        sync_remote_objects(&conn, &repo, vec!["blob".to_string()], vec![hash1], 100, None),
    )
    .await
    .expect("timeout")
    .expect("incremental sync");

    assert_eq!(objects_inc.len(), 1, "incremental pull should skip known objects");
    assert_eq!(objects_inc[0].hash, hash2);
    assert_eq!(objects_inc[0].data, data2);
}

// ============================================================================
// Bidirectional Sync Tests
// ============================================================================

/// Test: compute_ref_diff correctly categorizes refs from two clusters,
/// then wire protocol can transfer objects in both directions.
#[tokio::test]
#[ignore = "requires network access (iroh socket binding)"]
async fn test_bidi_sync_both_directions() {
    use aspen_federation::verified::ref_diff;

    // Alice has blob_a; Bob has blob_b. Both serve via resolvers.
    let data_a = b"alice-only-blob";
    let hash_a: [u8; 32] = blake3::hash(data_a).into();
    let data_b = b"bob-only-blob";
    let hash_b: [u8; 32] = blake3::hash(data_b).into();

    struct DirectionalResolver {
        objects: Vec<SyncObject>,
        heads: std::collections::HashMap<String, [u8; 32]>,
    }

    #[async_trait]
    impl FederationResourceResolver for DirectionalResolver {
        async fn get_resource_state(
            &self,
            _fed_id: &FederatedId,
        ) -> Result<FederationResourceState, FederationResourceError> {
            Ok(FederationResourceState {
                was_found: true,
                heads: self.heads.clone(),
                metadata: None,
            })
        }

        async fn sync_objects(
            &self,
            _fed_id: &FederatedId,
            _want_types: &[String],
            have_hashes: &[[u8; 32]],
            _limit: u32,
        ) -> Result<Vec<SyncObject>, FederationResourceError> {
            Ok(self.objects.iter().filter(|o| !have_hashes.contains(&o.hash)).cloned().collect())
        }

        async fn resource_exists(&self, _fed_id: &FederatedId) -> bool {
            true
        }

        async fn import_pushed_objects(
            &self,
            _fed_id: &FederatedId,
            objects: Vec<SyncObject>,
            _ref_updates: Vec<aspen_federation::sync::RefEntry>,
        ) -> Result<(u32, u32, u32), FederationResourceError> {
            Ok((objects.len() as u32, 0, 0))
        }
    }

    // Alice: has heads/feature with hash_a
    let alice_heads = {
        let mut h = std::collections::HashMap::new();
        h.insert("heads/main".to_string(), [0x11; 32]); // shared
        h.insert("heads/feature".to_string(), hash_a); // alice-only
        h
    };

    // Bob: has heads/experiment with hash_b, plus shared main
    let bob_heads = {
        let mut h = std::collections::HashMap::new();
        h.insert("heads/main".to_string(), [0x11; 32]); // shared
        h.insert("heads/experiment".to_string(), hash_b); // bob-only
        h
    };

    // Step 1: compute_ref_diff
    let diff = ref_diff::compute_ref_diff(&alice_heads, &bob_heads);
    assert_eq!(diff.in_sync, vec!["heads/main"]);
    assert_eq!(diff.to_pull, vec!["heads/experiment"]); // bob has, alice doesn't
    assert_eq!(diff.to_push, vec!["heads/feature"]); // alice has, bob doesn't
    assert!(diff.conflicts.is_empty());

    // Step 2: Wire protocol — Alice pulls bob's objects
    let bob_resolver = DirectionalResolver {
        objects: vec![SyncObject {
            object_type: "blob".to_string(),
            hash: hash_b,
            data: data_b.to_vec(),
            signature: None,
            signer: None,

            envelope_hash: None,

            origin_sha1: None,
        }],
        heads: bob_heads.clone(),
    };

    let alice = TestCluster::new("alice-bidi").await;
    let bob = TestCluster::new_with_resolver("bob-bidi", Some(Arc::new(bob_resolver))).await;

    alice.trust_manager.add_trusted(bob.cluster_key(), "bob".to_string(), None);
    bob.trust_manager.add_trusted(alice.cluster_key(), "alice".to_string(), None);
    // Push handler checks connection.remote_id() (iroh endpoint key), not cluster key
    bob.trust_manager.add_trusted(alice.endpoint.id(), "alice-node".to_string(), None);

    let repo = test_fed_id(bob.cluster_key(), "bidi-repo");
    bob.add_resource(repo, FederationMode::Public).await;

    let (conn, _) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_and_handshake(&alice.endpoint, &alice.identity, bob.endpoint_addr()),
    )
    .await
    .expect("timeout")
    .expect("handshake");

    // Pull from bob
    let (pulled_objects, _) = tokio::time::timeout(
        Duration::from_secs(5),
        sync_remote_objects(&conn, &repo, vec!["blob".to_string()], vec![], 100, None),
    )
    .await
    .expect("timeout")
    .expect("pull");

    assert_eq!(pulled_objects.len(), 1);
    assert_eq!(pulled_objects[0].hash, hash_b);

    // Push to bob (alice's objects)
    use aspen_federation::sync::RefEntry;
    use aspen_federation::sync::push_to_cluster;

    let alice_objects = vec![SyncObject {
        object_type: "blob".to_string(),
        hash: hash_a,
        data: data_a.to_vec(),
        signature: None,
        signer: None,

        envelope_hash: None,

        origin_sha1: None,
    }];
    let ref_updates = vec![RefEntry {
        ref_name: "heads/feature".to_string(),
        head_hash: hash_a,
        commit_sha1: None,
    }];

    let push_result =
        tokio::time::timeout(Duration::from_secs(5), push_to_cluster(&conn, &repo, alice_objects, ref_updates))
            .await
            .expect("timeout")
            .expect("push");

    assert!(push_result.accepted);
    assert_eq!(push_result.imported, 1);
}

/// Test: push half fails (untrusted) but pull still succeeds.
#[tokio::test]
#[ignore = "requires network access (iroh socket binding)"]
async fn test_bidi_sync_push_rejected_pull_succeeds() {
    let data_b = b"bob-data";
    let hash_b: [u8; 32] = blake3::hash(data_b).into();

    let resolver = MockResolver::new(vec![SyncObject {
        object_type: "blob".to_string(),
        hash: hash_b,
        data: data_b.to_vec(),
        signature: None,
        signer: None,

        envelope_hash: None,

        origin_sha1: None,
    }]);

    let alice = TestCluster::new("alice-partial").await;
    let bob = TestCluster::new_with_resolver("bob-partial", Some(Arc::new(resolver))).await;

    // Bob trusts Alice for pull (resource is public), but NOT for push
    // Push requires explicit trust via TrustManager
    alice.trust_manager.add_trusted(bob.cluster_key(), "bob".to_string(), None);
    // Bob does NOT add alice to trust_manager → push will be rejected

    let repo = test_fed_id(bob.cluster_key(), "partial-repo");
    bob.add_resource(repo, FederationMode::Public).await;

    let (conn, _) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_and_handshake(&alice.endpoint, &alice.identity, bob.endpoint_addr()),
    )
    .await
    .expect("timeout")
    .expect("handshake");

    // Pull succeeds (public resource, no trust needed for pull)
    let (objects, _) = tokio::time::timeout(
        Duration::from_secs(5),
        sync_remote_objects(&conn, &repo, vec!["blob".to_string()], vec![], 100, None),
    )
    .await
    .expect("timeout")
    .expect("pull should succeed");

    assert_eq!(objects.len(), 1, "pull should work even though push will fail");

    // Push fails (bob doesn't trust alice)
    use aspen_federation::sync::push_to_cluster;

    let push_result = tokio::time::timeout(Duration::from_secs(5), push_to_cluster(&conn, &repo, vec![], vec![]))
        .await
        .expect("timeout");

    // Push should fail with unauthorized error
    assert!(
        push_result.is_err() || !push_result.as_ref().unwrap().accepted,
        "push should fail when bob doesn't trust alice"
    );
}
