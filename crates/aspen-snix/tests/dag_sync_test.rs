//! Integration test for DirectoryLinkExtractor with FullTraversal.
//!
//! Builds a directory tree in a RaftDirectoryService backed by
//! DeterministicKeyValueStore, then traverses it using FullTraversal
//! + DirectoryLinkExtractor to verify the correct nodes are visited.

use std::sync::Arc;

use aspen_dag::DagTraversal;
use aspen_dag::FullTraversal;
use aspen_snix::DirectoryLinkExtractor;
use aspen_snix::RaftDirectoryService;
use aspen_snix::extract_directory_children;
use aspen_testing::DeterministicKeyValueStore;
use snix_castore::B3Digest;
use snix_castore::Node;
use snix_castore::directoryservice::DirectoryService;

/// Build a directory tree:
///
/// ```text
/// root/
///   ├── hello.txt (file, digest=H1)
///   ├── sub/
///   │   ├── nested.txt (file, digest=H2)
///   │   └── link → ../hello.txt (symlink, no digest)
///   └── empty/ (empty directory)
/// ```
///
/// Returns (root_digest, sub_digest, empty_digest, h1, h2)
async fn build_tree<D: DirectoryService>(ds: &D) -> (B3Digest, B3Digest, B3Digest, B3Digest, B3Digest) {
    let h1 = B3Digest::from(blake3::hash(b"hello content"));
    let h2 = B3Digest::from(blake3::hash(b"nested content"));

    // Empty dir
    let empty_dir = snix_castore::Directory::new();
    let empty_digest = ds.put(empty_dir).await.unwrap();

    // Sub dir
    let mut sub_dir = snix_castore::Directory::new();
    sub_dir
        .add("nested.txt".try_into().unwrap(), Node::File {
            digest: h2.clone(),
            size: 14,
            executable: false,
        })
        .unwrap();
    sub_dir
        .add("link".try_into().unwrap(), Node::Symlink {
            target: "../hello.txt".try_into().unwrap(),
        })
        .unwrap();
    let sub_digest = ds.put(sub_dir).await.unwrap();

    // Root dir
    let mut root_dir = snix_castore::Directory::new();
    root_dir
        .add("hello.txt".try_into().unwrap(), Node::File {
            digest: h1.clone(),
            size: 13,
            executable: false,
        })
        .unwrap();
    root_dir
        .add("sub".try_into().unwrap(), Node::Directory {
            digest: sub_digest.clone(),
            size: 100,
        })
        .unwrap();
    root_dir
        .add("empty".try_into().unwrap(), Node::Directory {
            digest: empty_digest.clone(),
            size: 0,
        })
        .unwrap();
    let root_digest = ds.put(root_dir).await.unwrap();

    (root_digest, sub_digest, empty_digest, h1, h2)
}

/// Full traversal visits all directory and file digests.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn directory_traversal_full() {
    let kv = DeterministicKeyValueStore::new();
    let ds = RaftDirectoryService::from_arc(kv);
    let (root_digest, sub_digest, empty_digest, h1, h2) = build_tree(&ds).await;

    let extractor = DirectoryLinkExtractor::new(Arc::new(ds.clone()));
    let mut trav = FullTraversal::new(root_digest.clone(), (), extractor);

    let mut visited = vec![];
    while let Some(digest) = trav.next().await.unwrap() {
        visited.push(digest);
    }

    // Root is first.
    assert_eq!(visited[0], root_digest);

    // All 5 digests visited (root, sub, empty, h1, h2).
    assert_eq!(visited.len(), 5);

    // Each expected digest is present.
    assert!(visited.contains(&root_digest));
    assert!(visited.contains(&sub_digest));
    assert!(visited.contains(&empty_digest));
    assert!(visited.contains(&h1));
    assert!(visited.contains(&h2));

    // No duplicates.
    let unique: std::collections::HashSet<B3Digest> = visited.iter().cloned().collect();
    assert_eq!(visited.len(), unique.len());
}

