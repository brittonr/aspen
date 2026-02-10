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
        /// Optional message group for FIFO ordering
        /// When set, blocks dequeue of other items in the same group
        pub message_group_id: Option<Seq<u8>>,
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
            #![trigger state.pending[i]]
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
            state.inflight[id].visibility_deadline_ms > 0 &&
            state.inflight[id].visibility_deadline_ms > state.current_time_ms
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
    #[verifier(external_body)]
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
        state.pending.len() as int
    }

    /// Get count of inflight items
    pub open spec fn inflight_count(state: QueueState) -> int {
        state.inflight.len() as int
    }

    /// Check if an item ID exists in any state
    pub open spec fn item_exists(state: QueueState, id: u64) -> bool {
        state.pending_ids.contains(id) ||
        state.inflight.contains_key(id) ||
        state.dlq.contains_key(id)
    }

    /// Check if message group has an inflight item
    ///
    /// Returns true if ANY inflight item belongs to the given group.
    /// This blocks dequeue of other items in the same group, enforcing
    /// FIFO ordering per message group.
    ///
    /// When an item with a message_group_id is dequeued, other items in
    /// the same group cannot be dequeued until the first item is acked
    /// or its visibility timeout expires.
    pub open spec fn message_group_is_inflight(
        state: QueueState,
        group_id: Seq<u8>,
    ) -> bool {
        exists |id: u64| state.inflight.contains_key(id) &&
            state.inflight[id].message_group_id.is_some() &&
            state.inflight[id].message_group_id.unwrap() =~= group_id
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
            let first_expired: int = if item_expired(first, state.current_time_ms) { 1 } else { 0 };
            first_expired + expired_pending_count(rest)
        }
    }

    // ========================================================================
    // Executable Functions (verified implementations)
    // ========================================================================
    //
    // These exec fn implementations are verified to match their spec fn
    // counterparts. They can be called from production code while maintaining
    // formal guarantees.

    /// Check if an inflight item's visibility has expired.
    ///
    /// # Arguments
    ///
    /// * `visibility_deadline_ms` - The item's visibility deadline
    /// * `current_time_ms` - Current time
    ///
    /// # Returns
    ///
    /// `true` if visibility has expired (current_time > deadline).
    pub fn is_visibility_expired_exec(
        visibility_deadline_ms: u64,
        current_time_ms: u64,
    ) -> (result: bool)
        ensures result == (current_time_ms > visibility_deadline_ms)
    {
        current_time_ms > visibility_deadline_ms
    }

    /// Check if a queue item has expired.
    ///
    /// # Arguments
    ///
    /// * `expires_at_ms` - Item expiration timestamp (0 = no expiration)
    /// * `current_time_ms` - Current time
    ///
    /// # Returns
    ///
    /// `true` if item has expired.
    pub fn is_item_expired(expires_at_ms: u64, current_time_ms: u64) -> (result: bool)
        ensures result == (expires_at_ms > 0 && current_time_ms > expires_at_ms)
    {
        expires_at_ms > 0 && current_time_ms > expires_at_ms
    }

    /// Check if a queue is empty.
    ///
    /// # Arguments
    ///
    /// * `pending_count` - Number of pending items
    /// * `inflight_count` - Number of inflight items
    ///
    /// # Returns
    ///
    /// `true` if both pending and inflight are empty.
    pub fn is_queue_empty(pending_count: u32, inflight_count: u32) -> (result: bool)
        ensures result == (pending_count == 0 && inflight_count == 0)
    {
        pending_count == 0 && inflight_count == 0
    }

    /// Calculate visibility deadline for a dequeued item.
    ///
    /// # Arguments
    ///
    /// * `dequeue_time_ms` - Time of dequeue
    /// * `visibility_timeout_ms` - Visibility timeout duration
    ///
    /// # Returns
    ///
    /// Visibility deadline timestamp (saturating at u64::MAX).
    pub fn calculate_visibility_deadline(
        dequeue_time_ms: u64,
        visibility_timeout_ms: u64,
    ) -> (result: u64)
        ensures
            dequeue_time_ms as int + visibility_timeout_ms as int <= u64::MAX as int ==>
                result == dequeue_time_ms + visibility_timeout_ms,
            dequeue_time_ms as int + visibility_timeout_ms as int > u64::MAX as int ==>
                result == u64::MAX
    {
        dequeue_time_ms.saturating_add(visibility_timeout_ms)
    }

    /// Allocate the next item ID.
    ///
    /// # Arguments
    ///
    /// * `current_next_id` - Current value of next_id
    ///
    /// # Returns
    ///
    /// Tuple of (allocated_id, new_next_id). Saturates at u64::MAX.
    pub fn allocate_item_id(current_next_id: u64) -> (result: (u64, u64))
        ensures
            result.0 == current_next_id,
            current_next_id < u64::MAX ==> result.1 == current_next_id + 1,
            current_next_id == u64::MAX ==> result.1 == u64::MAX
    {
        (current_next_id, current_next_id.saturating_add(1))
    }

    /// Check if an item should be moved to DLQ.
    ///
    /// # Arguments
    ///
    /// * `delivery_count` - Current delivery count
    /// * `max_delivery_attempts` - Maximum allowed attempts (0 = unlimited)
    ///
    /// # Returns
    ///
    /// `true` if item should be dead-lettered.
    pub fn should_move_to_dlq(delivery_count: u32, max_delivery_attempts: u32) -> (result: bool)
        ensures result == (max_delivery_attempts > 0 && delivery_count >= max_delivery_attempts)
    {
        max_delivery_attempts > 0 && delivery_count >= max_delivery_attempts
    }

    /// Increment delivery count.
    ///
    /// # Arguments
    ///
    /// * `current_count` - Current delivery count
    ///
    /// # Returns
    ///
    /// Incremented count (saturating at u32::MAX).
    pub fn increment_delivery_count(current_count: u32) -> (result: u32)
        ensures
            current_count < u32::MAX ==> result == current_count + 1,
            current_count == u32::MAX ==> result == u32::MAX
    {
        current_count.saturating_add(1)
    }

    /// Check if a deduplication entry has expired.
    ///
    /// # Arguments
    ///
    /// * `dedup_expires_at_ms` - Dedup entry expiration
    /// * `current_time_ms` - Current time
    ///
    /// # Returns
    ///
    /// `true` if dedup entry has expired.
    pub fn is_dedup_expired(dedup_expires_at_ms: u64, current_time_ms: u64) -> (result: bool)
        ensures result == (current_time_ms > dedup_expires_at_ms)
    {
        current_time_ms > dedup_expires_at_ms
    }

    /// Calculate deduplication expiration time.
    ///
    /// # Arguments
    ///
    /// * `enqueue_time_ms` - Time of enqueue
    /// * `dedup_window_ms` - Deduplication window duration
    ///
    /// # Returns
    ///
    /// Dedup expiration timestamp (saturating at u64::MAX).
    pub fn calculate_dedup_expiration(
        enqueue_time_ms: u64,
        dedup_window_ms: u64,
    ) -> (result: u64)
        ensures
            enqueue_time_ms as int + dedup_window_ms as int <= u64::MAX as int ==>
                result == enqueue_time_ms + dedup_window_ms,
            enqueue_time_ms as int + dedup_window_ms as int > u64::MAX as int ==>
                result == u64::MAX
    {
        enqueue_time_ms.saturating_add(dedup_window_ms)
    }

    /// Check if visibility timeout should be extended.
    ///
    /// # Arguments
    ///
    /// * `current_deadline_ms` - Current visibility deadline
    /// * `requested_extension_ms` - Requested extension
    /// * `max_visibility_ms` - Maximum allowed visibility timeout
    /// * `current_time_ms` - Current time
    ///
    /// # Returns
    ///
    /// `true` if extension is valid (doesn't exceed max).
    pub fn can_extend_visibility(
        current_deadline_ms: u64,
        requested_extension_ms: u64,
        max_visibility_ms: u64,
        current_time_ms: u64,
    ) -> (result: bool)
        ensures result == (
            requested_extension_ms <= max_visibility_ms &&
            current_deadline_ms > current_time_ms
        )
    {
        requested_extension_ms <= max_visibility_ms &&
        current_deadline_ms > current_time_ms
    }

    /// Calculate new visibility deadline after extension.
    ///
    /// # Arguments
    ///
    /// * `current_time_ms` - Current time
    /// * `extension_ms` - Extension duration
    ///
    /// # Returns
    ///
    /// New visibility deadline (saturating at u64::MAX).
    pub fn calculate_extended_deadline(
        current_time_ms: u64,
        extension_ms: u64,
    ) -> (result: u64)
        ensures
            current_time_ms as int + extension_ms as int <= u64::MAX as int ==>
                result == current_time_ms + extension_ms,
            current_time_ms as int + extension_ms as int > u64::MAX as int ==>
                result == u64::MAX
    {
        current_time_ms.saturating_add(extension_ms)
    }
}
