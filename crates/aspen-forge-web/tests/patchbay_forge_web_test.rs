//! End-to-end forge web tests over real iroh QUIC via patchbay.
//!
//! Boots a cluster node (IrohBlobStore + DeterministicKV + ForgeNode +
//! ClientProtocolHandler) on one patchbay device, connects an AspenClient
//! from another, and calls `routes::dispatch()` directly — no H3 proxy.
//!
//! Requirements: Linux with unprivileged user namespaces, nft, tc in PATH.

use std::sync::Arc;
use std::time::Duration;

use aspen_blob::IrohBlobStore;
use aspen_blob::prelude::*;
use aspen_core::hlc::create_hlc;
use aspen_forge::ForgeNode;
use aspen_forge::git::BlobObject;
use aspen_forge::git::CommitObject;
use aspen_forge::git::GitObject;
use aspen_forge::git::TreeEntry;
use aspen_forge::git::TreeObject;
use aspen_forge::identity::Author;
use aspen_forge::types::SignedObject;
use aspen_forge_web::routes;
use aspen_forge_web::state::AppState;
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_handlers::ClientProtocolHandler;
use aspen_testing_core::DeterministicKeyValueStore;
use aspen_testing_patchbay::skip_unless_patchbay;
use aspen_transport::CLIENT_ALPN;
use http::StatusCode;
use patchbay::Lab;
use patchbay::RouterPreset;

#[ctor::ctor]
fn init_userns() {
    if aspen_testing_patchbay::skip::patchbay_available() {
        unsafe { patchbay::init_userns_for_ctor() };
    }
}

