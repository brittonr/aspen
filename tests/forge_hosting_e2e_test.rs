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

use aspen::testing::DeterministicKeyValueStore;
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
    let identity = forge.create_repo("test-repo", vec![forge.public_key()], 1).await.expect("should create repository");

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
        let hash = forge.git.store_blob(content.clone()).await.unwrap_or_else(|_| panic!("should store {}", name));
        hashes.push(hash);
    }

    // Verify all blobs are retrievable
    for (i, (name, content)) in files.iter().enumerate() {
        let retrieved = forge.git.get_blob(&hashes[i]).await.unwrap_or_else(|_| panic!("should get {}", name));
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

    // Create tree
    let entries = vec![TreeEntry {
        mode: 0o100644,
        name: "README.md".to_string(),
        hash: *blob_hash.as_bytes(),
    }];
    let tree_hash = forge.git.create_tree(&entries).await.expect("create tree");

    // Create commit (no parents for initial)
    let commit_hash = forge.git.commit(tree_hash, vec![], "Initial commit").await.expect("create commit");

    // Verify commit
    let commit = forge.git.get_commit(&commit_hash).await.expect("get commit");
    assert_eq!(commit.message, "Initial commit");
    assert!(commit.parents.is_empty(), "initial commit should have no parents");
    assert_eq!(blake3::Hash::from_bytes(commit.tree), tree_hash);
}

#[tokio::test]
async fn test_create_commit_chain() {
    let forge = create_test_forge_node().await;

    // First commit
    let content1 = b"Version 1".to_vec();
    let blob1 = forge.git.store_blob(content1).await.expect("store blob 1");
    let tree1 = forge
        .git
        .create_tree(&[TreeEntry {
            mode: 0o100644,
            name: "file.txt".to_string(),
            hash: *blob1.as_bytes(),
        }])
        .await
        .expect("create tree 1");
    let commit1 = forge.git.commit(tree1, vec![], "First commit").await.expect("create commit 1");

    // Second commit (child of first)
    let content2 = b"Version 2".to_vec();
    let blob2 = forge.git.store_blob(content2).await.expect("store blob 2");
    let tree2 = forge
        .git
        .create_tree(&[TreeEntry {
            mode: 0o100644,
            name: "file.txt".to_string(),
            hash: *blob2.as_bytes(),
        }])
        .await
        .expect("create tree 2");
    let commit2 = forge.git.commit(tree2, vec![commit1], "Second commit").await.expect("create commit 2");

    // Verify chain
    let retrieved_commit2 = forge.git.get_commit(&commit2).await.expect("get commit 2");
    assert_eq!(retrieved_commit2.parents.len(), 1);
    assert_eq!(blake3::Hash::from_bytes(retrieved_commit2.parents[0]), commit1);
}

// ============================================================================
// Ref Management Tests
// ============================================================================

#[tokio::test]
async fn test_set_and_get_ref() {
    let forge = create_test_forge_node().await;

    // Create a repository first
    let identity = forge.create_repo("ref-test", vec![forge.public_key()], 1).await.expect("create repo");
    let repo_id = identity.repo_id();

    // Create a commit
    let content = b"Content".to_vec();
    let blob = forge.git.store_blob(content).await.expect("store blob");
    let tree = forge
        .git
        .create_tree(&[TreeEntry {
            mode: 0o100644,
            name: "file.txt".to_string(),
            hash: *blob.as_bytes(),
        }])
        .await
        .expect("create tree");
    let commit = forge.git.commit(tree, vec![], "Initial").await.expect("create commit");

    // Set the ref
    forge.refs.set(&repo_id, "heads/main", commit).await.expect("set ref");

    // Get the ref
    let retrieved = forge.refs.get(&repo_id, "heads/main").await.expect("get ref");
    assert_eq!(retrieved, Some(commit));
}

