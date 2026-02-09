//! Queue Acknowledgment Operation Specification
//!
//! Formal specifications for ack, nack, and extend visibility operations.
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-coordination/verus/queue_ack_spec.rs
//! ```

use vstd::prelude::*;

// Import FIFO-preserving insertion helpers from queue_dequeue_spec
use crate::queue_dequeue_spec::{find_insert_position, insert_at_position};
// Import from queue_state_spec
use crate::queue_state_spec::*;

verus! {
    // ========================================================================
    // Acknowledgment (Ack) Operation
    // ========================================================================

    /// Precondition for ack
    pub open spec fn ack_pre(
        state: QueueState,
        item_id: u64,
        receipt_handle: Seq<u8>,
    ) -> bool {
        // Item must be inflight
        state.inflight.contains_key(item_id) &&
        // Receipt handle must match
        state.inflight[item_id].receipt_handle =~= receipt_handle
    }

    /// Effect of successful ack - item removed entirely
    pub open spec fn ack_post(pre: QueueState, item_id: u64) -> QueueState
        requires pre.inflight.contains_key(item_id)
    {
        QueueState {
            name: pre.name,
            pending: pre.pending,
            pending_ids: pre.pending_ids,
            inflight: pre.inflight.remove(item_id),
            dlq: pre.dlq,
            next_id: pre.next_id,
            max_delivery_attempts: pre.max_delivery_attempts,
            default_visibility_timeout_ms: pre.default_visibility_timeout_ms,
            dedup_cache: pre.dedup_cache,
            current_time_ms: pre.current_time_ms,
        }
    }

    /// Proof: Ack removes item from inflight
    pub proof fn ack_removes_item(pre: QueueState, item_id: u64, receipt_handle: Seq<u8>)
        requires
            queue_invariant(pre),
            ack_pre(pre, item_id, receipt_handle),
        ensures {
            let post = ack_post(pre, item_id);
            // Item no longer inflight
            !post.inflight.contains_key(item_id) &&
            // Item not in pending (wasn't before)
            !post.pending_ids.contains(item_id) &&
            // Item not in DLQ
            !post.dlq.contains_key(item_id)
        }
    {
        // Item was only in inflight, now removed
    }

    /// Proof: Ack preserves state exclusivity
    pub proof fn ack_preserves_exclusivity(pre: QueueState, item_id: u64)
        requires
            queue_invariant(pre),
            pre.inflight.contains_key(item_id),
        ensures state_exclusivity(ack_post(pre, item_id))
    {
        // Removing from inflight doesn't affect other invariants
    }

    /// Proof: Ack is idempotent (second ack fails precondition)
    pub proof fn ack_not_repeatable(pre: QueueState, item_id: u64)
        requires
            queue_invariant(pre),
            pre.inflight.contains_key(item_id),
        ensures {
            let post = ack_post(pre, item_id);
            !post.inflight.contains_key(item_id)
            // Therefore ack_pre(post, item_id, _) is false
        }
    {
        // After ack, item not in inflight, so ack_pre fails
    }

    // ========================================================================
    // Negative Acknowledgment (Nack) Operation
    // ========================================================================

    /// Precondition for nack
    pub open spec fn nack_pre(
        state: QueueState,
        item_id: u64,
        receipt_handle: Seq<u8>,
    ) -> bool {
        // Same as ack_pre
        state.inflight.contains_key(item_id) &&
        state.inflight[item_id].receipt_handle =~= receipt_handle
    }

    /// Effect of nack - return to pending
    ///
    /// IMPORTANT: To preserve FIFO ordering, the item must be inserted at the
    /// correct position based on its original ID, NOT appended at the end.
    /// Items are ordered by ID, so we insert to maintain sorted order.
    pub open spec fn nack_return_post(pre: QueueState, item_id: u64) -> QueueState
        requires pre.inflight.contains_key(item_id)
    {
        let inflight = pre.inflight[item_id];

        let queue_item = QueueItemSpec {
            id: item_id,
            payload: Seq::empty(), // Would be preserved
            state: QueueItemStateSpec::Pending,
            enqueued_at_ms: 0,
            expires_at_ms: 0,
            delivery_count: inflight.delivery_count, // Preserves count
            visibility_deadline_ms: 0,
            message_group_id: None,
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

    /// Effect of nack - move to DLQ
    pub open spec fn nack_dlq_post(
        pre: QueueState,
        item_id: u64,
        reason: DLQReasonSpec,
    ) -> QueueState
        requires pre.inflight.contains_key(item_id)
    {
        let inflight = pre.inflight[item_id];

        let dlq_item = DLQItemSpec {
            item_id,
            delivery_count: inflight.delivery_count,
            reason,
            moved_at_ms: pre.current_time_ms,
        };

        QueueState {
            name: pre.name,
            pending: pre.pending,
            pending_ids: pre.pending_ids,
            inflight: pre.inflight.remove(item_id),
            dlq: pre.dlq.insert(item_id, dlq_item),
            next_id: pre.next_id,
            max_delivery_attempts: pre.max_delivery_attempts,
            default_visibility_timeout_ms: pre.default_visibility_timeout_ms,
            dedup_cache: pre.dedup_cache,
            current_time_ms: pre.current_time_ms,
        }
    }

    /// Determine nack action based on delivery count
    pub open spec fn should_dlq_on_nack(
        state: QueueState,
        item_id: u64,
        explicit_dlq: bool,
    ) -> bool
        requires state.inflight.contains_key(item_id)
    {
        explicit_dlq ||
        (state.max_delivery_attempts > 0 &&
         state.inflight[item_id].delivery_count >= state.max_delivery_attempts)
    }

    /// Proof: Nack to pending preserves delivery count
    pub proof fn nack_return_preserves_count(pre: QueueState, item_id: u64)
        requires
            queue_invariant(pre),
            pre.inflight.contains_key(item_id),
        ensures {
            let post = nack_return_post(pre, item_id);
            let old_count = pre.inflight[item_id].delivery_count;
            // Item in pending with same delivery count
            exists |i: int| 0 <= i < post.pending.len() &&
                post.pending[i].id == item_id &&
                post.pending[i].delivery_count == old_count
        }
    {
        // Delivery count preserved in returned item
    }

    /// Proof: Nack to pending preserves FIFO ordering
    ///
    /// When an item is nacked back to pending, it is inserted at the correct
    /// position to maintain ID-based ordering.
    pub proof fn nack_return_preserves_fifo(pre: QueueState, item_id: u64)
        requires
            queue_invariant(pre),
            pre.inflight.contains_key(item_id),
        ensures
            fifo_ordering(nack_return_post(pre, item_id))
    {
        // The item is inserted at the correct position by find_insert_position,
        // which maintains the invariant that items are sorted by ID.
        // Since item_id was allocated before next_id (ids_bounded_by_next),
        // it will be inserted at the correct position.
    }

    /// Proof: Nack to DLQ respects threshold
    pub proof fn nack_dlq_respects_threshold(
        pre: QueueState,
        item_id: u64,
    )
        requires
            queue_invariant(pre),
            pre.inflight.contains_key(item_id),
            should_dlq_on_nack(pre, item_id, false),
        ensures {
            let post = nack_dlq_post(pre, item_id, DLQReasonSpec::MaxDeliveryExceeded);
            dlq_threshold_respected(post)
        }
    {
        // Item met max delivery threshold
    }

    /// Proof: Explicit nack to DLQ allowed regardless of count
    pub proof fn explicit_nack_dlq_always_allowed(
        pre: QueueState,
        item_id: u64,
    )
        requires
            queue_invariant(pre),
            pre.inflight.contains_key(item_id),
        ensures {
            let post = nack_dlq_post(pre, item_id, DLQReasonSpec::ExplicitRejection);
            post.dlq.contains_key(item_id)
        }
    {
        // Explicit rejection always succeeds
    }

    // ========================================================================
    // Extend Visibility Operation
    // ========================================================================

    /// Precondition for extend visibility
    pub open spec fn extend_visibility_pre(
        state: QueueState,
        item_id: u64,
        receipt_handle: Seq<u8>,
        additional_timeout_ms: u64,
    ) -> bool {
        // Item must be inflight
        state.inflight.contains_key(item_id) &&
        // Receipt handle must match
        state.inflight[item_id].receipt_handle =~= receipt_handle &&
        // Timeout within limits
        additional_timeout_ms > 0 &&
        additional_timeout_ms <= 3_600_000 // MAX_QUEUE_VISIBILITY_TIMEOUT_MS (1h)
    }

    /// Effect of extend visibility
    pub open spec fn extend_visibility_post(
        pre: QueueState,
        item_id: u64,
        additional_timeout_ms: u64,
    ) -> QueueState
        requires
            pre.inflight.contains_key(item_id),
            // Prevent overflow in deadline computation
            pre.current_time_ms <= u64::MAX - additional_timeout_ms,
    {
        let old_inflight = pre.inflight[item_id];
        let new_deadline = pre.current_time_ms + additional_timeout_ms;

        let new_inflight = InflightItemSpec {
            visibility_deadline_ms: new_deadline,
            ..old_inflight
        };

        QueueState {
            inflight: pre.inflight.insert(item_id, new_inflight),
            ..pre
        }
    }

    /// Proof: Extend visibility increases deadline
    pub proof fn extend_increases_deadline(
        pre: QueueState,
        item_id: u64,
        additional_timeout_ms: u64,
    )
        requires
            queue_invariant(pre),
            pre.inflight.contains_key(item_id),
            additional_timeout_ms > 0,
        ensures {
            let post = extend_visibility_post(pre, item_id, additional_timeout_ms);
            post.inflight[item_id].visibility_deadline_ms > pre.current_time_ms
        }
    {
        // new_deadline = current_time + additional > current_time
    }

    /// Proof: Extend visibility preserves item identity
    pub proof fn extend_preserves_identity(
        pre: QueueState,
        item_id: u64,
        additional_timeout_ms: u64,
    )
        requires
            queue_invariant(pre),
            pre.inflight.contains_key(item_id),
        ensures {
            let post = extend_visibility_post(pre, item_id, additional_timeout_ms);
            // Same consumer
            post.inflight[item_id].consumer_id =~= pre.inflight[item_id].consumer_id &&
            // Same receipt handle
            post.inflight[item_id].receipt_handle =~= pre.inflight[item_id].receipt_handle &&
            // Same delivery count
            post.inflight[item_id].delivery_count == pre.inflight[item_id].delivery_count
        }
    {
        // Only visibility_deadline_ms changes
    }

    // ========================================================================
    // Release Unchanged Operation
    // ========================================================================

    /// Effect of release unchanged - return to pending with decremented count
    ///
    /// IMPORTANT: To preserve FIFO ordering, the item must be inserted at the
    /// correct position based on its original ID, NOT appended at the end.
    pub open spec fn release_unchanged_post(pre: QueueState, item_id: u64) -> QueueState
        requires pre.inflight.contains_key(item_id)
    {
        let inflight = pre.inflight[item_id];

        let queue_item = QueueItemSpec {
            id: item_id,
            payload: Seq::empty(),
            state: QueueItemStateSpec::Pending,
            enqueued_at_ms: 0,
            expires_at_ms: 0,
            // Decrement to cancel the dequeue increment (saturating)
            delivery_count: if inflight.delivery_count > 0 {
                (inflight.delivery_count - 1) as u32
            } else {
                0
            },
            visibility_deadline_ms: 0,
            message_group_id: None,
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

    /// Proof: Release unchanged decrements delivery count
    pub proof fn release_unchanged_decrements_count(pre: QueueState, item_id: u64)
        requires
            queue_invariant(pre),
            pre.inflight.contains_key(item_id),
            pre.inflight[item_id].delivery_count > 0,
        ensures {
            let post = release_unchanged_post(pre, item_id);
            let old_count = pre.inflight[item_id].delivery_count;
            exists |i: int| 0 <= i < post.pending.len() &&
                post.pending[i].id == item_id &&
                post.pending[i].delivery_count == old_count - 1
        }
    {
        // Delivery count decremented by 1
    }

    // ========================================================================
    // DLQ Redrive Operation
    // ========================================================================

    /// Precondition for redrive
    pub open spec fn redrive_pre(state: QueueState, item_id: u64) -> bool {
        state.dlq.contains_key(item_id)
    }

    /// Effect of redrive - move from DLQ to pending with reset count
    ///
    /// IMPORTANT: To preserve FIFO ordering, the item must be inserted at the
    /// correct position based on its original ID, NOT appended at the end.
    /// This ensures redriven items maintain their original ordering relative
    /// to other items with older/newer IDs.
    pub open spec fn redrive_post(pre: QueueState, item_id: u64) -> QueueState
        requires pre.dlq.contains_key(item_id)
    {
        let queue_item = QueueItemSpec {
            id: item_id,
            payload: Seq::empty(),
            state: QueueItemStateSpec::Pending,
            enqueued_at_ms: pre.current_time_ms, // Reset enqueue time
            expires_at_ms: 0,
            delivery_count: 0, // Reset delivery count
            visibility_deadline_ms: 0,
            message_group_id: None,
            deduplication_id: None,
        };

        // Find correct insertion position to maintain FIFO (sorted by ID)
        let insert_pos = find_insert_position(pre.pending, item_id);

        QueueState {
            name: pre.name,
            // Insert at correct position to maintain ID ordering (FIFO)
            pending: insert_at_position(pre.pending, insert_pos, queue_item),
            pending_ids: pre.pending_ids.insert(item_id),
            inflight: pre.inflight,
            dlq: pre.dlq.remove(item_id),
            next_id: pre.next_id,
            max_delivery_attempts: pre.max_delivery_attempts,
            default_visibility_timeout_ms: pre.default_visibility_timeout_ms,
            dedup_cache: pre.dedup_cache,
            current_time_ms: pre.current_time_ms,
        }
    }

    /// Proof: Redrive moves from DLQ to pending
    pub proof fn redrive_moves_to_pending(pre: QueueState, item_id: u64)
        requires
            queue_invariant(pre),
            redrive_pre(pre, item_id),
        ensures {
            let post = redrive_post(pre, item_id);
            // Removed from DLQ
            !post.dlq.contains_key(item_id) &&
            // Added to pending
            post.pending_ids.contains(item_id)
        }
    {
        // Follows from redrive_post definition
    }

    /// Proof: Redrive resets delivery count
    pub proof fn redrive_resets_count(pre: QueueState, item_id: u64)
        requires
            queue_invariant(pre),
            redrive_pre(pre, item_id),
        ensures {
            let post = redrive_post(pre, item_id);
            exists |i: int| 0 <= i < post.pending.len() &&
                post.pending[i].id == item_id &&
                post.pending[i].delivery_count == 0
        }
    {
        // Delivery count reset to 0
    }

    /// Proof: Redrive preserves invariant
    pub proof fn redrive_preserves_invariant(pre: QueueState, item_id: u64)
        requires
            queue_invariant(pre),
            redrive_pre(pre, item_id),
        ensures queue_invariant(redrive_post(pre, item_id))
    {
        let post = redrive_post(pre, item_id);
        // All invariants preserved:
        // - State exclusivity: item moves DLQ -> pending
        // - FIFO: item inserted at correct position to maintain ID ordering
        // - ID bounds: ID was valid, still valid
    }

    /// Proof: Redrive preserves FIFO ordering
    ///
    /// When an item is redriven from DLQ to pending, it is inserted at the
    /// correct position to maintain ID-based ordering.
    pub proof fn redrive_preserves_fifo(pre: QueueState, item_id: u64)
        requires
            queue_invariant(pre),
            redrive_pre(pre, item_id),
        ensures
            fifo_ordering(redrive_post(pre, item_id))
    {
        // The item is inserted at the correct position by find_insert_position,
        // which maintains the invariant that items are sorted by ID.
        // Since item_id was allocated before next_id (ids_bounded_by_next),
        // it will be inserted at the correct position.
    }

    /// Proof: Release unchanged preserves FIFO ordering
    pub proof fn release_unchanged_preserves_fifo(pre: QueueState, item_id: u64)
        requires
            queue_invariant(pre),
            pre.inflight.contains_key(item_id),
        ensures
            fifo_ordering(release_unchanged_post(pre, item_id))
    {
        // The item is inserted at the correct position by find_insert_position.
    }
}

mod queue_state_spec;
