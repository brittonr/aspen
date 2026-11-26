use super::*;

const OTHER_CAPABILITY: &str = "blake3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const EVENT_RECORD_FIELDS: usize = 9;

fn fixture_input(stream: EntropyStreamRequest, recorded: &[Event]) -> FixtureEntropyInput<'_> {
    FixtureEntropyInput {
        stream,
        draws: &super::fixture::DETERMINISTIC_ENTROPY_DRAWS,
        recorded,
    }
}

// r[verify molten.fabric_time.entropy_capability_evidence]
#[test]
fn event_binds_capability_under_versioned_label() {
    let profile = simulation_profile();
    let (stream, recorded) = super::fixture::deterministic_entropy_events(&profile).expect("fixture entropy");
    let event = &recorded[0];
    let entropy_record = event.value.collect_simple_record("fabric-time-entropy-event-v2", Some(EVENT_RECORD_FIELDS));
    assert!(entropy_record.is_some());
    assert_eq!(entropy_event_capability_ref(event).expect("capability ref"), stream.capability_ref);

    let decision = DeadlineDecision {
        subject_id: "deadline".to_string(),
        generation: GENERATION,
        domain: TimeDomain::Virtual,
        status: DeadlineStatus::Pending,
        observed_ticks: 0,
        target_ticks: TIMER_DEADLINE,
        uncertainty_ticks: 0,
    };
    let deadline = deadline_event(&profile.profile.profile_ref, &decision).expect("deadline event");
    let legacy_record = deadline.value.collect_simple_record("fabric-time-event-v1", Some(EVENT_RECORD_FIELDS));
    assert!(legacy_record.is_some());
}

// r[verify molten.fabric_time.entropy_capability_evidence]
#[test]
fn fixture_entropy_replays_against_its_admitted_stream() {
    let profile = simulation_profile();
    let (stream, recorded) = super::fixture::deterministic_entropy_events(&profile).expect("fixture entropy");
    replay_fixture_entropy(&profile, fixture_input(stream.clone(), &recorded)).expect("matching replay");
    let truncated = &recorded[..1];
    assert!(matches!(
        replay_fixture_entropy(&profile, fixture_input(stream, truncated)),
        Err(EntropyReplayError::Diverged { index: 1 })
    ));
}

// r[verify molten.fabric_time.entropy_capability_evidence]
#[test]
fn mismatched_capability_is_rejected_on_replay_with_the_typed_diagnostic() {
    let profile = simulation_profile();
    let (stream, recorded) = super::fixture::deterministic_entropy_events(&profile).expect("fixture entropy");
    let recorded_capability = stream.capability_ref.clone();
    let mut other_stream = stream;
    other_stream.capability_ref = OTHER_CAPABILITY.to_string();
    assert_ne!(other_stream.capability_ref, recorded_capability);
    let error = replay_fixture_entropy(&profile, fixture_input(other_stream, &recorded)).expect_err("mismatch");
    assert!(matches!(
        error,
        EntropyReplayError::Entropy(EntropyError::CapabilityMismatch { ref expected, ref actual })
            if expected == OTHER_CAPABILITY && *actual == recorded_capability
    ));
}

// r[verify molten.fabric_time.entropy_capability_evidence]
#[test]
fn unbound_or_tampered_evidence_is_rejected() {
    let profile = simulation_profile();
    let (stream, recorded) = super::fixture::deterministic_entropy_events(&profile).expect("fixture entropy");
    let mut tampered = recorded.clone();
    tampered[0].evidence_ref = HASH_A.to_string();
    assert!(matches!(
        replay_fixture_entropy(&profile, fixture_input(stream.clone(), &tampered)),
        Err(EntropyReplayError::Evidence(_))
    ));

    let header = EventHeader {
        profile_ref: &profile.profile.profile_ref,
        kind: EventKind::Entropy,
        generation: GENERATION,
        subject: "simulation-stream",
        action: "deterministic-simulation",
        ticks: ENTROPY_COUNT,
    };
    assert!(named_event(header).is_err());
    let unbound = named_event(EventHeader {
        kind: EventKind::Timer,
        ..header
    })
    .expect("timer event");
    assert!(matches!(
        replay_fixture_entropy(&profile, fixture_input(stream, &[unbound])),
        Err(EntropyReplayError::Evidence(_))
    ));
}
