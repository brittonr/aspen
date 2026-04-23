//! Property-based tests for chain hashing and KV storage computations.
//!
//! Tests invariants that must hold for ALL inputs, not just handpicked examples.
//! Catches edge cases in overflow, boundary conditions, and hash chaining.

use aspen_redb_storage::verified::integrity::*;
use aspen_redb_storage::verified::kv::*;
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Chain hash properties
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    /// Chain hash of an entry should always verify against itself.
    #[test]
    fn test_chain_hash_self_verify(
        prev_hash in prop::array::uniform32(any::<u8>()),
        log_index in any::<u64>(),
        term in any::<u64>(),
        entry_bytes in prop::collection::vec(any::<u8>(), 0..1024),
    ) {
        let hash = compute_entry_hash(EntryHashInput { prev_hash: &prev_hash, log_index, term, entry_bytes: &entry_bytes });
        assert!(
            verify_entry_hash(EntryHashVerificationInput { prev_hash: &prev_hash, log_index, term, entry_bytes: &entry_bytes, expected: &hash }),
            "Entry hash must verify against itself"
        );
    }

    /// Changing any single byte of entry data should produce a different hash.
    #[test]
    fn test_chain_hash_data_sensitivity(
        prev_hash in prop::array::uniform32(any::<u8>()),
        log_index in any::<u64>(),
        term in any::<u64>(),
        entry_bytes in prop::collection::vec(any::<u8>(), 1..256),
        flip_pos in any::<prop::sample::Index>(),
    ) {
        let hash = compute_entry_hash(EntryHashInput { prev_hash: &prev_hash, log_index, term, entry_bytes: &entry_bytes });

        let mut altered = entry_bytes.clone();
        let pos = flip_pos.index(altered.len());
        altered[pos] ^= 0xFF; // flip all bits at one position

        let altered_hash = compute_entry_hash(EntryHashInput { prev_hash: &prev_hash, log_index, term, entry_bytes: &altered });
        assert_ne!(
            hash, altered_hash,
            "Flipping byte at position {} should change the hash",
            pos
        );
    }

    /// Changing the previous hash should produce a different entry hash (chain linkage).
    #[test]
    fn test_chain_hash_prev_hash_sensitivity(
        prev_hash in prop::array::uniform32(any::<u8>()),
        log_index in any::<u64>(),
        term in any::<u64>(),
        entry_bytes in prop::collection::vec(any::<u8>(), 0..256),
    ) {
        let hash1 = compute_entry_hash(EntryHashInput { prev_hash: &prev_hash, log_index, term, entry_bytes: &entry_bytes });

        let mut different_prev = prev_hash;
        different_prev[0] ^= 0x01;
        let hash2 = compute_entry_hash(EntryHashInput { prev_hash: &different_prev, log_index, term, entry_bytes: &entry_bytes });

        assert_ne!(hash1, hash2, "Different prev_hash should produce different entry hash");
    }

    /// Changing the log index should produce a different hash (prevents index substitution).
    #[test]
    fn test_chain_hash_index_sensitivity(
        prev_hash in prop::array::uniform32(any::<u8>()),
        log_index in 0u64..u64::MAX,
        term in any::<u64>(),
        entry_bytes in prop::collection::vec(any::<u8>(), 0..256),
    ) {
        let hash1 = compute_entry_hash(EntryHashInput { prev_hash: &prev_hash, log_index, term, entry_bytes: &entry_bytes });
        let hash2 = compute_entry_hash(EntryHashInput {
            prev_hash: &prev_hash,
            log_index: log_index.saturating_add(1),
            term,
            entry_bytes: &entry_bytes,
        });
        assert_ne!(hash1, hash2, "Different log_index should produce different hash");
    }

    /// Changing the term should produce a different hash (prevents term substitution).
    #[test]
    fn test_chain_hash_term_sensitivity(
        prev_hash in prop::array::uniform32(any::<u8>()),
        log_index in any::<u64>(),
        term in 0u64..u64::MAX,
        entry_bytes in prop::collection::vec(any::<u8>(), 0..256),
    ) {
        let hash1 = compute_entry_hash(EntryHashInput { prev_hash: &prev_hash, log_index, term, entry_bytes: &entry_bytes });
        let hash2 = compute_entry_hash(EntryHashInput {
            prev_hash: &prev_hash,
            log_index,
            term: term.saturating_add(1),
            entry_bytes: &entry_bytes,
        });
        assert_ne!(hash1, hash2, "Different term should produce different hash");
    }

    /// Constant-time compare must agree with regular equality.
    #[test]
    fn test_constant_time_compare_agrees_with_eq(
        a in prop::array::uniform32(any::<u8>()),
        b in prop::array::uniform32(any::<u8>()),
    ) {
        let is_ct_result = constant_time_compare(&a, &b);
        let is_eq_result = a == b;
        assert_eq!(is_ct_result, is_eq_result, "constant_time_compare must agree with ==");
    }

    /// Constant-time compare must be reflexive.
    #[test]
    fn test_constant_time_compare_reflexive(
        a in prop::array::uniform32(any::<u8>()),
    ) {
        assert!(constant_time_compare(&a, &a), "constant_time_compare(x, x) must be true");
    }
}

