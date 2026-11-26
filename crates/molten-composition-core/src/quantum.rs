//! The slice contract of saturating work under the time-sliced execution profile.
//!
//! A saturating callback of the stateful service and one runnable of the independent sibling are
//! both ready under the admitted fabric-time scheduler. Each slice of the callback consumes at most
//! the admitted quantum; on a yield the continuation is re-enqueued, so the sibling is selected
//! between slices. The shell drives the scheduler and the slice ledger.

/// What one slice of saturating work did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaturationSlice {
    /// The callback consumed one quantum and yielded.
    Yielded,
    /// The callback ended: completed, or stopped at its budget.
    Ended { is_within_budget: bool },
}

/// A saturating callback driven one slice at a time.
pub trait SaturatingWork {
    fn run_slice(&mut self) -> SaturationSlice;
}
