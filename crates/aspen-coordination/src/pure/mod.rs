//! Pure functions extracted from coordination module for improved testability.
//!
//! This module implements the "Functional Core, Imperative Shell" pattern by
//! extracting pure business logic from impure async functions. All functions
//! here are deterministic and side-effect free, making them ideal for:
//!
//! - Unit testing with explicit inputs/outputs
//! - Property-based testing with Bolero
//! - Fuzzing for edge case discovery
//! - Formal verification with Verus
//! - WASM compilation (no async/I/O dependencies)
//!
//! # Module Organization
//!
//! - [`lock`]: Lock state computation, fencing token logic, backoff calculations
//! - [`queue`]: Queue item expiry, DLQ decisions, receipt handle parsing
//! - [`rate_limiter`]: Token bucket calculations, replenishment logic
//! - [`barrier`]: Barrier phase transitions, readiness checks
//! - [`election`]: Leadership state transitions
//! - [`semaphore`]: Permit calculations, holder expiry
//! - [`rwlock`]: Reader/writer acquisition, mode transitions
//! - [`worker`]: Capacity, liveness, work stealing
//! - [`registry`]: Instance expiry, discovery filtering
//!
//! # Tiger Style
//!
//! - All calculations use saturating arithmetic to prevent overflow/underflow
//! - Deterministic behavior (time passed as explicit parameter, no I/O)
//! - Explicit types (u64, u32, not usize)
//! - No panics - all functions are total

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
pub use lock::compute_next_fencing_token;

// TTL and Deadline
pub use lock::compute_lock_deadline;
pub use lock::is_lock_expired;
pub use lock::remaining_ttl_ms;

// Backoff
pub use lock::BackoffResult;
pub use lock::compute_backoff_with_jitter;

// ============================================================================
// Re-exports: Queue
// ============================================================================

// Key Generation
pub use queue::QUEUE_PREFIX;
pub use queue::dedup_key;
pub use queue::dedup_prefix;
pub use queue::dlq_key;
pub use queue::dlq_prefix;
pub use queue::item_key;
pub use queue::items_prefix;
pub use queue::pending_key;
pub use queue::pending_prefix;
pub use queue::queue_metadata_key;
pub use queue::sequence_key;

// Expiry Checks
pub use queue::is_dedup_entry_expired;
pub use queue::is_queue_item_expired;
pub use queue::is_visibility_expired;

// DLQ Decision
pub use queue::DLQDecision;
pub use queue::should_move_to_dlq;

// Item Construction
pub use queue::compute_item_expiration;
pub use queue::create_queue_item_from_pending;

// Visibility Timeout
pub use queue::compute_visibility_deadline;

// Receipt Handle
pub use queue::generate_receipt_handle;
pub use queue::parse_receipt_handle;

// Message Group Filtering
pub use queue::can_dequeue_from_group;
pub use queue::should_skip_for_message_group;
pub use queue::PendingGroupInfo;

// Delivery Attempts
pub use queue::compute_requeue_delivery_attempts;
pub use queue::has_exceeded_max_delivery_attempts;

// Batch Size
pub use queue::compute_dequeue_batch_size;
pub use queue::compute_effective_visibility_timeout;

// Requeue Priority
pub use queue::compute_requeue_priority;
pub use queue::RequeuePriority;

// Dequeue Eligibility
pub use queue::check_dequeue_eligibility;
pub use queue::DequeueEligibility;

// ============================================================================
// Re-exports: Rate Limiter
// ============================================================================

// Token Calculation
pub use rate_limiter::calculate_replenished_tokens;

// Availability Check
pub use rate_limiter::TokenAvailability;
pub use rate_limiter::check_token_availability;

// ============================================================================
// Re-exports: Barrier
// ============================================================================

// Phase Transitions
pub use barrier::compute_initial_barrier_phase;
pub use barrier::is_barrier_ready;
pub use barrier::should_transition_to_ready;

// Leave Phase
pub use barrier::is_valid_phase_transition;
pub use barrier::should_start_leave_phase;

// Deadlock Detection
pub use barrier::check_barrier_deadlock;
pub use barrier::compute_expected_completion_time;
pub use barrier::detect_stalled_participants;
pub use barrier::is_barrier_overdue;
pub use barrier::DeadlockCheckResult;
pub use barrier::ParticipantActivity;

// ============================================================================
// Re-exports: Election
// ============================================================================

// State Transitions
pub use election::compute_next_leadership_state;

// Timing Logic
pub use election::compute_election_interval_with_jitter;
pub use election::compute_next_renew_time;
pub use election::compute_renewal_backoff;
pub use election::is_renewal_time;
pub use election::should_maintain_leadership;

// ============================================================================
// Re-exports: Semaphore
// ============================================================================

// Key Prefix
pub use semaphore::SEMAPHORE_PREFIX;

// Key Generation
pub use semaphore::semaphore_key;

// Holder Expiry
pub use semaphore::is_holder_expired;

// Permit Calculations
pub use semaphore::calculate_available_permits;
pub use semaphore::can_acquire_permits;
pub use semaphore::compute_holder_deadline;

// ============================================================================
// Re-exports: RWLock
// ============================================================================

// Key Prefix
pub use rwlock::RWLOCK_PREFIX;

// Key Generation
pub use rwlock::rwlock_key;

// Deadline
pub use rwlock::compute_lock_deadline as compute_rwlock_deadline;

