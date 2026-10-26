use super::*;

const SERVICE_ID: &str = "service:orchestrator";
const PAYLOAD_REF: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const STATE_REF: &str = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn stream() -> CommittedChangeStream {
    new_stream(SERVICE_ID, 1).expect("stream")
}

fn commit_input(payload_ref: &str) -> CommitInput {
    CommitInput {
        payload_ref: payload_ref.to_string(),
        state_ref: STATE_REF.to_string(),
        committed_at_tick: 7,
        event_count: 1,
    }
}

// r[verify molten.observations.stream]
// r[verify molten.observations.cursors]
// r[verify molten.observations.tests]
#[test]
fn a_single_owner_commit_advances_the_state_the_event_and_the_position_together() {
    let mut stream = stream();
    let committed = commit_transition(&mut stream, &commit_input(PAYLOAD_REF)).expect("commit");
    assert_eq!(committed.len(), 1);
    assert_eq!(stream.revision, 1);
    assert_eq!(stream.state_ref.as_deref(), Some(STATE_REF));
    assert_eq!(stream.events.len(), 1);
    assert_eq!(stream.events[0].revision, 1);
    assert_eq!(stream.events[0].ordinal, 0);
    assert!(stream.events[0].event_ref.starts_with("blake3:"));
    assert_eq!(stream.events[0].payload_ref, PAYLOAD_REF);
}

// r[verify molten.observations.stream]
// r[verify molten.observations.cursors]
// r[verify molten.observations.tests]
#[test]
fn a_resumed_subscriber_receives_every_event_after_its_cursor() {
    let mut stream = stream();
    commit_transition(&mut stream, &commit_input(PAYLOAD_REF)).expect("first commit");
    let cursor = StreamCursor::after(&stream.events[0]);
    commit_transition(&mut stream, &commit_input(PAYLOAD_REF)).expect("second commit");
    commit_transition(&mut stream, &commit_input(PAYLOAD_REF)).expect("third commit");

    let outcome = subscribe(&stream, &cursor, 16).expect("subscription");
    let SubscriptionOutcome::Events { events, next_cursor } = outcome else {
        panic!("the subscriber resumes from its cursor");
    };
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].revision, 2);
    assert_eq!(events[1].revision, 3);
    assert_eq!(next_cursor.revision, 3);
}

// r[verify molten.observations.stream]
// r[verify molten.observations.cursors]
// r[verify molten.observations.tests]
#[test]
fn a_snapshot_and_its_watermark_miss_no_committed_change() {
    let mut stream = stream();
    commit_transition(&mut stream, &commit_input(PAYLOAD_REF)).expect("first commit");
    let snapshot = snapshot_with_watermark(&stream);
    assert_eq!(snapshot.state_ref.as_deref(), Some(STATE_REF));
    assert_eq!(snapshot.revision, 1);
    commit_transition(&mut stream, &commit_input(PAYLOAD_REF)).expect("second commit");

    let outcome = subscribe(&stream, &snapshot.watermark, 16).expect("subscription");
    let SubscriptionOutcome::Events { events, .. } = outcome else {
        panic!("the watermark resumes the stream");
    };
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].revision, 2);
}

// r[verify molten.observations.stream]
// r[verify molten.observations.cursors]
// r[verify molten.observations.tests]
#[test]
fn a_cursor_behind_the_retained_window_reports_an_explicit_gap() {
    let mut stream = stream();
    for _ in 0..(MAX_RETAINED_STREAM_EVENTS + 8) {
        commit_transition(&mut stream, &commit_input(PAYLOAD_REF)).expect("commit");
    }
    let stale = StreamCursor::start(SERVICE_ID, 1);
    let outcome = subscribe(&stream, &stale, 16).expect("subscription");
    assert!(
        matches!(outcome, SubscriptionOutcome::RetentionGap { .. }),
        "a cursor behind the retained window reports a gap"
    );
}

