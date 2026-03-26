//! Cluster integration tests for federation + auth.
//!
//! Tests that exercise credential-based authorization over real iroh QUIC
//! connections between simulated clusters. Validates the full handshake →
//! credential verification → resource access pipeline.
//!
//! # Running
//!
//! These tests require network access (iroh binds to loopback):
//!
//! ```sh
//! cargo nextest run -p aspen-federation \
//!     --test federation_auth_cluster_test --run-ignored all
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use aspen_auth::Capability;
use aspen_auth::Credential;
use aspen_auth::TokenBuilder;
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
use aspen_federation::sync::wire::read_message;
use aspen_federation::sync::wire::write_message;
use aspen_federation::trust::TrustManager;
use aspen_federation::types::FederatedId;
use aspen_federation::types::FederationMode;
use aspen_federation::types::FederationSettings;
use async_trait::async_trait;
use iroh::Endpoint;
use iroh::EndpointAddr;
use iroh::PublicKey;
use iroh::SecretKey;
use iroh::endpoint::Connection;
use iroh::protocol::Router;
use tokio::sync::RwLock;

// ============================================================================
// Test Infrastructure
// ============================================================================

/// Mock resource resolver that returns pre-configured objects.
struct MockResolver {
    objects: Arc<RwLock<Vec<SyncObject>>>,
}

impl MockResolver {
    fn new(objects: Vec<SyncObject>) -> Self {
        Self {
            objects: Arc::new(RwLock::new(objects)),
        }
    }

    fn empty() -> Self {
        Self::new(vec![])
    }
}

