//! Hybrid Logical Clock utilities for deterministic distributed ordering.
//!
//! This module re-exports the HLC functionality from `aspen-hlc` for backward compatibility.
//! New code should prefer importing from `aspen_hlc` directly for lighter dependencies.
//!
//! ## Key Features
//!
//! - **Deterministic ordering**: HLC provides total ordering even when wall clocks are equal
//! - **Causal consistency**: Updates from received timestamps maintain happened-before
//!   relationships
//! - **Node ID tiebreaker**: Each node has a unique 16-byte ID derived from its identifier
//!
//! ## Usage
//!
//! ```ignore
//! use aspen_core::hlc::{create_hlc, new_timestamp, HlcTimestamp};
//!
//! // Create an HLC instance for a node
//! let hlc = create_hlc("node-1");
//!
//! // Generate timestamps
//! let ts1 = new_timestamp(&hlc);
//! let ts2 = new_timestamp(&hlc);
//! assert!(ts2 > ts1); // Monotonically increasing
//!
//! // Convert to Unix milliseconds for display/logging
//! let unix_ms = to_unix_ms(&ts1);
//! ```

// Re-export everything from aspen-hlc
pub use aspen_hlc::create_hlc;
pub use aspen_hlc::new_timestamp;
pub use aspen_hlc::to_unix_ms;
pub use aspen_hlc::update_from_timestamp;
pub use aspen_hlc::HlcTimestamp;
pub use aspen_hlc::SerializableTimestamp;
pub use aspen_hlc::HLC;
pub use aspen_hlc::ID;
pub use aspen_hlc::NTP64;
