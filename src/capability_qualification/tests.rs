use super::*;

fn reference(label: &str) -> String {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::string(label)).expect("canonical fixture identity")
}

// Explicit cfg(test) source token. It never calls/stands in for production
// authentication and cannot be constructed through the public readback API.
fn fixture_source() -> (ReadbackSource, preserves::IOValue) {
    let binding = crate::core_api::startup_binding::Binding {
        schema: crate::core_api::startup_binding::SCHEMA.to_string(),
        producer_profile: crate::core_api::startup_binding::PRODUCER_PROFILE.to_string(),
        executable_ref: reference("test executable"),
        source_ref: reference("test source"),
        dependencies_ref: reference("test closure"),
        toolchain_ref: reference("test toolchain"),
        policy_ref: reference("test policy"),
        profile_ref: reference("test source profile"),
        artifacts: Vec::new(),
    };
    let validation = crate::preserves_rail::record("qualification-test-source-fixture-v1", Vec::new());
    let observed_subject = model::subject(&binding).expect("test subject");
    let mut source = ReadbackSource {
        binding_ref: reference("test binding"),
        validation_ref: crate::preserves_rail::canonical_hash(&validation).expect("test gate ref"),
        binding,
        observed_subject,
        observed_receipts: std::collections::BTreeMap::new(),
    };
    for scenario in [Scenario::GrantedSend, Scenario::DeniedSend] {
        let value = recorded(&source, validation.clone(), scenario);
        source.observed_receipts.insert(
            scenario.id().to_string(),
            crate::preserves_rail::canonical_hash(&value).expect("actually executed native fixture ref"),
        );
    }
    (source, validation)
}

fn recorded(source: &ReadbackSource, validation: preserves::IOValue, scenario: Scenario) -> preserves::IOValue {
    let suite = fixture(scenario).expect("real fixed suite");
    let (artifact, decision) = execute(&suite).expect("actual native runtime run");
    assert_eq!(decision, "pass");
    execution_value(&source.observed_subject, scenario, &source.binding_ref, ExecutionArtifacts {
        validation,
        suite,
        artifact,
    })
    .expect("canonical run receipt")
}

// r[verify molten.testing.capability_qualification.exact_evidence]
// r[verify molten.testing.capability_context.request_binding]
#[test]
fn fixed_native_positive_and_negative_execute_and_validate_without_live_promotion() {
    let (source, gate) = fixture_source();
    for scenario in [Scenario::GrantedSend, Scenario::DeniedSend] {
        let value = recorded(&source, gate.clone(), scenario);
        let actual = validate(&source, &value, &source.observed_subject).expect("native exact-scope validation");
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
    let (source, gate) = fixture_source();
    let suite = fixture(Scenario::GrantedSend).expect("fixed suite");
    let (artifact, _) = execute(&suite).expect("real run");
    for dimension in 0..8 {
        let mut forged = source.observed_subject.clone();
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
    let (source, gate) = fixture_source();
    let suite = fixture(Scenario::GrantedSend).expect("fixed suite");
    let (artifact, _) = execute(&suite).expect("real native run");
    let wrong_suite =
        execution_value(&source.observed_subject, Scenario::DeniedSend, &source.binding_ref, ExecutionArtifacts {
            validation: gate.clone(),
            suite: suite.clone(),
            artifact: artifact.clone(),
        })
        .expect("wrong scenario fixture");
    assert!(validate(&source, &wrong_suite, &source.observed_subject).is_err());
    let wrong_gate =
        execution_value(&source.observed_subject, Scenario::GrantedSend, &source.binding_ref, ExecutionArtifacts {
            validation: crate::preserves_rail::string("fake gate"),
            suite,
            artifact,
        })
        .expect("wrong gate fixture");
    assert!(validate(&source, &wrong_gate, &source.observed_subject).is_err());
    assert!(
        validate(&source, &crate::preserves_rail::record("generic-pass", vec![]), &source.observed_subject).is_err()
    );
}

// r[verify molten.testing.capability_qualification.exact_evidence]
#[test]
fn canonical_runtime_failure_is_retained_and_never_promoted() {
    let (source, gate) = fixture_source();
    let suite = fixture(Scenario::GrantedSend).expect("suite");
    let failure = crate::harness::suite_failure_value("execute", &invalid("bounded runtime failure control"), &suite)
        .expect("native failure");
    let value =
        execution_value(&source.observed_subject, Scenario::GrantedSend, &source.binding_ref, ExecutionArtifacts {
            validation: gate,
            suite,
            artifact: failure,
        })
        .expect("retained failed run");
    assert_eq!(validate(&source, &value, &source.observed_subject).expect("valid failed evidence").decision, "fail");
}

// r[verify molten.testing.capability_qualification.exact_evidence]
#[test]
fn producer_does_not_execute_without_independently_authenticated_current_source() {
    let workspace =
        crate::test_support::process_workspace("capability-source-admission").expect("isolated source root");
    let root = crate::node_state::DirectoryRoot::open_existing(&workspace).expect("root capability");
    assert!(produce(&root, Scenario::GrantedSend).is_err());
    assert!(observe_subject(&root).is_err());
    assert!(!workspace.join("receipts").exists());
}

// r[verify molten.testing.capability_qualification.diagnostic_output]
#[test]
fn repeated_actual_runtime_fixtures_have_identical_canonical_receipts() {
    let (source, gate) = fixture_source();
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
    let (mut source, gate) = fixture_source();
    let value = recorded(&source, gate, Scenario::GrantedSend);
    source.observed_receipts.clear();
    assert!(validate(&source, &value, &source.observed_subject).is_err());
    source
        .observed_receipts
        .insert(Scenario::GrantedSend.id().to_string(), reference("different actual native run"));
    assert!(validate(&source, &value, &source.observed_subject).is_err());
}