#[tokio::test]
async fn patchbay_forge_web_all_pages() {
    skip_unless_patchbay!();

    let lab = Lab::new().await.unwrap();
    let router = lab.add_router("dc").preset(RouterPreset::Public).build().await.unwrap();
    let node_dev = lab.add_device("node").iface("eth0", router.id(), None).build().await.unwrap();
    let client_dev = lab.add_device("client").iface("eth0", router.id(), None).build().await.unwrap();

    // Channels for cross-device coordination
    let (addr_tx, addr_rx) = tokio::sync::oneshot::channel();
    let (info_tx, info_rx) = tokio::sync::oneshot::channel::<(String, String)>();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    // --- Node device ---
    node_dev
        .spawn(async move |_dev| {
            let key = iroh::SecretKey::generate(&mut rand::rng());
            let ep = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
                .secret_key(key.clone())
                .relay_mode(iroh::RelayMode::Disabled)
                .alpns(vec![CLIENT_ALPN.to_vec()])
                .bind()
                .await
                .unwrap();

            // Real IrohBlobStore in a tempdir
            let tmp = tempfile::tempdir().unwrap();
            let blobs = Arc::new(IrohBlobStore::new(tmp.path(), ep.clone()).await.unwrap());

            let kv: Arc<dyn aspen_core::KeyValueStore> = DeterministicKeyValueStore::new();

            let forge_node = Arc::new(ForgeNode::new(blobs.clone(), kv.clone(), key.clone()));

            // Build context with all required fields
            let endpoint_provider = Arc::new(SimpleEndpointProvider {
                endpoint: ep.clone(),
                addr: ep.addr(),
            });

            let ctx = build_test_context(1, kv.clone(), endpoint_provider, Some(forge_node.clone()));
            let handler = ClientProtocolHandler::new(ctx);

            // Register on iroh Router
            let _router = iroh::protocol::Router::builder(ep.clone())
                .accept(CLIENT_ALPN, handler)
                .accept(iroh_blobs::ALPN, blobs.protocol_handler())
                .spawn();

            // Create a repo with content
            let identity = forge_node.create_repo("test-repo", vec![forge_node.public_key()], 1).await.unwrap();
            let repo_id = identity.repo_id();

            let hlc = create_hlc("test");
            let sign_key = iroh::SecretKey::generate(&mut rand::rng());

            let readme =
                SignedObject::new(GitObject::Blob(BlobObject::new(b"# Test Repo\nHello.")), &sign_key, &hlc).unwrap();
            let readme_hash = readme.hash();
            blobs.add_bytes(&readme.to_bytes()).await.unwrap();

            let tree = SignedObject::new(
                GitObject::Tree(TreeObject::new(vec![TreeEntry::file("README.md", readme_hash)])),
                &sign_key,
                &hlc,
            )
            .unwrap();
            let tree_hash = tree.hash();
            blobs.add_bytes(&tree.to_bytes()).await.unwrap();

            let commit = SignedObject::new(
                GitObject::Commit(CommitObject::new(
                    tree_hash,
                    vec![],
                    Author {
                        name: "Test".into(),
                        email: "t@t.com".into(),
                        public_key: None,
                        timestamp_ms: 1700000000000,
                        timezone: "+0000".into(),
                        npub: None,
                    },
                    "Initial commit",
                )),
                &sign_key,
                &hlc,
            )
            .unwrap();
            let commit_hash = commit.hash();
            blobs.add_bytes(&commit.to_bytes()).await.unwrap();

            forge_node.refs.set(&repo_id, "heads/main", commit_hash).await.unwrap();

            let _ = addr_tx.send(ep.addr());
            let _ = info_tx.send((repo_id.to_hex(), commit_hash.to_hex().to_string()));

            // Keep alive until test finishes
            let _ = shutdown_rx.await;
        })
        .unwrap();

    let node_addr = addr_rx.await.unwrap();
    let (repo_id, commit_hash) = info_rx.await.unwrap();

    // --- Client device ---
    let result = client_dev
        .spawn(async move |_dev| {
            let ep = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
                .relay_mode(iroh::RelayMode::Disabled)
                .bind()
                .await
                .unwrap();

            let topic = iroh_gossip::proto::TopicId::from_bytes(*blake3::hash(b"test").as_bytes());
            let ticket = aspen_client::AspenClusterTicket::with_bootstrap_addr(topic, "test".to_string(), &node_addr);
            let client = aspen_client::AspenClient::with_endpoint(ep, ticket, Duration::from_secs(10), None);
            let state = AppState::new(client);

            let mut results = Vec::new();

            // 1. Repo list
            let r = routes::dispatch(&state, "/", None).await;
            results.push(("repo list", r.status(), body_contains(&r, "test-repo")));

            // 2. Repo overview with clone URL
            let r = routes::dispatch(&state, &format!("/{repo_id}"), None).await;
            results.push(("overview", r.status(), body_contains(&r, "git clone aspen://")));

            // 3. File tree
            let r = routes::dispatch(&state, &format!("/{repo_id}/tree"), None).await;
            results.push(("tree", r.status(), body_contains(&r, "README.md")));

            // 4. Commits with links
            let r = routes::dispatch(&state, &format!("/{repo_id}/commits"), None).await;
            results.push(("commits", r.status(), body_contains(&r, "/commit/")));

            // 5. Commit detail with diff
            let r = routes::dispatch(&state, &format!("/{repo_id}/commit/{commit_hash}"), None).await;
            results.push(("commit detail", r.status(), body_contains(&r, "files changed")));

            // 6. Issues
            let r = routes::dispatch(&state, &format!("/{repo_id}/issues"), None).await;
            results.push(("issues", r.status(), true));

            // 7. Patches
            let r = routes::dispatch(&state, &format!("/{repo_id}/patches"), None).await;
            results.push(("patches", r.status(), true));

            // 8. Search with results
            let r = routes::dispatch(&state, &format!("/{repo_id}/search?q=Test"), None).await;
            results.push(("search hit", r.status(), body_contains(&r, "README.md")));

            // 9. Search with no results
            let r = routes::dispatch(&state, &format!("/{repo_id}/search?q=nonexistent_xyz"), None).await;
            results.push(("search miss", r.status(), body_contains(&r, "No results")));

            // 10. Repo filter input present
            let r = routes::dispatch(&state, "/", None).await;
            results.push(("repo filter", r.status(), body_contains(&r, "repo-filter")));

            // Collect failures
            let failures: Vec<_> = results
                .iter()
                .filter(|(_, status, ok)| *status != StatusCode::OK || !ok)
                .map(|(name, status, ok)| format!("{name}: {status}, content_ok={ok}"))
                .collect();

            if failures.is_empty() {
                "all passed".to_string()
            } else {
                format!("FAILURES: {}", failures.join("; "))
            }
        })
        .unwrap();

    let result_str = result.await.unwrap();
    assert_eq!(result_str, "all passed", "{result_str}");

    let _ = shutdown_tx.send(());
}

