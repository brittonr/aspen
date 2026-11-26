use super::*;

fn reference(label: &str) -> String {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::string(label)).expect("canonical fixture identity")
}

/// Distinct-process peer entry for product producers under the unit-test
/// binary. Without the explicit peer environment it does nothing.
#[test]
fn qualification_peer_entry() {
    let (Some(role), Some(run_directory)) = (
        std::env::var_os(products::peer::PEER_ROLE_ENV),
        std::env::var_os(products::peer::PEER_RUN_DIRECTORY_ENV),
    ) else {
        return;
    };
    let role = role.into_string().expect("UTF-8 peer role");
    run_peer(&role, std::path::Path::new(&run_directory)).expect("qualification peer role");
}

fn launcher() -> Launcher {
    Launcher::test_harness()
}

// Explicit cfg(test) source token. It never calls/stands in for production
// authentication and cannot be constructed through the public readback API.
fn fixture_source(scenarios: &[Scenario]) -> (ReadbackSource, preserves::IOValue) {
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
    let observed_subjects = model::subjects(&binding).expect("test subjects");
    let mut source = ReadbackSource {
        binding_ref: reference("test binding"),
        validation_ref: crate::preserves_rail::canonical_hash(&validation).expect("test gate ref"),
        binding,
        observed_subjects,
        observed_receipts: std::collections::BTreeMap::new(),
    };
    for scenario in scenarios {
        let value = recorded(&source, validation.clone(), *scenario);
        source.observed_receipts.insert(
            scenario.id().to_string(),
            crate::preserves_rail::canonical_hash(&value).expect("actually executed native fixture ref"),
        );
    }
    (source, validation)
}

fn recorded(source: &ReadbackSource, validation: preserves::IOValue, scenario: Scenario) -> preserves::IOValue {
    let suite = fixture(scenario).expect("real fixed suite");
    let (artifact, decision) = execute(scenario, &suite, &launcher()).expect("actual native runtime run");
    assert_eq!(decision, "pass", "{} artifact: {artifact:?}", scenario.id());
    execution_value(source.subject_for(scenario).expect("subject"), scenario, &source.binding_ref, ExecutionArtifacts {
        validation,
        suite,
        artifact,
    })
    .expect("canonical run receipt")
}

const HARNESS_SCENARIOS: [Scenario; 2] = [Scenario::GrantedSend, Scenario::DeniedSend];

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
    let (artifact, _) = execute(Scenario::GrantedSend, &suite, &launcher()).expect("real run");
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
    let (artifact, _) = execute(Scenario::GrantedSend, &suite, &launcher()).expect("real native run");
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

// Every fixed scenario is bound to exactly one reviewed capability row,
// requirement, family/environment and profile of the checked-in mapping, and
// every (requirement, family, polarity) of a producer-backed capability has
// exactly one scenario: no aliasing, no unbound or missing coverage.
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn scenarios_bind_exactly_the_reviewed_capability_mapping() {
    let mapping: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/config/capability-report/capabilities.json"
    )))
    .expect("capability mapping");
    let rows = mapping["capabilities"].as_array().expect("capability rows");
    let mut covered = std::collections::BTreeSet::new();
    for scenario in Scenario::ALL {
        let definition = scenario.definition();
        let row = rows
            .iter()
            .find(|row| row["id"] == definition.capability.id)
            .unwrap_or_else(|| panic!("{} capability is not mapped", definition.id));
        assert!(
            row["requirements"]
                .as_array()
                .expect("requirements")
                .iter()
                .any(|item| item == definition.requirement)
        );
        assert!(
            row["profiles"]
                .as_array()
                .expect("profiles")
                .iter()
                .any(|item| item == definition.capability.execution_profile)
        );
        let family = row["families"]
            .as_array()
            .expect("families")
            .iter()
            .find(|family| family["id"] == definition.family)
            .unwrap_or_else(|| panic!("{} family is not mapped", definition.id));
        assert_eq!(family["environment"], products::environment_id(definition.capability.environment));
        assert!(
            covered.insert((definition.requirement, definition.family, definition.coverage_kind)),
            "{} aliases another scenario",
            definition.id
        );
    }
    for capability in CAPABILITIES {
        let row = rows.iter().find(|row| row["id"] == capability.id).expect("mapped capability");
        for requirement in row["requirements"].as_array().expect("requirements") {
            for family in row["families"].as_array().expect("families") {
                if family["environment"] != products::environment_id(capability.environment) {
                    continue;
                }
                for kind in ["positive", "negative"] {
                    assert!(
                        covered.iter().any(|(covered_requirement, covered_family, covered_kind)| {
                            requirement == covered_requirement
                                && family["id"] == *covered_family
                                && kind == *covered_kind
                        }),
                        "{} {requirement} {} {kind} has no native producer scenario",
                        capability.id,
                        family["id"]
                    );
                }
            }
        }
    }
}

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
    let (artifact, _) = runtime_artifact(scenario, &launcher()).expect("actual run");
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