// r[verify molten.observations.stream]
// r[verify molten.observations.cursors]
// r[verify molten.observations.tests]
#[test]
fn a_cursor_from_a_prior_epoch_is_rejected_and_a_foreign_service_is_refused() {
    let stream = stream();
    let stale_epoch = StreamCursor {
        service_id: SERVICE_ID.to_string(),
        stream_epoch: 0,
        revision: 0,
        ordinal: 0,
    };
    let outcome = subscribe(&stream, &stale_epoch, 16).expect("subscription");
    assert_eq!(outcome, SubscriptionOutcome::StaleEpoch { current_epoch: 1 });

    let foreign = StreamCursor::start("service:other", 1);
    let outcome = subscribe(&stream, &foreign, 16).expect("subscription");
    assert_eq!(outcome, SubscriptionOutcome::ForeignService {
        service_id: "service:other".to_string(),
    });
}

// r[verify molten.observations.authorization]
// r[verify molten.observations.tests]
#[test]
fn a_replayed_event_keeps_one_identity_and_deduplicates() {
    let mut stream = stream();
    let first = commit_transition(&mut stream, &commit_input(PAYLOAD_REF)).expect("commit");
    let mut replay = stream.clone();
    replay.revision = 0;
    let repeated = commit_transition(&mut replay, &commit_input(PAYLOAD_REF)).expect("replayed commit");
    assert_eq!(first[0].event_ref, repeated[0].event_ref, "a replayed event keeps one identity");

    let mut sequence = first.clone();
    sequence.extend(repeated.clone());
    let outcome = deduplicate_events(&sequence);
    assert_eq!(outcome.events.len(), 1);
    assert_eq!(outcome.duplicate_refs, vec![repeated[0].event_ref.clone()]);
}

// r[verify molten.observations.stream]
// r[verify molten.observations.cursors]
// r[verify molten.observations.tests]
#[test]
fn an_invalid_commit_is_refused_before_the_stream_moves() {
    let mut stream = stream();
    let mut invalid = commit_input(PAYLOAD_REF);
    invalid.payload_ref = "not-a-content-ref".to_string();
    assert_eq!(
        commit_transition(&mut stream, &invalid).expect_err("an invalid payload ref is refused"),
        CommittedStreamIssue::InvalidPayloadRef
    );
    assert_eq!(stream.revision, 0);
    assert!(stream.events.is_empty());
    assert!(stream.state_ref.is_none());

    let mut empty = commit_input(PAYLOAD_REF);
    empty.event_count = 0;
    assert_eq!(
        commit_transition(&mut stream, &empty).expect_err("an empty transition is refused"),
        CommittedStreamIssue::InvalidEventCount
    );
    assert_eq!(stream.revision, 0);
}

// r[verify molten.observations.stream]
// r[verify molten.observations.cursors]
// r[verify molten.observations.tests]
#[test]
fn a_subscription_is_bounded_and_a_zero_bound_is_refused() {
    let mut stream = stream();
    for _ in 0..4 {
        commit_transition(&mut stream, &commit_input(PAYLOAD_REF)).expect("commit");
    }
    let cursor = StreamCursor::start(SERVICE_ID, 1);
    let outcome = subscribe(&stream, &cursor, 2).expect("bounded subscription");
    let SubscriptionOutcome::Events { events, next_cursor } = outcome else {
        panic!("the subscription returns the bounded events");
    };
    assert_eq!(events.len(), 2);
    assert_eq!(next_cursor.revision, 2);
    assert_eq!(
        subscribe(&stream, &cursor, 0).expect_err("a zero buffer bound is refused"),
        CommittedStreamIssue::InvalidBufferBound
    );
}

// r[verify molten.observations.cursors]
// r[verify molten.observations.stream]
#[test]
fn bounded_pages_do_not_skip_events_within_one_committed_revision() {
    let mut stream = stream();
    let mut input = commit_input(PAYLOAD_REF);
    input.event_count = 3;
    let committed = commit_transition(&mut stream, &input).expect("multi-event commit");
    let mut cursor = StreamCursor::start(SERVICE_ID, 1);
    for expected in committed {
        let SubscriptionOutcome::Events { events, next_cursor } =
            subscribe(&stream, &cursor, 1).expect("bounded subscription")
        else {
            panic!("retained events must remain resumable");
        };
        assert_eq!(events, vec![expected]);
        cursor = next_cursor;
    }
    assert_eq!(subscribe(&stream, &cursor, 1).expect("completed subscription"), SubscriptionOutcome::Events {
        events: Vec::new(),
        next_cursor: cursor,
    });
}
