//! Integration tests for the Git Bridge import↔export round-trip.
//!
//! These tests verify that objects survive the full SHA-1 → BLAKE3 → SHA-1
//! round-trip through the bridge. They use `DeterministicKeyValueStore` and
//! `InMemoryBlobStore` — no Raft or network needed.

use std::sync::Arc;

use aspen_blob::InMemoryBlobStore;
use aspen_core::KeyValueStore;
use aspen_core::create_hlc;
use aspen_testing_core::DeterministicKeyValueStore;

use super::exporter::GitExporter;
use super::importer::GitImporter;
use super::mapping::GitObjectType;
use super::mapping::HashMappingStore;
use super::sha1::Sha1Hash;
use crate::identity::RepoId;
use crate::refs::RefStore;

/// Test harness that wires up importer + exporter with in-memory stores.
///
/// Uses `dyn KeyValueStore` to match how the real system passes Arc<dyn KV>.
struct BridgeTestHarness {
    importer: GitImporter<dyn KeyValueStore, InMemoryBlobStore>,
    exporter: GitExporter<dyn KeyValueStore, InMemoryBlobStore>,
    mapping: Arc<HashMappingStore<dyn KeyValueStore>>,
    repo_id: RepoId,
}

impl BridgeTestHarness {
    fn new() -> Self {
        let kv: Arc<dyn KeyValueStore> = Arc::new(DeterministicKeyValueStore::new());
        let blobs = Arc::new(InMemoryBlobStore::new());
        let mapping = Arc::new(HashMappingStore::new(Arc::clone(&kv)));
        let refs = Arc::new(RefStore::new(Arc::clone(&kv)));
        let secret_key = iroh::SecretKey::generate(&mut rand::rng());

        let importer = GitImporter::new(
            Arc::clone(&mapping),
            Arc::clone(&blobs),
            Arc::clone(&refs),
            secret_key.clone(),
            create_hlc("test-importer"),
        );
        let exporter = GitExporter::new(
            Arc::clone(&mapping),
            Arc::clone(&blobs),
            Arc::clone(&refs),
            secret_key,
            create_hlc("test-exporter"),
        );

        // Create a deterministic repo ID
        let repo_id = RepoId::from_hash(blake3::hash(b"test-repo"));

        Self {
            importer,
            exporter,
            mapping,
            repo_id,
        }
    }
}

/// Build a git blob object in wire format: "blob <size>\0<content>"
fn make_git_blob(content: &[u8]) -> Vec<u8> {
    let header = format!("blob {}\0", content.len());
    let mut bytes = Vec::with_capacity(header.len() + content.len());
    bytes.extend_from_slice(header.as_bytes());
    bytes.extend_from_slice(content);
    bytes
}

/// Compute SHA-1 of a git object (including header).
fn compute_sha1(git_bytes: &[u8]) -> Sha1Hash {
    use sha1::Digest;
    let hash = sha1::Sha1::digest(git_bytes);
    let bytes: [u8; 20] = hash.into();
    Sha1Hash::from(bytes)
}

/// Build a git tree entry: "<mode> <name>\0<20-byte-sha1>"
fn make_tree_entry(mode: &str, name: &str, sha1: &Sha1Hash) -> Vec<u8> {
    let mut entry = Vec::new();
    entry.extend_from_slice(mode.as_bytes());
    entry.push(b' ');
    entry.extend_from_slice(name.as_bytes());
    entry.push(0);
    entry.extend_from_slice(sha1.as_slice());
    entry
}

/// Build a git tree object from entries.
fn make_git_tree(entries: &[Vec<u8>]) -> Vec<u8> {
    let mut content = Vec::new();
    for entry in entries {
        content.extend_from_slice(entry);
    }
    let header = format!("tree {}\0", content.len());
    let mut bytes = Vec::with_capacity(header.len() + content.len());
    bytes.extend_from_slice(header.as_bytes());
    bytes.extend_from_slice(&content);
    bytes
}

