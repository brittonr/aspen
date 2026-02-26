//! Tests for `CobStore`.

use std::sync::Arc;

use aspen_blob::InMemoryBlobStore;
use aspen_testing::DeterministicKeyValueStore;

use super::super::change::CobType;
use super::*;

async fn create_test_store() -> CobStore<InMemoryBlobStore, DeterministicKeyValueStore> {
    let blobs = Arc::new(InMemoryBlobStore::new());
    let kv = DeterministicKeyValueStore::new();
    let secret_key = iroh::SecretKey::generate(&mut rand::rng());
    let node_id = hex::encode(secret_key.public().as_bytes());
    CobStore::new(blobs, kv, secret_key, &node_id)
}

fn test_repo_id() -> crate::identity::RepoId {
    crate::identity::RepoId::from_hash(blake3::hash(b"test-repo"))
}

#[tokio::test]
async fn test_issue_creation() {
    let store = create_test_store().await;
    let repo_id = test_repo_id();

    let issue_id = store
        .create_issue(&repo_id, "Bug report", "Something is broken", vec!["bug".to_string()])
        .await
        .expect("should create issue");

    // Resolve and verify
    let issue = store.resolve_issue(&repo_id, &issue_id).await.expect("should resolve");

    assert_eq!(issue.title, "Bug report");
    assert_eq!(issue.body, "Something is broken");
    assert!(issue.labels.contains("bug"));
    assert!(issue.state.is_open());
}

#[tokio::test]
async fn test_issue_comments() {
    let store = create_test_store().await;
    let repo_id = test_repo_id();

    let issue_id = store.create_issue(&repo_id, "Test issue", "", vec![]).await.expect("should create issue");

    // Add comments
    store.add_comment(&repo_id, &issue_id, "First comment").await.expect("should add comment");

    store.add_comment(&repo_id, &issue_id, "Second comment").await.expect("should add comment");

    // Resolve and verify
    let issue = store.resolve_issue(&repo_id, &issue_id).await.expect("should resolve");

    assert_eq!(issue.comments.len(), 2);
}

#[tokio::test]
async fn test_issue_close() {
    let store = create_test_store().await;
    let repo_id = test_repo_id();

    let issue_id = store.create_issue(&repo_id, "To be closed", "", vec![]).await.expect("should create issue");

    store.close_issue(&repo_id, &issue_id, Some("Fixed".to_string())).await.expect("should close");

    let issue = store.resolve_issue(&repo_id, &issue_id).await.expect("should resolve");

    assert!(issue.state.is_closed());
}

#[tokio::test]
async fn test_list_issues_empty() {
    let store = create_test_store().await;
    let repo_id = test_repo_id();

    let issues = store.list_issues(&repo_id).await.expect("should list issues");
    assert!(issues.is_empty());
}

#[tokio::test]
async fn test_list_issues_single() {
    let store = create_test_store().await;
    let repo_id = test_repo_id();

    let issue_id = store.create_issue(&repo_id, "Test issue", "", vec![]).await.expect("should create issue");

    let issues = store.list_issues(&repo_id).await.expect("should list issues");
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0], issue_id);
}

#[tokio::test]
async fn test_list_issues_multiple() {
    let store = create_test_store().await;
    let repo_id = test_repo_id();

    let issue1 = store.create_issue(&repo_id, "Issue 1", "", vec![]).await.expect("should create issue");

    let issue2 = store.create_issue(&repo_id, "Issue 2", "", vec![]).await.expect("should create issue");

    let issue3 = store.create_issue(&repo_id, "Issue 3", "", vec![]).await.expect("should create issue");

    let issues = store.list_issues(&repo_id).await.expect("should list issues");
    assert_eq!(issues.len(), 3);
    assert!(issues.contains(&issue1));
    assert!(issues.contains(&issue2));
    assert!(issues.contains(&issue3));
}

#[tokio::test]
async fn test_list_patches_empty() {
    let store = create_test_store().await;
    let repo_id = test_repo_id();

    let patches = store.list_patches(&repo_id).await.expect("should list patches");
    assert!(patches.is_empty());
}

#[tokio::test]
async fn test_list_cobs_different_types_isolated() {
    let store = create_test_store().await;
    let repo_id = test_repo_id();

    // Create an issue
    let issue_id = store.create_issue(&repo_id, "Test issue", "", vec![]).await.expect("should create issue");

    // List issues should return the issue
    let issues = store.list_issues(&repo_id).await.expect("should list issues");
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0], issue_id);

    // List patches should return empty (different type)
    let patches = store.list_patches(&repo_id).await.expect("should list patches");
    assert!(patches.is_empty());
}

#[tokio::test]
async fn test_list_cobs_different_repos_isolated() {
    let store = create_test_store().await;
    let repo_id1 = crate::identity::RepoId::from_hash(blake3::hash(b"repo-1"));
    let repo_id2 = crate::identity::RepoId::from_hash(blake3::hash(b"repo-2"));

    // Create an issue in repo 1
    let issue_id = store.create_issue(&repo_id1, "Test issue", "", vec![]).await.expect("should create issue");

    // List issues in repo 1 should return the issue
    let issues1 = store.list_issues(&repo_id1).await.expect("should list issues");
    assert_eq!(issues1.len(), 1);
    assert_eq!(issues1[0], issue_id);

    // List issues in repo 2 should return empty (different repo)
    let issues2 = store.list_issues(&repo_id2).await.expect("should list issues");
    assert!(issues2.is_empty());
}

#[tokio::test]
async fn test_list_cobs_generic() {
    let store = create_test_store().await;
    let repo_id = test_repo_id();

    // Create an issue
    let issue_id = store.create_issue(&repo_id, "Test issue", "", vec![]).await.expect("should create issue");

    // Use generic list_cobs with CobType::Issue
    let cobs = store.list_cobs(&repo_id, CobType::Issue).await.expect("should list cobs");
    assert_eq!(cobs.len(), 1);
    assert_eq!(cobs[0], issue_id);

    // Use generic list_cobs with CobType::Patch
    let cobs = store.list_cobs(&repo_id, CobType::Patch).await.expect("should list cobs");
    assert!(cobs.is_empty());
}