/// Traversal with the empty dir as root yields only the empty dir
/// (which has no children, so no further nodes).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn directory_traversal_empty_root() {
    let kv = DeterministicKeyValueStore::new();
    let ds = RaftDirectoryService::from_arc(kv);
    let (_, _, empty_digest, _, _) = build_tree(&ds).await;

    let extractor = DirectoryLinkExtractor::new(Arc::new(ds.clone()));
    let mut trav = FullTraversal::new(empty_digest.clone(), (), extractor);

    let mut visited = vec![];
    while let Some(digest) = trav.next().await.unwrap() {
        visited.push(digest);
    }

    // Only the empty directory itself.
    assert_eq!(visited, vec![empty_digest]);
}

/// Traversal with sub as root yields sub + its children (h2),
/// but not root or empty or h1.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn directory_traversal_subtree() {
    let kv = DeterministicKeyValueStore::new();
    let ds = RaftDirectoryService::from_arc(kv);
    let (_, sub_digest, _, _, h2) = build_tree(&ds).await;

    let extractor = DirectoryLinkExtractor::new(Arc::new(ds.clone()));
    let mut trav = FullTraversal::new(sub_digest.clone(), (), extractor);

    let mut visited = vec![];
    while let Some(digest) = trav.next().await.unwrap() {
        visited.push(digest);
    }

    // sub + nested.txt (h2). Symlink has no digest.
    assert_eq!(visited.len(), 2);
    assert_eq!(visited[0], sub_digest);
    assert!(visited.contains(&h2));
}

/// Bounded traversal stops early.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn directory_traversal_bounded() {
    let kv = DeterministicKeyValueStore::new();
    let ds = RaftDirectoryService::from_arc(kv);
    let (root_digest, _, _, _, _) = build_tree(&ds).await;

    let extractor = DirectoryLinkExtractor::new(Arc::new(ds.clone()));
    let trav = FullTraversal::new(root_digest.clone(), (), extractor);
    let mut bounded = trav.bounded(2);

    let mut visited = vec![];
    while let Some(digest) = bounded.next().await.unwrap() {
        visited.push(digest);
    }

    assert_eq!(visited.len(), 2);
    assert_eq!(visited[0], root_digest);
}

/// Shared subtrees: two root dirs reference the same sub dir.
/// Traversal deduplicates — sub visited only once.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn directory_traversal_shared_subtree() {
    let kv = DeterministicKeyValueStore::new();
    let ds = RaftDirectoryService::from_arc(kv);

    let file_digest = B3Digest::from(blake3::hash(b"shared file"));

    // Shared sub dir
    let mut shared = snix_castore::Directory::new();
    shared
        .add("f.txt".try_into().unwrap(), Node::File {
            digest: file_digest.clone(),
            size: 11,
            executable: false,
        })
        .unwrap();
    let shared_digest = ds.put(shared).await.unwrap();

    // Root with two refs to the same sub
    let mut root = snix_castore::Directory::new();
    root.add("a".try_into().unwrap(), Node::Directory {
        digest: shared_digest.clone(),
        size: 50,
    })
    .unwrap();
    root.add("b".try_into().unwrap(), Node::Directory {
        digest: shared_digest.clone(),
        size: 50,
    })
    .unwrap();
    let root_digest = ds.put(root).await.unwrap();

    let extractor = DirectoryLinkExtractor::new(Arc::new(ds.clone()));
    let mut trav = FullTraversal::new(root_digest.clone(), (), extractor);

    let mut visited = vec![];
    while let Some(digest) = trav.next().await.unwrap() {
        visited.push(digest);
    }

    // root + shared (once) + file_digest = 3
    assert_eq!(visited.len(), 3);
    assert!(visited.contains(&root_digest));
    assert!(visited.contains(&shared_digest));
    assert!(visited.contains(&file_digest));
}

