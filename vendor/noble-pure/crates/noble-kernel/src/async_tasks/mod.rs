//! Serialized async ownership decisions, independent of native storage and authority.
//!
//! The shell owns all payloads, validates arguments and native provenance, and
//! serializes this table with resource transfer and one-shot witness consumption.
//! `prepare` reserves nothing and starts nothing; its exclusive borrow makes a
//! subsequent `commit` infallible. Dropping that preparation preserves the table.
//! Committed inputs belong to the task even though their storage stays in the shell.
//!
//! Cancellation acknowledges revoked guest access, not native stop or external
//! failure. Native terminal observation, pin settlement, cleanup and finalization
//! are separate decisions. Keep the table and task storage until `finish` succeeds.
//! Descriptive handles, callbacks, snapshots and decisions grant no authority and
//! cannot establish native authenticity or receipt truth.

mod accounting;
mod bounds;
mod domain;
mod schema;
mod table;
mod transition;

pub use bounds::{Footprint, Limits, Request, MAX_BYTES, MAX_OBLIGATIONS, MAX_SLOTS, MAX_WAKEUPS};
pub use domain::{
    Accounting, Action, Callback, Completion, Decision, Disposition, Error, Event, Failure, Handle,
    NativeId, Obligations, Outcome, Snapshot, State, TableId,
};
pub use schema::{
    classify, validate_coverage, CoverageError, CoverageRow, EventKind, Rule, EVENT_CONSTRUCTORS,
    EVENT_SCHEMA, STATE_CONSTRUCTORS, STATE_SCHEMA,
};
pub use table::{Admission, Table};
pub use transition::transition;

/// Component/invocation identity is shared with the resource ownership table.
pub use crate::resources::Context;

/// Opaque move-only result/cancellation capability. Dropping it does not clean up.
#[derive(Debug)]
pub struct Task {
    handle: Handle,
}

impl Task {
    /// Copying identity does not copy the ability to receive a task's result.
    pub const fn handle(&self) -> Handle {
        self.handle
    }
}

/// Failed admission/delivery/cancellation preserves the supplied opaque values.
#[derive(Debug)]
pub struct Rejected<T> {
    pub error: Error,
    pub input: T,
}

/// Task-owned shell storage after admission; no native operation has started.
#[derive(Debug)]
pub struct Admitted<T> {
    pub task: Task,
    pub callback: Callback,
    pub inputs: T,
    pub decision: Decision,
}

/// Counts include invisible retirement and delivered tasks with unsettled debt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Observation {
    pub pending: usize,
    pub ready: usize,
    pub delivered: usize,
    pub retiring: usize,
    pub retired: usize,
    pub outstanding_pins: usize,
    pub queued_wakeups: usize,
    pub reserved: Footprint,
}
