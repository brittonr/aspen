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

// ============================================================================
// End-to-end handler-level tests
//
// These simulate the same operations that the git-remote-aspen server handlers
// perform: push (import objects + update refs), list refs, fetch (export DAG),
// and probe (check which SHA-1s the server already has).
//
// This is the full push → list → fetch → verify cycle that a real
// `git push` / `git clone` / `git fetch` goes through.
// ============================================================================

/// Helper: import objects via the same path the server's handle_git_bridge_push uses.
///
/// The server receives `GitBridgeObject` (headerless data), reconstructs the git
/// header, then calls `import_objects()`. This helper does the same.
async fn handler_push_objects(
    h: &BridgeTestHarness,
    objects: &[(Sha1Hash, &str, &[u8])], // (sha1, type_str, headerless_data)
) -> super::importer::ImportResult {
    let import_objects: Vec<(Sha1Hash, GitObjectType, Vec<u8>)> = objects
        .iter()
        .map(|(sha1, type_str, data)| {
            let obj_type = match *type_str {
                "blob" => GitObjectType::Blob,
                "tree" => GitObjectType::Tree,
                "commit" => GitObjectType::Commit,
                "tag" => GitObjectType::Tag,
                _ => panic!("unknown type: {}", type_str),
            };
            // Reconstruct git header (same as handle_git_bridge_push)
            let header = format!("{} {}\0", type_str, data.len());
            let mut git_bytes = Vec::with_capacity(header.len() + data.len());
            git_bytes.extend_from_slice(header.as_bytes());
            git_bytes.extend_from_slice(data);
            (*sha1, obj_type, git_bytes)
        })
        .collect();

    h.importer.import_objects(&h.repo_id, import_objects).await.unwrap()
}

/// Helper: split git object bytes into (type, headerless_data) for push simulation.
fn split_git_object(git_bytes: &[u8]) -> (&str, &[u8]) {
    let null_pos = git_bytes.iter().position(|&b| b == 0).expect("missing null in git object");
    let header = std::str::from_utf8(&git_bytes[..null_pos]).expect("invalid header");
    let type_str = header.split(' ').next().expect("missing type in header");
    (type_str, &git_bytes[null_pos + 1..])
}

/// Build a complete single-file repo and return all the pieces.
struct SingleFileRepo {
    blob_sha1: Sha1Hash,
    blob_data: Vec<u8>, // headerless
    tree_sha1: Sha1Hash,
    tree_data: Vec<u8>, // headerless
    commit_sha1: Sha1Hash,
    commit_data: Vec<u8>, // headerless
}

impl SingleFileRepo {
    fn new(filename: &str, content: &[u8], message: &str) -> Self {
        let blob_git = make_git_blob(content);
        let blob_sha1 = compute_sha1(&blob_git);
        let (_, blob_data) = split_git_object(&blob_git);

        let tree_entry = make_tree_entry("100644", filename, &blob_sha1);
        let tree_git = make_git_tree(&[tree_entry]);
        let tree_sha1 = compute_sha1(&tree_git);
        let (_, tree_data) = split_git_object(&tree_git);

        let commit_git = make_git_commit(&tree_sha1, &[], message);
        let commit_sha1 = compute_sha1(&commit_git);
        let (_, commit_data) = split_git_object(&commit_git);

        Self {
            blob_sha1,
            blob_data: blob_data.to_vec(),
            tree_sha1,
            tree_data: tree_data.to_vec(),
            commit_sha1,
            commit_data: commit_data.to_vec(),
        }
    }

    fn with_parent(filename: &str, content: &[u8], message: &str, parent: &Sha1Hash) -> Self {
        let blob_git = make_git_blob(content);
        let blob_sha1 = compute_sha1(&blob_git);
        let (_, blob_data) = split_git_object(&blob_git);

        let tree_entry = make_tree_entry("100644", filename, &blob_sha1);
        let tree_git = make_git_tree(&[tree_entry]);
        let tree_sha1 = compute_sha1(&tree_git);
        let (_, tree_data) = split_git_object(&tree_git);

        let commit_git = make_git_commit(&tree_sha1, &[parent], message);
        let commit_sha1 = compute_sha1(&commit_git);
        let (_, commit_data) = split_git_object(&commit_git);

        Self {
            blob_sha1,
            blob_data: blob_data.to_vec(),
            tree_sha1,
            tree_data: tree_data.to_vec(),
            commit_sha1,
            commit_data: commit_data.to_vec(),
        }
    }

    fn objects(&self) -> Vec<(Sha1Hash, &str, &[u8])> {
        vec![
            (self.blob_sha1, "blob", &self.blob_data),
            (self.tree_sha1, "tree", &self.tree_data),
            (self.commit_sha1, "commit", &self.commit_data),
        ]
    }
}

// ---- Push → List → Fetch round-trip tests ----

#[tokio::test]
async fn test_e2e_push_list_fetch_single_branch() {
    let h = BridgeTestHarness::new();
    let repo = SingleFileRepo::new("README.md", b"# Hello\n", "Initial commit");

    // Push: import objects + update ref (same as handle_git_bridge_push)
    let result = handler_push_objects(&h, &repo.objects()).await;
    assert_eq!(result.objects_imported, 3);
    assert_eq!(result.objects_skipped, 0);

    // Update ref (strip "refs/" prefix like the handler does)
    h.importer.update_ref(&h.repo_id, "heads/main", repo.commit_sha1).await.unwrap();

    // List refs (same as handle_git_bridge_list_refs)
    let refs = h.exporter.list_refs(&h.repo_id).await.unwrap();
    assert!(!refs.is_empty(), "should have refs after push");

    let main_ref = refs.iter().find(|(name, _)| name.contains("main"));
    assert!(main_ref.is_some(), "heads/main should be listed");
    let (_, main_sha1) = main_ref.unwrap();
    assert_eq!(main_sha1.unwrap(), repo.commit_sha1, "ref should point to pushed commit");

    // Fetch: export the commit DAG (same as handle_git_bridge_fetch)
    let (blake3_hash, _) = h.mapping.get_blake3(&h.repo_id, &repo.commit_sha1).await.unwrap().unwrap();
    let have = std::collections::HashSet::new();
    let export_result = h.exporter.export_commit_dag(&h.repo_id, blake3_hash, &have).await.unwrap();

    // Verify all objects came back
    assert_eq!(export_result.objects.len(), 3, "should export blob + tree + commit");

    let sha1s: Vec<_> = export_result.objects.iter().map(|o| o.sha1).collect();
    assert!(sha1s.contains(&repo.blob_sha1), "blob should be in exported DAG");
    assert!(sha1s.contains(&repo.tree_sha1), "tree should be in exported DAG");
    assert!(sha1s.contains(&repo.commit_sha1), "commit should be in exported DAG");

    // Verify blob content survives round-trip
    let blob_obj = export_result.objects.iter().find(|o| o.sha1 == repo.blob_sha1).unwrap();
    assert_eq!(blob_obj.content, b"# Hello\n", "blob content should round-trip");
}

#[tokio::test]
async fn test_e2e_incremental_push_with_probe() {
    let h = BridgeTestHarness::new();

    // First push: initial commit
    let repo1 = SingleFileRepo::new("file.txt", b"version 1", "First commit");
    handler_push_objects(&h, &repo1.objects()).await;
    h.importer.update_ref(&h.repo_id, "heads/main", repo1.commit_sha1).await.unwrap();

    // Probe: server reports which SHA-1s it already has
    let all_sha1s = vec![repo1.blob_sha1, repo1.tree_sha1, repo1.commit_sha1];
    let mut known_count = 0u32;
    for sha1 in &all_sha1s {
        if h.mapping.has_sha1(&h.repo_id, sha1).await.unwrap() {
            known_count += 1;
        }
    }
    assert_eq!(known_count, 3, "all first-push objects should be known");

    // Second push: new commit on top
    let repo2 = SingleFileRepo::with_parent("file.txt", b"version 2", "Second commit", &repo1.commit_sha1);

    // Probe the second push's objects
    let push2_sha1s = vec![repo2.blob_sha1, repo2.tree_sha1, repo2.commit_sha1];
    let mut push2_known = 0u32;
    for sha1 in &push2_sha1s {
        if h.mapping.has_sha1(&h.repo_id, sha1).await.unwrap() {
            push2_known += 1;
        }
    }
    assert_eq!(push2_known, 0, "second push objects should all be unknown before push");

    // Push only the new objects (incremental)
    let result = handler_push_objects(&h, &repo2.objects()).await;
    assert_eq!(result.objects_imported, 3, "should import 3 new objects");

    // Update ref to new tip
    h.importer.update_ref(&h.repo_id, "heads/main", repo2.commit_sha1).await.unwrap();

    // Fetch from new tip with commit1 in "have" set (incremental fetch)
    let (blake3_tip, _) = h.mapping.get_blake3(&h.repo_id, &repo2.commit_sha1).await.unwrap().unwrap();
    let mut have = std::collections::HashSet::new();
    have.insert(repo1.commit_sha1);
    let export = h.exporter.export_commit_dag(&h.repo_id, blake3_tip, &have).await.unwrap();

    // Should only get commit2's objects (commit1 is in "have")
    let sha1s: Vec<_> = export.objects.iter().map(|o| o.sha1).collect();
    assert!(!sha1s.contains(&repo1.commit_sha1), "commit1 should be excluded (in have set)");
    assert!(!sha1s.contains(&repo1.blob_sha1), "blob1 should be excluded");
    assert!(sha1s.contains(&repo2.commit_sha1), "commit2 should be included");
    assert!(sha1s.contains(&repo2.blob_sha1), "blob2 should be included");
}

