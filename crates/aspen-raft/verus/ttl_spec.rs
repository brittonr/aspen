//! TTL (Time-To-Live) Specification
//!
//! Formal specifications for key expiration via TTL.
//!
//! # Key Properties
//!
//! - **TTL-1: Expired Not Returned**: Expired keys not returned by get()
//! - **TTL-2: Cleanup Correctness**: Cleanup removes only expired keys
//! - **TTL-3: TTL Monotonicity**: Expiration time is immutable after set
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-raft/verus/ttl_spec.rs
//! ```

use vstd::prelude::*;

use super::storage_state_spec::*;

verus! {
    // ========================================================================
    // TTL State Model
    // ========================================================================

    /// Check if a key has TTL set
    pub open spec fn has_ttl(entry: KvEntry) -> bool {
        entry.expires_at_ms.is_some()
    }

    /// Check if a key is expired at given time
    pub open spec fn is_expired(entry: KvEntry, current_time_ms: u64) -> bool {
        match entry.expires_at_ms {
            Some(expires_at) => current_time_ms >= expires_at,
            None => false, // No TTL means never expires
        }
    }

    /// Check if a key is live (not expired) at given time
    pub open spec fn is_live(entry: KvEntry, current_time_ms: u64) -> bool {
        !is_expired(entry, current_time_ms)
    }

    /// Time remaining until expiration
    pub open spec fn time_remaining_ms(entry: KvEntry, current_time_ms: u64) -> Option<u64> {
        match entry.expires_at_ms {
            Some(expires_at) => {
                if current_time_ms >= expires_at {
                    Some(0) // Already expired
                } else {
                    Some(expires_at - current_time_ms)
                }
            }
            None => None, // No TTL
        }
    }

    // ========================================================================
    // TTL-1: Expired Keys Not Returned
    // ========================================================================

    /// Get operation with TTL awareness
    pub open spec fn get_with_ttl(
        state: StorageState,
        key: Seq<u8>,
        current_time_ms: u64,
    ) -> Option<Seq<u8>> {
        if !state.kv.contains_key(key) {
            None
        } else {
            let entry = state.kv[key];
            if is_expired(entry, current_time_ms) {
                None // TTL-1: Don't return expired keys
            } else {
                Some(entry.value)
            }
        }
    }

    /// TTL-1: Expired keys are not returned by get
    pub proof fn expired_not_returned(
        state: StorageState,
        key: Seq<u8>,
        current_time_ms: u64,
    )
        requires
            state.kv.contains_key(key),
            is_expired(state.kv[key], current_time_ms),
        ensures get_with_ttl(state, key, current_time_ms).is_none()
    {
        // By definition of get_with_ttl
    }

    /// Live keys are returned
    pub proof fn live_keys_returned(
        state: StorageState,
        key: Seq<u8>,
        current_time_ms: u64,
    )
        requires
            state.kv.contains_key(key),
            is_live(state.kv[key], current_time_ms),
        ensures get_with_ttl(state, key, current_time_ms) == Some(state.kv[key].value)
    {
        // By definition of get_with_ttl
    }

    // ========================================================================
    // TTL-2: Cleanup Correctness
    // ========================================================================

    /// Set of expired keys
    pub open spec fn expired_keys(
        state: StorageState,
        current_time_ms: u64,
    ) -> Set<Seq<u8>> {
        Set::new(|k: Seq<u8>|
            state.kv.contains_key(k) &&
            is_expired(state.kv[k], current_time_ms)
        )
    }

    /// Set of live keys
    pub open spec fn live_keys(
        state: StorageState,
        current_time_ms: u64,
    ) -> Set<Seq<u8>> {
        Set::new(|k: Seq<u8>|
            state.kv.contains_key(k) &&
            is_live(state.kv[k], current_time_ms)
        )
    }

    /// Effect of TTL cleanup operation
    pub open spec fn cleanup_expired_post(
        pre: StorageState,
        current_time_ms: u64,
    ) -> StorageState {
        let expired = expired_keys(pre, current_time_ms);
        StorageState {
            kv: pre.kv.remove_keys(expired),
            ..pre
        }
    }

    /// TTL-2: Cleanup removes only expired keys
    pub proof fn cleanup_removes_only_expired(
        pre: StorageState,
        current_time_ms: u64,
    )
        ensures {
            let post = cleanup_expired_post(pre, current_time_ms);
            // All live keys preserved
            forall |k: Seq<u8>|
                pre.kv.contains_key(k) && is_live(pre.kv[k], current_time_ms) ==>
                post.kv.contains_key(k) && post.kv[k] == pre.kv[k]
        }
    {
        // Live keys are not in expired set, so they're preserved
    }

    /// Cleanup removes all expired keys
    pub proof fn cleanup_removes_all_expired(
        pre: StorageState,
        current_time_ms: u64,
    )
        ensures {
            let post = cleanup_expired_post(pre, current_time_ms);
            // No expired keys remain
            forall |k: Seq<u8>|
                post.kv.contains_key(k) ==>
                is_live(post.kv[k], current_time_ms)
        }
    {
        // All expired keys removed
    }

    // ========================================================================
    // TTL-3: TTL Monotonicity
    // ========================================================================

    /// TTL is set at creation and doesn't change
    pub open spec fn ttl_unchanged(
        pre_entry: KvEntry,
        post_entry: KvEntry,
    ) -> bool {
        pre_entry.expires_at_ms == post_entry.expires_at_ms
    }

    /// Expiration time only set during initial write
    /// Updates preserve existing TTL unless explicitly renewed
    pub open spec fn ttl_preserved_on_update(
        pre: StorageState,
        post: StorageState,
        key: Seq<u8>,
    ) -> bool {
        pre.kv.contains_key(key) && post.kv.contains_key(key) ==>
            // Either TTL unchanged, or version changed (new write)
            pre.kv[key].expires_at_ms == post.kv[key].expires_at_ms ||
            post.kv[key].version != pre.kv[key].version
    }

    // ========================================================================
    // Set With TTL Operation
    // ========================================================================

    /// Effect of set with TTL
    pub open spec fn set_with_ttl_post(
        pre: StorageState,
        key: Seq<u8>,
        value: Seq<u8>,
        ttl_seconds: u64,
        current_time_ms: u64,
        mod_revision: u64,
    ) -> StorageState {
        let expires_at = current_time_ms + (ttl_seconds * 1000);

        let (create_rev, version) = if pre.kv.contains_key(key) {
            (pre.kv[key].create_revision, pre.kv[key].version + 1)
        } else {
            (mod_revision, 1)
        };

        let new_entry = KvEntry {
            value: value,
            mod_revision: mod_revision,
            create_revision: create_rev,
            version: version,
            expires_at_ms: Some(expires_at),
        };

        StorageState {
            kv: pre.kv.insert(key, new_entry),
            ..pre
        }
    }

    /// Set with TTL creates entry with correct expiration
    pub proof fn set_with_ttl_correct_expiration(
        pre: StorageState,
        key: Seq<u8>,
        value: Seq<u8>,
        ttl_seconds: u64,
        current_time_ms: u64,
        mod_revision: u64,
    )
        ensures {
            let post = set_with_ttl_post(pre, key, value, ttl_seconds, current_time_ms, mod_revision);
            post.kv[key].expires_at_ms == Some(current_time_ms + (ttl_seconds * 1000))
        }
    {
        // By definition
    }

    /// New key is live immediately after set
    pub proof fn set_with_ttl_initially_live(
        pre: StorageState,
        key: Seq<u8>,
        value: Seq<u8>,
        ttl_seconds: u64,
        current_time_ms: u64,
        mod_revision: u64,
    )
        requires ttl_seconds > 0
        ensures {
            let post = set_with_ttl_post(pre, key, value, ttl_seconds, current_time_ms, mod_revision);
            is_live(post.kv[key], current_time_ms)
        }
    {
        // expires_at = current_time + ttl > current_time when ttl > 0
    }

    // ========================================================================
    // Time Progression
    // ========================================================================

    /// As time progresses, keys may transition from live to expired
    ///
    /// If expires_at <= time2_ms, the key is expired at time2.
    /// If expires_at > time2_ms (or no TTL), the key is still live.
    pub proof fn time_progression_may_expire(
        state: StorageState,
        time1_ms: u64,
        time2_ms: u64,
        key: Seq<u8>,
    )
        requires
            time1_ms < time2_ms,
            state.kv.contains_key(key),
            is_live(state.kv[key], time1_ms),
        ensures
            // If expires_at is set and <= time2, the key is expired
            state.kv[key].expires_at_ms.is_some() &&
            state.kv[key].expires_at_ms.unwrap() <= time2_ms ==>
                is_expired(state.kv[key], time2_ms),
            // If expires_at is not set or > time2, the key is still live
            (state.kv[key].expires_at_ms.is_none() ||
             state.kv[key].expires_at_ms.unwrap() > time2_ms) ==>
                is_live(state.kv[key], time2_ms),
    {
        // Follows from is_expired definition:
        // is_expired(entry, time) = entry.expires_at_ms.is_some() &&
        //                           time >= entry.expires_at_ms.unwrap()
    }

    /// Once expired, always expired
    pub proof fn expired_stays_expired(
        entry: KvEntry,
        time1_ms: u64,
        time2_ms: u64,
    )
        requires
            time1_ms <= time2_ms,
            is_expired(entry, time1_ms),
        ensures is_expired(entry, time2_ms)
    {
        // time2 >= time1 >= expires_at => time2 >= expires_at
    }

    // ========================================================================
    // Scan With TTL
    // ========================================================================

    /// Filter scan results by TTL
    pub open spec fn scan_with_ttl(
        state: StorageState,
        prefix: Seq<u8>,
        current_time_ms: u64,
    ) -> Map<Seq<u8>, Seq<u8>> {
        Map::new(
            |k: Seq<u8>|
                state.kv.contains_key(k) &&
                k.len() >= prefix.len() &&
                k.take(prefix.len() as int) == prefix &&
                is_live(state.kv[k], current_time_ms),
            |k: Seq<u8>| state.kv[k].value
        )
    }

    /// Scan excludes expired keys
    pub proof fn scan_excludes_expired(
        state: StorageState,
        prefix: Seq<u8>,
        current_time_ms: u64,
        key: Seq<u8>,
    )
        requires
            state.kv.contains_key(key),
            key.len() >= prefix.len(),
            key.take(prefix.len() as int) == prefix,
            is_expired(state.kv[key], current_time_ms),
        ensures {
            let results = scan_with_ttl(state, prefix, current_time_ms);
            !results.contains_key(key)
        }
    {
        // Expired keys don't satisfy is_live predicate
    }
}

mod storage_state_spec;
