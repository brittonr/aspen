//! End-to-end tests for DAG sync integration in Forge.
//!
//! Creates a git object graph in an in-memory blob store, runs the sender-side
//! `handle_sync_request`, reads frames on the receiver side, and verifies the
//! correct objects arrive in the correct order.

use std::collections::BTreeSet;
use std::collections::HashSet;
use std::sync::Arc;

use aspen_blob::InMemoryBlobStore;
use aspen_blob::prelude::*;
use aspen_core::hlc::create_hlc;
use aspen_dag::protocol::*;
use aspen_dag::sync::ReceivedFrame;
use aspen_dag::sync::recv_sync;
use aspen_forge::BlobObject;
use aspen_forge::CommitObject;
use aspen_forge::GitObject;
use aspen_forge::SignedObject;
use aspen_forge::TreeEntry;
use aspen_forge::TreeObject;
use aspen_forge::identity::Author;
use aspen_forge::sync::DagSyncResult;
use aspen_forge::sync::SyncService;

// ============================================================================
// Helpers
// ============================================================================

fn test_author() -> Author {
    Author {
        name: "Test".to_string(),
        email: "test@example.com".to_string(),
        public_key: None,
        timestamp_ms: 0,
        timezone: "+0000".to_string(),
    }
}

/// Store a signed git object in the blob store and return its blake3 hash.
async fn store_git_object(blobs: &InMemoryBlobStore, key: &iroh::SecretKey, object: GitObject) -> blake3::Hash {
    let hlc = create_hlc("test-node");
    let signed = SignedObject::new(object, key, &hlc).unwrap();
    let bytes = signed.to_bytes();
    let hash = signed.hash();

    blobs.add_bytes(&bytes).await.unwrap();

    hash
}

/// Build a simple git repo in the blob store:
///
/// ```text
/// commit_1
///    |
///   tree_1
///   / \
/// blob_a  blob_b
/// ```
///
/// Returns (commit_hash, tree_hash, blob_a_hash, blob_b_hash).
async fn build_simple_repo(
    blobs: &InMemoryBlobStore,
    key: &iroh::SecretKey,
) -> (blake3::Hash, blake3::Hash, blake3::Hash, blake3::Hash) {
    let blob_a = store_git_object(blobs, key, GitObject::Blob(BlobObject::new(b"file a content"))).await;
    let blob_b = store_git_object(blobs, key, GitObject::Blob(BlobObject::new(b"file b content"))).await;

    let tree = store_git_object(
        blobs,
        key,
        GitObject::Tree(TreeObject::new(vec![TreeEntry::file("a.txt", blob_a), TreeEntry::file("b.txt", blob_b)])),
    )
    .await;

    let commit = store_git_object(
        blobs,
        key,
        GitObject::Commit(CommitObject::new(tree, vec![], test_author(), "initial commit")),
    )
    .await;

    (commit, tree, blob_a, blob_b)
}

/// Run handle_sync_request and collect received frames.
async fn run_sync(
    sync: &SyncService<InMemoryBlobStore>,
    request: DagSyncRequest,
) -> (Vec<(blake3::Hash, Vec<u8>)>, Vec<blake3::Hash>, aspen_dag::SyncStats) {
    let mut buf = Vec::new();
    let send_stats = sync.handle_sync_request(request, &mut buf).await.unwrap();

    let mut data_frames: Vec<(blake3::Hash, Vec<u8>)> = Vec::new();
    let mut hash_only_frames: Vec<blake3::Hash> = Vec::new();

    let recv_stats = recv_sync(&mut buf.as_slice(), |frame| {
        match frame {
            ReceivedFrame::Data { hash, data } => {
                data_frames.push((blake3::Hash::from_bytes(hash), data));
            }
            ReceivedFrame::HashOnly { hash } => {
                hash_only_frames.push(blake3::Hash::from_bytes(hash));
            }
        }
        Ok(())
    })
    .await
    .unwrap();

    assert_eq!(recv_stats.data_frames, send_stats.data_frames);
    assert_eq!(recv_stats.hash_only_frames, send_stats.hash_only_frames);

    (data_frames, hash_only_frames, send_stats)
}

// ============================================================================
// Tests
// ============================================================================

