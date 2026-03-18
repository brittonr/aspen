//! End-to-end DAG sync over real iroh QUIC via patchbay network namespaces.
//!
//! Tests the full protocol path: sender registers `DagSyncProtocolHandler`
//! on its iroh Router, receiver calls `receive_dag_sync` which connects
//! via `connect_dag_sync`, sends the request, and reads response frames
//! into its blob store.
//!
//! Requirements: Linux with unprivileged user namespaces, nft, tc in PATH.

use std::collections::BTreeSet;
use std::sync::Arc;

use aspen_blob::InMemoryBlobStore;
use aspen_blob::prelude::*;
use aspen_core::hlc::create_hlc;
use aspen_dag::protocol::*;
use aspen_forge::BlobObject;
use aspen_forge::CommitObject;
use aspen_forge::GitObject;
use aspen_forge::SignedObject;
use aspen_forge::TreeEntry;
use aspen_forge::TreeObject;
use aspen_forge::identity::Author;
use aspen_forge::sync::SyncService;
use aspen_testing_patchbay::skip_unless_patchbay;
use patchbay::Lab;
use patchbay::RouterPreset;

/// Initialize user namespace before tokio runtime starts.
#[ctor::ctor]
fn init_userns() {
    if aspen_testing_patchbay::skip::patchbay_available() {
        unsafe { patchbay::init_userns_for_ctor() };
    }
}

fn test_author() -> Author {
    Author {
        name: "Test".to_string(),
        email: "test@example.com".to_string(),
        public_key: None,
        timestamp_ms: 0,
        timezone: "+0000".to_string(),
    }
}

async fn store_git_object(blobs: &InMemoryBlobStore, key: &iroh::SecretKey, object: GitObject) -> blake3::Hash {
    let hlc = create_hlc("test-node");
    let signed = SignedObject::new(object, key, &hlc).unwrap();
    let bytes = signed.to_bytes();
    let hash = signed.hash();
    blobs.add_bytes(&bytes).await.unwrap();
    hash
}

/// Build: commit → tree → [blob_a, blob_b]
async fn build_repo(
    blobs: &InMemoryBlobStore,
    key: &iroh::SecretKey,
) -> (blake3::Hash, blake3::Hash, blake3::Hash, blake3::Hash) {
    let blob_a = store_git_object(blobs, key, GitObject::Blob(BlobObject::new(b"file a"))).await;
    let blob_b = store_git_object(blobs, key, GitObject::Blob(BlobObject::new(b"file b"))).await;
    let tree = store_git_object(
        blobs,
        key,
        GitObject::Tree(TreeObject::new(vec![
            TreeEntry::file("a.txt", blob_a),
            TreeEntry::file("b.txt", blob_b),
        ])),
    )
    .await;
    let commit = store_git_object(
        blobs,
        key,
        GitObject::Commit(CommitObject::new(tree, vec![], test_author(), "initial")),
    )
    .await;
    (commit, tree, blob_a, blob_b)
}

// ============================================================================
// Tests
// ============================================================================

/// Full sync over real QUIC: sender serves git objects via DagSyncProtocolHandler,
/// receiver calls receive_dag_sync and gets all 4 objects.
#[tokio::test]
async fn patchbay_forge_dag_sync_full() {
    skip_unless_patchbay!();

    let lab = Lab::new().await.unwrap();
    let router = lab.add_router("dc").preset(RouterPreset::Public).build().await.unwrap();
    let sender_dev = lab.add_device("sender").iface("eth0", router.id(), None).build().await.unwrap();
    let receiver_dev = lab.add_device("receiver").iface("eth0", router.id(), None).build().await.unwrap();

    let key = iroh::SecretKey::generate(&mut rand::rng());

    // --- Sender side ---
    let sender_blobs = Arc::new(InMemoryBlobStore::new());
    let (commit, tree, blob_a, blob_b) = build_repo(&sender_blobs, &key).await;

    let (addr_tx, addr_rx) = tokio::sync::oneshot::channel();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let sender_blobs_clone = Arc::clone(&sender_blobs);
    sender_dev
        .spawn(async move |_dev| {
            let ep = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
                .relay_mode(iroh::RelayMode::Disabled)
                .alpns(vec![aspen_dag::DAG_SYNC_ALPN.to_vec()])
                .bind()
                .await
                .unwrap();

            let sync = Arc::new(SyncService::new(sender_blobs_clone));
            let handler = sync.into_dag_sync_handler();

            let _router = iroh::protocol::Router::builder(ep.clone())
                .accept(aspen_dag::DAG_SYNC_ALPN, handler)
                .spawn();

            let _ = addr_tx.send(ep.addr());
            let _ = shutdown_rx.await;
        })
        .unwrap();

    let sender_addr = addr_rx.await.unwrap();

    // --- Receiver side ---
    let receiver_blobs = Arc::new(InMemoryBlobStore::new());
    let receiver_blobs_clone = Arc::clone(&receiver_blobs);

    let request = DagSyncRequest {
        traversal: TraversalOpts::Full(FullTraversalOpts {
            root: *commit.as_bytes(),
            known_heads: BTreeSet::new(),
            order: TraversalOrder::DepthFirstPreOrder,
            filter: TraversalFilter::All,
        }),
        inline: InlinePolicy::All,
    };

    let result_jh = receiver_dev
        .spawn(async move |_dev| {
            let ep = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
                .relay_mode(iroh::RelayMode::Disabled)
                .bind()
                .await
                .unwrap();

            let receiver_sync = SyncService::new(receiver_blobs_clone);
            receiver_sync
                .receive_dag_sync(&ep, sender_addr, request)
                .await
                .unwrap()
        })
        .unwrap();

    let result = result_jh.await.unwrap();

    assert_eq!(result.objects_inserted, 4);
    assert!(result.is_complete());
    assert!(result.deferred_hashes.is_empty());

    // Verify objects are in the receiver's blob store.
    for hash in [commit, tree, blob_a, blob_b] {
        let iroh_hash = iroh_blobs::Hash::from_bytes(*hash.as_bytes());
        assert!(
            receiver_blobs.has(&iroh_hash).await.unwrap(),
            "object {} missing from receiver",
            blake3::Hash::from_bytes(*hash.as_bytes()).to_hex()
        );
    }

    let _ = shutdown_tx.send(());
}

