//! End-to-end hosting test for Forge.
//!
//! This test simulates hosting a git repository on Forge, testing:
//! - Repository creation
//! - Blob storage (git objects)
//! - Tree creation
//! - Commit creation
//! - Ref management (branches, tags)
//! - Git bridge operations (list refs, fetch, push)
//! - Issue and patch workflows
//!
//! This is a comprehensive test to identify gaps in the implementation.

use std::sync::Arc;

use aspen::api::DeterministicKeyValueStore;
use aspen::blob::InMemoryBlobStore;
use aspen::forge::ForgeNode;
use aspen::forge::TreeEntry;
use aspen::forge::identity::RepoId;

/// Create a test ForgeNode with in-memory storage.
async fn create_test_forge_node() -> ForgeNode<InMemoryBlobStore, DeterministicKeyValueStore> {
    let blobs = Arc::new(InMemoryBlobStore::new());
    let kv = DeterministicKeyValueStore::new(); // Returns Arc<Self>
    let secret_key = iroh::SecretKey::generate(&mut rand::rng());

    ForgeNode::new(blobs, kv, secret_key)
}

// ============================================================================
// Repository Creation and Management Tests
// ============================================================================

#[tokio::test]
async fn test_create_repository() {
    let forge = create_test_forge_node().await;

    // Create a repository
    let identity = forge
        .create_repo("test-repo", vec![forge.public_key()], 1)
        .await
        .expect("should create repository");

    assert_eq!(identity.name, "test-repo");
    assert_eq!(identity.delegates.len(), 1);
    assert_eq!(identity.threshold, 1);
    assert_eq!(identity.default_branch, "main");

    // Verify repo can be retrieved
    let retrieved = forge.get_repo(&identity.repo_id()).await.expect("should get repo");

    assert_eq!(retrieved.name, "test-repo");
}

#[tokio::test]
async fn test_create_duplicate_repo_fails() {
    let forge = create_test_forge_node().await;

    // Create first repo
    let _identity = forge
        .create_repo("duplicate-test", vec![forge.public_key()], 1)
        .await
        .expect("should create first repo");

    // Creating duplicate should fail
    let result = forge.create_repo("duplicate-test", vec![forge.public_key()], 1).await;

    assert!(result.is_err(), "duplicate repo creation should fail");
}

// ============================================================================
// Git Object Storage Tests
// ============================================================================

#[tokio::test]
async fn test_store_and_retrieve_blob() {
    let forge = create_test_forge_node().await;

    let content = b"Hello, Forge!".to_vec();
    let hash = forge.git.store_blob(content.clone()).await.expect("should store blob");

    // Retrieve the blob
    let retrieved = forge.git.get_blob(&hash).await.expect("should get blob");

    assert_eq!(retrieved, content);
}

#[tokio::test]
async fn test_store_multiple_blobs() {
    let forge = create_test_forge_node().await;

    let files = vec![
        ("README.md", b"# My Project\n\nThis is a test.".to_vec()),
        ("src/main.rs", b"fn main() { println!(\"Hello!\"); }".to_vec()),
        ("Cargo.toml", b"[package]\nname = \"test\"\nversion = \"0.1.0\"".to_vec()),
    ];

    let mut hashes = Vec::new();
    for (name, content) in &files {
        let hash = forge.git.store_blob(content.clone()).await.expect(&format!("should store {}", name));
        hashes.push(hash);
    }

    // Verify all blobs are retrievable
    for (i, (name, content)) in files.iter().enumerate() {
        let retrieved = forge.git.get_blob(&hashes[i]).await.expect(&format!("should get {}", name));
        assert_eq!(retrieved, *content);
    }
}

// ============================================================================
// Tree Creation Tests
// ============================================================================

#[tokio::test]
async fn test_create_tree() {
    let forge = create_test_forge_node().await;

    // First store some blobs
    let readme_content = b"# Test\n".to_vec();
    let readme_hash = forge.git.store_blob(readme_content).await.expect("should store readme");

    let main_content = b"fn main() {}".to_vec();
    let main_hash = forge.git.store_blob(main_content).await.expect("should store main.rs");

    // Create a tree with these blobs
    let entries = vec![
        TreeEntry {
            mode: 0o100644, // regular file
            name: "README.md".to_string(),
            hash: *readme_hash.as_bytes(),
        },
        TreeEntry {
            mode: 0o100644,
            name: "main.rs".to_string(),
            hash: *main_hash.as_bytes(),
        },
    ];

    let tree_hash = forge.git.create_tree(&entries).await.expect("should create tree");

    // Retrieve and verify tree
    let tree = forge.git.get_tree(&tree_hash).await.expect("should get tree");

    assert_eq!(tree.entries.len(), 2);
    assert!(tree.entries.iter().any(|e| e.name == "README.md"));
    assert!(tree.entries.iter().any(|e| e.name == "main.rs"));
}

#[tokio::test]
async fn test_create_nested_tree() {
    let forge = create_test_forge_node().await;

    // Create src/main.rs
    let main_content = b"fn main() {}".to_vec();
    let main_hash = forge.git.store_blob(main_content).await.expect("store main.rs");

    // Create src/ subtree
    let src_entries = vec![TreeEntry {
        mode: 0o100644,
        name: "main.rs".to_string(),
        hash: *main_hash.as_bytes(),
    }];
    let src_tree_hash = forge.git.create_tree(&src_entries).await.expect("create src tree");

    // Create root README
    let readme_content = b"# Project\n".to_vec();
    let readme_hash = forge.git.store_blob(readme_content).await.expect("store readme");

    // Create root tree with src/ subtree
    let root_entries = vec![
        TreeEntry {
            mode: 0o100644,
            name: "README.md".to_string(),
            hash: *readme_hash.as_bytes(),
        },
        TreeEntry {
            mode: 0o040000, // directory
            name: "src".to_string(),
            hash: *src_tree_hash.as_bytes(),
        },
    ];
    let root_tree_hash = forge.git.create_tree(&root_entries).await.expect("create root tree");

    // Verify structure
    let root_tree = forge.git.get_tree(&root_tree_hash).await.expect("get root tree");
    assert_eq!(root_tree.entries.len(), 2);

    let src_entry = root_tree.entries.iter().find(|e| e.name == "src").expect("src entry");
    assert_eq!(src_entry.mode, 0o040000);
}

