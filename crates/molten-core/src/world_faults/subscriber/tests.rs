use super::*;
use crate::committed_stream::StreamCursor;

fn case() -> CommittedSubscriberCase {
    standard_world_fault_profile("subscriber-regression").expect("profile").subscriber
}

// r[verify molten.world_faults.interruption]
#[test]
fn publication_before_ack_replays_the_prefix_and_completes_every_bounded_page() {
    for (event_count, buffer_bound) in [(2, 1), (3, 1), (7, 3), (64, 63), (64, 1)] {
        let case = CommittedSubscriberCase {
            event_count,
            buffer_bound,
            ..case()
        };
        let observed = exercise_committed_subscriber_case(&case).expect("real stream exercise");
        assert_eq!(observed.checkpoint_before, observed.checkpoint_after_restart);
        assert_eq!(observed.checkpoint_before.revision, 1);
        assert_eq!(observed.checkpoint_before.ordinal, 1);
        assert_eq!(observed.restart_count, 1);
        let buffer_bound = usize::try_from(buffer_bound).expect("bounded page size fits this platform");
        assert_eq!(observed.delivered_before_restart, observed.published[..buffer_bound]);
        assert_eq!(observed.delivered_after_restart, observed.published);
        assert_eq!(observed.deduplicated, observed.published);
        assert_eq!(
            observed.published.iter().map(|event| (event.revision, event.ordinal)).collect::<Vec<_>>(),
            (0..event_count).map(|ordinal| (2, ordinal)).collect::<Vec<_>>()
        );
        assert_eq!(evaluate_committed_subscriber(&case, Some(observed)).disposition, ConformanceDisposition::Passed);
    }
}

// r[verify molten.world_faults.profile]
#[test]
fn invalid_subscriber_cases_are_denied_before_the_exercise() {
    let mutations: [fn(&mut CommittedSubscriberCase); 8] = [
        |case| case.case_id.clear(),
        |case| case.phase = FaultPhase::AfterDurableWrite,
        |case| case.service_id.clear(),
        |case| case.stream_epoch = 0,
        |case| case.event_count = 1,
        |case| case.event_count = 65,
        |case| case.buffer_bound = 0,
        |case| case.buffer_bound = case.event_count,
    ];
    for mutate in mutations {
        let mut case = case();
        mutate(&mut case);
        assert!(exercise_committed_subscriber_case(&case).is_err());
        let result = evaluate_committed_subscriber(&case, None);
        assert_eq!(result.disposition, ConformanceDisposition::Failed);
        assert!(result.diagnostics.iter().any(|issue| matches!(issue, WorldFaultIssue::SubscriberInvalidCase(_))));
    }
}

// r[verify molten.world_faults.interruption]
#[test]
fn missing_delivery_or_acknowledging_before_restart_cannot_pass() {
    let case = case();
    let missing = evaluate_committed_subscriber(&case, None);
    assert_eq!(missing.disposition, ConformanceDisposition::Failed);
    assert_eq!(missing.diagnostics, vec![WorldFaultIssue::ObservationMissing(case.case_id.clone())]);
    let mutations: [fn(&mut CommittedSubscriberObservation); 17] = [
        |observed| observed.delivered_before_restart.clear(),
        |observed| {
            observed.delivered_after_restart.remove(0);
        },
        |observed| observed.delivered_after_restart.swap(0, 1),
        |observed| observed.checkpoint_after_restart = StreamCursor::after(&observed.delivered_before_restart[0]),
        |observed| {
            observed.checkpoint_before = StreamCursor::after(&observed.delivered_before_restart[0]);
            observed.checkpoint_after_restart = observed.checkpoint_before.clone();
        },
        |observed| observed.checkpoint_after_restart.stream_epoch += 1,
        |observed| observed.checkpoint_before.service_id = "service:foreign".to_string(),
        |observed| observed.restart_count = 0,
        |observed| observed.restart_count = 2,
        |observed| observed.case_id = "other-case".to_string(),
        |observed| observed.phase = FaultPhase::LostResponse,
        |observed| {
            observed.deduplicated.pop();
        },
        |observed| observed.delivered_before_restart[0] = observed.published[1].clone(),
        |observed| observed.deduplicated.swap(0, 1),
        |observed| observed.published.resize(65, observed.published[0].clone()),
        |observed| observed.delivered_after_restart.push(observed.published[0].clone()),
        |observed| observed.delivered_before_restart.push(observed.published[1].clone()),
    ];
    for mutate in mutations {
        let mut observed = exercise_committed_subscriber_case(&case).expect("exercise");
        mutate(&mut observed);
        let result = evaluate_committed_subscriber(&case, Some(observed));
        assert_eq!(result.disposition, ConformanceDisposition::Failed);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|issue| matches!(issue, WorldFaultIssue::SubscriberObservationMismatch(_)))
        );
    }
}

