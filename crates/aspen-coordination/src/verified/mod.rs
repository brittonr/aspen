//! Verified pure functions for distributed coordination primitives.
//!
//! This module contains the production implementations of pure business logic
//! for distributed coordination. All functions are:
//!
//! - **Deterministic**: No I/O, no system calls, time passed as explicit parameter
//! - **Verified**: Formally proved correct using Verus (see `verus/` directory)
//! - **Production-ready**: Compiled normally by cargo with no ghost code overhead
//!
//! # Architecture
//!
//! This module implements the "Functional Core, Imperative Shell" (FCIS) pattern:
//!
//! - **verified/** (this module): Production exec functions compiled by cargo
//! - **verus/**: Standalone Verus specs with ensures/requires clauses verified by Verus
//!
//! The exec functions here have corresponding spec functions in `verus/` that prove
//! correctness. The specs reference the same logic but with formal verification.
//!
//! # Verification
//!
//! Run Verus verification with:
//! ```bash
//! nix run .#verify-verus              # Verify all specs (Core + Raft + Coordination)
//! nix run .#verify-verus coordination # Verify coordination specs only
//! ```
//!
//! # Module Organization
//!
//! - [`sequence`]: Batch state, range reservation, overflow checking
//! - [`counter`]: Saturating increment/decrement, buffered counters
//! - [`barrier`]: Phase transitions, readiness checks, deadlock detection
//! - [`lock`]: Lock expiry, fencing tokens, backoff with jitter
//! - [`election`]: Leadership state transitions, renewal timing
//! - [`queue`]: Item expiry, DLQ decisions, visibility timeout
//! - [`rwlock`]: Reader/writer acquisition, mode transitions
//! - [`rate_limiter`]: Token bucket calculations, replenishment
//! - [`registry`]: Instance expiry, discovery filtering
//! - [`semaphore`]: Permit calculations, holder expiry
//! - [`worker`]: Capacity, liveness, work stealing
//! - [`fencing`]: Split-brain detection, quorum calculations
//! - [`strategies`]: Load scoring, round-robin, hash ring
//!
//! # Tiger Style
//!
//! All functions follow Tiger Style principles:
//! - Saturating/checked arithmetic (no panics)
//! - Explicit types (u64, u32, not usize)
//! - Functions under 70 lines
//! - Fixed limits on all data structures

pub mod barrier;
pub mod counter;
pub mod election;
pub mod fencing;
pub mod lock;
pub mod queue;
pub mod rate_limiter;
pub mod registry;
pub mod rwlock;
pub mod semaphore;
pub mod sequence;
pub mod strategies;
pub mod worker;

// ============================================================================
// Re-exports: Lock
// ============================================================================

// Fencing Token
pub use barrier::DeadlockCheckResult;
pub use barrier::ParticipantActivity;
// Deadlock Detection
pub use barrier::check_barrier_deadlock;
pub use barrier::compute_expected_completion_time;
// ============================================================================
// Re-exports: Barrier
// ============================================================================

// Phase Transitions
pub use barrier::compute_initial_barrier_phase;
pub use barrier::detect_stalled_participants;
pub use barrier::is_barrier_overdue;
pub use barrier::is_barrier_ready;
// Leave Phase
pub use barrier::is_valid_phase_transition;
pub use barrier::should_start_leave_phase;
pub use barrier::should_transition_to_ready;
pub use counter::CounterOpResult;
pub use counter::ParseSignedResult;
pub use counter::ParseUnsignedResult;
pub use counter::SignedCounterOpResult;
// ============================================================================
// Re-exports: Counter
// ============================================================================

// Unsigned Operations
pub use counter::apply_decrement;
pub use counter::apply_increment;
// Signed Operations
pub use counter::apply_signed_add;
pub use counter::apply_signed_sub;
// Buffered Counter
pub use counter::compute_approximate_total;
// Retry
pub use counter::compute_retry_delay;
// CAS Expected
pub use counter::compute_signed_cas_expected;
pub use counter::compute_unsigned_cas_expected;
// Parsing
pub use counter::parse_signed_counter;
pub use counter::parse_unsigned_counter;
pub use counter::should_flush_buffer;
// Timing Logic
pub use election::compute_election_interval_with_jitter;
// ============================================================================
// Re-exports: Election
// ============================================================================

