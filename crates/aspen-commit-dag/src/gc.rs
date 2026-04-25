//! Garbage collection for expired commit entries.
//!
//! Scans `_sys:commit:` entries, checks TTL and reachability, and deletes
//! expired unreachable commits in bounded batches.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use aspen_kv_types::DeleteRequest;
use aspen_kv_types::ScanRequest;
use aspen_traits::KeyValueStore;
use tokio::task::JoinHandle;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::constants::COMMIT_GC_BATCH_SIZE;
use crate::constants::COMMIT_GC_TTL_SECONDS;
use crate::constants::COMMIT_KV_PREFIX;
use crate::constants::COMMIT_TIP_PREFIX;
use crate::error::CommitDagError;
use crate::types::Commit;
use crate::verified::hash::hash_from_hex;
use crate::verified::hash::hash_to_hex;

/// Garbage collector for expired commit DAG entries.
pub struct CommitGc;

impl CommitGc {
    /// Run a single GC pass: scan commits, check TTL + reachability, delete expired.
    ///
    /// Returns the number of commits deleted.
    pub async fn run_gc(kv: &dyn KeyValueStore, now_ms: u64) -> Result<u64, CommitDagError> {
        // 1. Scan all branch tips to find protected commit IDs
        let tips = Self::collect_branch_tips(kv).await?;

        // 2. Scan all commits
        let commits = Self::scan_all_commits(kv).await?;
        if commits.is_empty() {
            return Ok(0);
        }

        // 3. Build reachability set: any commit that is a parent of a non-expired commit
        let ttl_ms = COMMIT_GC_TTL_SECONDS.saturating_mul(1000);
        let mut protected: HashSet<[u8; 32]> = HashSet::new();

        // Branch tips are always protected
        for tip_hex in &tips {
            if let Some(id) = hash_from_hex(tip_hex) {
                protected.insert(id);
            }
        }

        // Non-expired commits and their parents are protected
        for commit in &commits {
            let age_ms = now_ms.saturating_sub(commit.timestamp_ms);
            if age_ms < ttl_ms {
                // Not expired — protect it and its parent
                protected.insert(commit.id);
                if let Some(parent) = &commit.parent {
                    protected.insert(*parent);
                }
            }
        }

        // 4. Collect expired, unreachable commits (up to batch size)
        let mut to_delete: Vec<String> = Vec::new();
        for commit in &commits {
            if to_delete.len() as u32 >= COMMIT_GC_BATCH_SIZE {
                break;
            }
            let age_ms = now_ms.saturating_sub(commit.timestamp_ms);
            if age_ms >= ttl_ms && !protected.contains(&commit.id) {
                let key = format!("{COMMIT_KV_PREFIX}{}", hash_to_hex(&commit.id));
                to_delete.push(key);
            }
        }

        // 5. Delete in a single batch
        let deleted = to_delete.len() as u64;
        for key in to_delete {
            kv.delete(DeleteRequest { key: key.clone() }).await.map_err(|e| CommitDagError::GcError {
                reason: format!("failed to delete {key}: {e}"),
            })?;
        }

        if deleted > 0 {
            info!(deleted, "commit GC pass complete");
        } else {
            debug!("commit GC pass: nothing to collect");
        }

        Ok(deleted)
    }

    /// Spawn a periodic GC background task.
    ///
    /// Runs on the leader only — skips on NOT_LEADER errors (same pattern
    /// as `spawn_alert_evaluator`).
    pub fn spawn_gc_task(kv: Arc<dyn KeyValueStore>, interval: Duration) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;

                let now_ms =
                    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis()
                        as u64;

                match Self::run_gc(kv.as_ref(), now_ms).await {
                    Ok(_) => {}
                    Err(CommitDagError::CommitStorageError { reason })
                        if reason.contains("NOT_LEADER") || reason.contains("not leader") =>
                    {
                        debug!("commit GC skipped: not leader");
                    }
                    Err(e) => {
                        warn!(error = %e, "commit GC error");
                    }
                }
            }
        })
    }

    /// Scan all branch tip values (commit ID hex strings).
    async fn collect_branch_tips(kv: &dyn KeyValueStore) -> Result<Vec<String>, CommitDagError> {
        let result = kv
            .scan(ScanRequest {
                prefix: COMMIT_TIP_PREFIX.to_string(),
                limit_results: Some(10_000),
                continuation_token: None,
            })
            .await
            .map_err(|e| CommitDagError::GcError {
                reason: format!("scan branch tips: {e}"),
            })?;

        Ok(result.entries.into_iter().map(|e| e.value).collect())
    }

    /// Scan and deserialize all commit entries.
    async fn scan_all_commits(kv: &dyn KeyValueStore) -> Result<Vec<Commit>, CommitDagError> {
        let result = kv
            .scan(ScanRequest {
                prefix: COMMIT_KV_PREFIX.to_string(),
                limit_results: Some(10_000),
                continuation_token: None,
            })
            .await
            .map_err(|e| CommitDagError::GcError {
                reason: format!("scan commits: {e}"),
            })?;

        let mut commits = Vec::new();
        for entry in result.entries {
            let bytes = match hex::decode(&entry.value) {
                Ok(b) => b,
                Err(_) => continue, // Skip malformed entries
            };
            match postcard::from_bytes::<Commit>(&bytes) {
                Ok(c) => commits.push(c),
                Err(_) => continue, // Skip malformed entries
            }
        }

        Ok(commits)
    }
}

