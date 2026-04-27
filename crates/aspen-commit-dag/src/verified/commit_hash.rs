//! Pure commit hash computation functions.
//!
//! Formally verified — see `verus/commit_hash_spec.rs` for proofs.

use crate::types::Commit;
use crate::types::CommitId;
use crate::types::MutationType;
use crate::verified::hash::ChainHash;
use crate::verified::hash::GENESIS_HASH;
use crate::verified::hash::constant_time_compare;

/// Tag byte for `MutationType::Set` in the mutations hash.
const TAG_SET: u8 = 0x01;

/// Tag byte for `MutationType::Delete` in the mutations hash.
const TAG_DELETE: u8 = 0x02;

/// Compute a BLAKE3 hash over sorted mutation entries.
///
/// The hash is a streaming BLAKE3 over sorted entries:
/// - key_len (u32 LE)
/// - key bytes
/// - tag byte (0x01 = Set, 0x02 = Delete)
/// - for Set: value_len (u32 LE), value bytes
///
/// The mutations **must** be sorted by key. The caller is responsible for sorting.
///
/// # Determinism
///
/// Same sorted mutations always produce the same hash.
#[inline]
pub fn compute_mutations_hash(sorted_mutations: &[(String, MutationType)]) -> ChainHash {
    debug_assert!(
        sorted_mutations.windows(2).all(|w| w[0].0 <= w[1].0),
        "mutations must be sorted by key"
    );
    let mut hasher = blake3::Hasher::new();

    for (key, mutation) in sorted_mutations {
        // Key length + key bytes
        let key_len = key.len() as u32;
        hasher.update(&key_len.to_le_bytes());
        hasher.update(key.as_bytes());

        match mutation {
            MutationType::Set(value) => {
                hasher.update(&[TAG_SET]);
                let value_len = value.len() as u32;
                hasher.update(&value_len.to_le_bytes());
                hasher.update(value.as_bytes());
            }
            MutationType::Delete => {
                hasher.update(&[TAG_DELETE]);
            }
        }
    }

    *hasher.finalize().as_bytes()
}

/// Compute a CommitId from commit metadata.
///
/// `CommitId = blake3(parent_hash || branch_id_bytes || mutations_hash || raft_revision_le ||
/// timestamp_ms_le)`
///
/// Uses `GENESIS_HASH` ([0u8; 32]) when parent is None.
#[inline]
#[allow(tigerstyle::ambiguous_params)] // raft_revision and timestamp_ms are semantically distinct u64s with unit-suffixed names
pub fn compute_commit_id(
    parent: &Option<CommitId>,
    mutations_hash: &ChainHash,
    branch_id: &str,
    raft_revision: u64,
    timestamp_ms: u64,
) -> CommitId {
    let mut hasher = blake3::Hasher::new();

    // Parent hash (chain linkage)
    let parent_hash = parent.as_ref().unwrap_or(&GENESIS_HASH);
    hasher.update(parent_hash);

    // Branch ID
    hasher.update(branch_id.as_bytes());

    // Mutations hash
    hasher.update(mutations_hash);

    // Raft revision (little-endian)
    hasher.update(&raft_revision.to_le_bytes());

    // Timestamp (little-endian)
    hasher.update(&timestamp_ms.to_le_bytes());

    *hasher.finalize().as_bytes()
}