// ============================================================================
// Commit Creation Tests
// ============================================================================

#[tokio::test]
async fn test_create_initial_commit() {
    let forge = create_test_forge_node().await;

    // Create content
    let content = b"Initial content".to_vec();
    let blob_hash = forge.git.store_blob(content).await.expect("store blob");

    let entries = vec![TreeEntry {
        mode: 0o100644,
        name: "README.md".to_string(),
        hash: *blob_hash.as_bytes(),
    }];
    let tree_hash = forge.git.create_tree(&entries).await.expect("create tree");

    // Create initial commit (no parents)
    let commit_hash = forge.git.commit(tree_hash, vec![], "Initial commit").await.expect("create commit");

    // Verify commit
    let commit = forge.git.get_commit(&commit_hash).await.expect("get commit");
    assert_eq!(commit.message, "Initial commit");
    assert!(commit.parents.is_empty());
    assert_eq!(commit.tree, *tree_hash.as_bytes());
}

#[tokio::test]
async fn test_create_commit_chain() {
    let forge = create_test_forge_node().await;

    // First commit
    let content1 = b"Version 1".to_vec();
    let blob1 = forge.git.store_blob(content1).await.expect("store blob 1");
    let entries1 = vec![TreeEntry {
        mode: 0o100644,
        name: "file.txt".to_string(),
        hash: *blob1.as_bytes(),
    }];
    let tree1 = forge.git.create_tree(&entries1).await.expect("create tree 1");
    let commit1 = forge.git.commit(tree1, vec![], "First commit").await.expect("create commit 1");

    // Second commit with parent
    let content2 = b"Version 2".to_vec();
    let blob2 = forge.git.store_blob(content2).await.expect("store blob 2");
    let entries2 = vec![TreeEntry {
        mode: 0o100644,
        name: "file.txt".to_string(),
        hash: *blob2.as_bytes(),
    }];
    let tree2 = forge.git.create_tree(&entries2).await.expect("create tree 2");
    let commit2 = forge
        .git
        .commit(tree2, vec![commit1], "Second commit")
        .await
        .expect("create commit 2");

    // Verify parent link
    let retrieved = forge.git.get_commit(&commit2).await.expect("get commit 2");
    assert_eq!(retrieved.parents.len(), 1);
    assert_eq!(retrieved.parents[0], *commit1.as_bytes());
}

// ============================================================================
// Ref Management Tests
// ============================================================================

#[tokio::test]
async fn test_set_and_get_ref() {
    let forge = create_test_forge_node().await;

    // Create repository
    let identity = forge
        .create_repo("ref-test", vec![forge.public_key()], 1)
        .await
        .expect("create repo");
    let repo_id = identity.repo_id();

    // Create a commit to point to
    let content = b"test".to_vec();
    let blob = forge.git.store_blob(content).await.expect("store blob");
    let entries = vec![TreeEntry {
        mode: 0o100644,
        name: "test.txt".to_string(),
        hash: *blob.as_bytes(),
    }];
    let tree = forge.git.create_tree(&entries).await.expect("create tree");
    let commit = forge.git.commit(tree, vec![], "Test commit").await.expect("create commit");

    // Set main branch
    forge.refs.set(&repo_id, "heads/main", commit).await.expect("set ref");

    // Get the ref
    let retrieved = forge
        .refs
        .get(&repo_id, "heads/main")
        .await
        .expect("get ref result")
        .expect("ref should exist");

    assert_eq!(retrieved, commit);
}

#[tokio::test]
async fn test_list_branches() {
    let forge = create_test_forge_node().await;

    let identity = forge
        .create_repo("branch-list-test", vec![forge.public_key()], 1)
        .await
        .expect("create repo");
    let repo_id = identity.repo_id();

    // Create a commit
    let content = b"test".to_vec();
    let blob = forge.git.store_blob(content).await.expect("store blob");
    let entries = vec![TreeEntry {
        mode: 0o100644,
        name: "test.txt".to_string(),
        hash: *blob.as_bytes(),
    }];
    let tree = forge.git.create_tree(&entries).await.expect("create tree");
    let commit = forge.git.commit(tree, vec![], "Test").await.expect("create commit");

    // Create multiple branches
    forge.refs.set(&repo_id, "heads/main", commit).await.expect("set main");
    forge.refs.set(&repo_id, "heads/develop", commit).await.expect("set develop");
    forge.refs.set(&repo_id, "heads/feature/test", commit).await.expect("set feature");

    // List branches
    // Note: list_branches strips the "heads/" prefix
    let branches = forge.refs.list_branches(&repo_id).await.expect("list branches");

    assert_eq!(branches.len(), 3);
    assert!(branches.iter().any(|(name, _)| name == "main"));
    assert!(branches.iter().any(|(name, _)| name == "develop"));
    assert!(branches.iter().any(|(name, _)| name == "feature/test"));
}

#[tokio::test]
async fn test_list_tags() {
    let forge = create_test_forge_node().await;

    let identity = forge
        .create_repo("tag-list-test", vec![forge.public_key()], 1)
        .await
        .expect("create repo");
    let repo_id = identity.repo_id();

    // Create a commit
    let content = b"test".to_vec();
    let blob = forge.git.store_blob(content).await.expect("store blob");
    let entries = vec![TreeEntry {
        mode: 0o100644,
        name: "test.txt".to_string(),
        hash: *blob.as_bytes(),
    }];
    let tree = forge.git.create_tree(&entries).await.expect("create tree");
    let commit = forge.git.commit(tree, vec![], "Test").await.expect("create commit");

    // Create tags
    forge.refs.set(&repo_id, "tags/v1.0.0", commit).await.expect("set v1.0.0");
    forge.refs.set(&repo_id, "tags/v1.1.0", commit).await.expect("set v1.1.0");

    // List tags
    // Note: list_tags strips the "tags/" prefix
    let tags = forge.refs.list_tags(&repo_id).await.expect("list tags");

    assert_eq!(tags.len(), 2);
    assert!(tags.iter().any(|(name, _)| name == "v1.0.0"));
    assert!(tags.iter().any(|(name, _)| name == "v1.1.0"));
}