/// Incremental sync: receiver already has commit_1, only commit_2's subtree transfers.
#[tokio::test]
async fn patchbay_forge_dag_sync_incremental() {
    skip_unless_patchbay!();

    let lab = Lab::new().await.unwrap();
    let router = lab.add_router("dc").preset(RouterPreset::Public).build().await.unwrap();
    let sender_dev = lab.add_device("sender").iface("eth0", router.id(), None).build().await.unwrap();
    let receiver_dev = lab.add_device("receiver").iface("eth0", router.id(), None).build().await.unwrap();

    let key = iroh::SecretKey::generate(&mut rand::rng());

    // Build two-commit chain on sender.
    let sender_blobs = Arc::new(InMemoryBlobStore::new());
    let (commit_1, _tree_1, _blob_a, _blob_b) = build_repo(&sender_blobs, &key).await;

    let blob_c = store_git_object(&sender_blobs, &key, GitObject::Blob(BlobObject::new(b"file c"))).await;
    let tree_2 = store_git_object(
        &sender_blobs,
        &key,
        GitObject::Tree(TreeObject::new(vec![TreeEntry::file("c.txt", blob_c)])),
    )
    .await;
    let commit_2 = store_git_object(
        &sender_blobs,
        &key,
        GitObject::Commit(CommitObject::new(tree_2, vec![commit_1], test_author(), "second")),
    )
    .await;

    // Start sender.
    let (addr_tx, addr_rx) = tokio::sync::oneshot::channel();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let sender_blobs_clone = Arc::clone(&sender_blobs);
    sender_dev
        .spawn(async move |_dev| {
            let ep = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
                .relay_mode(iroh::RelayMode::Disabled)
                .alpns(vec![aspen_dag::DAG_SYNC_ALPN.to_vec()])
                .bind()
                .await
                .unwrap();

            let sync = Arc::new(SyncService::new(sender_blobs_clone));
            let handler = sync.into_dag_sync_handler();
            let _router = iroh::protocol::Router::builder(ep.clone())
                .accept(aspen_dag::DAG_SYNC_ALPN, handler)
                .spawn();

            let _ = addr_tx.send(ep.addr());
            let _ = shutdown_rx.await;
        })
        .unwrap();

    let sender_addr = addr_rx.await.unwrap();

    // Receiver: request with commit_1 as known head.
    let receiver_blobs = Arc::new(InMemoryBlobStore::new());
    let receiver_blobs_clone = Arc::clone(&receiver_blobs);

    let request = DagSyncRequest {
        traversal: TraversalOpts::Full(FullTraversalOpts {
            root: *commit_2.as_bytes(),
            known_heads: BTreeSet::from([*commit_1.as_bytes()]),
            order: TraversalOrder::DepthFirstPreOrder,
            filter: TraversalFilter::All,
        }),
        inline: InlinePolicy::All,
    };

    let result_jh = receiver_dev
        .spawn(async move |_dev| {
            let ep = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
                .relay_mode(iroh::RelayMode::Disabled)
                .bind()
                .await
                .unwrap();

            let receiver_sync = SyncService::new(receiver_blobs_clone);
            receiver_sync
                .receive_dag_sync(&ep, sender_addr, request)
                .await
                .unwrap()
        })
        .unwrap();

    let result = result_jh.await.unwrap();

    // Only commit_2 + tree_2 + blob_c = 3 objects.
    assert_eq!(result.objects_inserted, 3);
    assert!(result.is_complete());

    // commit_1 was NOT transferred.
    let commit_1_iroh = iroh_blobs::Hash::from_bytes(*commit_1.as_bytes());
    assert!(!receiver_blobs.has(&commit_1_iroh).await.unwrap());

    // New objects are present.
    for hash in [commit_2, tree_2, blob_c] {
        let iroh_hash = iroh_blobs::Hash::from_bytes(*hash.as_bytes());
        assert!(receiver_blobs.has(&iroh_hash).await.unwrap());
    }

    let _ = shutdown_tx.send(());
}

