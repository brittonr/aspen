//! Commit chain verification for federated entries.
//!
//! When `commit-dag-federation` is enabled, incoming `_sys:commit:` entries
//! are verified before being applied: the mutations_hash is recomputed and
//! compared to the stored value. Provenance metadata is recorded at
//! `_sys:commit-origin:{hex}`.
//!
//! This module is only compiled when the `commit-dag-federation` feature is on.

#![cfg(feature = "commit-dag-federation")]

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

use aspen_commit_dag::COMMIT_KV_PREFIX;
use aspen_commit_dag::COMMIT_ORIGIN_PREFIX;
use aspen_commit_dag::types::Commit;
use aspen_commit_dag::verified::commit_hash::verify_commit_integrity;
use tracing::debug;
use tracing::warn;

/// Verification result for a single commit.
#[derive(Debug, Clone)]
pub struct CommitVerificationResult {
    /// The commit ID that was verified.
    pub commit_id_hex: String,
    /// Whether the mutations_hash was valid.
    pub verified: bool,
    /// Source cluster that sent the commit.
    pub source_cluster_id: String,
    /// Timestamp of import.
    pub imported_at_ms: u64,
    /// Reason for failure (if any).
    pub failure_reason: Option<String>,
}

/// Rate limiter tracking commits per peer per minute.
pub struct PeerRateLimiter {
    /// peer_id -> (count, window_start)
    counters: Mutex<HashMap<String, (u32, Instant)>>,
    max_per_minute: u32,
}

impl PeerRateLimiter {
    pub fn new(max_per_minute: u32) -> Self {
        Self {
            counters: Mutex::new(HashMap::new()),
            max_per_minute,
        }
    }

    /// Check if a peer is within rate limits. Returns true if allowed.
    pub fn check_and_increment(&self, peer_id: &str) -> bool {
        let mut counters = self.counters.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();

        let entry = counters.entry(peer_id.to_string()).or_insert((0, now));

        // Reset window if more than 60 seconds have passed
        if now.duration_since(entry.1).as_secs() >= 60 {
            *entry = (0, now);
        }

        if entry.0 >= self.max_per_minute {
            warn!(peer_id, limit = self.max_per_minute, "commit rate limit exceeded");
            return false;
        }

        entry.0 += 1;
        true
    }
}

/// Verify a commit entry received from a federated peer.
///
/// Returns the verification result. The caller is responsible for writing
/// the provenance record.
pub fn verify_incoming_commit(
    key: &str,
    value: &[u8],
    source_cluster_id: &str,
    now_ms: u64,
) -> Option<CommitVerificationResult> {
    // Only process _sys:commit: entries (not _sys:commit-tip:)
    if !key.starts_with(COMMIT_KV_PREFIX) {
        return None;
    }

    let commit_id_hex = &key[COMMIT_KV_PREFIX.len()..];

    // Try to deserialize
    let bytes = match hex::decode(value) {
        Ok(b) => b,
        Err(e) => {
            return Some(CommitVerificationResult {
                commit_id_hex: commit_id_hex.to_string(),
                verified: false,
                source_cluster_id: source_cluster_id.to_string(),
                imported_at_ms: now_ms,
                failure_reason: Some(format!("hex decode: {e}")),
            });
        }
    };

    let commit: Commit = match postcard::from_bytes(&bytes) {
        Ok(c) => c,
        Err(e) => {
            return Some(CommitVerificationResult {
                commit_id_hex: commit_id_hex.to_string(),
                verified: false,
                source_cluster_id: source_cluster_id.to_string(),
                imported_at_ms: now_ms,
                failure_reason: Some(format!("deserialization: {e}")),
            });
        }
    };

    // Verify mutations_hash
    let verified = verify_commit_integrity(&commit);
    let failure_reason = if verified {
        None
    } else {
        Some("mutations_hash_mismatch".to_string())
    };

    if !verified {
        warn!(
            commit_id = commit_id_hex,
            source = source_cluster_id,
            "commit verification failed: mutations_hash mismatch"
        );
    } else {
        debug!(commit_id = commit_id_hex, source = source_cluster_id, "commit verified successfully");
    }

    Some(CommitVerificationResult {
        commit_id_hex: commit_id_hex.to_string(),
        verified,
        source_cluster_id: source_cluster_id.to_string(),
        imported_at_ms: now_ms,
        failure_reason,
    })
}

