//! Integration tests for the Pijul module.
//!
//! These tests exercise the full Pijul workflow including:
//! - Repository creation and management
//! - Channel (branch) operations
//! - Change recording and application
//! - Multi-channel workflows
//! - Error handling
//!
//! Note: Some tests require libpijul's pristine and change recording machinery,
//! which makes them true integration tests rather than unit tests.

use std::sync::Arc;

use aspen_blob::InMemoryBlobStore;
use aspen_forge::identity::RepoId;
use aspen_testing::DeterministicKeyValueStore;
use tempfile::TempDir;

use crate::apply::ChangeDirectory;
use crate::change_store::AspenChangeStore;
use crate::error::PijulError;
use crate::output::WorkingDirOutput;
use crate::pristine::PristineManager;
use crate::refs::PijulRefStore;
use crate::store::PijulStore;
use crate::types::ChangeHash;
use crate::types::ChangeMetadata;
use crate::types::PijulAuthor;
use crate::types::PijulRepoIdentity;

// ============================================================================
// Test Helpers
// ============================================================================

/// Create a test repository identity.
fn test_identity(name: &str) -> PijulRepoIdentity {
    PijulRepoIdentity::new(name, vec![])
}

/// Create a test repository identity with a custom default channel.
fn test_identity_with_channel(name: &str, channel: &str) -> PijulRepoIdentity {
    PijulRepoIdentity::new(name, vec![]).with_default_channel(channel)
}

/// Create a fixed test repo ID for deterministic tests.
fn fixed_repo_id() -> RepoId {
    RepoId([1u8; 32])
}

/// Create a test change hash from a byte value.
fn test_hash(value: u8) -> ChangeHash {
    ChangeHash([value; 32])
}

/// Create a test change metadata structure.
fn test_metadata(repo_id: RepoId, channel: &str, hash: ChangeHash, message: &str) -> ChangeMetadata {
    ChangeMetadata {
        hash,
        repo_id,
        channel: channel.to_string(),
        message: message.to_string(),
        authors: vec![PijulAuthor::from_name_email("Test User", "test@example.com")],
        dependencies: vec![],
        size_bytes: 100,
        recorded_at_ms: chrono::Utc::now().timestamp_millis() as u64,
    }
}

/// Create a metadata with dependencies.
fn test_metadata_with_deps(
    repo_id: RepoId,
    channel: &str,
    hash: ChangeHash,
    message: &str,
    deps: Vec<ChangeHash>,
) -> ChangeMetadata {
    let mut meta = test_metadata(repo_id, channel, hash, message);
    meta.dependencies = deps;
    meta
}

/// Setup test infrastructure for PijulStore tests.
async fn setup_pijul_store() -> (Arc<PijulStore<InMemoryBlobStore, dyn aspen_core::KeyValueStore>>, TempDir) {
    let tmp = TempDir::new().expect("should create temp dir");
    let blobs = Arc::new(InMemoryBlobStore::new());
    let kv: Arc<dyn aspen_core::KeyValueStore> = Arc::new(DeterministicKeyValueStore::new());
    let store = Arc::new(PijulStore::new(blobs, kv, tmp.path()));
    (store, tmp)
}

/// Create a test file in the working directory.
fn create_test_file(dir: &std::path::Path, name: &str, content: &str) {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("should create parent dirs");
    }
    std::fs::write(&path, content).expect("should write file");
}

// ============================================================================
// Repository Management Tests
// ============================================================================

#[tokio::test]
async fn test_create_repo_and_verify() {
    let (store, _tmp) = setup_pijul_store().await;

    let identity = test_identity("my-project");
    let repo_id = store.create_repo(identity.clone()).await.expect("should create repo");

    // Verify repo exists
    assert!(store.repo_exists(&repo_id).await.expect("should check existence"));

    // Verify identity can be retrieved
    let retrieved = store.get_repo(&repo_id).await.expect("should get repo");
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.name, "my-project");
    assert_eq!(retrieved.default_channel, "main");
}

#[tokio::test]
async fn test_create_repo_with_custom_channel() {
    let (store, _tmp) = setup_pijul_store().await;

    let identity = test_identity_with_channel("custom-project", "trunk");
    let repo_id = store.create_repo(identity).await.expect("should create repo");

    // Default channel should be created
    let channel = store.get_channel(&repo_id, "trunk").await.expect("should get channel");
    assert!(channel.is_some());
    assert!(channel.unwrap().head.is_none()); // Empty channel
}

#[tokio::test]
async fn test_create_duplicate_repo_fails() {
    let (store, _tmp) = setup_pijul_store().await;

    let identity = test_identity("duplicate-test");
    let _repo_id = store.create_repo(identity.clone()).await.expect("should create first");

    // Second creation should fail
    let result = store.create_repo(identity).await;
    assert!(matches!(result, Err(PijulError::RepoAlreadyExists { .. })));
}