/// Build a git commit object.
///
/// Note: Git commit messages conventionally end with `\n`. The bridge export
/// always ensures a trailing newline, so we include it here for SHA-1 fidelity.
fn make_git_commit(tree_sha1: &Sha1Hash, parents: &[&Sha1Hash], message: &str) -> Vec<u8> {
    let mut content = String::new();
    content.push_str(&format!("tree {}\n", tree_sha1));
    for parent in parents {
        content.push_str(&format!("parent {}\n", parent));
    }
    content.push_str("author Test User <test@example.com> 1700000000 +0000\n");
    content.push_str("committer Test User <test@example.com> 1700000000 +0000\n");
    content.push('\n');
    content.push_str(message);
    // Git commit messages always end with a newline
    if !message.ends_with('\n') {
        content.push('\n');
    }

    let header = format!("commit {}\0", content.len());
    let mut bytes = Vec::with_capacity(header.len() + content.len());
    bytes.extend_from_slice(header.as_bytes());
    bytes.extend_from_slice(content.as_bytes());
    bytes
}

/// Build a git annotated tag object.
///
/// Note: Git tag messages conventionally end with `\n`. The bridge export
/// always ensures a trailing newline.
fn make_git_tag(target_sha1: &Sha1Hash, tag_name: &str, message: &str) -> Vec<u8> {
    let mut content = String::new();
    content.push_str(&format!("object {}\n", target_sha1));
    content.push_str("type commit\n");
    content.push_str(&format!("tag {}\n", tag_name));
    content.push_str("tagger Test User <test@example.com> 1700000000 +0000\n");
    content.push('\n');
    content.push_str(message);
    // Git tag messages always end with a newline
    if !message.ends_with('\n') {
        content.push('\n');
    }

    let header = format!("tag {}\0", content.len());
    let mut bytes = Vec::with_capacity(header.len() + content.len());
    bytes.extend_from_slice(header.as_bytes());
    bytes.extend_from_slice(content.as_bytes());
    bytes
}

// ============================================================================
// Tests
// ============================================================================

#[tokio::test]
async fn test_blob_import_export_roundtrip() {
    let h = BridgeTestHarness::new();
    let content = b"Hello, Forge!\n";
    let git_bytes = make_git_blob(content);
    let original_sha1 = compute_sha1(&git_bytes);

    // Import
    let blake3_hash = h.importer.import_object(&h.repo_id, &git_bytes).await.unwrap();

    // Verify mapping exists
    let lookup = h.mapping.get_sha1(&h.repo_id, &blake3_hash).await.unwrap();
    assert!(lookup.is_some(), "BLAKE3→SHA-1 mapping should exist after import");
    let (exported_sha1, obj_type) = lookup.unwrap();
    assert_eq!(exported_sha1, original_sha1, "SHA-1 should round-trip");
    assert_eq!(obj_type, GitObjectType::Blob);

    // Export
    let exported = h.exporter.export_object(&h.repo_id, blake3_hash).await.unwrap();
    assert_eq!(exported.sha1, original_sha1);
    assert_eq!(exported.object_type, GitObjectType::Blob);
    assert_eq!(exported.content, content, "blob content should round-trip");
}

#[tokio::test]
async fn test_tree_import_export_roundtrip() {
    let h = BridgeTestHarness::new();

    // First import a blob (tree depends on it)
    let blob_content = b"file content";
    let blob_git = make_git_blob(blob_content);
    let blob_sha1 = compute_sha1(&blob_git);
    h.importer.import_object(&h.repo_id, &blob_git).await.unwrap();

    // Build and import a tree
    let tree_entry = make_tree_entry("100644", "README.md", &blob_sha1);
    let tree_git = make_git_tree(&[tree_entry]);
    let tree_sha1 = compute_sha1(&tree_git);
    let tree_blake3 = h.importer.import_object(&h.repo_id, &tree_git).await.unwrap();

    // Export and verify
    let exported = h.exporter.export_object(&h.repo_id, tree_blake3).await.unwrap();
    assert_eq!(exported.sha1, tree_sha1);
    assert_eq!(exported.object_type, GitObjectType::Tree);

    // The exported git bytes should match the original when reconstructed
    let exported_git = exported.to_git_bytes();
    assert_eq!(compute_sha1(&exported_git), tree_sha1, "exported tree's SHA-1 should match original");
}