#[async_trait]
impl FederationResourceResolver for MockResolver {
    async fn get_resource_state(
        &self,
        _fed_id: &FederatedId,
    ) -> Result<FederationResourceState, FederationResourceError> {
        Ok(FederationResourceState {
            was_found: true,
            heads: HashMap::new(),
            metadata: None,
        })
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

/// A test cluster with iroh endpoint, identity, trust, and federation handler.
struct TestCluster {
    endpoint: Endpoint,
    identity: ClusterIdentity,
    trust_manager: Arc<TrustManager>,
    resource_settings: Arc<RwLock<HashMap<FederatedId, FederationSettings>>>,
    _router: Router,
}

impl TestCluster {
    async fn new(name: &str) -> Self {
        Self::with_resolver(name, None).await
    }

    async fn with_resolver(name: &str, resolver: Option<Arc<dyn FederationResourceResolver>>) -> Self {
        let secret_key = SecretKey::generate(&mut rand::rng());
        let identity = ClusterIdentity::generate(name.to_string());
        let trust_manager = Arc::new(TrustManager::new());
        let resource_settings = Arc::new(RwLock::new(HashMap::new()));

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
            resource_resolver: resolver,
            session_credential: std::sync::Mutex::new(None),
        };

        let handler = FederationProtocolHandler::new(context);
        let router = Router::builder(endpoint.clone()).accept(FEDERATION_ALPN, handler).spawn();

        Self {
            endpoint,
            identity,
            trust_manager,
            resource_settings,
            _router: router,
        }
    }

    fn cluster_key(&self) -> PublicKey {
        self.identity.public_key()
    }

    fn secret_key(&self) -> &SecretKey {
        self.identity.secret_key()
    }

    async fn add_resource(&self, fed_id: FederatedId, mode: FederationMode) {
        self.add_typed_resource(fed_id, mode, "forge:repo").await;
    }

    async fn add_typed_resource(&self, fed_id: FederatedId, mode: FederationMode, resource_type: &str) {
        let settings = match mode {
            FederationMode::Public => FederationSettings::public(),
            FederationMode::AllowList => FederationSettings::allowlist(vec![]),
            FederationMode::Disabled => FederationSettings::disabled(),
        }
        .with_resource_type(resource_type);
        self.resource_settings.write().await.insert(fed_id, settings);
    }

    async fn add_allowlist_resource(&self, fed_id: FederatedId, allowed: Vec<PublicKey>, resource_type: &str) {
        let settings = FederationSettings::allowlist(allowed).with_resource_type(resource_type);
        self.resource_settings.write().await.insert(fed_id, settings);
    }

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

fn test_fed_id(origin: PublicKey, name: &str) -> FederatedId {
    let local_id: [u8; 32] = blake3::hash(name.as_bytes()).into();
    FederatedId::new(origin, local_id)
}

/// Connect and perform handshake, optionally including a credential.
async fn connect_with_credential(
    our_endpoint: &Endpoint,
    our_identity: &ClusterIdentity,
    remote_addr: EndpointAddr,
    credential: Option<Credential>,
) -> anyhow::Result<(Connection, aspen_federation::identity::SignedClusterIdentity, bool)> {
    let connection = our_endpoint
        .connect(remote_addr, FEDERATION_ALPN)
        .await
        .map_err(|e| anyhow::anyhow!("connect: {}", e))?;

    let (mut send, mut recv) = connection.open_bi().await.map_err(|e| anyhow::anyhow!("open_bi: {}", e))?;

    let request = FederationRequest::Handshake {
        identity: our_identity.to_signed(),
        protocol_version: FEDERATION_PROTOCOL_VERSION,
        capabilities: vec!["forge".to_string()],
        credential,
    };
    write_message(&mut send, &request).await?;

    let response: FederationResponse = read_message(&mut recv).await?;

    match response {
        FederationResponse::Handshake { identity, trusted, .. } => {
            if !identity.verify() {
                anyhow::bail!("peer identity verification failed");
            }
            Ok((connection, identity, trusted))
        }
        FederationResponse::Error { code, message } => {
            anyhow::bail!("handshake error: {} - {}", code, message)
        }
        _ => anyhow::bail!("unexpected response"),
    }
}

/// Simple handshake without credential.
async fn connect_plain(
    our_endpoint: &Endpoint,
    our_identity: &ClusterIdentity,
    remote_addr: EndpointAddr,
) -> anyhow::Result<(Connection, aspen_federation::identity::SignedClusterIdentity, bool)> {
    connect_with_credential(our_endpoint, our_identity, remote_addr, None).await
}

// ============================================================================
// Credential-Based Trust Tests
// ============================================================================

/// A credential issued by bob's cluster key grants alice trusted status
/// during the federation handshake, even without prior trust configuration.
#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn test_credential_grants_trust_on_handshake() {
    let alice = TestCluster::new("alice").await;
    let bob = TestCluster::new("bob").await;

    // No explicit trust configured. Alice will present a credential issued by Bob.
    let credential = {
        let token = TokenBuilder::new(bob.secret_key().clone())
            .for_key(alice.cluster_key())
            .with_capability(Capability::Read {
                prefix: "forge:repo".into(),
            })
            .with_lifetime(Duration::from_secs(3600))
            .build()
            .unwrap();
        Credential::from_root(token)
    };

    let (_conn, peer_id, trusted) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_with_credential(&alice.endpoint, &alice.identity, bob.endpoint_addr(), Some(credential)),
    )
    .await
    .expect("timeout")
    .expect("handshake should succeed");

    assert_eq!(peer_id.name(), "bob");
    assert!(trusted, "alice should be trusted via credential");

    // Bob's trust manager should now reflect the credential-derived trust.
    assert!(bob.trust_manager.is_trusted(&alice.cluster_key()), "credential should have updated trust manager");
}

/// A credential issued by a different cluster (not bob) should fail
/// verification at bob, and alice stays untrusted.
#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn test_credential_from_wrong_issuer_rejected() {
    let alice = TestCluster::new("alice").await;
    let bob = TestCluster::new("bob").await;

    // Carol's key — not trusted by bob.
    let carol_sk = SecretKey::generate(&mut rand::rng());

    let bad_credential = {
        let token = TokenBuilder::new(carol_sk)
            .for_key(alice.cluster_key())
            .with_capability(Capability::Read {
                prefix: "forge:repo".into(),
            })
            .with_lifetime(Duration::from_secs(3600))
            .build()
            .unwrap();
        Credential::from_root(token)
    };

    let result = tokio::time::timeout(
        Duration::from_secs(10),
        connect_with_credential(&alice.endpoint, &alice.identity, bob.endpoint_addr(), Some(bad_credential)),
    )
    .await
    .expect("timeout");

    // Handshake should fail with INVALID_CREDENTIAL
    assert!(result.is_err(), "wrong-issuer credential should be rejected");
    let err = result.unwrap_err().to_string();
    assert!(err.contains("INVALID_CREDENTIAL"), "error should mention INVALID_CREDENTIAL, got: {}", err);

    // Alice should remain untrusted.
    assert!(
        !bob.trust_manager.is_trusted(&alice.cluster_key()),
        "alice should not be trusted after failed credential"
    );
}

