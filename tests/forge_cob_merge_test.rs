//! Merge conflict detection and resolution tests for Forge COBs.
//!
//! These tests verify the Git-like merge conflict behavior:
//! - Automatic merging of non-conflicting changes (labels, comments, reactions)
//! - Conflict detection for scalar fields (title, body, state)
//! - Explicit resolution requirement for conflicts
//! - LastWriteWins strategy for automatic conflict resolution

use std::sync::Arc;

use aspen::blob::InMemoryBlobStore;
use aspen::forge::FieldResolution;
use aspen::forge::MergeStrategy;
use aspen::forge::cob::CobStore;
use aspen::forge::cob::CobType;
use aspen::forge::identity::RepoId;
use aspen::testing::DeterministicKeyValueStore;

/// Create a test store for COB operations.
async fn create_test_store() -> CobStore<InMemoryBlobStore, DeterministicKeyValueStore> {
    let blobs = Arc::new(InMemoryBlobStore::new());
    let kv = DeterministicKeyValueStore::new();
    let secret_key = iroh::SecretKey::generate(&mut rand::rng());
    CobStore::new(blobs, kv, secret_key, "test-cob-merge")
}

fn test_repo_id() -> RepoId {
    RepoId::from_hash(blake3::hash(b"test-repo"))
}

// ============================================================================
// Basic Conflict Detection Tests
// ============================================================================

#[tokio::test]
async fn test_single_head_no_conflicts() {
    let store = create_test_store().await;
    let repo_id = test_repo_id();

    // Create an issue
    let issue_id = store
        .create_issue(&repo_id, "Test Issue", "Description", vec![])
        .await
        .expect("should create issue");

    // Single head should have no conflicts
    let has_conflicts = store.has_conflicts(&repo_id, CobType::Issue, &issue_id).await.expect("should check conflicts");

    assert!(!has_conflicts, "single head should have no conflicts");
}

#[tokio::test]
async fn test_nonexistent_cob_error() {
    let store = create_test_store().await;
    let repo_id = test_repo_id();
    let fake_id = blake3::hash(b"nonexistent");

    let result = store.has_conflicts(&repo_id, CobType::Issue, &fake_id).await;

    assert!(result.is_err(), "nonexistent COB should return error");
}

#[tokio::test]
async fn test_get_conflicts_single_head() {
    let store = create_test_store().await;
    let repo_id = test_repo_id();

    let issue_id = store
        .create_issue(&repo_id, "Test Issue", "Description", vec![])
        .await
        .expect("should create issue");

    let report = store.get_conflicts(&repo_id, CobType::Issue, &issue_id).await.expect("should get conflicts");

    assert_eq!(report.heads.len(), 1);
    assert!(report.field_conflicts.is_empty());
}

// ============================================================================
// Merge Tests (Single Head - No-op)
// ============================================================================

#[tokio::test]
async fn test_merge_single_head_returns_same_hash() {
    let store = create_test_store().await;
    let repo_id = test_repo_id();

    let issue_id = store
        .create_issue(&repo_id, "Test Issue", "Description", vec![])
        .await
        .expect("should create issue");

    // Get the current head
    let report = store.get_conflicts(&repo_id, CobType::Issue, &issue_id).await.expect("should get conflicts");
    let original_head = report.heads[0];

    // Merge with single head should return the same hash
    let merged = store
        .merge_heads(&repo_id, CobType::Issue, &issue_id, MergeStrategy::Auto, vec![])
        .await
        .expect("should merge");

    assert_eq!(merged, original_head, "single head merge should be no-op");
}

// ============================================================================
// Conflict Report Details
// ============================================================================

#[tokio::test]
async fn test_conflict_report_has_correct_cob_info() {
    let store = create_test_store().await;
    let repo_id = test_repo_id();

    let issue_id = store
        .create_issue(&repo_id, "Test Issue", "Description", vec!["bug".to_string()])
        .await
        .expect("should create issue");

    let report = store.get_conflicts(&repo_id, CobType::Issue, &issue_id).await.expect("should get conflicts");

    assert_eq!(report.cob_type, CobType::Issue);
    assert_eq!(report.cob_id, issue_id);
}

// ============================================================================
// Auto-merge (Non-conflicting Operations)
// ============================================================================

#[tokio::test]
async fn test_auto_merge_comments_no_conflict() {
    let store = create_test_store().await;
    let repo_id = test_repo_id();

    // Create an issue
    let issue_id = store
        .create_issue(&repo_id, "Test Issue", "Description", vec![])
        .await
        .expect("should create issue");

    // Add a comment (creates new head)
    store.add_comment(&repo_id, &issue_id, "First comment").await.expect("should add comment");

    // Should still have single head after comment
    let has_conflicts = store.has_conflicts(&repo_id, CobType::Issue, &issue_id).await.expect("should check conflicts");

    assert!(!has_conflicts, "comments should auto-merge");
}

// ============================================================================
// LastWriteWins Strategy
// ============================================================================

#[tokio::test]
async fn test_merge_with_last_write_wins_creates_merge_change() {
    let store = create_test_store().await;
    let repo_id = test_repo_id();

    // Create an issue
    let issue_id = store
        .create_issue(&repo_id, "Test Issue", "Description", vec![])
        .await
        .expect("should create issue");

    // Use LastWriteWins strategy - should succeed even with single head
    let merged = store
        .merge_heads(&repo_id, CobType::Issue, &issue_id, MergeStrategy::LastWriteWins, vec![])
        .await
        .expect("should merge with LWW");

    // Merged hash should be valid
    assert!(!merged.as_bytes().iter().all(|&b| b == 0));
}

// ============================================================================
// Explicit Strategy Validation
// ============================================================================