/// Full sync of a simple repo: commit → tree → [blob_a, blob_b].
/// All 4 objects should be inlined in DFS pre-order.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dag_sync_full_simple_repo() {
    let blobs = Arc::new(InMemoryBlobStore::new());
    let key = iroh::SecretKey::generate(&mut rand::rng());
    let sync = SyncService::new(Arc::clone(&blobs));

    let (commit, tree, blob_a, blob_b) = build_simple_repo(&blobs, &key).await;

    let request = DagSyncRequest {
        traversal: TraversalOpts::Full(FullTraversalOpts {
            root: *commit.as_bytes(),
            known_heads: BTreeSet::new(),
            order: TraversalOrder::DepthFirstPreOrder,
            filter: TraversalFilter::All,
        }),
        inline: InlinePolicy::All,
    };

    let (data_frames, hash_only_frames, stats) = run_sync(&sync, request).await;

    // All 4 objects inlined, none hash-only.
    assert_eq!(stats.data_frames, 4);
    assert_eq!(stats.hash_only_frames, 0);
    assert!(hash_only_frames.is_empty());

    // DFS pre-order: commit, tree, blob_a, blob_b.
    assert_eq!(data_frames.len(), 4);
    assert_eq!(data_frames[0].0, commit);
    assert_eq!(data_frames[1].0, tree);
    // blob_a and blob_b are children of tree, order depends on TreeObject entry order.
    let received_hashes: Vec<blake3::Hash> = data_frames.iter().map(|(h, _)| *h).collect();
    assert!(received_hashes.contains(&blob_a));
    assert!(received_hashes.contains(&blob_b));

    // Every received frame deserializes back to a valid SignedObject<GitObject>.
    for (hash, data) in &data_frames {
        let signed: SignedObject<GitObject> = SignedObject::from_bytes(data).unwrap();
        assert_eq!(signed.hash(), *hash, "hash mismatch for received object");
    }
}

/// Incremental sync with known heads.
///
/// Build a two-commit chain and sync with the first commit as known head.
/// Only the second commit and its new tree/blobs should be transferred.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dag_sync_incremental_with_known_heads() {
    let blobs = Arc::new(InMemoryBlobStore::new());
    let key = iroh::SecretKey::generate(&mut rand::rng());
    let sync = SyncService::new(Arc::clone(&blobs));

    // First commit (already known by receiver).
    let (commit_1, _tree_1, _blob_a, _blob_b) = build_simple_repo(&blobs, &key).await;

    // Second commit with new content, parent = commit_1.
    let blob_c = store_git_object(&blobs, &key, GitObject::Blob(BlobObject::new(b"new file c"))).await;

    let tree_2 =
        store_git_object(&blobs, &key, GitObject::Tree(TreeObject::new(vec![TreeEntry::file("c.txt", blob_c)]))).await;

    let commit_2 = store_git_object(
        &blobs,
        &key,
        GitObject::Commit(CommitObject::new(tree_2, vec![commit_1], test_author(), "second commit")),
    )
    .await;

    // Sync from commit_2 with commit_1 as known head.
    let request = DagSyncRequest {
        traversal: TraversalOpts::Full(FullTraversalOpts {
            root: *commit_2.as_bytes(),
            known_heads: BTreeSet::from([*commit_1.as_bytes()]),
            order: TraversalOrder::DepthFirstPreOrder,
            filter: TraversalFilter::All,
        }),
        inline: InlinePolicy::All,
    };

    let (data_frames, _, stats) = run_sync(&sync, request).await;

    // Only commit_2, tree_2, blob_c should transfer. commit_1 is a known head.
    assert_eq!(stats.data_frames, 3);
    let received_hashes: HashSet<blake3::Hash> = data_frames.iter().map(|(h, _)| *h).collect();
    assert!(received_hashes.contains(&commit_2));
    assert!(received_hashes.contains(&tree_2));
    assert!(received_hashes.contains(&blob_c));
    assert!(!received_hashes.contains(&commit_1));
}

