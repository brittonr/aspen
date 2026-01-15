//! Git Bridge Integration Tests
//!
//! These tests verify that the Git Bridge can interoperate with standard Git
//! by simulating the full git-remote-aspen workflow:
//!
//! 1. Creating a repository in Forge
//! 2. Using GitImporter to push git objects
//! 3. Using GitExporter to fetch git objects
//! 4. Verifying SHA-1 ↔ BLAKE3 hash translation
//! 5. Testing the complete push/fetch cycle
//!
//! These tests are designed to identify gaps in the implementation when hosting
//! real Git repositories.

#![cfg(feature = "git-bridge")]

use std::collections::HashSet;
use std::sync::Arc;

use aspen::api::DeterministicKeyValueStore;
use aspen::blob::InMemoryBlobStore;
use aspen::forge::ForgeNode;
use aspen::forge::git::bridge::GitExporter;
use aspen::forge::git::bridge::GitImporter;
use aspen::forge::git::bridge::GitObjectType;
use aspen::forge::git::bridge::HashMappingStore;
use aspen::forge::git::bridge::Sha1Hash;
use aspen_core::hlc::create_hlc;
use sha1::Digest;
use sha1::Sha1;

/// Create a test ForgeNode with in-memory storage.
async fn create_test_forge_node() -> ForgeNode<InMemoryBlobStore, DeterministicKeyValueStore> {
    let blobs = Arc::new(InMemoryBlobStore::new());
    let kv = DeterministicKeyValueStore::new();
    let secret_key = iroh::SecretKey::generate(&mut rand::rng());
    ForgeNode::new(blobs, kv, secret_key)
}

/// Compute SHA-1 hash of git object bytes (git format: `<type> <size>\0<content>`).
fn compute_git_sha1(object_type: &str, content: &[u8]) -> Sha1Hash {
    let header = format!("{} {}\0", object_type, content.len());
    let mut hasher = Sha1::new();
    hasher.update(header.as_bytes());
    hasher.update(content);
    let result = hasher.finalize();
    Sha1Hash::from_bytes(result.into())
}

/// Build a git tree entry as raw bytes.
fn make_git_tree_entry(mode: &str, name: &str, sha1: &Sha1Hash) -> Vec<u8> {
    let mut entry = Vec::new();
    entry.extend_from_slice(mode.as_bytes());
    entry.push(b' ');
    entry.extend_from_slice(name.as_bytes());
    entry.push(0);
    entry.extend_from_slice(sha1.as_bytes());
    entry
}

/// Build a git commit object as raw bytes.
///
/// NOTE: Git commit messages should end with a newline for proper roundtrip.
/// The export path normalizes messages to end with newline, so we do the same
/// here to ensure SHA-1 roundtrip works correctly.
fn make_git_commit(tree_sha1: &Sha1Hash, parent_sha1s: &[Sha1Hash], message: &str) -> Vec<u8> {
    let mut content = String::new();
    content.push_str(&format!("tree {}\n", tree_sha1.to_hex()));
    for parent in parent_sha1s {
        content.push_str(&format!("parent {}\n", parent.to_hex()));
    }
    content.push_str("author Test Author <test@example.com> 1700000000 +0000\n");
    content.push_str("committer Test Author <test@example.com> 1700000000 +0000\n");
    content.push('\n');
    content.push_str(message);
    // Git normalizes commit messages to end with newline
    if !message.ends_with('\n') {
        content.push('\n');
    }
    content.into_bytes()
}

// ============================================================================
// Basic Git Object Import/Export Tests
// ============================================================================

#[tokio::test]
async fn test_import_export_single_blob() {
    let forge = create_test_forge_node().await;
    let identity = forge
        .create_repo("import-export-blob-test", vec![forge.public_key()], 1)
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
        create_hlc("test-importer"),
    );
    let exporter = GitExporter::new(
        Arc::clone(&mapping),
        forge.git.blobs().clone(),
        Arc::new(forge.refs.clone()),
        secret_key,
        create_hlc("test-exporter"),
    );

    // Create a git blob
    let blob_content = b"Hello, World!";
    let sha1 = compute_git_sha1("blob", blob_content);

    // Import the blob
    let import_result = importer.import_object_raw(&repo_id, sha1, "blob", blob_content).await.expect("import blob");
    assert!(!import_result.already_existed);

    // Export the blob
    let blake3 = import_result.blake3;
    let exported = exporter.export_object(&repo_id, blake3).await.expect("export object");

    // Verify roundtrip
    assert_eq!(exported.sha1, sha1);
    assert_eq!(exported.content, blob_content);
    assert_eq!(exported.object_type, GitObjectType::Blob);
}

