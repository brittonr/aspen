//! Integration tests for Forge sync service.
//!
//! Tests the recursive DAG traversal logic for both Git objects and COB changes.

use std::sync::Arc;

use aspen::blob::{BlobStore, InMemoryBlobStore};
use aspen::forge::git::GitBlobStore;
use aspen::forge::sync::SyncService;
use aspen::forge::{CobChange, CobOperation, CobType, SignedObject, TreeEntry};

/// Test helper to create a GitBlobStore with InMemoryBlobStore.
fn create_git_store() -> (Arc<InMemoryBlobStore>, GitBlobStore<InMemoryBlobStore>) {
    let blobs = Arc::new(InMemoryBlobStore::new());
    let secret_key = iroh::SecretKey::generate(&mut rand::rng());
    let git = GitBlobStore::new(blobs.clone(), secret_key);
    (blobs, git)
}

/// Create a linear commit chain: C1 <- C2 <- C3 <- ... <- Cn
/// Returns the hash of the last commit (head) and all commit hashes in order.
async fn create_commit_chain(
    git: &GitBlobStore<InMemoryBlobStore>,
    depth: usize,
) -> (blake3::Hash, Vec<blake3::Hash>) {
    let mut commits = Vec::with_capacity(depth);
    let mut parent: Option<blake3::Hash> = None;

    for i in 0..depth {
        // Create a unique blob for each commit
        let blob_hash = git.store_blob(format!("content-{}", i)).await.unwrap();
        let tree_hash = git
            .create_tree(&[TreeEntry::file(format!("file-{}.txt", i), blob_hash)])
            .await
            .unwrap();

        let parents = parent.map(|p| vec![p]).unwrap_or_default();
        let commit_hash = git
            .commit(tree_hash, parents, format!("Commit {}", i))
            .await
            .unwrap();

        commits.push(commit_hash);
        parent = Some(commit_hash);
    }

    let head = *commits.last().unwrap();
    (head, commits)
}

/// Create a diamond DAG:
///     C1
///    /  \
///   C2  C3
///    \  /
///     C4 (merge)
async fn create_diamond_dag(
    git: &GitBlobStore<InMemoryBlobStore>,
) -> (blake3::Hash, Vec<blake3::Hash>) {
    // Root commit C1
    let blob1 = git.store_blob(b"root content").await.unwrap();
    let tree1 = git
        .create_tree(&[TreeEntry::file("root.txt", blob1)])
        .await
        .unwrap();
    let c1 = git.commit(tree1, vec![], "Root commit").await.unwrap();

    // Branch 1: C2
    let blob2 = git.store_blob(b"branch1 content").await.unwrap();
    let tree2 = git
        .create_tree(&[
            TreeEntry::file("root.txt", blob1),
            TreeEntry::file("branch1.txt", blob2),
        ])
        .await
        .unwrap();
    let c2 = git.commit(tree2, vec![c1], "Branch 1").await.unwrap();

    // Branch 2: C3
    let blob3 = git.store_blob(b"branch2 content").await.unwrap();
    let tree3 = git
        .create_tree(&[
            TreeEntry::file("root.txt", blob1),
            TreeEntry::file("branch2.txt", blob3),
        ])
        .await
        .unwrap();
    let c3 = git.commit(tree3, vec![c1], "Branch 2").await.unwrap();

    // Merge: C4
    let tree4 = git
        .create_tree(&[
            TreeEntry::file("root.txt", blob1),
            TreeEntry::file("branch1.txt", blob2),
            TreeEntry::file("branch2.txt", blob3),
        ])
        .await
        .unwrap();
    let c4 = git.commit(tree4, vec![c2, c3], "Merge").await.unwrap();

    (c4, vec![c1, c2, c3, c4])
}

/// Create a nested tree structure with subdirectories.
async fn create_nested_tree(
    git: &GitBlobStore<InMemoryBlobStore>,
    depth: usize,
) -> blake3::Hash {
    // Build from bottom up
    let leaf_blob = git.store_blob(b"leaf content").await.unwrap();
    let mut current_tree = git
        .create_tree(&[TreeEntry::file("leaf.txt", leaf_blob)])
        .await
        .unwrap();

    for i in 0..depth {
        current_tree = git
            .create_tree(&[TreeEntry::directory(format!("level-{}", i), current_tree)])
            .await
            .unwrap();
    }

    current_tree
}

// ============================================================================
// Tests for fetch_commits
// ============================================================================