/// A credential with a tampered signature should be rejected during
/// handshake. This validates the handler's signature verification.
#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn test_tampered_credential_rejected() {
    let alice = TestCluster::new("alice").await;
    let bob = TestCluster::new("bob").await;

    let tampered_credential = {
        let mut token = TokenBuilder::new(bob.secret_key().clone())
            .for_key(alice.cluster_key())
            .with_capability(Capability::Read {
                prefix: "forge:repo".into(),
            })
            .with_lifetime(Duration::from_secs(3600))
            .build()
            .unwrap();
        // Corrupt the signature.
        token.signature[0] ^= 0xff;
        token.signature[1] ^= 0xff;
        Credential::from_root(token)
    };

    let result = tokio::time::timeout(
        Duration::from_secs(10),
        connect_with_credential(&alice.endpoint, &alice.identity, bob.endpoint_addr(), Some(tampered_credential)),
    )
    .await
    .expect("timeout");

    assert!(result.is_err(), "tampered credential should be rejected");
    let err = result.unwrap_err().to_string();
    assert!(err.contains("INVALID_CREDENTIAL"), "expected INVALID_CREDENTIAL, got: {}", err);
}

/// A token with a very short lifetime (within clock skew tolerance) is
/// still accepted. This validates the 60-second skew tolerance.
#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn test_recently_expired_credential_within_skew_accepted() {
    let alice = TestCluster::new("alice").await;
    let bob = TestCluster::new("bob").await;

    // 1-second lifetime, wait 2 seconds — still within 60s skew tolerance.
    let credential = {
        let token = TokenBuilder::new(bob.secret_key().clone())
            .for_key(alice.cluster_key())
            .with_capability(Capability::Read {
                prefix: "forge:repo".into(),
            })
            .with_lifetime(Duration::from_secs(1))
            .build()
            .unwrap();
        Credential::from_root(token)
    };

    tokio::time::sleep(Duration::from_secs(2)).await;

    let result = tokio::time::timeout(
        Duration::from_secs(10),
        connect_with_credential(&alice.endpoint, &alice.identity, bob.endpoint_addr(), Some(credential)),
    )
    .await
    .expect("timeout");

    // Should succeed — within clock skew tolerance.
    assert!(result.is_ok(), "recently expired credential should be accepted within clock skew tolerance");
}

/// Plain handshake without credential: alice is untrusted and can still
/// access public resources but not allowlisted ones.
#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn test_plain_handshake_untrusted() {
    let alice = TestCluster::new("alice").await;
    let bob = TestCluster::new("bob").await;

    let (_conn, peer_id, trusted) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_plain(&alice.endpoint, &alice.identity, bob.endpoint_addr()),
    )
    .await
    .expect("timeout")
    .expect("plain handshake should succeed");

    assert_eq!(peer_id.name(), "bob");
    assert!(!trusted, "alice should be untrusted without credential or explicit trust");
}

// ============================================================================
// Credential-Based Resource Access Tests
// ============================================================================

/// With a valid credential, alice can access allowlisted resources on bob
/// even though alice's cluster key is not in the allowlist.
#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn test_credential_grants_access_to_restricted_resource() {
    let alice = TestCluster::new("alice").await;
    let bob = TestCluster::with_resolver("bob", Some(Arc::new(MockResolver::empty()))).await;

    // Bob has a restricted resource (allowlist mode, alice's key NOT in it).
    let repo = test_fed_id(bob.cluster_key(), "private-repo");
    bob.add_allowlist_resource(repo, vec![], "forge:repo").await;

    // Bob issues a credential to alice for forge:repo reads.
    let credential = {
        let token = TokenBuilder::new(bob.secret_key().clone())
            .for_key(alice.cluster_key())
            .with_capability(Capability::Read {
                prefix: "forge:repo".into(),
            })
            .with_lifetime(Duration::from_secs(3600))
            .build()
            .unwrap();
        Credential::from_root(token)
    };

    let (conn, _, trusted) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_with_credential(&alice.endpoint, &alice.identity, bob.endpoint_addr(), Some(credential)),
    )
    .await
    .expect("timeout")
    .expect("handshake");

    assert!(trusted);

    // Alice should now be able to query the restricted resource.
    let (was_found, _, _) = tokio::time::timeout(Duration::from_secs(5), get_remote_resource_state(&conn, &repo))
        .await
        .expect("timeout")
        .expect("resource state query should succeed");

    assert!(was_found, "credential should grant access to allowlisted resource");
}