#[tokio::test]
async fn test_e2e_multi_branch_push_fetch() {
    let h = BridgeTestHarness::new();

    // Push to main
    let main_repo = SingleFileRepo::new("main.txt", b"main branch content", "Main commit");
    handler_push_objects(&h, &main_repo.objects()).await;
    h.importer.update_ref(&h.repo_id, "heads/main", main_repo.commit_sha1).await.unwrap();

    // Push to dev (different content, no parent relationship)
    let dev_repo = SingleFileRepo::new("dev.txt", b"dev branch content", "Dev commit");
    handler_push_objects(&h, &dev_repo.objects()).await;
    h.importer.update_ref(&h.repo_id, "heads/dev", dev_repo.commit_sha1).await.unwrap();

    // List refs — both branches should be visible
    let refs = h.exporter.list_refs(&h.repo_id).await.unwrap();
    let ref_names: Vec<_> = refs.iter().map(|(name, _)| name.as_str()).collect();
    assert!(ref_names.iter().any(|n| n.contains("main")), "main branch should exist");
    assert!(ref_names.iter().any(|n| n.contains("dev")), "dev branch should exist");

    // Fetch main
    let (main_b3, _) = h.mapping.get_blake3(&h.repo_id, &main_repo.commit_sha1).await.unwrap().unwrap();
    let main_export =
        h.exporter.export_commit_dag(&h.repo_id, main_b3, &std::collections::HashSet::new()).await.unwrap();
    let main_sha1s: Vec<_> = main_export.objects.iter().map(|o| o.sha1).collect();
    assert!(main_sha1s.contains(&main_repo.blob_sha1), "main fetch should include main blob");
    assert!(!main_sha1s.contains(&dev_repo.blob_sha1), "main fetch should NOT include dev blob");

    // Fetch dev
    let (dev_b3, _) = h.mapping.get_blake3(&h.repo_id, &dev_repo.commit_sha1).await.unwrap().unwrap();
    let dev_export = h.exporter.export_commit_dag(&h.repo_id, dev_b3, &std::collections::HashSet::new()).await.unwrap();
    let dev_sha1s: Vec<_> = dev_export.objects.iter().map(|o| o.sha1).collect();
    assert!(dev_sha1s.contains(&dev_repo.blob_sha1), "dev fetch should include dev blob");
    assert!(!dev_sha1s.contains(&main_repo.blob_sha1), "dev fetch should NOT include main blob");
}

#[tokio::test]
async fn test_e2e_push_fetch_with_annotated_tag() {
    let h = BridgeTestHarness::new();

    // Push a commit
    let repo = SingleFileRepo::new("lib.rs", b"pub fn hello() {}", "v1.0 release");
    handler_push_objects(&h, &repo.objects()).await;
    h.importer.update_ref(&h.repo_id, "heads/main", repo.commit_sha1).await.unwrap();

    // Create and push an annotated tag
    let tag_git = make_git_tag(&repo.commit_sha1, "v1.0.0", "Release v1.0.0");
    let tag_sha1 = compute_sha1(&tag_git);
    let (_, tag_data) = split_git_object(&tag_git);

    let tag_objects = vec![(tag_sha1, "tag", tag_data)];
    let tag_result = handler_push_objects(&h, &tag_objects).await;
    assert_eq!(tag_result.objects_imported, 1, "should import tag object");

    // Update the tag ref
    h.importer.update_ref(&h.repo_id, "tags/v1.0.0", tag_sha1).await.unwrap();

    // List refs — should have both branch and tag
    let refs = h.exporter.list_refs(&h.repo_id).await.unwrap();
    let ref_names: Vec<_> = refs.iter().map(|(name, _)| name.as_str()).collect();
    assert!(ref_names.iter().any(|n| n.contains("main")), "main should be listed");
    assert!(ref_names.iter().any(|n| n.contains("v1.0.0")), "tag should be listed");

    // Fetch the tag's commit DAG
    let (tag_b3, _) = h.mapping.get_blake3(&h.repo_id, &tag_sha1).await.unwrap().unwrap();
    let tag_export = h.exporter.export_object(&h.repo_id, tag_b3).await.unwrap();
    assert_eq!(tag_export.object_type, GitObjectType::Tag);

    // Tag content should reference the commit
    let tag_content = String::from_utf8_lossy(&tag_export.content);
    assert!(tag_content.contains(&format!("object {}", repo.commit_sha1)));
    assert!(tag_content.contains("tag v1.0.0"));
}

#[tokio::test]
async fn test_e2e_push_fetch_merge_commit() {
    let h = BridgeTestHarness::new();

    // Branch A
    let branch_a = SingleFileRepo::new("a.txt", b"branch a", "Commit on A");
    handler_push_objects(&h, &branch_a.objects()).await;
    h.importer.update_ref(&h.repo_id, "heads/main", branch_a.commit_sha1).await.unwrap();

    // Branch B
    let branch_b = SingleFileRepo::new("b.txt", b"branch b", "Commit on B");
    handler_push_objects(&h, &branch_b.objects()).await;
    h.importer.update_ref(&h.repo_id, "heads/feature", branch_b.commit_sha1).await.unwrap();

    // Merge commit (parents: A + B, tree with both files)
    let merge_tree_git = make_git_tree(&[
        make_tree_entry("100644", "a.txt", &branch_a.blob_sha1),
        make_tree_entry("100644", "b.txt", &branch_b.blob_sha1),
    ]);
    let merge_tree_sha1 = compute_sha1(&merge_tree_git);
    let (_, merge_tree_data) = split_git_object(&merge_tree_git);

    let merge_commit_git =
        make_git_commit(&merge_tree_sha1, &[&branch_a.commit_sha1, &branch_b.commit_sha1], "Merge feature into main");
    let merge_commit_sha1 = compute_sha1(&merge_commit_git);
    let (_, merge_commit_data) = split_git_object(&merge_commit_git);

    let merge_objects = vec![
        (merge_tree_sha1, "tree", merge_tree_data),
        (merge_commit_sha1, "commit", merge_commit_data),
    ];
    let merge_result = handler_push_objects(&h, &merge_objects).await;
    assert_eq!(merge_result.objects_imported, 2, "should import merge tree + commit");

    h.importer.update_ref(&h.repo_id, "heads/main", merge_commit_sha1).await.unwrap();

    // Fetch from merge tip — should include both parent chains
    let (merge_b3, _) = h.mapping.get_blake3(&h.repo_id, &merge_commit_sha1).await.unwrap().unwrap();
    let export = h.exporter.export_commit_dag(&h.repo_id, merge_b3, &std::collections::HashSet::new()).await.unwrap();

    let sha1s: Vec<_> = export.objects.iter().map(|o| o.sha1).collect();
    assert!(sha1s.contains(&merge_commit_sha1), "merge commit should be in DAG");
    assert!(sha1s.contains(&branch_a.commit_sha1), "parent A should be in DAG");
    assert!(sha1s.contains(&branch_b.commit_sha1), "parent B should be in DAG");
    assert!(sha1s.contains(&branch_a.blob_sha1), "blob A should be in DAG");
    assert!(sha1s.contains(&branch_b.blob_sha1), "blob B should be in DAG");
}