/// Traversal with known heads prunes subtrees.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn directory_traversal_known_heads_prunes() {
    let kv = DeterministicKeyValueStore::new();
    let ds = RaftDirectoryService::from_arc(kv);
    let (root_digest, sub_digest, empty_digest, h1, _h2) = build_tree(&ds).await;

    // Mark sub_digest as a known head — traversal should skip it and its children (h2).
    let mut known_heads = std::collections::HashSet::new();
    known_heads.insert(sub_digest.clone());

    let extractor = DirectoryLinkExtractor::new(Arc::new(ds.clone()));
    let mut trav = FullTraversal::with_known_heads(root_digest.clone(), (), extractor, known_heads);

    let mut visited = vec![];
    while let Some(digest) = trav.next().await.unwrap() {
        visited.push(digest);
    }

    // Should visit: root, empty, h1 — but NOT sub or h2
    assert_eq!(visited.len(), 3);
    assert!(visited.contains(&root_digest));
    assert!(visited.contains(&empty_digest));
    assert!(visited.contains(&h1));
    assert!(!visited.contains(&sub_digest));
}

/// Closure sync: use DAG traversal to walk a store path's directory tree
/// and collect all content digests needed for transfer.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn closure_sync_via_dag_traversal() {
    use aspen_snix::RaftPathInfoService;
    use snix_store::pathinfoservice::PathInfo;
    use snix_store::pathinfoservice::PathInfoService;

    let kv = DeterministicKeyValueStore::new();
    let ds = RaftDirectoryService::from_arc(kv.clone());
    let ps = RaftPathInfoService::from_arc(kv);

    // Build the directory tree
    let (root_digest, sub_digest, empty_digest, h1, h2) = build_tree(&ds).await;

    // Create a store path with the root directory as its content
    let hash_bytes = blake3::hash(b"test-pkg");
    let mut sp_digest = [0u8; 20];
    sp_digest.copy_from_slice(&hash_bytes.as_bytes()[..20]);
    let store_path =
        nix_compat::store_path::StorePath::from_name_and_digest_fixed("test-pkg".try_into().unwrap(), sp_digest)
            .unwrap();

    let path_info = PathInfo {
        store_path: store_path.clone(),
        node: Node::Directory {
            digest: root_digest.clone(),
            size: 300,
        },
        references: vec![],
        nar_size: 500,
        nar_sha256: [0u8; 32],
        signatures: vec![],
        deriver: None,
        ca: None,
    };
    ps.put(path_info).await.unwrap();

    // Use DAG traversal to walk the entire content tree
    let extractor = DirectoryLinkExtractor::new(Arc::new(ds.clone()));
    let mut trav = FullTraversal::new(root_digest.clone(), (), extractor);

    let mut content_digests = vec![];
    while let Some(digest) = trav.next().await.unwrap() {
        content_digests.push(digest);
    }

    // All content digests collected in DFS order
    assert_eq!(content_digests.len(), 5);
    assert!(content_digests.contains(&root_digest));
    assert!(content_digests.contains(&sub_digest));
    assert!(content_digests.contains(&empty_digest));
    assert!(content_digests.contains(&h1));
    assert!(content_digests.contains(&h2));

    // Root is always first (DFS from root)
    assert_eq!(content_digests[0], root_digest);
}

/// extract_directory_children matches what the extractor returns.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn extract_children_matches_extractor() {
    let kv = DeterministicKeyValueStore::new();
    let ds = RaftDirectoryService::from_arc(kv);
    let (root_digest, _, _, _, _) = build_tree(&ds).await;

    // Get the root directory.
    let root_dir = ds.get(&root_digest).await.unwrap().unwrap();

    // Pure function result.
    let pure_children = extract_directory_children(&root_dir);

    // Extractor result.
    let extractor = DirectoryLinkExtractor::new(Arc::new(ds.clone()));
    let extractor_children = aspen_dag::LinkExtractor::extract_links(&extractor, &root_digest, &());

    assert_eq!(pure_children, extractor_children.unwrap());
}