#[tokio::test]
async fn test_commit_import_export_roundtrip() {
    let h = BridgeTestHarness::new();

    // Build dependency chain: blob → tree → commit
    let blob_git = make_git_blob(b"initial");
    let blob_sha1 = compute_sha1(&blob_git);
    h.importer.import_object(&h.repo_id, &blob_git).await.unwrap();

    let tree_entry = make_tree_entry("100644", "file.txt", &blob_sha1);
    let tree_git = make_git_tree(&[tree_entry]);
    let tree_sha1 = compute_sha1(&tree_git);
    h.importer.import_object(&h.repo_id, &tree_git).await.unwrap();

    let commit_git = make_git_commit(&tree_sha1, &[], "Initial commit");
    let commit_sha1 = compute_sha1(&commit_git);
    let commit_blake3 = h.importer.import_object(&h.repo_id, &commit_git).await.unwrap();

    // Verify the mapping SHA-1 matches (import correctness)
    let (mapped_sha1, obj_type) = h.mapping.get_sha1(&h.repo_id, &commit_blake3).await.unwrap().unwrap();
    assert_eq!(mapped_sha1, commit_sha1, "import should record correct SHA-1 in mapping");
    assert_eq!(obj_type, GitObjectType::Commit);

    // Export and verify lossless round-trip
    let exported = h.exporter.export_object(&h.repo_id, commit_blake3).await.unwrap();
    assert_eq!(exported.sha1, commit_sha1, "exported SHA-1 should match import");
    assert_eq!(exported.object_type, GitObjectType::Commit);
}

#[tokio::test]
async fn test_single_file_repo_roundtrip() {
    let h = BridgeTestHarness::new();

    // Build a complete single-file repo: blob → tree → commit
    let blob_git = make_git_blob(b"# My Project\n");
    let blob_sha1 = compute_sha1(&blob_git);
    let blob_b3 = h.importer.import_object(&h.repo_id, &blob_git).await.unwrap();

    let tree_entry = make_tree_entry("100644", "README.md", &blob_sha1);
    let tree_git = make_git_tree(&[tree_entry]);
    let tree_sha1 = compute_sha1(&tree_git);
    let tree_b3 = h.importer.import_object(&h.repo_id, &tree_git).await.unwrap();

    let commit_git = make_git_commit(&tree_sha1, &[], "Initial commit");
    let commit_sha1 = compute_sha1(&commit_git);
    let commit_b3 = h.importer.import_object(&h.repo_id, &commit_git).await.unwrap();

    // Verify all three round-trip through export
    for (blake3_hash, expected_sha1, expected_type) in [
        (blob_b3, blob_sha1, GitObjectType::Blob),
        (tree_b3, tree_sha1, GitObjectType::Tree),
        (commit_b3, commit_sha1, GitObjectType::Commit),
    ] {
        let exported = h.exporter.export_object(&h.repo_id, blake3_hash).await.unwrap();
        assert_eq!(exported.sha1, expected_sha1, "SHA-1 mismatch for {:?}", expected_type);
        assert_eq!(exported.object_type, expected_type);
    }
}

