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
        GitObject::Tree(TreeObject::new(vec![
            TreeEntry::file("a.txt", blob_a),
            TreeEntry::file("b.txt", blob_b),
        ])),
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
    let blob_c = store_git_object(
        &blobs,
        &key,
        GitObject::Blob(BlobObject::new(b"new file c")),
    )
    .await;

    let tree_2 = store_git_object(
        &blobs,
        &key,
        GitObject::Tree(TreeObject::new(vec![TreeEntry::file("c.txt", blob_c)])),
    )
    .await;

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