#[tokio::test]
async fn test_compare_and_set_ref() {
    let forge = create_test_forge_node().await;

    let identity = forge
        .create_repo("cas-test", vec![forge.public_key()], 1)
        .await
        .expect("create repo");
    let repo_id = identity.repo_id();

    // Create two commits
    let content1 = b"v1".to_vec();
    let blob1 = forge.git.store_blob(content1).await.expect("store blob1");
    let entries1 = vec![TreeEntry {
        mode: 0o100644,
        name: "test.txt".to_string(),
        hash: *blob1.as_bytes(),
    }];
    let tree1 = forge.git.create_tree(&entries1).await.expect("create tree1");
    let commit1 = forge.git.commit(tree1, vec![], "Commit 1").await.expect("create commit1");

    let content2 = b"v2".to_vec();
    let blob2 = forge.git.store_blob(content2).await.expect("store blob2");
    let entries2 = vec![TreeEntry {
        mode: 0o100644,
        name: "test.txt".to_string(),
        hash: *blob2.as_bytes(),
    }];
    let tree2 = forge.git.create_tree(&entries2).await.expect("create tree2");
    let commit2 = forge.git.commit(tree2, vec![commit1], "Commit 2").await.expect("create commit2");

    // Set initial ref
    forge.refs.set(&repo_id, "heads/main", commit1).await.expect("set initial");

    // CAS with correct expected value should succeed
    forge
        .refs
        .compare_and_set(&repo_id, "heads/main", Some(commit1), commit2)
        .await
        .expect("CAS should succeed");

    // Verify update
    let current = forge.refs.get(&repo_id, "heads/main").await.expect("get").expect("exists");
    assert_eq!(current, commit2);
}

#[tokio::test]
async fn test_delete_ref() {
    let forge = create_test_forge_node().await;

    let identity = forge
        .create_repo("delete-ref-test", vec![forge.public_key()], 1)
        .await
        .expect("create repo");
    let repo_id = identity.repo_id();

    // Create a commit and branch
    let content = b"test".to_vec();
    let blob = forge.git.store_blob(content).await.expect("store blob");
    let entries = vec![TreeEntry {
        mode: 0o100644,
        name: "test.txt".to_string(),
        hash: *blob.as_bytes(),
    }];
    let tree = forge.git.create_tree(&entries).await.expect("create tree");
    let commit = forge.git.commit(tree, vec![], "Test").await.expect("create commit");

    forge.refs.set(&repo_id, "heads/to-delete", commit).await.expect("set ref");

    // Delete the ref
    forge.refs.delete(&repo_id, "heads/to-delete").await.expect("delete ref");

    // Verify it's gone
    let result = forge.refs.get(&repo_id, "heads/to-delete").await.expect("get result");
    assert!(result.is_none());
}

// ============================================================================
// Issue Workflow Tests
// ============================================================================

#[tokio::test]
async fn test_full_issue_workflow() {
    let forge = create_test_forge_node().await;

    let identity = forge
        .create_repo("issue-workflow-test", vec![forge.public_key()], 1)
        .await
        .expect("create repo");
    let repo_id = identity.repo_id();

    // Create an issue
    let issue_id = forge
        .cobs
        .create_issue(&repo_id, "Bug: Something is broken", "Steps to reproduce:\n1. Do X\n2. See error", vec!["bug".to_string()])
        .await
        .expect("create issue");

    // Add a comment
    forge
        .cobs
        .add_comment(&repo_id, &issue_id, "I can reproduce this issue.")
        .await
        .expect("add comment");

    // Add another comment
    forge
        .cobs
        .add_comment(&repo_id, &issue_id, "Working on a fix.")
        .await
        .expect("add second comment");

    // Resolve the issue to check state
    let resolved = forge.cobs.resolve_issue(&repo_id, &issue_id).await.expect("resolve issue");

    assert_eq!(resolved.title, "Bug: Something is broken");
    assert!(resolved.state.is_open());
    assert_eq!(resolved.comments.len(), 2);
    assert!(resolved.labels.contains(&"bug".to_string()));

    // Close the issue
    forge
        .cobs
        .close_issue(&repo_id, &issue_id, Some("fixed".to_string()))
        .await
        .expect("close issue");

    // Verify closed state
    let closed = forge.cobs.resolve_issue(&repo_id, &issue_id).await.expect("resolve after close");
    assert!(!closed.state.is_open());

    // Reopen the issue
    forge.cobs.reopen_issue(&repo_id, &issue_id).await.expect("reopen issue");

    // Verify reopened
    let reopened = forge.cobs.resolve_issue(&repo_id, &issue_id).await.expect("resolve after reopen");
    assert!(reopened.state.is_open());
}

#[tokio::test]
async fn test_list_issues() {
    let forge = create_test_forge_node().await;

    let identity = forge
        .create_repo("list-issues-test", vec![forge.public_key()], 1)
        .await
        .expect("create repo");
    let repo_id = identity.repo_id();

    // Create multiple issues
    let _id1 = forge
        .cobs
        .create_issue(&repo_id, "Issue 1", "Body 1", vec![])
        .await
        .expect("create issue 1");

    let _id2 = forge
        .cobs
        .create_issue(&repo_id, "Issue 2", "Body 2", vec![])
        .await
        .expect("create issue 2");

    let _id3 = forge
        .cobs
        .create_issue(&repo_id, "Issue 3", "Body 3", vec![])
        .await
        .expect("create issue 3");

    // List issues
    let issues = forge.cobs.list_issues(&repo_id).await.expect("list issues");

    assert_eq!(issues.len(), 3);
}

// ============================================================================
// Patch Workflow Tests
// ============================================================================