#[tokio::test]
async fn test_multi_commit_history_roundtrip() {
    let h = BridgeTestHarness::new();

    // Commit 1: initial file
    let blob1_git = make_git_blob(b"version 1");
    let blob1_sha1 = compute_sha1(&blob1_git);
    h.importer.import_object(&h.repo_id, &blob1_git).await.unwrap();

    let tree1_entry = make_tree_entry("100644", "file.txt", &blob1_sha1);
    let tree1_git = make_git_tree(&[tree1_entry]);
    let tree1_sha1 = compute_sha1(&tree1_git);
    h.importer.import_object(&h.repo_id, &tree1_git).await.unwrap();

    let commit1_git = make_git_commit(&tree1_sha1, &[], "First commit");
    let commit1_sha1 = compute_sha1(&commit1_git);
    h.importer.import_object(&h.repo_id, &commit1_git).await.unwrap();

    // Commit 2: update file (references commit 1 as parent)
    let blob2_git = make_git_blob(b"version 2");
    let blob2_sha1 = compute_sha1(&blob2_git);
    h.importer.import_object(&h.repo_id, &blob2_git).await.unwrap();

    let tree2_entry = make_tree_entry("100644", "file.txt", &blob2_sha1);
    let tree2_git = make_git_tree(&[tree2_entry]);
    let tree2_sha1 = compute_sha1(&tree2_git);
    h.importer.import_object(&h.repo_id, &tree2_git).await.unwrap();

    let commit2_git = make_git_commit(&tree2_sha1, &[&commit1_sha1], "Second commit");
    let commit2_sha1 = compute_sha1(&commit2_git);
    h.importer.import_object(&h.repo_id, &commit2_git).await.unwrap();

    // Commit 3: another update (references commit 2)
    let blob3_git = make_git_blob(b"version 3");
    let blob3_sha1 = compute_sha1(&blob3_git);
    h.importer.import_object(&h.repo_id, &blob3_git).await.unwrap();

    let tree3_entry = make_tree_entry("100644", "file.txt", &blob3_sha1);
    let tree3_git = make_git_tree(&[tree3_entry]);
    let tree3_sha1 = compute_sha1(&tree3_git);
    h.importer.import_object(&h.repo_id, &tree3_git).await.unwrap();

    let commit3_git = make_git_commit(&tree3_sha1, &[&commit2_sha1], "Third commit");
    let commit3_sha1 = compute_sha1(&commit3_git);
    let commit3_b3 = h.importer.import_object(&h.repo_id, &commit3_git).await.unwrap();

    // Export the commit DAG from the tip and verify all objects come back
    let have: std::collections::HashSet<Sha1Hash> = std::collections::HashSet::new();
    let export_result = h.exporter.export_commit_dag(&h.repo_id, commit3_b3, &have).await.unwrap();

    // Should include all 9 objects (3 blobs + 3 trees + 3 commits)
    assert!(
        export_result.objects.len() >= 9,
        "expected at least 9 objects in DAG, got {}",
        export_result.objects.len()
    );

    // Verify the tip commit's SHA-1 is correct
    let tip = export_result.objects.iter().find(|o| o.sha1 == commit3_sha1);
    assert!(tip.is_some(), "tip commit should be in exported DAG");
}

