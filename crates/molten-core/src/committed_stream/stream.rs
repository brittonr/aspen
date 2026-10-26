//! The committed-change stream operations.

use super::*;

/// The bound on the events a stream retains.
pub const MAX_RETAINED_STREAM_EVENTS: usize = 4_096;
/// The bound on the events one subscription returns.
pub const MAX_SUBSCRIPTION_EVENTS: usize = 1_024;

// r[impl molten.observations.stream]
/// A new stream for one service.
pub fn new_stream(service_id: &str, stream_epoch: u64) -> Result<CommittedChangeStream, CommittedStreamIssue> {
    if service_id.is_empty() {
        return Err(CommittedStreamIssue::MissingServiceId);
    }
    if stream_epoch == 0 {
        return Err(CommittedStreamIssue::MissingStreamEpoch);
    }
    Ok(CommittedChangeStream {
        service_id: service_id.to_string(),
        stream_epoch,
        revision: 0,
        events: Vec::new(),
        first_retained_revision: 0,
        state_ref: None,
    })
}

/// Commit one service transition.
///
/// The state reference, the events, and the stream position advance together:
/// the caller observes the new state and the new position together, or neither.
/// A transition that commits several events commits them in one revision with
/// increasing ordinals.
pub fn commit_transition(
    stream: &mut CommittedChangeStream,
    input: &CommitInput,
) -> Result<Vec<CommittedChangeEvent>, CommittedStreamIssue> {
    if input.payload_ref.is_empty() || !input.payload_ref.starts_with("blake3:") {
        return Err(CommittedStreamIssue::InvalidPayloadRef);
    }
    if input.state_ref.is_empty() || !input.state_ref.starts_with("blake3:") {
        return Err(CommittedStreamIssue::InvalidStateRef);
    }
    if input.event_count == 0 {
        return Err(CommittedStreamIssue::InvalidEventCount);
    }
    let revision = stream.revision.checked_add(1).ok_or(CommittedStreamIssue::RevisionOverflow)?;
    let event_count = usize::try_from(input.event_count).map_err(|_| CommittedStreamIssue::InvalidEventCount)?;
    let mut committed = Vec::with_capacity(event_count);
    for ordinal in 0..input.event_count {
        let event = CommittedChangeEvent {
            event_ref: String::new(),
            service_id: stream.service_id.clone(),
            stream_epoch: stream.stream_epoch,
            revision,
            ordinal,
            payload_ref: input.payload_ref.clone(),
            committed_at_tick: input.committed_at_tick,
        };
        let event_ref = committed_change_identity(&event);
        committed.push(CommittedChangeEvent { event_ref, ..event });
    }
    stream.events.extend(committed.iter().cloned());
    stream.revision = revision;
    stream.state_ref = Some(input.state_ref.clone());
    if stream.events.len() > MAX_RETAINED_STREAM_EVENTS {
        let overflow = stream.events.len() - MAX_RETAINED_STREAM_EVENTS;
        stream.events.drain(0..overflow);
        stream.first_retained_revision = stream.events.first().map(|event| event.revision).unwrap_or(revision);
    }
    Ok(committed)
}

// r[impl molten.observations.cursors]
/// Subscribe from the inclusive next-event cursor.
///
/// The subscription is bounded. A cursor behind the retained window produces an
/// explicit retention gap, a cursor from a prior epoch is rejected, and a cursor
/// for another service is refused.
pub fn subscribe(
    stream: &CommittedChangeStream,
    cursor: &StreamCursor,
    buffer_bound: u32,
) -> Result<SubscriptionOutcome, CommittedStreamIssue> {
    let buffer_bound = usize::try_from(buffer_bound).map_err(|_| CommittedStreamIssue::InvalidBufferBound)?;
    if buffer_bound == 0 || buffer_bound > MAX_SUBSCRIPTION_EVENTS {
        return Err(CommittedStreamIssue::InvalidBufferBound);
    }
    if cursor.service_id != stream.service_id {
        return Ok(SubscriptionOutcome::ForeignService {
            service_id: cursor.service_id.clone(),
        });
    }
    if cursor.stream_epoch != stream.stream_epoch {
        return Ok(SubscriptionOutcome::StaleEpoch {
            current_epoch: stream.stream_epoch,
        });
    }
    if cursor.revision < stream.first_retained_revision {
        return Ok(SubscriptionOutcome::RetentionGap {
            resnapshot_from: StreamCursor::start(&stream.service_id, stream.stream_epoch),
        });
    }
    let mut events = Vec::with_capacity(buffer_bound);
    for event in &stream.events {
        let is_after_cursor =
            event.revision > cursor.revision || (event.revision == cursor.revision && event.ordinal >= cursor.ordinal);
        if is_after_cursor {
            if events.len() == buffer_bound {
                break;
            }
            events.push(event.clone());
        }
    }
    let next_cursor = events.last().map(StreamCursor::after).unwrap_or_else(|| cursor.clone());
    Ok(SubscriptionOutcome::Events { events, next_cursor })
}

// r[impl molten.observations.cursors]
/// Take a snapshot and the watermark that starts the stream from it.
///
/// The watermark is the position after the last committed event, so the
/// combined view of the snapshot and the events from the watermark onward
/// cannot miss a committed change.
pub fn snapshot_with_watermark(stream: &CommittedChangeStream) -> SnapshotWithWatermark {
    let watermark = stream
        .events
        .last()
        .map(StreamCursor::after)
        .unwrap_or_else(|| StreamCursor::start(&stream.service_id, stream.stream_epoch));
    SnapshotWithWatermark {
        state_ref: stream.state_ref.clone(),
        watermark,
        revision: stream.revision,
    }
}