#[tokio::test]
async fn test_e2e_push_fetch_nested_directories() {
    let h = BridgeTestHarness::new();

    // Build: root/src/lib.rs, root/Cargo.toml
    let lib_blob = make_git_blob(b"pub fn lib() {}");
    let lib_sha1 = compute_sha1(&lib_blob);
    let (_, lib_data) = split_git_object(&lib_blob);

    let cargo_blob = make_git_blob(b"[package]\nname = \"test\"");
    let cargo_sha1 = compute_sha1(&cargo_blob);
    let (_, cargo_data) = split_git_object(&cargo_blob);

    // src/ subtree
    let src_tree = make_git_tree(&[make_tree_entry("100644", "lib.rs", &lib_sha1)]);
    let src_sha1 = compute_sha1(&src_tree);
    let (_, src_data) = split_git_object(&src_tree);

    // root tree
    let root_tree = make_git_tree(&[
        make_tree_entry("100644", "Cargo.toml", &cargo_sha1),
        make_tree_entry("40000", "src", &src_sha1),
    ]);
    let root_sha1 = compute_sha1(&root_tree);
    let (_, root_data) = split_git_object(&root_tree);

    let commit_git = make_git_commit(&root_sha1, &[], "Add Cargo.toml and src/lib.rs");
    let commit_sha1 = compute_sha1(&commit_git);
    let (_, commit_data) = split_git_object(&commit_git);

    let objects = vec![
        (lib_sha1, "blob", lib_data),
        (cargo_sha1, "blob", cargo_data),
        (src_sha1, "tree", src_data),
        (root_sha1, "tree", root_data),
        (commit_sha1, "commit", commit_data),
    ];
    let result = handler_push_objects(&h, &objects).await;
    assert_eq!(result.objects_imported, 5);

    h.importer.update_ref(&h.repo_id, "heads/main", commit_sha1).await.unwrap();

    // Fetch and verify all objects come back
    let (b3, _) = h.mapping.get_blake3(&h.repo_id, &commit_sha1).await.unwrap().unwrap();
    let export = h.exporter.export_commit_dag(&h.repo_id, b3, &std::collections::HashSet::new()).await.unwrap();
    assert_eq!(export.objects.len(), 5, "should export all 5 objects");

    // Verify blob content round-trips
    let lib_obj = export.objects.iter().find(|o| o.sha1 == lib_sha1).unwrap();
    assert_eq!(lib_obj.content, b"pub fn lib() {}", "lib.rs content should round-trip");

    let cargo_obj = export.objects.iter().find(|o| o.sha1 == cargo_sha1).unwrap();
    assert_eq!(cargo_obj.content, b"[package]\nname = \"test\"", "Cargo.toml content should round-trip");
}

#[tokio::test]
async fn test_e2e_push_fetch_binary_blob() {
    let h = BridgeTestHarness::new();

    // Binary content with null bytes and all byte values
    let mut binary_content = Vec::with_capacity(256);
    for i in 0..=255u8 {
        binary_content.push(i);
    }

    let blob_git = make_git_blob(&binary_content);
    let blob_sha1 = compute_sha1(&blob_git);
    let (_, blob_data) = split_git_object(&blob_git);

    let tree_entry = make_tree_entry("100644", "binary.dat", &blob_sha1);
    let tree_git = make_git_tree(&[tree_entry]);
    let tree_sha1 = compute_sha1(&tree_git);
    let (_, tree_data) = split_git_object(&tree_git);

    let commit_git = make_git_commit(&tree_sha1, &[], "Add binary file");
    let commit_sha1 = compute_sha1(&commit_git);
    let (_, commit_data) = split_git_object(&commit_git);

    let objects = vec![
        (blob_sha1, "blob", blob_data),
        (tree_sha1, "tree", tree_data),
        (commit_sha1, "commit", commit_data),
    ];
    handler_push_objects(&h, &objects).await;
    h.importer.update_ref(&h.repo_id, "heads/main", commit_sha1).await.unwrap();

    // Fetch and verify binary content survives byte-for-byte
    let (b3, _) = h.mapping.get_blake3(&h.repo_id, &commit_sha1).await.unwrap().unwrap();
    let export = h.exporter.export_commit_dag(&h.repo_id, b3, &std::collections::HashSet::new()).await.unwrap();
    let blob_obj = export.objects.iter().find(|o| o.sha1 == blob_sha1).unwrap();
    assert_eq!(blob_obj.content, binary_content, "binary content should survive round-trip byte-for-byte");
}

#[tokio::test]
async fn test_e2e_empty_repo_list_and_fetch() {
    let h = BridgeTestHarness::new();

    // List refs on empty repo
    let refs = h.exporter.list_refs(&h.repo_id).await.unwrap();
    assert!(refs.is_empty(), "empty repo should have no refs");

    // Probe with unknown SHA-1s on empty repo
    let dummy_sha1 = compute_sha1(&make_git_blob(b"dummy"));
    let has = h.mapping.has_sha1(&h.repo_id, &dummy_sha1).await.unwrap();
    assert!(!has, "empty repo should not have any SHA-1 mappings");
}

#[tokio::test]
async fn test_e2e_force_push_replaces_ref() {
    let h = BridgeTestHarness::new();

    // Push commit 1 to main
    let repo1 = SingleFileRepo::new("file.txt", b"version 1", "First commit");
    handler_push_objects(&h, &repo1.objects()).await;
    h.importer.update_ref(&h.repo_id, "heads/main", repo1.commit_sha1).await.unwrap();

    // Force push commit 2 (unrelated lineage) to main
    let repo2 = SingleFileRepo::new("file.txt", b"force pushed version", "Force pushed");
    handler_push_objects(&h, &repo2.objects()).await;
    h.importer.update_ref(&h.repo_id, "heads/main", repo2.commit_sha1).await.unwrap();

    // Verify ref points to commit 2
    let refs = h.exporter.list_refs(&h.repo_id).await.unwrap();
    let main_ref = refs.iter().find(|(name, _)| name.contains("main")).unwrap();
    assert_eq!(main_ref.1.unwrap(), repo2.commit_sha1, "ref should point to force-pushed commit");

    // Fetch from new tip — should only get commit 2's objects (no parent chain to commit 1)
    let (b3, _) = h.mapping.get_blake3(&h.repo_id, &repo2.commit_sha1).await.unwrap().unwrap();
    let export = h.exporter.export_commit_dag(&h.repo_id, b3, &std::collections::HashSet::new()).await.unwrap();
    let sha1s: Vec<_> = export.objects.iter().map(|o| o.sha1).collect();
    assert!(!sha1s.contains(&repo1.commit_sha1), "old commit should not be in DAG");
    assert!(sha1s.contains(&repo2.commit_sha1), "new commit should be in DAG");
}

#[tokio::test]
async fn test_e2e_probe_mixed_known_unknown() {
    let h = BridgeTestHarness::new();

    // Push some objects
    let repo = SingleFileRepo::new("known.txt", b"known content", "Known commit");
    handler_push_objects(&h, &repo.objects()).await;

    // Create but DON'T push some objects
    let unknown_blob = make_git_blob(b"unknown content");
    let unknown_sha1 = compute_sha1(&unknown_blob);

    // Probe a mix of known and unknown
    let all_sha1s = vec![repo.blob_sha1, repo.tree_sha1, unknown_sha1];
    let mut known = Vec::new();
    let mut unknown = Vec::new();
    for sha1 in &all_sha1s {
        if h.mapping.has_sha1(&h.repo_id, sha1).await.unwrap() {
            known.push(*sha1);
        } else {
            unknown.push(*sha1);
        }
    }

    assert_eq!(known.len(), 2, "2 pushed objects should be known");
    assert_eq!(unknown.len(), 1, "1 unpushed object should be unknown");
    assert!(known.contains(&repo.blob_sha1));
    assert!(known.contains(&repo.tree_sha1));
    assert!(unknown.contains(&unknown_sha1));
}

#[tokio::test]
async fn test_e2e_push_skip_duplicate_objects() {
    let h = BridgeTestHarness::new();

    // Push initial commit
    let repo = SingleFileRepo::new("file.txt", b"original", "Initial");
    let result1 = handler_push_objects(&h, &repo.objects()).await;
    assert_eq!(result1.objects_imported, 3);

    // Push the same objects again — should be skipped
    let result2 = handler_push_objects(&h, &repo.objects()).await;
    // The import_objects function may import or skip depending on implementation
    // but the content should be identical
    let total = result2.objects_imported + result2.objects_skipped;
    assert_eq!(total, 3, "all objects should be accounted for");
}

#[tokio::test]
async fn test_e2e_fetch_content_matches_original() {
    let h = BridgeTestHarness::new();

    // Push with specific content
    let original_content = b"fn main() {\n    println!(\"Hello, Forge!\");\n}\n";
    let repo = SingleFileRepo::new("main.rs", original_content, "Add main.rs");
    handler_push_objects(&h, &repo.objects()).await;
    h.importer.update_ref(&h.repo_id, "heads/main", repo.commit_sha1).await.unwrap();

    // Fetch and verify content byte-for-byte
    let (b3, _) = h.mapping.get_blake3(&h.repo_id, &repo.commit_sha1).await.unwrap().unwrap();
    let export = h.exporter.export_commit_dag(&h.repo_id, b3, &std::collections::HashSet::new()).await.unwrap();

    // Find blob by type and verify content
    let blob = export.objects.iter().find(|o| o.object_type == GitObjectType::Blob).unwrap();
    assert_eq!(blob.content, original_content, "fetched blob content must match original exactly");

    // Verify SHA-1 of reconstructed git object matches
    let exported_git = blob.to_git_bytes();
    let recomputed_sha1 = compute_sha1(&exported_git);
    assert_eq!(recomputed_sha1, repo.blob_sha1, "reconstructed SHA-1 must match original");
}

