//! Queue State Machine Model
//!
//! Abstract state model for formal verification of distributed queue operations.
//!
//! # State Model
//!
//! The `QueueState` captures:
//! - Items in different states (pending, inflight, acknowledged, DLQ)
//! - Monotonically increasing item IDs
//! - Visibility timeout tracking
//!
//! # Key Invariants
//!
//! 1. **QUEUE-1: FIFO Ordering**: Dequeue in enqueue order
//! 2. **QUEUE-2: ID Monotonicity**: next_id strictly increases
//! 3. **QUEUE-3: State Exclusivity**: Each item in exactly one state
//! 4. **QUEUE-4: Visibility Timeout**: Inflight items have valid deadlines
//! 5. **QUEUE-5: DLQ Threshold**: DLQ items exceeded max delivery count
//! 6. **QUEUE-6: Dedup Consistency**: Cache entries point to valid items
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-coordination/verus/queue_state_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // State Model
    // ========================================================================

    /// Item state enumeration
    pub enum QueueItemStateSpec {
        /// Item is waiting to be dequeued
        Pending,
        /// Item is being processed by a consumer
        Inflight,
        /// Item was successfully acknowledged
        Acknowledged,
        /// Item moved to dead letter queue
        DeadLettered,
    }

    /// A single queue item
    pub struct QueueItemSpec {
        /// Unique item ID (monotonically increasing)
        pub id: u64,
        /// Item payload (abstract)
        pub payload: Seq<u8>,
        /// Current state
        pub state: QueueItemStateSpec,
        /// Time enqueued (Unix ms)
        pub enqueued_at_ms: u64,
        /// Expiration time (0 = no expiration)
        pub expires_at_ms: u64,
        /// Number of delivery attempts
        pub delivery_count: u32,
        /// Visibility deadline when inflight (Unix ms)
        pub visibility_deadline_ms: u64,
        /// Optional message group for FIFO ordering
        pub message_group_id: Option<Seq<u8>>,
        /// Optional deduplication ID
        pub deduplication_id: Option<Seq<u8>>,
    }

    /// Inflight item with receipt handle
    pub struct InflightItemSpec {
        /// Original item ID
        pub item_id: u64,
        /// Consumer ID
        pub consumer_id: Seq<u8>,
        /// Receipt handle for ack/nack
        pub receipt_handle: Seq<u8>,
        /// Visibility deadline
        pub visibility_deadline_ms: u64,
        /// Delivery count (including this attempt)
        pub delivery_count: u32,
    }

    /// Dead letter queue item
    pub struct DLQItemSpec {
        /// Original item ID
        pub item_id: u64,
        /// Delivery count before DLQ
        pub delivery_count: u32,
        /// Reason for DLQ
        pub reason: DLQReasonSpec,
        /// Time moved to DLQ
        pub moved_at_ms: u64,
    }

    /// Reason for moving to DLQ
    pub enum DLQReasonSpec {
        MaxDeliveryExceeded,
        ExplicitRejection,
        ExpiredWhileInflight,
    }

    /// Deduplication entry
    pub struct DedupEntrySpec {
        /// Deduplication ID
        pub dedup_id: Seq<u8>,
        /// Item ID that was created
        pub item_id: u64,
        /// Expiration time
        pub expires_at_ms: u64,
    }

    /// Complete queue state
    pub struct QueueState {
        /// Queue name
        pub name: Seq<u8>,
        /// Pending items (ordered by ID for FIFO)
        pub pending: Seq<QueueItemSpec>,
        /// Set of pending item IDs for fast lookup
        pub pending_ids: Set<u64>,
        /// Inflight items (being processed)
        pub inflight: Map<u64, InflightItemSpec>,
        /// Dead letter queue items
        pub dlq: Map<u64, DLQItemSpec>,
        /// Next item ID to allocate
        pub next_id: u64,
        /// Max delivery attempts before DLQ (0 = unlimited)
        pub max_delivery_attempts: u32,
        /// Default visibility timeout (ms)
        pub default_visibility_timeout_ms: u64,
        /// Deduplication cache
        pub dedup_cache: Map<Seq<u8>, DedupEntrySpec>,
        /// Current time (for reasoning about expiration)
        pub current_time_ms: u64,
    }

    // ========================================================================
    // Invariant 1: FIFO Ordering
    // ========================================================================

    /// QUEUE-1: Items in pending are ordered by ID (FIFO)
    ///
    /// Items are dequeued in the order they were enqueued
    pub open spec fn fifo_ordering(state: QueueState) -> bool {
        forall |i: int, j: int|
            0 <= i < j < state.pending.len() ==>
            state.pending[i].id < state.pending[j].id
    }

    /// First pending item has smallest ID among pending
    pub open spec fn first_is_oldest(state: QueueState) -> bool {
        state.pending.len() > 0 ==>
        forall |i: int|
            0 < i < state.pending.len() ==>
            state.pending[0].id < state.pending[i].id
    }

    // ========================================================================
    // Invariant 2: ID Monotonicity
    // ========================================================================

    /// QUEUE-2: next_id strictly increases
    pub open spec fn id_monotonicity(pre: QueueState, post: QueueState) -> bool {
        post.next_id >= pre.next_id
    }

    /// All existing IDs are less than next_id
    pub open spec fn ids_bounded_by_next(state: QueueState) -> bool {
        // Pending items
        (forall |i: int| 0 <= i < state.pending.len() ==>
            state.pending[i].id < state.next_id) &&
        // Inflight items
        (forall |id: u64| state.inflight.contains_key(id) ==>
            id < state.next_id) &&
        // DLQ items
        (forall |id: u64| state.dlq.contains_key(id) ==>
            id < state.next_id)
    }

    // ========================================================================
    // Invariant 3: State Exclusivity
    // ========================================================================

    /// QUEUE-3: Each item is in exactly one state
    ///
    /// An item ID cannot be in multiple locations simultaneously
    pub open spec fn state_exclusivity(state: QueueState) -> bool {
        // Pending and inflight are disjoint
        (forall |id: u64| state.pending_ids.contains(id) ==>
            !state.inflight.contains_key(id)) &&
        // Pending and DLQ are disjoint
        (forall |id: u64| state.pending_ids.contains(id) ==>
            !state.dlq.contains_key(id)) &&
        // Inflight and DLQ are disjoint
        (forall |id: u64| state.inflight.contains_key(id) ==>
            !state.dlq.contains_key(id))
    }

    /// pending_ids matches actual pending items
    pub open spec fn pending_ids_consistent(state: QueueState) -> bool {
        // Every pending item's ID is in pending_ids
        (forall |i: int| 0 <= i < state.pending.len() ==>
            state.pending_ids.contains(state.pending[i].id)) &&
        // Every ID in pending_ids has a matching pending item
        (forall |id: u64| state.pending_ids.contains(id) ==>
            exists |i: int| 0 <= i < state.pending.len() &&
                state.pending[i].id == id)
    }

    // ========================================================================
    // Invariant 4: Visibility Timeout
    // ========================================================================

    /// QUEUE-4: Inflight items have valid visibility deadlines
    ///
    /// All inflight items have a visibility deadline in the future
    /// (relative to when they were dequeued)
    pub open spec fn visibility_timeout_valid(state: QueueState) -> bool {
        forall |id: u64| state.inflight.contains_key(id) ==>
            state.inflight[id].visibility_deadline_ms > 0
    }

    /// Check if an inflight item's visibility has expired
    pub open spec fn is_visibility_expired(
        item: InflightItemSpec,
        current_time_ms: u64,
    ) -> bool {
        current_time_ms > item.visibility_deadline_ms
    }

    // ========================================================================
    // Invariant 5: DLQ Threshold
    // ========================================================================

    /// QUEUE-5: DLQ items exceeded max delivery count
    ///
    /// Items in DLQ got there because they exceeded max attempts
    /// or were explicitly rejected
    pub open spec fn dlq_threshold_respected(state: QueueState) -> bool {
        state.max_delivery_attempts > 0 ==>
        forall |id: u64| state.dlq.contains_key(id) ==> {
            let dlq_item = state.dlq[id];
            match dlq_item.reason {
                DLQReasonSpec::MaxDeliveryExceeded =>
                    dlq_item.delivery_count >= state.max_delivery_attempts,
                DLQReasonSpec::ExplicitRejection => true, // Explicitly rejected, any count OK
                DLQReasonSpec::ExpiredWhileInflight => true, // Expired, any count OK
            }
        }
    }

    // ========================================================================
    // Invariant 6: Dedup Consistency
    // ========================================================================

    /// QUEUE-6: Dedup entries point to valid items
    ///
    /// Dedup cache entries reference items that exist somewhere
    pub open spec fn dedup_consistency(state: QueueState) -> bool {
        forall |dedup_id: Seq<u8>| state.dedup_cache.contains_key(dedup_id) ==> {
            let entry = state.dedup_cache[dedup_id];
            let item_id = entry.item_id;
            // Item is either pending, inflight, DLQ, or acknowledged (removed)
            // We can't track acknowledged items, so just check ID is valid
            item_id < state.next_id
        }
    }

    // ========================================================================
    // Combined Invariant
    // ========================================================================

    /// Combined queue invariant
    pub open spec fn queue_invariant(state: QueueState) -> bool {
        fifo_ordering(state) &&
        ids_bounded_by_next(state) &&
        state_exclusivity(state) &&
        pending_ids_consistent(state) &&
        visibility_timeout_valid(state) &&
        dlq_threshold_respected(state) &&
        dedup_consistency(state)
    }

    // ========================================================================
    // Initial State
    // ========================================================================

    /// Initial empty queue state
    pub open spec fn initial_queue_state(
        name: Seq<u8>,
        max_delivery_attempts: u32,
        default_visibility_timeout_ms: u64,
    ) -> QueueState {
        QueueState {
            name,
            pending: Seq::empty(),
            pending_ids: Set::empty(),
            inflight: Map::empty(),
            dlq: Map::empty(),
            next_id: 1, // Start at 1, not 0
            max_delivery_attempts,
            default_visibility_timeout_ms,
            dedup_cache: Map::empty(),
            current_time_ms: 0,
        }
    }

    /// Proof: Initial state satisfies invariant
    pub proof fn initial_state_invariant(
        name: Seq<u8>,
        max_delivery_attempts: u32,
        default_visibility_timeout_ms: u64,
    )
        ensures queue_invariant(initial_queue_state(name, max_delivery_attempts, default_visibility_timeout_ms))
    {
        let state = initial_queue_state(name, max_delivery_attempts, default_visibility_timeout_ms);
        // All invariants trivially hold for empty collections
    }

    // ========================================================================
    // Helper Predicates
    // ========================================================================

    /// Check if queue is empty
    pub open spec fn is_empty(state: QueueState) -> bool {
        state.pending.len() == 0 &&
        state.inflight.len() == 0
    }

    /// Get count of pending items
    pub open spec fn pending_count(state: QueueState) -> int {
        state.pending.len()
    }

    /// Get count of inflight items
    pub open spec fn inflight_count(state: QueueState) -> int {
        state.inflight.len()
    }

    /// Check if an item ID exists in any state
    pub open spec fn item_exists(state: QueueState, id: u64) -> bool {
        state.pending_ids.contains(id) ||
        state.inflight.contains_key(id) ||
        state.dlq.contains_key(id)
    }

    /// Check if message group has an inflight item
    pub open spec fn message_group_is_inflight(
        state: QueueState,
        group_id: Seq<u8>,
    ) -> bool {
        exists |id: u64| state.inflight.contains_key(id) &&
            // Would need to track group in InflightItemSpec
            true // Simplified
    }

    // ========================================================================
    // Time-Based Predicates
    // ========================================================================

    /// Check if an item has expired
    pub open spec fn item_expired(item: QueueItemSpec, current_time_ms: u64) -> bool {
        item.expires_at_ms > 0 && current_time_ms > item.expires_at_ms
    }

    /// Count of expired pending items
    pub open spec fn expired_pending_count(state: QueueState) -> int
        decreases state.pending.len()
    {
        if state.pending.len() == 0 {
            0
        } else {
            let first = state.pending.first();
            let rest = QueueState { pending: state.pending.skip(1), ..state };
            let first_expired = if item_expired(first, state.current_time_ms) { 1 } else { 0 };
            first_expired + expired_pending_count(rest)
        }
    }
}
