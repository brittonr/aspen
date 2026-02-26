//! Pure functions for RPC handler logic.
//!
//! This module contains pure functions extracted from RPC handlers for:
//! - Improved testability (explicit inputs/outputs)
//! - Property-based testing with Bolero
//! - Deterministic behavior verification
//!
//! # Functions
//!
//! - [`normalize_timeout_ms`]: Convert 0 timeout to None (8+ occurrences in handlers)
//! - [`bytes_to_string_lossy`]: Safe UTF-8 conversion (10+ occurrences)
//! - [`is_lock_owner`]: Verify lock ownership (2 occurrences)
//! - [`convert_dlq_reason`]: Convert DLQ reason enum to string
//! - [`convert_dequeued_item`]: Convert coordination item to response type
//! - [`convert_queue_item`]: Convert queue item to response type
//!
//! # Tiger Style
//!
//! - All functions are pure (no I/O, no system calls)
//! - Deterministic: same inputs always produce same outputs
//! - Bounded outputs where applicable

mod conversions;
mod timeout;
mod validation;

pub use conversions::*;
pub use timeout::*;
pub use validation::*;