#[tokio::test]
async fn test_get_nonexistent_repo() {
    let (store, _tmp) = setup_pijul_store().await;

    let fake_id = fixed_repo_id();
    let result = store.get_repo(&fake_id).await.expect("should not error");
    assert!(result.is_none());

    assert!(!store.repo_exists(&fake_id).await.expect("should check"));
}

#[tokio::test]
async fn test_list_repos() {
    let (store, _tmp) = setup_pijul_store().await;

    // Create multiple repos
    for i in 0..5 {
        let identity = test_identity(&format!("project-{}", i));
        store.create_repo(identity).await.expect("should create");
    }

    let repos = store.list_repos(100).await.expect("should list");
    assert_eq!(repos.len(), 5);

    // Verify all names are present
    let names: Vec<_> = repos.iter().map(|(_, id)| id.name.as_str()).collect();
    for i in 0..5 {
        assert!(names.contains(&format!("project-{}", i).as_str()));
    }
}

#[tokio::test]
async fn test_list_repos_respects_limit() {
    let (store, _tmp) = setup_pijul_store().await;

    // Create 10 repos
    for i in 0..10 {
        let identity = test_identity(&format!("limited-{}", i));
        store.create_repo(identity).await.expect("should create");
    }

    // List with limit of 5
    let repos = store.list_repos(5).await.expect("should list");
    assert_eq!(repos.len(), 5);
}

// ============================================================================
// Channel Management Tests
// ============================================================================

#[tokio::test]
async fn test_create_and_list_channels() {
    let (store, _tmp) = setup_pijul_store().await;

    let identity = test_identity("channel-test");
    let repo_id = store.create_repo(identity).await.expect("should create repo");

    // Create additional channels
    store.create_channel(&repo_id, "develop").await.expect("should create develop");
    store.create_channel(&repo_id, "feature/new-ui").await.expect("should create feature");

    // List all channels
    let channels = store.list_channels(&repo_id).await.expect("should list");
    assert_eq!(channels.len(), 3); // main, develop, feature/new-ui

    let names: Vec<_> = channels.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"main"));
    assert!(names.contains(&"develop"));
    assert!(names.contains(&"feature/new-ui"));
}

#[tokio::test]
async fn test_create_duplicate_channel_fails() {
    let (store, _tmp) = setup_pijul_store().await;

    let identity = test_identity("dup-channel");
    let repo_id = store.create_repo(identity).await.expect("should create repo");

    // Default channel already exists
    let result = store.create_channel(&repo_id, "main").await;
    assert!(matches!(result, Err(PijulError::ChannelAlreadyExists { .. })));
}

#[tokio::test]
async fn test_create_channel_on_nonexistent_repo_fails() {
    let (store, _tmp) = setup_pijul_store().await;

    let fake_id = fixed_repo_id();
    let result = store.create_channel(&fake_id, "main").await;
    assert!(matches!(result, Err(PijulError::RepoNotFound { .. })));
}

#[tokio::test]
async fn test_delete_channel() {
    let (store, _tmp) = setup_pijul_store().await;

    let identity = test_identity("delete-channel");
    let repo_id = store.create_repo(identity).await.expect("should create repo");

    // Create and delete a channel
    store.create_channel(&repo_id, "temp").await.expect("should create");
    store.delete_channel(&repo_id, "temp").await.expect("should delete");

    // Verify it's gone
    let channel = store.get_channel(&repo_id, "temp").await.expect("should query");
    assert!(channel.is_none());
}

#[tokio::test]
async fn test_delete_nonexistent_channel_fails() {
    let (store, _tmp) = setup_pijul_store().await;

    let identity = test_identity("delete-missing");
    let repo_id = store.create_repo(identity).await.expect("should create repo");

    let result = store.delete_channel(&repo_id, "nonexistent").await;
    assert!(matches!(result, Err(PijulError::ChannelNotFound { .. })));
}

#[tokio::test]
async fn test_fork_channel() {
    let (store, _tmp) = setup_pijul_store().await;

    let identity = test_identity("fork-test");
    let repo_id = store.create_repo(identity).await.expect("should create repo");

    // Store a change on main
    let change_data = b"fake change data";
    let meta = test_metadata(repo_id, "main", test_hash(1), "Initial commit");
    let hash = store.store_change(&repo_id, "main", change_data, meta).await.expect("should store");

    // Fork main to develop
    let forked = store.fork_channel(&repo_id, "main", "develop").await.expect("should fork");

    // Forked channel should have same head
    assert_eq!(forked.head, Some(hash));

    // Both channels should exist
    let main = store.get_channel(&repo_id, "main").await.expect("query").unwrap();
    let develop = store.get_channel(&repo_id, "develop").await.expect("query").unwrap();
    assert_eq!(main.head, develop.head);
}

