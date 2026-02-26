//! Verus Formal Specifications for Aspen Coordination
//!
//! This crate contains standalone Verus specifications for verifying the
//! correctness of distributed coordination primitives in Aspen.
//!
//! # Verification
//!
//! Run Verus verification with:
//! ```bash
//! nix run .#verify-verus-coordination          # Verify all specs
//! nix run .#verify-verus-coordination -- quick # Syntax check only
//! nix run .#verify-verus                       # Verify all (Raft + Coordination)
//! ```
//!
//! # Module Overview
//!
//! ## Distributed Lock
//! - `lock_state_spec`: Lock state model and invariants
//! - `acquire_spec`: Lock acquisition operation correctness
//! - `release_spec`: Lock release operation correctness
//! - `renew_spec`: Lock renewal operation correctness
//!
//! ## Sequence Generator
//! - `sequence_state_spec`: Sequence state model and invariants
//! - `sequence_reserve_spec`: Reserve operation correctness
//!
//! ## Atomic Counter
//! - `counter_state_spec`: Counter state model and invariants
//! - `counter_ops_spec`: Add, subtract, CAS operation correctness
//!
//! ## Leader Election
//! - `election_state_spec`: Election state model and invariants
//! - `election_ops_spec`: Election operation correctness
//!
//! ## Read-Write Lock
//! - `rwlock_state_spec`: RWLock state model and invariants
//! - `rwlock_ops_spec`: Read/write operation correctness
//!
//! ## Semaphore
//! - `semaphore_spec`: Semaphore state model and operations
//!
//! ## Barrier
//! - `barrier_spec`: Barrier state model and operations
//!
//! ## Distributed Queue
//! - `queue_state_spec`: Queue state model and invariants
//! - `queue_enqueue_spec`: Enqueue operation correctness
//! - `queue_dequeue_spec`: Dequeue with visibility timeout correctness
//! - `queue_ack_spec`: Acknowledgment and nack operations
//!
//! ## Service Registry
//! - `registry_state_spec`: Registry state model and invariants
//! - `registry_ops_spec`: Register, deregister, heartbeat operations
//!
//! ## Rate Limiter
//! - `rate_limiter_state_spec`: Token bucket state model
//! - `rate_limiter_ops_spec`: Acquire and refill operations
//!
//! ## Worker Coordinator
//! - `worker_state_spec`: Worker/task state model
//! - `worker_ops_spec`: Register, heartbeat, assign, complete operations
//!
//! ## Fencing and Quorum (Cross-Primitive)
//! - `fencing_spec`: Fencing token validation, split-brain detection, quorum calculations
//!
//! ## Load Balancing Strategies
//! - `strategies_spec`: Load scoring, round-robin, hash ring, work stealing
//!
//! # Invariants Verified
//!
//! ## Distributed Lock
//!
//! 1. **Fencing Token Monotonicity**: Tokens strictly increase on each acquisition
//!    - Every successful acquire returns a token greater than any previously issued
//!    - This prevents split-brain scenarios where old holders think they still own the lock
//!
//! 2. **Mutual Exclusion**: At most one holder at any time (via CAS semantics)
//!    - Lock acquisition uses compare-and-swap to ensure atomicity
//!    - Only one client can successfully acquire a non-expired lock
//!
//! 3. **TTL Expiration Safety**: Expired locks become reacquirable
//!    - A lock is considered expired when current_time > deadline_ms or deadline_ms == 0
//!    - Expired locks can be taken by new holders with incremented tokens
//!
//! ## Sequence Generator
//!
//! 1. **Uniqueness**: No two next() calls ever return the same value
//!    - Proved via disjoint range property of sequential reserves
//!
//! 2. **Monotonicity**: Each returned value is strictly greater than the previous
//!    - Reserve operations strictly increase the global counter
//!
//! 3. **Batch Disjointness**: Different reserve() calls return non-overlapping ranges
//!    - Critical for multi-client uniqueness guarantees
//!
//! 4. **Overflow Safety**: Operations check and fail before overflow
//!    - checked_add prevents wrap-around
//!
//! ## Atomic Counter
//!
//! 1. **CAS Atomicity**: Each modify is atomic via compare-and-swap
//!    - Retries on contention with exponential backoff
//!
//! 2. **Saturating Arithmetic**: Operations saturate at bounds
//!    - Add saturates at u64::MAX, never wraps
//!    - Subtract saturates at 0, never goes negative
//!
//! 3. **Linearizability**: All operations are linearizable through Raft
//!    - Each CAS is a single atomic operation in the log
//!
//! ## Leader Election
//!
//! 1. **Single Leader**: At most one leader at any time
//!    - Guaranteed by underlying DistributedLock's mutual exclusion
//!
//! 2. **Fencing Token Monotonicity**: Each leadership term has strictly greater token
//!    - Inherited from DistributedLock
//!
//! 3. **Valid State Transitions**: Follower <-> Transitioning <-> Leader
//!    - Stepdown: Leader -> Follower
//!
//! ## Read-Write Lock
//!
//! 1. **Mutual Exclusion**: Multiple readers OR one exclusive writer
//!    - Cannot have readers and writers simultaneously
//!
//! 2. **Writer Preference**: Pending writers block new readers
//!    - Prevents writer starvation in read-heavy workloads
//!
//! 3. **Fencing Token Monotonicity**: Token increments on write acquisition
//!    - Each write lock gets a strictly greater token
//!
//! 4. **Downgrade Safety**: Write -> Read transition is atomic
//!    - Preserves fencing token, writer becomes reader
//!
//! ## Semaphore
//!
//! 1. **Capacity Bound**: Total permits held never exceeds capacity
//!    - Enforced by checking available before acquire
//!
//! 2. **Holder Limit**: Number of holders <= MAX_SEMAPHORE_HOLDERS
//!    - Prevents resource exhaustion
//!
//! 3. **Available Correctness**: available = capacity - used
//!    - Sum of all holder permits equals used
//!
//! ## Barrier
//!
//! 1. **Phase Consistency**: Phase matches participant count
//!    - Waiting: count < required
//!    - Ready: count >= required
//!
//! 2. **Phase Ordering**: Waiting -> Ready -> Leaving
//!    - No backward transitions allowed
//!
//! 3. **Completion**: Last leave triggers barrier deletion
//!
//! ## Distributed Queue
//!
//! 1. **QUEUE-1: FIFO Ordering**: Dequeue returns items in enqueue order
//!    - Monotonically increasing IDs ensure order
//!
//! 2. **QUEUE-2: ID Monotonicity**: next_id strictly increases
//!    - Never reuses IDs even after deletions
//!
//! 3. **QUEUE-3: State Exclusivity**: Each item in exactly one state
//!    - Pending, Inflight, Acknowledged, or DeadLettered
//!
//! 4. **QUEUE-4: Visibility Timeout**: Inflight items have valid deadlines
//!    - Auto-requeued when deadline passes
//!
//! 5. **QUEUE-5: DLQ Threshold**: DLQ items exceeded max delivery count
//!    - Prevents infinite retry loops
//!
//! ## Service Registry
//!
//! 1. **REG-1: Registration Validity**: Valid TTLs and timestamps
//!    - TTL within bounds, deadline = registered_at + ttl
//!
//! 2. **REG-2: Fencing Token Monotonicity**: Tokens strictly increase
//!    - Prevents stale registrations
//!
//! 3. **REG-3: Index Consistency**: type_index matches services
//!    - Bidirectional consistency
//!
//! 4. **REG-4: TTL Expiration Safety**: Expired services not returned
//!    - Live checks exclude expired entries
//!
//! ## Rate Limiter
//!
//! 1. **RATE-1: Capacity Bound**: Tokens never exceed capacity
//!    - Refill saturates at capacity
//!
//! 2. **RATE-2: Token Conservation**: Only change via acquire/refill
//!    - No spontaneous token changes
//!
//! 3. **RATE-3: Refill Monotonicity**: last_refill_ms only increases
//!    - Time advances monotonically
//!
//! ## Worker Coordinator
//!
//! 1. **WORK-1: Task Uniqueness**: Each task assigned to at most one worker
//!    - No duplicate assignments
//!
//! 2. **WORK-2: Worker Isolation**: No task in multiple workers' sets
//!    - Strengthened uniqueness
//!
//! 3. **WORK-3: Lease Validity**: Active workers have valid leases
//!    - Expired leases mark worker inactive
//!
//! 4. **WORK-4: Load Bounded**: Load never exceeds capacity
//!    - Prevents overloading workers
//!
//! ## Fencing and Quorum
//!
//! 1. **FENCE-1: Token Validity**: token >= min_expected is valid
//!    - Stale tokens indicate lost ownership
//!
//! 2. **FENCE-2: Quorum Majority**: quorum(n) > n/2
//!    - Guarantees only one partition can have quorum
//!    - quorum(n) = (n/2) + 1
//!
//! 3. **FENCE-3: Split-Brain Detection**: observed_token >= my_token indicates conflict
//!    - Step-down when observing strictly greater token
//!
//! 4. **FENCE-4: Failover Safety**: Triggers on timeout or consecutive failures
//!    - Wait state provides hysteresis
//!
//! 5. **FENCE-5: Lease Validity**: now <= expires_at + grace_period
//!    - Renewal happens before expiry
//!
//! ## Load Balancing Strategies
//!
//! 1. **STRAT-1: Load Score Bounds**: Scores always in [0.0, 1.0]
//!    - Composite of load and queue components
//!
//! 2. **STRAT-2: Round-Robin Validity**: Selected index < eligible_count
//!    - Fair cycling through all workers
//!
//! 3. **STRAT-3: Hash Ring Correctness**: Lookup returns valid ring entry
//!    - Binary search with wraparound
//!
//! 4. **STRAT-4: Work Stealing Safety**: Respects batch limits and load ceiling
//!    - Prevents overloading target worker
//!
//! 5. **STRAT-5: Group Balance**: All workers within tolerance of average
//!    - Rebalancing preserves total work
//!
//! # State Model
//!
//! The lock state is modeled as:
//! ```ignore
//! LockState {
//!     entry: Option<LockEntrySpec>,     // Current lock holder (if any)
//!     current_time_ms: u64,             // System time for expiration checks
//!     max_fencing_token_issued: u64,    // Highest token ever issued (monotonic)
//! }
//! ```
//!
//! # Operation Specifications
//!
//! ## Acquire
//! - Pre: Lock must be available (None or expired)
//! - Post: New holder with token = max_fencing_token_issued + 1
//! - Preserves: Fencing token monotonicity
//!
//! ## Release
//! - Pre: Caller must be current holder with matching token
//! - Post: Lock marked as released (deadline_ms = 0), token preserved
//! - Preserves: Max token tracking
//!
//! ## Renew
//! - Pre: Caller must hold non-expired lock with matching token
//! - Post: Deadline extended, same fencing token
//! - Preserves: Fencing token unchanged
//!
//! # Trusted Axioms
//!
//! The specification assumes:
//!
//! ## CAS Linearizability
//! All compare-and-swap operations are linearizable, meaning they appear to
//! execute atomically at some point between invocation and response. This is
//! provided by the underlying Raft consensus layer, which serializes all
//! state machine commands.
//!
//! ## Clock Monotonicity
//! System clocks advance monotonically within each node. This is a standard
//! OS assumption. Note that:
//! - Clocks may advance at different rates across nodes (clock skew)
//! - NTP adjustments may cause small backward jumps (we assume monotonic reads)
//! - TTL-based operations have accuracy bounded by clock skew
//!
//! ## Bounded Clock Skew
//! For TTL-based primitives (locks, leases, semaphores), the effective TTL
//! accuracy is bounded by the maximum clock skew between nodes. A lock with
//! TTL=30s may actually be held for 30s +/- clock_skew. Production systems
//! should configure TTLs with sufficient margin for their network's skew.
//!
//! ## Type Bounds
//! All integer types are bounded by their Rust definitions:
//! - u64: [0, 2^64 - 1]
//! - u32: [0, 2^32 - 1]
//! - i64: [-2^63, 2^63 - 1]
//!
//! The Verus type system enforces these bounds. Overflow protection predicates
//! in `overflow_constants_spec.rs` provide reusable safety checks.