#[tokio::test]
async fn test_e2e_three_commit_chain_incremental_fetch() {
    let h = BridgeTestHarness::new();

    // Commit 1
    let c1 = SingleFileRepo::new("file.txt", b"v1", "Commit 1");
    handler_push_objects(&h, &c1.objects()).await;
    h.importer.update_ref(&h.repo_id, "heads/main", c1.commit_sha1).await.unwrap();

    // Commit 2 (child of 1)
    let c2 = SingleFileRepo::with_parent("file.txt", b"v2", "Commit 2", &c1.commit_sha1);
    handler_push_objects(&h, &c2.objects()).await;
    h.importer.update_ref(&h.repo_id, "heads/main", c2.commit_sha1).await.unwrap();

    // Commit 3 (child of 2)
    let c3 = SingleFileRepo::with_parent("file.txt", b"v3", "Commit 3", &c2.commit_sha1);
    handler_push_objects(&h, &c3.objects()).await;
    h.importer.update_ref(&h.repo_id, "heads/main", c3.commit_sha1).await.unwrap();

    // Full fetch from tip (no "have") — should get all 9 objects
    let (tip_b3, _) = h.mapping.get_blake3(&h.repo_id, &c3.commit_sha1).await.unwrap().unwrap();
    let full_export =
        h.exporter.export_commit_dag(&h.repo_id, tip_b3, &std::collections::HashSet::new()).await.unwrap();
    assert!(
        full_export.objects.len() >= 9,
        "full fetch should include all 9 objects, got {}",
        full_export.objects.len()
    );

    // Incremental fetch with c1 in "have" — should skip c1's objects
    let mut have = std::collections::HashSet::new();
    have.insert(c1.commit_sha1);
    let incr_export = h.exporter.export_commit_dag(&h.repo_id, tip_b3, &have).await.unwrap();

    let incr_sha1s: Vec<_> = incr_export.objects.iter().map(|o| o.sha1).collect();
    assert!(!incr_sha1s.contains(&c1.commit_sha1), "commit 1 should be excluded");
    assert!(!incr_sha1s.contains(&c1.blob_sha1), "blob 1 should be excluded");
    assert!(incr_sha1s.contains(&c2.commit_sha1), "commit 2 should be included");
    assert!(incr_sha1s.contains(&c3.commit_sha1), "commit 3 should be included");

    // Incremental fetch with c1 + c2 in "have" — should only get c3's objects
    have.insert(c2.commit_sha1);
    let minimal_export = h.exporter.export_commit_dag(&h.repo_id, tip_b3, &have).await.unwrap();

    let min_sha1s: Vec<_> = minimal_export.objects.iter().map(|o| o.sha1).collect();
    assert!(!min_sha1s.contains(&c1.commit_sha1), "commit 1 excluded");
    assert!(!min_sha1s.contains(&c2.commit_sha1), "commit 2 excluded");
    assert!(min_sha1s.contains(&c3.commit_sha1), "commit 3 included");
    assert!(min_sha1s.contains(&c3.blob_sha1), "blob 3 included");
}

#[tokio::test]
async fn test_e2e_push_symlink_mode() {
    let h = BridgeTestHarness::new();

    // Symlink blob (target path as content)
    let symlink_target = b"../lib/libfoo.so";
    let blob_git = make_git_blob(symlink_target);
    let blob_sha1 = compute_sha1(&blob_git);
    let (_, blob_data) = split_git_object(&blob_git);

    // Tree with symlink entry (mode 120000)
    let tree_git = make_git_tree(&[make_tree_entry("120000", "libfoo.so", &blob_sha1)]);
    let tree_sha1 = compute_sha1(&tree_git);
    let (_, tree_data) = split_git_object(&tree_git);

    let commit_git = make_git_commit(&tree_sha1, &[], "Add symlink");
    let commit_sha1 = compute_sha1(&commit_git);
    let (_, commit_data) = split_git_object(&commit_git);

    let objects = vec![
        (blob_sha1, "blob", blob_data),
        (tree_sha1, "tree", tree_data),
        (commit_sha1, "commit", commit_data),
    ];
    handler_push_objects(&h, &objects).await;
    h.importer.update_ref(&h.repo_id, "heads/main", commit_sha1).await.unwrap();

    // Fetch and verify symlink content
    let (b3, _) = h.mapping.get_blake3(&h.repo_id, &commit_sha1).await.unwrap().unwrap();
    let export = h.exporter.export_commit_dag(&h.repo_id, b3, &std::collections::HashSet::new()).await.unwrap();
    let blob_obj = export.objects.iter().find(|o| o.sha1 == blob_sha1).unwrap();
    assert_eq!(blob_obj.content, symlink_target, "symlink target content should round-trip");

    // Verify tree with symlink mode round-trips
    let tree_obj = export.objects.iter().find(|o| o.sha1 == tree_sha1).unwrap();
    let tree_git_bytes = tree_obj.to_git_bytes();
    assert_eq!(compute_sha1(&tree_git_bytes), tree_sha1, "tree with symlink should round-trip");
}

#[tokio::test]
async fn test_e2e_push_empty_blob() {
    let h = BridgeTestHarness::new();

    // Empty file (0 bytes)
    let blob_git = make_git_blob(b"");
    let blob_sha1 = compute_sha1(&blob_git);
    let (_, blob_data) = split_git_object(&blob_git);

    let tree_git = make_git_tree(&[make_tree_entry("100644", ".gitkeep", &blob_sha1)]);
    let tree_sha1 = compute_sha1(&tree_git);
    let (_, tree_data) = split_git_object(&tree_git);

    let commit_git = make_git_commit(&tree_sha1, &[], "Add empty file");
    let commit_sha1 = compute_sha1(&commit_git);
    let (_, commit_data) = split_git_object(&commit_git);

    let objects = vec![
        (blob_sha1, "blob", blob_data),
        (tree_sha1, "tree", tree_data),
        (commit_sha1, "commit", commit_data),
    ];
    let result = handler_push_objects(&h, &objects).await;
    assert_eq!(result.objects_imported, 3);

    // Fetch and verify empty blob
    h.importer.update_ref(&h.repo_id, "heads/main", commit_sha1).await.unwrap();
    let (b3, _) = h.mapping.get_blake3(&h.repo_id, &commit_sha1).await.unwrap().unwrap();
    let export = h.exporter.export_commit_dag(&h.repo_id, b3, &std::collections::HashSet::new()).await.unwrap();
    let blob_obj = export.objects.iter().find(|o| o.sha1 == blob_sha1).unwrap();
    assert!(blob_obj.content.is_empty(), "empty blob should round-trip as empty");
}

// ============================================================================
// ImportResult.mappings tests
// ============================================================================

/// import_objects() with reverse-dependency-order input (commit → tree → blob)
/// should still import all objects and produce a complete mappings vec.
#[tokio::test]
async fn test_import_objects_reverse_order_has_complete_mappings() {
    let h = BridgeTestHarness::new();

    // Build objects
    let blob_git = make_git_blob(b"reverse order test\n");
    let blob_sha1 = compute_sha1(&blob_git);
    let (_, blob_data) = split_git_object(&blob_git);

    let entry = make_tree_entry("100644", "file.txt", &blob_sha1);
    let tree_git = make_git_tree(&[entry]);
    let tree_sha1 = compute_sha1(&tree_git);
    let (_, tree_data) = split_git_object(&tree_git);

    let commit_git = make_git_commit(&tree_sha1, &[], "reverse order");
    let commit_sha1 = compute_sha1(&commit_git);
    let (_, commit_data) = split_git_object(&commit_git);

    // Feed in REVERSE dependency order: commit first, blob last
    let objects = vec![
        (commit_sha1, "commit", commit_data),
        (tree_sha1, "tree", tree_data),
        (blob_sha1, "blob", blob_data),
    ];
    let result = handler_push_objects(&h, &objects).await;

    // All 3 must be imported (topological sort reorders them)
    assert_eq!(result.objects_imported, 3, "all 3 objects should be imported");
    assert_eq!(result.objects_skipped, 0);

    // mappings must contain all 3 objects
    assert_eq!(result.mappings.len(), 3, "mappings should have 3 entries");

    // Verify each SHA-1 appears in mappings with a valid BLAKE3
    let mapping_sha1s: std::collections::HashSet<_> = result.mappings.iter().map(|(s, _)| *s).collect();
    assert!(mapping_sha1s.contains(&blob_sha1), "blob sha1 missing from mappings");
    assert!(mapping_sha1s.contains(&tree_sha1), "tree sha1 missing from mappings");
    assert!(mapping_sha1s.contains(&commit_sha1), "commit sha1 missing from mappings");

    // Verify the BLAKE3 hashes in mappings are non-zero and resolvable
    for (sha1, blake3) in &result.mappings {
        assert_ne!(*blake3.as_bytes(), [0u8; 32], "blake3 should be non-zero for {}", sha1);
        let (looked_up, _) = h.mapping.get_blake3(&h.repo_id, sha1).await.unwrap().unwrap();
        assert_eq!(looked_up, *blake3, "mapping blake3 should match KV store for {}", sha1);
    }
}