/// Stem/leaf split sync.
///
/// Phase 1: exclude blobs → only commit + tree.
/// Phase 2: only blobs → blob_a + blob_b.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dag_sync_stem_leaf_split() {
    let blobs = Arc::new(InMemoryBlobStore::new());
    let key = iroh::SecretKey::generate(&mut rand::rng());
    let sync = SyncService::new(Arc::clone(&blobs));

    let (commit, tree, blob_a, blob_b) = build_simple_repo(&blobs, &key).await;

    // Phase 1: stem only (exclude blobs).
    let plan = sync.plan_git_sync(commit, HashSet::new());
    let stem_request = sync.build_stem_sync_request(&plan);
    let (stem_data, _, stem_stats) = run_sync(&sync, stem_request).await;

    assert_eq!(stem_stats.data_frames, 2);
    let stem_hashes: HashSet<blake3::Hash> = stem_data.iter().map(|(h, _)| *h).collect();
    assert!(stem_hashes.contains(&commit));
    assert!(stem_hashes.contains(&tree));
    assert!(!stem_hashes.contains(&blob_a));
    assert!(!stem_hashes.contains(&blob_b));

    // Phase 2: leaf only (only blobs).
    let leaf_request = sync.build_leaf_sync_request(&plan);
    let (leaf_data, _, leaf_stats) = run_sync(&sync, leaf_request).await;

    assert_eq!(leaf_stats.data_frames, 2);
    let leaf_hashes: HashSet<blake3::Hash> = leaf_data.iter().map(|(h, _)| *h).collect();
    assert!(leaf_hashes.contains(&blob_a));
    assert!(leaf_hashes.contains(&blob_b));
    assert!(!leaf_hashes.contains(&commit));
    assert!(!leaf_hashes.contains(&tree));
}

/// Sequence sync: request specific hashes by listing them.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dag_sync_sequence_request() {
    let blobs = Arc::new(InMemoryBlobStore::new());
    let key = iroh::SecretKey::generate(&mut rand::rng());
    let sync = SyncService::new(Arc::clone(&blobs));

    let (commit, _tree, blob_a, _blob_b) = build_simple_repo(&blobs, &key).await;

    // Request only commit and blob_a by hash.
    let request = DagSyncRequest {
        traversal: TraversalOpts::Sequence(vec![*commit.as_bytes(), *blob_a.as_bytes()]),
        inline: InlinePolicy::All,
    };

    let (data_frames, _, stats) = run_sync(&sync, request).await;

    assert_eq!(stats.data_frames, 2);
    assert_eq!(data_frames[0].0, commit);
    assert_eq!(data_frames[1].0, blob_a);
}

/// SizeThreshold inline policy: large blobs become hash-only.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dag_sync_size_threshold_policy() {
    let blobs = Arc::new(InMemoryBlobStore::new());
    let key = iroh::SecretKey::generate(&mut rand::rng());
    let sync = SyncService::new(Arc::clone(&blobs));

    let (commit, _tree, _blob_a, _blob_b) = build_simple_repo(&blobs, &key).await;

    // Use SizeThreshold(1) — almost nothing gets inlined since SignedObject
    // serialized bytes are always > 1 byte.
    let request = DagSyncRequest {
        traversal: TraversalOpts::Full(FullTraversalOpts {
            root: *commit.as_bytes(),
            known_heads: BTreeSet::new(),
            order: TraversalOrder::DepthFirstPreOrder,
            filter: TraversalFilter::All,
        }),
        inline: InlinePolicy::SizeThreshold(1),
    };

    let (data_frames, hash_only_frames, stats) = run_sync(&sync, request).await;

    // All 4 objects exceed 1 byte → all hash-only.
    assert_eq!(stats.data_frames, 0);
    assert_eq!(stats.hash_only_frames, 4);
    assert!(data_frames.is_empty());
    assert_eq!(hash_only_frames.len(), 4);
}

/// The plan → build_sync_request → handle_sync_request roundtrip.
///
/// Verifies that `plan_git_sync` + `build_sync_request` produces a request
/// that `handle_sync_request` can serve correctly.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dag_sync_plan_to_execution_roundtrip() {
    let blobs = Arc::new(InMemoryBlobStore::new());
    let key = iroh::SecretKey::generate(&mut rand::rng());
    let sync = SyncService::new(Arc::clone(&blobs));

    let (commit, _tree, _blob_a, _blob_b) = build_simple_repo(&blobs, &key).await;

    let plan = sync.plan_git_sync(commit, HashSet::new());
    let request = sync.build_sync_request(&plan, InlinePolicy::All);

    let (data_frames, _, stats) = run_sync(&sync, request).await;

    assert_eq!(stats.data_frames, 4);
    assert_eq!(data_frames[0].0, commit);
}

