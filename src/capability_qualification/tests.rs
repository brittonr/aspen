use super::*;

mod harness;
mod producers;

fn reference(label: &str) -> String {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::string(label)).expect("canonical fixture identity")
}

/// Distinct-process peer entry for product producers under the unit-test
/// binary. Without the explicit peer environment it does nothing.
#[test]
fn peer_entry() {
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

// Every fixed scenario is bound to exactly one reviewed capability row,
// requirement, family/environment and profile of the checked-in mapping, and
// every (requirement, family, polarity) of a producer-backed capability has
// exactly one scenario: no aliasing, no unbound or missing coverage.
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn scenarios_bind_exactly_the_reviewed_mapping() {
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

fn matrix_target(capability: NativeProducer) -> &'static str {
    match capability.id {
        "durable-state-recovery" => "src/capability_qualification/products/durable.rs",
        "effect-uncertainty" => "src/capability_qualification/products/effect.rs",
        "cross-process-transport" => "src/capability_qualification/products/transport.rs",
        "world-replay-promotion-restore" => "src/capability_qualification/products/world.rs",
        "control-plane-failover" => "src/capability_qualification/products/control_plane/mod.rs",
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