/// Without a credential, alice cannot access an allowlisted resource on bob.
#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn test_no_credential_blocked_from_restricted_resource() {
    let alice = TestCluster::new("alice").await;
    let bob = TestCluster::with_resolver("bob", Some(Arc::new(MockResolver::empty()))).await;

    // Allowlist resource with no allowed clusters.
    let repo = test_fed_id(bob.cluster_key(), "secret-repo");
    bob.add_allowlist_resource(repo, vec![], "forge:repo").await;

    // Alice connects without credential, bob doesn't trust alice.
    let (conn, _, trusted) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_plain(&alice.endpoint, &alice.identity, bob.endpoint_addr()),
    )
    .await
    .expect("timeout")
    .expect("plain handshake");

    assert!(!trusted);

    // Querying the restricted resource should be denied.
    let result = tokio::time::timeout(Duration::from_secs(5), get_remote_resource_state(&conn, &repo))
        .await
        .expect("timeout");

    assert!(result.is_err(), "untrusted peer should be denied access to allowlisted resource");
}

/// Public resources remain accessible without any credential.
#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn test_public_resource_accessible_without_credential() {
    let alice = TestCluster::new("alice").await;
    let bob = TestCluster::with_resolver("bob", Some(Arc::new(MockResolver::empty()))).await;

    let repo = test_fed_id(bob.cluster_key(), "open-source-repo");
    bob.add_resource(repo, FederationMode::Public).await;

    // No trust, no credential — just plain connect.
    let (conn, _, _) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_plain(&alice.endpoint, &alice.identity, bob.endpoint_addr()),
    )
    .await
    .expect("timeout")
    .expect("plain handshake");

    let (was_found, _, _) = tokio::time::timeout(Duration::from_secs(5), get_remote_resource_state(&conn, &repo))
        .await
        .expect("timeout")
        .expect("public resource should be accessible");

    assert!(was_found);
}

// ============================================================================
// Delegation Chain Tests
// ============================================================================

/// A two-level delegation chain works: bob issues to carol, carol delegates
/// to alice, and alice can use the delegated credential to access bob.
#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn test_delegated_credential_two_levels() {
    let alice = TestCluster::new("alice").await;
    let bob = TestCluster::with_resolver("bob", Some(Arc::new(MockResolver::empty()))).await;

    let carol_sk = SecretKey::generate(&mut rand::rng());
    let carol_pk = carol_sk.public();

    // Bob issues root token to carol with delegation rights.
    let root_cred = {
        let token = TokenBuilder::new(bob.secret_key().clone())
            .for_key(carol_pk)
            .with_capability(Capability::Read {
                prefix: "forge:repo".into(),
            })
            .with_capability(Capability::Delegate)
            .with_lifetime(Duration::from_secs(3600))
            .build()
            .unwrap();
        Credential::from_root(token)
    };

    // Carol delegates a narrower credential to alice.
    let alice_cred = root_cred
        .delegate(
            &carol_sk,
            alice.cluster_key(),
            vec![Capability::Read {
                prefix: "forge:repo".into(),
            }],
            Duration::from_secs(1800),
        )
        .unwrap();

    assert_eq!(alice_cred.proofs.len(), 1);
    assert_eq!(alice_cred.token.delegation_depth, 1);

    // Add a restricted resource on bob.
    let repo = test_fed_id(bob.cluster_key(), "delegated-repo");
    bob.add_allowlist_resource(repo, vec![], "forge:repo").await;

    let (conn, _, trusted) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_with_credential(&alice.endpoint, &alice.identity, bob.endpoint_addr(), Some(alice_cred)),
    )
    .await
    .expect("timeout")
    .expect("delegated credential handshake");

    assert!(trusted, "delegated credential should be accepted");

    let (was_found, _, _) = tokio::time::timeout(Duration::from_secs(5), get_remote_resource_state(&conn, &repo))
        .await
        .expect("timeout")
        .expect("access with delegated credential");

    assert!(was_found);
}