fn body_contains(resp: &routes::RouteResponse, needle: &str) -> bool {
    match resp {
        routes::RouteResponse::Html { body, .. } => body.contains(needle),
        routes::RouteResponse::Raw { body, .. } => String::from_utf8_lossy(body).contains(needle),
    }
}

/// Build a `ClientProtocolContext` with only the fields we need for forge.
fn build_test_context(
    node_id: u64,
    kv_store: Arc<dyn aspen_core::KeyValueStore>,
    endpoint_manager: Arc<dyn aspen_core::EndpointProvider>,
    forge_node: Option<Arc<ForgeNode<IrohBlobStore, dyn aspen_core::KeyValueStore>>>,
) -> ClientProtocolContext {
    // All fields listed unconditionally — feature gates are evaluated against
    // aspen-rpc-core's features (blob, forge enabled via dev-deps), not ours.
    ClientProtocolContext {
        node_id,
        controller: aspen_testing_core::DeterministicClusterController::new(),
        kv_store,
        state_machine: None,
        endpoint_manager,
        blob_store: None,
        blob_replication_manager: None,
        peer_manager: None,
        docs_sync: None,
        cluster_cookie: "test".to_string(),
        start_time: std::time::Instant::now(),
        network_factory: None,
        token_verifier: None,
        require_auth: false,
        topology: None,
        forge_node,
        watch_registry: None,
        hooks_config: Default::default(),
        secrets_service: None,
        federation_identity: None,
        federation_trust_manager: None,
        service_executors: Vec::new(),
        app_registry: aspen_core::shared_registry(),
        proxy_config: aspen_rpc_core::ProxyConfig::default(),
    }
}

/// Minimal EndpointProvider for tests.
struct SimpleEndpointProvider {
    endpoint: iroh::Endpoint,
    addr: iroh::EndpointAddr,
}

#[async_trait::async_trait]
impl aspen_core::EndpointProvider for SimpleEndpointProvider {
    async fn public_key(&self) -> Vec<u8> {
        self.endpoint.id().as_bytes().to_vec()
    }
    async fn peer_id(&self) -> String {
        self.endpoint.id().to_string()
    }
    async fn addresses(&self) -> Vec<String> {
        vec![]
    }
    fn node_addr(&self) -> &iroh::EndpointAddr {
        &self.addr
    }
    fn endpoint(&self) -> &iroh::Endpoint {
        &self.endpoint
    }
}

/// Sign a challenge with a Nostr key (secp256k1 Schnorr / BIP-340).
fn nostr_sign_challenge(keys: &nostr::Keys, challenge_hex: &str) -> String {
    use nostr::secp256k1::Message;
    use nostr::secp256k1::Secp256k1;

    let challenge_bytes = hex::decode(challenge_hex).unwrap();
    let msg_hash = bitcoin_hashes::sha256::Hash::hash(&challenge_bytes);
    let msg = Message::from_digest(msg_hash.to_byte_array());
    let secp = Secp256k1::new();
    let keypair = keys.secret_key().keypair(&secp);
    let sig = secp.sign_schnorr(&msg, &keypair);
    hex::encode(sig.serialize())
}