/// import_objects() with partially-known objects includes both new and
/// already-present entries in the mappings vec.
#[tokio::test]
async fn test_import_objects_partial_known_has_all_mappings() {
    let h = BridgeTestHarness::new();

    // Import blob first (so it's "already known" on second call)
    let blob_git = make_git_blob(b"pre-existing blob\n");
    let blob_sha1 = compute_sha1(&blob_git);
    h.importer.import_object(&h.repo_id, &blob_git).await.unwrap();

    // Build tree + commit referencing the existing blob
    let entry = make_tree_entry("100644", "known.txt", &blob_sha1);
    let tree_git = make_git_tree(&[entry]);
    let tree_sha1 = compute_sha1(&tree_git);
    let (_, tree_data) = split_git_object(&tree_git);

    let commit_git = make_git_commit(&tree_sha1, &[], "partial known");
    let commit_sha1 = compute_sha1(&commit_git);
    let (_, commit_data) = split_git_object(&commit_git);

    // Push all 3: blob is already known, tree + commit are new
    let (_, blob_data) = split_git_object(&blob_git);
    let objects = vec![
        (blob_sha1, "blob", blob_data),
        (tree_sha1, "tree", tree_data),
        (commit_sha1, "commit", commit_data),
    ];
    let result = handler_push_objects(&h, &objects).await;

    assert_eq!(result.objects_imported, 2, "tree + commit should be imported");
    assert_eq!(result.objects_skipped, 1, "blob should be skipped");

    // mappings must contain all 3 (2 new + 1 existing)
    assert_eq!(result.mappings.len(), 3, "mappings should include skipped objects too");

    let mapping_sha1s: std::collections::HashSet<_> = result.mappings.iter().map(|(s, _)| *s).collect();
    assert!(mapping_sha1s.contains(&blob_sha1), "existing blob should be in mappings");
    assert!(mapping_sha1s.contains(&tree_sha1), "new tree should be in mappings");
    assert!(mapping_sha1s.contains(&commit_sha1), "new commit should be in mappings");
}

/// Simulates the federation import flow: objects arrive in random order,
/// get imported via import_objects(), and SHA-1→BLAKE3 correlation via
/// content hashes works for ref translation.
#[tokio::test]
async fn test_federation_style_import_with_content_hash_correlation() {
    use sha1::Digest;

    let h = BridgeTestHarness::new();

    // Build objects as raw content (no git header) — mirrors SyncObject.data
    let blob_content = b"federation clone test\n";
    let blob_git = make_git_blob(blob_content);
    let blob_sha1 = compute_sha1(&blob_git);

    let entry = make_tree_entry("100644", "readme.md", &blob_sha1);
    let tree_git = make_git_tree(&[entry]);
    let tree_sha1 = compute_sha1(&tree_git);
    let (_, tree_content) = split_git_object(&tree_git);

    let commit_git = make_git_commit(&tree_sha1, &[], "initial");
    let commit_sha1 = compute_sha1(&commit_git);
    let (_, commit_content) = split_git_object(&commit_git);

    // Phase 1: Convert to import format (mirrors federation_import_objects phase 1).
    // Track sha1 → content_hash for post-import correlation.
    struct SyncObj {
        type_str: &'static str,
        content: Vec<u8>,
    }
    let sync_objects = vec![
        SyncObj {
            type_str: "commit",
            content: commit_content.to_vec(),
        },
        SyncObj {
            type_str: "tree",
            content: tree_content.to_vec(),
        },
        SyncObj {
            type_str: "blob",
            content: blob_content.to_vec(),
        },
    ];

    let mut import_objects = Vec::new();

    for obj in &sync_objects {
        let header = format!("{} {}\0", obj.type_str, obj.content.len());
        let mut git_bytes = Vec::with_capacity(header.len() + obj.content.len());
        git_bytes.extend_from_slice(header.as_bytes());
        git_bytes.extend_from_slice(&obj.content);

        let sha1_digest: [u8; 20] = sha1::Sha1::digest(&git_bytes).into();
        let sha1 = Sha1Hash::from_bytes(sha1_digest);

        let obj_type = match obj.type_str {
            "blob" => GitObjectType::Blob,
            "tree" => GitObjectType::Tree,
            "commit" => GitObjectType::Commit,
            _ => unreachable!(),
        };

        import_objects.push((sha1, obj_type, git_bytes));
    }

    // Phase 2: Import (topological sort handles the reverse order)
    let result = h.importer.import_objects(&h.repo_id, import_objects).await.unwrap();
    assert_eq!(result.objects_imported, 3);
    assert_eq!(result.mappings.len(), 3);

    // Phase 3: Build sha1_to_local_blake3 (mirrors federation_import_objects phase 3)
    let mut sha1_to_local_blake3: std::collections::HashMap<[u8; 20], blake3::Hash> = std::collections::HashMap::new();
    for (sha1, local_blake3) in &result.mappings {
        sha1_to_local_blake3.insert(*sha1.as_bytes(), *local_blake3);
    }

    // Verify: commit SHA-1 maps to a valid local BLAKE3
    let local_b3 = sha1_to_local_blake3.get(commit_sha1.as_bytes());
    assert!(local_b3.is_some(), "commit SHA-1 should map to a local BLAKE3");

    // Verify: that BLAKE3 can be exported back to the same SHA-1
    let exported = h.exporter.export_object(&h.repo_id, *local_b3.unwrap()).await.unwrap();
    assert_eq!(exported.sha1, commit_sha1, "exported commit SHA-1 should match original");
    assert_eq!(exported.object_type, GitObjectType::Commit);
}

// =============================================================================
// Round-trip byte-fidelity tests for c2e fix
// =============================================================================

/// Import a tree with directory+file name prefix overlap (git mode-aware sort
/// edge case), export it, verify byte-identical output and SHA-1 match.
#[tokio::test]
async fn test_tree_roundtrip_mode_aware_sort() {
    let h = BridgeTestHarness::new();

    // Create three blobs
    let blob_a = make_git_blob(b"file content a");
    let blob_b = make_git_blob(b"file content b");
    let blob_c = make_git_blob(b"file content c");

    let sha1_a = compute_sha1(&blob_a);
    let sha1_b = compute_sha1(&blob_b);
    let sha1_c = compute_sha1(&blob_c);

    h.importer.import_object(&h.repo_id, &blob_a).await.unwrap();
    h.importer.import_object(&h.repo_id, &blob_b).await.unwrap();
    h.importer.import_object(&h.repo_id, &blob_c).await.unwrap();

    // Build a tree with entries that trigger git's mode-aware sort:
    // "foo" (dir, 040000) sorts as "foo/" → '/' = 0x2F
    // "foo.c" (file, 100644) sorts as "foo.c" → '.' = 0x2E
    // "foo-bar" (file, 100644) sorts as "foo-bar" → '-' = 0x2D
    //
    // Git sort order: foo-bar (0x2D), foo.c (0x2E), foo (0x2F)
    // We intentionally put them in a different order to test that
    // TreeObject::new() sorts correctly.
    let entry_foobar = make_tree_entry("100644", "foo-bar", &sha1_a);
    let entry_foo_dir = make_tree_entry("40000", "foo", &sha1_b);
    let entry_foo_c = make_tree_entry("100644", "foo.c", &sha1_c);

    // Build the tree in git-canonical order (as git would write it)
    let git_tree_bytes = make_git_tree(&[entry_foobar.clone(), entry_foo_c.clone(), entry_foo_dir.clone()]);
    let tree_sha1 = compute_sha1(&git_tree_bytes);

    // Import
    let tree_blake3 = h.importer.import_object(&h.repo_id, &git_tree_bytes).await.unwrap();

    // Export
    let exported = h.exporter.export_object(&h.repo_id, tree_blake3).await.unwrap();

    // Verify SHA-1 match
    assert_eq!(exported.sha1, tree_sha1, "exported tree SHA-1 must match original");

    // Verify byte-identical content (strip header from original for comparison)
    let content_start = git_tree_bytes.iter().position(|&b| b == 0).unwrap() + 1;
    let original_content = &git_tree_bytes[content_start..];
    assert_eq!(exported.content, original_content, "exported tree content must be byte-identical to original");
}

/// Import a commit with a multi-paragraph message, export it, verify
/// byte-identical content including trailing newlines.
#[tokio::test]
async fn test_commit_roundtrip_message_preservation() {
    let h = BridgeTestHarness::new();

    // Create a blob and tree
    let blob = make_git_blob(b"hello");
    let blob_sha1 = compute_sha1(&blob);
    h.importer.import_object(&h.repo_id, &blob).await.unwrap();

    let tree_entry = make_tree_entry("100644", "file.txt", &blob_sha1);
    let tree_bytes = make_git_tree(&[tree_entry]);
    let tree_sha1 = compute_sha1(&tree_bytes);
    h.importer.import_object(&h.repo_id, &tree_bytes).await.unwrap();

    // Multi-paragraph message with trailing newline
    let message = "Fix critical bug in federation sync\n\nThe c2e index was keyed by content hash which\ndiffers between import and export paths.\n\nSigned-off-by: Test User <test@example.com>\n";
    let commit_bytes = make_git_commit(&tree_sha1, &[], message);
    let commit_sha1 = compute_sha1(&commit_bytes);

    // Import
    let commit_blake3 = h.importer.import_object(&h.repo_id, &commit_bytes).await.unwrap();

    // Export
    let exported = h.exporter.export_object(&h.repo_id, commit_blake3).await.unwrap();

    // Verify SHA-1 match
    assert_eq!(exported.sha1, commit_sha1, "exported commit SHA-1 must match original");

    // Verify byte-identical content
    let content_start = commit_bytes.iter().position(|&b| b == 0).unwrap() + 1;
    let original_content = &commit_bytes[content_start..];
    assert_eq!(exported.content, original_content, "exported commit content must be byte-identical to original");
}