#[tokio::test]
async fn test_import_export_tree() {
    let forge = create_test_forge_node().await;
    let identity = forge
        .create_repo("import-export-tree-test", vec![forge.public_key()], 1)
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
        create_hlc("test-importer"),
    );
    let exporter = GitExporter::new(
        Arc::clone(&mapping),
        forge.git.blobs().clone(),
        Arc::new(forge.refs.clone()),
        secret_key,
        create_hlc("test-exporter"),
    );

    // Create blob first
    let blob_content = b"File content";
    let blob_sha1 = compute_git_sha1("blob", blob_content);
    importer.import_object_raw(&repo_id, blob_sha1, "blob", blob_content).await.expect("import blob");

    // Create tree with the blob
    let tree_content = make_git_tree_entry("100644", "file.txt", &blob_sha1);
    let tree_sha1 = compute_git_sha1("tree", &tree_content);
    let tree_result =
        importer.import_object_raw(&repo_id, tree_sha1, "tree", &tree_content).await.expect("import tree");

    // Export tree
    let exported = exporter.export_object(&repo_id, tree_result.blake3).await.expect("export tree");

    assert_eq!(exported.sha1, tree_sha1);
    assert_eq!(exported.object_type, GitObjectType::Tree);
}

#[tokio::test]
async fn test_import_export_commit() {
    let forge = create_test_forge_node().await;
    let identity = forge
        .create_repo("import-export-commit-test", vec![forge.public_key()], 1)
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
        create_hlc("test-importer"),
    );
    let exporter = GitExporter::new(
        Arc::clone(&mapping),
        forge.git.blobs().clone(),
        Arc::new(forge.refs.clone()),
        secret_key,
        create_hlc("test-exporter"),
    );

    // Import blob
    let blob_content = b"Initial content";
    let blob_sha1 = compute_git_sha1("blob", blob_content);
    importer.import_object_raw(&repo_id, blob_sha1, "blob", blob_content).await.expect("import blob");

    // Import tree
    let tree_content = make_git_tree_entry("100644", "README.md", &blob_sha1);
    let tree_sha1 = compute_git_sha1("tree", &tree_content);
    importer.import_object_raw(&repo_id, tree_sha1, "tree", &tree_content).await.expect("import tree");

    // Import commit
    let commit_content = make_git_commit(&tree_sha1, &[], "Initial commit");
    let commit_sha1 = compute_git_sha1("commit", &commit_content);
    let commit_result = importer
        .import_object_raw(&repo_id, commit_sha1, "commit", &commit_content)
        .await
        .expect("import commit");

    // Export commit
    let exported = exporter.export_object(&repo_id, commit_result.blake3).await.expect("export commit");

    assert_eq!(exported.sha1, commit_sha1);
    assert_eq!(exported.object_type, GitObjectType::Commit);
}

// ============================================================================
// Full DAG Import/Export Tests
// ============================================================================