/// Escalated capabilities in a delegation chain are rejected. Carol has
/// Read but tries to delegate Write to alice.
#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn test_delegation_escalation_rejected_locally() {
    let bob_sk = SecretKey::generate(&mut rand::rng());
    let carol_sk = SecretKey::generate(&mut rand::rng());
    let carol_pk = carol_sk.public();
    let alice_pk = SecretKey::generate(&mut rand::rng()).public();

    // Bob gives carol Read + Delegate.
    let root_cred = {
        let token = TokenBuilder::new(bob_sk)
            .for_key(carol_pk)
            .with_capability(Capability::Read { prefix: "data:".into() })
            .with_capability(Capability::Delegate)
            .with_lifetime(Duration::from_secs(3600))
            .build()
            .unwrap();
        Credential::from_root(token)
    };

    // Carol tries to delegate Write (escalation).
    let result = root_cred.delegate(
        &carol_sk,
        alice_pk,
        vec![Capability::Write { prefix: "data:".into() }],
        Duration::from_secs(1800),
    );

    assert!(result.is_err(), "escalation from Read to Write should be rejected");
    let err = format!("{:?}", result.unwrap_err());
    assert!(err.contains("CapabilityEscalation"), "got: {}", err);
}

// ============================================================================
// Trust Manager Integration Tests
// ============================================================================

/// Credential-derived trust expires when expire_credential is called,
/// reverting the cluster to Public trust level.
#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn test_credential_trust_expiration() {
    let alice = TestCluster::new("alice").await;
    let bob = TestCluster::new("bob").await;

    let credential = {
        let token = TokenBuilder::new(bob.secret_key().clone())
            .for_key(alice.cluster_key())
            .with_capability(Capability::Read {
                prefix: "forge:repo".into(),
            })
            .with_lifetime(Duration::from_secs(3600))
            .build()
            .unwrap();
        Credential::from_root(token)
    };

    // Handshake with credential establishes trust.
    let (_conn, _, trusted) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_with_credential(&alice.endpoint, &alice.identity, bob.endpoint_addr(), Some(credential)),
    )
    .await
    .expect("timeout")
    .expect("handshake");

    assert!(trusted);
    assert!(bob.trust_manager.is_trusted(&alice.cluster_key()));

    // Simulate credential expiration.
    bob.trust_manager.expire_credential(&alice.cluster_key());

    assert!(
        !bob.trust_manager.is_trusted(&alice.cluster_key()),
        "expired credential should revert trust to Public"
    );
}

/// Revoking a credential blocks the cluster entirely.
#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn test_credential_revocation_blocks() {
    let alice = TestCluster::new("alice").await;
    let bob = TestCluster::new("bob").await;

    let credential = {
        let token = TokenBuilder::new(bob.secret_key().clone())
            .for_key(alice.cluster_key())
            .with_capability(Capability::Read {
                prefix: "forge:repo".into(),
            })
            .with_lifetime(Duration::from_secs(3600))
            .build()
            .unwrap();
        Credential::from_root(token)
    };

    // Establish trust via credential.
    let (_conn, _, _) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_with_credential(&alice.endpoint, &alice.identity, bob.endpoint_addr(), Some(credential)),
    )
    .await
    .expect("timeout")
    .expect("handshake");

    assert!(bob.trust_manager.is_trusted(&alice.cluster_key()));

    // Revoke.
    bob.trust_manager.revoke_credential(alice.cluster_key());

    assert!(bob.trust_manager.is_blocked(&alice.cluster_key()), "revoked credential should block the cluster");
    assert!(!bob.trust_manager.is_trusted(&alice.cluster_key()));
}

// ============================================================================
// Multi-Cluster Federation Topology Tests
// ============================================================================