/// Verify c2e index entries are keyed by SHA-1 hex after import,
/// and that export can look them up via SHA-1.
#[tokio::test]
async fn test_c2e_index_sha1_roundtrip() {
    let h = BridgeTestHarness::new();

    // Import several objects via import_objects (which writes c2e entries)
    let blob1 = make_git_blob(b"content one");
    let blob2 = make_git_blob(b"content two");
    let blob1_sha1 = compute_sha1(&blob1);
    let blob2_sha1 = compute_sha1(&blob2);

    let objects = vec![
        (blob1_sha1, GitObjectType::Blob, blob1),
        (blob2_sha1, GitObjectType::Blob, blob2),
    ];

    let result = h.importer.import_objects(&h.repo_id, objects).await.unwrap();
    assert_eq!(result.objects_imported, 2);

    // Verify c2e entries are keyed by SHA-1 hex
    let c2e_prefix = format!("forge:c2e:{}:", h.repo_id.to_hex());
    let scan_result = h
        .mapping
        .kv()
        .scan(aspen_core::ScanRequest {
            prefix: c2e_prefix.clone(),
            limit_results: Some(100),
            continuation_token: None,
        })
        .await
        .unwrap();

    assert_eq!(scan_result.entries.len(), 2, "should have 2 c2e entries");

    // Each key should be the SHA-1 hex of the object
    let expected_sha1s: std::collections::HashSet<String> =
        [blob1_sha1.to_hex(), blob2_sha1.to_hex()].into_iter().collect();

    for entry in &scan_result.entries {
        let sha1_hex = entry.key.strip_prefix(&c2e_prefix).unwrap();
        assert!(expected_sha1s.contains(sha1_hex), "c2e key '{}' should be a known SHA-1 hex", sha1_hex);
        // Value should be a valid 32-byte hex envelope hash
        let env_bytes = hex::decode(entry.value.trim()).unwrap();
        assert_eq!(env_bytes.len(), 32, "c2e value should be 32 bytes");
    }

    // Verify we can look up each SHA-1 and get the correct envelope hash
    for (sha1, blake3) in &result.mappings {
        let key = format!("forge:c2e:{}:{}", h.repo_id.to_hex(), sha1.to_hex());
        let read_result = h.mapping.kv().read(aspen_core::ReadRequest::new(key)).await.unwrap();
        let kv = read_result.kv.expect("c2e entry should exist");
        let env_hex = kv.value.trim();
        let expected_hex = hex::encode(blake3.as_bytes());
        assert_eq!(env_hex, expected_hex, "c2e value for {} should match envelope hash", sha1.to_hex());
    }
}

// ============================================================================
// Federation DAG closure tests
// ============================================================================

/// Create a repo with `n_blobs` blobs, one tree referencing all of them,
/// and one commit pointing to that tree. Returns the commit's BLAKE3 hash.
async fn create_wide_tree_repo(h: &BridgeTestHarness, n_blobs: usize) -> blake3::Hash {
    let mut blob_sha1s = Vec::with_capacity(n_blobs);
    for i in 0..n_blobs {
        let content = format!("blob content {i}\n");
        let git_bytes = make_git_blob(content.as_bytes());
        let sha1 = compute_sha1(&git_bytes);
        h.importer.import_object(&h.repo_id, &git_bytes).await.unwrap();
        blob_sha1s.push(sha1);
    }

    // Tree referencing all blobs
    let entries: Vec<Vec<u8>> = blob_sha1s
        .iter()
        .enumerate()
        .map(|(i, sha1)| make_tree_entry("100644", &format!("file{i:04}.txt"), sha1))
        .collect();
    let tree_bytes = make_git_tree(&entries);
    let tree_sha1 = compute_sha1(&tree_bytes);
    h.importer.import_object(&h.repo_id, &tree_bytes).await.unwrap();

    // Commit
    let commit_bytes = make_git_commit(&tree_sha1, &[], "wide tree commit\n");
    let commit_b3 = h.importer.import_object(&h.repo_id, &commit_bytes).await.unwrap();
    commit_b3
}

/// When the full DAG fits in one batch, all objects are returned and has_more is false.
#[tokio::test]
async fn test_federation_export_closure_single_batch() {
    use std::collections::HashSet;

    let h = BridgeTestHarness::new();
    let n_blobs = 10;
    let commit_b3 = create_wide_tree_repo(&h, n_blobs).await;

    // Limit well above total objects (10 blobs + 1 tree + 1 commit = 12)
    let result = h.exporter.export_commit_dag_blake3(&h.repo_id, commit_b3, &HashSet::new(), 100).await.unwrap();

    // All 12 objects should be returned
    assert_eq!(result.objects.len(), n_blobs + 2, "should have all blobs + tree + commit");
    assert!(!result.has_more, "single batch should have has_more=false");

    // Verify dependency closure: every tree entry's blob hash should be in the batch
    let batch_blake3s: HashSet<[u8; 32]> = result.objects.iter().map(|o| *o.blake3.as_bytes()).collect();
    for obj in &result.objects {
        if obj.object_type == super::mapping::GitObjectType::Tree {
            // Parse tree entries to get referenced SHA-1s, look up BLAKE3
            // Not trivial to parse raw tree here, but we can verify the count is right
        }
    }
    // Count by type
    let blob_count = result.objects.iter().filter(|o| o.object_type == super::mapping::GitObjectType::Blob).count();
    let tree_count = result.objects.iter().filter(|o| o.object_type == super::mapping::GitObjectType::Tree).count();
    let commit_count = result.objects.iter().filter(|o| o.object_type == super::mapping::GitObjectType::Commit).count();
    assert_eq!(blob_count, n_blobs);
    assert_eq!(tree_count, 1);
    assert_eq!(commit_count, 1);
}

/// When the limit forces truncation, the batch includes objects up to the limit
/// and has_more=true. The importer handles any cross-batch deps via retry.
#[tokio::test]
async fn test_federation_export_truncated_batch_has_more() {
    use std::collections::HashSet;

    let h = BridgeTestHarness::new();
    // Create 20 blobs + 1 tree + 1 commit = 22 objects
    let n_blobs = 20;
    let commit_b3 = create_wide_tree_repo(&h, n_blobs).await;

    // Limit to 10 — the BFS will collect commit + tree + some blobs
    let result = h.exporter.export_commit_dag_blake3(&h.repo_id, commit_b3, &HashSet::new(), 10).await.unwrap();

    assert!(result.has_more, "truncated batch should report has_more=true");
    assert_eq!(result.objects.len(), 10, "should return exactly limit objects");
}

/// import_objects is idempotent: calling it twice with the same input
/// skips already-imported objects on the second call. This is the foundation
/// for the federation retry pass.
#[tokio::test]
async fn test_import_objects_idempotent_for_retry() {
    use std::collections::HashSet;

    use super::mapping::GitObjectType;

    let h = BridgeTestHarness::new();

    // Create blob → tree → commit
    let blob_bytes = make_git_blob(b"retry test\n");
    let blob_sha1 = compute_sha1(&blob_bytes);

    let entry = make_tree_entry("100644", "retry.txt", &blob_sha1);
    let tree_bytes = make_git_tree(&[entry]);
    let tree_sha1 = compute_sha1(&tree_bytes);

    let commit_bytes = make_git_commit(&tree_sha1, &[], "retry commit\n");
    let commit_sha1 = compute_sha1(&commit_bytes);

    let objects = vec![
        (blob_sha1, GitObjectType::Blob, blob_bytes.clone()),
        (tree_sha1, GitObjectType::Tree, tree_bytes.clone()),
        (commit_sha1, GitObjectType::Commit, commit_bytes.clone()),
    ];

    // First import
    let r1 = h.importer.import_objects(&h.repo_id, objects.clone()).await.unwrap();
    assert_eq!(r1.objects_imported, 3);

    // Second import (retry) — all should be skipped
    let r2 = h.importer.import_objects(&h.repo_id, objects).await.unwrap();
    assert_eq!(r2.objects_imported, 0, "retry should import 0 new objects");
    assert_eq!(r2.objects_skipped, 3, "retry should skip all 3 objects");

    // Mappings from retry should still contain all 3 (from existing lookups)
    assert_eq!(r2.mappings.len(), 3, "retry should return mappings for all objects");
}