#[tokio::test]
async fn test_import_export_commit_chain() {
    let forge = create_test_forge_node().await;
    let identity = forge.create_repo("commit-chain-test", vec![forge.public_key()], 1).await.expect("create repo");
    let repo_id = identity.repo_id();

    let mapping = Arc::new(HashMappingStore::new(forge.kv().clone()));
    let secret_key = iroh::SecretKey::generate(&mut rand::rng());

    let importer = GitImporter::new(
        Arc::clone(&mapping),
        forge.git.blobs().clone(),
        Arc::new(forge.refs.clone()),
        secret_key.clone(),
        create_hlc("test-importer"),
    );
    let exporter = GitExporter::new(
        Arc::clone(&mapping),
        forge.git.blobs().clone(),
        Arc::new(forge.refs.clone()),
        secret_key,
        create_hlc("test-exporter"),
    );

    // Create first commit
    let blob1_content = b"Version 1";
    let blob1_sha1 = compute_git_sha1("blob", blob1_content);
    importer
        .import_object_raw(&repo_id, blob1_sha1, "blob", blob1_content)
        .await
        .expect("import blob 1");

    let tree1_content = make_git_tree_entry("100644", "file.txt", &blob1_sha1);
    let tree1_sha1 = compute_git_sha1("tree", &tree1_content);
    importer
        .import_object_raw(&repo_id, tree1_sha1, "tree", &tree1_content)
        .await
        .expect("import tree 1");

    let commit1_content = make_git_commit(&tree1_sha1, &[], "First commit");
    let commit1_sha1 = compute_git_sha1("commit", &commit1_content);
    let _commit1_result = importer
        .import_object_raw(&repo_id, commit1_sha1, "commit", &commit1_content)
        .await
        .expect("import commit 1");

    // Create second commit (child of first)
    let blob2_content = b"Version 2";
    let blob2_sha1 = compute_git_sha1("blob", blob2_content);
    importer
        .import_object_raw(&repo_id, blob2_sha1, "blob", blob2_content)
        .await
        .expect("import blob 2");

    let tree2_content = make_git_tree_entry("100644", "file.txt", &blob2_sha1);
    let tree2_sha1 = compute_git_sha1("tree", &tree2_content);
    importer
        .import_object_raw(&repo_id, tree2_sha1, "tree", &tree2_content)
        .await
        .expect("import tree 2");

    let commit2_content = make_git_commit(&tree2_sha1, &[commit1_sha1], "Second commit");
    let commit2_sha1 = compute_git_sha1("commit", &commit2_content);
    let commit2_result = importer
        .import_object_raw(&repo_id, commit2_sha1, "commit", &commit2_content)
        .await
        .expect("import commit 2");

    // Export the full DAG from commit2
    let have_set: HashSet<Sha1Hash> = HashSet::new();
    let dag_result = exporter.export_commit_dag(&repo_id, commit2_result.blake3, &have_set).await.expect("export dag");

    // Should have: 2 blobs + 2 trees + 2 commits = 6 objects
    assert_eq!(dag_result.objects.len(), 6, "DAG should have 6 objects");

    let blob_count = dag_result.objects.iter().filter(|o| o.object_type == GitObjectType::Blob).count();
    let tree_count = dag_result.objects.iter().filter(|o| o.object_type == GitObjectType::Tree).count();
    let commit_count = dag_result.objects.iter().filter(|o| o.object_type == GitObjectType::Commit).count();

    assert_eq!(blob_count, 2, "DAG should have 2 blobs");
    assert_eq!(tree_count, 2, "DAG should have 2 trees");
    assert_eq!(commit_count, 2, "DAG should have 2 commits");
}

#[tokio::test]
async fn test_incremental_fetch_with_have_set() {
    let forge = create_test_forge_node().await;
    let identity = forge.create_repo("incremental-fetch-test", vec![forge.public_key()], 1).await.expect("create repo");
    let repo_id = identity.repo_id();

    let mapping = Arc::new(HashMappingStore::new(forge.kv().clone()));
    let secret_key = iroh::SecretKey::generate(&mut rand::rng());

    let importer = GitImporter::new(
        Arc::clone(&mapping),
        forge.git.blobs().clone(),
        Arc::new(forge.refs.clone()),
        secret_key.clone(),
        create_hlc("test-importer"),
    );
    let exporter = GitExporter::new(
        Arc::clone(&mapping),
        forge.git.blobs().clone(),
        Arc::new(forge.refs.clone()),
        secret_key,
        create_hlc("test-exporter"),
    );

    // Create 3 commits in a chain
    let mut commits: Vec<(Sha1Hash, blake3::Hash)> = Vec::new();

    for i in 0..3 {
        let blob_content = format!("Version {}", i).into_bytes();
        let blob_sha1 = compute_git_sha1("blob", &blob_content);
        importer.import_object_raw(&repo_id, blob_sha1, "blob", &blob_content).await.expect("import blob");

        let tree_content = make_git_tree_entry("100644", "file.txt", &blob_sha1);
        let tree_sha1 = compute_git_sha1("tree", &tree_content);
        importer.import_object_raw(&repo_id, tree_sha1, "tree", &tree_content).await.expect("import tree");

        let parent_sha1s: Vec<Sha1Hash> = if i == 0 { vec![] } else { vec![commits[i - 1].0] };
        let commit_content = make_git_commit(&tree_sha1, &parent_sha1s, &format!("Commit {}", i));
        let commit_sha1 = compute_git_sha1("commit", &commit_content);
        let result = importer
            .import_object_raw(&repo_id, commit_sha1, "commit", &commit_content)
            .await
            .expect("import commit");
        commits.push((commit_sha1, result.blake3));
    }

    // First, fetch full DAG (should get 9 objects: 3 blobs + 3 trees + 3 commits)
    let have_none: HashSet<Sha1Hash> = HashSet::new();
    let full_result = exporter.export_commit_dag(&repo_id, commits[2].1, &have_none).await.expect("full export");
    assert_eq!(full_result.objects.len(), 9, "Full DAG should have 9 objects");

    // Now fetch incrementally with commit0 as "have"
    let mut have_commit0: HashSet<Sha1Hash> = HashSet::new();
    have_commit0.insert(commits[0].0);

    let incremental_result =
        exporter.export_commit_dag(&repo_id, commits[2].1, &have_commit0).await.expect("incremental export");

    // Should skip commit0 and its objects
    assert!(
        incremental_result.objects.len() < full_result.objects.len(),
        "Incremental fetch should have fewer objects"
    );
    assert!(incremental_result.objects_skipped > 0, "Should have skipped some objects");
}

