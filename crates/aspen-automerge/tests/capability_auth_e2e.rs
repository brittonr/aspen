//! End-to-end tests proving capability-based auth works on the sync protocol.
//!
//! Each test spins up real iroh endpoints, connects them, and verifies that
//! the sync handler correctly accepts or rejects requests based on the
//! capability token presented.

use std::sync::Arc;
use std::time::Duration;

use aspen_auth::Capability;
use aspen_auth::TokenBuilder;
use aspen_automerge::AUTOMERGE_SYNC_ALPN;
use aspen_automerge::AutomergeSyncHandler;
use aspen_automerge::CapabilityToken;
use aspen_automerge::DocumentId;
use aspen_automerge::DocumentStore;
use aspen_automerge::sync_with_peer;
use aspen_automerge::sync_with_peer_cap;
use aspen_testing::DeterministicKeyValueStore;
use automerge::ReadDoc;
use automerge::transaction::Transactable;
use iroh::Endpoint;
use iroh::protocol::Router;

type Store = aspen_automerge::AspenAutomergeStore<DeterministicKeyValueStore>;

/// Create a fresh store with an optional pre-populated document.
async fn make_store(doc_id: &DocumentId, content: Option<&str>) -> Arc<Store> {
    let kv = DeterministicKeyValueStore::new();
    let store = Arc::new(aspen_automerge::AspenAutomergeStore::new(kv));
    store.create(Some(doc_id.clone()), None).await.unwrap();
    if let Some(text) = content {
        let mut doc = store.get(doc_id).await.unwrap().unwrap();
        doc.put(automerge::ROOT, "content", text).unwrap();
        store.save(doc_id, &mut doc).await.unwrap();
    }
    store
}

/// Spin up an endpoint + router with the automerge sync handler.
/// Returns (endpoint, router, store).
async fn make_server(store: Arc<Store>, auth_key: Option<iroh::PublicKey>) -> (Endpoint, Router, Arc<Store>) {
    let endpoint = Endpoint::builder().bind().await.unwrap();
    let handler: Arc<AutomergeSyncHandler<Store>> = match auth_key {
        Some(key) => Arc::new(AutomergeSyncHandler::with_capability_auth(store.clone(), key)),
        None => Arc::new(AutomergeSyncHandler::new(store.clone())),
    };
    let router = Router::builder(endpoint.clone()).accept(AUTOMERGE_SYNC_ALPN, handler).spawn();
    (endpoint, router, store)
}

/// Connect a client endpoint to a server and sync a document.
async fn try_sync(
    server_endpoint: &Endpoint,
    client_store: &Store,
    doc_id: &DocumentId,
    capability: Option<&CapabilityToken>,
) -> Result<(), aspen_automerge::SyncError> {
    let client_ep = Endpoint::builder().bind().await.unwrap();
    let server_addr = server_endpoint.addr();
    let conn = client_ep
        .connect(server_addr, AUTOMERGE_SYNC_ALPN)
        .await
        .map_err(|e| aspen_automerge::SyncError::Io(e.to_string()))?;

    let result = match capability {
        Some(cap) => sync_with_peer_cap(client_store, doc_id, &conn, Some(cap)).await,
        None => sync_with_peer(client_store, doc_id, &conn).await,
    };

    conn.close(0u32.into(), b"done");
    client_ep.close().await;
    result
}

/// Generate a capability token signed by the given secret key.
fn make_token(secret_key: &iroh::SecretKey, capability: Capability, lifetime_secs: u64) -> CapabilityToken {
    TokenBuilder::new(secret_key.clone())
        .with_capability(capability)
        .with_lifetime(Duration::from_secs(lifetime_secs))
        .build()
        .unwrap()
}

// ============================================================================
// Tests
// ============================================================================

