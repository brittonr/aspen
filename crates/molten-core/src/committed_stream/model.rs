//! The committed-change stream: durable, resumable observation of committed
//! service transitions.
//!
//! The stream records committed transitions only. It never records a
//! speculative callback, and it never implies an atomicity across independent
//! stores. Where one storage owner controls the state and the event, the owner
//! commits the state, the event, and the stream position together.
//!
//! A cursor identifies a position by service identity, stream epoch, revision,
//! and event ordinal. An epoch change invalidates prior cursors explicitly, a
//! retention gap is explicit, and a snapshot plus its watermark cannot miss a
//! committed change.

/// The identity of the service whose committed transitions the stream records.
pub type StreamServiceId = String;

/// The stable identity of a committed change event.
pub type CommittedChangeRef = String;

/// One committed service transition.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CommittedChangeEvent {
    /// The stable identity of the event. It deduplicates a replayed event.
    pub event_ref: CommittedChangeRef,
    /// The service whose transition the event records.
    pub service_id: StreamServiceId,
    /// The stream epoch the event belongs to.
    pub stream_epoch: u64,
    /// The revision the event belongs to.
    pub revision: u64,
    /// The ordinal of the event inside its revision.
    pub ordinal: u32,
    /// The content identity of the committed payload. The payload is referenced
    /// by identity, never inlined.
    pub payload_ref: String,
    /// The logical tick the transition committed at.
    pub committed_at_tick: u64,
}

/// A position in the stream.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct StreamCursor {
    pub service_id: StreamServiceId,
    pub stream_epoch: u64,
    pub revision: u64,
    /// The inclusive lower bound for the next event within this revision.
    pub ordinal: u32,
}

impl StreamCursor {
    /// The cursor that precedes every event of the stream.
    pub fn start(service_id: &str, stream_epoch: u64) -> Self {
        Self {
            service_id: service_id.to_string(),
            stream_epoch,
            revision: 0,
            ordinal: 0,
        }
    }

    /// The cursor that follows the given event.
    pub fn after(event: &CommittedChangeEvent) -> Self {
        Self {
            service_id: event.service_id.clone(),
            stream_epoch: event.stream_epoch,
            revision: event.revision,
            ordinal: event.ordinal.saturating_add(1),
        }
    }
}

/// The retained events and the position of a stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedChangeStream {
    pub service_id: StreamServiceId,
    pub stream_epoch: u64,
    pub revision: u64,
    /// The retained events, in commit order.
    pub events: Vec<CommittedChangeEvent>,
    /// The oldest revision the stream still retains.
    pub first_retained_revision: u64,
    /// The state reference the single owner advanced with the last commit.
    pub state_ref: Option<String>,
}

/// The input of one committed transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitInput {
    /// The payload the transition committed, referenced by content identity.
    pub payload_ref: String,
    /// The state reference the transition produced.
    pub state_ref: String,
    /// The logical tick the transition committed at.
    pub committed_at_tick: u64,
    /// The number of events the transition commits in its revision.
    pub event_count: u32,
}

/// The outcome of a subscription request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscriptionOutcome {
    /// The events after the cursor, up to the buffer bound.
    Events {
        events: Vec<CommittedChangeEvent>,
        next_cursor: StreamCursor,
    },
    /// The cursor is behind the retained window, so the subscriber must take a
    /// fresh snapshot.
    RetentionGap { resnapshot_from: StreamCursor },
    /// The cursor belongs to a prior stream epoch.
    StaleEpoch { current_epoch: u64 },
    /// The cursor names another service.
    ForeignService { service_id: StreamServiceId },
}

/// A snapshot and the watermark that starts the stream from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotWithWatermark {
    /// The state reference the snapshot observed.
    pub state_ref: Option<String>,
    /// The position the subscriber starts from. Every change committed after
    /// the snapshot follows it.
    pub watermark: StreamCursor,
    /// The revision the snapshot observed.
    pub revision: u64,
}

/// The outcome of a deduplication over replayed events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeduplicationOutcome {
    /// The events in commit order, without the repeated identities.
    pub events: Vec<CommittedChangeEvent>,
    /// The identities that were dropped as repeats.
    pub duplicate_refs: Vec<CommittedChangeRef>,
}
