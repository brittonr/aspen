use super::*;

// r[verify molten.chaoscontrol_client.real_execution]
// r[verify molten.chaoscontrol_client.observation]
#[test]
fn retained_native_healthy_control() {
    let workspace = crate::test_support::process_workspace("client-native-healthy").expect("workspace");
    let input = crate::fabric_simulation::bounded_client::Input::native(
        NATIVE_RECOVERY_TICKS,
        crate::fabric_simulation::bounded_client::SemanticControl::Healthy,
    )
    .expect("input");
    let history = run(&input, &workspace).expect("real native execution");
    let check = crate::fabric_simulation::bounded_client::check_history(&input, &history, true);
    persist_check(&workspace, &history, &check).expect("retain check");
    retain(&workspace, "healthy").expect("retain native artifacts");
    assert_eq!(check.verdict, crate::fabric_simulation::bounded_client::Verdict::Pass);
    assert_eq!(check.liveness.len(), crate::fabric_simulation::bounded_client::CLIENT_PARTICIPANTS.len());
    assert!(history.participants.iter().all(|participant| {
        participant.observed_ticks == input.recovery_ticks
            && participant.end_term.is_some_and(|term| term > participant.start_term)
            && participant.is_fault_off
    }));
    assert!(history.participants.iter().all(|participant| matches!(
        participant.recovery,
        crate::fabric_simulation::bounded_client::RecoverySource::Local(_)
    )));
    assert_eq!(
        crate::fabric_simulation::bounded_client::classify_semantic_control(&history, &check),
        crate::fabric_simulation::bounded_client::SemanticControlVerdict::Healthy
    );
    forge_owner_receipt(&workspace);
    let forged =
        history::collect(&input, &workspace, history.executed_requests).expect("parse forged canonical history");
    let rejected = crate::fabric_simulation::bounded_client::check_history(&input, &forged, true);
    assert!(matches!(rejected.verdict, crate::fabric_simulation::bounded_client::Verdict::Invalid { .. }));
    persist_check(&workspace, &forged, &rejected).expect("retain forged check");
    retain(&workspace, "forged-receipt").expect("retain forged receipt control");
}

// r[verify molten.chaoscontrol_client.controls]
// r[verify molten.chaoscontrol_client.verification]
#[test]
fn retained_native_mutant_repeats_reduces_and_requires_its_property() {
    let input = crate::fabric_simulation::bounded_client::Input::native(
        NATIVE_RECOVERY_TICKS,
        crate::fabric_simulation::bounded_client::SemanticControl::DuplicateApplication,
    )
    .expect("input");
    let original = crate::test_support::process_workspace("client-native-original").expect("original workspace");
    let history = run(&input, &original).expect("original real native execution");
    let check = crate::fabric_simulation::bounded_client::check_history(&input, &history, true);
    persist_check(&original, &history, &check).expect("original check");
    let expected = crate::fabric_simulation::bounded_client::SemanticControlVerdict::ReachedDetected {
        property: crate::fabric_simulation::bounded_client::APPLY_ONCE_PROPERTY.to_string(),
    };
    if crate::fabric_simulation::bounded_client::classify_semantic_control(&history, &check) != expected {
        retain(&original, "original-unexpected").expect("retain unexpected result");
    }
    assert_eq!(crate::fabric_simulation::bounded_client::classify_semantic_control(&history, &check), expected);
    let suppressed = crate::fabric_simulation::bounded_client::check_history(&input, &history, false);
    assert!(matches!(suppressed.verdict, crate::fabric_simulation::bounded_client::Verdict::Invalid { .. }));
    write_value(
        &original.join("client-property-suppressed.preserves"),
        &crate::preserves_rail::record("molten-client-property-suppressed-v1", vec![
            crate::preserves_rail::string(&history.input_ref),
            crate::preserves_rail::string(crate::fabric_simulation::bounded_client::APPLY_ONCE_PROPERTY),
            crate::preserves_rail::string("invalid-required-property-suppressed"),
        ]),
    )
    .expect("suppression evidence");

    // run() returns only after its children exited or were reaped. This fresh
    // process set uses the same immutable test executable, not saved outcomes.
    let repeated = crate::test_support::process_workspace("client-native-repeat").expect("repeat workspace");
    let repeated_history = run(&input, &repeated).expect("independent native repeat");
    let repeated_check = crate::fabric_simulation::bounded_client::check_history(&input, &repeated_history, true);
    persist_check(&repeated, &repeated_history, &repeated_check).expect("repeat check");
    retain(&repeated, "original-repeat").expect("retain repeat");
    assert_eq!(
        crate::fabric_simulation::bounded_client::classify_semantic_control(&repeated_history, &repeated_check),
        expected
    );
    assert_eq!(history.input_ref, repeated_history.input_ref);
    let original_binding = read_value(&original.join("client-build-bindings.preserves")).expect("original binding");
    let repeated_binding = read_value(&repeated.join("client-build-bindings.preserves")).expect("repeat binding");
    assert_eq!(original_binding, repeated_binding);

    let reduction = crate::fabric_simulation::bounded_client::reduce_valid_prefix(&input, crate::fabric_simulation::bounded_client::APPLY_ONCE_PROPERTY, |candidate, property| {
        let workspace = crate::test_support::process_workspace("client-native-reduction")?;
        let history = run(candidate, &workspace)?;
        let check = crate::fabric_simulation::bounded_client::check_history(candidate, &history, true);
        persist_check(&workspace, &history, &check)?;
        retain(&workspace, &format!("reduction-prefix-{}", candidate.steps.len()))?;
        Ok(matches!(&check.verdict, crate::fabric_simulation::bounded_client::Verdict::Fail { property: observed, .. } if observed == property))
    }).expect("real valid-prefix reduction");
    assert_eq!(reduction.prefix.steps.len(), 3);
    write_value(
        &original.join("client-reduced-prefix.preserves"),
        &crate::fabric_simulation::bounded_client::reduced_value(&reduction).expect("reduced value"),
    )
    .expect("retain reduction");
    retain(&original, "original").expect("retain original");
}