/// Valid wildcard token: sync succeeds, document content transfers.
#[tokio::test]
async fn valid_wildcard_token_syncs_document() {
    let secret_key = iroh::SecretKey::generate(&mut rand::rngs::ThreadRng::default());
    let doc_id = DocumentId::from_string("test-doc").unwrap();

    // Server has a document with content
    let server_store = make_store(&doc_id, Some("hello from server")).await;
    let (server_ep, _router, _) = make_server(server_store.clone(), Some(secret_key.public())).await;

    // Client has an empty document
    let client_store = make_store(&doc_id, None).await;

    // Token: wildcard automerge access
    let token = make_token(
        &secret_key,
        Capability::Full {
            prefix: "automerge:".into(),
        },
        3600,
    );

    // Sync should succeed
    let result = try_sync(&server_ep, &client_store, &doc_id, Some(&token)).await;
    assert!(result.is_ok(), "sync should succeed: {:?}", result.err());

    // Verify content arrived
    let doc = client_store.get(&doc_id).await.unwrap().unwrap();
    let val = doc
        .get(automerge::ROOT, "content")
        .unwrap()
        .map(|(v, _): (automerge::Value<'_>, _)| v.into_string().unwrap());
    assert_eq!(val.as_deref(), Some("hello from server"));

    server_ep.close().await;
}

/// No token when auth is required: sync rejected.
#[tokio::test]
async fn no_token_rejected() {
    let secret_key = iroh::SecretKey::generate(&mut rand::rngs::ThreadRng::default());
    let doc_id = DocumentId::from_string("test-doc").unwrap();

    let server_store = make_store(&doc_id, Some("secret data")).await;
    let (server_ep, _router, _) = make_server(server_store, Some(secret_key.public())).await;

    let client_store = make_store(&doc_id, None).await;

    // No capability token
    let result = try_sync(&server_ep, &client_store, &doc_id, None).await;
    assert!(result.is_err(), "sync without token should be rejected");
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("capability token required"), "error should mention missing token: {}", err_msg);

    // Document should remain empty
    let doc = client_store.get(&doc_id).await.unwrap().unwrap();
    assert!(doc.get(automerge::ROOT, "content").unwrap().is_none());

    server_ep.close().await;
}

/// Token signed by wrong key: sync rejected.
#[tokio::test]
async fn wrong_key_rejected() {
    let server_key = iroh::SecretKey::generate(&mut rand::rngs::ThreadRng::default());
    let attacker_key = iroh::SecretKey::generate(&mut rand::rngs::ThreadRng::default());
    let doc_id = DocumentId::from_string("test-doc").unwrap();

    let server_store = make_store(&doc_id, Some("secret data")).await;
    let (server_ep, _router, _) = make_server(server_store, Some(server_key.public())).await;

    let client_store = make_store(&doc_id, None).await;

    // Token signed by attacker, not the server
    let bad_token = make_token(
        &attacker_key,
        Capability::Full {
            prefix: "automerge:".into(),
        },
        3600,
    );

    let result = try_sync(&server_ep, &client_store, &doc_id, Some(&bad_token)).await;
    assert!(result.is_err(), "wrong-key token should be rejected");

    // Document should remain empty
    let doc = client_store.get(&doc_id).await.unwrap().unwrap();
    assert!(doc.get(automerge::ROOT, "content").unwrap().is_none());

    server_ep.close().await;
}

/// Token scoped to wrong document prefix: sync rejected.
#[tokio::test]
async fn wrong_document_scope_rejected() {
    let secret_key = iroh::SecretKey::generate(&mut rand::rngs::ThreadRng::default());
    let doc_id = DocumentId::from_string("test-doc").unwrap();

    let server_store = make_store(&doc_id, Some("secret data")).await;
    let (server_ep, _router, _) = make_server(server_store, Some(secret_key.public())).await;

    let client_store = make_store(&doc_id, None).await;

    // Token scoped to a DIFFERENT document prefix
    let narrow_token = make_token(
        &secret_key,
        Capability::Full {
            prefix: "automerge:other-doc".into(),
        },
        3600,
    );

    let result = try_sync(&server_ep, &client_store, &doc_id, Some(&narrow_token)).await;
    assert!(result.is_err(), "wrong-scope token should be rejected");

    // Document should remain empty
    let doc = client_store.get(&doc_id).await.unwrap().unwrap();
    assert!(doc.get(automerge::ROOT, "content").unwrap().is_none());

    server_ep.close().await;
}

/// Token scoped to specific document: sync succeeds for that document.
#[tokio::test]
async fn document_scoped_token_works() {
    let secret_key = iroh::SecretKey::generate(&mut rand::rngs::ThreadRng::default());
    let doc_id = DocumentId::from_string("my-doc").unwrap();

    let server_store = make_store(&doc_id, Some("scoped content")).await;
    let (server_ep, _router, _) = make_server(server_store, Some(secret_key.public())).await;

    let client_store = make_store(&doc_id, None).await;

    // Token scoped exactly to this document
    let scoped_token = make_token(
        &secret_key,
        Capability::Full {
            prefix: "automerge:my-doc".into(),
        },
        3600,
    );

    let result = try_sync(&server_ep, &client_store, &doc_id, Some(&scoped_token)).await;
    assert!(result.is_ok(), "scoped token should work: {:?}", result.err());

    let doc = client_store.get(&doc_id).await.unwrap().unwrap();
    let val = doc
        .get(automerge::ROOT, "content")
        .unwrap()
        .map(|(v, _): (automerge::Value<'_>, _)| v.into_string().unwrap());
    assert_eq!(val.as_deref(), Some("scoped content"));

    server_ep.close().await;
}

/// No auth required (handler created with ::new()): sync works without token.
#[tokio::test]
async fn no_auth_mode_allows_all() {
    let doc_id = DocumentId::from_string("open-doc").unwrap();

    let server_store = make_store(&doc_id, Some("open data")).await;
    // No auth key — handler created with ::new()
    let (server_ep, _router, _) = make_server(server_store, None).await;

    let client_store = make_store(&doc_id, None).await;

    let result = try_sync(&server_ep, &client_store, &doc_id, None).await;
    assert!(result.is_ok(), "no-auth mode should allow: {:?}", result.err());

    let doc = client_store.get(&doc_id).await.unwrap().unwrap();
    let val = doc
        .get(automerge::ROOT, "content")
        .unwrap()
        .map(|(v, _): (automerge::Value<'_>, _)| v.into_string().unwrap());
    assert_eq!(val.as_deref(), Some("open data"));

    server_ep.close().await;
}

/// Read-only token rejected (sync requires Write).
#[tokio::test]
async fn read_only_token_rejected() {
    let secret_key = iroh::SecretKey::generate(&mut rand::rngs::ThreadRng::default());
    let doc_id = DocumentId::from_string("test-doc").unwrap();

    let server_store = make_store(&doc_id, Some("data")).await;
    let (server_ep, _router, _) = make_server(server_store, Some(secret_key.public())).await;

    let client_store = make_store(&doc_id, None).await;

    // Read-only token — sync requires Write
    let read_token = make_token(
        &secret_key,
        Capability::Read {
            prefix: "automerge:".into(),
        },
        3600,
    );

    let result = try_sync(&server_ep, &client_store, &doc_id, Some(&read_token)).await;
    assert!(result.is_err(), "read-only token should be rejected for sync");

    server_ep.close().await;
}