#[tokio::test]
async fn test_full_patch_workflow() {
    let forge = create_test_forge_node().await;

    let identity = forge
        .create_repo("patch-workflow-test", vec![forge.public_key()], 1)
        .await
        .expect("create repo");
    let repo_id = identity.repo_id();

    // Create base commit (simulating main branch)
    let content1 = b"Original content".to_vec();
    let blob1 = forge.git.store_blob(content1).await.expect("store blob1");
    let entries1 = vec![TreeEntry {
        mode: 0o100644,
        name: "file.txt".to_string(),
        hash: *blob1.as_bytes(),
    }];
    let tree1 = forge.git.create_tree(&entries1).await.expect("create tree1");
    let base_commit = forge.git.commit(tree1, vec![], "Initial commit").await.expect("create base");

    // Create head commit (simulating feature branch)
    let content2 = b"Modified content".to_vec();
    let blob2 = forge.git.store_blob(content2).await.expect("store blob2");
    let entries2 = vec![TreeEntry {
        mode: 0o100644,
        name: "file.txt".to_string(),
        hash: *blob2.as_bytes(),
    }];
    let tree2 = forge.git.create_tree(&entries2).await.expect("create tree2");
    let head_commit = forge
        .git
        .commit(tree2, vec![base_commit], "Add feature")
        .await
        .expect("create head");

    // Set up branches
    forge.refs.set(&repo_id, "heads/main", base_commit).await.expect("set main");
    forge.refs.set(&repo_id, "heads/feature", head_commit).await.expect("set feature");

    // Create a patch (pull request)
    let patch_id = forge
        .cobs
        .create_patch(&repo_id, "Add new feature", "This patch adds a cool feature.", base_commit, head_commit)
        .await
        .expect("create patch");

    // Resolve and verify patch
    let patch = forge.cobs.resolve_patch(&repo_id, &patch_id).await.expect("resolve patch");
    assert_eq!(patch.title, "Add new feature");
    assert!(matches!(patch.state, aspen::forge::cob::PatchState::Open));

    // Add approval
    forge
        .cobs
        .approve_patch(&repo_id, &patch_id, head_commit, Some("LGTM!".to_string()))
        .await
        .expect("approve patch");

    // Verify approval
    let approved = forge.cobs.resolve_patch(&repo_id, &patch_id).await.expect("resolve after approval");
    assert_eq!(approved.approvals.len(), 1);

    // Create merge commit
    let merge_tree = forge.git.create_tree(&entries2).await.expect("create merge tree");
    let merge_commit = forge
        .git
        .commit(merge_tree, vec![base_commit, head_commit], "Merge feature into main")
        .await
        .expect("create merge commit");

    // Merge the patch
    forge
        .cobs
        .merge_patch(&repo_id, &patch_id, merge_commit)
        .await
        .expect("merge patch");

    // Update main branch
    forge.refs.set(&repo_id, "heads/main", merge_commit).await.expect("update main");

    // Verify merged state
    let merged = forge.cobs.resolve_patch(&repo_id, &patch_id).await.expect("resolve after merge");
    assert!(matches!(merged.state, aspen::forge::cob::PatchState::Merged { .. }));
}

// ============================================================================
// Git Bridge Tests (SHA-1 ↔ BLAKE3 mapping)
// ============================================================================

#[cfg(feature = "git-bridge")]
mod git_bridge_tests {
    use std::collections::HashSet;

    use super::*;
    use aspen::forge::git::bridge::GitExporter;
    use aspen::forge::git::bridge::GitImporter;
    use aspen::forge::git::bridge::GitObjectType;
    use aspen::forge::git::bridge::HashMappingStore;
    use aspen::forge::git::bridge::Sha1Hash;
    use aspen_core::hlc::HLC;
    use sha1::Digest;
    use sha1::Sha1;

    /// Compute SHA-1 hash of git object bytes (git format: `<type> <size>\0<content>`).
    fn compute_sha1(object_type: &str, content: &[u8]) -> Sha1Hash {
        let header = format!("{} {}\0", object_type, content.len());
        let mut hasher = Sha1::new();
        hasher.update(header.as_bytes());
        hasher.update(content);
        let result = hasher.finalize();
        Sha1Hash::from_bytes(result.into())
    }

    // -------------------------------------------------------------------------
    // HashMappingStore Tests
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_hash_mapping_store_basic() {
        let forge = create_test_forge_node().await;
        let identity = forge
            .create_repo("mapping-basic-test", vec![forge.public_key()], 1)
            .await
            .expect("create repo");
        let repo_id = identity.repo_id();

        let mapping = HashMappingStore::new(forge.kv().clone());

        // Create a mapping
        let blake3 = blake3::hash(b"test content");
        let sha1 = Sha1Hash::from_bytes([0xab; 20]);

        // Store the mapping
        mapping
            .store(&repo_id, blake3, sha1, GitObjectType::Blob)
            .await
            .expect("store mapping");

        // Verify bidirectional lookup
        let (retrieved_sha1, obj_type) = mapping
            .get_sha1(&repo_id, &blake3)
            .await
            .expect("get sha1")
            .expect("should exist");
        assert_eq!(retrieved_sha1, sha1);
        assert_eq!(obj_type, GitObjectType::Blob);

        let (retrieved_blake3, obj_type) = mapping
            .get_blake3(&repo_id, &sha1)
            .await
            .expect("get blake3")
            .expect("should exist");
        assert_eq!(retrieved_blake3, blake3);
        assert_eq!(obj_type, GitObjectType::Blob);
    }

    #[tokio::test]
    async fn test_hash_mapping_store_batch() {
        let forge = create_test_forge_node().await;
        let identity = forge
            .create_repo("mapping-batch-test", vec![forge.public_key()], 1)
            .await
            .expect("create repo");
        let repo_id = identity.repo_id();

        let mapping = HashMappingStore::new(forge.kv().clone());

        // Create multiple mappings
        let mappings: Vec<_> = (0..10)
            .map(|i| {
                let blake3 = blake3::hash(&[i as u8; 32]);
                let sha1 = Sha1Hash::from_bytes([i as u8; 20]);
                (blake3, sha1, GitObjectType::Blob)
            })
            .collect();

        // Store batch
        mapping
            .store_batch(&repo_id, &mappings)
            .await
            .expect("store batch");

        // Verify all mappings exist
        for (blake3, sha1, _) in &mappings {
            assert!(
                mapping.has_blake3(&repo_id, blake3).await.expect("check blake3"),
                "should have blake3 mapping"
            );
            assert!(
                mapping.has_sha1(&repo_id, sha1).await.expect("check sha1"),
                "should have sha1 mapping"
            );
        }
    }

