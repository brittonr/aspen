use super::*;
use crate::committed_stream::CommitInput;
use crate::committed_stream::CommittedChangeEvent;
use crate::committed_stream::CommittedChangeStream;
use crate::committed_stream::StreamCursor;
use crate::committed_stream::SubscriptionOutcome;
use crate::committed_stream::commit_transition;
use crate::committed_stream::deduplicate_events;
use crate::committed_stream::new_stream;
use crate::committed_stream::subscribe;

/// Model subscriber restart in process, retaining the stream and only the acknowledged cursor.
/// This does not exercise process crashes, storage durability, or physical power loss.
pub fn exercise_committed_subscriber_case(
    case: &CommittedSubscriberCase,
) -> Result<CommittedSubscriberObservation, Vec<WorldFaultIssue>> {
    let issues = validate_committed_subscriber_case(case);
    if !issues.is_empty() {
        return Err(issues);
    }
    let mut stream = new_stream(&case.service_id, case.stream_epoch).map_err(|_| failure("new-stream"))?;
    let baseline = commit_transition(&mut stream, &CommitInput {
        payload_ref: reference("committed-subscriber:baseline-payload"),
        state_ref: reference("committed-subscriber:baseline-state"),
        committed_at_tick: 1,
        event_count: 1,
    })
    .map_err(|_| failure("baseline-commit"))?;
    let checkpoint_before = baseline.first().map(StreamCursor::after).ok_or_else(|| failure("baseline-event"))?;
    let published = commit_transition(&mut stream, &CommitInput {
        payload_ref: reference("committed-subscriber:published-payload"),
        state_ref: reference("committed-subscriber:published-state"),
        committed_at_tick: SUBSCRIBER_PUBLISHED_TICK,
        event_count: case.event_count,
    })
    .map_err(|_| failure("publication-commit"))?;
    // The page helper drops its volatile next cursor; receiving the page does not acknowledge it.
    let delivered_before_restart = unacknowledged_page(&stream, &checkpoint_before, case.buffer_bound)?;
    let checkpoint_after_restart = checkpoint_before.clone();
    let delivered_after_restart = resumed_pages(&stream, &checkpoint_after_restart, case)?;
    let mut combined = Vec::with_capacity(delivered_before_restart.len() + delivered_after_restart.len());
    combined.extend_from_slice(&delivered_before_restart);
    combined.extend_from_slice(&delivered_after_restart);
    let deduplicated = deduplicate_events(&combined).events;
    Ok(CommittedSubscriberObservation {
        case_id: case.case_id.clone(),
        phase: case.phase,
        checkpoint_before,
        checkpoint_after_restart,
        published,
        delivered_before_restart,
        delivered_after_restart,
        deduplicated,
        restart_count: 1,
    })
}

fn unacknowledged_page(
    stream: &CommittedChangeStream,
    checkpoint: &StreamCursor,
    buffer_bound: u32,
) -> Result<Vec<CommittedChangeEvent>, Vec<WorldFaultIssue>> {
    match subscribe(stream, checkpoint, buffer_bound).map_err(|_| failure("pre-restart-subscription"))? {
        SubscriptionOutcome::Events { events, .. } => Ok(events),
        SubscriptionOutcome::RetentionGap { .. }
        | SubscriptionOutcome::StaleEpoch { .. }
        | SubscriptionOutcome::ForeignService { .. } => Err(failure("pre-restart-subscription-outcome")),
    }
}

fn resumed_pages(
    stream: &CommittedChangeStream,
    checkpoint: &StreamCursor,
    case: &CommittedSubscriberCase,
) -> Result<Vec<CommittedChangeEvent>, Vec<WorldFaultIssue>> {
    let Ok(event_count) = usize::try_from(case.event_count) else {
        return Err(vec![WorldFaultIssue::SubscriberInvalidCase("event-count")]);
    };
    let Ok(buffer_bound) = usize::try_from(case.buffer_bound) else {
        return Err(vec![WorldFaultIssue::SubscriberInvalidCase("buffer-bound")]);
    };
    let mut acknowledged = checkpoint.clone();
    let mut delivered = Vec::with_capacity(event_count);
    for _ in 0..case.event_count {
        let outcome =
            subscribe(stream, &acknowledged, case.buffer_bound).map_err(|_| failure("post-restart-subscription"))?;
        let SubscriptionOutcome::Events { events, next_cursor } = outcome else {
            return Err(failure("post-restart-subscription-outcome"));
        };
        if events.is_empty() {
            break;
        }
        if events.len() > buffer_bound || delivered.len() + events.len() > event_count {
            return Err(failure("post-restart-page-bound"));
        }
        delivered.extend(events);
        // Acknowledge only after accepting the resumed page, never its pre-restart counterpart.
        acknowledged = next_cursor;
        if delivered.len() == event_count {
            break;
        }
    }
    Ok(delivered)
}

fn failure(field: &'static str) -> Vec<WorldFaultIssue> {
    vec![WorldFaultIssue::SubscriberObservationMismatch(field)]
}