#[tokio::test]
async fn test_merge_commit_roundtrip() {
    let h = BridgeTestHarness::new();

    // Branch A: blob → tree → commit
    let blob_a_git = make_git_blob(b"branch a");
    let blob_a_sha1 = compute_sha1(&blob_a_git);
    h.importer.import_object(&h.repo_id, &blob_a_git).await.unwrap();

    let tree_a_entry = make_tree_entry("100644", "a.txt", &blob_a_sha1);
    let tree_a_git = make_git_tree(&[tree_a_entry]);
    let tree_a_sha1 = compute_sha1(&tree_a_git);
    h.importer.import_object(&h.repo_id, &tree_a_git).await.unwrap();

    let commit_a_git = make_git_commit(&tree_a_sha1, &[], "Branch A");
    let commit_a_sha1 = compute_sha1(&commit_a_git);
    h.importer.import_object(&h.repo_id, &commit_a_git).await.unwrap();

    // Branch B: blob → tree → commit
    let blob_b_git = make_git_blob(b"branch b");
    let blob_b_sha1 = compute_sha1(&blob_b_git);
    h.importer.import_object(&h.repo_id, &blob_b_git).await.unwrap();

    let tree_b_entry = make_tree_entry("100644", "b.txt", &blob_b_sha1);
    let tree_b_git = make_git_tree(&[tree_b_entry]);
    let tree_b_sha1 = compute_sha1(&tree_b_git);
    h.importer.import_object(&h.repo_id, &tree_b_git).await.unwrap();

    let commit_b_git = make_git_commit(&tree_b_sha1, &[], "Branch B");
    let commit_b_sha1 = compute_sha1(&commit_b_git);
    h.importer.import_object(&h.repo_id, &commit_b_git).await.unwrap();

    // Merge commit: both files, two parents
    let merge_tree_git = make_git_tree(&[
        make_tree_entry("100644", "a.txt", &blob_a_sha1),
        make_tree_entry("100644", "b.txt", &blob_b_sha1),
    ]);
    let merge_tree_sha1 = compute_sha1(&merge_tree_git);
    h.importer.import_object(&h.repo_id, &merge_tree_git).await.unwrap();

    let merge_git = make_git_commit(&merge_tree_sha1, &[&commit_a_sha1, &commit_b_sha1], "Merge A and B");
    let merge_sha1 = compute_sha1(&merge_git);
    let merge_b3 = h.importer.import_object(&h.repo_id, &merge_git).await.unwrap();

    // Export and verify
    let exported = h.exporter.export_object(&h.repo_id, merge_b3).await.unwrap();
    assert_eq!(exported.sha1, merge_sha1);
    assert_eq!(exported.object_type, GitObjectType::Commit);

    // Verify the exported content contains both parent hashes
    let content = String::from_utf8_lossy(&exported.content);
    assert!(content.contains(&format!("parent {}", commit_a_sha1)), "merge commit should reference parent A");
    assert!(content.contains(&format!("parent {}", commit_b_sha1)), "merge commit should reference parent B");
}

#[tokio::test]
async fn test_nested_tree_roundtrip() {
    let h = BridgeTestHarness::new();

    // Create a nested directory structure: root/src/main.rs
    let blob_git = make_git_blob(b"fn main() {}");
    let blob_sha1 = compute_sha1(&blob_git);
    h.importer.import_object(&h.repo_id, &blob_git).await.unwrap();

    // Inner tree: src/ containing main.rs
    let inner_entry = make_tree_entry("100644", "main.rs", &blob_sha1);
    let inner_tree_git = make_git_tree(&[inner_entry]);
    let inner_sha1 = compute_sha1(&inner_tree_git);
    h.importer.import_object(&h.repo_id, &inner_tree_git).await.unwrap();

    // Root tree: contains src/ subdirectory
    let root_entry = make_tree_entry("40000", "src", &inner_sha1);
    let root_tree_git = make_git_tree(&[root_entry]);
    let root_sha1 = compute_sha1(&root_tree_git);
    let root_b3 = h.importer.import_object(&h.repo_id, &root_tree_git).await.unwrap();

    // Export root tree and verify SHA-1 match
    let exported = h.exporter.export_object(&h.repo_id, root_b3).await.unwrap();
    assert_eq!(exported.sha1, root_sha1);
    assert_eq!(exported.object_type, GitObjectType::Tree);

    // Reconstruct the git bytes and verify SHA-1 of the export
    let exported_git = exported.to_git_bytes();
    assert_eq!(compute_sha1(&exported_git), root_sha1, "nested tree SHA-1 should survive round-trip");
}

