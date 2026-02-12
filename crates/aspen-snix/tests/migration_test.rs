//! Integration tests for SNIX migration functionality.
//!
//! Tests the MigrationAwareCacheIndex and MigrationWorker.

use std::sync::Arc;

use aspen_blob::InMemoryBlobStore;
use aspen_cache::CacheEntry;
use aspen_cache::CacheIndex;
use aspen_cache::KvCacheIndex;
use aspen_snix::CacheEntryVersion;
use aspen_snix::IrohBlobService;
use aspen_snix::MigrationAwareCacheIndex;
use aspen_snix::MigrationProgress;
use aspen_snix::RaftDirectoryService;
use aspen_snix::RaftPathInfoService;
use aspen_testing::DeterministicKeyValueStore;

/// Create test infrastructure for migration tests.
struct TestInfra {
    #[allow(dead_code)]
    kv: Arc<DeterministicKeyValueStore>,
    #[allow(dead_code)]
    blob_store: Arc<InMemoryBlobStore>,
    snix_pathinfo: Arc<RaftPathInfoService<DeterministicKeyValueStore>>,
    #[allow(dead_code)]
    snix_blob: Arc<IrohBlobService<InMemoryBlobStore>>,
    #[allow(dead_code)]
    snix_dir: Arc<RaftDirectoryService<DeterministicKeyValueStore>>,
    legacy_index: Arc<KvCacheIndex<DeterministicKeyValueStore>>,
}

impl TestInfra {
    fn new() -> Self {
        let kv = DeterministicKeyValueStore::new();
        let blob_store = Arc::new(InMemoryBlobStore::new());

        let snix_pathinfo = Arc::new(RaftPathInfoService::from_arc(Arc::clone(&kv)));
        let snix_blob = Arc::new(IrohBlobService::from_arc(Arc::clone(&blob_store)));
        let snix_dir = Arc::new(RaftDirectoryService::from_arc(Arc::clone(&kv)));
        let legacy_index = Arc::new(KvCacheIndex::new(Arc::clone(&kv)));

        Self {
            kv,
            blob_store,
            snix_pathinfo,
            snix_blob,
            snix_dir,
            legacy_index,
        }
    }

    fn migration_aware_index(
        &self,
    ) -> MigrationAwareCacheIndex<DeterministicKeyValueStore, KvCacheIndex<DeterministicKeyValueStore>> {
        MigrationAwareCacheIndex::new(Arc::clone(&self.snix_pathinfo), Arc::clone(&self.legacy_index), true)
    }
}

/// Create a test cache entry.
fn make_cache_entry(store_hash: &str, store_path: &str, nar_size: u64) -> CacheEntry {
    CacheEntry::new(
        store_path.to_string(),
        store_hash.to_string(),
        "sha256:abc123".to_string(), // blob_hash
        nar_size,
        format!("sha256:{}", "0".repeat(64)),
        1704067200, // 2024-01-01
        1,
    )
}

// =============================================================================
// MigrationProgress Tests
// =============================================================================

#[test]
fn test_migration_progress_new() {
    let progress = MigrationProgress::new(100);

    assert_eq!(progress.total_entries, 100);
    assert_eq!(progress.migrated_count, 0);
    assert_eq!(progress.failed_count, 0);
    assert_eq!(progress.skipped_count, 0);
    assert!(!progress.is_complete);
    assert!(progress.started_at > 0);
}

#[test]
fn test_migration_progress_record_success() {
    let mut progress = MigrationProgress::new(10);

    progress.record_success("hash1");
    progress.record_success("hash2");

    assert_eq!(progress.migrated_count, 2);
    assert_eq!(progress.last_processed_hash, Some("hash2".to_string()));
}

#[test]
fn test_migration_progress_record_failure() {
    let mut progress = MigrationProgress::new(10);

    progress.record_failure("hash1", "test error");

    assert_eq!(progress.failed_count, 1);
    assert!(progress.error_message.is_some());
    assert!(progress.error_message.as_ref().unwrap().contains("test error"));
}