#[tokio::test]
async fn test_fork_empty_channel() {
    let (store, _tmp) = setup_pijul_store().await;

    let identity = test_identity("fork-empty");
    let repo_id = store.create_repo(identity).await.expect("should create repo");

    // Fork empty main to develop
    let forked = store.fork_channel(&repo_id, "main", "develop").await.expect("should fork");

    // Forked channel should also be empty
    assert!(forked.head.is_none());
}

#[tokio::test]
async fn test_fork_to_existing_channel_fails() {
    let (store, _tmp) = setup_pijul_store().await;

    let identity = test_identity("fork-exists");
    let repo_id = store.create_repo(identity).await.expect("should create repo");

    store.create_channel(&repo_id, "develop").await.expect("should create");

    let result = store.fork_channel(&repo_id, "main", "develop").await;
    assert!(matches!(result, Err(PijulError::ChannelAlreadyExists { .. })));
}

#[tokio::test]
async fn test_fork_nonexistent_source_fails() {
    let (store, _tmp) = setup_pijul_store().await;

    let identity = test_identity("fork-missing");
    let repo_id = store.create_repo(identity).await.expect("should create repo");

    let result = store.fork_channel(&repo_id, "nonexistent", "develop").await;
    assert!(matches!(result, Err(PijulError::ChannelNotFound { .. })));
}

// ============================================================================
// Change Storage Tests
// ============================================================================

#[tokio::test]
async fn test_store_and_retrieve_change() {
    let (store, _tmp) = setup_pijul_store().await;

    let identity = test_identity("change-test");
    let repo_id = store.create_repo(identity).await.expect("should create");

    let change_data = b"compressed pijul change data";
    let meta = test_metadata(repo_id, "main", test_hash(0), "Test commit");

    let hash = store.store_change(&repo_id, "main", change_data, meta).await.expect("should store");

    // Retrieve the change
    let retrieved = store.get_change(&hash).await.expect("should get");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap(), change_data);

    // Check existence
    assert!(store.has_change(&hash).await.expect("should check"));
}

#[tokio::test]
async fn test_store_change_updates_channel_head() {
    let (store, _tmp) = setup_pijul_store().await;

    let identity = test_identity("head-test");
    let repo_id = store.create_repo(identity).await.expect("should create");

    // Channel starts empty
    let channel = store.get_channel(&repo_id, "main").await.expect("query").unwrap();
    assert!(channel.head.is_none());

    // Store a change
    let change_data = b"change 1";
    let meta = test_metadata(repo_id, "main", test_hash(1), "First");
    let hash = store.store_change(&repo_id, "main", change_data, meta).await.expect("should store");

    // Channel head should be updated
    let channel = store.get_channel(&repo_id, "main").await.expect("query").unwrap();
    assert_eq!(channel.head, Some(hash));
}

#[tokio::test]
async fn test_store_multiple_changes() {
    let (store, _tmp) = setup_pijul_store().await;

    let identity = test_identity("multi-change");
    let repo_id = store.create_repo(identity).await.expect("should create");

    // Store multiple changes
    let mut hashes = Vec::new();
    for i in 0..5 {
        let change_data = format!("change {}", i).into_bytes();
        let meta = test_metadata(repo_id, "main", test_hash(i as u8), &format!("Commit {}", i));
        let hash = store.store_change(&repo_id, "main", &change_data, meta).await.expect("should store");
        hashes.push(hash);
    }

    // All changes should exist
    for hash in &hashes {
        assert!(store.has_change(hash).await.expect("should check"));
    }

    // Channel head should be last change
    let channel = store.get_channel(&repo_id, "main").await.expect("query").unwrap();
    assert_eq!(channel.head, Some(hashes[4]));
}

#[tokio::test]
async fn test_get_change_metadata() {
    let (store, _tmp) = setup_pijul_store().await;

    let identity = test_identity("meta-test");
    let repo_id = store.create_repo(identity).await.expect("should create");

    let change_data = b"with metadata";
    let meta = ChangeMetadata {
        hash: test_hash(0),
        repo_id,
        channel: "main".to_string(),
        message: "Detailed commit message".to_string(),
        authors: vec![
            PijulAuthor::from_name_email("Alice", "alice@example.com"),
            PijulAuthor::from_name_email("Bob", "bob@example.com"),
        ],
        dependencies: vec![test_hash(10), test_hash(11)],
        size_bytes: 500,
        recorded_at_ms: chrono::Utc::now().timestamp_millis() as u64,
    };

    let hash = store.store_change(&repo_id, "main", change_data, meta.clone()).await.expect("should store");

    // Retrieve metadata
    let retrieved = store.get_change_metadata(&repo_id, &hash).await.expect("should get");
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();

    assert_eq!(retrieved.message, "Detailed commit message");
    assert_eq!(retrieved.authors.len(), 2);
    assert_eq!(retrieved.dependencies.len(), 2);
}

