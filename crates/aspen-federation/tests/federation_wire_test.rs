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
        let endpoint =
            Endpoint::builder().secret_key(secret_key).clear_discovery().bind().await.expect("bind endpoint");

        let hlc = Arc::new(aspen_hlc::create_hlc(name));

        let context = FederationProtocolContext {
            cluster_identity: identity.clone(),
            trust_manager: trust_manager.clone(),
            resource_settings: resource_settings.clone(),
            endpoint: Arc::new(endpoint.clone()),
            hlc,
            resource_resolver,
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
#[ignore]
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
#[ignore]
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
#[ignore]
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
#[ignore]
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
#[ignore]
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
#[ignore]
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
#[ignore]
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
#[ignore]
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
#[ignore]
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
#[ignore]
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
#[ignore]
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
        },
        // Bad object: hash doesn't match data
        SyncObject {
            object_type: "blob".to_string(),
            hash: bad_hash,
            data: bad_data.to_vec(),
            signature: None,
            signer: None,
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
#[ignore]
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
#[ignore]
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
#[ignore]
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
        },
        SyncObject {
            object_type: "blob".to_string(),
            hash: hash2,
            data: data2.to_vec(),
            signature: None,
            signer: None,
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