/// Stem/leaf split over QUIC: two separate syncs to the same sender.
#[tokio::test]
async fn patchbay_forge_dag_sync_stem_leaf() {
    skip_unless_patchbay!();

    let lab = Lab::new().await.unwrap();
    let router = lab.add_router("dc").preset(RouterPreset::Public).build().await.unwrap();
    let sender_dev = lab.add_device("sender").iface("eth0", router.id(), None).build().await.unwrap();
    let receiver_dev = lab.add_device("receiver").iface("eth0", router.id(), None).build().await.unwrap();

    let key = iroh::SecretKey::generate(&mut rand::rng());
    let sender_blobs = Arc::new(InMemoryBlobStore::new());
    let (commit, tree, blob_a, blob_b) = build_repo(&sender_blobs, &key).await;

    let (addr_tx, addr_rx) = tokio::sync::oneshot::channel();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let sender_blobs_clone = Arc::clone(&sender_blobs);
    sender_dev
        .spawn(async move |_dev| {
            let ep = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
                .relay_mode(iroh::RelayMode::Disabled)
                .alpns(vec![aspen_dag::DAG_SYNC_ALPN.to_vec()])
                .bind()
                .await
                .unwrap();

            let sync = Arc::new(SyncService::new(sender_blobs_clone));
            let handler = sync.into_dag_sync_handler();
            let _router = iroh::protocol::Router::builder(ep.clone())
                .accept(aspen_dag::DAG_SYNC_ALPN, handler)
                .spawn();

            let _ = addr_tx.send(ep.addr());
            let _ = shutdown_rx.await;
        })
        .unwrap();

    let sender_addr = addr_rx.await.unwrap();

    // Phase 1: stem (exclude blobs).
    let receiver_blobs = Arc::new(InMemoryBlobStore::new());
    let rb1 = Arc::clone(&receiver_blobs);
    let sa1 = sender_addr.clone();

    let stem_request = DagSyncRequest {
        traversal: TraversalOpts::Full(FullTraversalOpts {
            root: *commit.as_bytes(),
            known_heads: BTreeSet::new(),
            order: TraversalOrder::DepthFirstPreOrder,
            filter: TraversalFilter::Exclude(BTreeSet::from([2])), // Exclude blobs
        }),
        inline: InlinePolicy::All,
    };

    let stem_jh = receiver_dev
        .spawn(async move |_dev| {
            let ep = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
                .relay_mode(iroh::RelayMode::Disabled)
                .bind()
                .await
                .unwrap();
            let sync = SyncService::new(rb1);
            sync.receive_dag_sync(&ep, sa1, stem_request).await.unwrap()
        })
        .unwrap();

    let stem_result = stem_jh.await.unwrap();
    assert_eq!(stem_result.objects_inserted, 2); // commit + tree

    // Verify stem objects present, blobs absent.
    let commit_iroh = iroh_blobs::Hash::from_bytes(*commit.as_bytes());
    let tree_iroh = iroh_blobs::Hash::from_bytes(*tree.as_bytes());
    let blob_a_iroh = iroh_blobs::Hash::from_bytes(*blob_a.as_bytes());
    let blob_b_iroh = iroh_blobs::Hash::from_bytes(*blob_b.as_bytes());
    assert!(receiver_blobs.has(&commit_iroh).await.unwrap());
    assert!(receiver_blobs.has(&tree_iroh).await.unwrap());
    assert!(!receiver_blobs.has(&blob_a_iroh).await.unwrap());
    assert!(!receiver_blobs.has(&blob_b_iroh).await.unwrap());

    // Phase 2: leaf (only blobs).
    let rb2 = Arc::clone(&receiver_blobs);
    let sa2 = sender_addr.clone();

    let leaf_request = DagSyncRequest {
        traversal: TraversalOpts::Full(FullTraversalOpts {
            root: *commit.as_bytes(),
            known_heads: BTreeSet::new(),
            order: TraversalOrder::DepthFirstPreOrder,
            filter: TraversalFilter::Only(BTreeSet::from([2])), // Only blobs
        }),
        inline: InlinePolicy::All,
    };

    let leaf_jh = receiver_dev
        .spawn(async move |_dev| {
            let ep = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
                .relay_mode(iroh::RelayMode::Disabled)
                .bind()
                .await
                .unwrap();
            let sync = SyncService::new(rb2);
            sync.receive_dag_sync(&ep, sa2, leaf_request).await.unwrap()
        })
        .unwrap();

    let leaf_result = leaf_jh.await.unwrap();
    assert_eq!(leaf_result.objects_inserted, 2); // blob_a + blob_b

    // All objects now present.
    assert!(receiver_blobs.has(&blob_a_iroh).await.unwrap());
    assert!(receiver_blobs.has(&blob_b_iroh).await.unwrap());

    let _ = shutdown_tx.send(());
}