#[tokio::test]
async fn test_get_nonexistent_change() {
    let (store, _tmp) = setup_pijul_store().await;

    let fake_hash = test_hash(99);
    let result = store.get_change(&fake_hash).await.expect("should not error");
    assert!(result.is_none());

    assert!(!store.has_change(&fake_hash).await.expect("should check"));
}

// ============================================================================
// Change Log Tests
// ============================================================================

#[tokio::test]
async fn test_get_change_log_empty_channel() {
    let (store, _tmp) = setup_pijul_store().await;

    let identity = test_identity("log-empty");
    let repo_id = store.create_repo(identity).await.expect("should create");

    let log = store.get_change_log(&repo_id, "main", 100).await.expect("should get log");
    assert!(log.is_empty());
}

#[tokio::test]
async fn test_get_change_log_with_changes() {
    let (store, _tmp) = setup_pijul_store().await;

    let identity = test_identity("log-test");
    let repo_id = store.create_repo(identity).await.expect("should create");

    // Store changes with dependencies
    let change1 = b"change 1";
    let meta1 = test_metadata(repo_id, "main", test_hash(1), "First commit");
    let hash1 = store.store_change(&repo_id, "main", change1, meta1).await.expect("store");

    let change2 = b"change 2";
    let meta2 = test_metadata_with_deps(repo_id, "main", test_hash(2), "Second commit", vec![hash1]);
    let _hash2 = store.store_change(&repo_id, "main", change2, meta2).await.expect("store");

    // Get the log
    let log = store.get_change_log(&repo_id, "main", 100).await.expect("should get log");

    // Should have 2 entries
    assert_eq!(log.len(), 2);
}

#[tokio::test]
async fn test_get_change_log_respects_limit() {
    let (store, _tmp) = setup_pijul_store().await;

    let identity = test_identity("log-limit");
    let repo_id = store.create_repo(identity).await.expect("should create");

    // Store 10 changes (not linked as dependencies for simplicity)
    for i in 0..10 {
        let change = format!("change {}", i).into_bytes();
        let meta = test_metadata(repo_id, "main", test_hash(i as u8), &format!("Commit {}", i));
        store.store_change(&repo_id, "main", &change, meta).await.expect("store");
    }

    // Get log with limit of 5
    let log = store.get_change_log(&repo_id, "main", 5).await.expect("should get log");

    // Note: The log uses BFS traversal, so without dependencies it only gets the head
    // In a real DAG with dependencies, it would traverse more
    assert!(log.len() <= 5);
}

// ============================================================================
// Ref Store Tests
// ============================================================================

#[tokio::test]
async fn test_ref_store_set_and_get() {
    let kv = Arc::new(DeterministicKeyValueStore::new());
    let refs = PijulRefStore::new(kv);

    let repo_id = fixed_repo_id();
    let hash = test_hash(42);

    refs.set_channel(&repo_id, "main", hash).await.expect("should set");

    let retrieved = refs.get_channel(&repo_id, "main").await.expect("should get");
    assert_eq!(retrieved, Some(hash));
}

#[tokio::test]
async fn test_ref_store_empty_channel() {
    let kv = Arc::new(DeterministicKeyValueStore::new());
    let refs = PijulRefStore::new(kv);

    let repo_id = fixed_repo_id();

    // Create empty channel
    refs.create_empty_channel(&repo_id, "empty").await.expect("should create");

    // Should exist but have no head
    assert!(refs.channel_exists(&repo_id, "empty").await.expect("check"));
    assert!(refs.get_channel(&repo_id, "empty").await.expect("get").is_none());
}

#[tokio::test]
async fn test_ref_store_compare_and_set() {
    let kv = Arc::new(DeterministicKeyValueStore::new());
    let refs = PijulRefStore::new(kv);

    let repo_id = fixed_repo_id();
    let hash1 = test_hash(1);
    let hash2 = test_hash(2);

    // Set initial value
    refs.set_channel(&repo_id, "main", hash1).await.expect("set");

    // CAS with correct expected should succeed
    refs.compare_and_set_channel(&repo_id, "main", Some(hash1), hash2).await.expect("cas success");

    // Verify update
    assert_eq!(refs.get_channel(&repo_id, "main").await.expect("get"), Some(hash2));

    // CAS with wrong expected should fail
    let result = refs.compare_and_set_channel(&repo_id, "main", Some(hash1), test_hash(3)).await;
    assert!(matches!(result, Err(PijulError::ChannelConflict { .. })));
}

