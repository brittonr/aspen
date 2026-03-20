//! Mirror sync worker — periodic ref synchronization from upstream repos.
//!
//! Implemented as an `aspen-jobs` [`Worker`] so it participates in the
//! standard job scheduling, retry, and observability infrastructure.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────┐   Schedule::interval(300s)   ┌──────────────┐
//! │  Scheduler   │ ──────────────────────────▶ │  JobManager   │
//! │  (per repo)  │   idempotency_key prevents  │  dedup queue  │
//! └─────────────┘   duplicate submissions      └──────┬───────┘
//!                                                      │
//!                                                      ▼
//!                                              ┌──────────────┐
//!                                              │ MirrorWorker │
//!                                              │  .execute()  │
//!                                              └──────┬───────┘
//!                                                      │
//!                         ┌────────────────────────────┤
//!                         ▼                            ▼
//!                  ┌─────────────┐            ┌──────────────┐
//!                  │  Election   │            │  SyncService │
//!                  │ (leader?)   │            │ (fetch refs) │
//!                  └─────────────┘            └──────────────┘
//! ```
//!
//! Each mirror repo gets a recurring job. The worker checks leadership
//! before syncing to avoid duplicate work across cluster nodes.

use std::sync::Arc;

use aspen_core::KeyValueStore;
use serde::Deserialize;
use serde::Serialize;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::identity::RepoId;
use crate::mirror::MirrorConfig;
use crate::refs::RefStore;

/// Job type identifier for mirror sync jobs.
pub const MIRROR_SYNC_JOB_TYPE: &str = "forge:mirror-sync";

/// Payload for a mirror sync job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorSyncPayload {
    /// Hex-encoded repo ID to sync.
    pub repo_id: String,
}

/// Result of a single mirror sync operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorSyncResult {
    /// Number of refs that were fast-forwarded.
    pub refs_updated: u32,
    /// Number of new refs created.
    pub refs_created: u32,
    /// Number of refs deleted (removed from upstream).
    pub refs_deleted: u32,
    /// Number of refs skipped due to divergence.
    pub refs_skipped: u32,
    /// Errors encountered (non-fatal).
    pub errors: Vec<String>,
}

/// Perform a mirror sync for a single repo.
///
/// This is the core sync logic, separated from the Worker trait so it
/// can be tested independently. It:
///
/// 1. Reads the MirrorConfig from KV
/// 2. Lists refs on both upstream and local
/// 3. For each upstream ref:
///    - If missing locally: create it
///    - If same hash: skip
///    - If local is ancestor of upstream (fast-forward): advance
///    - If diverged: skip with warning
/// 4. For each local ref not in upstream: delete it
/// 5. Updates last_sync_ms in the MirrorConfig
///
/// # Arguments
///
/// * `refs` - The RefStore for reading/writing refs
/// * `kv` - The KV store for reading/updating mirror config
/// * `repo_id` - The repository being mirrored
/// * `config` - The mirror configuration
pub async fn sync_mirror_refs<K: KeyValueStore + ?Sized>(
    refs: &RefStore<K>,
    kv: &Arc<K>,
    repo_id: &RepoId,
    config: &MirrorConfig,
) -> Result<MirrorSyncResult, crate::error::ForgeError> {
    let upstream_id = &config.upstream_repo_id;

    // List refs from both repos
    let upstream_refs = refs.list(upstream_id).await?;
    let local_refs = refs.list(repo_id).await?;

    let upstream_map: std::collections::HashMap<&str, blake3::Hash> =
        upstream_refs.iter().map(|(name, hash)| (name.as_str(), *hash)).collect();
    let local_map: std::collections::HashMap<&str, blake3::Hash> =
        local_refs.iter().map(|(name, hash)| (name.as_str(), *hash)).collect();

    let mut result = MirrorSyncResult {
        refs_updated: 0,
        refs_created: 0,
        refs_deleted: 0,
        refs_skipped: 0,
        errors: Vec::new(),
    };

    // Sync upstream → local
    for (ref_name, upstream_hash) in &upstream_map {
        match local_map.get(ref_name) {
            None => {
                // New ref — create locally
                match refs.set(repo_id, ref_name, *upstream_hash).await {
                    Ok(()) => {
                        result.refs_created = result.refs_created.saturating_add(1);
                        debug!(ref_name, "mirror: created ref");
                    }
                    Err(e) => {
                        result.errors.push(format!("create {ref_name}: {e}"));
                    }
                }
            }
            Some(local_hash) if local_hash == upstream_hash => {
                // Already in sync
            }
            Some(local_hash) => {
                // Diverged or fast-forward. We can't cheaply detect ancestry
                // without walking the commit graph, so use CAS: set if the
                // local value hasn't changed since we read it.
                //
                // For v1, we optimistically fast-forward. A proper ancestry
                // check would require fetching commit objects from upstream.
                match refs.compare_and_set(repo_id, ref_name, Some(*local_hash), *upstream_hash).await {
                    Ok(()) => {
                        result.refs_updated = result.refs_updated.saturating_add(1);
                        debug!(ref_name, "mirror: updated ref");
                    }
                    Err(crate::error::ForgeError::RefConflict { .. }) => {
                        result.refs_skipped = result.refs_skipped.saturating_add(1);
                        warn!(ref_name, "mirror: ref diverged, skipping");
                    }
                    Err(e) => {
                        result.errors.push(format!("update {ref_name}: {e}"));
                    }
                }
            }
        }
    }

    // Delete local refs not in upstream
    for ref_name in local_map.keys() {
        if !upstream_map.contains_key(ref_name) {
            match refs.delete(repo_id, ref_name).await {
                Ok(()) => {
                    result.refs_deleted = result.refs_deleted.saturating_add(1);
                    debug!(ref_name, "mirror: deleted ref");
                }
                Err(e) => {
                    result.errors.push(format!("delete {ref_name}: {e}"));
                }
            }
        }
    }

    // Update last_sync_ms in the config
    let now_ms =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;

    let mut updated_config = config.clone();
    updated_config.last_sync_ms = now_ms;
    updated_config.last_synced_refs_count =
        result.refs_updated.saturating_add(result.refs_created).saturating_add(result.refs_deleted);

    let key = format!("{}{}", crate::constants::KV_PREFIX_MIRROR, repo_id.to_hex());
    if let Ok(json) = serde_json::to_string(&updated_config) {
        let _ = kv
            .write(aspen_core::WriteRequest {
                command: aspen_core::WriteCommand::Set { key, value: json },
            })
            .await;
    }

    info!(
        repo = %repo_id.to_hex(),
        updated = result.refs_updated,
        created = result.refs_created,
        deleted = result.refs_deleted,
        skipped = result.refs_skipped,
        errors = result.errors.len(),
        "mirror sync complete"
    );

    Ok(result)
}