/// Verify a commit's integrity by recomputing the mutations hash.
///
/// Returns `true` if the stored `mutations_hash` matches the recomputed hash
/// from `commit.mutations`. Uses constant-time comparison.
#[inline]
pub fn verify_commit_integrity(commit: &Commit) -> bool {
    let recomputed = compute_mutations_hash(&commit.mutations);
    constant_time_compare(&recomputed, &commit.mutations_hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verified::hash::hash_from_hex;
    use crate::verified::hash::hash_to_hex;

    const GOLDEN_MUTATIONS_HASH_HEX: &str = "dbd53f33679f0e296670008cd0df8020e3eb514c2ca8a84098e4860f15b4a489";
    const GOLDEN_GENESIS_COMMIT_ID_HEX: &str = "66bd5d2fbe90bd12c9ae86194472f5c2673859250f18178f7e99fd02c651d3c4";
    const GOLDEN_PARENT_COMMIT_ID_HEX: &str = "f55650d469fcfcde65cf57bb6e87f937915228fae61392e4940940fb8fdbdbe4";
    const GOLDEN_BRANCH_ID: &str = "branch-1";
    const GOLDEN_RAFT_REVISION: u64 = 42;
    const GOLDEN_TIMESTAMP_MS: u64 = 1000;
    const GOLDEN_PARENT_BYTE: u8 = 0x11;

    #[test]
    fn determinism_same_inputs_same_output() {
        let mutations = vec![
            ("a".to_string(), MutationType::Set("1".to_string())),
            ("b".to_string(), MutationType::Delete),
        ];

        let hash1 = compute_mutations_hash(&mutations);
        let hash2 = compute_mutations_hash(&mutations);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn different_inputs_different_hashes() {
        let m1 = vec![("a".to_string(), MutationType::Set("1".to_string()))];
        let m2 = vec![("a".to_string(), MutationType::Set("2".to_string()))];

        assert_ne!(compute_mutations_hash(&m1), compute_mutations_hash(&m2));
    }

    #[test]
    fn empty_mutations_produce_known_hash() {
        let empty: Vec<(String, MutationType)> = vec![];
        let hash = compute_mutations_hash(&empty);
        // BLAKE3 of empty input
        let expected = *blake3::hash(b"").as_bytes();
        assert_eq!(hash, expected);
    }

    #[test]
    fn tombstone_only_mutations() {
        let mutations = vec![
            ("a".to_string(), MutationType::Delete),
            ("b".to_string(), MutationType::Delete),
        ];
        let hash = compute_mutations_hash(&mutations);
        assert_ne!(hash, [0u8; 32]); // Not genesis
    }

    #[test]
    fn tombstone_differs_from_set() {
        let set_only = vec![("a".to_string(), MutationType::Set("1".to_string()))];
        let delete_only = vec![("a".to_string(), MutationType::Delete)];

        assert_ne!(compute_mutations_hash(&set_only), compute_mutations_hash(&delete_only),);
    }

    #[test]
    fn sort_order_matters() {
        let mutations_ab = vec![
            ("a".to_string(), MutationType::Set("1".to_string())),
            ("b".to_string(), MutationType::Set("2".to_string())),
        ];
        let mutations_bc = vec![
            ("b".to_string(), MutationType::Set("2".to_string())),
            ("c".to_string(), MutationType::Set("3".to_string())),
        ];

        // Different sorted inputs produce different hashes
        assert_ne!(compute_mutations_hash(&mutations_ab), compute_mutations_hash(&mutations_bc));
    }

    #[test]
    fn golden_mutations_hash_preserves_pre_move_behavior() {
        let mutations = vec![("k".to_string(), MutationType::Set("v".to_string()))];

        let hash = compute_mutations_hash(&mutations);

        assert_eq!(hash_to_hex(&hash), GOLDEN_MUTATIONS_HASH_HEX);
        assert_eq!(hash_from_hex(GOLDEN_MUTATIONS_HASH_HEX), Some(hash));
    }

    #[test]
    fn golden_commit_id_preserves_genesis_parent_behavior() {
        let mutations_hash = compute_mutations_hash(&[("k".to_string(), MutationType::Set("v".to_string()))]);

        let id = compute_commit_id(&None, &mutations_hash, GOLDEN_BRANCH_ID, GOLDEN_RAFT_REVISION, GOLDEN_TIMESTAMP_MS);

        assert_eq!(hash_to_hex(&id), GOLDEN_GENESIS_COMMIT_ID_HEX);
    }

    #[test]
    fn golden_commit_id_preserves_non_genesis_parent_behavior() {
        let mutations_hash = compute_mutations_hash(&[("k".to_string(), MutationType::Set("v".to_string()))]);
        let parent = Some([GOLDEN_PARENT_BYTE; crate::verified::hash::CHAIN_HASH_BYTES]);

        let id =
            compute_commit_id(&parent, &mutations_hash, GOLDEN_BRANCH_ID, GOLDEN_RAFT_REVISION, GOLDEN_TIMESTAMP_MS);

        assert_eq!(hash_to_hex(&id), GOLDEN_PARENT_COMMIT_ID_HEX);
    }

    #[test]
    fn commit_id_deterministic() {
        let parent = None;
        let mutations_hash = compute_mutations_hash(&[("k".to_string(), MutationType::Set("v".to_string()))]);

        let id1 = compute_commit_id(&parent, &mutations_hash, "branch-1", 42, 1000);
        let id2 = compute_commit_id(&parent, &mutations_hash, "branch-1", 42, 1000);
        assert_eq!(id1, id2);
    }

    #[test]
    fn commit_id_depends_on_parent() {
        let mutations_hash = [0xAA; 32];
        let p1 = Some([1u8; 32]);
        let p2 = Some([2u8; 32]);

        let id1 = compute_commit_id(&p1, &mutations_hash, "b", 1, 1000);
        let id2 = compute_commit_id(&p2, &mutations_hash, "b", 1, 1000);
        assert_ne!(id1, id2);
    }

    #[test]
    fn commit_id_depends_on_branch() {
        let mutations_hash = [0xAA; 32];
        let parent = None;

        let id1 = compute_commit_id(&parent, &mutations_hash, "branch-a", 1, 1000);
        let id2 = compute_commit_id(&parent, &mutations_hash, "branch-b", 1, 1000);
        assert_ne!(id1, id2);
    }

    #[test]
    fn commit_id_depends_on_revision() {
        let mutations_hash = [0xAA; 32];
        let parent = None;

        let id1 = compute_commit_id(&parent, &mutations_hash, "b", 1, 1000);
        let id2 = compute_commit_id(&parent, &mutations_hash, "b", 2, 1000);
        assert_ne!(id1, id2);
    }

    #[test]
    fn commit_id_depends_on_timestamp() {
        let mutations_hash = [0xAA; 32];
        let parent = None;

        let id1 = compute_commit_id(&parent, &mutations_hash, "b", 1, 1000);
        let id2 = compute_commit_id(&parent, &mutations_hash, "b", 1, 2000);
        assert_ne!(id1, id2);
    }

    #[test]
    fn first_commit_uses_genesis() {
        let mutations_hash = [0xAA; 32];
        let with_none = compute_commit_id(&None, &mutations_hash, "b", 1, 1000);
        let with_genesis = compute_commit_id(&Some(GENESIS_HASH), &mutations_hash, "b", 1, 1000);
        assert_eq!(with_none, with_genesis);
    }

    #[test]
    fn verify_commit_integrity_valid() {
        let mutations = vec![
            ("a".to_string(), MutationType::Set("1".to_string())),
            ("b".to_string(), MutationType::Delete),
        ];
        let mutations_hash = compute_mutations_hash(&mutations);
        let id = compute_commit_id(&None, &mutations_hash, "test", 1, 1000);

        let commit = Commit {
            id,
            parent: None,
            branch_id: "test".to_string(),
            mutations,
            mutations_hash,
            raft_revision: 1,
            chain_hash_at_commit: [0u8; 32],
            timestamp_ms: 1000,
        };

        assert!(verify_commit_integrity(&commit));
    }

    #[test]
    fn verify_commit_integrity_tampered() {
        let mutations = vec![("a".to_string(), MutationType::Set("1".to_string()))];
        let mutations_hash = compute_mutations_hash(&mutations);
        let id = compute_commit_id(&None, &mutations_hash, "test", 1, 1000);

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

        // Tamper with mutations
        commit.mutations.push(("z".to_string(), MutationType::Set("injected".to_string())));

        assert!(!verify_commit_integrity(&commit));
    }
}