    #[tokio::test]
    async fn test_hash_mapping_store_cache() {
        let forge = create_test_forge_node().await;
        let identity = forge
            .create_repo("mapping-cache-test", vec![forge.public_key()], 1)
            .await
            .expect("create repo");
        let repo_id = identity.repo_id();

        let mapping = HashMappingStore::new(forge.kv().clone());

        let blake3 = blake3::hash(b"cache test");
        let sha1 = Sha1Hash::from_bytes([0xcc; 20]);

        mapping
            .store(&repo_id, blake3, sha1, GitObjectType::Commit)
            .await
            .expect("store");

        // First lookup populates cache
        let _ = mapping.get_sha1(&repo_id, &blake3).await.expect("first lookup");

        // Check cache stats
        let (len, _cap) = mapping.cache_stats();
        assert!(len > 0, "cache should have entries");

        // Clear cache
        mapping.clear_cache();
        let (len_after, _) = mapping.cache_stats();
        assert_eq!(len_after, 0, "cache should be empty after clear");

        // Lookup still works (from KV)
        let result = mapping.get_sha1(&repo_id, &blake3).await.expect("lookup after clear");
        assert!(result.is_some(), "should still find from KV");
    }

    // -------------------------------------------------------------------------
    // GitImporter Tests
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_git_importer_blob() {
        let forge = create_test_forge_node().await;
        let identity = forge
            .create_repo("importer-blob-test", vec![forge.public_key()], 1)
            .await
            .expect("create repo");
        let repo_id = identity.repo_id();

        let mapping = Arc::new(HashMappingStore::new(forge.kv().clone()));
        let secret_key = iroh::SecretKey::generate(&mut rand::rng());
        let hlc = HLC::default();

        let importer = GitImporter::new(
            Arc::clone(&mapping),
            forge.git.blobs().clone(),
            Arc::new(forge.refs.clone()),
            secret_key,
            hlc,
        );

        // Import a raw git blob
        let blob_content = b"Hello from Git!";
        let sha1 = compute_sha1("blob", blob_content);

        let result = importer
            .import_object_raw(&repo_id, sha1, "blob", blob_content)
            .await
            .expect("import blob");

        assert!(!result.already_existed, "should be new import");

        // Verify mapping exists
        let blake3 = importer.get_blake3(&repo_id, &sha1).await.expect("get blake3").expect("should exist");
        assert!(!blake3.as_bytes().iter().all(|&b| b == 0), "blake3 should not be all zeros");

        // Import again should show it already exists
        let result2 = importer
            .import_object_raw(&repo_id, sha1, "blob", blob_content)
            .await
            .expect("import blob again");
        assert!(result2.already_existed, "should already exist");
    }

    #[tokio::test]
    async fn test_git_importer_full_object() {
        let forge = create_test_forge_node().await;
        let identity = forge
            .create_repo("importer-full-test", vec![forge.public_key()], 1)
            .await
            .expect("create repo");
        let repo_id = identity.repo_id();

        let mapping = Arc::new(HashMappingStore::new(forge.kv().clone()));
        let secret_key = iroh::SecretKey::generate(&mut rand::rng());
        let hlc = HLC::default();

        let importer = GitImporter::new(
            Arc::clone(&mapping),
            forge.git.blobs().clone(),
            Arc::new(forge.refs.clone()),
            secret_key,
            hlc,
        );

        // Create a full git blob object (with header)
        let content = b"Test blob content";
        let header = format!("blob {}\0", content.len());
        let mut git_bytes = Vec::new();
        git_bytes.extend_from_slice(header.as_bytes());
        git_bytes.extend_from_slice(content);

        // Import the full object
        let blake3 = importer.import_object(&repo_id, &git_bytes).await.expect("import object");

        // Verify it was stored
        let sha1 = compute_sha1("blob", content);
        let retrieved = mapping.get_blake3(&repo_id, &sha1).await.expect("get blake3").expect("should exist");
        assert_eq!(retrieved.0, blake3);
    }

    // -------------------------------------------------------------------------
    // GitExporter Tests
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_git_exporter_list_refs() {
        let forge = create_test_forge_node().await;
        let identity = forge
            .create_repo("exporter-list-test", vec![forge.public_key()], 1)
            .await
            .expect("create repo");
        let repo_id = identity.repo_id();

        // Create a commit using Forge's native API
        let content = b"Test file content".to_vec();
        let blob_hash = forge.git.store_blob(content).await.expect("store blob");

        let entries = vec![TreeEntry {
            mode: 0o100644,
            name: "test.txt".to_string(),
            hash: *blob_hash.as_bytes(),
        }];
        let tree_hash = forge.git.create_tree(&entries).await.expect("create tree");
        let commit_hash = forge.git.commit(tree_hash, vec![], "Test commit").await.expect("create commit");

        // Set up branches
        forge.refs.set(&repo_id, "heads/main", commit_hash).await.expect("set main");
        forge.refs.set(&repo_id, "heads/develop", commit_hash).await.expect("set develop");

        // Create exporter
        let mapping = Arc::new(HashMappingStore::new(forge.kv().clone()));
        let secret_key = iroh::SecretKey::generate(&mut rand::rng());
        let hlc = HLC::default();

        let exporter = GitExporter::new(
            Arc::clone(&mapping),
            forge.git.blobs().clone(),
            Arc::new(forge.refs.clone()),
            secret_key,
            hlc,
        );

        // List refs
        let ref_list = exporter.list_refs(&repo_id).await.expect("list refs");

        // Should have both branches
        assert!(ref_list.len() >= 2, "should have at least 2 refs");
        assert!(
            ref_list.iter().any(|(name, sha1)| name.contains("main") && sha1.is_some()),
            "should have main branch with SHA-1"
        );
        assert!(
            ref_list.iter().any(|(name, sha1)| name.contains("develop") && sha1.is_some()),
            "should have develop branch with SHA-1"
        );
    }

    #[tokio::test]
    async fn test_git_exporter_export_object() {
        let forge = create_test_forge_node().await;
        let identity = forge
            .create_repo("exporter-object-test", vec![forge.public_key()], 1)
            .await
            .expect("create repo");
        let repo_id = identity.repo_id();

        // Create a blob using Forge's native API
        let content = b"Export test content".to_vec();
        let blob_hash = forge.git.store_blob(content.clone()).await.expect("store blob");

        // Create exporter
        let mapping = Arc::new(HashMappingStore::new(forge.kv().clone()));
        let secret_key = iroh::SecretKey::generate(&mut rand::rng());
        let hlc = HLC::default();

        let exporter = GitExporter::new(
            Arc::clone(&mapping),
            forge.git.blobs().clone(),
            Arc::new(forge.refs.clone()),
            secret_key,
            hlc,
        );

        // Export the blob
        let exported = exporter.export_object(&repo_id, blob_hash).await.expect("export object");

        assert_eq!(exported.object_type, GitObjectType::Blob);
        assert_eq!(exported.content, content);

        // Verify the exported SHA-1 matches expected
        let expected_sha1 = compute_sha1("blob", &content);
        assert_eq!(exported.sha1, expected_sha1);
    }

