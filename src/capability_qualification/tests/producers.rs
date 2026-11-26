use super::*;

/// One actual run, bound to the runtime-artifact ref in its matrix row.
fn assert_product_scenario(scenario: Scenario) -> preserves::IOValue {
    matrix::assert_scenario_bound(scenario)
}

// r[verify molten.fabric_durability.snapshot_recovery]
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn durable_restart_recovers_through_the_redb_adapter() {
    assert_product_scenario(Scenario::DurableRestartRecovery);
}

// r[verify molten.fabric_durability.snapshot_recovery]
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn durable_corrupt_snapshot_is_denied_by_the_redb_adapter() {
    assert_product_scenario(Scenario::DurableCorruptSnapshotDenied);
}

// Positive and negative tamper controls over a real product receipt.
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn product_receipts_validate_only_untampered_actual_execution() {
    let scenario = Scenario::DurableRestartRecovery;
    let (source, gate) = fixture_source(&[scenario]);
    let subject = source.subject_for(scenario).expect("subject").clone();
    let value = recorded(&source, gate.clone(), scenario);
    let validated = validate(&source, &value, &subject).expect("positive control");
    assert_eq!(validated.requirement_id, "molten.fabric_durability.snapshot_recovery");
    assert_eq!(validated.family, "durable-restart-same-process");
    assert_eq!(validated.subject.environment, Environment::SameProcess);
    assert_eq!(validated.canonical_decision, "pass");

    let suite = fixture(scenario).expect("plan");
    let (artifact, _) = runtime_artifact(scenario, &programs()).expect("actual run");
    let rewrite = |artifact: preserves::IOValue| {
        execution_value(&subject, scenario, &source.binding_ref, ExecutionArtifacts {
            validation: gate.clone(),
            suite: suite.clone(),
            artifact,
        })
        .expect("tampered receipt")
    };
    let text = crate::preserves_rail::to_text(&artifact).expect("observation text");
    let flipped = crate::preserves_rail::parse_text(&text.replacen("#t", "#f", 1)).expect("flipped check");
    assert!(validate(&source, &rewrite(flipped), &subject).is_err(), "failed invariant accepted");
    let relabelled =
        crate::preserves_rail::parse_text(&text.replace(scenario.id(), Scenario::DurableCorruptSnapshotDenied.id()))
            .expect("relabelled observation");
    assert!(validate(&source, &rewrite(relabelled), &subject).is_err(), "scenario relabel accepted");
    let empty_checks = products::observation(scenario, &products::Evidence::default());
    assert!(empty_checks.is_err(), "empty observation accepted");
    let other_scenario = recorded(&source, gate.clone(), Scenario::DurableCorruptSnapshotDenied);
    assert!(validate(&source, &other_scenario, &subject).is_err(), "unobserved scenario run accepted");
    let mut environment = subject.clone();
    environment.environment = Environment::DistinctProcess;
    let rebound = execution_value(&environment, scenario, &source.binding_ref, ExecutionArtifacts {
        validation: gate,
        suite: suite.clone(),
        artifact: artifact.clone(),
    })
    .expect("environment rewrite");
    assert!(validate(&source, &rebound, &environment).is_err(), "environment promotion accepted");
    let harness_plan = execution_value(&subject, scenario, &source.binding_ref, ExecutionArtifacts {
        validation: crate::preserves_rail::record("qualification-test-source-fixture-v1", Vec::new()),
        suite: fixture(Scenario::GrantedSend).expect("harness suite"),
        artifact,
    })
    .expect("plan swap");
    assert!(validate(&source, &harness_plan, &subject).is_err(), "plan swap accepted");
}

// r[verify molten.fabric_execution.uncertainty]
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn started_child_effect_without_completion_stays_unknown() {
    assert_product_scenario(Scenario::ExecutionUnknownAfterStart);
}

// r[verify molten.fabric_execution.uncertainty]
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn prestart_execution_failures_stay_definite() {
    assert_product_scenario(Scenario::ExecutionPreStartDefinite);
}

// r[verify molten.system_extension.native_effect_uncertainty]
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn host_work_stays_unknown_after_a_started_process_effect() {
    assert_product_scenario(Scenario::NativeWorkUnknownAfterEffect);
}

// r[verify molten.system_extension.native_effect_uncertainty]
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn uncertain_host_work_is_never_readmitted_or_repeated() {
    assert_product_scenario(Scenario::NativeUncertainWorkNotReadmitted);
}

// r[verify molten.fabric_transport.cross_process_endpoint]
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn transport_endpoint_handoff_imports_from_a_distinct_listener_process() {
    assert_product_scenario(Scenario::TransportEndpointImport);
}

// r[verify molten.fabric_transport.cross_process_endpoint]
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn transport_endpoint_tampering_is_denied_before_dial() {
    assert_product_scenario(Scenario::TransportEndpointTamperDenied);
}

// r[verify molten.fabric_transport.cross_process_session]
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn transport_session_frame_is_acknowledged_across_processes() {
    assert_product_scenario(Scenario::TransportSessionAcknowledged);
}

// r[verify molten.fabric_transport.cross_process_session]
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn transport_session_withheld_acknowledgement_is_not_retried() {
    assert_product_scenario(Scenario::TransportSessionLostAcknowledgement);
}