#[tokio::test]
async fn test_annotated_tag_roundtrip() {
    let h = BridgeTestHarness::new();

    // Build a commit to tag
    let blob_git = make_git_blob(b"tagged content");
    let blob_sha1 = compute_sha1(&blob_git);
    h.importer.import_object(&h.repo_id, &blob_git).await.unwrap();

    let tree_entry = make_tree_entry("100644", "file.txt", &blob_sha1);
    let tree_git = make_git_tree(&[tree_entry]);
    let tree_sha1 = compute_sha1(&tree_git);
    h.importer.import_object(&h.repo_id, &tree_git).await.unwrap();

    let commit_git = make_git_commit(&tree_sha1, &[], "Release commit");
    let commit_sha1 = compute_sha1(&commit_git);
    h.importer.import_object(&h.repo_id, &commit_git).await.unwrap();

    // Create annotated tag
    let tag_git = make_git_tag(&commit_sha1, "v1.0.0", "Release v1.0.0\n\nFirst stable release.");
    let tag_sha1 = compute_sha1(&tag_git);
    let tag_b3 = h.importer.import_object(&h.repo_id, &tag_git).await.unwrap();

    // Export and verify
    let exported = h.exporter.export_object(&h.repo_id, tag_b3).await.unwrap();
    assert_eq!(exported.sha1, tag_sha1);
    assert_eq!(exported.object_type, GitObjectType::Tag);

    let content = String::from_utf8_lossy(&exported.content);
    assert!(content.contains("tag v1.0.0"), "tag name should survive round-trip");
    assert!(content.contains(&format!("object {}", commit_sha1)), "tag target should reference correct commit");
}

#[tokio::test]
async fn test_import_idempotent_sha1_mapping() {
    let h = BridgeTestHarness::new();

    let git_bytes = make_git_blob(b"idempotent test");
    let original_sha1 = compute_sha1(&git_bytes);

    // Import twice — BLAKE3 hashes may differ (SignedObject wraps with HLC
    // timestamp), but the SHA-1 mapping should be stable.
    let blake3_1 = h.importer.import_object(&h.repo_id, &git_bytes).await.unwrap();
    let blake3_2 = h.importer.import_object(&h.repo_id, &git_bytes).await.unwrap();

    // Both mappings should resolve to the same SHA-1
    let (sha1_1, _) = h.mapping.get_sha1(&h.repo_id, &blake3_1).await.unwrap().unwrap();
    let (sha1_2, _) = h.mapping.get_sha1(&h.repo_id, &blake3_2).await.unwrap().unwrap();
    assert_eq!(sha1_1, original_sha1, "first import SHA-1 should match");
    assert_eq!(sha1_2, original_sha1, "second import SHA-1 should match");

    // Reverse lookup from SHA-1 should resolve (to one of the BLAKE3 hashes)
    let lookup = h.mapping.get_blake3(&h.repo_id, &original_sha1).await.unwrap();
    assert!(lookup.is_some(), "SHA-1→BLAKE3 reverse lookup should exist");
}

