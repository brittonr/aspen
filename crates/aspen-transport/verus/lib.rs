//! Verus Formal Specifications for Aspen Transport
//!
//! This crate contains standalone Verus specifications for verifying the
//! correctness of connection and stream management in Aspen.
//!
//! # Verification
//!
//! Run Verus verification with:
//! ```bash
//! verus --crate-type=lib crates/aspen-transport/verus/lib.rs
//! ```
//!
//! # Module Overview
//!
//! ## Connection Management
//! - `connection_spec`: Semaphore-based connection limiting
//!
//! ## Stream Management
//! - `stream_spec`: Per-connection stream limiting
//!
//! # Invariants Verified
//!
//! ## Connection Properties
//!
//! 1. **CONN-1: Connection Bounds**: active_connections <= max_connections
//!    - Semaphore enforces upper bound
//!    - Prevents connection exhaustion
//!
//! 2. **CONN-2: Permit RAII**: Dropping permit releases slot
//!    - Automatic resource cleanup
//!    - No resource leaks
//!
//! 3. **CONN-3: Shutdown Semantics**: After shutdown, no new permits
//!    - Graceful degradation
//!
//! ## Stream Properties
//!
//! 4. **STREAM-1: Stream Bounds**: active_streams <= max_streams
//!    - Per-connection stream limiting
//!
//! 5. **STREAM-2: Counter Consistency**: Atomic counter matches semaphore
//!    - active_streams accurately reflects permit count
//!
//! 6. **STREAM-3: Permit Release**: Drop decrements counter
//!    - RAII ensures counter accuracy
//!
//! # Trusted Axioms
//!
//! The specifications assume:
//! - Tokio semaphore correctly enforces permit counts
//! - Atomic operations are sequentially consistent for our use case
//! - RAII Drop is always called on permit release

use vstd::prelude::*;

verus! {
    // Re-export connection specifications
    pub use connection_spec::connection_bounded;
    pub use connection_spec::permit_released_on_drop;
    pub use connection_spec::shutdown_prevents_acquire;

    // Re-export stream specifications
    pub use stream_spec::stream_bounded;
    pub use stream_spec::counter_consistent;
    pub use stream_spec::permit_release_decrements;
}

mod connection_spec;
mod stream_spec;