// Expiry Checks
pub use rwlock::is_reader_expired;
pub use rwlock::is_writer_expired;

// State Checks
pub use rwlock::count_active_readers;
pub use rwlock::filter_expired_readers;
pub use rwlock::has_read_lock;
pub use rwlock::has_write_lock;

// Acquisition Logic
pub use rwlock::can_acquire_read;
pub use rwlock::can_acquire_write;

// Token and Mode
pub use rwlock::compute_mode_after_cleanup;
pub use rwlock::compute_next_write_token;

// ============================================================================
// Re-exports: Worker
// ============================================================================

// Key Prefixes
pub use worker::STEAL_HINT_PREFIX;
pub use worker::WORKER_GROUP_PREFIX;
pub use worker::WORKER_STATS_PREFIX;

// Key Generation
pub use worker::steal_hint_key;
pub use worker::steal_hint_prefix;
pub use worker::worker_group_key;
pub use worker::worker_stats_key;

// Capacity
pub use worker::calculate_available_capacity;
pub use worker::can_handle_job;

// Liveness
pub use worker::is_worker_alive;

// Work Stealing
pub use worker::compute_steal_batch_size;
pub use worker::compute_steal_hint_deadline;
pub use worker::is_steal_hint_expired;
pub use worker::is_steal_source;
pub use worker::is_steal_target;
pub use worker::steal_hint_remaining_ttl;

// Filtering
pub use worker::matches_capability_filter;
pub use worker::matches_load_filter;
pub use worker::matches_node_filter;
pub use worker::matches_tags_filter;

// Hashing
pub use worker::simple_hash;

// ============================================================================
// Re-exports: Registry
// ============================================================================

// Key Prefix
pub use registry::SERVICE_PREFIX;

// Key Generation
pub use registry::instance_key;
pub use registry::service_instances_prefix;
pub use registry::services_scan_prefix;

// Instance Expiry
pub use registry::instance_remaining_ttl;
pub use registry::is_instance_expired;

// Discovery
pub use registry::matches_discovery_filter;

// Heartbeat
pub use registry::compute_heartbeat_deadline;
pub use registry::compute_next_instance_token;

// ============================================================================
// Re-exports: Strategies
// ============================================================================

// Load Score
pub use strategies::calculate_load_score;

// Round Robin
pub use strategies::compute_round_robin_selection;

// Hashing
pub use strategies::compute_virtual_node_hash;
pub use strategies::hash_key;
pub use strategies::lookup_hash_ring;

// Work Stealing
pub use strategies::is_worker_idle_for_stealing;

// Work Stealing Heuristics
pub use strategies::compute_affinity_score;
pub use strategies::compute_group_average_load;
pub use strategies::compute_rebalance_pairs;
pub use strategies::is_group_balanced;
pub use strategies::rank_steal_targets;
pub use strategies::should_continue_stealing;
pub use strategies::RankedStealTarget;
pub use strategies::StealStrategy;
pub use strategies::WorkerStealInfo;

// Selection
pub use strategies::select_from_scored;
pub use strategies::SelectionResult;

// Metrics
pub use strategies::compute_running_average;

// Filtering
pub use strategies::worker_matches_tags;

// ============================================================================
// Re-exports: Sequence
// ============================================================================

// Batch State
pub use sequence::batch_remaining;
pub use sequence::should_refill_batch;

// Batch Computation
pub use sequence::compute_batch_end;
pub use sequence::compute_next_after_refill;

// Reservation
pub use sequence::compute_cas_expected;
pub use sequence::compute_new_sequence_value;
pub use sequence::compute_range_start;
pub use sequence::SequenceReservationResult;

// Initial State
pub use sequence::compute_initial_current;
pub use sequence::is_initial_reservation;

// Parsing
pub use sequence::parse_sequence_value;
pub use sequence::ParseSequenceResult;

// ============================================================================
// Re-exports: Counter
// ============================================================================

// Unsigned Operations
pub use counter::apply_decrement;
pub use counter::apply_increment;
pub use counter::CounterOpResult;

// Signed Operations
pub use counter::apply_signed_add;
pub use counter::apply_signed_sub;
pub use counter::SignedCounterOpResult;

// CAS Expected
pub use counter::compute_signed_cas_expected;
pub use counter::compute_unsigned_cas_expected;

// Parsing
pub use counter::parse_signed_counter;
pub use counter::parse_unsigned_counter;
pub use counter::ParseSignedResult;
pub use counter::ParseUnsignedResult;

// Buffered Counter
pub use counter::compute_approximate_total;
pub use counter::should_flush_buffer;

// Retry
pub use counter::compute_retry_delay;

// ============================================================================
// Re-exports: Fencing
// ============================================================================

// Token Validation
pub use fencing::is_token_valid;
pub use fencing::validate_consistent_fencing_tokens;
pub use fencing::FencingValidation;

// Split-Brain Detection
pub use fencing::check_for_split_brain;
pub use fencing::should_step_down;
pub use fencing::SplitBrainCheck;

// Quorum
pub use fencing::compute_quorum_threshold;
pub use fencing::has_quorum;
pub use fencing::partition_maintains_quorum;

// Failover
pub use fencing::compute_election_timeout_with_jitter;
pub use fencing::should_trigger_failover;
pub use fencing::FailoverDecision;

// Lease Validation
pub use fencing::compute_lease_renew_time;
pub use fencing::is_lease_valid;