fn matrix_target(capability: Capability) -> &'static str {
    match capability.id {
        "durable-state-recovery" => "src/capability_qualification/products/durable.rs",
        "effect-uncertainty" => "src/capability_qualification/products/effect.rs",
        "cross-process-transport" => "src/capability_qualification/products/transport.rs",
        "world-replay-promotion-restore" => "src/capability_qualification/products/world.rs",
        "control-plane-failover" => "src/capability_qualification/products/control_plane.rs",
        _ => "src/capability_qualification/mod.rs",
    }
}

/// Malformed or invented (sequential/repeated-nibble) digests are not the
/// identity of any real artifact.
fn is_placeholder_ref(reference: &str) -> bool {
    crate::preserves_rail::validate_content_ref(reference).is_err()
        || crate::hardening_core::is_placeholder_ref(reference)
}

// Every fixed scenario has exactly one checked-in matrix row with its exact
// requirement, polarity, scope, target and plan ref. The runtime-artifact ref
// of each row is re-executed and compared by that scenario's own test
// (`matrix::assert_scenario_bound`), so no single test runs every scenario.
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn evidence_matrix_rows_bind_every_scenario_plan() {
    for scenario in Scenario::ALL {
        let definition = scenario.definition();
        let row = matrix::scenario_entry(scenario);
        let plan_ref = crate::preserves_rail::canonical_hash(&fixture(scenario).expect("plan")).expect("plan ref");
        assert_eq!(row.requirement_id, definition.requirement, "{}", definition.id);
        assert_eq!(row.coverage_kind, definition.coverage_kind, "{}", definition.id);
        assert_eq!(row.evidence_scope, "integration", "{}", definition.id);
        assert_eq!(row.target, matrix_target(definition.capability), "{}", definition.id);
        assert_eq!(row.artifact_refs, [plan_ref], "{}", definition.id);
    }
}

// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn checked_in_evidence_matrix_has_no_placeholder_refs_or_aliased_commands() {
    let entries = matrix::entries();
    assert!(!entries.is_empty());
    let mut commands = std::collections::BTreeSet::new();
    for entry in &entries {
        assert!(commands.insert(entry.command.clone()), "command aliased across rows: {}", entry.command);
        assert!(!is_placeholder_ref(&entry.receipt_ref), "placeholder receipt ref for {}", entry.requirement_id);
        for reference in &entry.artifact_refs {
            assert!(!is_placeholder_ref(reference), "placeholder artifact ref for {}", entry.requirement_id);
        }
    }
    // Negative controls: the old sequential refs and aliasing are recognised.
    assert!(is_placeholder_ref("blake3:0000000000000000000000000000000000000000000000000000000000000101"));
    assert!(is_placeholder_ref("blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
    assert!(!is_placeholder_ref(&crate::preserves_rail::content_ref_from_bytes(b"actual bytes")));
}
