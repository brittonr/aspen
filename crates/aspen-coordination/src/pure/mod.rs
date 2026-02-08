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
pub mod election;
pub mod lock;
pub mod queue;
pub mod rate_limiter;
pub mod registry;
pub mod rwlock;
pub mod semaphore;
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

// Receipt Handle
pub use queue::parse_receipt_handle;

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

// ============================================================================
// Re-exports: Election
// ============================================================================

// State Transitions
pub use election::compute_next_leadership_state;

// ============================================================================
// Re-exports: Semaphore
// ============================================================================

// Holder Expiry
pub use semaphore::is_holder_expired;

// Permit Calculations
pub use semaphore::calculate_available_permits;
pub use semaphore::can_acquire_permits;
pub use semaphore::compute_holder_deadline;

// ============================================================================
// Re-exports: RWLock
// ============================================================================

// Expiry Checks
pub use rwlock::is_reader_expired;
pub use rwlock::is_writer_expired;

// Acquisition Logic
pub use rwlock::can_acquire_read;
pub use rwlock::can_acquire_write;

// Token and Mode
pub use rwlock::compute_mode_after_cleanup;
pub use rwlock::compute_next_write_token;

// ============================================================================
// Re-exports: Worker
// ============================================================================

// Capacity
pub use worker::calculate_available_capacity;
pub use worker::can_handle_job;

// Liveness
pub use worker::is_worker_alive;

// Work Stealing
pub use worker::is_steal_hint_expired;
pub use worker::is_steal_source;
pub use worker::is_steal_target;
pub use worker::steal_hint_remaining_ttl;

// Hashing
pub use worker::simple_hash;

// ============================================================================
// Re-exports: Registry
// ============================================================================

// Instance Expiry
pub use registry::instance_remaining_ttl;
pub use registry::is_instance_expired;

// Discovery
pub use registry::matches_discovery_filter;

// Heartbeat
pub use registry::compute_heartbeat_deadline;
pub use registry::compute_next_instance_token;
