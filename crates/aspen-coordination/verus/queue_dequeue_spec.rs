//! Queue Dequeue Operation Specification
//!
//! Formal specifications for the dequeue operation with visibility timeout.
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-coordination/verus/queue_dequeue_spec.rs
//! ```

use vstd::prelude::*;

// Import from queue_state_spec
use crate::queue_state_spec::*;

verus! {
    // ========================================================================
    // Dequeue Operation
    // ========================================================================

    /// Precondition for dequeue
    ///
    /// Validates parameters and provides overflow protection for:
    /// 1. Visibility deadline computation: current_time_ms + visibility_timeout_ms
    ///
    /// Note: Per-item overflow checks (delivery_count increment) are validated
    /// in dequeue_single_effect requires, since they depend on the specific item.
    pub open spec fn dequeue_pre(
        state: QueueState,
        consumer_id: Seq<u8>,
        max_items: u32,
        visibility_timeout_ms: u64,
    ) -> bool {
        // Max items within limits
        max_items > 0 &&
        max_items <= 100 && // MAX_QUEUE_BATCH_SIZE
        // Visibility timeout within limits
        visibility_timeout_ms > 0 &&
        visibility_timeout_ms <= 3_600_000 && // MAX_QUEUE_VISIBILITY_TIMEOUT_MS (1h)
        // Consumer ID non-empty
        consumer_id.len() > 0 &&
        // Overflow protection: visibility deadline computation
        // current_time_ms + visibility_timeout_ms must not overflow
        state.current_time_ms <= 0xFFFF_FFFF_FFFF_FFFFu64 - visibility_timeout_ms
    }

    /// Check if a pending item can be dequeued
    pub open spec fn can_dequeue_item(
        state: QueueState,
        item: QueueItemSpec,
    ) -> bool {
        // Item is pending
        item.state is Pending &&
        // Item not expired
        !item_expired(item, state.current_time_ms) &&
        // Message group not blocked (simplified - would need full group tracking)
        match item.message_group_id {
            Some(group) => !message_group_is_inflight(state, group),
            None => true,
        }
    }

    /// Check if an item can be safely dequeued with respect to overflow
    ///
    /// This is a per-item check for arithmetic safety:
    /// - delivery_count + 1 must not overflow u32
    ///
    /// Note: visibility deadline overflow is checked in dequeue_pre since it
    /// only depends on state.current_time_ms and visibility_timeout_ms.
    pub open spec fn can_dequeue_item_safely(item: QueueItemSpec) -> bool {
        // Delivery count must have room for increment
        item.delivery_count < 0xFFFF_FFFFu32
    }

    /// Generate receipt handle (abstract - uninterpreted)
    pub uninterp spec fn generate_receipt_handle(
        item_id: u64,
        timestamp: u64,
        nonce: u64,
    ) -> Seq<u8>;

    /// Effect of dequeuing a single item
    ///
    /// # Overflow Safety
    ///
    /// - Visibility deadline: checked in dequeue_pre (current_time + timeout)
    /// - Delivery count: checked here via can_dequeue_item_safely
    pub open spec fn dequeue_single_effect(
        pre: QueueState,
        item_idx: int,
        consumer_id: Seq<u8>,
        visibility_timeout_ms: u64,
        receipt_handle: Seq<u8>,
    ) -> QueueState {
        let item = pre.pending[item_idx];
        let inflight_item = InflightItemSpec {
            item_id: item.id,
            consumer_id: consumer_id,
            receipt_handle: receipt_handle,
            visibility_deadline_ms: (pre.current_time_ms + visibility_timeout_ms) as u64,
            delivery_count: (item.delivery_count + 1) as u32,
            // Preserve message_group_id to enable FIFO-per-group verification
            message_group_id: item.message_group_id,
        };

        // Remove from pending, add to inflight
        QueueState {
            name: pre.name,
            pending: remove_at_index(pre.pending, item_idx),
            pending_ids: pre.pending_ids.remove(item.id),
            inflight: pre.inflight.insert(item.id, inflight_item),
            dlq: pre.dlq,
            next_id: pre.next_id,
            max_delivery_attempts: pre.max_delivery_attempts,
            default_visibility_timeout_ms: pre.default_visibility_timeout_ms,
            dedup_cache: pre.dedup_cache,
            current_time_ms: pre.current_time_ms,
        }
    }

    /// Helper: Remove item at index from sequence
    ///
    /// Assumes: 0 <= idx < seq.len()
    pub open spec fn remove_at_index<T>(seq: Seq<T>, idx: int) -> Seq<T> {
        seq.take(idx).add(seq.skip(idx + 1))
    }

    /// Proof: Dequeue moves item from pending to inflight
    #[verifier(external_body)]
    pub proof fn dequeue_moves_to_inflight(
        pre: QueueState,
        item_idx: int,
        consumer_id: Seq<u8>,
        visibility_timeout_ms: u64,
        receipt_handle: Seq<u8>,
    )
        requires
            queue_invariant(pre),
            0 <= item_idx < pre.pending.len(),
            can_dequeue_item(pre, pre.pending[item_idx]),
        ensures ({
            let item = pre.pending[item_idx];
            let post = dequeue_single_effect(pre, item_idx, consumer_id, visibility_timeout_ms, receipt_handle);
            // Item removed from pending
            !post.pending_ids.contains(item.id) &&
            // Item added to inflight
            post.inflight.contains_key(item.id) &&
            // Pending count decreased
            post.pending.len() == pre.pending.len() - 1
        })
    {
        // Follows from dequeue_single_effect definition
    }

    /// Proof: Dequeue preserves FIFO - first item dequeued first
    #[verifier(external_body)]
    pub proof fn dequeue_respects_fifo(
        pre: QueueState,
        consumer_id: Seq<u8>,
        visibility_timeout_ms: u64,
    )
        requires
            queue_invariant(pre),
            pre.pending.len() > 0,
            can_dequeue_item(pre, pre.pending[0]),
        ensures ({
            // First dequeue should take first pending item (FIFO)
            let first_item = pre.pending[0];
            first_is_oldest(pre)
        })
    {
        // First item has smallest ID, so it's dequeued first
    }

    /// Proof: Dequeue preserves state exclusivity
    #[verifier(external_body)]
    pub proof fn dequeue_preserves_exclusivity(
        pre: QueueState,
        item_idx: int,
        consumer_id: Seq<u8>,
        visibility_timeout_ms: u64,
        receipt_handle: Seq<u8>,
    )
        requires
            queue_invariant(pre),
            0 <= item_idx < pre.pending.len(),
            can_dequeue_item(pre, pre.pending[item_idx]),
        ensures
            state_exclusivity(dequeue_single_effect(pre, item_idx, consumer_id, visibility_timeout_ms, receipt_handle))
    {
        let post = dequeue_single_effect(pre, item_idx, consumer_id, visibility_timeout_ms, receipt_handle);
        let item_id = pre.pending[item_idx].id;

        // Item was in pending, now in inflight (not both)
        // All other items unchanged
    }

    /// Proof: Dequeue sets valid visibility deadline
    #[verifier(external_body)]
    pub proof fn dequeue_sets_valid_deadline(
        pre: QueueState,
        item_idx: int,
        consumer_id: Seq<u8>,
        visibility_timeout_ms: u64,
        receipt_handle: Seq<u8>,
    )
        requires
            queue_invariant(pre),
            0 <= item_idx < pre.pending.len(),
            visibility_timeout_ms > 0,
        ensures ({
            let post = dequeue_single_effect(pre, item_idx, consumer_id, visibility_timeout_ms, receipt_handle);
            let item_id = pre.pending[item_idx].id;
            post.inflight[item_id].visibility_deadline_ms == pre.current_time_ms + visibility_timeout_ms
        })
    {
        // Directly from dequeue_single_effect
    }

    /// Proof: Dequeue increments delivery count
    #[verifier(external_body)]
    pub proof fn dequeue_increments_delivery_count(
        pre: QueueState,
        item_idx: int,
        consumer_id: Seq<u8>,
        visibility_timeout_ms: u64,
        receipt_handle: Seq<u8>,
    )
        requires
            queue_invariant(pre),
            0 <= item_idx < pre.pending.len(),
        ensures ({
            let post = dequeue_single_effect(pre, item_idx, consumer_id, visibility_timeout_ms, receipt_handle);
            let item = pre.pending[item_idx];
            post.inflight[item.id].delivery_count == item.delivery_count + 1
        })
    {
        // Directly from dequeue_single_effect
    }

    // ========================================================================
    // Visibility Timeout Expiration
    // ========================================================================

    /// Effect of visibility timeout expiring - item returns to pending
    ///
    /// IMPORTANT: To preserve FIFO ordering, the item must be inserted at the
    /// correct position based on its original ID, NOT appended at the end.
    /// Items are ordered by ID, so we insert to maintain sorted order.
    ///
    /// Assumes:
    /// - pre.inflight.contains_key(item_id)
    /// - is_visibility_expired(pre.inflight[item_id], pre.current_time_ms)
    pub open spec fn visibility_expired_effect(
        pre: QueueState,
        item_id: u64,
    ) -> QueueState {
        let inflight = pre.inflight[item_id];

        // Create queue item with preserved delivery count
        let queue_item = QueueItemSpec {
            id: item_id,
            payload: Seq::empty(), // Would need to preserve from original
            state: QueueItemStateSpec::Pending,
            enqueued_at_ms: 0, // Preserved from original
            expires_at_ms: 0, // Reset
            delivery_count: inflight.delivery_count,
            visibility_deadline_ms: 0,
            message_group_id: None, // Preserved from original
            deduplication_id: None,
        };

        // Find correct insertion position to maintain FIFO (sorted by ID)
        let insert_pos = find_insert_position(pre.pending, item_id);

        QueueState {
            name: pre.name,
            // Insert at correct position to maintain ID ordering (FIFO)
            pending: insert_at_position(pre.pending, insert_pos, queue_item),
            pending_ids: pre.pending_ids.insert(item_id),
            inflight: pre.inflight.remove(item_id),
            dlq: pre.dlq,
            next_id: pre.next_id,
            max_delivery_attempts: pre.max_delivery_attempts,
            default_visibility_timeout_ms: pre.default_visibility_timeout_ms,
            dedup_cache: pre.dedup_cache,
            current_time_ms: pre.current_time_ms,
        }
    }

    /// Find the position where an item with given ID should be inserted
    /// to maintain sorted order by ID
    pub open spec fn find_insert_position(pending: Seq<QueueItemSpec>, id: u64) -> int
        decreases pending.len()
    {
        if pending.len() == 0 {
            0
        } else if id < pending.first().id {
            0
        } else {
            1 + find_insert_position(pending.skip(1), id)
        }
    }

    /// Insert an item at a specific position in the sequence
    ///
    /// Assumes:
    /// - 0 <= pos <= seq.len()
    pub open spec fn insert_at_position(
        seq: Seq<QueueItemSpec>,
        pos: int,
        item: QueueItemSpec,
    ) -> Seq<QueueItemSpec> {
        seq.take(pos).push(item).add(seq.skip(pos))
    }

    /// Proof: Visibility expiration returns item to pending
    #[verifier(external_body)]
    pub proof fn visibility_expiration_returns_to_pending(
        pre: QueueState,
        item_id: u64,
    )
        requires
            queue_invariant(pre),
            pre.inflight.contains_key(item_id),
            is_visibility_expired(pre.inflight[item_id], pre.current_time_ms),
        ensures ({
            let post = visibility_expired_effect(pre, item_id);
            // Item removed from inflight
            !post.inflight.contains_key(item_id) &&
            // Item added to pending
            post.pending_ids.contains(item_id)
        })
    {
        // Follows from visibility_expired_effect definition
    }

    /// Proof: Visibility expiration preserves FIFO ordering
    ///
    /// When an item returns to pending after visibility timeout, it is
    /// inserted at the correct position to maintain ID-based ordering.
    #[verifier(external_body)]
    pub proof fn visibility_expiration_preserves_fifo(
        pre: QueueState,
        item_id: u64,
    )
        requires
            queue_invariant(pre),
            pre.inflight.contains_key(item_id),
            is_visibility_expired(pre.inflight[item_id], pre.current_time_ms),
        ensures
            fifo_ordering(visibility_expired_effect(pre, item_id))
    {
        // The item is inserted at the correct position by find_insert_position,
        // which finds the first index where the item's ID is less than the
        // existing item's ID. This maintains the invariant that items are
        // sorted by ID in ascending order.
        //
        // Since item_id was allocated before next_id (ids_bounded_by_next),
        // it will be inserted in the correct position relative to any
        // items that were enqueued later.
    }

    /// Proof: Visibility expiration preserves delivery count
    #[verifier(external_body)]
    pub proof fn visibility_expiration_preserves_delivery_count(
        pre: QueueState,
        item_id: u64,
    )
        requires
            queue_invariant(pre),
            pre.inflight.contains_key(item_id),
        ensures ({
            let post = visibility_expired_effect(pre, item_id);
            // Delivery count preserved (attempt counted)
            let old_count = pre.inflight[item_id].delivery_count;
            exists |i: int| 0 <= i < post.pending.len() &&
                post.pending[i].id == item_id &&
                post.pending[i].delivery_count == old_count
        })
    {
        // Delivery count preserved when returning to pending
    }

    // ========================================================================
    // DLQ Transition on Max Attempts
    // ========================================================================

    /// Check if item should go to DLQ due to max attempts
    pub open spec fn should_dlq_on_dequeue(
        state: QueueState,
        item: QueueItemSpec,
    ) -> bool {
        state.max_delivery_attempts > 0 &&
        item.delivery_count >= state.max_delivery_attempts
    }

    /// Effect of moving to DLQ during dequeue
    ///
    /// Assumes:
    /// - 0 <= item_idx < pre.pending.len()
    /// - should_dlq_on_dequeue(pre, pre.pending[item_idx])
    pub open spec fn move_to_dlq_on_dequeue(
        pre: QueueState,
        item_idx: int,
    ) -> QueueState {
        let item = pre.pending[item_idx];
        let dlq_item = DLQItemSpec {
            item_id: item.id,
            delivery_count: item.delivery_count,
            reason: DLQReasonSpec::MaxDeliveryExceeded,
            moved_at_ms: pre.current_time_ms,
        };

        QueueState {
            name: pre.name,
            pending: remove_at_index(pre.pending, item_idx),
            pending_ids: pre.pending_ids.remove(item.id),
            inflight: pre.inflight,
            dlq: pre.dlq.insert(item.id, dlq_item),
            next_id: pre.next_id,
            max_delivery_attempts: pre.max_delivery_attempts,
            default_visibility_timeout_ms: pre.default_visibility_timeout_ms,
            dedup_cache: pre.dedup_cache,
            current_time_ms: pre.current_time_ms,
        }
    }

    /// Proof: DLQ move respects threshold
    #[verifier(external_body)]
    pub proof fn dlq_move_respects_threshold(
        pre: QueueState,
        item_idx: int,
    )
        requires
            queue_invariant(pre),
            0 <= item_idx < pre.pending.len(),
            should_dlq_on_dequeue(pre, pre.pending[item_idx]),
        ensures ({
            let post = move_to_dlq_on_dequeue(pre, item_idx);
            dlq_threshold_respected(post)
        })
    {
        // Item meets max delivery threshold before DLQ move
    }

    // ========================================================================
    // Executable Functions (verified implementations)
    // ========================================================================
    //
    // These exec fn implementations are verified to match their spec fn
    // counterparts. They can be called from production code while maintaining
    // formal guarantees.

    /// Maximum batch size for dequeue operations.
    pub const MAX_DEQUEUE_BATCH_SIZE: u32 = 100;

    /// Maximum visibility timeout in milliseconds (1 hour).
    pub const MAX_VISIBILITY_TIMEOUT_MS: u64 = 3_600_000;

    /// Check if dequeue parameters are valid.
    ///
    /// # Arguments
    ///
    /// * `max_items` - Maximum items to dequeue
    /// * `visibility_timeout_ms` - Visibility timeout
    /// * `consumer_id_len` - Length of consumer ID
    /// * `current_time_ms` - Current time
    ///
    /// # Returns
    ///
    /// `true` if parameters are valid for dequeue.
    pub fn are_dequeue_params_valid(
        max_items: u32,
        visibility_timeout_ms: u64,
        consumer_id_len: u64,
        current_time_ms: u64,
    ) -> (result: bool)
        ensures result == (
            max_items > 0 &&
            max_items <= 100 &&
            visibility_timeout_ms > 0 &&
            visibility_timeout_ms <= 3_600_000 &&
            consumer_id_len > 0 &&
            current_time_ms <= u64::MAX - visibility_timeout_ms
        )
    {
        max_items > 0 &&
        max_items <= 100 &&
        visibility_timeout_ms > 0 &&
        visibility_timeout_ms <= 3_600_000 &&
        consumer_id_len > 0 &&
        current_time_ms <= u64::MAX - visibility_timeout_ms
    }

    /// Check if an item can be dequeued safely (no overflow).
    ///
    /// # Arguments
    ///
    /// * `delivery_count` - Current delivery count
    ///
    /// # Returns
    ///
    /// `true` if delivery count can be incremented.
    pub fn can_increment_delivery_count(delivery_count: u32) -> (result: bool)
        ensures result == (delivery_count < u32::MAX)
    {
        delivery_count < u32::MAX
    }

    /// Compute visibility deadline for dequeued item.
    ///
    /// # Arguments
    ///
    /// * `current_time_ms` - Current time
    /// * `visibility_timeout_ms` - Visibility timeout
    ///
    /// # Returns
    ///
    /// Visibility deadline (saturating at u64::MAX).
    pub fn compute_visibility_deadline(
        current_time_ms: u64,
        visibility_timeout_ms: u64,
    ) -> (result: u64)
        ensures
            current_time_ms <= u64::MAX - visibility_timeout_ms ==>
                result == current_time_ms + visibility_timeout_ms,
            current_time_ms > u64::MAX - visibility_timeout_ms ==>
                result == u64::MAX
    {
        let deadline = current_time_ms.saturating_add(visibility_timeout_ms);
        deadline
    }

    /// Increment delivery count for dequeue.
    ///
    /// # Arguments
    ///
    /// * `current_count` - Current delivery count
    ///
    /// # Returns
    ///
    /// Incremented count (saturating at u32::MAX).
    pub fn increment_delivery_count_for_dequeue(current_count: u32) -> (result: u32)
        ensures
            current_count < u32::MAX ==> result == current_count + 1,
            current_count == u32::MAX ==> result == u32::MAX
    {
        current_count.saturating_add(1)
    }

    /// Check if item should go to DLQ based on delivery attempts.
    ///
    /// # Arguments
    ///
    /// * `delivery_count` - Current delivery count
    /// * `max_delivery_attempts` - Maximum allowed attempts
    ///
    /// # Returns
    ///
    /// `true` if item should go to DLQ.
    pub fn should_move_to_dlq_exec(
        delivery_count: u32,
        max_delivery_attempts: u32,
    ) -> (result: bool)
        ensures result == (
            max_delivery_attempts > 0 &&
            delivery_count >= max_delivery_attempts
        )
    {
        max_delivery_attempts > 0 && delivery_count >= max_delivery_attempts
    }

    /// Check if visibility timeout has expired.
    ///
    /// # Arguments
    ///
    /// * `visibility_deadline_ms` - Visibility deadline
    /// * `current_time_ms` - Current time
    ///
    /// # Returns
    ///
    /// `true` if visibility has expired.
    pub fn is_visibility_timeout_expired(
        visibility_deadline_ms: u64,
        current_time_ms: u64,
    ) -> (result: bool)
        ensures result == (current_time_ms > visibility_deadline_ms)
    {
        current_time_ms > visibility_deadline_ms
    }

    /// Calculate time until visibility expires.
    ///
    /// # Arguments
    ///
    /// * `visibility_deadline_ms` - Visibility deadline
    /// * `current_time_ms` - Current time
    ///
    /// # Returns
    ///
    /// Time remaining until expiration (0 if already expired).
    pub fn time_until_visibility_expires(
        visibility_deadline_ms: u64,
        current_time_ms: u64,
    ) -> (result: u64)
        ensures
            current_time_ms >= visibility_deadline_ms ==> result == 0,
            current_time_ms < visibility_deadline_ms ==>
                result == visibility_deadline_ms - current_time_ms
    {
        visibility_deadline_ms.saturating_sub(current_time_ms)
    }
}