// ---------------------------------------------------------------------------
// Snapshot integrity properties
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    /// Snapshot integrity should self-verify.
    #[test]
    fn test_snapshot_integrity_self_verify(
        meta_bytes in prop::collection::vec(any::<u8>(), 0..512),
        data in prop::collection::vec(any::<u8>(), 0..2048),
        chain_hash in prop::array::uniform32(any::<u8>()),
    ) {
        let integrity = SnapshotIntegrity::compute(&meta_bytes, &data, chain_hash);
        assert!(
            integrity.verify(&meta_bytes, &data),
            "Snapshot integrity must verify against itself"
        );
    }

    /// Snapshot integrity should detect data corruption.
    #[test]
    fn test_snapshot_integrity_detects_corruption(
        meta_bytes in prop::collection::vec(any::<u8>(), 0..256),
        data in prop::collection::vec(any::<u8>(), 1..1024),
        chain_hash in prop::array::uniform32(any::<u8>()),
        flip_pos in any::<prop::sample::Index>(),
    ) {
        let integrity = SnapshotIntegrity::compute(&meta_bytes, &data, chain_hash);

        let mut corrupted = data.clone();
        let pos = flip_pos.index(corrupted.len());
        corrupted[pos] ^= 0xFF;

        assert!(
            !integrity.verify(&meta_bytes, &corrupted),
            "Snapshot integrity must detect corrupted data"
        );
    }
}

// ---------------------------------------------------------------------------
// KV version computation properties
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    /// New key creation always produces version 1.
    #[test]
    fn test_new_key_version_is_1(log_index in 1u64..u64::MAX) {
        let versions = compute_kv_versions(None, log_index);
        assert_eq!(versions.version, 1, "New keys always start at version 1");
        assert_eq!(versions.create_revision, log_index as i64);
        assert_eq!(versions.mod_revision, log_index as i64);
    }

    /// Updating a key increments its version by 1.
    #[test]
    fn test_update_increments_version(
        create_rev in 1i64..1_000_000,
        old_version in 1i64..1_000_000,
        log_index in 1_000_001u64..2_000_000,
    ) {
        let versions = compute_kv_versions(Some((create_rev, old_version)), log_index);
        assert_eq!(versions.version, old_version + 1, "Version must increment by 1");
        assert_eq!(versions.create_revision, create_rev, "create_revision must be preserved");
        assert_eq!(versions.mod_revision, log_index as i64, "mod_revision must be the new log index");
    }

    /// CAS: check_cas_condition(None, None) = true (create-if-absent, key absent).
    #[test]
    fn test_cas_absent_key_absent_expect(_dummy in 0..1u8) {
        assert!(check_cas_condition(None, None));
    }

    /// CAS: check_cas_condition(None, Some(_)) = false (create-if-absent, key exists).
    #[test]
    fn test_cas_absent_key_present(
        current in "[a-zA-Z0-9]{1,20}",
    ) {
        assert!(!check_cas_condition(None, Some(&current)));
    }

    /// CAS: check_cas_condition(Some(x), Some(x)) = true (matching values).
    #[test]
    fn test_cas_matching_values(
        val in "[a-zA-Z0-9]{1,100}",
    ) {
        assert!(check_cas_condition(Some(&val), Some(&val)));
    }

    /// CAS: check_cas_condition(Some(x), Some(y)) = false when x != y.
    #[test]
    fn test_cas_mismatching_values(
        expected in "[a-z]{1,50}",
        current in "[A-Z]{1,50}",
    ) {
        // Upper vs lower guarantees they differ
        assert!(!check_cas_condition(Some(&expected), Some(&current)));
    }

    /// Key expiration: TTL=None means no expiration (returns None).
    #[test]
    fn test_no_ttl_no_expiration(now_ms in any::<u64>()) {
        assert_eq!(compute_key_expiration(None, now_ms), None);
    }

    /// Key expiration: TTL>0 means expiration = now + ttl*1000.
    #[test]
    fn test_ttl_expiration(
        ttl_seconds in 1u32..100_000,
        now_ms in 0u64..u64::MAX / 2,
    ) {
        let result = compute_key_expiration(Some(ttl_seconds), now_ms);
        prop_assert!(result.is_some(), "TTL>0 must yield an expiration timestamp");
        if let Some(expires_at_ms) = result {
            assert!(expires_at_ms > now_ms, "Expiration must be in the future");
        }
    }

    /// Lease: created lease is not expired at creation time.
    #[test]
    fn test_new_lease_not_expired(
        ttl_seconds in 1u32..100_000,
        now_ms in 0u64..u64::MAX / 2,
    ) {
        let lease = create_lease_entry(ttl_seconds, now_ms);
        assert!(
            !is_lease_expired(LeaseExpirationInput { expires_at_ms: lease.expires_at_ms, now_ms }),
            "New lease must not be expired at creation time"
        );
    }

    /// Lease: expired check is monotonic (if expired at T, expired at T+1).
    #[test]
    fn test_lease_expiry_monotonic(
        expires_at_ms in 1u64..u64::MAX,
        now_ms in 1u64..u64::MAX,
    ) {
        if is_lease_expired(LeaseExpirationInput { expires_at_ms, now_ms }) && now_ms < u64::MAX {
            assert!(
                is_lease_expired(LeaseExpirationInput {
                    expires_at_ms,
                    now_ms: now_ms.saturating_add(1),
                }),
                "If expired at T, must be expired at T+1"
            );
        }
    }
}