    #[tokio::test]
    async fn test_git_exporter_commit_dag() {
        let forge = create_test_forge_node().await;
        let identity = forge
            .create_repo("exporter-dag-test", vec![forge.public_key()], 1)
            .await
            .expect("create repo");
        let repo_id = identity.repo_id();

        // Create a commit chain
        let content1 = b"File v1".to_vec();
        let blob1 = forge.git.store_blob(content1).await.expect("store blob1");
        let entries1 = vec![TreeEntry {
            mode: 0o100644,
            name: "file.txt".to_string(),
            hash: *blob1.as_bytes(),
        }];
        let tree1 = forge.git.create_tree(&entries1).await.expect("create tree1");
        let commit1 = forge.git.commit(tree1, vec![], "First commit").await.expect("create commit1");

        let content2 = b"File v2".to_vec();
        let blob2 = forge.git.store_blob(content2).await.expect("store blob2");
        let entries2 = vec![TreeEntry {
            mode: 0o100644,
            name: "file.txt".to_string(),
            hash: *blob2.as_bytes(),
        }];
        let tree2 = forge.git.create_tree(&entries2).await.expect("create tree2");
        let commit2 = forge.git.commit(tree2, vec![commit1], "Second commit").await.expect("create commit2");

        // Create exporter
        let mapping = Arc::new(HashMappingStore::new(forge.kv().clone()));
        let secret_key = iroh::SecretKey::generate(&mut rand::rng());
        let hlc = HLC::default();

        let exporter = GitExporter::new(
            Arc::clone(&mapping),
            forge.git.blobs().clone(),
            Arc::new(forge.refs.clone()),
            secret_key,
            hlc,
        );

        // Export the entire DAG from commit2
        let result = exporter
            .export_commit_dag(&repo_id, commit2, &HashSet::new())
            .await
            .expect("export dag");

        // Should have: 2 blobs + 2 trees + 2 commits = 6 objects
        assert_eq!(result.objects.len(), 6, "should export 6 objects (2 blobs, 2 trees, 2 commits)");

        // Verify object types
        let blob_count = result.objects.iter().filter(|o| o.object_type == GitObjectType::Blob).count();
        let tree_count = result.objects.iter().filter(|o| o.object_type == GitObjectType::Tree).count();
        let commit_count = result.objects.iter().filter(|o| o.object_type == GitObjectType::Commit).count();

        assert_eq!(blob_count, 2, "should have 2 blobs");
        assert_eq!(tree_count, 2, "should have 2 trees");
        assert_eq!(commit_count, 2, "should have 2 commits");
    }

    #[tokio::test]
    async fn test_git_exporter_incremental() {
        let forge = create_test_forge_node().await;
        let identity = forge
            .create_repo("exporter-incremental-test", vec![forge.public_key()], 1)
            .await
            .expect("create repo");
        let repo_id = identity.repo_id();

        // Create initial commit
        let content1 = b"Initial".to_vec();
        let blob1 = forge.git.store_blob(content1).await.expect("store blob1");
        let entries1 = vec![TreeEntry {
            mode: 0o100644,
            name: "file.txt".to_string(),
            hash: *blob1.as_bytes(),
        }];
        let tree1 = forge.git.create_tree(&entries1).await.expect("create tree1");
        let commit1 = forge.git.commit(tree1, vec![], "Initial").await.expect("create commit1");

        // Create second commit
        let content2 = b"Updated".to_vec();
        let blob2 = forge.git.store_blob(content2).await.expect("store blob2");
        let entries2 = vec![TreeEntry {
            mode: 0o100644,
            name: "file.txt".to_string(),
            hash: *blob2.as_bytes(),
        }];
        let tree2 = forge.git.create_tree(&entries2).await.expect("create tree2");
        let commit2 = forge.git.commit(tree2, vec![commit1], "Updated").await.expect("create commit2");

        // Create exporter
        let mapping = Arc::new(HashMappingStore::new(forge.kv().clone()));
        let secret_key = iroh::SecretKey::generate(&mut rand::rng());
        let hlc = HLC::default();

        let exporter = GitExporter::new(
            Arc::clone(&mapping),
            forge.git.blobs().clone(),
            Arc::new(forge.refs.clone()),
            secret_key,
            hlc,
        );

        // First export full DAG
        let full_result = exporter
            .export_commit_dag(&repo_id, commit2, &HashSet::new())
            .await
            .expect("full export");
        assert_eq!(full_result.objects.len(), 6);

        // Get SHA-1 of commit1 for incremental fetch
        let commit1_sha1 = exporter.get_sha1(&repo_id, &commit1).await.expect("get sha1").expect("should exist");

        // Build "have" set with commit1's objects
        let mut known: HashSet<Sha1Hash> = HashSet::new();
        known.insert(commit1_sha1);

        // Export incrementally - should only get objects not in "have" set
        let incremental = exporter
            .export_commit_dag(&repo_id, commit2, &known)
            .await
            .expect("incremental export");

        // Should skip commit1 since it's known
        assert!(
            incremental.objects.len() < full_result.objects.len(),
            "incremental should have fewer objects"
        );
        assert!(incremental.objects_skipped > 0, "should have skipped some objects");
    }