#[tokio::test]
async fn test_ref_store_list_channels() {
    let kv = Arc::new(DeterministicKeyValueStore::new());
    let refs = PijulRefStore::new(kv);

    let repo_id = fixed_repo_id();

    // Create multiple channels
    refs.set_channel(&repo_id, "main", test_hash(1)).await.expect("set");
    refs.set_channel(&repo_id, "develop", test_hash(2)).await.expect("set");
    refs.create_empty_channel(&repo_id, "empty").await.expect("create");

    let channels = refs.list_channels(&repo_id).await.expect("list");
    assert_eq!(channels.len(), 3);

    // Check each channel
    let map: std::collections::HashMap<_, _> = channels.into_iter().collect();
    assert_eq!(map.get("main"), Some(&Some(test_hash(1))));
    assert_eq!(map.get("develop"), Some(&Some(test_hash(2))));
    assert_eq!(map.get("empty"), Some(&None));
}

#[tokio::test]
async fn test_ref_store_count_channels() {
    let kv = Arc::new(DeterministicKeyValueStore::new());
    let refs = PijulRefStore::new(kv);

    let repo_id = fixed_repo_id();

    assert_eq!(refs.count_channels(&repo_id).await.expect("count"), 0);

    refs.set_channel(&repo_id, "main", test_hash(1)).await.expect("set");
    assert_eq!(refs.count_channels(&repo_id).await.expect("count"), 1);

    refs.create_empty_channel(&repo_id, "dev").await.expect("create");
    assert_eq!(refs.count_channels(&repo_id).await.expect("count"), 2);
}

#[tokio::test]
async fn test_ref_store_delete_channel() {
    let kv = Arc::new(DeterministicKeyValueStore::new());
    let refs = PijulRefStore::new(kv);

    let repo_id = fixed_repo_id();

    refs.set_channel(&repo_id, "temp", test_hash(1)).await.expect("set");
    assert!(refs.channel_exists(&repo_id, "temp").await.expect("exists"));

    refs.delete_channel(&repo_id, "temp").await.expect("delete");
    assert!(!refs.channel_exists(&repo_id, "temp").await.expect("exists"));
}

#[tokio::test]
async fn test_ref_store_invalid_channel_names() {
    let kv = Arc::new(DeterministicKeyValueStore::new());
    let refs = PijulRefStore::new(kv);

    let repo_id = fixed_repo_id();

    // Empty name
    let result = refs.set_channel(&repo_id, "", test_hash(1)).await;
    assert!(matches!(result, Err(PijulError::InvalidChannelName { .. })));

    // Name with control characters
    let result = refs.set_channel(&repo_id, "test\x00name", test_hash(1)).await;
    assert!(matches!(result, Err(PijulError::InvalidChannelName { .. })));
}

// ============================================================================
// Change Store Tests
// ============================================================================

#[tokio::test]
async fn test_change_store_cache_behavior() {
    let blobs = Arc::new(InMemoryBlobStore::new());
    let store = AspenChangeStore::new(blobs);

    let data = b"cached change data";
    let hash = store.store_change(data).await.expect("store");

    // Should be in cache after store
    assert_eq!(store.cache_size(), 1);

    // Clear and verify
    store.clear_cache();
    assert_eq!(store.cache_size(), 0);

    // Retrieve should re-cache
    let _ = store.get_change(&hash).await.expect("get");
    assert_eq!(store.cache_size(), 1);
}

#[tokio::test]
async fn test_change_store_size_limit() {
    use crate::constants::MAX_CHANGE_SIZE_BYTES;

    let blobs = Arc::new(InMemoryBlobStore::new());
    let store = AspenChangeStore::new(blobs);

    // Create data larger than the limit
    let large_data = vec![0u8; (MAX_CHANGE_SIZE_BYTES + 1) as usize];
    let result = store.store_change(&large_data).await;

    assert!(matches!(result, Err(PijulError::ChangeTooLarge { .. })));
}

// ============================================================================
// Pristine Manager Tests
// ============================================================================

#[test]
fn test_pristine_manager_open_or_create() {
    let tmp = TempDir::new().expect("temp dir");
    let manager = PristineManager::new(tmp.path());
    let repo_id = fixed_repo_id();

    // Should not exist initially
    assert!(!manager.exists(&repo_id));

    // Create
    let handle = manager.open_or_create(&repo_id).expect("create");
    assert_eq!(handle.repo_id(), &repo_id);

    // Should exist now
    assert!(manager.exists(&repo_id));

    // Should be cached
    assert_eq!(manager.cache_size(), 1);
}