/// Verify into_dag_sync_handler creates a valid protocol handler.
/// (Structural test — the handler is an opaque type, so we just verify it builds.)
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dag_sync_handler_construction() {
    let blobs = Arc::new(InMemoryBlobStore::new());
    let sync = Arc::new(SyncService::new(blobs));
    let _handler = sync.into_dag_sync_handler();
    // Handler exists and implements ProtocolHandler. If this compiles, it works.
}

// ============================================================================
// Receiver Tests
// ============================================================================

/// Helper: run sender and pipe the output into a receiver backed by a fresh store.
async fn send_then_receive(
    sender: &SyncService<InMemoryBlobStore>,
    receiver: &SyncService<InMemoryBlobStore>,
    request: DagSyncRequest,
) -> DagSyncResult {
    let mut buf = Vec::new();
    sender.handle_sync_request(request, &mut buf).await.unwrap();
    receiver.receive_dag_sync_from_bytes(&buf).await.unwrap()
}

/// Full sender→receiver roundtrip: objects created on one store arrive in another.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dag_sync_receiver_full_roundtrip() {
    let sender_blobs = Arc::new(InMemoryBlobStore::new());
    let key = iroh::SecretKey::generate(&mut rand::rng());
    let sender = SyncService::new(Arc::clone(&sender_blobs));

    let (commit, tree, blob_a, blob_b) = build_simple_repo(&sender_blobs, &key).await;

    // Receiver has an empty store.
    let receiver_blobs = Arc::new(InMemoryBlobStore::new());
    let receiver = SyncService::new(Arc::clone(&receiver_blobs));

    let request = DagSyncRequest {
        traversal: TraversalOpts::Full(FullTraversalOpts {
            root: *commit.as_bytes(),
            known_heads: BTreeSet::new(),
            order: TraversalOrder::DepthFirstPreOrder,
            filter: TraversalFilter::All,
        }),
        inline: InlinePolicy::All,
    };

    let result = send_then_receive(&sender, &receiver, request).await;

    assert_eq!(result.objects_inserted, 4);
    assert_eq!(result.objects_already_present, 0);
    assert!(result.is_complete());
    assert!(result.deferred_hashes.is_empty());
    assert!(result.insert_errors.is_empty());
    assert!(result.bytes_received > 0);

    // Verify every object is now in the receiver's blob store.
    for hash in [commit, tree, blob_a, blob_b] {
        let iroh_hash = iroh_blobs::Hash::from_bytes(*hash.as_bytes());
        assert!(
            receiver_blobs.has(&iroh_hash).await.unwrap(),
            "object {} missing from receiver store",
            hash.to_hex()
        );
    }

    // Verify the received bytes deserialize to valid SignedObject<GitObject>.
    for hash in [commit, tree, blob_a, blob_b] {
        let iroh_hash = iroh_blobs::Hash::from_bytes(*hash.as_bytes());
        let bytes = receiver_blobs.get_bytes(&iroh_hash).await.unwrap().unwrap();
        let _signed: SignedObject<GitObject> = SignedObject::from_bytes(&bytes).unwrap();
    }
}