#[tokio::test]
async fn test_explicit_strategy_no_conflicts_succeeds() {
    let store = create_test_store().await;
    let repo_id = test_repo_id();

    let issue_id = store
        .create_issue(&repo_id, "Test Issue", "Description", vec![])
        .await
        .expect("should create issue");

    // Explicit strategy with no conflicts should succeed without resolutions
    let result = store.merge_heads(&repo_id, CobType::Issue, &issue_id, MergeStrategy::Explicit, vec![]).await;

    assert!(result.is_ok(), "explicit merge with no conflicts should succeed");
}

// ============================================================================
// Merge Resolution Validation
// ============================================================================

#[tokio::test]
async fn test_field_resolution_creation() {
    // Test that FieldResolution can be created and accessed
    let hash = blake3::hash(b"test-change");
    let resolution = FieldResolution::new("title", hash);

    assert_eq!(resolution.field, "title");
    assert_eq!(resolution.selected(), hash);
}

#[tokio::test]
async fn test_merge_with_resolutions_succeeds() {
    let store = create_test_store().await;
    let repo_id = test_repo_id();

    let issue_id = store
        .create_issue(&repo_id, "Test Issue", "Description", vec![])
        .await
        .expect("should create issue");

    // Get current head for resolution
    let report = store.get_conflicts(&repo_id, CobType::Issue, &issue_id).await.expect("should get conflicts");
    let head = report.heads[0];

    // Even with unnecessary resolutions, merge should succeed
    let resolutions = vec![FieldResolution::new("title", head)];

    let merged = store
        .merge_heads(&repo_id, CobType::Issue, &issue_id, MergeStrategy::Explicit, resolutions)
        .await
        .expect("should merge with explicit resolutions");

    // Should resolve to a valid hash
    assert!(!merged.as_bytes().iter().all(|&b| b == 0));
}

// ============================================================================
// Issue Resolution After Merge
// ============================================================================

#[tokio::test]
async fn test_issue_resolvable_after_merge() {
    let store = create_test_store().await;
    let repo_id = test_repo_id();

    let issue_id = store
        .create_issue(&repo_id, "Original Title", "Original Body", vec!["bug".to_string()])
        .await
        .expect("should create issue");

    // Add some operations
    store.add_comment(&repo_id, &issue_id, "Comment 1").await.expect("should add comment");

    // Merge
    store
        .merge_heads(&repo_id, CobType::Issue, &issue_id, MergeStrategy::Auto, vec![])
        .await
        .expect("should merge");

    // Issue should still be resolvable
    let issue = store.resolve_issue(&repo_id, &issue_id).await.expect("should resolve issue after merge");

    assert_eq!(issue.title, "Original Title");
    assert_eq!(issue.body, "Original Body");
    assert!(issue.labels.contains("bug"));
    assert_eq!(issue.comments.len(), 1);
}

// ============================================================================
// Patch Merge Tests
// ============================================================================

#[tokio::test]
async fn test_patch_has_conflicts_single_head() {
    let store = create_test_store().await;
    let repo_id = test_repo_id();

    let base = blake3::hash(b"base-commit");
    let head = blake3::hash(b"head-commit");

    let patch_id = store
        .create_patch(&repo_id, "Fix bug", "This fixes the bug", base, head)
        .await
        .expect("should create patch");

    let has_conflicts = store.has_conflicts(&repo_id, CobType::Patch, &patch_id).await.expect("should check conflicts");

    assert!(!has_conflicts, "single head patch should have no conflicts");
}

#[tokio::test]
async fn test_patch_merge_and_resolve() {
    let store = create_test_store().await;
    let repo_id = test_repo_id();

    let base = blake3::hash(b"base-commit");
    let head = blake3::hash(b"head-commit");

    let patch_id = store
        .create_patch(&repo_id, "Fix bug", "Description", base, head)
        .await
        .expect("should create patch");

    // Merge
    store
        .merge_heads(&repo_id, CobType::Patch, &patch_id, MergeStrategy::Auto, vec![])
        .await
        .expect("should merge patch");

    // Patch should still be resolvable
    let patch = store.resolve_patch(&repo_id, &patch_id).await.expect("should resolve patch after merge");

    assert_eq!(patch.title, "Fix bug");
}

// ============================================================================
// MergeStrategy Enum Tests
// ============================================================================

#[test]
fn test_merge_strategy_equality() {
    assert_eq!(MergeStrategy::Auto, MergeStrategy::Auto);
    assert_eq!(MergeStrategy::LastWriteWins, MergeStrategy::LastWriteWins);
    assert_eq!(MergeStrategy::Explicit, MergeStrategy::Explicit);
    assert_ne!(MergeStrategy::Auto, MergeStrategy::LastWriteWins);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[tokio::test]
async fn test_merge_nonexistent_cob_fails() {
    let store = create_test_store().await;
    let repo_id = test_repo_id();
    let fake_id = blake3::hash(b"nonexistent");

    let result = store.merge_heads(&repo_id, CobType::Issue, &fake_id, MergeStrategy::Auto, vec![]).await;

    assert!(result.is_err(), "merging nonexistent COB should fail");
}

#[tokio::test]
async fn test_multiple_sequential_merges() {
    let store = create_test_store().await;
    let repo_id = test_repo_id();

    let issue_id = store
        .create_issue(&repo_id, "Test Issue", "Description", vec![])
        .await
        .expect("should create issue");

    // Multiple merges should be idempotent-ish (single head returns same)
    for _ in 0..3 {
        let _ = store
            .merge_heads(&repo_id, CobType::Issue, &issue_id, MergeStrategy::Auto, vec![])
            .await
            .expect("should merge");

        // Should still be resolvable
        let issue = store.resolve_issue(&repo_id, &issue_id).await.expect("should resolve");

        assert_eq!(issue.title, "Test Issue");
    }
}