// r[verify molten.chaoscontrol_client.real_execution]
#[test]
fn native_executor_rejects_request_substitution_before_spawning() {
    use crate::fabric_simulation::WorkloadExecutor as _;
    let workspace = crate::test_support::process_workspace("client-native-substitution").expect("workspace");
    let input = crate::fabric_simulation::bounded_client::Input::native(
        1,
        crate::fabric_simulation::bounded_client::SemanticControl::Healthy,
    )
    .expect("input");
    let executable = running_executable().expect("executing test image");
    let mut executor = executor::Driver::new(&input, &workspace, &executable);
    let mut step = input.differential_workload().expect("workload").steps.remove(0);
    step.request_ref = super::super::super::tests::test_ref("substitution");
    assert_eq!(executor.execute_step(&step), None);
    assert!(executor.executed.is_empty());
    assert!(executor.failure.is_some());
}

// r[verify molten.chaoscontrol_client.real_execution]
// r[verify molten.chaoscontrol_client.bounded_input]
#[test]
fn native_precheckpoint_prefixes_remain_incomplete_without_io_abort() {
    let full = crate::fabric_simulation::bounded_client::Input::native(
        NATIVE_RECOVERY_TICKS,
        crate::fabric_simulation::bounded_client::SemanticControl::DuplicateApplication,
    )
    .expect("input");
    for length in 1..=2 {
        let input = full.valid_prefix(length).expect("valid precheckpoint prefix");
        let workspace = crate::test_support::process_workspace("client-native-precheckpoint").expect("workspace");
        let history = run(&input, &workspace).expect("prefix execution without nonexistent checkpoint read");
        assert_eq!(
            history.executed_requests,
            input.steps.iter().map(|step| step.request_ref.clone()).collect::<Vec<_>>()
        );
        assert!(history.participants.is_empty());
        let check = crate::fabric_simulation::bounded_client::check_history(&input, &history, true);
        assert!(matches!(check.verdict, crate::fabric_simulation::bounded_client::Verdict::Incomplete { .. }));
        persist_check(&workspace, &history, &check).expect("retain incomplete check");
        retain(&workspace, &format!("precheckpoint-prefix-{length}")).expect("retain prefix evidence");
    }
}

// These artifact-level controls do not claim child-process execution.
// r[verify molten.chaoscontrol_client.observation]
#[test]
fn native_recovery_horizon_counts_only_after_advancement_and_fault_off() {
    let workspace = crate::test_support::process_workspace("client-horizon-eligibility").expect("workspace");
    let input = crate::fabric_simulation::bounded_client::Input::native(
        4,
        crate::fabric_simulation::bounded_client::SemanticControl::Healthy,
    )
    .expect("input");
    let mut horizon = client_projection::RecoveryHorizon::new(1);
    let marker = workspace.join("node-a-client-horizon.preserves");
    for step in 0..=input.recovery_ticks {
        horizon.observe(&workspace, "node-a", step, 1, &input).expect("not advanced");
    }
    assert!(!marker.is_file());
    assert_eq!(
        history::observed_horizon(&workspace, "node-a", &input, Some(1), Some(2)).expect("missing interval"),
        (0, 1, None, false)
    );
    horizon.observe(&workspace, "node-a", 5, 2, &input).expect("advanced");
    write_signal(&workspace.join(PARTITION_FILE), "partition").expect("activate disruptive fault");
    for step in 6..=10 {
        horizon.observe(&workspace, "node-a", step, 2, &input).expect("fault active");
    }
    assert!(!marker.is_file());
    std::fs::remove_file(workspace.join(PARTITION_FILE)).expect("fault off");
    for step in 11..15 {
        horizon.observe(&workspace, "node-a", step, 2, &input).expect("fault-off interval");
        assert!(!marker.is_file());
    }
    horizon.observe(&workspace, "node-a", 15, 2, &input).expect("complete interval");
    assert_eq!(
        history::observed_horizon(&workspace, "node-a", &input, Some(1), Some(3)).expect("bound interval"),
        (4, 1, Some(2), true)
    );
    assert_eq!(
        history::observed_horizon(&workspace, "node-a", &input, Some(1), None).expect("terminal missing"),
        (4, 1, None, true)
    );
}