/// Hub-and-spoke topology: hub issues credentials to both spokes.
/// Each spoke can access hub's resources via its credential, but spokes
/// cannot access each other directly.
#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn test_hub_spoke_topology_with_credentials() {
    let hub = TestCluster::with_resolver("hub", Some(Arc::new(MockResolver::empty()))).await;
    let spoke_a = TestCluster::new("spoke-a").await;
    let spoke_b = TestCluster::new("spoke-b").await;

    // Hub has a restricted resource.
    let repo = test_fed_id(hub.cluster_key(), "shared-data");
    hub.add_allowlist_resource(repo, vec![], "forge:repo").await;

    // Hub issues credentials to both spokes.
    let spoke_a_cred = {
        let token = TokenBuilder::new(hub.secret_key().clone())
            .for_key(spoke_a.cluster_key())
            .with_capability(Capability::Read {
                prefix: "forge:repo".into(),
            })
            .with_lifetime(Duration::from_secs(3600))
            .build()
            .unwrap();
        Credential::from_root(token)
    };

    let spoke_b_cred = {
        let token = TokenBuilder::new(hub.secret_key().clone())
            .for_key(spoke_b.cluster_key())
            .with_capability(Capability::Read {
                prefix: "forge:repo".into(),
            })
            .with_lifetime(Duration::from_secs(3600))
            .build()
            .unwrap();
        Credential::from_root(token)
    };

    // Spoke A connects to hub with credential.
    let (conn_a, _, trusted_a) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_with_credential(&spoke_a.endpoint, &spoke_a.identity, hub.endpoint_addr(), Some(spoke_a_cred)),
    )
    .await
    .expect("timeout")
    .expect("spoke_a handshake");

    assert!(trusted_a);

    let (found_a, _, _) = tokio::time::timeout(Duration::from_secs(5), get_remote_resource_state(&conn_a, &repo))
        .await
        .expect("timeout")
        .expect("spoke_a resource access");

    assert!(found_a, "spoke_a should access hub resource");

    // Spoke B connects to hub with credential.
    let (conn_b, _, trusted_b) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_with_credential(&spoke_b.endpoint, &spoke_b.identity, hub.endpoint_addr(), Some(spoke_b_cred)),
    )
    .await
    .expect("timeout")
    .expect("spoke_b handshake");

    assert!(trusted_b);

    let (found_b, _, _) = tokio::time::timeout(Duration::from_secs(5), get_remote_resource_state(&conn_b, &repo))
        .await
        .expect("timeout")
        .expect("spoke_b resource access");

    assert!(found_b, "spoke_b should access hub resource");
}

/// Mutual federation: two clusters exchange credentials and can access
/// each other's resources.
#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn test_mutual_credential_exchange() {
    let alice = TestCluster::with_resolver("alice", Some(Arc::new(MockResolver::empty()))).await;
    let bob = TestCluster::with_resolver("bob", Some(Arc::new(MockResolver::empty()))).await;

    // Both clusters have restricted resources.
    let alice_repo = test_fed_id(alice.cluster_key(), "alice-private");
    alice.add_allowlist_resource(alice_repo, vec![], "forge:repo").await;

    let bob_repo = test_fed_id(bob.cluster_key(), "bob-private");
    bob.add_allowlist_resource(bob_repo, vec![], "forge:repo").await;

    // Exchange credentials.
    let alice_to_bob_cred = {
        let token = TokenBuilder::new(bob.secret_key().clone())
            .for_key(alice.cluster_key())
            .with_capability(Capability::Read {
                prefix: "forge:repo".into(),
            })
            .with_lifetime(Duration::from_secs(3600))
            .build()
            .unwrap();
        Credential::from_root(token)
    };

    let bob_to_alice_cred = {
        let token = TokenBuilder::new(alice.secret_key().clone())
            .for_key(bob.cluster_key())
            .with_capability(Capability::Read {
                prefix: "forge:repo".into(),
            })
            .with_lifetime(Duration::from_secs(3600))
            .build()
            .unwrap();
        Credential::from_root(token)
    };

    // Alice accesses bob.
    let (conn_ab, _, _) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_with_credential(&alice.endpoint, &alice.identity, bob.endpoint_addr(), Some(alice_to_bob_cred)),
    )
    .await
    .expect("timeout")
    .expect("alice->bob");

    let (found, _, _) = tokio::time::timeout(Duration::from_secs(5), get_remote_resource_state(&conn_ab, &bob_repo))
        .await
        .expect("timeout")
        .expect("alice reads bob's repo");

    assert!(found);

    // Bob accesses alice.
    let (conn_ba, _, _) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_with_credential(&bob.endpoint, &bob.identity, alice.endpoint_addr(), Some(bob_to_alice_cred)),
    )
    .await
    .expect("timeout")
    .expect("bob->alice");

    let (found, _, _) = tokio::time::timeout(Duration::from_secs(5), get_remote_resource_state(&conn_ba, &alice_repo))
        .await
        .expect("timeout")
        .expect("bob reads alice's repo");

    assert!(found);
}