/// Incremental receive: receiver already has some objects.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dag_sync_receiver_incremental() {
    let sender_blobs = Arc::new(InMemoryBlobStore::new());
    let key = iroh::SecretKey::generate(&mut rand::rng());
    let sender = SyncService::new(Arc::clone(&sender_blobs));

    // Build a two-commit chain.
    let (commit_1, tree_1, blob_a, blob_b) = build_simple_repo(&sender_blobs, &key).await;

    let blob_c = store_git_object(&sender_blobs, &key, GitObject::Blob(BlobObject::new(b"new file c"))).await;
    let tree_2 =
        store_git_object(&sender_blobs, &key, GitObject::Tree(TreeObject::new(vec![TreeEntry::file("c.txt", blob_c)])))
            .await;
    let commit_2 = store_git_object(
        &sender_blobs,
        &key,
        GitObject::Commit(CommitObject::new(tree_2, vec![commit_1], test_author(), "second commit")),
    )
    .await;

    // Receiver already has commit_1 and below — sync only new objects.
    let receiver_blobs = Arc::new(InMemoryBlobStore::new());
    // Pre-populate receiver with commit_1's subgraph.
    for hash in [commit_1, tree_1, blob_a, blob_b] {
        let iroh_hash = iroh_blobs::Hash::from_bytes(*hash.as_bytes());
        let bytes = sender_blobs.get_bytes(&iroh_hash).await.unwrap().unwrap();
        receiver_blobs.add_bytes(&bytes).await.unwrap();
    }

    let receiver = SyncService::new(Arc::clone(&receiver_blobs));

    let request = DagSyncRequest {
        traversal: TraversalOpts::Full(FullTraversalOpts {
            root: *commit_2.as_bytes(),
            known_heads: BTreeSet::from([*commit_1.as_bytes()]),
            order: TraversalOrder::DepthFirstPreOrder,
            filter: TraversalFilter::All,
        }),
        inline: InlinePolicy::All,
    };

    let result = send_then_receive(&sender, &receiver, request).await;

    // Only 3 new objects: commit_2, tree_2, blob_c.
    assert_eq!(result.objects_inserted, 3);
    assert!(result.is_complete());

    // Old objects still present.
    for hash in [commit_1, tree_1, blob_a, blob_b] {
        let iroh_hash = iroh_blobs::Hash::from_bytes(*hash.as_bytes());
        assert!(receiver_blobs.has(&iroh_hash).await.unwrap());
    }
    // New objects arrived.
    for hash in [commit_2, tree_2, blob_c] {
        let iroh_hash = iroh_blobs::Hash::from_bytes(*hash.as_bytes());
        assert!(receiver_blobs.has(&iroh_hash).await.unwrap(), "new object {} missing", hash.to_hex());
    }
}

/// Receiver with hash-only policy collects deferred hashes.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dag_sync_receiver_hash_only_deferred() {
    let sender_blobs = Arc::new(InMemoryBlobStore::new());
    let key = iroh::SecretKey::generate(&mut rand::rng());
    let sender = SyncService::new(Arc::clone(&sender_blobs));

    let (commit, _tree, _blob_a, _blob_b) = build_simple_repo(&sender_blobs, &key).await;

    let receiver_blobs = Arc::new(InMemoryBlobStore::new());
    let receiver = SyncService::new(Arc::clone(&receiver_blobs));

    // InlinePolicy::None → everything is hash-only.
    let request = DagSyncRequest {
        traversal: TraversalOpts::Full(FullTraversalOpts {
            root: *commit.as_bytes(),
            known_heads: BTreeSet::new(),
            order: TraversalOrder::DepthFirstPreOrder,
            filter: TraversalFilter::All,
        }),
        inline: InlinePolicy::None,
    };

    let result = send_then_receive(&sender, &receiver, request).await;

    // Nothing inserted (all hash-only).
    assert_eq!(result.objects_inserted, 0);
    assert!(!result.is_complete());
    assert_eq!(result.deferred_hashes.len(), 4);

    // Receiver store is still empty.
    let iroh_hash = iroh_blobs::Hash::from_bytes(*commit.as_bytes());
    assert!(!receiver_blobs.has(&iroh_hash).await.unwrap());
}