#[tokio::test]
async fn test_list_branches() {
    let forge = create_test_forge_node().await;

    // Create a repository
    let identity = forge.create_repo("branches-test", vec![forge.public_key()], 1).await.expect("create repo");
    let repo_id = identity.repo_id();

    // Create a commit
    let blob = forge.git.store_blob(b"Content".to_vec()).await.expect("store blob");
    let tree = forge
        .git
        .create_tree(&[TreeEntry {
            mode: 0o100644,
            name: "file.txt".to_string(),
            hash: *blob.as_bytes(),
        }])
        .await
        .expect("create tree");
    let commit = forge.git.commit(tree, vec![], "Initial").await.expect("create commit");

    // Set multiple branches
    forge.refs.set(&repo_id, "heads/main", commit).await.expect("set main");
    forge.refs.set(&repo_id, "heads/develop", commit).await.expect("set develop");
    forge.refs.set(&repo_id, "heads/feature", commit).await.expect("set feature");

    // List branches (note: list_branches strips the "heads/" prefix)
    let branches = forge.refs.list_branches(&repo_id).await.expect("list branches");

    assert_eq!(branches.len(), 3);
    assert!(branches.iter().any(|(name, _)| name == "main"));
    assert!(branches.iter().any(|(name, _)| name == "develop"));
    assert!(branches.iter().any(|(name, _)| name == "feature"));
}

#[tokio::test]
async fn test_delete_ref() {
    let forge = create_test_forge_node().await;

    // Create a repository
    let identity = forge.create_repo("delete-ref-test", vec![forge.public_key()], 1).await.expect("create repo");
    let repo_id = identity.repo_id();

    // Create and set a ref
    let blob = forge.git.store_blob(b"Content".to_vec()).await.expect("store blob");
    let tree = forge
        .git
        .create_tree(&[TreeEntry {
            mode: 0o100644,
            name: "file.txt".to_string(),
            hash: *blob.as_bytes(),
        }])
        .await
        .expect("create tree");
    let commit = forge.git.commit(tree, vec![], "Initial").await.expect("create commit");

    forge.refs.set(&repo_id, "heads/temp", commit).await.expect("set ref");

    // Verify it exists
    let exists = forge.refs.get(&repo_id, "heads/temp").await.expect("get ref");
    assert!(exists.is_some());

    // Delete it
    forge.refs.delete(&repo_id, "heads/temp").await.expect("delete ref");

    // Verify it's gone
    let deleted = forge.refs.get(&repo_id, "heads/temp").await.expect("get deleted ref");
    assert!(deleted.is_none());
}

// Git Bridge tests are in forge_git_bridge_integration_test.rs
// They cover SHA-1 ↔ BLAKE3 translation with the full GitImporter/GitExporter API

// ============================================================================
// Issue and Patch (COB) Tests
// ============================================================================

#[tokio::test]
async fn test_create_issue() {
    let forge = create_test_forge_node().await;

    // Create a repository
    let identity = forge.create_repo("issue-test", vec![forge.public_key()], 1).await.expect("create repo");
    let repo_id = identity.repo_id();

    // Create an issue
    let issue_hash = forge
        .cobs
        .create_issue(&repo_id, "Test Issue", "This is a test issue body", vec!["bug".to_string()])
        .await
        .expect("create issue");

    // List issues
    let issues = forge.cobs.list_issues(&repo_id).await.expect("list issues");
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0], issue_hash);

    // Resolve the issue
    let issue = forge.cobs.resolve_issue(&repo_id, &issue_hash).await.expect("resolve issue");
    assert_eq!(issue.title, "Test Issue");
    assert_eq!(issue.body, "This is a test issue body");
    assert!(issue.state.is_open());
    assert!(issue.labels.contains("bug"));
}

#[tokio::test]
async fn test_issue_comment_and_close() {
    let forge = create_test_forge_node().await;

    let identity = forge.create_repo("issue-lifecycle", vec![forge.public_key()], 1).await.expect("create repo");
    let repo_id = identity.repo_id();

    // Create and comment on issue
    let issue_hash = forge
        .cobs
        .create_issue(&repo_id, "Lifecycle Test", "Testing issue lifecycle", vec![])
        .await
        .expect("create");

    forge.cobs.add_comment(&repo_id, &issue_hash, "This is a comment").await.expect("add comment");

    // Verify comment
    let issue = forge.cobs.resolve_issue(&repo_id, &issue_hash).await.expect("resolve");
    assert_eq!(issue.comments.len(), 1);
    assert_eq!(issue.comments[0].body, "This is a comment");

    // Close the issue
    forge.cobs.close_issue(&repo_id, &issue_hash, Some("completed".to_string())).await.expect("close");

    // Verify closed
    let closed_issue = forge.cobs.resolve_issue(&repo_id, &issue_hash).await.expect("resolve closed");
    assert!(!closed_issue.state.is_open());
}

