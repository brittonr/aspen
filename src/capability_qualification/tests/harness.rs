use super::*;

// r[verify molten.testing.capability_qualification.exact_evidence]
// r[verify molten.testing.capability_context.request_binding]
#[test]
fn fixed_native_positive_and_negative_execute_and_validate_without_live_promotion() {
    let (source, gate) = fixture_source(&HARNESS_SCENARIOS);
    for scenario in HARNESS_SCENARIOS {
        matrix::assert_scenario_bound(scenario);
        let value = recorded(&source, gate.clone(), scenario);
        let subject = source.subject_for(scenario).expect("subject");
        let actual = validate(&source, &value, subject).expect("native exact-scope validation");
        assert_eq!(actual.decision, "pass");
        assert_eq!(actual.scenario, scenario);
        assert_eq!(actual.subject.environment, Environment::Simulation);
        assert_eq!(
            actual.canonical_decision,
            if scenario == Scenario::DeniedSend {
                "expected-deny"
            } else {
                "pass"
            }
        );
    }
}

// r[verify molten.testing.capability_qualification.exact_evidence]
#[test]
fn every_subject_dimension_denies_even_when_caller_rewrites_receipt_and_input_together() {
    let (source, gate) = fixture_source(&HARNESS_SCENARIOS);
    let suite = fixture(Scenario::GrantedSend).expect("fixed suite");
    let (artifact, _) = execute(Scenario::GrantedSend, &suite, &programs()).expect("real run");
    for dimension in 0..8 {
        let mut forged = source.subject_for(Scenario::GrantedSend).expect("subject").clone();
        match dimension {
            0 => forged.source_ref = reference("other source"),
            1 => forged.build_ref = reference("other executable"),
            2 => forged.dependency_closure_ref = reference("other closure"),
            3 => forged.feature_set.push("caller-invented-feature".to_string()),
            4 => forged.execution_profile = "onixos-live".to_string(),
            5 => forged.adapter_refs = vec![reference("other adapter")],
            6 => forged.platform = "other-platform".to_string(),
            _ => forged.environment = Environment::CrossHost,
        }
        let value = execution_value(&forged, Scenario::GrantedSend, &source.binding_ref, ExecutionArtifacts {
            validation: gate.clone(),
            suite: suite.clone(),
            artifact: artifact.clone(),
        })
        .expect("forged metadata fixture");
        assert!(validate(&source, &value, &forged).is_err(), "dimension {dimension} cannot substitute native facts");
    }
}

// r[verify molten.testing.capability_qualification.exact_evidence]
#[test]
fn stale_gate_wrong_fixed_scenario_and_generic_success_are_rejected() {
    let (source, gate) = fixture_source(&HARNESS_SCENARIOS);
    let subject = source.subject_for(Scenario::GrantedSend).expect("subject").clone();
    let suite = fixture(Scenario::GrantedSend).expect("fixed suite");
    let (artifact, _) = execute(Scenario::GrantedSend, &suite, &programs()).expect("real native run");
    let wrong_suite = execution_value(&subject, Scenario::DeniedSend, &source.binding_ref, ExecutionArtifacts {
        validation: gate.clone(),
        suite: suite.clone(),
        artifact: artifact.clone(),
    })
    .expect("wrong scenario fixture");
    assert!(validate(&source, &wrong_suite, &subject).is_err());
    let wrong_gate = execution_value(&subject, Scenario::GrantedSend, &source.binding_ref, ExecutionArtifacts {
        validation: crate::preserves_rail::string("fake gate"),
        suite,
        artifact,
    })
    .expect("wrong gate fixture");
    assert!(validate(&source, &wrong_gate, &subject).is_err());
    assert!(validate(&source, &crate::preserves_rail::record("generic-pass", vec![]), &subject).is_err());
}

// r[verify molten.testing.capability_qualification.exact_evidence]
#[test]
fn canonical_runtime_failure_is_retained_and_never_promoted() {
    let (source, gate) = fixture_source(&[]);
    for scenario in [Scenario::GrantedSend, Scenario::DurableRestartRecovery] {
        let suite = fixture(scenario).expect("suite");
        let failure =
            crate::harness::suite_failure_value("execute", &invalid("bounded runtime failure control"), &suite)
                .expect("native failure");
        let subject = source.subject_for(scenario).expect("subject");
        let value = execution_value(subject, scenario, &source.binding_ref, ExecutionArtifacts {
            validation: gate.clone(),
            suite,
            artifact: failure,
        })
        .expect("retained failed run");
        let validated = validate(&source, &value, subject).expect("valid failed evidence");
        assert_eq!(validated.decision, "fail");
        assert_eq!(validated.canonical_decision, "fail");
    }
}

// r[verify molten.testing.capability_qualification.exact_evidence]
#[test]
fn producer_does_not_execute_without_independently_authenticated_current_source() {
    let workspace =
        crate::test_support::process_workspace("capability-source-admission").expect("isolated source root");
    let root = crate::node_state::DirectoryRoot::open_existing(&workspace).expect("root capability");
    assert!(produce(&root, Scenario::GrantedSend).is_err());
    assert!(produce(&root, Scenario::DurableRestartRecovery).is_err());
    assert!(observe_subject(&root).is_err());
    assert!(!workspace.join("receipts").exists());
}

// r[verify molten.testing.capability_qualification.diagnostic_output]
#[test]
fn repeated_actual_runtime_fixtures_have_identical_canonical_receipts() {
    let (source, gate) = fixture_source(&[]);
    let first = recorded(&source, gate.clone(), Scenario::GrantedSend);
    let second = recorded(&source, gate, Scenario::GrantedSend);
    assert_eq!(
        crate::preserves_rail::canonical_bytes(&first).expect("canonical first"),
        crate::preserves_rail::canonical_bytes(&second).expect("canonical second")
    );
}

// r[verify molten.testing.capability_qualification.exact_evidence]
#[test]
fn valid_native_artifact_requires_the_selected_binarys_actual_run_identity() {
    let (mut source, gate) = fixture_source(&[Scenario::GrantedSend]);
    let value = recorded(&source, gate, Scenario::GrantedSend);
    let subject = source.subject_for(Scenario::GrantedSend).expect("subject").clone();
    source.observed_receipts.clear();
    assert!(validate(&source, &value, &subject).is_err());
    source
        .observed_receipts
        .insert(Scenario::GrantedSend.id().to_string(), reference("different actual native run"));
    assert!(validate(&source, &value, &subject).is_err());
}