    // -------------------------------------------------------------------------
    // Roundtrip Tests (Import → Export)
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_roundtrip_blob() {
        let forge = create_test_forge_node().await;
        let identity = forge
            .create_repo("roundtrip-blob-test", vec![forge.public_key()], 1)
            .await
            .expect("create repo");
        let repo_id = identity.repo_id();

        let mapping = Arc::new(HashMappingStore::new(forge.kv().clone()));
        let secret_key = iroh::SecretKey::generate(&mut rand::rng());

        let importer = GitImporter::new(
            Arc::clone(&mapping),
            forge.git.blobs().clone(),
            Arc::new(forge.refs.clone()),
            secret_key.clone(),
            HLC::default(),
        );
        let exporter = GitExporter::new(
            Arc::clone(&mapping),
            forge.git.blobs().clone(),
            Arc::new(forge.refs.clone()),
            secret_key,
            HLC::default(),
        );

        // Import a git blob
        let original_content = b"Roundtrip test content";
        let original_sha1 = compute_sha1("blob", original_content);

        let import_result = importer
            .import_object_raw(&repo_id, original_sha1, "blob", original_content)
            .await
            .expect("import");

        // Get the BLAKE3 hash
        let blake3 = importer
            .get_blake3(&repo_id, &original_sha1)
            .await
            .expect("get blake3")
            .expect("should exist");
        assert_eq!(blake3, import_result.blake3);

        // Export back
        let exported = exporter.export_object(&repo_id, blake3).await.expect("export");

        // Verify roundtrip
        assert_eq!(exported.sha1, original_sha1, "SHA-1 should match after roundtrip");
        assert_eq!(exported.content, original_content, "content should match after roundtrip");
        assert_eq!(exported.object_type, GitObjectType::Blob, "type should be blob");
    }

    #[tokio::test]
    async fn test_roundtrip_multiple_types() {
        let forge = create_test_forge_node().await;
        let identity = forge
            .create_repo("roundtrip-multi-test", vec![forge.public_key()], 1)
            .await
            .expect("create repo");
        let repo_id = identity.repo_id();

        let mapping = Arc::new(HashMappingStore::new(forge.kv().clone()));
        let secret_key = iroh::SecretKey::generate(&mut rand::rng());

        let importer = GitImporter::new(
            Arc::clone(&mapping),
            forge.git.blobs().clone(),
            Arc::new(forge.refs.clone()),
            secret_key.clone(),
            HLC::default(),
        );

        // Import multiple blobs
        let files = vec![
            ("README.md", b"# Project\n".as_slice()),
            ("main.rs", b"fn main() {}\n".as_slice()),
            ("Cargo.toml", b"[package]\nname = \"test\"\n".as_slice()),
        ];

        let mut imported_hashes = Vec::new();
        for (name, content) in &files {
            let sha1 = compute_sha1("blob", content);
            let result = importer
                .import_object_raw(&repo_id, sha1, "blob", content)
                .await
                .expect(&format!("import {}", name));
            imported_hashes.push((sha1, result.blake3));
        }

        // Verify all mappings are bidirectional
        for (sha1, blake3) in &imported_hashes {
            let retrieved_blake3 = mapping.get_blake3(&repo_id, sha1).await.expect("get blake3").expect("exists");
            assert_eq!(retrieved_blake3.0, *blake3);

            let (retrieved_sha1, _) = mapping.get_sha1(&repo_id, blake3).await.expect("get sha1").expect("exists");
            assert_eq!(retrieved_sha1, *sha1);
        }
    }

    // -------------------------------------------------------------------------
    // Ref Update Tests via Importer
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_importer_update_ref() {
        let forge = create_test_forge_node().await;
        let identity = forge
            .create_repo("importer-ref-test", vec![forge.public_key()], 1)
            .await
            .expect("create repo");
        let repo_id = identity.repo_id();

        let mapping = Arc::new(HashMappingStore::new(forge.kv().clone()));
        let secret_key = iroh::SecretKey::generate(&mut rand::rng());
        let hlc = HLC::default();

        let importer = GitImporter::new(
            Arc::clone(&mapping),
            forge.git.blobs().clone(),
            Arc::new(forge.refs.clone()),
            secret_key,
            hlc,
        );

        // Create a blob and get its SHA-1
        let content = b"ref test content";
        let sha1 = compute_sha1("blob", content);
        importer
            .import_object_raw(&repo_id, sha1, "blob", content)
            .await
            .expect("import blob");

        // Update a ref to point to this blob (note: normally refs point to commits)
        let blake3 = importer.update_ref(&repo_id, "heads/test-branch", sha1).await.expect("update ref");

        // Verify ref was set
        let ref_value = forge.refs.get(&repo_id, "heads/test-branch").await.expect("get ref").expect("should exist");
        assert_eq!(ref_value, blake3);
    }
}

// ============================================================================
// Comprehensive Hosting Scenario Test
// ============================================================================