// ============================================================================
// Ref Management Tests (via Git Bridge)
// ============================================================================

#[tokio::test]
async fn test_ref_update_via_importer() {
    let forge = create_test_forge_node().await;
    let identity = forge.create_repo("ref-update-test", vec![forge.public_key()], 1).await.expect("create repo");
    let repo_id = identity.repo_id();

    let mapping = Arc::new(HashMappingStore::new(forge.kv().clone()));
    let secret_key = iroh::SecretKey::generate(&mut rand::rng());

    let importer = GitImporter::new(
        Arc::clone(&mapping),
        forge.git.blobs().clone(),
        Arc::new(forge.refs.clone()),
        secret_key.clone(),
        create_hlc("test-importer"),
    );
    let exporter = GitExporter::new(
        Arc::clone(&mapping),
        forge.git.blobs().clone(),
        Arc::new(forge.refs.clone()),
        secret_key,
        create_hlc("test-exporter"),
    );

    // Import a commit
    let blob_content = b"Test content";
    let blob_sha1 = compute_git_sha1("blob", blob_content);
    importer.import_object_raw(&repo_id, blob_sha1, "blob", blob_content).await.expect("import blob");

    let tree_content = make_git_tree_entry("100644", "test.txt", &blob_sha1);
    let tree_sha1 = compute_git_sha1("tree", &tree_content);
    importer.import_object_raw(&repo_id, tree_sha1, "tree", &tree_content).await.expect("import tree");

    let commit_content = make_git_commit(&tree_sha1, &[], "Test commit");
    let commit_sha1 = compute_git_sha1("commit", &commit_content);
    let _commit_result = importer
        .import_object_raw(&repo_id, commit_sha1, "commit", &commit_content)
        .await
        .expect("import commit");

    // Update ref via importer
    let blake3_hash = importer.update_ref(&repo_id, "heads/main", commit_sha1).await.expect("update ref");

    // Verify ref via RefStore
    let ref_value = forge.refs.get(&repo_id, "heads/main").await.expect("get ref").expect("ref should exist");
    assert_eq!(ref_value, blake3_hash);

    // Verify ref listing via exporter
    let ref_list = exporter.list_refs(&repo_id).await.expect("list refs");
    assert!(!ref_list.is_empty(), "Should have at least one ref");

    let main_ref = ref_list.iter().find(|(name, _)| name.contains("main")).expect("Should have main ref");
    assert!(main_ref.1.is_some(), "main ref should have SHA-1 mapping");
}