/// Receiver with stem/leaf split: receive structure first, blobs second.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dag_sync_receiver_two_phase() {
    let sender_blobs = Arc::new(InMemoryBlobStore::new());
    let key = iroh::SecretKey::generate(&mut rand::rng());
    let sender = SyncService::new(Arc::clone(&sender_blobs));

    let (commit, tree, blob_a, blob_b) = build_simple_repo(&sender_blobs, &key).await;

    let receiver_blobs = Arc::new(InMemoryBlobStore::new());
    let receiver = SyncService::new(Arc::clone(&receiver_blobs));

    let plan = sender.plan_git_sync(commit, HashSet::new());

    // Phase 1: stem only.
    let stem_request = sender.build_stem_sync_request(&plan);
    let stem_result = send_then_receive(&sender, &receiver, stem_request).await;

    assert_eq!(stem_result.objects_inserted, 2); // commit + tree
    assert!(stem_result.is_complete());

    // Verify only structural objects arrived.
    let commit_iroh = iroh_blobs::Hash::from_bytes(*commit.as_bytes());
    let tree_iroh = iroh_blobs::Hash::from_bytes(*tree.as_bytes());
    let blob_a_iroh = iroh_blobs::Hash::from_bytes(*blob_a.as_bytes());
    let blob_b_iroh = iroh_blobs::Hash::from_bytes(*blob_b.as_bytes());

    assert!(receiver_blobs.has(&commit_iroh).await.unwrap());
    assert!(receiver_blobs.has(&tree_iroh).await.unwrap());
    assert!(!receiver_blobs.has(&blob_a_iroh).await.unwrap());
    assert!(!receiver_blobs.has(&blob_b_iroh).await.unwrap());

    // Phase 2: leaf only.
    let leaf_request = sender.build_leaf_sync_request(&plan);
    let leaf_result = send_then_receive(&sender, &receiver, leaf_request).await;

    assert_eq!(leaf_result.objects_inserted, 2); // blob_a + blob_b
    assert!(leaf_result.is_complete());

    // Now all objects are present.
    assert!(receiver_blobs.has(&blob_a_iroh).await.unwrap());
    assert!(receiver_blobs.has(&blob_b_iroh).await.unwrap());
}

/// Double-receive is idempotent: second sync inserts nothing new.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dag_sync_receiver_idempotent() {
    let sender_blobs = Arc::new(InMemoryBlobStore::new());
    let key = iroh::SecretKey::generate(&mut rand::rng());
    let sender = SyncService::new(Arc::clone(&sender_blobs));

    let (commit, _, _, _) = build_simple_repo(&sender_blobs, &key).await;

    let receiver_blobs = Arc::new(InMemoryBlobStore::new());
    let receiver = SyncService::new(Arc::clone(&receiver_blobs));

    let request = DagSyncRequest {
        traversal: TraversalOpts::Full(FullTraversalOpts {
            root: *commit.as_bytes(),
            known_heads: BTreeSet::new(),
            order: TraversalOrder::DepthFirstPreOrder,
            filter: TraversalFilter::All,
        }),
        inline: InlinePolicy::All,
    };

    // First sync.
    let mut buf = Vec::new();
    sender.handle_sync_request(request.clone(), &mut buf).await.unwrap();
    let first = receiver.receive_dag_sync_from_bytes(&buf).await.unwrap();
    assert_eq!(first.objects_inserted, 4);

    // Second sync — same data.
    let second = receiver.receive_dag_sync_from_bytes(&buf).await.unwrap();
    assert_eq!(second.objects_inserted, 0);
    assert_eq!(second.objects_already_present, 4);
    assert!(second.is_complete());
}

/// DagSyncResult helper methods.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dag_sync_result_helpers() {
    let mut result = DagSyncResult::default();
    assert_eq!(result.total_objects(), 0);
    assert!(result.is_complete());

    result.objects_inserted = 3;
    result.objects_already_present = 2;
    assert_eq!(result.total_objects(), 5);
    assert!(result.is_complete());

    result.deferred_hashes.push(blake3::hash(b"deferred"));
    assert!(!result.is_complete());
}

// ============================================================================
// Worker Tests
// ============================================================================