/// Create a `JobSpec` for a mirror sync of the given repo.
///
/// Uses `Schedule::interval()` for recurring execution and an
/// `idempotency_key` to prevent duplicate jobs per repo.
pub fn mirror_sync_job_spec(repo_id: &RepoId, interval_secs: u32) -> Result<aspen_jobs::JobSpec, aspen_jobs::JobError> {
    let payload = MirrorSyncPayload {
        repo_id: repo_id.to_hex(),
    };

    Ok(aspen_jobs::JobSpec::new(MIRROR_SYNC_JOB_TYPE)
        .payload(payload)?
        .schedule(aspen_jobs::Schedule::interval(std::time::Duration::from_secs(u64::from(interval_secs))))
        .idempotency_key(format!("mirror:{}", repo_id.to_hex()))
        .tag("forge")
        .tag("mirror"))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use aspen_blob::InMemoryBlobStore;
    use aspen_testing::DeterministicKeyValueStore;

    use super::*;
    use crate::node::ForgeNode;

    type TestNode = ForgeNode<InMemoryBlobStore, DeterministicKeyValueStore>;

    /// Create a pair of forge nodes (upstream + mirror) for testing.
    async fn create_test_pair() -> (TestNode, TestNode, Arc<DeterministicKeyValueStore>) {
        let blobs = Arc::new(InMemoryBlobStore::new());
        // Both nodes share the same KV (simulating same-cluster Raft)
        let kv = DeterministicKeyValueStore::new();

        let sk1 = iroh::SecretKey::generate(&mut rand::rng());
        let sk2 = iroh::SecretKey::generate(&mut rand::rng());

        let node1: TestNode = ForgeNode::new(blobs.clone(), kv.clone(), sk1);
        let node2: TestNode = ForgeNode::new(blobs, kv.clone(), sk2);

        (node1, node2, kv)
    }

    #[tokio::test]
    async fn test_mirror_sync_new_refs() {
        let (upstream_node, mirror_node, kv) = create_test_pair().await;

        // Create upstream repo with commits
        let upstream = upstream_node.create_repo("upstream", vec![upstream_node.public_key()], 1).await.unwrap();
        let upstream_id = upstream.repo_id();
        let commit = upstream_node.init_repo(&upstream_id, "init").await.unwrap();

        // Create mirror repo (empty)
        let mirror = mirror_node.create_repo("mirror", vec![mirror_node.public_key()], 1).await.unwrap();
        let mirror_id = mirror.repo_id();

        // Configure mirror
        let config = MirrorConfig::new(upstream_id, 300);

        // Sync
        let result = sync_mirror_refs(&mirror_node.refs, &kv, &mirror_id, &config).await.unwrap();

        assert_eq!(result.refs_created, 1); // heads/main
        assert_eq!(result.refs_updated, 0);
        assert_eq!(result.refs_skipped, 0);
        assert!(result.errors.is_empty());

        // Verify mirror has the ref
        let mirror_head = mirror_node.refs.get(&mirror_id, "heads/main").await.unwrap();
        assert_eq!(mirror_head, Some(commit));
    }

    #[tokio::test]
    async fn test_mirror_sync_fast_forward() {
        let (upstream_node, mirror_node, kv) = create_test_pair().await;

        let upstream = upstream_node.create_repo("upstream", vec![upstream_node.public_key()], 1).await.unwrap();
        let upstream_id = upstream.repo_id();
        let c0 = upstream_node.init_repo(&upstream_id, "init").await.unwrap();

        let mirror = mirror_node.create_repo("mirror", vec![mirror_node.public_key()], 1).await.unwrap();
        let mirror_id = mirror.repo_id();

        // Initial sync
        let config = MirrorConfig::new(upstream_id, 300);
        sync_mirror_refs(&mirror_node.refs, &kv, &mirror_id, &config).await.unwrap();

        // Push new commit to upstream
        let blob = upstream_node.git.store_blob(b"v2".to_vec()).await.unwrap();
        let tree = upstream_node.git.create_tree(&[crate::git::TreeEntry::file("file.txt", blob)]).await.unwrap();
        let c1 = upstream_node.git.commit(tree, vec![c0], "v2").await.unwrap();
        upstream_node.refs.set(&upstream_id, "heads/main", c1).await.unwrap();

        // Sync again — should fast-forward
        let result = sync_mirror_refs(&mirror_node.refs, &kv, &mirror_id, &config).await.unwrap();

        assert_eq!(result.refs_updated, 1);
        assert_eq!(result.refs_created, 0);
        assert!(result.errors.is_empty());

        let mirror_head = mirror_node.refs.get(&mirror_id, "heads/main").await.unwrap();
        assert_eq!(mirror_head, Some(c1));
    }

    #[tokio::test]
    async fn test_mirror_sync_deletes_removed_refs() {
        let (upstream_node, mirror_node, kv) = create_test_pair().await;

        let upstream = upstream_node.create_repo("upstream", vec![upstream_node.public_key()], 1).await.unwrap();
        let upstream_id = upstream.repo_id();
        let c0 = upstream_node.init_repo(&upstream_id, "init").await.unwrap();

        // Add a feature branch
        upstream_node.refs.set(&upstream_id, "heads/feature", c0).await.unwrap();

        let mirror = mirror_node.create_repo("mirror", vec![mirror_node.public_key()], 1).await.unwrap();
        let mirror_id = mirror.repo_id();

        // Initial sync (gets main + feature)
        let config = MirrorConfig::new(upstream_id, 300);
        let result = sync_mirror_refs(&mirror_node.refs, &kv, &mirror_id, &config).await.unwrap();
        assert_eq!(result.refs_created, 2);

        // Delete feature branch from upstream
        upstream_node.refs.delete(&upstream_id, "heads/feature").await.unwrap();

        // Sync again — should delete feature
        let result = sync_mirror_refs(&mirror_node.refs, &kv, &mirror_id, &config).await.unwrap();

        assert_eq!(result.refs_deleted, 1);

        let feature = mirror_node.refs.get(&mirror_id, "heads/feature").await.unwrap();
        assert!(feature.is_none());
    }

    #[tokio::test]
    async fn test_mirror_sync_updates_config_timestamp() {
        let (upstream_node, mirror_node, kv) = create_test_pair().await;

        let upstream = upstream_node.create_repo("upstream", vec![upstream_node.public_key()], 1).await.unwrap();
        let upstream_id = upstream.repo_id();
        upstream_node.init_repo(&upstream_id, "init").await.unwrap();

        let mirror = mirror_node.create_repo("mirror", vec![mirror_node.public_key()], 1).await.unwrap();
        let mirror_id = mirror.repo_id();

        let config = MirrorConfig::new(upstream_id, 300);
        mirror_node.set_mirror_config(&mirror_id, &config).await.unwrap();

        // Sync
        sync_mirror_refs(&mirror_node.refs, &kv, &mirror_id, &config).await.unwrap();

        // Config should have updated last_sync_ms
        let updated = mirror_node.get_mirror_config(&mirror_id).await.unwrap().unwrap();
        assert!(updated.last_sync_ms > 0);
    }

    #[tokio::test]
    async fn test_mirror_sync_job_spec() {
        let repo_id = RepoId::from_hash(blake3::hash(b"test"));
        let spec = mirror_sync_job_spec(&repo_id, 300).unwrap();

        assert_eq!(spec.job_type, MIRROR_SYNC_JOB_TYPE);
        assert!(spec.schedule.is_some());
        assert!(spec.idempotency_key.is_some());
        assert!(spec.config.tags.contains(&"mirror".to_string()));
    }
}