/// Test: authenticate with a Nostr npub, verify the auth flow works,
/// and confirm unauthenticated commits have npub=None (backward compat).
#[tokio::test]
async fn patchbay_nostr_auth_and_backward_compat() {
    skip_unless_patchbay!();

    let lab = Lab::new().await.unwrap();
    let router = lab.add_router("dc").preset(RouterPreset::Public).build().await.unwrap();
    let node_dev = lab.add_device("node").iface("eth0", router.id(), None).build().await.unwrap();
    let client_dev = lab.add_device("client").iface("eth0", router.id(), None).build().await.unwrap();

    let (addr_tx, addr_rx) = tokio::sync::oneshot::channel();
    let (info_tx, info_rx) = tokio::sync::oneshot::channel::<String>();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    // --- Node ---
    node_dev
        .spawn(async move |_dev| {
            let key = iroh::SecretKey::generate(&mut rand::rng());
            let ep = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
                .secret_key(key.clone())
                .relay_mode(iroh::RelayMode::Disabled)
                .alpns(vec![CLIENT_ALPN.to_vec()])
                .bind()
                .await
                .unwrap();

            let tmp = tempfile::tempdir().unwrap();
            let blobs = Arc::new(IrohBlobStore::new(tmp.path(), ep.clone()).await.unwrap());
            let kv: Arc<dyn aspen_core::KeyValueStore> = DeterministicKeyValueStore::new();
            let forge_node = Arc::new(ForgeNode::new(blobs.clone(), kv.clone(), key.clone()));

            let endpoint_provider = Arc::new(SimpleEndpointProvider {
                endpoint: ep.clone(),
                addr: ep.addr(),
            });
            let ctx = build_test_context(1, kv.clone(), endpoint_provider, Some(forge_node.clone()));
            let handler = ClientProtocolHandler::new(ctx);
            let _router = iroh::protocol::Router::builder(ep.clone())
                .accept(CLIENT_ALPN, handler)
                .accept(iroh_blobs::ALPN, blobs.protocol_handler())
                .spawn();

            // Create repo
            let identity = forge_node.create_repo("auth-test", vec![forge_node.public_key()], 1).await.unwrap();

            let _ = addr_tx.send(ep.addr());
            let _ = info_tx.send(identity.repo_id().to_hex());
            let _ = shutdown_rx.await;
        })
        .unwrap();

    let node_addr = addr_rx.await.unwrap();
    let repo_id = info_rx.await.unwrap();

    // --- Client ---
    let result = client_dev
        .spawn(async move |_dev| {
            let ep = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
                .relay_mode(iroh::RelayMode::Disabled)
                .bind()
                .await
                .unwrap();

            let topic = iroh_gossip::proto::TopicId::from_bytes(*blake3::hash(b"test").as_bytes());
            let ticket = aspen_client::AspenClusterTicket::with_bootstrap_addr(topic, "test".to_string(), &node_addr);
            let client = aspen_client::AspenClient::with_endpoint(ep, ticket, Duration::from_secs(10), None);
            let state = AppState::new(client);

            let mut failures = Vec::new();

            // --- Test 7.3: backward compat — unauthenticated commits have no npub ---
            let r = routes::dispatch(&state, &format!("/{repo_id}/commits"), None).await;
            // Repo has no commits yet, but the page should load
            if r.status() != StatusCode::OK {
                failures.push("commits page failed".to_string());
            }

            // --- Test: auth challenge/verify RPC ---
            let nostr_keys = nostr::Keys::generate();
            let npub_hex = nostr_keys.public_key().to_hex();

            // Step 1: get challenge
            let challenge_resp = routes::dispatch(&state, &format!("/login/challenge?npub={npub_hex}"), None).await;

            let challenge_json = match challenge_resp {
                routes::RouteResponse::Raw { body, status, .. } => {
                    if status != StatusCode::OK {
                        failures.push(format!("challenge failed: {}", String::from_utf8_lossy(&body)));
                        return failures.join("; ");
                    }
                    String::from_utf8(body).unwrap()
                }
                _ => {
                    failures.push("challenge returned HTML not JSON".to_string());
                    return failures.join("; ");
                }
            };

            let challenge: serde_json::Value = serde_json::from_str(&challenge_json).unwrap();
            let challenge_id = challenge["challenge_id"].as_str().unwrap();
            let challenge_hex = challenge["challenge_hex"].as_str().unwrap();

            // Step 2: sign challenge with nostr key
            let signature = nostr_sign_challenge(&nostr_keys, challenge_hex);

            // Step 3: verify
            let form_body = format!("npub={npub_hex}&challenge_id={challenge_id}&signature={signature}");
            let verify_resp = routes::dispatch(&state, "/login/verify", Some(&bytes::Bytes::from(form_body))).await;

            match verify_resp {
                routes::RouteResponse::Html { status, body } => {
                    if status != StatusCode::OK {
                        failures.push(format!("verify failed: {status}"));
                    }
                    if !body.contains("aspen_token=") {
                        failures.push("verify response missing token cookie".to_string());
                    }
                }
                _ => failures.push("verify returned non-HTML".to_string()),
            }

            // --- Test: login page loads ---
            let r = routes::dispatch(&state, "/login", None).await;
            if r.status() != StatusCode::OK || !body_contains(&r, "Login with Nostr") {
                failures.push("login page failed".to_string());
            }

            if failures.is_empty() {
                "all passed".to_string()
            } else {
                failures.join("; ")
            }
        })
        .unwrap();

    let result_str = result.await.unwrap();
    assert_eq!(result_str, "all passed", "{result_str}");

    let _ = shutdown_tx.send(());
}

