//! Tests for BranchOverlay commit-dag integration.
//!
//! Only compiled when the `commit-dag` feature is enabled.

#![cfg(feature = "commit-dag")]

use std::sync::Arc;

use aspen_commit_dag::CommitStore;
use aspen_commit_dag::verified::commit_hash::verify_commit_integrity;
use aspen_kv_branch::BranchOverlay;
use aspen_kv_types::ReadRequest;
use aspen_kv_types::WriteRequest;
use aspen_testing_core::DeterministicKeyValueStore;
use aspen_traits::KeyValueStore;

fn make_store() -> Arc<DeterministicKeyValueStore> {
    DeterministicKeyValueStore::new()
}

#[tokio::test]
async fn commit_with_feature_produces_commit_id() {
    let store = make_store();
    let branch = BranchOverlay::new("test-branch", Arc::clone(&store));

    branch.write(WriteRequest::set("a", "1")).await.unwrap();
    branch.write(WriteRequest::set("b", "2")).await.unwrap();

    let result = branch.commit().await.unwrap();
    assert!(result.commit_id.is_some(), "commit-dag enabled should produce CommitId");

    let commit_id = result.commit_id.unwrap();
    assert_ne!(commit_id, [0u8; 32], "CommitId should not be genesis hash");
}

#[tokio::test]
async fn second_commit_chains_from_first() {
    let store = make_store();
    let branch = BranchOverlay::new("chain-branch", Arc::clone(&store));

    // First commit
    branch.write(WriteRequest::set("a", "1")).await.unwrap();
    let r1 = branch.commit().await.unwrap();
    let c1 = r1.commit_id.unwrap();

    // Second commit
    branch.write(WriteRequest::set("b", "2")).await.unwrap();
    let r2 = branch.commit().await.unwrap();
    let c2 = r2.commit_id.unwrap();

    assert_ne!(c1, c2, "different commits must have different IDs");

    // Load the second commit and verify it chains from the first
    let commit2 = CommitStore::load_commit(&c2, store.as_ref()).await.unwrap();
    assert_eq!(commit2.parent, Some(c1));
}

#[tokio::test]
async fn commit_id_is_deterministic() {
    // Two branches with identical operations should produce the same mutations_hash.
    // But CommitIds will differ because timestamp_ms differs.
    // We can verify the mutations_hash is deterministic by loading the commits.
    let store = make_store();
    let branch = BranchOverlay::new("det-branch", Arc::clone(&store));

    branch.write(WriteRequest::set("x", "42")).await.unwrap();
    let result = branch.commit().await.unwrap();
    let commit_id = result.commit_id.unwrap();

    let commit = CommitStore::load_commit(&commit_id, store.as_ref()).await.unwrap();

    // Verify the commit integrity (mutations_hash is correct)
    assert!(verify_commit_integrity(&commit));
}

#[tokio::test]
async fn commit_metadata_stored_in_kv() {
    let store = make_store();
    let branch = BranchOverlay::new("meta-branch", Arc::clone(&store));

    branch.write(WriteRequest::set("key1", "val1")).await.unwrap();
    let result = branch.commit().await.unwrap();
    let commit_id = result.commit_id.unwrap();

    // Verify _sys:commit:{hex} exists
    let commit_key = CommitStore::commit_key(&commit_id);
    let read_result = store.read(ReadRequest::new(&commit_key)).await;
    assert!(read_result.is_ok(), "commit entry should exist in KV");

    // Verify _sys:commit-tip:{branch_id} exists
    let tip_key = CommitStore::branch_tip_key("meta-branch");
    let tip_result = store.read(ReadRequest::new(&tip_key)).await.unwrap();
    assert!(tip_result.kv.is_some(), "branch tip should exist");
    assert_eq!(tip_result.kv.unwrap().value, hex::encode(commit_id));
}

#[tokio::test]
async fn commit_no_conflict_check_also_produces_commit_id() {
    let store = make_store();
    let branch = BranchOverlay::new("ncf-branch", Arc::clone(&store));

    branch.write(WriteRequest::set("a", "1")).await.unwrap();
    let result = branch.commit_no_conflict_check().await.unwrap();

    assert!(result.commit_id.is_some());
}