#[tokio::test]
async fn test_export_dag_with_have_set() {
    let h = BridgeTestHarness::new();

    // Build two commits
    let blob1_git = make_git_blob(b"v1");
    let blob1_sha1 = compute_sha1(&blob1_git);
    h.importer.import_object(&h.repo_id, &blob1_git).await.unwrap();

    let tree1_entry = make_tree_entry("100644", "f.txt", &blob1_sha1);
    let tree1_git = make_git_tree(&[tree1_entry]);
    let tree1_sha1 = compute_sha1(&tree1_git);
    h.importer.import_object(&h.repo_id, &tree1_git).await.unwrap();

    let commit1_git = make_git_commit(&tree1_sha1, &[], "First");
    let commit1_sha1 = compute_sha1(&commit1_git);
    h.importer.import_object(&h.repo_id, &commit1_git).await.unwrap();

    let blob2_git = make_git_blob(b"v2");
    let blob2_sha1 = compute_sha1(&blob2_git);
    h.importer.import_object(&h.repo_id, &blob2_git).await.unwrap();

    let tree2_entry = make_tree_entry("100644", "f.txt", &blob2_sha1);
    let tree2_git = make_git_tree(&[tree2_entry]);
    let tree2_sha1 = compute_sha1(&tree2_git);
    h.importer.import_object(&h.repo_id, &tree2_git).await.unwrap();

    let commit2_git = make_git_commit(&tree2_sha1, &[&commit1_sha1], "Second");
    let _commit2_sha1 = compute_sha1(&commit2_git);
    let commit2_b3 = h.importer.import_object(&h.repo_id, &commit2_git).await.unwrap();

    // Export with commit1 in the "have" set — should skip it and its tree/blob
    let mut have = std::collections::HashSet::new();
    have.insert(commit1_sha1);

    let result = h.exporter.export_commit_dag(&h.repo_id, commit2_b3, &have).await.unwrap();

    // Should NOT include commit1's objects (they're in "have")
    let sha1s: Vec<_> = result.objects.iter().map(|o| o.sha1).collect();
    assert!(!sha1s.contains(&commit1_sha1), "commit1 should be excluded (in have set)");

    // Should include commit2's new objects
    assert!(!result.objects.is_empty(), "should have at least commit2 + its tree + its blob");
}

#[tokio::test]
async fn test_empty_repo_list_refs() {
    let h = BridgeTestHarness::new();

    // List refs on a repo with no refs
    let refs = h.exporter.list_refs(&h.repo_id).await.unwrap();
    assert!(refs.is_empty(), "empty repo should have no refs");
}

#[tokio::test]
async fn test_ref_update_through_import() {
    let h = BridgeTestHarness::new();

    // Build a commit
    let blob_git = make_git_blob(b"ref test");
    let blob_sha1 = compute_sha1(&blob_git);
    h.importer.import_object(&h.repo_id, &blob_git).await.unwrap();

    let tree_entry = make_tree_entry("100644", "f.txt", &blob_sha1);
    let tree_git = make_git_tree(&[tree_entry]);
    let tree_sha1 = compute_sha1(&tree_git);
    h.importer.import_object(&h.repo_id, &tree_git).await.unwrap();

    let commit_git = make_git_commit(&tree_sha1, &[], "Ref test commit");
    let commit_sha1 = compute_sha1(&commit_git);
    h.importer.import_object(&h.repo_id, &commit_git).await.unwrap();

    // Update a ref through the importer
    h.importer.update_ref(&h.repo_id, "refs/heads/main", commit_sha1).await.unwrap();

    // Verify ref is visible through the exporter
    let refs = h.exporter.list_refs(&h.repo_id).await.unwrap();
    assert!(!refs.is_empty(), "should have at least one ref after update");

    let main_ref = refs.iter().find(|(name, _)| name.contains("main"));
    assert!(main_ref.is_some(), "refs/heads/main should exist");
}

#[tokio::test]
async fn test_executable_file_mode_roundtrip() {
    let h = BridgeTestHarness::new();

    let blob_git = make_git_blob(b"#!/bin/sh\necho hello");
    let blob_sha1 = compute_sha1(&blob_git);
    h.importer.import_object(&h.repo_id, &blob_git).await.unwrap();

    // Tree with executable file (mode 100755)
    let entry = make_tree_entry("100755", "run.sh", &blob_sha1);
    let tree_git = make_git_tree(&[entry]);
    let tree_sha1 = compute_sha1(&tree_git);
    let tree_b3 = h.importer.import_object(&h.repo_id, &tree_git).await.unwrap();

    let exported = h.exporter.export_object(&h.repo_id, tree_b3).await.unwrap();
    let exported_git = exported.to_git_bytes();
    assert_eq!(compute_sha1(&exported_git), tree_sha1, "executable mode tree should round-trip correctly");
}