#[tokio::test]
async fn test_create_patch() {
    let forge = create_test_forge_node().await;

    let identity = forge.create_repo("patch-test", vec![forge.public_key()], 1).await.expect("create repo");
    let repo_id = identity.repo_id();

    // Create commits for base and head
    let blob1 = forge.git.store_blob(b"base content".to_vec()).await.expect("store base");
    let tree1 = forge
        .git
        .create_tree(&[TreeEntry {
            mode: 0o100644,
            name: "file.txt".to_string(),
            hash: *blob1.as_bytes(),
        }])
        .await
        .expect("create base tree");
    let base_commit = forge.git.commit(tree1, vec![], "Base commit").await.expect("base commit");

    let blob2 = forge.git.store_blob(b"head content".to_vec()).await.expect("store head");
    let tree2 = forge
        .git
        .create_tree(&[TreeEntry {
            mode: 0o100644,
            name: "file.txt".to_string(),
            hash: *blob2.as_bytes(),
        }])
        .await
        .expect("create head tree");
    let head_commit = forge.git.commit(tree2, vec![base_commit], "Head commit").await.expect("head commit");

    // Create a patch
    let patch_hash = forge
        .cobs
        .create_patch(&repo_id, "Test Patch", "This patch adds a feature", base_commit, head_commit)
        .await
        .expect("create patch");

    // List patches
    let patches = forge.cobs.list_patches(&repo_id).await.expect("list patches");
    assert_eq!(patches.len(), 1);

    // Resolve patch
    let patch = forge.cobs.resolve_patch(&repo_id, &patch_hash).await.expect("resolve patch");
    assert_eq!(patch.title, "Test Patch");
    assert!(patch.state.is_open());
}

// ============================================================================
// Full Workflow Tests
// ============================================================================

#[tokio::test]
async fn test_complete_git_workflow() {
    let forge = create_test_forge_node().await;

    // 1. Create repository
    let identity = forge.create_repo("workflow-test", vec![forge.public_key()], 1).await.expect("create repo");
    let repo_id = identity.repo_id();

    // 2. Create initial content
    let readme = forge.git.store_blob(b"# My Project\n\nA test project.".to_vec()).await.expect("store readme");
    let main_rs = forge
        .git
        .store_blob(b"fn main() {\n    println!(\"Hello!\");\n}".to_vec())
        .await
        .expect("store main.rs");
    let cargo_toml = forge.git.store_blob(b"[package]\nname = \"test\"".to_vec()).await.expect("store Cargo.toml");

    // 3. Create src/ subtree
    let src_tree = forge
        .git
        .create_tree(&[TreeEntry {
            mode: 0o100644,
            name: "main.rs".to_string(),
            hash: *main_rs.as_bytes(),
        }])
        .await
        .expect("create src tree");

    // 4. Create root tree
    let root_tree = forge
        .git
        .create_tree(&[
            TreeEntry {
                mode: 0o100644,
                name: "README.md".to_string(),
                hash: *readme.as_bytes(),
            },
            TreeEntry {
                mode: 0o100644,
                name: "Cargo.toml".to_string(),
                hash: *cargo_toml.as_bytes(),
            },
            TreeEntry {
                mode: 0o040000,
                name: "src".to_string(),
                hash: *src_tree.as_bytes(),
            },
        ])
        .await
        .expect("create root tree");

    // 5. Create initial commit
    let initial_commit = forge.git.commit(root_tree, vec![], "Initial commit").await.expect("initial commit");

    // 6. Set main branch
    forge.refs.set(&repo_id, "heads/main", initial_commit).await.expect("set main");

    // 7. Create a feature branch with changes
    let updated_main = forge
        .git
        .store_blob(b"fn main() {\n    println!(\"Hello, World!\");\n}".to_vec())
        .await
        .expect("store updated main.rs");

    let updated_src_tree = forge
        .git
        .create_tree(&[TreeEntry {
            mode: 0o100644,
            name: "main.rs".to_string(),
            hash: *updated_main.as_bytes(),
        }])
        .await
        .expect("create updated src tree");

    let updated_root_tree = forge
        .git
        .create_tree(&[
            TreeEntry {
                mode: 0o100644,
                name: "README.md".to_string(),
                hash: *readme.as_bytes(),
            },
            TreeEntry {
                mode: 0o100644,
                name: "Cargo.toml".to_string(),
                hash: *cargo_toml.as_bytes(),
            },
            TreeEntry {
                mode: 0o040000,
                name: "src".to_string(),
                hash: *updated_src_tree.as_bytes(),
            },
        ])
        .await
        .expect("create updated root tree");

    let feature_commit = forge
        .git
        .commit(updated_root_tree, vec![initial_commit], "Update greeting message")
        .await
        .expect("feature commit");

    forge.refs.set(&repo_id, "heads/feature", feature_commit).await.expect("set feature branch");

    // 8. Create a patch for the feature
    let patch_hash = forge
        .cobs
        .create_patch(
            &repo_id,
            "Update greeting",
            "Changes the greeting to be more friendly",
            initial_commit,
            feature_commit,
        )
        .await
        .expect("create patch");

    // 9. Add a review comment
    // Note: Currently patches don't have direct comment support like issues
    // This would be done via approve_patch or similar

    // 10. Verify final state
    let branches = forge.refs.list_branches(&repo_id).await.expect("list branches");
    assert_eq!(branches.len(), 2);

    let patches = forge.cobs.list_patches(&repo_id).await.expect("list patches");
    assert_eq!(patches.len(), 1);
    assert_eq!(patches[0], patch_hash);

    // 11. Simulate merge by updating main to point to feature commit
    forge.refs.set(&repo_id, "heads/main", feature_commit).await.expect("merge to main");

    // 12. Close the patch
    forge.cobs.merge_patch(&repo_id, &patch_hash, feature_commit).await.expect("merge patch");

    // Verify patch is merged
    let merged_patch = forge.cobs.resolve_patch(&repo_id, &patch_hash).await.expect("resolve merged patch");
    assert!(merged_patch.state.is_merged());
}

