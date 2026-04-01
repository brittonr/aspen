//! Integration test: git push fires RefUpdate announcement.
//!
//! Verifies that `ForgeNode::announce_ref_update` dispatches to the gossip
//! handler. This is the path exercised by `handle_git_bridge_push` after
//! each successful ref update — the fix that enables CI auto-trigger from
//! `git push` via `git-remote-aspen`.

use std::sync::Arc;

use aspen_blob::InMemoryBlobStore;
use aspen_forge::ForgeNode;
use aspen_forge::gossip::ForgeAnnouncementHandler;
use aspen_forge::gossip::SyncRequest;
use aspen_forge::identity::RepoId;
use tokio::sync::mpsc;

/// Create a ForgeNode with gossip enabled and a recording handler.
/// Returns the node and a receiver for sync requests triggered by announcements.
async fn create_node_with_gossip()
-> (ForgeNode<InMemoryBlobStore, aspen_testing::DeterministicKeyValueStore>, mpsc::Receiver<SyncRequest>) {
    let blobs = Arc::new(InMemoryBlobStore::new());
    let kv = aspen_testing::DeterministicKeyValueStore::new();
    let secret_key = iroh::SecretKey::generate(&mut rand::rng());

    // Create an iroh endpoint + gossip so announce_ref_update has a path to dispatch.
    let endpoint = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
        .secret_key(secret_key.clone())
        .bind()
        .await
        .expect("bind iroh endpoint");
    let gossip = Arc::new(iroh_gossip::net::Gossip::builder().spawn(endpoint.clone()));

    let mut node = ForgeNode::new(blobs, kv, secret_key);

    // Wire up gossip with a recording handler so we can observe announcements.
    let (handler, sync_rx, _seeding_rx) = ForgeAnnouncementHandler::with_channels(64);
    node.enable_gossip(gossip, Some(Arc::new(handler))).await.expect("enable gossip");

    (node, sync_rx)
}

/// After a ref update, `announce_ref_update` must dispatch a RefUpdate
/// announcement that the handler converts into a SyncRequest::RefUpdate.
/// This is the exact path that CI TriggerService listens on.
#[tokio::test]
async fn test_announce_ref_update_fires_handler() {
    let (node, mut sync_rx) = create_node_with_gossip().await;

    let repo_id = RepoId::from_hash(blake3::hash(b"test-repo"));
    let new_hash = [1u8; 32];
    let old_hash = Some([0u8; 32]);

    node.announce_ref_update(&repo_id, "refs/heads/main", new_hash, old_hash).await;

    // The handler should have received a SyncRequest::RefUpdate.
    let request = sync_rx.try_recv().expect("handler should receive RefUpdate");
    match request {
        SyncRequest::RefUpdate {
            repo_id: got_repo,
            ref_name,
            ..
        } => {
            assert_eq!(got_repo, repo_id);
            assert_eq!(ref_name, "refs/heads/main");
        }
        other => panic!("expected SyncRequest::RefUpdate, got {:?}", other),
    }
}

/// New refs (old_hash = None) should also fire announcements.
#[tokio::test]
async fn test_announce_new_ref_fires_handler() {
    let (node, mut sync_rx) = create_node_with_gossip().await;

    let repo_id = RepoId::from_hash(blake3::hash(b"another-repo"));
    let new_hash = [42u8; 32];

    node.announce_ref_update(&repo_id, "refs/heads/feature", new_hash, None).await;

    let request = sync_rx.try_recv().expect("handler should receive RefUpdate for new ref");
    match request {
        SyncRequest::RefUpdate {
            repo_id: got_repo,
            ref_name,
            ..
        } => {
            assert_eq!(got_repo, repo_id);
            assert_eq!(ref_name, "refs/heads/feature");
        }
        other => panic!("expected SyncRequest::RefUpdate, got {:?}", other),
    }
}

/// Without gossip enabled, `announce_ref_update` is a no-op (no panic, no error).
#[tokio::test]
async fn test_announce_without_gossip_is_noop() {
    let blobs = Arc::new(InMemoryBlobStore::new());
    let kv = aspen_testing::DeterministicKeyValueStore::new();
    let secret_key = iroh::SecretKey::generate(&mut rand::rng());
    let node = ForgeNode::new(blobs, kv, secret_key);

    let repo_id = RepoId::from_hash(blake3::hash(b"no-gossip-repo"));
    // Should not panic or error — just silently returns.
    node.announce_ref_update(&repo_id, "refs/heads/main", [1u8; 32], None).await;
}
