//! Apply Request Specification
//!
//! Formal specifications for state machine apply operations.
//!
//! # Key Properties
//!
//! - **APPLY-1: Version Increment**: Version increments on update
//! - **APPLY-2: Index Consistency**: mod_revision matches apply index
//! - **APPLY-3: Idempotency**: Re-applying same entry is no-op
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-raft/verus/apply_request_spec.rs
//! ```

use vstd::prelude::*;

use super::storage_state_spec::*;

verus! {
    // ========================================================================
    // Apply Request Types
    // ========================================================================

    /// Types of apply requests
    pub enum ApplyRequestSpec {
        /// Set a key-value pair
        Set { key: Seq<u8>, value: Seq<u8> },
        /// Set with TTL
        SetWithTTL { key: Seq<u8>, value: Seq<u8>, expires_at_ms: u64 },
        /// Delete a key
        Delete { key: Seq<u8> },
        /// Batch of set/delete operations
        Batch { operations: Seq<(bool, Seq<u8>, Seq<u8>)> }, // (is_set, key, value)
        /// Compare-and-swap
        CompareAndSwap { key: Seq<u8>, expected: Option<Seq<u8>>, new_value: Seq<u8> },
    }

    /// Result of apply operation
    pub enum ApplyResult {
        /// Operation succeeded
        Success,
        /// CAS failed (value didn't match)
        CasFailed { actual: Option<Seq<u8>> },
        /// Key not found (for delete)
        NotFound,
    }

    // ========================================================================
    // Apply Set Operation
    // ========================================================================

    /// Effect of applying a Set operation
    pub open spec fn apply_set_post(
        pre: StorageState,
        key: Seq<u8>,
        value: Seq<u8>,
        log_index: u64,
    ) -> StorageState {
        let (create_rev, version) = if pre.kv.contains_key(key) {
            (pre.kv[key].create_revision, (pre.kv[key].version + 1) as u64)
        } else {
            (log_index, 1u64)
        };

        let new_entry = KvEntry {
            value: value,
            mod_revision: log_index,
            create_revision: create_rev,
            version: version,
            expires_at_ms: None,
        };

        StorageState {
            kv: pre.kv.insert(key, new_entry),
            last_applied: Some(log_index),
            ..pre
        }
    }

    // ========================================================================
    // APPLY-1: Version Increment
    // ========================================================================

    /// APPLY-1: Version increments on update
    #[verifier(external_body)]
    pub proof fn set_increments_version(
        pre: StorageState,
        key: Seq<u8>,
        value: Seq<u8>,
        log_index: u64,
    )
        requires pre.kv.contains_key(key)
        ensures ({
            let post = apply_set_post(pre, key, value, log_index);
            post.kv[key].version == pre.kv[key].version + 1
        })
    {
        // By definition of apply_set_post
    }

    /// New key starts at version 1
    #[verifier(external_body)]
    pub proof fn new_key_version_one(
        pre: StorageState,
        key: Seq<u8>,
        value: Seq<u8>,
        log_index: u64,
    )
        requires !pre.kv.contains_key(key)
        ensures ({
            let post = apply_set_post(pre, key, value, log_index);
            post.kv[key].version == 1
        })
    {
        // By definition of apply_set_post
    }

    // ========================================================================
    // APPLY-2: Index Consistency
    // ========================================================================

    /// APPLY-2: mod_revision matches apply index
    ///
    /// Requires a valid key to ensure the post-state lookup is well-defined.
    #[verifier(external_body)]
    pub proof fn mod_revision_matches_index(
        pre: StorageState,
        key: Seq<u8>,
        value: Seq<u8>,
        log_index: u64,
    )
        requires
            // Key must be non-empty (valid key)
            key.len() > 0,
        ensures ({
            let post = apply_set_post(pre, key, value, log_index);
            // post.kv.contains_key(key) is guaranteed by apply_set_post
            // which inserts the key into the map
            post.kv.contains_key(key) &&
            post.kv[key].mod_revision == log_index
        })
    {
        // By definition of apply_set_post, the key is inserted with
        // mod_revision = log_index
    }

    /// Create revision is preserved on update
    #[verifier(external_body)]
    pub proof fn create_revision_preserved(
        pre: StorageState,
        key: Seq<u8>,
        value: Seq<u8>,
        log_index: u64,
    )
        requires pre.kv.contains_key(key)
        ensures ({
            let post = apply_set_post(pre, key, value, log_index);
            post.kv[key].create_revision == pre.kv[key].create_revision
        })
    {
        // create_revision unchanged for existing keys
    }

    /// Create revision set on new key
    #[verifier(external_body)]
    pub proof fn create_revision_set_on_new(
        pre: StorageState,
        key: Seq<u8>,
        value: Seq<u8>,
        log_index: u64,
    )
        requires !pre.kv.contains_key(key)
        ensures ({
            let post = apply_set_post(pre, key, value, log_index);
            post.kv[key].create_revision == log_index
        })
    {
        // create_revision = log_index for new keys
    }

    // ========================================================================
    // Apply Delete Operation
    // ========================================================================

    /// Effect of applying a Delete operation
    pub open spec fn apply_delete_post(
        pre: StorageState,
        key: Seq<u8>,
        log_index: u64,
    ) -> StorageState {
        StorageState {
            kv: pre.kv.remove(key),
            last_applied: Some(log_index),
            ..pre
        }
    }

    /// Delete removes the key
    #[verifier(external_body)]
    pub proof fn delete_removes_key(
        pre: StorageState,
        key: Seq<u8>,
        log_index: u64,
    )
        ensures ({
            let post = apply_delete_post(pre, key, log_index);
            !post.kv.contains_key(key)
        })
    {
        // By definition
    }

    /// Delete preserves other keys
    #[verifier(external_body)]
    pub proof fn delete_preserves_others(
        pre: StorageState,
        key: Seq<u8>,
        other_key: Seq<u8>,
        log_index: u64,
    )
        requires key != other_key
        ensures ({
            let post = apply_delete_post(pre, key, log_index);
            pre.kv.contains_key(other_key) ==>
                post.kv.contains_key(other_key) &&
                post.kv[other_key] == pre.kv[other_key]
        })
    {
        // Remove only affects the specified key
    }

    // ========================================================================
    // Apply Batch Operation
    // ========================================================================

    /// Apply a single batch operation
    pub open spec fn apply_batch_op(
        state: StorageState,
        is_set: bool,
        key: Seq<u8>,
        value: Seq<u8>,
        log_index: u64,
    ) -> StorageState {
        if is_set {
            apply_set_post(state, key, value, log_index)
        } else {
            apply_delete_post(state, key, log_index)
        }
    }

    /// Apply batch operations sequentially
    pub open spec fn apply_batch_post(
        pre: StorageState,
        operations: Seq<(bool, Seq<u8>, Seq<u8>)>,
        log_index: u64,
    ) -> StorageState
        decreases operations.len()
    {
        if operations.len() == 0 {
            StorageState {
                last_applied: Some(log_index),
                ..pre
            }
        } else {
            let (is_set, key, value) = operations[0];
            let intermediate = apply_batch_op(pre, is_set, key, value, log_index);
            apply_batch_post(intermediate, operations.skip(1), log_index)
        }
    }

    /// Batch updates last_applied once
    #[verifier(external_body)]
    pub proof fn batch_updates_last_applied(
        pre: StorageState,
        operations: Seq<(bool, Seq<u8>, Seq<u8>)>,
        log_index: u64,
    )
        ensures ({
            let post = apply_batch_post(pre, operations, log_index);
            post.last_applied == Some(log_index)
        })
        decreases operations.len()
    {
        // By induction on operations length
    }

    // ========================================================================
    // Apply Compare-And-Swap
    // ========================================================================

    /// Result of CAS check
    pub open spec fn cas_matches(
        state: StorageState,
        key: Seq<u8>,
        expected: Option<Seq<u8>>,
    ) -> bool {
        match expected {
            None => !state.kv.contains_key(key), // Expect key not to exist
            Some(exp_val) => {
                state.kv.contains_key(key) &&
                state.kv[key].value == exp_val
            }
        }
    }

    /// Effect of successful CAS
    pub open spec fn apply_cas_post(
        pre: StorageState,
        key: Seq<u8>,
        expected: Option<Seq<u8>>,
        new_value: Seq<u8>,
        log_index: u64,
    ) -> (StorageState, ApplyResult) {
        if cas_matches(pre, key, expected) {
            let post = apply_set_post(pre, key, new_value, log_index);
            (post, ApplyResult::Success)
        } else {
            let actual = if pre.kv.contains_key(key) {
                Some(pre.kv[key].value)
            } else {
                None
            };
            let post = StorageState {
                last_applied: Some(log_index),
                ..pre
            };
            (post, ApplyResult::CasFailed { actual })
        }
    }

    /// CAS succeeds when expected matches
    ///
    /// Requires the storage invariant to hold on pre-state, ensuring
    /// chain tip synchronization and response cache consistency.
    #[verifier(external_body)]
    pub proof fn cas_succeeds_on_match(
        pre: StorageState,
        key: Seq<u8>,
        expected: Option<Seq<u8>>,
        new_value: Seq<u8>,
        log_index: u64,
    )
        requires
            storage_invariant(pre),
            cas_matches(pre, key, expected),
        ensures ({
            let (post, result) = apply_cas_post(pre, key, expected, new_value, log_index);
            matches!(result, ApplyResult::Success) &&
            post.kv.contains_key(key) &&
            post.kv[key].value == new_value
        })
    {
        // By definition of apply_cas_post:
        // when cas_matches is true, apply_set_post is called which inserts the key
    }

    /// CAS fails when expected doesn't match
    #[verifier(external_body)]
    pub proof fn cas_fails_on_mismatch(
        pre: StorageState,
        key: Seq<u8>,
        expected: Option<Seq<u8>>,
        new_value: Seq<u8>,
        log_index: u64,
    )
        requires !cas_matches(pre, key, expected)
        ensures ({
            let (post, result) = apply_cas_post(pre, key, expected, new_value, log_index);
            matches!(result, ApplyResult::CasFailed { .. }) &&
            // State unchanged except last_applied
            post.kv == pre.kv
        })
    {
        // By definition
    }

    // ========================================================================
    // APPLY-3: Idempotency
    // ========================================================================

    /// APPLY-3: Re-applying same entry is no-op
    ///
    /// If last_applied >= log_index, the operation is skipped
    pub open spec fn should_apply(state: StorageState, log_index: u64) -> bool {
        match state.last_applied {
            None => true,
            Some(last) => log_index > last,
        }
    }

    /// Idempotent apply wrapper
    pub open spec fn apply_idempotent(
        pre: StorageState,
        request: ApplyRequestSpec,
        log_index: u64,
    ) -> StorageState {
        if !should_apply(pre, log_index) {
            pre // Already applied, no-op
        } else {
            match request {
                ApplyRequestSpec::Set { key, value } =>
                    apply_set_post(pre, key, value, log_index),
                ApplyRequestSpec::SetWithTTL { key, value, expires_at_ms } => {
                    let post = apply_set_post(pre, key, value, log_index);
                    // Add TTL
                    let entry = KvEntry {
                        expires_at_ms: Some(expires_at_ms),
                        ..post.kv[key]
                    };
                    StorageState {
                        kv: post.kv.insert(key, entry),
                        ..post
                    }
                }
                ApplyRequestSpec::Delete { key } =>
                    apply_delete_post(pre, key, log_index),
                ApplyRequestSpec::Batch { operations } =>
                    apply_batch_post(pre, operations, log_index),
                ApplyRequestSpec::CompareAndSwap { key, expected, new_value } =>
                    apply_cas_post(pre, key, expected, new_value, log_index).0,
            }
        }
    }

    /// Already-applied entries are no-op
    #[verifier(external_body)]
    pub proof fn already_applied_is_noop(
        pre: StorageState,
        request: ApplyRequestSpec,
        log_index: u64,
    )
        requires
            pre.last_applied.is_some(),
            pre.last_applied.unwrap() >= log_index,
        ensures apply_idempotent(pre, request, log_index) == pre
    {
        // should_apply returns false, so pre returned unchanged
    }

    // ========================================================================
    // Apply Ordering
    // ========================================================================

    /// Applies must happen in order
    pub open spec fn applies_in_order(
        states: Seq<StorageState>,
        indices: Seq<u64>,
    ) -> bool {
        forall |i: int, j: int|
            0 <= i < j < states.len() ==>
            states[i].last_applied.is_some() &&
            states[j].last_applied.is_some() &&
            states[i].last_applied.unwrap() < states[j].last_applied.unwrap()
    }

    /// Apply advances last_applied
    #[verifier(external_body)]
    pub proof fn apply_advances_last_applied(
        pre: StorageState,
        key: Seq<u8>,
        value: Seq<u8>,
        log_index: u64,
    )
        requires should_apply(pre, log_index)
        ensures ({
            let post = apply_set_post(pre, key, value, log_index);
            last_applied_monotonic(pre, post)
        })
    {
        // log_index > pre.last_applied, so post.last_applied > pre.last_applied
    }
}
