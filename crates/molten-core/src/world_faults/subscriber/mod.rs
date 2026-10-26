mod exercise;

pub use exercise::exercise_committed_subscriber_case;

use super::*;
use crate::committed_stream::committed_change_identity;

pub(super) const SUBSCRIBER_PUBLISHED_REVISION: u64 = 2;
pub(super) const SUBSCRIBER_PUBLISHED_TICK: u64 = 2;

pub fn validate_committed_subscriber_case(case: &CommittedSubscriberCase) -> Vec<WorldFaultIssue> {
    let mut issues = Vec::new();
    for (field, is_invalid) in [
        ("case-id", case.case_id.is_empty()),
        ("phase", case.phase != FaultPhase::BeforeResponse),
        ("service-id", case.service_id.is_empty()),
        ("stream-epoch", case.stream_epoch == 0),
        ("event-count", !(2..=64).contains(&case.event_count)),
        ("buffer-bound", case.buffer_bound == 0 || case.buffer_bound >= case.event_count),
    ] {
        if is_invalid {
            issues.push(WorldFaultIssue::SubscriberInvalidCase(field));
        }
    }
    issues
}

pub fn evaluate_committed_subscriber(
    case: &CommittedSubscriberCase,
    observation: Option<CommittedSubscriberObservation>,
) -> CommittedSubscriberResult {
    let diagnostics = subscriber_diagnostics(case, observation.as_ref());
    CommittedSubscriberResult {
        observation,
        disposition: if diagnostics.is_empty() {
            ConformanceDisposition::Passed
        } else {
            ConformanceDisposition::Failed
        },
        diagnostics,
    }
}

// Borrowed revalidation keeps oversized untrusted evidence from being cloned by receipt validation.
pub(super) fn subscriber_diagnostics(
    case: &CommittedSubscriberCase,
    observation: Option<&CommittedSubscriberObservation>,
) -> Vec<WorldFaultIssue> {
    let mut issues = validate_committed_subscriber_case(case);
    if !issues.is_empty() {
        return issues;
    }
    let Some(observation) = observation else {
        issues.push(WorldFaultIssue::ObservationMissing(case.case_id.clone()));
        return issues;
    };
    let Ok(event_count) = usize::try_from(case.event_count) else {
        issues.push(WorldFaultIssue::SubscriberInvalidCase("event-count"));
        return issues;
    };
    let Ok(buffer_bound) = usize::try_from(case.buffer_bound) else {
        issues.push(WorldFaultIssue::SubscriberInvalidCase("buffer-bound"));
        return issues;
    };
    for (field, is_invalid) in [
        ("published-bound", observation.published.len() != event_count),
        ("pre-restart-bound", observation.delivered_before_restart.len() != buffer_bound),
        ("post-restart-bound", observation.delivered_after_restart.len() != event_count),
        ("deduplicated-bound", observation.deduplicated.len() != event_count),
    ] {
        if is_invalid {
            issues.push(WorldFaultIssue::SubscriberObservationMismatch(field));
        }
    }
    if !issues.is_empty() {
        return issues;
    }
    validate_bindings(case, observation, &mut issues);
    validate_published(case, observation, &mut issues);
    if observation.delivered_before_restart != observation.published[..buffer_bound] {
        issues.push(WorldFaultIssue::SubscriberObservationMismatch("pre-restart-prefix"));
    }
    if observation.delivered_after_restart != observation.published {
        issues.push(WorldFaultIssue::SubscriberObservationMismatch("post-restart-sequence"));
    }
    // With an exact replayed prefix and complete resumed sequence, the unique sequence must be
    // published.
    if observation.deduplicated != observation.published {
        issues.push(WorldFaultIssue::SubscriberObservationMismatch("deduplicated-sequence"));
    }
    issues
}

fn validate_bindings(
    case: &CommittedSubscriberCase,
    observation: &CommittedSubscriberObservation,
    issues: &mut Vec<WorldFaultIssue>,
) {
    if observation.case_id != case.case_id || observation.phase != case.phase {
        issues.push(WorldFaultIssue::SubscriberObservationMismatch("case-binding"));
    }
    if observation.restart_count != 1 {
        issues.push(WorldFaultIssue::SubscriberObservationMismatch("restart-count"));
    }
    let checkpoint = &observation.checkpoint_before;
    if checkpoint.service_id != case.service_id
        || checkpoint.stream_epoch != case.stream_epoch
        || checkpoint.revision != 1
        || checkpoint.ordinal != 1
    {
        issues.push(WorldFaultIssue::SubscriberObservationMismatch("checkpoint-binding"));
    }
    if observation.checkpoint_after_restart != observation.checkpoint_before {
        issues.push(WorldFaultIssue::SubscriberObservationMismatch("acknowledged-checkpoint"));
    }
}

fn validate_published(
    case: &CommittedSubscriberCase,
    observation: &CommittedSubscriberObservation,
    issues: &mut Vec<WorldFaultIssue>,
) {
    let payload_ref = reference("committed-subscriber:published-payload");
    for (event, ordinal) in observation.published.iter().zip(0..case.event_count) {
        if event.service_id != case.service_id
            || event.stream_epoch != case.stream_epoch
            || event.revision != SUBSCRIBER_PUBLISHED_REVISION
            || event.ordinal != ordinal
            || event.payload_ref != payload_ref
            || event.committed_at_tick != SUBSCRIBER_PUBLISHED_TICK
            || event.event_ref != committed_change_identity(event)
        {
            issues.push(WorldFaultIssue::SubscriberObservationMismatch("published-event"));
            break;
        }
    }
}

#[cfg(test)]
mod tests;