/// Gossip handler → SyncRequest channel → verify the request arrives.
///
/// This tests the integration between ForgeAnnouncementHandler and the
/// mpsc channel that DagSyncWorker would consume. We can't test the
/// full worker (it needs a real endpoint), but we can verify the
/// announcement→request pipeline.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn gossip_to_sync_request_pipeline() {
    use aspen_forge::Announcement;
    use aspen_forge::AnnouncementCallback;
    use aspen_forge::gossip::ForgeAnnouncementHandler;
    use aspen_forge::gossip::SyncRequest;
    use aspen_forge::identity::RepoId;

    let (handler, mut sync_rx, _seeding_rx) = ForgeAnnouncementHandler::with_channels(10);

    let key = iroh::SecretKey::generate(&mut rand::rng());
    let repo_id = RepoId::from_hash(blake3::hash(b"test-repo"));
    let commit_hash = blake3::hash(b"new-commit");

    // Simulate a gossip announcement arriving.
    let announcement = Announcement::RefUpdate {
        repo_id,
        ref_name: "heads/main".to_string(),
        new_hash: *commit_hash.as_bytes(),
        old_hash: None,
    };

    handler.on_announcement(&announcement, &key.public());

    // Verify the sync request was queued.
    let request = sync_rx.try_recv().expect("should receive sync request");
    match request {
        SyncRequest::RefUpdate {
            repo_id: r,
            ref_name,
            commit_hash: c,
            peer,
        } => {
            assert_eq!(r, repo_id);
            assert_eq!(ref_name, "heads/main");
            assert_eq!(c, commit_hash);
            assert_eq!(peer, key.public());
        }
        _ => panic!("expected RefUpdate"),
    }
}

/// ForgeNode exposes dag_sync_handler() for router registration.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn forge_node_dag_sync_handler() {
    use aspen_forge::ForgeNode;

    let blobs = Arc::new(InMemoryBlobStore::new());
    let kv = Arc::new(aspen_testing_core::DeterministicKeyValueStore::new());
    let key = iroh::SecretKey::generate(&mut rand::rng());

    let forge = ForgeNode::new(blobs, kv, key);
    let _handler = forge.dag_sync_handler();
    // Compiles and returns a valid ProtocolHandler.
}

/// End-to-end: create objects on sender, pipe through sender handler,
/// receive into a fresh store via receive_dag_sync_from_bytes, then
/// verify the receiver can serve the same objects back via its own
/// handle_sync_request — proving the objects are fully usable.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dag_sync_full_loop_sender_receiver_sender() {
    let key = iroh::SecretKey::generate(&mut rand::rng());

    // Sender: create a repo.
    let sender_blobs = Arc::new(InMemoryBlobStore::new());
    let sender = SyncService::new(Arc::clone(&sender_blobs));
    let (commit, tree, blob_a, blob_b) = build_simple_repo(&sender_blobs, &key).await;

    // Sender → wire bytes.
    let request = DagSyncRequest {
        traversal: TraversalOpts::Full(FullTraversalOpts {
            root: *commit.as_bytes(),
            known_heads: BTreeSet::new(),
            order: TraversalOrder::DepthFirstPreOrder,
            filter: TraversalFilter::All,
        }),
        inline: InlinePolicy::All,
    };
    let mut wire_bytes = Vec::new();
    sender.handle_sync_request(request.clone(), &mut wire_bytes).await.unwrap();

    // Receiver: empty store, receive the bytes.
    let receiver_blobs = Arc::new(InMemoryBlobStore::new());
    let receiver = SyncService::new(Arc::clone(&receiver_blobs));
    let result = receiver.receive_dag_sync_from_bytes(&wire_bytes).await.unwrap();
    assert_eq!(result.objects_inserted, 4);

    // Now the receiver serves the same DAG back.
    let mut re_served = Vec::new();
    let re_stats = receiver.handle_sync_request(request, &mut re_served).await.unwrap();
    assert_eq!(re_stats.data_frames, 4);

    // Third store receives from the receiver's output.
    let third_blobs = Arc::new(InMemoryBlobStore::new());
    let third = SyncService::new(Arc::clone(&third_blobs));
    let third_result = third.receive_dag_sync_from_bytes(&re_served).await.unwrap();
    assert_eq!(third_result.objects_inserted, 4);

    // All objects present in all three stores.
    for hash in [commit, tree, blob_a, blob_b] {
        let iroh_hash = iroh_blobs::Hash::from_bytes(*hash.as_bytes());
        assert!(sender_blobs.has(&iroh_hash).await.unwrap());
        assert!(receiver_blobs.has(&iroh_hash).await.unwrap());
        assert!(third_blobs.has(&iroh_hash).await.unwrap());
    }
}

// ============================================================================
// Traversal Bounds Tests
// ============================================================================