use vstd::prelude::*;

verus! {
    // Re-export overflow constants and helpers
    pub use overflow_constants_spec::U64_MAX;
    pub use overflow_constants_spec::U32_MAX;
    pub use overflow_constants_spec::I64_MAX;
    pub use overflow_constants_spec::I64_MIN;
    pub use overflow_constants_spec::can_add_u64;
    pub use overflow_constants_spec::can_add_u32;
    pub use overflow_constants_spec::can_increment_u64;
    pub use overflow_constants_spec::can_increment_u32;
    pub use overflow_constants_spec::can_sub_u64;
    pub use overflow_constants_spec::can_sub_u32;
    pub use overflow_constants_spec::can_add_i64;
    pub use overflow_constants_spec::can_sub_i64;

    // Re-export core specifications
    pub use lock_state_spec::LockEntrySpec;
    pub use lock_state_spec::LockState;
    pub use lock_state_spec::is_expired;
    pub use lock_state_spec::is_lock_available;
    pub use lock_state_spec::is_held_by;
    pub use lock_state_spec::fencing_token_monotonic;
    pub use lock_state_spec::entry_token_bounded;
    pub use lock_state_spec::lock_invariant;

    // Lock exec functions (verified implementations)
    pub use lock_state_spec::is_lock_expired;
    pub use lock_state_spec::compute_lock_deadline;
    pub use lock_state_spec::remaining_ttl_ms;
    pub use lock_state_spec::compute_next_fencing_token;
    pub use lock_state_spec::BackoffResult;
    pub use lock_state_spec::compute_backoff_with_jitter;

    pub use acquire_spec::acquire_pre;
    pub use acquire_spec::acquire_post;
    // Acquire exec functions
    pub use acquire_spec::compute_new_fencing_token;
    pub use acquire_spec::compute_acquire_deadline;

    pub use release_spec::release_pre;
    pub use release_spec::release_post;

    pub use renew_spec::renew_pre;
    pub use renew_spec::renew_post;

    // Sequence generator exports
    pub use sequence_state_spec::SequenceState;
    pub use sequence_state_spec::ReservedRange;
    pub use sequence_state_spec::sequence_monotonic;
    pub use sequence_state_spec::sequence_invariant;

    // Sequence exec functions (verified implementations)
    pub use sequence_state_spec::should_refill_batch;
    pub use sequence_state_spec::batch_remaining;
    pub use sequence_state_spec::compute_batch_end;
    pub use sequence_state_spec::compute_next_after_refill;
    pub use sequence_state_spec::SequenceReservationResult;
    pub use sequence_state_spec::compute_new_sequence_value;
    pub use sequence_state_spec::compute_range_start;
    pub use sequence_state_spec::is_initial_reservation;
    pub use sequence_state_spec::compute_initial_current;
    pub use sequence_state_spec::compute_cas_expected;

    pub use sequence_reserve_spec::reserve_pre;
    pub use sequence_reserve_spec::reserve_post;
    pub use sequence_reserve_spec::reserve_range;

    // Counter exports
    pub use counter_state_spec::CounterState;
    pub use counter_state_spec::saturating_add_u64;
    pub use counter_state_spec::saturating_sub_u64;
    pub use counter_state_spec::counter_invariant;

    // Counter exec functions (verified implementations)
    pub use counter_state_spec::CounterOpResult;
    pub use counter_state_spec::SignedCounterOpResult;
    pub use counter_state_spec::apply_increment;
    pub use counter_state_spec::apply_decrement;
    pub use counter_state_spec::apply_signed_add;
    pub use counter_state_spec::apply_signed_sub;
    pub use counter_state_spec::compute_approximate_total;
    pub use counter_state_spec::should_flush_buffer;
    pub use counter_state_spec::compute_unsigned_cas_expected;
    pub use counter_state_spec::compute_signed_cas_expected;
    pub use counter_state_spec::compute_retry_delay;

    pub use counter_ops_spec::add_pre;
    pub use counter_ops_spec::add_post;
    pub use counter_ops_spec::sub_pre;
    pub use counter_ops_spec::sub_post;

    // Election exports
    pub use election_state_spec::ElectionState;
    pub use election_state_spec::LeadershipStateSpec;
    pub use election_state_spec::is_leader;
    pub use election_state_spec::election_invariant;
    // Election timing exec functions
    pub use election_state_spec::compute_next_renew_time;
    pub use election_state_spec::is_renewal_time;
    pub use election_state_spec::should_maintain_leadership;
    pub use election_state_spec::compute_renewal_backoff;

    pub use election_ops_spec::win_election_pre;
    pub use election_ops_spec::win_election_post;
    pub use election_ops_spec::stepdown_pre;
    pub use election_ops_spec::stepdown_post;

    // RWLock exports
    pub use rwlock_state_spec::RWLockStateSpec;
    pub use rwlock_state_spec::RWLockModeSpec;
    pub use rwlock_state_spec::mutual_exclusion_holds;
    pub use rwlock_state_spec::rwlock_invariant;

    pub use rwlock_ops_spec::acquire_read_pre;
    pub use rwlock_ops_spec::acquire_read_post;
    pub use rwlock_ops_spec::acquire_write_pre;
    pub use rwlock_ops_spec::acquire_write_post;

    // Semaphore exports
    pub use semaphore_spec::SemaphoreStateSpec;
    pub use semaphore_spec::semaphore_invariant;
    pub use semaphore_spec::acquire_pre as semaphore_acquire_pre;
    pub use semaphore_spec::acquire_post as semaphore_acquire_post;

    // Semaphore exec functions (verified implementations)
    pub use semaphore_spec::calculate_available_permits;
    pub use semaphore_spec::can_acquire_permits;
    pub use semaphore_spec::compute_holder_deadline;
    pub use semaphore_spec::is_holder_expired;

    // Barrier exports
    pub use barrier_spec::BarrierStateSpec;
    pub use barrier_spec::BarrierPhaseSpec;
    pub use barrier_spec::barrier_invariant;
    pub use barrier_spec::enter_pre as barrier_enter_pre;
    pub use barrier_spec::enter_post as barrier_enter_post;

    // Barrier exec functions (verified implementations)
    pub use barrier_spec::BarrierPhase;
    pub use barrier_spec::compute_initial_barrier_phase;
    pub use barrier_spec::is_barrier_ready_exec;
    pub use barrier_spec::should_transition_to_ready;
    pub use barrier_spec::should_start_leave_phase;
    pub use barrier_spec::is_valid_phase_transition;
    pub use barrier_spec::is_barrier_overdue;
    pub use barrier_spec::compute_expected_completion_time;

    // Queue exports
    pub use queue_state_spec::QueueState;
    pub use queue_state_spec::QueueItemSpec;
    pub use queue_state_spec::QueueItemStateSpec;
    pub use queue_state_spec::queue_invariant;
    pub use queue_state_spec::fifo_ordering;

    pub use queue_enqueue_spec::enqueue_pre;
    pub use queue_enqueue_spec::enqueue_post;

    pub use queue_dequeue_spec::dequeue_pre;
    pub use queue_dequeue_spec::dequeue_single_effect;

    pub use queue_ack_spec::ack_pre;
    pub use queue_ack_spec::ack_post;

    // Registry exports
    pub use registry_state_spec::RegistryState;
    pub use registry_state_spec::ServiceEntrySpec;
    pub use registry_state_spec::registry_invariant;
    pub use registry_state_spec::is_live;

    pub use registry_ops_spec::register_pre;
    pub use registry_ops_spec::register_post;
    pub use registry_ops_spec::deregister_pre;
    pub use registry_ops_spec::deregister_post;
    pub use registry_ops_spec::heartbeat_pre;
    pub use registry_ops_spec::heartbeat_post;

    // Rate limiter exports
    pub use rate_limiter_state_spec::RateLimiterState;
    pub use rate_limiter_state_spec::rate_limiter_invariant;
    pub use rate_limiter_state_spec::has_tokens;

    // Rate limiter exec functions (verified implementations)
    // NOTE: Float-based functions skipped (Verus doesn't support floats)
    pub use rate_limiter_state_spec::has_tokens_available;
    pub use rate_limiter_state_spec::consume_tokens;
    pub use rate_limiter_state_spec::refill_tokens;
    pub use rate_limiter_state_spec::calculate_intervals_elapsed;
    pub use rate_limiter_state_spec::is_refill_needed;

    pub use rate_limiter_ops_spec::acquire_pre as rate_limiter_acquire_pre;
    pub use rate_limiter_ops_spec::acquire_post as rate_limiter_acquire_post;
    pub use rate_limiter_ops_spec::refill_pre;
    pub use rate_limiter_ops_spec::refill_post;

    // Worker coordinator exports
    pub use worker_state_spec::WorkerState;
    pub use worker_state_spec::WorkerEntrySpec;
    pub use worker_state_spec::TaskAssignmentSpec;
    pub use worker_state_spec::worker_invariant;
    pub use worker_state_spec::has_capacity;

    pub use worker_ops_spec::register_worker_pre;
    pub use worker_ops_spec::register_worker_post;
    pub use worker_ops_spec::heartbeat_pre as worker_heartbeat_pre;
    pub use worker_ops_spec::heartbeat_post as worker_heartbeat_post;
    pub use worker_ops_spec::assign_task_pre;
    pub use worker_ops_spec::assign_task_post;
    pub use worker_ops_spec::complete_task_pre;
    pub use worker_ops_spec::complete_task_post;

    // Fencing and quorum exports
    pub use fencing_spec::token_is_valid;
    pub use fencing_spec::token_is_stale;
    pub use fencing_spec::tokens_consistent;
    pub use fencing_spec::quorum_threshold;
    pub use fencing_spec::has_quorum;
    pub use fencing_spec::partition_has_quorum;
    pub use fencing_spec::indicates_split_brain;
    pub use fencing_spec::should_step_down;
    pub use fencing_spec::FailoverDecisionSpec;
    pub use fencing_spec::failover_triggered;
    pub use fencing_spec::compute_failover_decision;
    pub use fencing_spec::lease_is_valid;
    pub use fencing_spec::lease_renew_time;
    pub use fencing_spec::FencingState;
    pub use fencing_spec::fencing_invariant;
    pub use fencing_spec::is_safe_state;

    // Fencing exec functions (verified implementations)
    pub use fencing_spec::is_token_valid_exec;
    pub use fencing_spec::is_token_stale_exec;
    pub use fencing_spec::compute_quorum_threshold;
    pub use fencing_spec::has_quorum_exec;
    pub use fencing_spec::partition_maintains_quorum;
    pub use fencing_spec::check_for_split_brain;
    pub use fencing_spec::should_step_down_exec;
    pub use fencing_spec::FailoverDecision;
    pub use fencing_spec::should_trigger_failover;
    pub use fencing_spec::is_lease_valid_exec;
    pub use fencing_spec::compute_lease_renew_time;
    pub use fencing_spec::compute_election_timeout_with_jitter;

    // Load balancing strategy exports
    pub use strategies_spec::load_score_bounded;
    pub use strategies_spec::compute_load_score_scaled;
    pub use strategies_spec::round_robin_valid;
    pub use strategies_spec::compute_round_robin;
    pub use strategies_spec::ring_is_sorted;
    pub use strategies_spec::lookup_result_valid;
    pub use strategies_spec::should_continue_stealing;
    pub use strategies_spec::worker_is_idle;
    pub use strategies_spec::StealStrategySpec;
    pub use strategies_spec::is_steal_source_eligible;
    pub use strategies_spec::group_is_balanced;
    pub use strategies_spec::compute_affinity_score;
    pub use strategies_spec::affinity_score_bounded;
    pub use strategies_spec::SelectionResultSpec;
    pub use strategies_spec::selection_result_valid;
    pub use strategies_spec::WorkerLoadState;
    pub use strategies_spec::WorkerGroup;
    pub use strategies_spec::worker_group_invariant;
}

mod acquire_spec;
mod barrier_spec;
mod counter_ops_spec;
mod counter_state_spec;
mod election_ops_spec;
mod election_state_spec;
mod lock_state_spec;
mod overflow_constants_spec;
mod release_spec;
mod renew_spec;
mod rwlock_ops_spec;
mod rwlock_state_spec;
mod semaphore_spec;
mod sequence_reserve_spec;
mod sequence_state_spec;

// Queue specifications
mod queue_ack_spec;
mod queue_dequeue_spec;
mod queue_enqueue_spec;
mod queue_state_spec;

// Registry specifications
mod registry_ops_spec;
mod registry_state_spec;

// Rate limiter specifications
mod rate_limiter_ops_spec;
mod rate_limiter_state_spec;

// Worker coordinator specifications
mod worker_ops_spec;
mod worker_state_spec;

// Fencing and quorum specifications
mod fencing_spec;

// Load balancing strategy specifications
mod strategies_spec;