#[test]
fn test_pristine_manager_open_nonexistent() {
    let tmp = TempDir::new().expect("temp dir");
    let manager = PristineManager::new(tmp.path());
    let repo_id = fixed_repo_id();

    let result = manager.open(&repo_id);
    assert!(matches!(result, Err(PijulError::PristineNotInitialized { .. })));
}

#[test]
fn test_pristine_manager_channel_operations() {
    let tmp = TempDir::new().expect("temp dir");
    let manager = PristineManager::new(tmp.path());
    let repo_id = fixed_repo_id();

    let handle = manager.open_or_create(&repo_id).expect("create");

    // Create channels
    {
        let mut txn = handle.mut_txn_begin().expect("begin");
        let _main = txn.open_or_create_channel("main").expect("create main");
        let _dev = txn.open_or_create_channel("develop").expect("create develop");
        txn.commit().expect("commit");
    }

    // List channels
    {
        let txn = handle.txn_begin().expect("begin read");
        let channels = txn.list_channels().expect("list");
        assert!(channels.contains(&"main".to_string()));
        assert!(channels.contains(&"develop".to_string()));
    }
}

#[test]
fn test_pristine_manager_fork_channel() {
    let tmp = TempDir::new().expect("temp dir");
    let manager = PristineManager::new(tmp.path());
    let repo_id = fixed_repo_id();

    let handle = manager.open_or_create(&repo_id).expect("create");

    // Create and fork
    {
        let mut txn = handle.mut_txn_begin().expect("begin");
        let _main = txn.open_or_create_channel("main").expect("create main");
        let _feature = txn.fork_channel("main", "feature").expect("fork");
        txn.commit().expect("commit");
    }

    // Verify both exist
    {
        let txn = handle.txn_begin().expect("begin read");
        assert!(txn.channel_exists("main").expect("check"));
        assert!(txn.channel_exists("feature").expect("check"));
    }
}

#[test]
fn test_pristine_manager_delete() {
    let tmp = TempDir::new().expect("temp dir");
    let manager = PristineManager::new(tmp.path());
    let repo_id = fixed_repo_id();

    // Create and then delete
    let _ = manager.open_or_create(&repo_id).expect("create");
    assert!(manager.exists(&repo_id));

    manager.delete(&repo_id).expect("delete");
    assert!(!manager.exists(&repo_id));
    assert_eq!(manager.cache_size(), 0);
}

// ============================================================================
// Change Directory Tests
// ============================================================================

#[test]
fn test_change_directory_path_format() {
    let tmp = TempDir::new().expect("temp dir");
    let blobs = Arc::new(InMemoryBlobStore::new());
    let change_store = Arc::new(AspenChangeStore::new(blobs));
    let dir = ChangeDirectory::new(&tmp.path().to_path_buf(), fixed_repo_id(), change_store);

    let hash = ChangeHash([
        0xAB, 0xCD, 0xEF, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE,
        0xFF, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC,
    ]);

    let path = dir.change_path(&hash);
    let path_str = path.to_string_lossy();

    // Should have prefix directory structure (first 2 hex chars)
    assert!(path_str.contains("ab"));
    assert!(path_str.ends_with(".change"));
}

#[test]
fn test_change_directory_hash_conversion() {
    let aspen_hash = ChangeHash([42u8; 32]);
    let pijul_hash = ChangeDirectory::<InMemoryBlobStore>::to_pijul_hash(&aspen_hash);

    // Should be Blake3 variant
    assert!(matches!(pijul_hash, libpijul::pristine::Hash::Blake3(_)));

    // Convert back
    let back = ChangeDirectory::<InMemoryBlobStore>::from_pijul_hash(&pijul_hash);
    assert_eq!(back, Some(aspen_hash));
}

// ============================================================================
// Working Directory Output Tests
// ============================================================================

#[test]
fn test_output_empty_channel() {
    let tmp = TempDir::new().expect("temp dir");
    let data_dir = tmp.path().join("data");
    let work_dir = tmp.path().join("work");

    let repo_id = fixed_repo_id();
    let blobs = Arc::new(InMemoryBlobStore::new());
    let change_store = Arc::new(AspenChangeStore::new(blobs));
    let change_dir = ChangeDirectory::new(&data_dir, repo_id, change_store);

    let pristine_mgr = PristineManager::new(&data_dir);
    let pristine = pristine_mgr.open_or_create(&repo_id).expect("create");

    let outputter = WorkingDirOutput::new(pristine, change_dir, work_dir.clone());

    // Output empty channel
    let result = outputter.output("main").expect("output");
    assert!(result.is_clean());
    assert!(work_dir.exists());
}

// ============================================================================
// Full Workflow Integration Tests
// ============================================================================