// ============================================================================
// Object Sync with Auth Tests
// ============================================================================

/// Credential authorizes sync of objects from a restricted resource.
#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn test_sync_objects_with_credential() {
    let data1 = b"commit-object-1";
    let hash1 = *blake3::hash(data1).as_bytes();
    let data2 = b"commit-object-2";
    let hash2 = *blake3::hash(data2).as_bytes();

    let mock_objects = vec![
        SyncObject {
            object_type: "commit".to_string(),
            hash: hash1,
            data: data1.to_vec(),
            signature: None,
            signer: None,
        },
        SyncObject {
            object_type: "commit".to_string(),
            hash: hash2,
            data: data2.to_vec(),
            signature: None,
            signer: None,
        },
    ];

    let alice = TestCluster::new("alice").await;
    let bob = TestCluster::with_resolver("bob", Some(Arc::new(MockResolver::new(mock_objects)))).await;

    let repo = test_fed_id(bob.cluster_key(), "synced-repo");
    bob.add_allowlist_resource(repo, vec![], "forge:repo").await;

    let credential = {
        let token = TokenBuilder::new(bob.secret_key().clone())
            .for_key(alice.cluster_key())
            .with_capability(Capability::Read {
                prefix: "forge:repo".into(),
            })
            .with_lifetime(Duration::from_secs(3600))
            .build()
            .unwrap();
        Credential::from_root(token)
    };

    let (conn, _, trusted) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_with_credential(&alice.endpoint, &alice.identity, bob.endpoint_addr(), Some(credential)),
    )
    .await
    .expect("timeout")
    .expect("handshake");

    assert!(trusted);

    let (objects, has_more) = tokio::time::timeout(
        Duration::from_secs(10),
        sync_remote_objects(&conn, &repo, vec!["commit".to_string()], vec![], 100, None),
    )
    .await
    .expect("timeout")
    .expect("sync objects");

    assert_eq!(objects.len(), 2, "should sync both objects");
    assert!(!has_more);

    // Verify content hashes.
    assert_eq!(objects[0].hash, hash1);
    assert_eq!(objects[1].hash, hash2);
}

/// Without credential, syncing a restricted resource fails with ACCESS_DENIED.
#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn test_sync_objects_denied_without_credential() {
    let alice = TestCluster::new("alice").await;
    let bob = TestCluster::with_resolver("bob", Some(Arc::new(MockResolver::empty()))).await;

    let repo = test_fed_id(bob.cluster_key(), "locked-repo");
    bob.add_allowlist_resource(repo, vec![], "forge:repo").await;

    // Connect without credential.
    let (conn, _, trusted) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_plain(&alice.endpoint, &alice.identity, bob.endpoint_addr()),
    )
    .await
    .expect("timeout")
    .expect("plain handshake");

    assert!(!trusted);

    let result = tokio::time::timeout(
        Duration::from_secs(10),
        sync_remote_objects(&conn, &repo, vec!["commit".to_string()], vec![], 100, None),
    )
    .await
    .expect("timeout");

    assert!(result.is_err(), "sync should fail without credential");
    let err = result.unwrap_err().to_string();
    assert!(err.contains("ACCESS_DENIED"), "expected ACCESS_DENIED, got: {}", err);
}

// ============================================================================
// Allowlist with Explicit Cluster Key Tests
// ============================================================================

/// When alice's endpoint ID is explicitly in the allowlist, no credential is needed.
/// Note: the allowlist checks the iroh endpoint key (connection identity),
/// not the cluster identity key.
#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn test_allowlist_with_explicit_key() {
    let alice = TestCluster::new("alice").await;
    let bob = TestCluster::with_resolver("bob", Some(Arc::new(MockResolver::empty()))).await;

    // Bob allows alice's iroh endpoint key (the QUIC connection identity).
    let alice_endpoint_key = alice.endpoint.id();
    let repo = test_fed_id(bob.cluster_key(), "explicit-allow-repo");
    bob.add_allowlist_resource(repo, vec![alice_endpoint_key], "forge:repo").await;

    // Plain connect (no credential).
    let (conn, _, _) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_plain(&alice.endpoint, &alice.identity, bob.endpoint_addr()),
    )
    .await
    .expect("timeout")
    .expect("plain handshake");

    let (was_found, _, _) = tokio::time::timeout(Duration::from_secs(5), get_remote_resource_state(&conn, &repo))
        .await
        .expect("timeout")
        .expect("resource access with explicit allowlist");

    assert!(was_found, "alice should access resource when her key is in the allowlist");
}