#[tokio::test]
async fn test_sync_single_commit_already_present() {
    let (blobs, git) = create_git_store();
    let sync = SyncService::new(blobs.clone());

    // Create a single commit
    let blob_hash = git.store_blob(b"hello world").await.unwrap();
    let tree_hash = git
        .create_tree(&[TreeEntry::file("hello.txt", blob_hash)])
        .await
        .unwrap();
    let commit_hash = git.commit(tree_hash, vec![], "Initial").await.unwrap();

    // Sync with no peers (all objects already present)
    let result = sync.fetch_commits(vec![commit_hash], &[]).await.unwrap();

    // Should have found all objects (commit, tree, blob)
    assert!(result.already_present >= 3, "Expected at least 3 objects (commit, tree, blob), got {}", result.already_present);
    assert_eq!(result.fetched, 0);
    assert!(result.missing.is_empty());
    assert!(result.errors.is_empty());
    assert!(!result.truncated);
    assert!(result.is_complete());
}

#[tokio::test]
async fn test_sync_linear_chain_traversal() {
    let (blobs, git) = create_git_store();
    let sync = SyncService::new(blobs.clone());

    // Create a chain of 5 commits
    let (head, _commits) = create_commit_chain(&git, 5).await;

    // Sync from head
    let result = sync.fetch_commits(vec![head], &[]).await.unwrap();

    // Each commit has: 1 commit + 1 tree + 1 blob = 3 objects
    // 5 commits * 3 = 15 objects minimum
    assert!(result.already_present >= 15, "Expected at least 15 objects, got {}", result.already_present);
    assert!(result.is_complete());
}

#[tokio::test]
async fn test_sync_diamond_dag_traversal() {
    let (blobs, git) = create_git_store();
    let sync = SyncService::new(blobs.clone());

    // Create diamond DAG
    let (merge_head, _commits) = create_diamond_dag(&git).await;

    // Sync from merge commit
    let result = sync.fetch_commits(vec![merge_head], &[]).await.unwrap();

    // Diamond has: 4 commits, 4 trees, 3 blobs = 11 objects minimum
    assert!(result.already_present >= 11, "Expected at least 11 objects, got {}", result.already_present);
    assert!(result.is_complete());
}

#[tokio::test]
async fn test_sync_nested_tree_traversal() {
    let (blobs, git) = create_git_store();
    let sync = SyncService::new(blobs.clone());

    // Create nested tree with 3 levels
    let root_tree = create_nested_tree(&git, 3).await;
    let commit = git.commit(root_tree, vec![], "Nested tree").await.unwrap();

    // Sync from commit
    let result = sync.fetch_commits(vec![commit], &[]).await.unwrap();

    // 1 commit + 4 trees (root + 3 levels) + 1 blob = 6 objects minimum
    assert!(result.already_present >= 6, "Expected at least 6 objects, got {}", result.already_present);
    assert!(result.is_complete());
}

#[tokio::test]
async fn test_sync_deduplicates_shared_objects() {
    let (blobs, git) = create_git_store();
    let sync = SyncService::new(blobs.clone());

    // Create diamond DAG where the same blob/tree is reachable via multiple paths
    let (merge_head, _) = create_diamond_dag(&git).await;

    // Sync from merge commit
    let result = sync.fetch_commits(vec![merge_head], &[]).await.unwrap();

    // The root blob (blob1) and root tree (tree1) should only be counted once
    // Even though they're reachable via both C2 and C3
    let first_count = result.already_present;

    // Run again - should get same count (no double counting)
    let result2 = sync.fetch_commits(vec![merge_head], &[]).await.unwrap();
    assert_eq!(result2.already_present, first_count);
}

#[tokio::test]
async fn test_sync_handles_missing_objects() {
    let blobs = Arc::new(InMemoryBlobStore::new());
    let sync = SyncService::new(blobs.clone());

    // Try to sync a non-existent commit
    let fake_hash = blake3::hash(b"nonexistent");
    let result = sync.fetch_commits(vec![fake_hash], &[]).await.unwrap();

    // Should report as missing (no peers to fetch from)
    assert_eq!(result.already_present, 0);
    assert_eq!(result.fetched, 0);
    assert_eq!(result.missing.len(), 1);
    assert_eq!(result.missing[0], fake_hash);
}

#[tokio::test]
async fn test_sync_multiple_seed_commits() {
    let (blobs, git) = create_git_store();
    let sync = SyncService::new(blobs.clone());

    // Create two independent commit chains
    let (head1, _) = create_commit_chain(&git, 3).await;
    let (head2, _) = create_commit_chain(&git, 2).await;

    // Sync both
    let result = sync.fetch_commits(vec![head1, head2], &[]).await.unwrap();

    // Should traverse both chains
    // Chain 1: 3 commits * 3 objects = 9
    // Chain 2: 2 commits * 3 objects = 6
    // Total: 15 minimum
    assert!(result.already_present >= 15, "Expected at least 15 objects, got {}", result.already_present);
    assert!(result.is_complete());
}

// ============================================================================
// Tests for fetch_cob_changes
// ============================================================================

