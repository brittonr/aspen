//! Verified pure functions for snapshot management.
//!
//! These functions are deterministic, have no I/O, and encode decision logic
//! that can be formally verified with Verus. See `verus/` for specs.

pub mod snapshot;

pub use snapshot::compute_adaptive_fork_count;
pub use snapshot::should_allow_restore;
pub use snapshot::should_invalidate_snapshot;