#[tokio::test]
async fn test_full_workflow_create_record_sync() {
    let tmp = TempDir::new().expect("temp dir");
    let data_dir = tmp.path().join("data");
    let work_dir = tmp.path().join("work");
    std::fs::create_dir_all(&work_dir).expect("create work dir");

    let blobs = Arc::new(InMemoryBlobStore::new());
    let kv = Arc::new(DeterministicKeyValueStore::new());
    let store = PijulStore::new(blobs.clone(), kv, &data_dir);

    // Create repository
    let identity = test_identity("full-workflow");
    let repo_id = store.create_repo(identity).await.expect("create repo");

    // Verify channel exists
    let channel = store.get_channel(&repo_id, "main").await.expect("get");
    assert!(channel.is_some());
    assert!(channel.unwrap().head.is_none());

    // Store a simulated change
    let change_data = b"simulated pijul change";
    let meta = test_metadata(repo_id, "main", test_hash(1), "Initial commit");
    let hash1 = store.store_change(&repo_id, "main", change_data, meta).await.expect("store");

    // Verify head updated
    let channel = store.get_channel(&repo_id, "main").await.expect("get").unwrap();
    assert_eq!(channel.head, Some(hash1));

    // Store second change with dependency
    let change2_data = b"second change";
    let meta2 = test_metadata_with_deps(repo_id, "main", test_hash(2), "Second commit", vec![hash1]);
    let hash2 = store.store_change(&repo_id, "main", change2_data, meta2).await.expect("store");

    // Verify head updated again
    let channel = store.get_channel(&repo_id, "main").await.expect("get").unwrap();
    assert_eq!(channel.head, Some(hash2));

    // Fork to feature branch
    let feature = store.fork_channel(&repo_id, "main", "feature/test").await.expect("fork");
    assert_eq!(feature.head, Some(hash2));

    // Store change only on feature branch
    let feature_data = b"feature change";
    let meta_feature = test_metadata(repo_id, "feature/test", test_hash(3), "Feature work");
    let hash3 = store.store_change(&repo_id, "feature/test", feature_data, meta_feature).await.expect("store");

    // Feature branch should have new head
    let feature = store.get_channel(&repo_id, "feature/test").await.expect("get").unwrap();
    assert_eq!(feature.head, Some(hash3));

    // Main should still have old head
    let main = store.get_channel(&repo_id, "main").await.expect("get").unwrap();
    assert_eq!(main.head, Some(hash2));
}

#[tokio::test]
async fn test_multi_repo_isolation() {
    let (store, _tmp) = setup_pijul_store().await;

    // Create two repos
    let id1 = test_identity("repo-one");
    let id2 = test_identity("repo-two");

    let repo1 = store.create_repo(id1).await.expect("create 1");
    let repo2 = store.create_repo(id2).await.expect("create 2");

    // Store changes in each
    let change1 = b"repo1 change";
    let meta1 = test_metadata(repo1, "main", test_hash(1), "Repo1 commit");
    let hash1 = store.store_change(&repo1, "main", change1, meta1).await.expect("store");

    let change2 = b"repo2 change";
    let meta2 = test_metadata(repo2, "main", test_hash(2), "Repo2 commit");
    let hash2 = store.store_change(&repo2, "main", change2, meta2).await.expect("store");

    // Each repo should have its own head
    let ch1 = store.get_channel(&repo1, "main").await.expect("get").unwrap();
    let ch2 = store.get_channel(&repo2, "main").await.expect("get").unwrap();

    assert_eq!(ch1.head, Some(hash1));
    assert_eq!(ch2.head, Some(hash2));
    assert_ne!(hash1, hash2);

    // List repos should show both
    let repos = store.list_repos(100).await.expect("list");
    assert_eq!(repos.len(), 2);
}

// ============================================================================
// Event Subscription Tests
// ============================================================================

#[tokio::test]
async fn test_store_events() {
    let (store, _tmp) = setup_pijul_store().await;

    // Subscribe before operations
    let mut rx = store.subscribe();

    // Create repo
    let identity = test_identity("event-test");
    let repo_id = store.create_repo(identity).await.expect("create");

    // Should receive RepoCreated event
    let event = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
        .await
        .expect("timeout")
        .expect("recv");

    match event {
        crate::PijulStoreEvent::RepoCreated { repo_id: id, .. } => {
            assert_eq!(id, repo_id);
        }
        _ => panic!("expected RepoCreated event"),
    }

    // Store change
    let change = b"event change";
    let meta = test_metadata(repo_id, "main", test_hash(1), "Event commit");
    let hash = store.store_change(&repo_id, "main", change, meta).await.expect("store");

    // Should receive ChangeRecorded event
    let event = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
        .await
        .expect("timeout")
        .expect("recv");

    match event {
        crate::PijulStoreEvent::ChangeRecorded {
            repo_id: id,
            channel,
            change_hash,
        } => {
            assert_eq!(id, repo_id);
            assert_eq!(channel, "main");
            assert_eq!(change_hash, hash);
        }
        _ => panic!("expected ChangeRecorded event"),
    }
}