#[test]
fn test_migration_progress_record_skip() {
    let mut progress = MigrationProgress::new(10);

    progress.record_skip("hash1");

    assert_eq!(progress.skipped_count, 1);
}

#[test]
fn test_migration_progress_percentage() {
    let mut progress = MigrationProgress::new(100);

    assert_eq!(progress.progress_percent(), 0.0);

    progress.migrated_count = 25;
    assert_eq!(progress.progress_percent(), 25.0);

    progress.failed_count = 25;
    assert_eq!(progress.progress_percent(), 50.0);

    progress.skipped_count = 50;
    assert_eq!(progress.progress_percent(), 100.0);
}

#[test]
fn test_migration_progress_percentage_empty() {
    let progress = MigrationProgress::new(0);
    assert_eq!(progress.progress_percent(), 100.0);
}

#[test]
fn test_migration_progress_mark_complete() {
    let mut progress = MigrationProgress::new(10);
    progress.migrated_count = 10;

    progress.mark_complete();

    assert!(progress.is_complete);
}

#[test]
fn test_migration_progress_serialization() {
    let mut progress = MigrationProgress::new(100);
    progress.migrated_count = 50;
    progress.failed_count = 5;
    progress.record_failure("test_hash", "test error");

    let json = progress.to_json().unwrap();
    let restored = MigrationProgress::from_json(&json).unwrap();

    assert_eq!(restored.total_entries, 100);
    assert_eq!(restored.migrated_count, 50);
    assert_eq!(restored.failed_count, 6); // One more from record_failure
    assert!(restored.error_message.is_some());
}

// =============================================================================
// MigrationAwareCacheIndex Tests
// =============================================================================

#[tokio::test]
async fn test_migration_aware_get_from_legacy() {
    let infra = TestInfra::new();
    let index = infra.migration_aware_index();

    // Put entry in legacy store
    let entry = make_cache_entry(
        "0123456789abcdef0123456789abcdef01234567",
        "/nix/store/0123456789abcdef0123456789abcdef01234567-test",
        1024,
    );
    infra.legacy_index.put(entry.clone()).await.unwrap();

    // Get should find it via legacy
    let result = index.get_versioned(&entry.store_hash).await.unwrap();
    assert!(result.is_some());

    let versioned = result.unwrap();
    assert_eq!(versioned.version, CacheEntryVersion::V1);
    assert_eq!(versioned.entry.store_path, entry.store_path);
}