/// Build a long linear chain of commits: C_n → C_{n-1} → ... → C_0.
/// Each commit has an empty tree. Returns the tip commit hash.
async fn build_commit_chain(blobs: &InMemoryBlobStore, key: &iroh::SecretKey, length: usize) -> blake3::Hash {
    let empty_tree = store_git_object(blobs, key, GitObject::Tree(TreeObject::new(vec![]))).await;

    let mut prev = store_git_object(
        blobs,
        key,
        GitObject::Commit(CommitObject::new(empty_tree, vec![], test_author(), "commit 0")),
    )
    .await;

    for i in 1..length {
        prev = store_git_object(
            blobs,
            key,
            GitObject::Commit(CommitObject::new(empty_tree, vec![prev], test_author(), &format!("commit {i}"))),
        )
        .await;
    }

    prev
}

/// Verify that handle_sync_request respects the visited set bound.
///
/// The traversal should stop before visiting more than MAX_VISITED_SET_SIZE
/// nodes. We can't test the exact 1M limit (too slow), but we can verify
/// the mechanism works with a moderate chain.
#[tokio::test]
async fn dag_sync_handles_large_chain_without_panic() {
    let key = iroh::SecretKey::generate(&mut rand::rng());
    let blobs = Arc::new(InMemoryBlobStore::new());
    let sync = SyncService::new(Arc::clone(&blobs));

    // A chain of 500 commits is large enough to verify the sync
    // completes and returns correct stats, but stays well under the
    // 1M visited set limit.
    let tip = build_commit_chain(&blobs, &key, 500).await;

    let request = DagSyncRequest {
        traversal: TraversalOpts::Full(FullTraversalOpts {
            root: *tip.as_bytes(),
            known_heads: BTreeSet::new(),
            order: TraversalOrder::DepthFirstPreOrder,
            filter: TraversalFilter::All,
        }),
        inline: InlinePolicy::All,
    };

    let (data_frames, hash_only, stats) = run_sync(&sync, request).await;

    // 500 commits + 1 empty tree = 501 unique objects.
    // Each commit references the same empty tree, so the tree is
    // visited once (deduped by the visited set).
    assert_eq!(data_frames.len(), 501, "expected 500 commits + 1 tree, got {} data frames", data_frames.len());
    assert_eq!(hash_only.len(), 0);
    assert_eq!(stats.data_frames, 501);
}

/// Verify that known_heads correctly stops traversal.
///
/// Build a chain A → B → C. Sync from A with B as known head.
/// Should only transfer A and its tree.
#[tokio::test]
async fn dag_sync_known_heads_stops_traversal() {
    let key = iroh::SecretKey::generate(&mut rand::rng());
    let blobs = Arc::new(InMemoryBlobStore::new());
    let sync = SyncService::new(Arc::clone(&blobs));

    let empty_tree = store_git_object(&blobs, &key, GitObject::Tree(TreeObject::new(vec![]))).await;

    let c =
        store_git_object(&blobs, &key, GitObject::Commit(CommitObject::new(empty_tree, vec![], test_author(), "C")))
            .await;

    let b =
        store_git_object(&blobs, &key, GitObject::Commit(CommitObject::new(empty_tree, vec![c], test_author(), "B")))
            .await;

    let a =
        store_git_object(&blobs, &key, GitObject::Commit(CommitObject::new(empty_tree, vec![b], test_author(), "A")))
            .await;

    // Sync from A with B as known — should get A + tree only
    let request = DagSyncRequest {
        traversal: TraversalOpts::Full(FullTraversalOpts {
            root: *a.as_bytes(),
            known_heads: BTreeSet::from([*b.as_bytes()]),
            order: TraversalOrder::DepthFirstPreOrder,
            filter: TraversalFilter::All,
        }),
        inline: InlinePolicy::All,
    };

    let (data_frames, _hash_only, stats) = run_sync(&sync, request).await;

    // A is transferred, its tree is new so also transferred.
    // B and C are known, so skipped.
    assert_eq!(stats.data_frames, 2, "expected A + tree");
    let hashes: HashSet<blake3::Hash> = data_frames.iter().map(|(h, _)| *h).collect();
    assert!(hashes.contains(&a), "should contain commit A");
    assert!(hashes.contains(&empty_tree), "should contain the tree");
    assert!(!hashes.contains(&b), "should NOT contain known head B");
    assert!(!hashes.contains(&c), "should NOT contain C (behind known head)");
}
