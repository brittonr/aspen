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
// Barrier (Verus-aligned)
pub use barrier::is_barrier_ready_exec;
// Leave Phase
pub use barrier::is_valid_phase_transition;
pub use barrier::should_start_leave_phase;
pub use barrier::should_transition_to_ready;
pub use barrier::time_until_expiration;
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
// Election Timing
pub use election::calculate_election_timeout;
pub use election::can_lose_election;
pub use election::can_lose_leadership;
// Election Preconditions
pub use election::can_start_election;
pub use election::can_stepdown;
pub use election::can_win_election;
// Timing Logic
pub use election::compute_election_interval_with_jitter;
pub use election::compute_max_token_after_win;
// Token Computation
pub use election::compute_next_election_token;
// ============================================================================
// Re-exports: Election
// ============================================================================

// State Transitions
pub use election::compute_next_leadership_state;
pub use election::compute_next_renew_time;
pub use election::compute_renewal_backoff;
pub use election::get_state_after_lose_election;
pub use election::get_state_after_lose_leadership;
// State After Transitions
pub use election::get_state_after_start_election;
pub use election::get_state_after_stepdown;
pub use election::get_state_after_win_election;
// State Predicates (Verus-aligned)
pub use election::is_follower_exec;
pub use election::is_leader_exec;
pub use election::is_leader_state_wellformed;
pub use election::is_renewal_time;
pub use election::is_transitioning_exec;
pub use election::is_valid_state_transition;
pub use election::should_maintain_leadership;
pub use election::should_start_election;
pub use election::should_step_down_exec;
pub use election::should_stop_running_after_stepdown;
pub use fencing::FailoverDecision;
pub use fencing::FencingValidation;
pub use fencing::SplitBrainCheck;
// Split-Brain Detection
pub use fencing::check_for_split_brain;
// Failover
pub use fencing::compute_election_timeout_with_jitter;
// Verus-aligned variants (integer-based)
pub use fencing::compute_election_timeout_with_jitter_u32;
// Lease Validation
pub use fencing::compute_lease_renew_time;
pub use fencing::compute_lease_renew_time_percent;
// Quorum
pub use fencing::compute_quorum_threshold;
pub use fencing::has_quorum;
pub use fencing::has_quorum_exec;
pub use fencing::is_lease_valid;
pub use fencing::is_lease_valid_exec;
pub use fencing::is_token_stale_exec;
// ============================================================================
// Re-exports: Fencing
// ============================================================================

// Token Validation
pub use fencing::is_token_valid;
pub use fencing::is_token_valid_exec;
pub use fencing::partition_maintains_quorum;
pub use fencing::should_step_down;
pub use fencing::should_step_down_exec as fencing_should_step_down_exec;
pub use fencing::should_trigger_failover;
pub use fencing::validate_consistent_fencing_tokens;
// Backoff
pub use lock::BackoffResult;
pub use lock::compute_acquire_deadline;
pub use lock::compute_backoff_with_jitter;
// TTL and Deadline
pub use lock::compute_lock_deadline;
pub use lock::compute_new_fencing_token;
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
// Dequeue Validation (Verus-aligned)
pub use queue::are_dequeue_params_valid;
pub use queue::calculate_dedup_expiration;
pub use queue::calculate_extended_deadline;
pub use queue::calculate_visibility_deadline;
pub use queue::can_compute_dedup_ttl;
// Message Group Filtering
pub use queue::can_dequeue_from_group;
pub use queue::can_extend_visibility;
pub use queue::can_increment_delivery_count;
// Dequeue Eligibility
pub use queue::check_dequeue_eligibility;
pub use queue::compute_dedup_expiration;
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
// Delivery Count Operations
pub use queue::increment_delivery_count_for_dequeue;
// Ack/Nack/Redrive Validation
pub use queue::is_ack_valid;
pub use queue::is_acquire_valid as queue_is_acquire_valid;
// Expiry Checks
pub use queue::is_dedup_entry_expired;
pub use queue::is_duplicate_message;
pub use queue::is_nack_valid;
pub use queue::is_queue_item_expired;
pub use queue::is_redrive_valid;
pub use queue::is_visibility_expired;
// Visibility Timeout (Verus-aligned)
pub use queue::is_visibility_expired_exec;
pub use queue::is_visibility_timeout_expired;
pub use queue::item_key;
pub use queue::items_prefix;
pub use queue::parse_receipt_handle;
pub use queue::pending_key;
pub use queue::pending_prefix;
pub use queue::queue_metadata_key;
pub use queue::sequence_key;
pub use queue::should_move_to_dlq;
// DLQ Decisions (exec version)
pub use queue::should_move_to_dlq_exec;
pub use queue::should_skip_for_message_group;
// Availability Check
pub use rate_limiter::TokenAvailability;
// Interval Calculations
pub use rate_limiter::calculate_intervals_elapsed;
pub use rate_limiter::calculate_load_factor;
pub use rate_limiter::calculate_new_last_refill;
// ============================================================================
// Re-exports: Rate Limiter
// ============================================================================