// r[verify molten.world_faults.receipt]
#[test]
fn self_consistent_but_foreign_or_forged_publication_is_rejected() {
    let case = case();
    let mutations: [fn(&mut crate::committed_stream::CommittedChangeEvent); 7] = [
        |event| event.service_id = "service:foreign".to_string(),
        |event| event.stream_epoch += 1,
        |event| event.revision += 1,
        |event| event.ordinal += 1,
        |event| event.payload_ref = reference("foreign-payload"),
        |event| event.committed_at_tick += 1,
        |event| event.event_ref = reference("forged-identity"),
    ];
    for mutate in mutations {
        let mut observed = exercise_committed_subscriber_case(&case).expect("exercise");
        mutate(&mut observed.published[0]);
        observed.delivered_before_restart[0] = observed.published[0].clone();
        observed.delivered_after_restart[0] = observed.published[0].clone();
        observed.deduplicated[0] = observed.published[0].clone();
        let result = evaluate_committed_subscriber(&case, Some(observed));
        assert_eq!(result.disposition, ConformanceDisposition::Failed);
        assert!(result.diagnostics.contains(&WorldFaultIssue::SubscriberObservationMismatch("published-event")));
    }
}

// r[verify molten.world_faults.profile]
#[test]
fn subscriber_identity_binds_every_configurable_valid_case_field() {
    let profile = standard_world_fault_profile("subscriber-regression").expect("profile");
    let original = identify_world_fault_profile(&profile).expect("identity");
    let mutations: [fn(&mut CommittedSubscriberCase); 5] = [
        |case| case.case_id.push_str(":other"),
        |case| case.service_id.push_str(":other"),
        |case| case.stream_epoch += 1,
        |case| case.event_count += 1,
        |case| case.buffer_bound += 1,
    ];
    for mutate in mutations {
        let mut changed = profile.clone();
        mutate(&mut changed.subscriber);
        assert_ne!(identify_world_fault_profile(&changed).expect("changed identity"), original);
    }
    let mut wrong_phase = profile;
    wrong_phase.subscriber.phase = FaultPhase::ProcessRestart;
    assert!(identify_world_fault_profile(&wrong_phase).is_err());
}

// r[verify molten.world_faults.receipt]
#[test]
fn a_complete_observation_from_another_service_or_epoch_is_not_reusable() {
    let case = case();
    for (service_id, stream_epoch) in [("service:foreign", 1), (case.service_id.as_str(), 2)] {
        let foreign = CommittedSubscriberCase {
            service_id: service_id.to_string(),
            stream_epoch,
            ..case.clone()
        };
        let observed = exercise_committed_subscriber_case(&foreign).expect("foreign real stream exercise");
        let result = evaluate_committed_subscriber(&case, Some(observed));
        assert_eq!(result.disposition, ConformanceDisposition::Failed);
        assert!(result.diagnostics.contains(&WorldFaultIssue::SubscriberObservationMismatch("checkpoint-binding")));
        assert!(result.diagnostics.contains(&WorldFaultIssue::SubscriberObservationMismatch("published-event")));
    }
}