#[tokio::test]
async fn test_list_refs_with_multiple_branches() {
    let forge = create_test_forge_node().await;
    let identity = forge.create_repo("multi-branch-test", vec![forge.public_key()], 1).await.expect("create repo");
    let repo_id = identity.repo_id();

    let mapping = Arc::new(HashMappingStore::new(forge.kv().clone()));
    let secret_key = iroh::SecretKey::generate(&mut rand::rng());

    let importer = GitImporter::new(
        Arc::clone(&mapping),
        forge.git.blobs().clone(),
        Arc::new(forge.refs.clone()),
        secret_key.clone(),
        create_hlc("test-importer"),
    );
    let exporter = GitExporter::new(
        Arc::clone(&mapping),
        forge.git.blobs().clone(),
        Arc::new(forge.refs.clone()),
        secret_key,
        create_hlc("test-exporter"),
    );

    // Import a commit
    let blob_content = b"Base content";
    let blob_sha1 = compute_git_sha1("blob", blob_content);
    importer.import_object_raw(&repo_id, blob_sha1, "blob", blob_content).await.expect("import blob");

    let tree_content = make_git_tree_entry("100644", "base.txt", &blob_sha1);
    let tree_sha1 = compute_git_sha1("tree", &tree_content);
    importer.import_object_raw(&repo_id, tree_sha1, "tree", &tree_content).await.expect("import tree");

    let commit_content = make_git_commit(&tree_sha1, &[], "Base commit");
    let commit_sha1 = compute_git_sha1("commit", &commit_content);
    importer
        .import_object_raw(&repo_id, commit_sha1, "commit", &commit_content)
        .await
        .expect("import commit");

    // Create multiple branches
    importer.update_ref(&repo_id, "heads/main", commit_sha1).await.expect("update main");
    importer.update_ref(&repo_id, "heads/develop", commit_sha1).await.expect("update develop");
    importer.update_ref(&repo_id, "heads/feature/test", commit_sha1).await.expect("update feature");

    // Create a tag
    importer.update_ref(&repo_id, "tags/v1.0.0", commit_sha1).await.expect("update tag");

    // List refs
    let ref_list = exporter.list_refs(&repo_id).await.expect("list refs");

    // Should have 4 refs: main, develop, feature/test, v1.0.0
    assert!(ref_list.len() >= 4, "Should have at least 4 refs");

    // Verify all refs have SHA-1 mappings
    for (name, sha1_opt) in &ref_list {
        assert!(sha1_opt.is_some(), "Ref {} should have SHA-1 mapping", name);
        assert_eq!(sha1_opt.as_ref().unwrap(), &commit_sha1, "Ref {} should point to the commit", name);
    }
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

#[tokio::test]
async fn test_import_idempotency() {
    let forge = create_test_forge_node().await;
    let identity = forge.create_repo("idempotent-import-test", vec![forge.public_key()], 1).await.expect("create repo");
    let repo_id = identity.repo_id();

    let mapping = Arc::new(HashMappingStore::new(forge.kv().clone()));
    let secret_key = iroh::SecretKey::generate(&mut rand::rng());

    let importer = GitImporter::new(
        Arc::clone(&mapping),
        forge.git.blobs().clone(),
        Arc::new(forge.refs.clone()),
        secret_key,
        create_hlc("test-importer"),
    );

    // Import a blob
    let blob_content = b"Idempotent content";
    let sha1 = compute_git_sha1("blob", blob_content);

    // First import
    let result1 = importer.import_object_raw(&repo_id, sha1, "blob", blob_content).await.expect("first import");
    assert!(!result1.already_existed);

    // Second import (should be idempotent)
    let result2 = importer.import_object_raw(&repo_id, sha1, "blob", blob_content).await.expect("second import");
    assert!(result2.already_existed);

    // Both should return the same blake3 hash
    assert_eq!(result1.blake3, result2.blake3);
}

#[tokio::test]
async fn test_merge_commit_import() {
    let forge = create_test_forge_node().await;
    let identity = forge.create_repo("merge-commit-test", vec![forge.public_key()], 1).await.expect("create repo");
    let repo_id = identity.repo_id();

    let mapping = Arc::new(HashMappingStore::new(forge.kv().clone()));
    let secret_key = iroh::SecretKey::generate(&mut rand::rng());

    let importer = GitImporter::new(
        Arc::clone(&mapping),
        forge.git.blobs().clone(),
        Arc::new(forge.refs.clone()),
        secret_key,
        create_hlc("test-importer"),
    );

    // Create first branch
    let blob1_content = b"Branch A content";
    let blob1_sha1 = compute_git_sha1("blob", blob1_content);
    importer
        .import_object_raw(&repo_id, blob1_sha1, "blob", blob1_content)
        .await
        .expect("import blob 1");

    let tree1_content = make_git_tree_entry("100644", "a.txt", &blob1_sha1);
    let tree1_sha1 = compute_git_sha1("tree", &tree1_content);
    importer
        .import_object_raw(&repo_id, tree1_sha1, "tree", &tree1_content)
        .await
        .expect("import tree 1");

    let commit1_content = make_git_commit(&tree1_sha1, &[], "Commit A");
    let commit1_sha1 = compute_git_sha1("commit", &commit1_content);
    importer
        .import_object_raw(&repo_id, commit1_sha1, "commit", &commit1_content)
        .await
        .expect("import commit 1");

    // Create second branch
    let blob2_content = b"Branch B content";
    let blob2_sha1 = compute_git_sha1("blob", blob2_content);
    importer
        .import_object_raw(&repo_id, blob2_sha1, "blob", blob2_content)
        .await
        .expect("import blob 2");

    let tree2_content = make_git_tree_entry("100644", "b.txt", &blob2_sha1);
    let tree2_sha1 = compute_git_sha1("tree", &tree2_content);
    importer
        .import_object_raw(&repo_id, tree2_sha1, "tree", &tree2_content)
        .await
        .expect("import tree 2");

    let commit2_content = make_git_commit(&tree2_sha1, &[], "Commit B");
    let commit2_sha1 = compute_git_sha1("commit", &commit2_content);
    importer
        .import_object_raw(&repo_id, commit2_sha1, "commit", &commit2_content)
        .await
        .expect("import commit 2");

    // Create merge tree with both files
    let mut merge_tree_content = Vec::new();
    merge_tree_content.extend(make_git_tree_entry("100644", "a.txt", &blob1_sha1));
    merge_tree_content.extend(make_git_tree_entry("100644", "b.txt", &blob2_sha1));
    let merge_tree_sha1 = compute_git_sha1("tree", &merge_tree_content);
    importer
        .import_object_raw(&repo_id, merge_tree_sha1, "tree", &merge_tree_content)
        .await
        .expect("import merge tree");

    // Create merge commit with TWO parents
    let merge_commit_content = make_git_commit(&merge_tree_sha1, &[commit1_sha1, commit2_sha1], "Merge commit");
    let merge_commit_sha1 = compute_git_sha1("commit", &merge_commit_content);
    let merge_result = importer
        .import_object_raw(&repo_id, merge_commit_sha1, "commit", &merge_commit_content)
        .await
        .expect("import merge commit");

    assert!(!merge_result.already_existed);

    // Verify the merge commit exists and can be retrieved
    let _ = importer.get_blake3(&repo_id, &merge_commit_sha1).await.expect("get blake3").expect("should exist");
}

#[tokio::test]
async fn test_nested_tree_structure() {
    let forge = create_test_forge_node().await;
    let identity = forge.create_repo("nested-tree-test", vec![forge.public_key()], 1).await.expect("create repo");
    let repo_id = identity.repo_id();

    let mapping = Arc::new(HashMappingStore::new(forge.kv().clone()));
    let secret_key = iroh::SecretKey::generate(&mut rand::rng());

    let importer = GitImporter::new(
        Arc::clone(&mapping),
        forge.git.blobs().clone(),
        Arc::new(forge.refs.clone()),
        secret_key.clone(),
        create_hlc("test-importer"),
    );
    let exporter = GitExporter::new(
        Arc::clone(&mapping),
        forge.git.blobs().clone(),
        Arc::new(forge.refs.clone()),
        secret_key,
        create_hlc("test-exporter"),
    );

    // Create src/main.rs
    let main_content = b"fn main() {}";
    let main_sha1 = compute_git_sha1("blob", main_content);
    importer.import_object_raw(&repo_id, main_sha1, "blob", main_content).await.expect("import main.rs");

    // Create src/ subtree
    let src_tree_content = make_git_tree_entry("100644", "main.rs", &main_sha1);
    let src_tree_sha1 = compute_git_sha1("tree", &src_tree_content);
    importer
        .import_object_raw(&repo_id, src_tree_sha1, "tree", &src_tree_content)
        .await
        .expect("import src tree");

    // Create README
    let readme_content = b"# Project";
    let readme_sha1 = compute_git_sha1("blob", readme_content);
    importer
        .import_object_raw(&repo_id, readme_sha1, "blob", readme_content)
        .await
        .expect("import README");

    // Create root tree with src/ subtree
    let mut root_tree_content = Vec::new();
    root_tree_content.extend(make_git_tree_entry("100644", "README.md", &readme_sha1));
    root_tree_content.extend(make_git_tree_entry("40000", "src", &src_tree_sha1));
    let root_tree_sha1 = compute_git_sha1("tree", &root_tree_content);
    importer
        .import_object_raw(&repo_id, root_tree_sha1, "tree", &root_tree_content)
        .await
        .expect("import root tree");

    // Create commit
    let commit_content = make_git_commit(&root_tree_sha1, &[], "Add project structure");
    let commit_sha1 = compute_git_sha1("commit", &commit_content);
    let commit_result = importer
        .import_object_raw(&repo_id, commit_sha1, "commit", &commit_content)
        .await
        .expect("import commit");

    // Export full DAG
    let have_set: HashSet<Sha1Hash> = HashSet::new();
    let dag_result = exporter.export_commit_dag(&repo_id, commit_result.blake3, &have_set).await.expect("export dag");

    // Should have: 2 blobs (main.rs, README) + 2 trees (src/, root) + 1 commit = 5 objects
    assert_eq!(dag_result.objects.len(), 5, "DAG should have 5 objects");
}

// ============================================================================
// Binary Content Tests
// ============================================================================

#[tokio::test]
async fn test_binary_blob_content() {
    let forge = create_test_forge_node().await;
    let identity = forge.create_repo("binary-blob-test", vec![forge.public_key()], 1).await.expect("create repo");
    let repo_id = identity.repo_id();

    let mapping = Arc::new(HashMappingStore::new(forge.kv().clone()));
    let secret_key = iroh::SecretKey::generate(&mut rand::rng());

    let importer = GitImporter::new(
        Arc::clone(&mapping),
        forge.git.blobs().clone(),
        Arc::new(forge.refs.clone()),
        secret_key.clone(),
        create_hlc("test-importer"),
    );
    let exporter = GitExporter::new(
        Arc::clone(&mapping),
        forge.git.blobs().clone(),
        Arc::new(forge.refs.clone()),
        secret_key,
        create_hlc("test-exporter"),
    );

    // Create binary content with null bytes and special characters
    let binary_content: Vec<u8> = (0..256).map(|i| i as u8).collect();
    let sha1 = compute_git_sha1("blob", &binary_content);

    // Import
    let result = importer
        .import_object_raw(&repo_id, sha1, "blob", &binary_content)
        .await
        .expect("import binary blob");

    // Export
    let exported = exporter.export_object(&repo_id, result.blake3).await.expect("export binary blob");

    assert_eq!(exported.content, binary_content);
    assert_eq!(exported.sha1, sha1);
}

#[tokio::test]
async fn test_large_blob_import_export() {
    let forge = create_test_forge_node().await;
    let identity = forge.create_repo("large-blob-test", vec![forge.public_key()], 1).await.expect("create repo");
    let repo_id = identity.repo_id();

    let mapping = Arc::new(HashMappingStore::new(forge.kv().clone()));
    let secret_key = iroh::SecretKey::generate(&mut rand::rng());

    let importer = GitImporter::new(
        Arc::clone(&mapping),
        forge.git.blobs().clone(),
        Arc::new(forge.refs.clone()),
        secret_key.clone(),
        create_hlc("test-importer"),
    );
    let exporter = GitExporter::new(
        Arc::clone(&mapping),
        forge.git.blobs().clone(),
        Arc::new(forge.refs.clone()),
        secret_key,
        create_hlc("test-exporter"),
    );

    // Create a 1MB blob
    let large_content: Vec<u8> = (0..1_000_000).map(|i| (i % 256) as u8).collect();
    let sha1 = compute_git_sha1("blob", &large_content);

    // Import
    let result = importer.import_object_raw(&repo_id, sha1, "blob", &large_content).await.expect("import large blob");

    // Export
    let exported = exporter.export_object(&repo_id, result.blake3).await.expect("export large blob");

    assert_eq!(exported.content.len(), 1_000_000);
    assert_eq!(exported.sha1, sha1);
}

// ============================================================================
// Full Clone/Push Simulation
// ============================================================================

/// Simulates a full git clone operation from Forge to a client.
/// This tests the complete export path that git-remote-aspen uses.
#[tokio::test]
async fn test_simulate_git_clone() {
    let forge = create_test_forge_node().await;
    let identity = forge.create_repo("clone-sim-test", vec![forge.public_key()], 1).await.expect("create repo");
    let repo_id = identity.repo_id();

    let mapping = Arc::new(HashMappingStore::new(forge.kv().clone()));
    let secret_key = iroh::SecretKey::generate(&mut rand::rng());

    let importer = GitImporter::new(
        Arc::clone(&mapping),
        forge.git.blobs().clone(),
        Arc::new(forge.refs.clone()),
        secret_key.clone(),
        create_hlc("test-importer"),
    );
    let exporter = GitExporter::new(
        Arc::clone(&mapping),
        forge.git.blobs().clone(),
        Arc::new(forge.refs.clone()),
        secret_key,
        create_hlc("test-exporter"),
    );

    // Create a simple repository structure
    let readme_content = b"# My Project\n\nThis is a test project.";
    let readme_sha1 = compute_git_sha1("blob", readme_content);
    importer
        .import_object_raw(&repo_id, readme_sha1, "blob", readme_content)
        .await
        .expect("import README");

    let tree_content = make_git_tree_entry("100644", "README.md", &readme_sha1);
    let tree_sha1 = compute_git_sha1("tree", &tree_content);
    importer.import_object_raw(&repo_id, tree_sha1, "tree", &tree_content).await.expect("import tree");

    let commit_content = make_git_commit(&tree_sha1, &[], "Initial commit\n\nThis is the initial commit.");
    let commit_sha1 = compute_git_sha1("commit", &commit_content);
    let commit_result = importer
        .import_object_raw(&repo_id, commit_sha1, "commit", &commit_content)
        .await
        .expect("import commit");

    // Set up refs
    importer.update_ref(&repo_id, "heads/main", commit_sha1).await.expect("set main");
    importer.update_ref(&repo_id, "tags/v1.0", commit_sha1).await.expect("set tag");

    // STEP 1: git ls-remote (client lists refs)
    let refs = exporter.list_refs(&repo_id).await.expect("list refs");
    assert!(refs.len() >= 2, "Should have main branch and tag");

    // STEP 2: git fetch-pack (client requests objects)
    let want_sha1s: Vec<Sha1Hash> = refs.iter().filter_map(|(_, sha1)| *sha1).collect();
    assert!(!want_sha1s.is_empty(), "Should have at least one ref to fetch");

    // STEP 3: Server sends pack (export objects)
    let have_set: HashSet<Sha1Hash> = HashSet::new();
    let dag_result = exporter.export_commit_dag(&repo_id, commit_result.blake3, &have_set).await.expect("export dag");

    // Verify all objects are present
    assert_eq!(dag_result.objects.len(), 3, "Should have blob, tree, commit");

    // Verify objects are in valid git format
    for obj in &dag_result.objects {
        let git_bytes = obj.to_git_bytes();
        assert!(!git_bytes.is_empty());
        // Verify header format
        let header_end = git_bytes.iter().position(|&b| b == 0).expect("should have null byte");
        let header = std::str::from_utf8(&git_bytes[..header_end]).expect("valid header");
        assert!(
            header.starts_with("blob ") || header.starts_with("tree ") || header.starts_with("commit "),
            "Invalid header: {}",
            header
        );
    }
}

/// Simulates a full git push operation from a client to Forge.
/// This tests the complete import path that git-remote-aspen uses.
#[tokio::test]
async fn test_simulate_git_push() {
    let forge = create_test_forge_node().await;
    let identity = forge.create_repo("push-sim-test", vec![forge.public_key()], 1).await.expect("create repo");
    let repo_id = identity.repo_id();

    let mapping = Arc::new(HashMappingStore::new(forge.kv().clone()));
    let secret_key = iroh::SecretKey::generate(&mut rand::rng());

    let importer = GitImporter::new(
        Arc::clone(&mapping),
        forge.git.blobs().clone(),
        Arc::new(forge.refs.clone()),
        secret_key.clone(),
        create_hlc("test-importer"),
    );
    let exporter = GitExporter::new(
        Arc::clone(&mapping),
        forge.git.blobs().clone(),
        Arc::new(forge.refs.clone()),
        secret_key,
        create_hlc("test-exporter"),
    );

    // Simulate a client sending a push with multiple commits

    // Commit 1: Initial
    let file1_content = b"fn main() { println!(\"Hello\"); }";
    let file1_sha1 = compute_git_sha1("blob", file1_content);
    importer.import_object_raw(&repo_id, file1_sha1, "blob", file1_content).await.expect("import file1");

    let tree1_content = make_git_tree_entry("100644", "main.rs", &file1_sha1);
    let tree1_sha1 = compute_git_sha1("tree", &tree1_content);
    importer
        .import_object_raw(&repo_id, tree1_sha1, "tree", &tree1_content)
        .await
        .expect("import tree1");

    let commit1_content = make_git_commit(&tree1_sha1, &[], "Initial commit");
    let commit1_sha1 = compute_git_sha1("commit", &commit1_content);
    importer
        .import_object_raw(&repo_id, commit1_sha1, "commit", &commit1_content)
        .await
        .expect("import commit1");

    // Commit 2: Add feature
    let file2_content = b"fn main() { println!(\"Hello, World!\"); }";
    let file2_sha1 = compute_git_sha1("blob", file2_content);
    importer.import_object_raw(&repo_id, file2_sha1, "blob", file2_content).await.expect("import file2");

    let tree2_content = make_git_tree_entry("100644", "main.rs", &file2_sha1);
    let tree2_sha1 = compute_git_sha1("tree", &tree2_content);
    importer
        .import_object_raw(&repo_id, tree2_sha1, "tree", &tree2_content)
        .await
        .expect("import tree2");

    let commit2_content = make_git_commit(&tree2_sha1, &[commit1_sha1], "Add greeting");
    let commit2_sha1 = compute_git_sha1("commit", &commit2_content);
    importer
        .import_object_raw(&repo_id, commit2_sha1, "commit", &commit2_content)
        .await
        .expect("import commit2");

    // Update refs (simulating push refs)
    importer.update_ref(&repo_id, "heads/main", commit2_sha1).await.expect("update main");

    // Verify the push succeeded by listing refs
    let refs = exporter.list_refs(&repo_id).await.expect("list refs");
    let main_ref = refs.iter().find(|(name, _)| name.contains("main")).expect("main ref");
    assert_eq!(main_ref.1.as_ref().unwrap(), &commit2_sha1);

    // Verify the commit chain is intact
    let have_set: HashSet<Sha1Hash> = HashSet::new();
    let blake3 = importer.get_blake3(&repo_id, &commit2_sha1).await.unwrap().unwrap();
    let dag_result = exporter.export_commit_dag(&repo_id, blake3, &have_set).await.expect("export dag");

    // Should have all 6 objects: 2 blobs + 2 trees + 2 commits
    assert_eq!(dag_result.objects.len(), 6, "Should have all objects from both commits");
}