#[tokio::test]
async fn test_complete_hosting_scenario() {
    let forge = create_test_forge_node().await;

    // === Phase 1: Project Setup ===
    let identity = forge
        .create_repo("awesome-project", vec![forge.public_key()], 1)
        .await
        .expect("create repository");
    let repo_id = identity.repo_id();

    // === Phase 2: Initial Commit ===

    // Create project files
    let readme = b"# Awesome Project\n\nA demonstration of Aspen Forge hosting.".to_vec();
    let readme_hash = forge.git.store_blob(readme).await.expect("store readme");

    let license = b"MIT License\n\nCopyright (c) 2025".to_vec();
    let license_hash = forge.git.store_blob(license).await.expect("store license");

    let main_rs = b"fn main() {\n    println!(\"Hello, Forge!\");\n}".to_vec();
    let main_hash = forge.git.store_blob(main_rs).await.expect("store main.rs");

    let cargo = b"[package]\nname = \"awesome\"\nversion = \"0.1.0\"\nedition = \"2024\"".to_vec();
    let cargo_hash = forge.git.store_blob(cargo).await.expect("store Cargo.toml");

    // Create src/ tree
    let src_entries = vec![TreeEntry {
        mode: 0o100644,
        name: "main.rs".to_string(),
        hash: *main_hash.as_bytes(),
    }];
    let src_tree = forge.git.create_tree(&src_entries).await.expect("create src tree");

    // Create root tree
    let root_entries = vec![
        TreeEntry {
            mode: 0o100644,
            name: "README.md".to_string(),
            hash: *readme_hash.as_bytes(),
        },
        TreeEntry {
            mode: 0o100644,
            name: "LICENSE".to_string(),
            hash: *license_hash.as_bytes(),
        },
        TreeEntry {
            mode: 0o100644,
            name: "Cargo.toml".to_string(),
            hash: *cargo_hash.as_bytes(),
        },
        TreeEntry {
            mode: 0o040000,
            name: "src".to_string(),
            hash: *src_tree.as_bytes(),
        },
    ];
    let root_tree = forge.git.create_tree(&root_entries).await.expect("create root tree");

    // Initial commit
    let initial_commit = forge
        .git
        .commit(root_tree, vec![], "Initial commit: project setup")
        .await
        .expect("initial commit");

    // Set up main branch
    forge.refs.set(&repo_id, "heads/main", initial_commit).await.expect("set main branch");

    // === Phase 3: Create Issue ===
    let issue_id = forge
        .cobs
        .create_issue(&repo_id, "Add documentation", "We need to add more documentation to the README.", vec!["enhancement".to_string(), "documentation".to_string()])
        .await
        .expect("create issue");

    // === Phase 4: Feature Branch and Patch ===

    // Update README with more content
    let new_readme = b"# Awesome Project\n\nA demonstration of Aspen Forge hosting.\n\n## Features\n\n- Distributed git hosting\n- Issue tracking\n- Patch management".to_vec();
    let new_readme_hash = forge.git.store_blob(new_readme).await.expect("store new readme");

    // Create new tree with updated README
    let feature_entries = vec![
        TreeEntry {
            mode: 0o100644,
            name: "README.md".to_string(),
            hash: *new_readme_hash.as_bytes(),
        },
        TreeEntry {
            mode: 0o100644,
            name: "LICENSE".to_string(),
            hash: *license_hash.as_bytes(),
        },
        TreeEntry {
            mode: 0o100644,
            name: "Cargo.toml".to_string(),
            hash: *cargo_hash.as_bytes(),
        },
        TreeEntry {
            mode: 0o040000,
            name: "src".to_string(),
            hash: *src_tree.as_bytes(),
        },
    ];
    let feature_tree = forge.git.create_tree(&feature_entries).await.expect("create feature tree");

    // Feature commit
    let feature_commit = forge
        .git
        .commit(feature_tree, vec![initial_commit], "Add documentation to README")
        .await
        .expect("feature commit");

    // Create feature branch
    forge.refs.set(&repo_id, "heads/add-docs", feature_commit).await.expect("set feature branch");

    // Create patch
    let patch_id = forge
        .cobs
        .create_patch(
            &repo_id,
            "Add documentation",
            "Addresses #1 - adds feature documentation to README.\n\nCloses: issue_id",
            initial_commit,
            feature_commit,
        )
        .await
        .expect("create patch");

    // === Phase 5: Review and Merge ===

    // Add review comment
    forge
        .cobs
        .add_comment(&repo_id, &issue_id, "Patch submitted: see patch_id")
        .await
        .expect("comment on issue");

    // Approve patch
    forge
        .cobs
        .approve_patch(&repo_id, &patch_id, feature_commit, Some("Looks good! Documentation is helpful.".to_string()))
        .await
        .expect("approve patch");

    // Merge (fast-forward in this case)
    forge.cobs.merge_patch(&repo_id, &patch_id, feature_commit).await.expect("merge patch");

    // Update main branch
    forge.refs.set(&repo_id, "heads/main", feature_commit).await.expect("fast-forward main");

    // Close the issue
    forge
        .cobs
        .close_issue(&repo_id, &issue_id, Some("completed".to_string()))
        .await
        .expect("close issue");

    // === Phase 6: Create Release Tag ===
    forge.refs.set(&repo_id, "tags/v0.1.0", feature_commit).await.expect("create tag");

    // === Verification ===

    // Verify branches (list_branches strips "heads/" prefix)
    let branches = forge.refs.list_branches(&repo_id).await.expect("list branches");
    assert!(branches.iter().any(|(name, _)| name == "main"));
    assert!(branches.iter().any(|(name, _)| name == "add-docs"));

    // Verify tags (list_tags strips "tags/" prefix)
    let tags = forge.refs.list_tags(&repo_id).await.expect("list tags");
    assert!(tags.iter().any(|(name, _)| name == "v0.1.0"));

    // Verify main points to feature commit
    let main_ref = forge.refs.get(&repo_id, "heads/main").await.expect("get main").expect("exists");
    assert_eq!(main_ref, feature_commit);

    // Verify issue is closed
    let final_issue = forge.cobs.resolve_issue(&repo_id, &issue_id).await.expect("final issue state");
    assert!(!final_issue.state.is_open());

    // Verify patch is merged
    let final_patch = forge.cobs.resolve_patch(&repo_id, &patch_id).await.expect("final patch state");
    assert!(matches!(final_patch.state, aspen::forge::cob::PatchState::Merged { .. }));

    // Verify commit history
    let head_commit = forge.git.get_commit(&feature_commit).await.expect("get head commit");
    assert_eq!(head_commit.message, "Add documentation to README");
    assert_eq!(head_commit.parents.len(), 1);

    let parent_commit = forge
        .git
        .get_commit(&blake3::Hash::from_bytes(head_commit.parents[0]))
        .await
        .expect("get parent");
    assert_eq!(parent_commit.message, "Initial commit: project setup");
}

// ============================================================================
// Edge Cases and Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_get_nonexistent_repo() {
    let forge = create_test_forge_node().await;
    let fake_id = RepoId::from_hash(blake3::hash(b"nonexistent"));

    let result = forge.get_repo(&fake_id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_nonexistent_ref() {
    let forge = create_test_forge_node().await;

    let identity = forge
        .create_repo("nonexistent-ref-test", vec![forge.public_key()], 1)
        .await
        .expect("create repo");

    let result = forge.refs.get(&identity.repo_id(), "heads/nonexistent").await.expect("get result");
    assert!(result.is_none());
}

#[tokio::test]
async fn test_get_nonexistent_blob() {
    let forge = create_test_forge_node().await;
    let fake_hash = blake3::hash(b"nonexistent");

    let result = forge.git.get_blob(&fake_hash).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_empty_tree() {
    let forge = create_test_forge_node().await;

    let entries: Vec<TreeEntry> = vec![];
    let tree_hash = forge.git.create_tree(&entries).await.expect("create empty tree");

    let tree = forge.git.get_tree(&tree_hash).await.expect("get empty tree");
    assert!(tree.entries.is_empty());
}

#[tokio::test]
async fn test_large_blob() {
    let forge = create_test_forge_node().await;

    // Create a 1MB blob
    let content: Vec<u8> = (0..1_000_000).map(|i| (i % 256) as u8).collect();
    let hash = forge.git.store_blob(content.clone()).await.expect("store large blob");

    let retrieved = forge.git.get_blob(&hash).await.expect("get large blob");
    assert_eq!(retrieved.len(), 1_000_000);
    assert_eq!(retrieved, content);
}