// ============================================================================
// Large Content Tests
// ============================================================================

#[tokio::test]
async fn test_large_blob() {
    let forge = create_test_forge_node().await;

    // Create a 1MB blob
    let large_content: Vec<u8> = (0..1024 * 1024).map(|i| (i % 256) as u8).collect();
    let hash = forge.git.store_blob(large_content.clone()).await.expect("store large blob");

    // Retrieve and verify
    let retrieved = forge.git.get_blob(&hash).await.expect("get large blob");
    assert_eq!(retrieved.len(), large_content.len());
    assert_eq!(retrieved, large_content);
}

#[tokio::test]
async fn test_many_files_in_tree() {
    let forge = create_test_forge_node().await;

    // Create 100 files
    let mut entries = Vec::new();
    for i in 0..100 {
        let content = format!("File {} content", i);
        let hash = forge.git.store_blob(content.into_bytes()).await.unwrap_or_else(|_| panic!("store file {}", i));
        entries.push(TreeEntry {
            mode: 0o100644,
            name: format!("file_{:03}.txt", i),
            hash: *hash.as_bytes(),
        });
    }

    let tree_hash = forge.git.create_tree(&entries).await.expect("create large tree");

    // Verify tree
    let tree = forge.git.get_tree(&tree_hash).await.expect("get large tree");
    assert_eq!(tree.entries.len(), 100);
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_get_nonexistent_blob() {
    let forge = create_test_forge_node().await;

    // Try to get a blob that doesn't exist
    let fake_hash = blake3::hash(b"nonexistent");
    let result = forge.git.get_blob(&fake_hash).await;

    assert!(result.is_err(), "should fail for nonexistent blob");
}

#[tokio::test]
async fn test_get_nonexistent_ref() {
    let forge = create_test_forge_node().await;

    let identity = forge.create_repo("nonexistent-ref", vec![forge.public_key()], 1).await.expect("create repo");
    let repo_id = identity.repo_id();

    // Try to get a ref that doesn't exist
    let result = forge.refs.get(&repo_id, "heads/nonexistent").await.expect("get should succeed");
    assert!(result.is_none(), "nonexistent ref should return None");
}

#[tokio::test]
async fn test_set_ref_with_nonexistent_repo() {
    let forge = create_test_forge_node().await;

    // Try to set a ref for a repo that doesn't exist
    let fake_repo_id = RepoId::from_hash(blake3::Hash::from_bytes([0u8; 32]));
    let fake_commit = blake3::hash(b"fake");

    // This should still work since refs don't validate repo existence
    // (refs are just key-value pairs)
    let result = forge.refs.set(&fake_repo_id, "heads/main", fake_commit).await;
    assert!(result.is_ok(), "setting ref for any repo_id should work");
}
