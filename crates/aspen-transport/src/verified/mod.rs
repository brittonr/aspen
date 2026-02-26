//! Verified pure functions for transport layer primitives.
//!
//! This module contains the production implementations of pure business logic
//! for transport operations. All functions are:
//!
//! - **Deterministic**: No I/O, no system calls
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
//! # Module Organization
//!
//! - [`encoding`]: Shard prefix encoding/decoding
//! - [`error`]: Error classification functions
//!
//! # Tiger Style
//!
//! All functions follow Tiger Style principles:
//! - Saturating/checked arithmetic (no panics)
//! - Explicit types (u32, u8, not usize)
//! - Functions under 70 lines
//! - Fixed limits on all data structures

pub mod encoding;
pub mod error;

// ============================================================================
// Re-exports: Encoding
// ============================================================================

pub use encoding::SHARD_PREFIX_SIZE;
pub use encoding::encode_shard_prefix;
pub use encoding::try_decode_shard_prefix;
// ============================================================================
// Re-exports: Error
// ============================================================================
pub use error::FatalErrorClassification;
pub use error::classify_fatal_error_kind;