// State Transitions
pub use election::compute_next_leadership_state;
pub use election::compute_next_renew_time;
pub use election::compute_renewal_backoff;
pub use election::is_renewal_time;
pub use election::should_maintain_leadership;
pub use fencing::FailoverDecision;
pub use fencing::FencingValidation;
pub use fencing::SplitBrainCheck;
// Split-Brain Detection
pub use fencing::check_for_split_brain;
// Failover
pub use fencing::compute_election_timeout_with_jitter;
// Lease Validation
pub use fencing::compute_lease_renew_time;
// Quorum
pub use fencing::compute_quorum_threshold;
pub use fencing::has_quorum;
pub use fencing::is_lease_valid;
// ============================================================================
// Re-exports: Fencing
// ============================================================================

// Token Validation
pub use fencing::is_token_valid;
pub use fencing::partition_maintains_quorum;
pub use fencing::should_step_down;
pub use fencing::should_trigger_failover;
pub use fencing::validate_consistent_fencing_tokens;
// Backoff
pub use lock::BackoffResult;
pub use lock::compute_backoff_with_jitter;
// TTL and Deadline
pub use lock::compute_lock_deadline;
pub use lock::compute_next_fencing_token;
pub use lock::is_lock_expired;
pub use lock::remaining_ttl_ms;
// DLQ Decision
pub use queue::DLQDecision;
pub use queue::DequeueEligibility;
pub use queue::PendingGroupInfo;
// ============================================================================
// Re-exports: Queue
// ============================================================================

// Key Generation
pub use queue::QUEUE_PREFIX;
pub use queue::RequeuePriority;
// Message Group Filtering
pub use queue::can_dequeue_from_group;
// Dequeue Eligibility
pub use queue::check_dequeue_eligibility;
// Batch Size
pub use queue::compute_dequeue_batch_size;
pub use queue::compute_effective_visibility_timeout;
// Item Construction
pub use queue::compute_item_expiration;
// Delivery Attempts
pub use queue::compute_requeue_delivery_attempts;
// Requeue Priority
pub use queue::compute_requeue_priority;
// Visibility Timeout
pub use queue::compute_visibility_deadline;
pub use queue::create_queue_item_from_pending;
pub use queue::dedup_key;
pub use queue::dedup_prefix;
pub use queue::dlq_key;
pub use queue::dlq_prefix;
// Receipt Handle
pub use queue::generate_receipt_handle;
pub use queue::has_exceeded_max_delivery_attempts;
// Expiry Checks
pub use queue::is_dedup_entry_expired;
pub use queue::is_queue_item_expired;
pub use queue::is_visibility_expired;
pub use queue::item_key;
pub use queue::items_prefix;
pub use queue::parse_receipt_handle;
pub use queue::pending_key;
pub use queue::pending_prefix;
pub use queue::queue_metadata_key;
pub use queue::sequence_key;
pub use queue::should_move_to_dlq;
pub use queue::should_skip_for_message_group;
// Availability Check
pub use rate_limiter::TokenAvailability;
// ============================================================================
// Re-exports: Rate Limiter
// ============================================================================

// Token Calculation
pub use rate_limiter::calculate_replenished_tokens;
pub use rate_limiter::check_token_availability;
// ============================================================================
// Re-exports: Registry
// ============================================================================

// Key Prefix
pub use registry::SERVICE_PREFIX;
// Heartbeat
pub use registry::compute_heartbeat_deadline;
pub use registry::compute_next_instance_token;
// Key Generation
pub use registry::instance_key;
// Instance Expiry
pub use registry::instance_remaining_ttl;
pub use registry::is_instance_expired;
// Discovery
pub use registry::matches_discovery_filter;
pub use registry::service_instances_prefix;
pub use registry::services_scan_prefix;
// ============================================================================
// Re-exports: RWLock
// ============================================================================