// Token Calculation
pub use rate_limiter::calculate_replenished_tokens;
// Burst and Rate Calculations
pub use rate_limiter::can_handle_burst_exec;
pub use rate_limiter::check_token_availability;
pub use rate_limiter::compute_rate_per_second;
pub use rate_limiter::compute_tokens_to_add;
pub use rate_limiter::consume_tokens;
// Token Operations (Verus-aligned)
pub use rate_limiter::has_tokens_available;
// Precondition Checks
pub use rate_limiter::is_acquire_valid as rate_limiter_is_acquire_valid;
pub use rate_limiter::is_refill_time_valid;
pub use rate_limiter::refill_tokens;
// Constants
pub use registry::MAX_REGISTRY_TTL_MS;
pub use registry::MAX_SERVICE_WEIGHT;
pub use registry::MIN_SERVICE_WEIGHT;
// ============================================================================
// Re-exports: Registry
// ============================================================================

// Key Prefix
pub use registry::SERVICE_PREFIX;
// Service Lifecycle
pub use registry::calculate_heartbeat_deadline;
pub use registry::calculate_service_deadline;
pub use registry::can_compute_deadline;
// Heartbeat
pub use registry::compute_heartbeat_deadline;
pub use registry::compute_next_instance_token;
// Token (Verus-aligned)
pub use registry::compute_next_registry_token;
// Key Generation
pub use registry::instance_key;
// Instance Expiry
pub use registry::instance_remaining_ttl;
pub use registry::is_instance_expired;
pub use registry::is_service_expired;
pub use registry::is_service_live;
// Registration Validation
pub use registry::is_valid_heartbeat;
pub use registry::is_valid_registration;
pub use registry::is_valid_registration_params;
// Discovery
pub use registry::matches_discovery_filter;
pub use registry::normalize_service_weight;
pub use registry::service_instances_prefix;
pub use registry::services_scan_prefix;
pub use registry::time_until_lease_expiration;
// ============================================================================
// Re-exports: RWLock
// ============================================================================

// Key Prefix
pub use rwlock::RWLOCK_PREFIX;
// Acquisition Logic
pub use rwlock::can_acquire_read;
pub use rwlock::can_acquire_write;
pub use rwlock::can_downgrade_lock;
// Lock Release Preconditions
pub use rwlock::can_release_read_lock;
pub use rwlock::can_release_write_lock;
// Invariant Checks
pub use rwlock::check_mutual_exclusion;
pub use rwlock::check_readers_bounded;
pub use rwlock::compute_fencing_token_after_write_acquire;
// Deadline
pub use rwlock::compute_lock_deadline as compute_rwlock_deadline;
// Token and Mode
pub use rwlock::compute_mode_after_cleanup;
// Token (Verus-aligned)
pub use rwlock::compute_next_rwlock_fencing_token;
pub use rwlock::compute_next_write_token;
pub use rwlock::compute_pending_writers_after_acquire;
pub use rwlock::compute_reader_count_after_acquire;
pub use rwlock::compute_reader_count_after_downgrade;
pub use rwlock::compute_reader_count_after_release;
// State Checks
pub use rwlock::count_active_readers;
pub use rwlock::decrement_pending_writers;
pub use rwlock::decrement_reader_count;
pub use rwlock::filter_expired_readers;
pub use rwlock::has_read_lock;
pub use rwlock::has_write_lock;
// Pending Writer Operations
pub use rwlock::increment_pending_writers;
// Reader Count Operations
pub use rwlock::increment_reader_count;
// Expiry Checks
pub use rwlock::is_reader_expired;
pub use rwlock::is_writer_expired;
// Mode Transitions
pub use rwlock::mode_after_read_acquire;
pub use rwlock::mode_after_read_release;
pub use rwlock::mode_after_write_release;
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
pub use worker::calculate_available_capacity_f32;
// Worker Lease Operations
pub use worker::calculate_worker_lease_deadline;
pub use worker::calculate_worker_load_factor;
pub use worker::can_assign_task_to_worker;
pub use worker::can_handle_job;
// Work Stealing
pub use worker::compute_steal_batch_size;
pub use worker::compute_steal_hint_deadline;
pub use worker::decrement_worker_load;
// Worker Load Operations
pub use worker::increment_worker_load;
pub use worker::is_steal_hint_expired;
pub use worker::is_steal_source;
pub use worker::is_steal_target;
pub use worker::is_valid_worker_capacity;
pub use worker::is_worker_active;
// Liveness
pub use worker::is_worker_alive;
pub use worker::is_worker_lease_expired;
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
pub use worker::time_until_worker_lease_expiration;
pub use worker::worker_group_key;
pub use worker::worker_has_capacity;
pub use worker::worker_stats_key;
