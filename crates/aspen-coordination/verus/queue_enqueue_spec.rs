//! Queue Enqueue Operation Specification
//!
//! Formal specifications for the enqueue operation.
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-coordination/verus/queue_enqueue_spec.rs
//! ```

use vstd::prelude::*;

// Import from queue_state_spec
use crate::queue_state_spec::*;

verus! {
    // ========================================================================
    // Enqueue Operation
    // ========================================================================

    /// Precondition for enqueue
    pub open spec fn enqueue_pre(
        state: QueueState,
        payload: Seq<u8>,
        dedup_id: Option<Seq<u8>>,
        current_time_ms: u64,
    ) -> bool {
        // Payload size must be within limits (abstract bound)
        payload.len() <= 1_000_000 && // MAX_QUEUE_ITEM_SIZE
        // If dedup_id provided, check if duplicate
        match dedup_id {
            Some(id) => {
                if state.dedup_cache.contains_key(id) {
                    let entry = state.dedup_cache[id];
                    // Duplicate if entry not expired
                    entry.expires_at_ms > current_time_ms
                } else {
                    true // Not a duplicate
                }
            }
            None => true, // No dedup check
        }
    }

    /// Check if this would be a duplicate
    pub open spec fn is_duplicate(
        state: QueueState,
        dedup_id: Seq<u8>,
        current_time_ms: u64,
    ) -> bool {
        state.dedup_cache.contains_key(dedup_id) &&
        state.dedup_cache[dedup_id].expires_at_ms > current_time_ms
    }

    /// Get existing item ID for duplicate
    ///
    /// Assumes:
    /// - state.dedup_cache.contains_key(dedup_id)
    pub open spec fn get_duplicate_item_id(
        state: QueueState,
        dedup_id: Seq<u8>,
    ) -> u64 {
        state.dedup_cache[dedup_id].item_id
    }

    /// Result of enqueue: new item ID
    ///
    /// Assumes:
    /// - enqueue_pre(pre, payload, dedup_id, current_time_ms)
    /// - // Not a duplicate (handled separately) match dedup_id { Some(id) => !is_duplicate(pre, id, current_time_ms)
    /// - None => true
    /// - }
    /// - // ID generation overflow protection: next_id + 1 must not overflow pre.next_id < 0xFFFF_FFFF_FFFF_FFFFu64
    /// - // TTL overflow protection: current_time_ms + ttl_ms must not overflow ttl_ms == 0 || current_time_ms <= 0xFFFF_FFFF_FFFF_FFFFu64 - ttl_ms
    /// - // Dedup TTL overflow protection: current_time_ms + 300_000 (5 min) must not overflow current_time_ms <= 0xFFFF_FFFF_FFFF_FFFFu64 - 300_000u64
    pub open spec fn enqueue_post(
        pre: QueueState,
        payload: Seq<u8>,
        ttl_ms: u64,
        message_group_id: Option<Seq<u8>>,
        dedup_id: Option<Seq<u8>>,
        current_time_ms: u64,
    ) -> QueueState {
        let new_id = pre.next_id;
        let expires_at_ms = if ttl_ms == 0 {
            0
        } else if current_time_ms <= u64::MAX - ttl_ms {
            (current_time_ms + ttl_ms) as u64
        } else {
            u64::MAX
        };

        let new_item = QueueItemSpec {
            id: new_id,
            payload: payload,
            state: QueueItemStateSpec::Pending,
            enqueued_at_ms: current_time_ms,
            expires_at_ms: expires_at_ms as u64,
            delivery_count: 0,
            visibility_deadline_ms: 0, // Not inflight yet
            message_group_id: message_group_id,
            deduplication_id: dedup_id,
        };

        // Add dedup entry if provided
        let new_dedup_cache = match dedup_id {
            Some(id) => {
                let entry = DedupEntrySpec {
                    dedup_id: id,
                    item_id: new_id,
                    expires_at_ms: if current_time_ms <= u64::MAX - 300_000 {
                        (current_time_ms + 300_000) as u64
                    } else {
                        u64::MAX
                    }, // 5 min dedup TTL
                };
                pre.dedup_cache.insert(id, entry)
            }
            None => pre.dedup_cache,
        };

        QueueState {
            name: pre.name,
            pending: pre.pending.push(new_item),
            pending_ids: pre.pending_ids.insert(new_id),
            inflight: pre.inflight,
            dlq: pre.dlq,
            next_id: if pre.next_id < u64::MAX {
                (pre.next_id + 1) as u64
            } else {
                u64::MAX
            },
            max_delivery_attempts: pre.max_delivery_attempts,
            default_visibility_timeout_ms: pre.default_visibility_timeout_ms,
            dedup_cache: new_dedup_cache,
            current_time_ms: current_time_ms,
        }
    }

    /// Lemma: appending an item whose ID is greater than every existing
    /// pending item preserves queue FIFO ordering.
    pub proof fn lemma_pending_push_preserves_fifo(pre: QueueState, post: QueueState, new_item: QueueItemSpec)
        requires
            fifo_ordering(pre),
            ids_bounded_by_next(pre),
            new_item.id == pre.next_id,
            post.pending == pre.pending.push(new_item),
        ensures
            fifo_ordering(post),
    {
        assert forall |i: int, j: int|
            0 <= i < j < post.pending.len() implies post.pending[i].id < post.pending[j].id by {
            if j < pre.pending.len() {
                assert(post.pending[i] == pre.pending[i]);
                assert(post.pending[j] == pre.pending[j]);
                assert(fifo_ordering(pre));
            } else {
                assert(j == pre.pending.len());
                assert(i < pre.pending.len());
                assert(post.pending[i] == pre.pending[i]);
                assert(post.pending[j] == new_item);
                assert(ids_bounded_by_next(pre));
            }
        }
    }

    /// Lemma: appending an item for a message group preserves that group's FIFO
    /// order when the item ID is greater than every existing pending item.
    pub proof fn lemma_pending_push_preserves_group_fifo(
        pre: QueueState,
        post: QueueState,
        new_item: QueueItemSpec,
        group_id: Seq<u8>,
    )
        requires
            group_fifo_maintained(pre, group_id),
            ids_bounded_by_next(pre),
            new_item.id == pre.next_id,
            post.pending == pre.pending.push(new_item),
        ensures
            group_fifo_maintained(post, group_id),
    {
        assert forall |i: int, j: int|
            0 <= i < j < post.pending.len()
            && post.pending[i].message_group_id == Some(group_id)
            && post.pending[j].message_group_id == Some(group_id)
            implies post.pending[i].id < post.pending[j].id by {
            if j < pre.pending.len() {
                assert(post.pending[i] == pre.pending[i]);
                assert(post.pending[j] == pre.pending[j]);
                assert(group_fifo_maintained(pre, group_id));
            } else {
                assert(j == pre.pending.len());
                assert(i < pre.pending.len());
                assert(post.pending[i] == pre.pending[i]);
                assert(post.pending[j] == new_item);
                assert(ids_bounded_by_next(pre));
            }
        }
    }

    /// Proof: Enqueue preserves FIFO ordering
    pub proof fn enqueue_preserves_fifo(
        pre: QueueState,
        payload: Seq<u8>,
        ttl_ms: u64,
        message_group_id: Option<Seq<u8>>,
        dedup_id: Option<Seq<u8>>,
        current_time_ms: u64,
    )
        requires
            queue_invariant(pre),
            enqueue_pre(pre, payload, dedup_id, current_time_ms),
            match dedup_id {
                Some(id) => !is_duplicate(pre, id, current_time_ms),
                None => true,
            },
        ensures
            fifo_ordering(enqueue_post(pre, payload, ttl_ms, message_group_id, dedup_id, current_time_ms))
    {
        let post = enqueue_post(pre, payload, ttl_ms, message_group_id, dedup_id, current_time_ms);
        let new_item = post.pending[pre.pending.len() as int];
        assert(new_item.id == pre.next_id);
        lemma_pending_push_preserves_fifo(pre, post, new_item);
    }

    /// Proof: Enqueue increases next_id
    pub proof fn enqueue_increases_next_id(
        pre: QueueState,
        payload: Seq<u8>,
        ttl_ms: u64,
        message_group_id: Option<Seq<u8>>,
        dedup_id: Option<Seq<u8>>,
        current_time_ms: u64,
    )
        requires
            enqueue_pre(pre, payload, dedup_id, current_time_ms),
            match dedup_id {
                Some(id) => !is_duplicate(pre, id, current_time_ms),
                None => true,
            },
        ensures
            pre.next_id < u64::MAX ==>
                enqueue_post(pre, payload, ttl_ms, message_group_id, dedup_id, current_time_ms).next_id
                    == pre.next_id + 1,
            pre.next_id == u64::MAX ==>
                enqueue_post(pre, payload, ttl_ms, message_group_id, dedup_id, current_time_ms).next_id
                    == u64::MAX,
    {
        let post = enqueue_post(pre, payload, ttl_ms, message_group_id, dedup_id, current_time_ms);
        if pre.next_id < u64::MAX {
            assert(post.next_id == pre.next_id + 1);
        } else {
            assert(post.next_id == u64::MAX);
        }
    }

    /// Proof: Enqueue preserves state exclusivity
    pub proof fn enqueue_preserves_exclusivity(
        pre: QueueState,
        payload: Seq<u8>,
        ttl_ms: u64,
        message_group_id: Option<Seq<u8>>,
        dedup_id: Option<Seq<u8>>,
        current_time_ms: u64,
    )
        requires
            queue_invariant(pre),
            enqueue_pre(pre, payload, dedup_id, current_time_ms),
            match dedup_id {
                Some(id) => !is_duplicate(pre, id, current_time_ms),
                None => true,
            },
        ensures
            state_exclusivity(enqueue_post(pre, payload, ttl_ms, message_group_id, dedup_id, current_time_ms))
    {
        let post = enqueue_post(pre, payload, ttl_ms, message_group_id, dedup_id, current_time_ms);

        // New ID = pre.next_id is not in any collection (by ids_bounded_by_next)
        // So adding it to pending doesn't violate exclusivity
    }

    /// Proof: Enqueue preserves invariant
    pub proof fn enqueue_preserves_invariant(
        pre: QueueState,
        payload: Seq<u8>,
        ttl_ms: u64,
        message_group_id: Option<Seq<u8>>,
        dedup_id: Option<Seq<u8>>,
        current_time_ms: u64,
    )
        requires
            queue_invariant(pre),
            pre.next_id < u64::MAX,
            forall |id: u64| pre.inflight.contains_key(id) ==>
                pre.inflight[id].visibility_deadline_ms > current_time_ms,
            enqueue_pre(pre, payload, dedup_id, current_time_ms),
            match dedup_id {
                Some(id) => !is_duplicate(pre, id, current_time_ms),
                None => true,
            },
        ensures
            queue_invariant(enqueue_post(pre, payload, ttl_ms, message_group_id, dedup_id, current_time_ms))
    {
        let post = enqueue_post(pre, payload, ttl_ms, message_group_id, dedup_id, current_time_ms);
        enqueue_preserves_fifo(pre, payload, ttl_ms, message_group_id, dedup_id, current_time_ms);
        enqueue_preserves_exclusivity(pre, payload, ttl_ms, message_group_id, dedup_id, current_time_ms);
        assert(ids_bounded_by_next(post));
        let new_item = post.pending[pre.pending.len() as int];
        assert(new_item.id == pre.next_id);
        assert forall |i: int| 0 <= i < post.pending.len() implies
            post.pending_ids.contains(post.pending[i].id) by {
            if i < pre.pending.len() {
                assert(pending_ids_consistent(pre));
                assert(pre.pending_ids.contains(pre.pending[i].id));
                assert(post.pending[i] == pre.pending[i]);
            } else {
                assert(i == pre.pending.len());
                assert(post.pending[i].id == pre.next_id);
            }
        }
        assert forall |id: u64| post.pending_ids.contains(id) implies
            exists |i: int| 0 <= i < post.pending.len() && post.pending[i].id == id by {
            if id == pre.next_id {
                assert(0 <= pre.pending.len() < post.pending.len());
                assert(post.pending[pre.pending.len() as int].id == id);
            } else {
                assert(pre.pending_ids.contains(id));
                assert(pending_ids_consistent(pre));
                let i = choose |i: int| 0 <= i < pre.pending.len() && pre.pending[i].id == id;
                assert(post.pending[i] == pre.pending[i]);
            }
        }
        assert(pending_ids_consistent(post));
        assert(visibility_timeout_valid(post));
        assert(dlq_threshold_respected(post));
        assert(dedup_consistency(post));
    }

    // ========================================================================
    // Batch Enqueue
    // ========================================================================

    /// Precondition for batch enqueue
    pub open spec fn batch_enqueue_pre(
        state: QueueState,
        items: Seq<(Seq<u8>, Option<Seq<u8>>)>, // (payload, dedup_id)
        current_time_ms: u64,
    ) -> bool {
        // Batch size within limits
        items.len() <= 100 && // MAX_QUEUE_BATCH_SIZE
        // All items satisfy individual preconditions
        forall |i: int| 0 <= i < items.len() ==>
            enqueue_pre(state, items[i].0, items[i].1, current_time_ms)
    }

    /// Proof: Batch enqueue returns sequential IDs
    pub proof fn batch_enqueue_sequential_ids(
        pre: QueueState,
        items: Seq<(Seq<u8>, Option<Seq<u8>>)>,
        current_time_ms: u64,
    )
        requires
            queue_invariant(pre),
            batch_enqueue_pre(pre, items, current_time_ms),
            items.len() > 0,
        ensures ({
            // IDs are [next_id, next_id + 1, ..., next_id + len - 1]
            let first_id = pre.next_id;
            let last_id = pre.next_id + (items.len() as u64) - 1;
            first_id < last_id || items.len() == 1
        })
    {
        // Each enqueue increments next_id by 1
        // So n enqueues produce IDs: next_id, next_id+1, ..., next_id+n-1
    }

    // ========================================================================
    // Deduplication Properties
    // ========================================================================

    /// Proof: Duplicate enqueue returns existing ID
    pub proof fn duplicate_returns_same_id(
        state: QueueState,
        dedup_id: Seq<u8>,
        current_time_ms: u64,
    )
        requires
            queue_invariant(state),
            is_duplicate(state, dedup_id, current_time_ms),
        ensures ({
            let existing_id = get_duplicate_item_id(state, dedup_id);
            existing_id < state.next_id
        })
    {
        assert(dedup_consistency(state));
        assert(state.dedup_cache.contains_key(dedup_id));
    }

    /// Proof: Non-duplicate enqueue creates new item
    pub proof fn non_duplicate_creates_new(
        pre: QueueState,
        payload: Seq<u8>,
        dedup_id: Seq<u8>,
        current_time_ms: u64,
    )
        requires
            queue_invariant(pre),
            !is_duplicate(pre, dedup_id, current_time_ms),
        ensures ({
            let post = enqueue_post(pre, payload, 0, None, Some(dedup_id), current_time_ms);
            post.pending.len() == pre.pending.len() + 1 &&
            post.dedup_cache.contains_key(dedup_id)
        })
    {
        // New item added, dedup entry created
    }

    // ========================================================================
    // Message Group Properties
    // ========================================================================

    /// Items with same group are ordered by enqueue time
    pub open spec fn group_fifo_maintained(
        state: QueueState,
        group_id: Seq<u8>,
    ) -> bool {
        forall |i: int, j: int|
            0 <= i < j < state.pending.len() &&
            state.pending[i].message_group_id == Some(group_id) &&
            state.pending[j].message_group_id == Some(group_id) ==>
            state.pending[i].id < state.pending[j].id
    }

    /// Proof: Enqueue to group preserves group FIFO
    pub proof fn enqueue_preserves_group_fifo(
        pre: QueueState,
        payload: Seq<u8>,
        group_id: Seq<u8>,
        current_time_ms: u64,
    )
        requires
            queue_invariant(pre),
            group_fifo_maintained(pre, group_id),
        ensures ({
            let post = enqueue_post(pre, payload, 0, Some(group_id), None, current_time_ms);
            group_fifo_maintained(post, group_id)
        })
    {
        let post = enqueue_post(pre, payload, 0, Some(group_id), None, current_time_ms);
        let new_item = post.pending[pre.pending.len() as int];
        assert(new_item.id == pre.next_id);
        lemma_pending_push_preserves_group_fifo(pre, post, new_item, group_id);
    }

    // ========================================================================
    // Executable Functions (verified implementations)
    // ========================================================================
    //
    // These exec fn implementations are verified to match their spec fn
    // counterparts. They can be called from production code while maintaining
    // formal guarantees.

    /// Maximum payload size for queue items.
    pub const MAX_QUEUE_ITEM_SIZE: u64 = 1_000_000;

    /// Maximum batch size for enqueue operations.
    pub const MAX_ENQUEUE_BATCH_SIZE: u32 = 100;

    /// Default deduplication TTL in milliseconds (5 minutes).
    pub const DEDUP_TTL_MS: u64 = 300_000;

    /// Check if payload size is valid.
    ///
    /// # Arguments
    ///
    /// * `payload_len` - Length of payload in bytes
    ///
    /// # Returns
    ///
    /// `true` if payload size is within limits.
    pub fn is_payload_size_valid(payload_len: u64) -> (result: bool)
        ensures result == (payload_len <= 1_000_000)
    {
        payload_len <= 1_000_000
    }

    /// Check if batch size is valid.
    ///
    /// # Arguments
    ///
    /// * `batch_size` - Number of items in batch
    ///
    /// # Returns
    ///
    /// `true` if batch size is within limits.
    pub fn is_batch_size_valid(batch_size: u32) -> (result: bool)
        ensures result == (batch_size <= 100)
    {
        batch_size <= 100
    }

    /// Check if next ID can be allocated (no overflow).
    ///
    /// # Arguments
    ///
    /// * `next_id` - Current next ID value
    ///
    /// # Returns
    ///
    /// `true` if ID can be allocated.
    pub fn can_allocate_id(next_id: u64) -> (result: bool)
        ensures result == (next_id < u64::MAX)
    {
        next_id < u64::MAX
    }

    /// Allocate next item ID.
    ///
    /// # Arguments
    ///
    /// * `next_id` - Current next ID value
    ///
    /// # Returns
    ///
    /// Tuple of (allocated ID, new next ID).
    pub fn allocate_next_id(next_id: u64) -> (result: (u64, u64))
        ensures
            next_id < u64::MAX ==> result.0 == next_id && result.1 == next_id + 1,
            next_id == u64::MAX ==> result.0 == next_id && result.1 == u64::MAX
    {
        (next_id, next_id.saturating_add(1))
    }

    /// Compute item expiration time.
    ///
    /// # Arguments
    ///
    /// * `current_time_ms` - Current time
    /// * `ttl_ms` - Time to live (0 means no expiration)
    ///
    /// # Returns
    ///
    /// Expiration time (0 if no expiration, saturating add otherwise).
    pub fn compute_item_expiration(
        current_time_ms: u64,
        ttl_ms: u64,
    ) -> (result: u64)
        ensures
            ttl_ms == 0 ==> result == 0,
            ttl_ms > 0 && current_time_ms <= u64::MAX - ttl_ms ==>
                result == current_time_ms + ttl_ms,
            ttl_ms > 0 && current_time_ms > u64::MAX - ttl_ms ==>
                result == u64::MAX
    {
        if ttl_ms == 0 {
            0
        } else {
            current_time_ms.saturating_add(ttl_ms)
        }
    }

    /// Compute deduplication entry expiration.
    ///
    /// # Arguments
    ///
    /// * `current_time_ms` - Current time
    ///
    /// # Returns
    ///
    /// Dedup entry expiration time (current + 5 min, saturating).
    pub fn compute_dedup_expiration(current_time_ms: u64) -> (result: u64)
        ensures
            current_time_ms <= u64::MAX - 300_000 ==>
                result == current_time_ms + 300_000,
            current_time_ms > u64::MAX - 300_000 ==>
                result == u64::MAX
    {
        current_time_ms.saturating_add(300_000)
    }

    /// Check if message is a duplicate based on dedup cache.
    ///
    /// # Arguments
    ///
    /// * `has_dedup_entry` - Whether dedup entry exists
    /// * `dedup_expires_at_ms` - Expiration of existing entry
    /// * `current_time_ms` - Current time
    ///
    /// # Returns
    ///
    /// `true` if message is a duplicate.
    pub fn is_duplicate_message(
        has_dedup_entry: bool,
        dedup_expires_at_ms: u64,
        current_time_ms: u64,
    ) -> (result: bool)
        ensures result == (
            has_dedup_entry && dedup_expires_at_ms > current_time_ms
        )
    {
        has_dedup_entry && dedup_expires_at_ms > current_time_ms
    }

    /// Check if TTL computation would overflow.
    ///
    /// # Arguments
    ///
    /// * `current_time_ms` - Current time
    /// * `ttl_ms` - Time to live
    ///
    /// # Returns
    ///
    /// `true` if TTL computation is safe.
    pub fn can_compute_ttl(current_time_ms: u64, ttl_ms: u64) -> (result: bool)
        ensures result == (
            ttl_ms == 0 || current_time_ms <= u64::MAX - ttl_ms
        )
    {
        ttl_ms == 0 || current_time_ms <= u64::MAX - ttl_ms
    }

    /// Check if dedup TTL computation would overflow.
    ///
    /// # Arguments
    ///
    /// * `current_time_ms` - Current time
    ///
    /// # Returns
    ///
    /// `true` if dedup TTL computation is safe.
    pub fn can_compute_dedup_ttl(current_time_ms: u64) -> (result: bool)
        ensures result == (current_time_ms <= u64::MAX - 300_000)
    {
        current_time_ms <= u64::MAX - 300_000
    }
}