// Key Prefix
pub use rwlock::RWLOCK_PREFIX;
// Acquisition Logic
pub use rwlock::can_acquire_read;
pub use rwlock::can_acquire_write;
// Deadline
pub use rwlock::compute_lock_deadline as compute_rwlock_deadline;
// Token and Mode
pub use rwlock::compute_mode_after_cleanup;
pub use rwlock::compute_next_write_token;
// State Checks
pub use rwlock::count_active_readers;
pub use rwlock::filter_expired_readers;
pub use rwlock::has_read_lock;
pub use rwlock::has_write_lock;
// Expiry Checks
pub use rwlock::is_reader_expired;
pub use rwlock::is_writer_expired;
// Key Generation
pub use rwlock::rwlock_key;
// ============================================================================
// Re-exports: Semaphore
// ============================================================================

// Key Prefix
pub use semaphore::SEMAPHORE_PREFIX;
// Permit Calculations
pub use semaphore::calculate_available_permits;
pub use semaphore::can_acquire_permits;
pub use semaphore::compute_holder_deadline;
// Holder Expiry
pub use semaphore::is_holder_expired;
// Key Generation
pub use semaphore::semaphore_key;
pub use sequence::ParseSequenceResult;
pub use sequence::SequenceReservationResult;
// ============================================================================
// Re-exports: Sequence
// ============================================================================

// Batch State
pub use sequence::batch_remaining;
// Batch Computation
pub use sequence::compute_batch_end;
// Reservation
pub use sequence::compute_cas_expected;
// Initial State
pub use sequence::compute_initial_current;
pub use sequence::compute_new_sequence_value;
pub use sequence::compute_next_after_refill;
pub use sequence::compute_range_start;
pub use sequence::is_initial_reservation;
// Parsing
pub use sequence::parse_sequence_value;
pub use sequence::should_refill_batch;
pub use strategies::RankedStealTarget;
pub use strategies::SelectionResult;
pub use strategies::StealStrategy;
pub use strategies::WorkerStealInfo;
// ============================================================================
// Re-exports: Strategies
// ============================================================================

// Load Score
pub use strategies::calculate_load_score;
// Work Stealing Heuristics
pub use strategies::compute_affinity_score;
pub use strategies::compute_group_average_load;
pub use strategies::compute_rebalance_pairs;
// Round Robin
pub use strategies::compute_round_robin_selection;
// Metrics
pub use strategies::compute_running_average;
// Hashing
pub use strategies::compute_virtual_node_hash;
pub use strategies::hash_key;
pub use strategies::is_group_balanced;
// Work Stealing
pub use strategies::is_worker_idle_for_stealing;
pub use strategies::lookup_hash_ring;
pub use strategies::rank_steal_targets;
// Selection
pub use strategies::select_from_scored;
pub use strategies::should_continue_stealing;
// Filtering
pub use strategies::worker_matches_tags;
// ============================================================================
// Re-exports: Worker
// ============================================================================

// Key Prefixes
pub use worker::STEAL_HINT_PREFIX;
pub use worker::WORKER_GROUP_PREFIX;
pub use worker::WORKER_STATS_PREFIX;
// Capacity
pub use worker::calculate_available_capacity;
pub use worker::can_handle_job;
// Work Stealing
pub use worker::compute_steal_batch_size;
pub use worker::compute_steal_hint_deadline;
pub use worker::is_steal_hint_expired;
pub use worker::is_steal_source;
pub use worker::is_steal_target;
// Liveness
pub use worker::is_worker_alive;
// Filtering
pub use worker::matches_capability_filter;
pub use worker::matches_load_filter;
pub use worker::matches_node_filter;
pub use worker::matches_tags_filter;
// Hashing
pub use worker::simple_hash;
// Key Generation
pub use worker::steal_hint_key;
pub use worker::steal_hint_prefix;
pub use worker::steal_hint_remaining_ttl;
pub use worker::worker_group_key;
pub use worker::worker_stats_key;