/// Helper to store a COB change in the blob store.
async fn store_cob_change(
    blobs: &InMemoryBlobStore,
    secret_key: &iroh::SecretKey,
    change: CobChange,
) -> blake3::Hash {
    let signed = SignedObject::new(change, secret_key).unwrap();
    let hash = signed.hash();
    let bytes = signed.to_bytes();
    blobs.add_bytes(&bytes).await.unwrap();
    hash
}

#[tokio::test]
async fn test_sync_cob_single_change() {
    let blobs = Arc::new(InMemoryBlobStore::new());
    let sync = SyncService::new(blobs.clone());
    let secret_key = iroh::SecretKey::generate(&mut rand::rng());

    // Create a root COB change
    let cob_id = blake3::hash(b"issue-1");
    let change = CobChange::root(
        CobType::Issue,
        cob_id,
        CobOperation::CreateIssue {
            title: "Bug report".to_string(),
            body: "Something is broken".to_string(),
            labels: vec![],
        },
    );
    let change_hash = store_cob_change(&blobs, &secret_key, change).await;

    // Sync
    let result = sync.fetch_cob_changes(vec![change_hash], &[]).await.unwrap();

    assert_eq!(result.already_present, 1);
    assert!(result.is_complete());
}

#[tokio::test]
async fn test_sync_cob_chain() {
    let blobs = Arc::new(InMemoryBlobStore::new());
    let sync = SyncService::new(blobs.clone());
    let secret_key = iroh::SecretKey::generate(&mut rand::rng());

    let cob_id = blake3::hash(b"issue-1");

    // Create root
    let root = CobChange::root(
        CobType::Issue,
        cob_id,
        CobOperation::CreateIssue {
            title: "Bug".to_string(),
            body: "Details".to_string(),
            labels: vec![],
        },
    );
    let root_hash = store_cob_change(&blobs, &secret_key, root).await;

    // Create child 1
    let child1 = CobChange::new(
        CobType::Issue,
        cob_id,
        vec![root_hash],
        CobOperation::Comment {
            body: "Comment 1".to_string(),
        },
    );
    let child1_hash = store_cob_change(&blobs, &secret_key, child1).await;

    // Create child 2
    let child2 = CobChange::new(
        CobType::Issue,
        cob_id,
        vec![child1_hash],
        CobOperation::Comment {
            body: "Comment 2".to_string(),
        },
    );
    let child2_hash = store_cob_change(&blobs, &secret_key, child2).await;

    // Sync from head
    let result = sync.fetch_cob_changes(vec![child2_hash], &[]).await.unwrap();

    assert_eq!(result.already_present, 3);
    assert!(result.is_complete());
}

#[tokio::test]
async fn test_sync_cob_diamond() {
    let blobs = Arc::new(InMemoryBlobStore::new());
    let sync = SyncService::new(blobs.clone());
    let secret_key = iroh::SecretKey::generate(&mut rand::rng());

    let cob_id = blake3::hash(b"issue-1");

    // Root
    let root = CobChange::root(
        CobType::Issue,
        cob_id,
        CobOperation::CreateIssue {
            title: "Bug".to_string(),
            body: "Details".to_string(),
            labels: vec![],
        },
    );
    let root_hash = store_cob_change(&blobs, &secret_key, root).await;

    // Branch A
    let branch_a = CobChange::new(
        CobType::Issue,
        cob_id,
        vec![root_hash],
        CobOperation::AddLabel {
            label: "bug".to_string(),
        },
    );
    let branch_a_hash = store_cob_change(&blobs, &secret_key, branch_a).await;

    // Branch B
    let branch_b = CobChange::new(
        CobType::Issue,
        cob_id,
        vec![root_hash],
        CobOperation::AddLabel {
            label: "urgent".to_string(),
        },
    );
    let branch_b_hash = store_cob_change(&blobs, &secret_key, branch_b).await;

    // Merge (has both branches as parents)
    let merge = CobChange::new(
        CobType::Issue,
        cob_id,
        vec![branch_a_hash, branch_b_hash],
        CobOperation::Comment {
            body: "Merged changes".to_string(),
        },
    );
    let merge_hash = store_cob_change(&blobs, &secret_key, merge).await;

    // Sync from merge
    let result = sync.fetch_cob_changes(vec![merge_hash], &[]).await.unwrap();

    // Should find all 4 changes
    assert_eq!(result.already_present, 4);
    assert!(result.is_complete());
}

#[tokio::test]
async fn test_sync_cob_missing_change() {
    let blobs = Arc::new(InMemoryBlobStore::new());
    let sync = SyncService::new(blobs.clone());

    // Try to sync a non-existent change
    let fake_hash = blake3::hash(b"nonexistent-cob");
    let result = sync.fetch_cob_changes(vec![fake_hash], &[]).await.unwrap();

    assert_eq!(result.already_present, 0);
    assert_eq!(result.missing.len(), 1);
    assert_eq!(result.missing[0], fake_hash);
}