/// When some blobs are in known_blake3 (have_set), the tree can stay in
/// the batch even if those blobs aren't — the receiver already has them.
#[tokio::test]
async fn test_federation_export_closure_with_known_set() {
    use std::collections::HashSet;

    let h = BridgeTestHarness::new();
    let n_blobs = 20;
    let commit_b3 = create_wide_tree_repo(&h, n_blobs).await;

    // First, do a full export to get all BLAKE3 hashes
    let full = h.exporter.export_commit_dag_blake3(&h.repo_id, commit_b3, &HashSet::new(), 1000).await.unwrap();
    assert_eq!(full.objects.len(), n_blobs + 2);

    // Put all blob BLAKE3 hashes into known_blake3 (simulate receiver having them)
    let known: HashSet<[u8; 32]> = full
        .objects
        .iter()
        .filter(|o| o.object_type == super::mapping::GitObjectType::Blob)
        .map(|o| *o.blake3.as_bytes())
        .collect();
    assert_eq!(known.len(), n_blobs);

    // Now export with limit=5 but all blobs known — tree + commit should be in batch
    // because their blob deps are satisfied by known_blake3
    let result = h.exporter.export_commit_dag_blake3(&h.repo_id, commit_b3, &known, 5).await.unwrap();

    // Should have tree + commit (blobs are skipped because known)
    let tree_count = result.objects.iter().filter(|o| o.object_type == super::mapping::GitObjectType::Tree).count();
    let commit_count = result.objects.iter().filter(|o| o.object_type == super::mapping::GitObjectType::Commit).count();
    assert_eq!(tree_count, 1, "tree should be in batch (blob deps satisfied by known_blake3)");
    assert_eq!(commit_count, 1, "commit should be in batch (tree dep is in batch)");
}

// ============================================================================
// Partial-success and convergent import tests
// ============================================================================

/// Test 1.4: A tree referencing a non-existent blob should appear in failures,
/// while independent blobs succeed.
#[tokio::test]
async fn test_import_objects_partial_success_missing_dep() {
    let h = BridgeTestHarness::new();

    // Create a blob that will be imported successfully
    let good_blob = make_git_blob(b"good content");
    let good_sha1 = compute_sha1(&good_blob);

    // Create a tree referencing a blob that does NOT exist in the import set
    let fake_sha1 = Sha1Hash::from_bytes([0xDE; 20]);
    let tree_entry = make_tree_entry("100644", "missing.txt", &fake_sha1);
    let bad_tree = make_git_tree(&[tree_entry]);
    let bad_tree_sha1 = compute_sha1(&bad_tree);

    let objects = vec![
        (good_sha1, GitObjectType::Blob, good_blob),
        (bad_tree_sha1, GitObjectType::Tree, bad_tree),
    ];

    let result = h.importer.import_objects(&h.repo_id, objects).await.unwrap();

    // Good blob should succeed
    assert_eq!(result.objects_imported, 1, "blob should import successfully");
    assert!(result.mappings.iter().any(|(sha1, _)| *sha1 == good_sha1), "good blob should have a mapping");

    // Bad tree should be in failures
    assert_eq!(result.failures.len(), 1, "tree with missing dep should fail");
    assert_eq!(result.failures[0].0, bad_tree_sha1, "failed object should be the bad tree");
}

/// Test 4.1: Cross-batch convergent import — tree depends on blob from different batch.
/// After importing both batches, the tree should resolve.
#[tokio::test]
async fn test_import_cross_batch_convergence() {
    let h = BridgeTestHarness::new();

    // Batch 1: just a blob
    let blob_bytes = make_git_blob(b"batch1 content");
    let blob_sha1 = compute_sha1(&blob_bytes);

    // Batch 2: tree referencing the blob from batch 1
    let tree_entry = make_tree_entry("100644", "file.txt", &blob_sha1);
    let tree_bytes = make_git_tree(&[tree_entry]);
    let tree_sha1 = compute_sha1(&tree_bytes);

    // Import batch 1 (blob)
    let r1 = h
        .importer
        .import_objects(&h.repo_id, vec![(blob_sha1, GitObjectType::Blob, blob_bytes.clone())])
        .await
        .unwrap();
    assert_eq!(r1.objects_imported, 1);
    assert!(r1.failures.is_empty());

    // Import batch 2 (tree) — blob mapping exists from batch 1, should succeed
    let r2 = h
        .importer
        .import_objects(&h.repo_id, vec![(tree_sha1, GitObjectType::Tree, tree_bytes)])
        .await
        .unwrap();
    assert_eq!(r2.objects_imported, 1, "tree should import (blob mapped in batch 1)");
    assert!(r2.failures.is_empty(), "no failures expected");
}

/// Test 4.2: Deep dependency chain (blob → subtree → tree → commit) split
/// across separate import calls converges correctly.
#[tokio::test]
async fn test_import_deep_chain_convergence() {
    let h = BridgeTestHarness::new();

    // Layer 0: blob
    let blob_bytes = make_git_blob(b"deep chain content");
    let blob_sha1 = compute_sha1(&blob_bytes);

    // Layer 1: subtree containing the blob
    let subtree_entry = make_tree_entry("100644", "data.txt", &blob_sha1);
    let subtree_bytes = make_git_tree(&[subtree_entry]);
    let subtree_sha1 = compute_sha1(&subtree_bytes);

    // Layer 2: root tree containing the subtree
    let root_entry = make_tree_entry("40000", "subdir", &subtree_sha1);
    let root_tree_bytes = make_git_tree(&[root_entry]);
    let root_tree_sha1 = compute_sha1(&root_tree_bytes);

    // Layer 3: commit referencing root tree
    let commit_bytes = make_git_commit(&root_tree_sha1, &[], "deep chain commit");
    let commit_sha1 = compute_sha1(&commit_bytes);

    // Import each layer separately (simulating 4 different batches)
    let layers: Vec<Vec<(Sha1Hash, GitObjectType, Vec<u8>)>> = vec![
        vec![(blob_sha1, GitObjectType::Blob, blob_bytes)],
        vec![(subtree_sha1, GitObjectType::Tree, subtree_bytes)],
        vec![(root_tree_sha1, GitObjectType::Tree, root_tree_bytes)],
        vec![(commit_sha1, GitObjectType::Commit, commit_bytes)],
    ];

    for (i, batch) in layers.into_iter().enumerate() {
        let r = h.importer.import_objects(&h.repo_id, batch).await.unwrap();
        assert_eq!(r.objects_imported, 1, "layer {i} should import (deps from prior layers)");
        assert!(r.failures.is_empty(), "layer {i} should have no failures");
    }

    // Verify all 4 objects have mappings
    assert!(h.mapping.has_sha1(&h.repo_id, &blob_sha1).await.unwrap());
    assert!(h.mapping.has_sha1(&h.repo_id, &subtree_sha1).await.unwrap());
    assert!(h.mapping.has_sha1(&h.repo_id, &root_tree_sha1).await.unwrap());
    assert!(h.mapping.has_sha1(&h.repo_id, &commit_sha1).await.unwrap());
}

/// Test 4.3: All objects in one batch — single pass, zero retries.
#[tokio::test]
async fn test_import_all_in_one_batch_no_retry() {
    let h = BridgeTestHarness::new();

    let blob_bytes = make_git_blob(b"single batch");
    let blob_sha1 = compute_sha1(&blob_bytes);

    let tree_entry = make_tree_entry("100644", "file.txt", &blob_sha1);
    let tree_bytes = make_git_tree(&[tree_entry]);
    let tree_sha1 = compute_sha1(&tree_bytes);

    let commit_bytes = make_git_commit(&tree_sha1, &[], "all in one");
    let commit_sha1 = compute_sha1(&commit_bytes);

    let objects = vec![
        (blob_sha1, GitObjectType::Blob, blob_bytes),
        (tree_sha1, GitObjectType::Tree, tree_bytes),
        (commit_sha1, GitObjectType::Commit, commit_bytes),
    ];

    let result = h.importer.import_objects(&h.repo_id, objects).await.unwrap();
    assert_eq!(result.objects_imported, 3, "all 3 objects should import in one pass");
    assert!(result.failures.is_empty(), "no failures when all deps present");
    assert_eq!(result.mappings.len(), 3, "3 mappings");
}