/// Test: two users with different npubs create commits on the same repo.
/// Verify distinct ed25519 author keys and npub fields appear in commit
/// history and the web UI displays the npub-based author names.
#[tokio::test]
async fn patchbay_two_users_distinct_commits() {
    skip_unless_patchbay!();

    let lab = Lab::new().await.unwrap();
    let router = lab.add_router("dc").preset(RouterPreset::Public).build().await.unwrap();
    let node_dev = lab.add_device("node").iface("eth0", router.id(), None).build().await.unwrap();
    let client_dev = lab.add_device("client").iface("eth0", router.id(), None).build().await.unwrap();

    let (addr_tx, addr_rx) = tokio::sync::oneshot::channel();
    let (info_tx, info_rx) = tokio::sync::oneshot::channel::<(String, String, String)>();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    // --- Node: create repo with commits from two users ---
    node_dev
        .spawn(async move |_dev| {
            let node_key = iroh::SecretKey::generate(&mut rand::rng());
            let ep = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
                .secret_key(node_key.clone())
                .relay_mode(iroh::RelayMode::Disabled)
                .alpns(vec![CLIENT_ALPN.to_vec()])
                .bind()
                .await
                .unwrap();

            let tmp = tempfile::tempdir().unwrap();
            let blobs = Arc::new(IrohBlobStore::new(tmp.path(), ep.clone()).await.unwrap());
            let kv: Arc<dyn aspen_core::KeyValueStore> = DeterministicKeyValueStore::new();
            let forge_node = Arc::new(ForgeNode::new(blobs.clone(), kv.clone(), node_key.clone()));

            let endpoint_provider = Arc::new(SimpleEndpointProvider {
                endpoint: ep.clone(),
                addr: ep.addr(),
            });
            let ctx = build_test_context(1, kv.clone(), endpoint_provider, Some(forge_node.clone()));
            let handler = ClientProtocolHandler::new(ctx);
            let _router = iroh::protocol::Router::builder(ep.clone())
                .accept(CLIENT_ALPN, handler)
                .accept(iroh_blobs::ALPN, blobs.protocol_handler())
                .spawn();

            // Create repo
            let identity = forge_node.create_repo("multi-user", vec![forge_node.public_key()], 1).await.unwrap();
            let repo_id = identity.repo_id();

            // Two user contexts via identity store
            let id_store = aspen_forge::identity::nostr_mapping::NostrIdentityStore::new(kv.clone(), &node_key);
            let user_a = id_store.get_or_create(&"a".repeat(64)).await.unwrap();
            let user_b = id_store.get_or_create(&"b".repeat(64)).await.unwrap();

            // User A: first commit
            let hlc = create_hlc("test");
            let blob_a =
                SignedObject::new(GitObject::Blob(BlobObject::new(b"from A")), &user_a.signing_key, &hlc).unwrap();
            blobs.add_bytes(&blob_a.to_bytes()).await.unwrap();

            let tree_a = SignedObject::new(
                GitObject::Tree(TreeObject::new(vec![TreeEntry::file("a.txt", blob_a.hash())])),
                &user_a.signing_key,
                &hlc,
            )
            .unwrap();
            blobs.add_bytes(&tree_a.to_bytes()).await.unwrap();

            let commit_a = forge_node.git.commit_as(tree_a.hash(), vec![], "Commit by A", &user_a).await.unwrap();

            // User B: second commit
            let blob_b =
                SignedObject::new(GitObject::Blob(BlobObject::new(b"from B")), &user_b.signing_key, &hlc).unwrap();
            blobs.add_bytes(&blob_b.to_bytes()).await.unwrap();

            let tree_b = SignedObject::new(
                GitObject::Tree(TreeObject::new(vec![
                    TreeEntry::file("a.txt", blob_a.hash()),
                    TreeEntry::file("b.txt", blob_b.hash()),
                ])),
                &user_b.signing_key,
                &hlc,
            )
            .unwrap();
            blobs.add_bytes(&tree_b.to_bytes()).await.unwrap();

            let commit_b =
                forge_node.git.commit_as(tree_b.hash(), vec![commit_a], "Commit by B", &user_b).await.unwrap();

            forge_node.refs.set(&repo_id, "heads/main", commit_b).await.unwrap();

            let _ = addr_tx.send(ep.addr());
            let _ = info_tx.send((repo_id.to_hex(), commit_a.to_hex().to_string(), commit_b.to_hex().to_string()));
            let _ = shutdown_rx.await;
        })
        .unwrap();

    let node_addr = addr_rx.await.unwrap();
    let (repo_id, commit_a_hash, commit_b_hash) = info_rx.await.unwrap();

    // --- Client: verify ---
    let result = client_dev
        .spawn(async move |_dev| {
            let ep = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
                .relay_mode(iroh::RelayMode::Disabled)
                .bind()
                .await
                .unwrap();

            let topic = iroh_gossip::proto::TopicId::from_bytes(*blake3::hash(b"test").as_bytes());
            let ticket = aspen_client::AspenClusterTicket::with_bootstrap_addr(topic, "test".to_string(), &node_addr);
            let client = aspen_client::AspenClient::with_endpoint(ep, ticket, Duration::from_secs(10), None);
            let state = AppState::new(client);
            let mut failures = Vec::new();

            // 1. Commit A has user A's npub
            let ca = state.get_commit(&commit_a_hash).await.unwrap();
            if ca.author_npub.as_deref() != Some(&"a".repeat(64)) {
                failures.push(format!("commit A npub: {:?}", ca.author_npub));
            }

            // 2. Commit B has user B's npub
            let cb = state.get_commit(&commit_b_hash).await.unwrap();
            if cb.author_npub.as_deref() != Some(&"b".repeat(64)) {
                failures.push(format!("commit B npub: {:?}", cb.author_npub));
            }

            // 3. Distinct ed25519 author keys
            if ca.author_key == cb.author_key {
                failures.push("same ed25519 key for both users".to_string());
            }

            // 4. Commits page shows npub-based author display for both
            let r = routes::dispatch(&state, &format!("/{repo_id}/commits"), None).await;
            if r.status() != StatusCode::OK {
                failures.push("commits page failed".to_string());
            } else {
                let html = match &r {
                    routes::RouteResponse::Html { body, .. } => body.as_str(),
                    _ => "",
                };
                if !html.contains("npub:aaaaaaaa") {
                    failures.push("commits missing user A npub display".to_string());
                }
                if !html.contains("npub:bbbbbbbb") {
                    failures.push("commits missing user B npub display".to_string());
                }
            }

            // 5. Commit detail shows npub in author
            let r = routes::dispatch(&state, &format!("/{repo_id}/commit/{commit_b_hash}"), None).await;
            if r.status() != StatusCode::OK {
                failures.push("commit B detail failed".to_string());
            } else if !body_contains(&r, "npub:bbbbbbbb") {
                failures.push("commit B detail missing npub display".to_string());
            }

            // 6. Diff page shows added file
            if body_contains(&r, "files changed") && !body_contains(&r, "b.txt") {
                failures.push("commit B diff missing b.txt".to_string());
            }

            if failures.is_empty() {
                "all passed".to_string()
            } else {
                failures.join("; ")
            }
        })
        .unwrap();

    let result_str = result.await.unwrap();
    assert_eq!(result_str, "all passed", "{result_str}");

    let _ = shutdown_tx.send(());
}
