use std::collections::BTreeMap;

use super::*;
use crate::fabric::ReferenceSystemKind;
use crate::fabric_simulation::*;

fn log_state(entries: &[u64], next_offset: u64, retained_from: u64, replicated: Option<u64>) -> ReferenceServiceState {
    ReferenceServiceState::ReplicatedLog(ReplicatedLogState {
        next_offset,
        retained_from,
        replicated_through: replicated,
        entries: entries
            .iter()
            .map(|offset| ReplicatedLogEntry {
                offset: *offset,
                payload_ref: content(&format!("payload-{offset}")),
            })
            .collect(),
    })
}

fn reference_projection(state: &ReferenceServiceState, source_sequence: u64, reported: &[&str]) -> ProtocolProjection {
    let value = reference_state_value(state);
    ProtocolProjection {
        protocol: REFERENCE_PROTOCOL.to_owned(),
        service: state.kind().as_str().to_owned(),
        participant: "node-replicated-log".to_owned(),
        generation: 1,
        source_sequence,
        run_sequence: source_sequence,
        scheduler_choice_ref: content(&format!("choice-{source_sequence}")),
        transition_class: TransitionClass::Applied,
        logical_position: reference_logical_position(state, source_sequence),
        state_ref: state_ref_of(&value),
        state: value,
        physical_ref: None,
        history_ref: content(&format!("history-{source_sequence}")),
        durability: DurabilityFact::Volatile,
        progress: source_sequence,
        completeness: ObservationCompleteness::Complete,
        reported_invariants: reported.iter().map(|name| (*name).to_owned()).collect(),
    }
}

fn reference_profile() -> ProtocolOracleProfile {
    let mut profile = profile();
    profile.participants = vec![ParticipantProfile {
        participant: "node-replicated-log".to_owned(),
        service: SERVICE.to_owned(),
        is_required: true,
    }];
    profile
}

// r[verify molten.fabric_simulation.protocol_projection]
#[test]
fn reference_states_project_canonically_and_decode_back_exactly() {
    let mut values = BTreeMap::new();
    values.insert("k".to_owned(), content("v"));
    let mut jobs = BTreeMap::new();
    jobs.insert("job-a".to_owned(), ReferenceJobState {
        phase: ReferenceJobPhase::Completed,
        lease_owner: Some("worker-a".to_owned()),
        lease_epoch: 2,
        authoritative_completion_ref: Some(content("done")),
    });
    for state in [
        ReferenceServiceState::TransactionalKeyValue(TransactionalKeyValueState {
            version: 3,
            values,
            recovered_version: Some(2),
        }),
        log_state(&[1, 2], 3, 1, Some(2)),
        ReferenceServiceState::DistributedScheduler(DistributedSchedulerState { jobs }),
    ] {
        let value = reference_state_value(&state);
        assert_eq!(decode_reference_state(&value), Some(state.clone()));
        assert_eq!(reference_state_ref(&state), state_ref_of(&value));
        assert_ne!(
            reference_state_ref(&state),
            format!("blake3:{}", blake3::hash(format!("{state:?}").as_bytes()).to_hex())
        );
    }
    assert_eq!(decode_reference_state(&CanonicalValue::record("replicated-log", Vec::new())), None);
}

// r[verify molten.fabric_simulation.oracle_independence]
// r[verify molten.fabric_simulation.protocol_oracle_validation]
#[test]
fn a_false_self_report_cannot_hide_a_projection_that_violates_a_local_rule() {
    let profile = reference_profile();
    let first = reference_projection(&log_state(&[0], 1, 0, None), 0, &["log-offsets-contiguous"]);
    let broken = reference_projection(&log_state(&[0, 2], 3, 0, None), 1, &[
        "log-offsets-contiguous",
        "retention-follows-replication",
    ]);
    let cohort = assemble_cohort(&profile, vec![first, broken.clone()]).expect("cohort");
    let results = evaluate_safety(&profile, &cohort, &ReferenceProtocolSemantics);
    let contiguous = results.iter().find(|result| result.rule == "log-offsets-contiguous").expect("rule");
    let OracleVerdict::Fail(witness) = &contiguous.verdict else {
        panic!("the reported success is ignored: {results:?}");
    };
    assert_eq!(witness.projection_ref, projection_ref(&broken));
    let retention = results.iter().find(|result| result.rule == "retention-follows-replication").expect("rule");
    assert_eq!(retention.verdict, OracleVerdict::Pass);
}

// r[verify molten.fabric_simulation.local_protocol_guards]
#[test]
fn the_executor_guard_and_the_oracle_share_one_local_rule_function() {
    let before = log_state(&[0], 1, 0, None);
    let operation = ReferenceServiceOperation::ReplicatedLog(ReplicatedLogOperation::Append {
        payload_ref: content("next"),
    });
    let honest = apply_reference_operation(&before, &operation).expect("append");
    assert_eq!(guard_reference_transition(&before, &operation, &honest), Ok(()));

    let mut impossible = honest.clone();
    impossible.next = log_state(&[0], 1, 1, None);
    assert_eq!(
        guard_reference_transition(&before, &operation, &impossible),
        Err(ReferenceGuardViolation {
            service: ReferenceSystemKind::ReplicatedLog,
            rule: "log-offsets-contiguous",
        })
    );
    let projected = reference_projection(&impossible.next, 1, &[]);
    let previous = reference_projection(&before, 0, &[]);
    assert_eq!(ReferenceProtocolSemantics.check_local(Some(&previous), &projected), vec![
        "log-offsets-contiguous",
        "retention-follows-replication"
    ]);

    let malformed = apply_reference_operation(
        &before,
        &ReferenceServiceOperation::ReplicatedLog(ReplicatedLogOperation::Append {
            payload_ref: "not-a-ref".to_owned(),
        }),
    );
    assert!(
        matches!(malformed, Err(ReferenceServiceIssue::MalformedRef { .. })),
        "a malformed command stays a typed admission error, not an invariant violation"
    );
}
