//! Verified pure functions for cluster coordination primitives.
//!
//! This module contains the production implementations of pure business logic
//! for cluster coordination. All functions are:
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
//! # Module Organization
//!
//! - [`gossip`]: Topic ID derivation, backoff calculation for gossip subsystem
//! - [`rate_limiter`]: Token bucket calculations, replenishment, eviction
//! - [`peer`]: Peer address parsing and validation
//! - [`message`]: Message size and version validation
//!
//! # Tiger Style
//!
//! All functions follow Tiger Style principles:
//! - Saturating/checked arithmetic (no panics)
//! - Explicit types (u64, u32, not usize)
//! - Functions under 70 lines
//! - Fixed limits on all data structures

pub mod gossip;
pub mod message;
pub mod peer;
pub mod rate_limiter;

// ============================================================================
// Re-exports: Gossip
// ============================================================================

pub use gossip::calculate_backoff_duration_ms;
pub use gossip::derive_topic_id_bytes;
// ============================================================================
// Re-exports: Message
// ============================================================================
pub use message::is_message_size_valid;
pub use message::is_version_compatible;
// ============================================================================
// Re-exports: Peer
// ============================================================================
pub use peer::PeerSpecParseResult;
pub use peer::is_json_endpoint;
pub use peer::parse_peer_spec;
// ============================================================================
// Re-exports: Rate Limiter
// ============================================================================
pub use rate_limiter::TokenConsumptionResult;
pub use rate_limiter::calculate_replenished_tokens;
pub use rate_limiter::can_consume_token;
pub use rate_limiter::should_evict_oldest;