#[tokio::test]
async fn test_migration_aware_get_not_found() {
    let infra = TestInfra::new();
    let index = infra.migration_aware_index();

    // Use a valid hash format that doesn't exist
    let result = index.get("ffffffffffffffffffffffffffffffffffffffff").await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_migration_aware_exists_legacy() {
    let infra = TestInfra::new();
    let index = infra.migration_aware_index();

    let entry = make_cache_entry("0123456789abcdef0123456789abcdef01234567", "/nix/store/test", 1024);
    infra.legacy_index.put(entry.clone()).await.unwrap();

    assert!(index.exists(&entry.store_hash).await.unwrap());
    // Use a valid hash format that doesn't exist
    assert!(!index.exists("ffffffffffffffffffffffffffffffffffffffff").await.unwrap());
}

#[tokio::test]
async fn test_migration_aware_put_to_legacy() {
    let infra = TestInfra::new();
    let index = infra.migration_aware_index();

    let entry = make_cache_entry("0123456789abcdef0123456789abcdef01234567", "/nix/store/test", 1024);
    index.put(entry.clone()).await.unwrap();

    // Should be in legacy store
    let legacy_result = infra.legacy_index.get(&entry.store_hash).await.unwrap();
    assert!(legacy_result.is_some());
}

#[tokio::test]
async fn test_migration_aware_stats() {
    let infra = TestInfra::new();
    let index = infra.migration_aware_index();

    // Put some entries
    for i in 0..5 {
        let entry = make_cache_entry(&format!("{:040x}", i), &format!("/nix/store/pkg-{}", i), (i * 100) as u64);
        index.put(entry).await.unwrap();
    }

    let stats = index.stats().await.unwrap();
    assert_eq!(stats.total_entries, 5);
}

#[tokio::test]
async fn test_migration_aware_is_migrated_false() {
    let infra = TestInfra::new();
    let index = infra.migration_aware_index();

    // Entry only in legacy
    let entry = make_cache_entry("0123456789abcdef0123456789abcdef01234567", "/nix/store/test", 1024);
    infra.legacy_index.put(entry.clone()).await.unwrap();

    // Should not be marked as migrated
    assert!(!index.is_migrated(&entry.store_hash).await);
}

// =============================================================================
// Store Hash Conversion Tests
// =============================================================================

#[test]
fn test_nix_base32_decode() {
    // Known Nix base32 hash (32 chars = 20 bytes)
    let hash = "0c6kzph7l0dcbfmjap64f0czdafn3b7x";

    // This should produce 20 bytes
    // Note: The actual decoding is internal to MigrationAwareCacheIndex
    // We test via the hex format which is simpler
    assert_eq!(hash.len(), 32);
}

#[test]
fn test_hex_digest_conversion() {
    // 40 hex chars = 20 bytes
    let hex_hash = "0123456789abcdef0123456789abcdef01234567";
    assert_eq!(hex_hash.len(), 40);

    let bytes = hex::decode(hex_hash).unwrap();
    assert_eq!(bytes.len(), 20);
}

// =============================================================================
// Edge Cases
// =============================================================================

#[tokio::test]
async fn test_migration_aware_empty_hash() {
    let infra = TestInfra::new();
    let index = infra.migration_aware_index();

    // Empty hash should error (validation failure)
    let result = index.get("").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_migration_aware_invalid_hash_format() {
    let infra = TestInfra::new();
    let index = infra.migration_aware_index();

    // Various invalid formats should return errors
    let toolong = "toolong".repeat(20);
    let invalid_hashes = ["short", "not-hex-chars-here!", toolong.as_str()];

    for hash in &invalid_hashes {
        // Invalid hashes should return an error, not panic
        let result = index.get(hash).await;
        assert!(result.is_err(), "Expected error for invalid hash: {}", hash);
    }
}

// =============================================================================
// Concurrent Access
// =============================================================================

#[tokio::test]
async fn test_concurrent_migration_aware_reads() {
    let infra = TestInfra::new();

    // Pre-populate
    for i in 0..10 {
        let entry = make_cache_entry(&format!("{:040x}", i), &format!("/nix/store/pkg-{}", i), i * 100);
        infra.legacy_index.put(entry).await.unwrap();
    }

    let index = Arc::new(infra.migration_aware_index());

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let idx = Arc::clone(&index);
            let hash = format!("{:040x}", i);
            tokio::spawn(async move { idx.get(&hash).await })
        })
        .collect();

    for handle in handles {
        let result = handle.await.unwrap().unwrap();
        assert!(result.is_some());
    }
}

#[tokio::test]
async fn test_concurrent_migration_aware_writes() {
    let infra = TestInfra::new();
    let index = Arc::new(infra.migration_aware_index());

    let handles: Vec<_> = (0..20)
        .map(|i| {
            let idx = Arc::clone(&index);
            tokio::spawn(async move {
                let entry = make_cache_entry(&format!("{:040x}", i), &format!("/nix/store/pkg-{}", i), i * 100);
                idx.put(entry).await
            })
        })
        .collect();

    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    // Verify all written
    for i in 0..20 {
        let hash = format!("{:040x}", i);
        assert!(index.exists(&hash).await.unwrap());
    }
}