// r[verify molten.chaoscontrol_client.observation]
#[test]
fn native_horizon_checker_rejects_late_election_after_earlier_interval() {
    let workspace = crate::test_support::process_workspace("client-horizon-late-election").expect("workspace");
    let input = crate::fabric_simulation::bounded_client::Input::native(
        4,
        crate::fabric_simulation::bounded_client::SemanticControl::Healthy,
    )
    .expect("input");
    // The terminal term advanced, but this older interval did not. Neither a
    // stalled interval nor one that advances only at its end is stabilized.
    for terms in [[1, 1], [1, 2]] {
        write_horizon_fixture(&workspace, &input, [1, 0, 4, terms[0], terms[1]], true);
        assert!(history::observed_horizon(&workspace, "node-a", &input, Some(1), Some(2)).is_err());
    }
    write_horizon_fixture(&workspace, &input, [1, 5, 9, 2, 2], true);
    assert_eq!(
        history::observed_horizon(&workspace, "node-a", &input, Some(1), Some(3)).expect("healthy interval"),
        (4, 1, Some(2), true)
    );
    assert!(history::observed_horizon(&workspace, "node-a", &input, Some(2), Some(3)).is_err());
    assert!(history::observed_horizon(&workspace, "node-a", &input, None, Some(3)).is_err());
    assert!(history::observed_horizon(&workspace, "node-a", &input, Some(1), Some(1)).is_err());
    write_horizon_fixture(&workspace, &input, [1, 5, 8, 2, 2], true);
    assert!(history::observed_horizon(&workspace, "node-a", &input, Some(1), Some(3)).is_err());
    let end_step = input.max_event_loop_iterations;
    write_horizon_fixture(&workspace, &input, [1, end_step - input.recovery_ticks, end_step, 2, 2], true);
    assert!(history::observed_horizon(&workspace, "node-a", &input, Some(1), Some(3)).is_err());
    write_horizon_fixture(&workspace, &input, [1, 5, 9, 2, 2], false);
    assert!(history::observed_horizon(&workspace, "node-a", &input, Some(1), Some(3)).is_err());
}

fn write_horizon_fixture(
    directory: &std::path::Path,
    input: &crate::fabric_simulation::bounded_client::Input,
    interval: [u64; 5],
    is_fault_off: bool,
) {
    let mut fields = vec![
        crate::preserves_rail::string(crate::fabric_simulation::bounded_client::input_ref(input).expect("input ref")),
        crate::preserves_rail::string("node-a"),
    ];
    fields.extend(interval.into_iter().map(crate::preserves_rail::u64_value));
    fields.push(crate::preserves_rail::bool_value(is_fault_off));
    write_value(
        &directory.join("node-a-client-horizon.preserves"),
        &crate::preserves_rail::record("molten-client-native-horizon-v2", fields),
    )
    .expect("write horizon fixture");
}

fn forge_owner_receipt(directory: &std::path::Path) {
    let owner_path = directory.join("node-a-client-checkpoint-owner.preserves");
    let mut receipt = parse(&read_value(&owner_path).expect("actual owner receipt")).expect("owner projection");
    receipt.commit_quorum_members = vec!["node-a".to_string(), "node-a".to_string()];
    let forged = value(&receipt);
    let reference = crate::preserves_rail::canonical_hash(&forged).expect("forged receipt identity");
    write_value(&owner_path, &forged).expect("write tampered canonical receipt");
    let sample_path = directory.join("node-a-client-checkpoint.preserves");
    let sample = read_value(&sample_path).expect("actual sample");
    let fields = crate::preserves_rail::simple_record_fields(&sample, "molten-client-native-sample-v1", 10)
        .expect("sample shape");
    let values = (0..10)
        .map(|index| {
            if index == 4 {
                crate::preserves_rail::string(&reference)
            } else {
                crate::preserves_rail::value_to_iovalue(&fields[index])
            }
        })
        .collect();
    write_value(&sample_path, &crate::preserves_rail::record("molten-client-native-sample-v1", values))
        .expect("bind forged receipt");
}