/// A valid credential overrides the disabled mode for resource access.
/// This is the expected behavior: credentials act as an admin override
/// for per-resource federation settings.
#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn test_credential_overrides_disabled_resource() {
    let alice = TestCluster::new("alice").await;
    let bob = TestCluster::with_resolver("bob", Some(Arc::new(MockResolver::empty()))).await;

    let repo = test_fed_id(bob.cluster_key(), "disabled-repo");
    bob.add_typed_resource(repo, FederationMode::Disabled, "forge:repo").await;

    let credential = {
        let token = TokenBuilder::new(bob.secret_key().clone())
            .for_key(alice.cluster_key())
            .with_capability(Capability::Read {
                prefix: "forge:repo".into(),
            })
            .with_lifetime(Duration::from_secs(3600))
            .build()
            .unwrap();
        Credential::from_root(token)
    };

    let (conn, _, _) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_with_credential(&alice.endpoint, &alice.identity, bob.endpoint_addr(), Some(credential)),
    )
    .await
    .expect("timeout")
    .expect("handshake");

    // Credential overrides disabled mode — access is granted.
    let (was_found, _, _) = tokio::time::timeout(Duration::from_secs(5), get_remote_resource_state(&conn, &repo))
        .await
        .expect("timeout")
        .expect("credential should override disabled mode");

    assert!(was_found);
}

/// Without a credential, a disabled resource is inaccessible.
#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn test_disabled_resource_inaccessible_without_credential() {
    let alice = TestCluster::new("alice").await;
    let bob = TestCluster::with_resolver("bob", Some(Arc::new(MockResolver::empty()))).await;

    let repo = test_fed_id(bob.cluster_key(), "disabled-repo");
    bob.add_typed_resource(repo, FederationMode::Disabled, "forge:repo").await;

    let (conn, _, _) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_plain(&alice.endpoint, &alice.identity, bob.endpoint_addr()),
    )
    .await
    .expect("timeout")
    .expect("handshake");

    let result = tokio::time::timeout(Duration::from_secs(5), get_remote_resource_state(&conn, &repo))
        .await
        .expect("timeout");

    assert!(result.is_err(), "disabled resource should be inaccessible without credential");
}

// ============================================================================
// Resource Listing with Auth Tests
// ============================================================================

/// Resource listing respects federation mode visibility regardless of trust.
/// Public resources are always listed; disabled ones never are.
#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn test_resource_listing_visibility() {
    let alice = TestCluster::new("alice").await;
    let bob = TestCluster::new("bob").await;

    // Bob has mixed resources.
    let public_repo = test_fed_id(bob.cluster_key(), "public");
    bob.add_typed_resource(public_repo, FederationMode::Public, "forge:repo").await;

    let disabled_repo = test_fed_id(bob.cluster_key(), "disabled");
    bob.add_typed_resource(disabled_repo, FederationMode::Disabled, "forge:repo").await;

    let allowlist_repo = test_fed_id(bob.cluster_key(), "restricted");
    bob.add_allowlist_resource(allowlist_repo, vec![], "forge:repo").await;

    // Untrusted listing.
    let (conn, _, _) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_plain(&alice.endpoint, &alice.identity, bob.endpoint_addr()),
    )
    .await
    .expect("timeout")
    .expect("handshake");

    let resources = tokio::time::timeout(Duration::from_secs(5), list_remote_resources(&conn, None, 100))
        .await
        .expect("timeout")
        .expect("list");

    // Only public and allowlist resources are listed (disabled is excluded).
    // The listing shows available resources; access control is applied per-resource.
    let listed_ids: Vec<FederatedId> = resources.iter().map(|r| r.fed_id).collect();
    assert!(listed_ids.contains(&public_repo), "public should be listed");
    assert!(!listed_ids.contains(&disabled_repo), "disabled should not be listed");
}