// ============================================================================
// Error Case Tests
// ============================================================================

#[tokio::test]
async fn test_operations_on_nonexistent_repo() {
    let (store, _tmp) = setup_pijul_store().await;
    let fake_id = fixed_repo_id();

    // All operations should fail with RepoNotFound
    let result = store.create_channel(&fake_id, "test").await;
    assert!(matches!(result, Err(PijulError::RepoNotFound { .. })));

    let result = store.fork_channel(&fake_id, "main", "dev").await;
    assert!(matches!(result, Err(PijulError::RepoNotFound { .. })));

    let result = store.delete_channel(&fake_id, "main").await;
    assert!(matches!(result, Err(PijulError::RepoNotFound { .. })));

    let meta = test_metadata(fake_id, "main", test_hash(1), "test");
    let result = store.store_change(&fake_id, "main", b"data", meta).await;
    assert!(matches!(result, Err(PijulError::RepoNotFound { .. })));

    let result = store.get_change_log(&fake_id, "main", 100).await;
    assert!(matches!(result, Err(PijulError::RepoNotFound { .. })));
}

#[tokio::test]
async fn test_channel_limit() {
    use crate::constants::MAX_CHANNELS;

    let (store, _tmp) = setup_pijul_store().await;

    let identity = test_identity("many-channels");
    let repo_id = store.create_repo(identity).await.expect("create");

    // Create channels up to the limit
    // Note: Default channel "main" already exists
    for i in 1..MAX_CHANNELS {
        store.create_channel(&repo_id, &format!("ch-{}", i)).await.expect("create channel");
    }

    // One more should fail
    let result = store.create_channel(&repo_id, "one-too-many").await;
    assert!(matches!(result, Err(PijulError::TooManyChannels { .. })));
}

// ============================================================================
// Gossip Types Tests
// ============================================================================

#[test]
fn test_pijul_announcement_serialization() {
    use crate::gossip::PijulAnnouncement;

    let repo_id = fixed_repo_id();
    let announcement = PijulAnnouncement::ChannelUpdate {
        repo_id,
        channel: "main".to_string(),
        new_head: test_hash(1),
        old_head: Some(test_hash(0)),
        merkle: [2u8; 32],
    };

    let bytes = announcement.to_bytes();
    let recovered = PijulAnnouncement::from_bytes(&bytes).expect("deserialize");

    assert_eq!(announcement, recovered);
}

#[test]
fn test_pijul_topic_determinism() {
    use crate::gossip::PijulTopic;

    let repo_id = fixed_repo_id();
    let topic1 = PijulTopic::for_repo(repo_id);
    let topic2 = PijulTopic::for_repo(repo_id);

    // Same repo should give same topic
    assert_eq!(topic1.to_topic_bytes(), topic2.to_topic_bytes());

    // Different from global
    let global = PijulTopic::global();
    assert_ne!(topic1.to_topic_bytes(), global.to_topic_bytes());
}

#[test]
fn test_signed_announcement_verification() {
    use crate::gossip::PijulAnnouncement;
    use crate::gossip::SignedPijulAnnouncement;

    let secret_key = iroh::SecretKey::generate(&mut rand::rng());
    let repo_id = fixed_repo_id();

    let announcement = PijulAnnouncement::Seeding {
        repo_id,
        node_id: secret_key.public(),
        channels: vec!["main".to_string()],
    };

    let signed = SignedPijulAnnouncement::sign(announcement.clone(), &secret_key);

    // Verify should succeed
    let verified = signed.verify();
    assert!(verified.is_some());
    assert_eq!(verified.unwrap(), &announcement);
}

#[test]
fn test_signed_announcement_tamper_detection() {
    use crate::gossip::PijulAnnouncement;
    use crate::gossip::SignedPijulAnnouncement;

    let secret_key = iroh::SecretKey::generate(&mut rand::rng());
    let repo_id = fixed_repo_id();

    let announcement = PijulAnnouncement::ChangeAvailable {
        repo_id,
        change_hash: test_hash(1),
        size_bytes: 1000,
        dependencies: vec![],
    };

    let mut signed = SignedPijulAnnouncement::sign(announcement, &secret_key);

    // Tamper with the payload
    if let PijulAnnouncement::ChangeAvailable { ref mut size_bytes, .. } = signed.announcement {
        *size_bytes = 9999;
    }

    // Verification should fail
    assert!(signed.verify().is_none());
}
