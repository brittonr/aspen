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
//! - CAS operations are linearizable (provided by Raft consensus)
//! - System clock advances monotonically (standard OS assumption)

use vstd::prelude::*;

verus! {
    // Re-export core specifications
    pub use lock_state_spec::LockEntrySpec;
    pub use lock_state_spec::LockState;
    pub use lock_state_spec::is_expired;
    pub use lock_state_spec::is_lock_available;
    pub use lock_state_spec::is_held_by;
    pub use lock_state_spec::fencing_token_monotonic;
    pub use lock_state_spec::entry_token_bounded;
    pub use lock_state_spec::lock_invariant;

    pub use acquire_spec::acquire_pre;
    pub use acquire_spec::acquire_post;

    pub use release_spec::release_pre;
    pub use release_spec::release_post;

    pub use renew_spec::renew_pre;
    pub use renew_spec::renew_post;

    // Sequence generator exports
    pub use sequence_state_spec::SequenceState;
    pub use sequence_state_spec::ReservedRange;
    pub use sequence_state_spec::sequence_monotonic;
    pub use sequence_state_spec::sequence_invariant;

    pub use sequence_reserve_spec::reserve_pre;
    pub use sequence_reserve_spec::reserve_post;
    pub use sequence_reserve_spec::reserve_range;

    // Counter exports
    pub use counter_state_spec::CounterState;
    pub use counter_state_spec::saturating_add_u64;
    pub use counter_state_spec::saturating_sub_u64;
    pub use counter_state_spec::counter_invariant;

    pub use counter_ops_spec::add_pre;
    pub use counter_ops_spec::add_post;
    pub use counter_ops_spec::sub_pre;
    pub use counter_ops_spec::sub_post;

    // Election exports
    pub use election_state_spec::ElectionState;
    pub use election_state_spec::LeadershipStateSpec;
    pub use election_state_spec::is_leader;
    pub use election_state_spec::election_invariant;

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

    // Barrier exports
    pub use barrier_spec::BarrierStateSpec;
    pub use barrier_spec::BarrierPhaseSpec;
    pub use barrier_spec::barrier_invariant;
    pub use barrier_spec::enter_pre as barrier_enter_pre;
    pub use barrier_spec::enter_post as barrier_enter_post;

    // Queue exports
    pub use queue_state_spec::QueueState;
    pub use queue_state_spec::QueueItemSpec;
    pub use queue_state_spec::QueueItemStateSpec;
    pub use queue_state_spec::queue_invariant;
    pub use queue_state_spec::fifo_ordering;

    pub use queue_enqueue_spec::enqueue_pre;
    pub use queue_enqueue_spec::enqueue_post;

    pub use queue_dequeue_spec::dequeue_pre;
    pub use queue_dequeue_spec::dequeue_post;

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
}

mod acquire_spec;
mod barrier_spec;
mod counter_ops_spec;
mod counter_state_spec;
mod election_ops_spec;
mod election_state_spec;
mod lock_state_spec;
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