/// Build the provenance KV key for a commit.
pub fn provenance_key(commit_id_hex: &str) -> String {
    format!("{COMMIT_ORIGIN_PREFIX}{commit_id_hex}")
}

/// Serialize a provenance record as JSON for KV storage.
pub fn serialize_provenance(result: &CommitVerificationResult) -> String {
    serde_json::json!({
        "source": result.source_cluster_id,
        "verified": result.verified,
        "imported_at_ms": result.imported_at_ms,
        "reason": result.failure_reason,
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use aspen_commit_dag::types::MutationType;
    use aspen_commit_dag::verified::commit_hash::compute_commit_id;
    use aspen_commit_dag::verified::commit_hash::compute_mutations_hash;

    use super::*;

    fn make_commit_bytes(tamper: bool) -> (String, Vec<u8>) {
        let mutations = vec![("a".to_string(), MutationType::Set("1".to_string()))];
        let mutations_hash = compute_mutations_hash(&mutations);
        let id = compute_commit_id(&None, "test", &mutations_hash, 1, 1000);

        let mut commit = Commit {
            id,
            parent: None,
            branch_id: "test".to_string(),
            mutations,
            mutations_hash,
            raft_revision: 1,
            chain_hash_at_commit: [0u8; 32],
            timestamp_ms: 1000,
        };

        if tamper {
            commit.mutations.push(("z".to_string(), MutationType::Set("injected".to_string())));
        }

        let bytes = postcard::to_allocvec(&commit).unwrap();
        let hex_value = hex::encode(&bytes);
        let key = format!("{COMMIT_KV_PREFIX}{}", hex::encode(id));
        (key, hex_value.into_bytes())
    }

    #[test]
    fn valid_commit_passes_verification() {
        let (key, value) = make_commit_bytes(false);
        let result = verify_incoming_commit(&key, &value, "cluster-west", 5000);
        assert!(result.is_some());
        let r = result.unwrap();
        assert!(r.verified);
        assert!(r.failure_reason.is_none());
    }

    #[test]
    fn tampered_commit_detected() {
        let (key, value) = make_commit_bytes(true);
        let result = verify_incoming_commit(&key, &value, "cluster-east", 5000);
        assert!(result.is_some());
        let r = result.unwrap();
        assert!(!r.verified);
        assert_eq!(r.failure_reason.as_deref(), Some("mutations_hash_mismatch"));
    }

    #[test]
    fn non_commit_key_returns_none() {
        let result = verify_incoming_commit("regular-key", b"value", "cluster", 5000);
        assert!(result.is_none());
    }

    #[test]
    fn rate_limiter_enforces_limit() {
        let limiter = PeerRateLimiter::new(3);
        assert!(limiter.check_and_increment("peer-a"));
        assert!(limiter.check_and_increment("peer-a"));
        assert!(limiter.check_and_increment("peer-a"));
        // 4th should be denied
        assert!(!limiter.check_and_increment("peer-a"));
        // Different peer is fine
        assert!(limiter.check_and_increment("peer-b"));
    }

    #[test]
    fn provenance_serialization() {
        let result = CommitVerificationResult {
            commit_id_hex: "abcd1234".to_string(),
            verified: true,
            source_cluster_id: "cluster-west".to_string(),
            imported_at_ms: 1000,
            failure_reason: None,
        };
        let json = serialize_provenance(&result);
        assert!(json.contains("\"verified\":true"));
        assert!(json.contains("\"source\":\"cluster-west\""));
    }
}