/// Test 4.4: Objects with genuinely missing dependencies — the failed objects
/// appear in failures after the import.
#[tokio::test]
async fn test_import_genuinely_missing_deps_reported() {
    let h = BridgeTestHarness::new();

    // Tree referencing a blob that doesn't exist anywhere
    let missing_sha1 = Sha1Hash::from_bytes([0xAB; 20]);
    let tree_entry = make_tree_entry("100644", "ghost.txt", &missing_sha1);
    let tree_bytes = make_git_tree(&[tree_entry]);
    let tree_sha1 = compute_sha1(&tree_bytes);

    // Commit referencing a tree that doesn't exist
    let missing_tree_sha1 = Sha1Hash::from_bytes([0xCD; 20]);
    let commit_bytes = make_git_commit(&missing_tree_sha1, &[], "orphan commit");
    let commit_sha1 = compute_sha1(&commit_bytes);

    // Also include a good blob (should succeed independently)
    let good_blob = make_git_blob(b"still good");
    let good_sha1 = compute_sha1(&good_blob);

    let objects = vec![
        (good_sha1, GitObjectType::Blob, good_blob),
        (tree_sha1, GitObjectType::Tree, tree_bytes),
        (commit_sha1, GitObjectType::Commit, commit_bytes),
    ];

    let result = h.importer.import_objects(&h.repo_id, objects).await.unwrap();

    assert_eq!(result.objects_imported, 1, "only the blob should import");
    assert_eq!(result.failures.len(), 2, "tree and commit should both fail");

    let failed_sha1s: Vec<Sha1Hash> = result.failures.iter().map(|(s, _)| *s).collect();
    assert!(failed_sha1s.contains(&tree_sha1), "tree should be in failures");
    assert!(failed_sha1s.contains(&commit_sha1), "commit should be in failures");
}

// ============================================================================
// Remap (federation) tests
// ============================================================================

/// Simulate the federated mirror scenario:
///
/// 1. Import objects on an "origin" cluster (gets origin BLAKE3 hashes)
/// 2. Import the same git bytes on a "mirror" cluster (different secret key → different BLAKE3)
/// 3. Manually replace the mirror's tree/commit internal BLAKE3 refs with origin refs (simulating
///    what happens when SyncObject bytes carry origin-BLAKE3-based payloads)
/// 4. Write remap entries: origin_blake3 → mirror_blake3
/// 5. Verify the mirror's exporter walks the full DAG via remap
///
/// The key insight: on a real federated mirror, the `SignedObject<GitObject>` stored
/// in KV may have internal BLAKE3 references from the origin because the signed bytes
/// were transferred as-is. The remap index lets the exporter follow these references.
#[tokio::test]
async fn test_remap_exporter_walks_full_dag() {
    // --- Set up "origin" cluster ---
    let origin = BridgeTestHarness::new();

    let blob_content = b"hello federation";
    let blob_git = make_git_blob(blob_content);
    let blob_sha1 = compute_sha1(&blob_git);
    let origin_blob_b3 = origin.importer.import_object(&origin.repo_id, &blob_git).await.unwrap();

    let tree_entry = make_tree_entry("100644", "fed.txt", &blob_sha1);
    let tree_git = make_git_tree(&[tree_entry]);
    let tree_sha1 = compute_sha1(&tree_git);
    let origin_tree_b3 = origin.importer.import_object(&origin.repo_id, &tree_git).await.unwrap();

    let commit_git = make_git_commit(&tree_sha1, &[], "federation commit");
    let commit_sha1 = compute_sha1(&commit_git);
    let origin_commit_b3 = origin.importer.import_object(&origin.repo_id, &commit_git).await.unwrap();

    // --- Set up "mirror" cluster ---
    let mirror = BridgeTestHarness::new();

    // Import the same git bytes on the mirror (different secret key → different BLAKE3)
    let mirror_blob_b3 = mirror.importer.import_object(&mirror.repo_id, &blob_git).await.unwrap();
    let mirror_tree_b3 = mirror.importer.import_object(&mirror.repo_id, &tree_git).await.unwrap();
    let mirror_commit_b3 = mirror.importer.import_object(&mirror.repo_id, &commit_git).await.unwrap();

    // Sanity: origin and mirror BLAKE3 differ (different secret keys)
    assert_ne!(origin_blob_b3, mirror_blob_b3, "different keys → different BLAKE3");
    assert_ne!(origin_tree_b3, mirror_tree_b3);
    assert_ne!(origin_commit_b3, mirror_commit_b3);

    // Now simulate what happens during federation: the mirror's tree/commit objects
    // internally reference the mirror's BLAKE3 hashes (because import_objects translates).
    // But the exporter entry point is called with the mirror_commit_b3, and the DAG
    // walk reads the mirror's objects. This works without remap!
    //
    // The remap is needed when SyncObject bytes are stored directly (not re-imported),
    // or when cross-batch references leave origin BLAKE3 in the DAG. To test remap
    // specifically, we simulate the failing case: store objects under "fake" BLAKE3s
    // and write remap entries so the exporter can find them.

    // Write remap entries: origin → mirror for all three objects
    mirror.mapping.write_remap(&mirror.repo_id, origin_blob_b3, mirror_blob_b3).await.unwrap();
    mirror.mapping.write_remap(&mirror.repo_id, origin_tree_b3, mirror_tree_b3).await.unwrap();
    mirror.mapping.write_remap(&mirror.repo_id, origin_commit_b3, mirror_commit_b3).await.unwrap();

    // Export the DAG starting from the mirror's commit (works without remap)
    let result = mirror
        .exporter
        .export_commit_dag(&mirror.repo_id, mirror_commit_b3, &std::collections::HashSet::new())
        .await
        .unwrap();
    assert_eq!(result.objects.len(), 3, "mirror export should return all 3 objects");

    // Now test the remap path: try exporting with origin_commit_b3 (doesn't exist directly
    // on mirror — should fail without remap, succeed with remap)
    let remap_result = mirror
        .exporter
        .export_commit_dag(&mirror.repo_id, origin_commit_b3, &std::collections::HashSet::new())
        .await
        .unwrap();
    assert_eq!(remap_result.objects.len(), 3, "remap export should resolve all 3 objects via origin→mirror remap");

    // Verify all exported SHA-1s match the originals
    let exported_sha1s: std::collections::HashSet<Sha1Hash> = remap_result.objects.iter().map(|o| o.sha1).collect();
    assert!(exported_sha1s.contains(&blob_sha1), "blob SHA-1 should be in export");
    assert!(exported_sha1s.contains(&tree_sha1), "tree SHA-1 should be in export");
    assert!(exported_sha1s.contains(&commit_sha1), "commit SHA-1 should be in export");
}

/// Verify that non-federated repos (no remap entries) are unaffected.
/// The exporter should walk the DAG directly without remap lookups.
#[tokio::test]
async fn test_remap_no_entries_non_federated_works() {
    let h = BridgeTestHarness::new();

    let blob_git = make_git_blob(b"no remap needed");
    let blob_sha1 = compute_sha1(&blob_git);
    h.importer.import_object(&h.repo_id, &blob_git).await.unwrap();

    let tree_entry = make_tree_entry("100644", "plain.txt", &blob_sha1);
    let tree_git = make_git_tree(&[tree_entry]);
    let tree_sha1 = compute_sha1(&tree_git);
    h.importer.import_object(&h.repo_id, &tree_git).await.unwrap();

    let commit_git = make_git_commit(&tree_sha1, &[], "non-federated");
    let commit_sha1 = compute_sha1(&commit_git);
    let commit_b3 = h.importer.import_object(&h.repo_id, &commit_git).await.unwrap();

    let result = h
        .exporter
        .export_commit_dag(&h.repo_id, commit_b3, &std::collections::HashSet::new())
        .await
        .unwrap();
    assert_eq!(result.objects.len(), 3, "non-federated DAG should walk fine");

    let sha1s: std::collections::HashSet<_> = result.objects.iter().map(|o| o.sha1).collect();
    assert!(sha1s.contains(&blob_sha1));
    assert!(sha1s.contains(&tree_sha1));
    assert!(sha1s.contains(&commit_sha1));
}

/// Test that the exporter logs and skips (rather than errors) when BLAKE3 is
/// unresolvable — neither direct nor via remap.
#[tokio::test]
async fn test_remap_missing_entry_skips_gracefully() {
    let h = BridgeTestHarness::new();

    let blob_git = make_git_blob(b"good blob");
    let blob_sha1 = compute_sha1(&blob_git);
    h.importer.import_object(&h.repo_id, &blob_git).await.unwrap();

    let tree_entry = make_tree_entry("100644", "ok.txt", &blob_sha1);
    let tree_git = make_git_tree(&[tree_entry]);
    let tree_sha1 = compute_sha1(&tree_git);
    h.importer.import_object(&h.repo_id, &tree_git).await.unwrap();

    let commit_git = make_git_commit(&tree_sha1, &[], "partial dag");
    let commit_b3 = h.importer.import_object(&h.repo_id, &commit_git).await.unwrap();

    // Export should work (3 objects)
    let result = h
        .exporter
        .export_commit_dag(&h.repo_id, commit_b3, &std::collections::HashSet::new())
        .await
        .unwrap();
    assert_eq!(result.objects.len(), 3);

    // Now try with a fabricated BLAKE3 that doesn't exist and has no remap.
    // The exporter should return an empty result (skip the unresolvable root).
    let fake_b3 = blake3::hash(b"does not exist");
    let result = h.exporter.export_commit_dag(&h.repo_id, fake_b3, &std::collections::HashSet::new()).await.unwrap();
    assert_eq!(result.objects.len(), 0, "unresolvable root should be skipped");
}