#[cfg(test)]
mod tests {
    use aspen_testing_core::DeterministicKeyValueStore;

    use super::*;
    use crate::store::CommitStore;
    use crate::types::MutationType;
    use crate::verified::commit_hash::compute_commit_id;
    use crate::verified::commit_hash::compute_mutations_hash;

    fn make_commit(branch_id: &str, parent: Option<[u8; 32]>, ts: u64) -> Commit {
        let mutations = vec![("k".into(), MutationType::Set("v".into()))];
        let mutations_hash = compute_mutations_hash(&mutations);
        let id = compute_commit_id(&parent, branch_id, &mutations_hash, 1, ts);
        Commit {
            id,
            parent,
            branch_id: branch_id.to_string(),
            mutations,
            mutations_hash,
            raft_revision: 1,
            chain_hash_at_commit: [0u8; 32],
            timestamp_ms: ts,
        }
    }

    #[tokio::test]
    async fn expired_commit_no_refs_is_collected() {
        let kv = DeterministicKeyValueStore::new();
        let old_ts = 1000u64; // Very old
        let now_ms = old_ts + COMMIT_GC_TTL_SECONDS * 1000 + 1;

        let c = make_commit("br", None, old_ts);
        CommitStore::store_commit(&c, &kv).await.unwrap();

        let deleted = CommitGc::run_gc(&kv, now_ms).await.unwrap();
        assert_eq!(deleted, 1);

        // Verify it's actually gone
        let load_result = CommitStore::load_commit(&c.id, &kv).await;
        assert!(load_result.is_err());
    }

    #[tokio::test]
    async fn expired_commit_with_live_child_is_protected() {
        let kv = DeterministicKeyValueStore::new();
        let old_ts = 1000u64;
        let recent_ts = old_ts + COMMIT_GC_TTL_SECONDS * 1000 - 1000; // Within TTL relative to "now"
        let now_ms = old_ts + COMMIT_GC_TTL_SECONDS * 1000 + 1;

        let c1 = make_commit("br", None, old_ts);
        let c2 = make_commit("br", Some(c1.id), recent_ts);

        CommitStore::store_commit(&c1, &kv).await.unwrap();
        CommitStore::store_commit(&c2, &kv).await.unwrap();

        let deleted = CommitGc::run_gc(&kv, now_ms).await.unwrap();
        assert_eq!(deleted, 0); // c1 protected by c2's parent link

        // Both still exist
        assert!(CommitStore::load_commit(&c1.id, &kv).await.is_ok());
        assert!(CommitStore::load_commit(&c2.id, &kv).await.is_ok());
    }

    #[tokio::test]
    async fn branch_tip_commit_is_never_collected() {
        let kv = DeterministicKeyValueStore::new();
        let old_ts = 1000u64;
        let now_ms = old_ts + COMMIT_GC_TTL_SECONDS * 1000 + 1;

        let c = make_commit("br", None, old_ts);
        CommitStore::store_commit(&c, &kv).await.unwrap();
        CommitStore::update_branch_tip("br", &c.id, &kv).await.unwrap();

        let deleted = CommitGc::run_gc(&kv, now_ms).await.unwrap();
        assert_eq!(deleted, 0); // Protected by branch tip

        assert!(CommitStore::load_commit(&c.id, &kv).await.is_ok());
    }

    #[tokio::test]
    async fn gc_batch_size_respected() {
        let kv = DeterministicKeyValueStore::new();
        let old_ts = 1000u64;
        let now_ms = old_ts + COMMIT_GC_TTL_SECONDS * 1000 + 1;

        // Create several expired commits with distinct inputs
        let mut stored_ids = Vec::new();
        for i in 0u32..5 {
            let mutations = vec![(format!("unique-key-{i}"), MutationType::Set(format!("val-{i}")))];
            let mutations_hash = compute_mutations_hash(&mutations);
            let branch = format!("gc-test-branch-{i}");
            let id = compute_commit_id(&None, &branch, &mutations_hash, (i + 100) as u64, old_ts);
            let c = Commit {
                id,
                parent: None,
                branch_id: branch,
                mutations,
                mutations_hash,
                raft_revision: (i + 100) as u64,
                chain_hash_at_commit: [0u8; 32],
                timestamp_ms: old_ts,
            };
            CommitStore::store_commit(&c, &kv).await.unwrap();
            stored_ids.push(c.id);
        }

        // Verify all 5 are distinct
        let unique: HashSet<[u8; 32]> = stored_ids.iter().copied().collect();
        assert_eq!(unique.len(), 5, "all commits must have unique IDs");

        let deleted = CommitGc::run_gc(&kv, now_ms).await.unwrap();
        // All 5 should be deleted (well under batch size of 1000)
        assert_eq!(deleted, 5);
    }
}
