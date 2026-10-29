use super::super::*;

#[test]
fn mantle_materialized_executes_through_generated_wit_bindings() {
    // r[verify molten.wasm_component.abi]
    // r[verify molten.wasm_component.determinism]
    // r[verify molten.wasm_component.receipts]
    // r[verify molten.wasm_component.validation]
    for consumer in [Consumer::Actor, Consumer::SystemExtension] {
        let fixture = super::support::Fixture::new(consumer);
        let input = super::support::input_value();
        let outcome = super::support::execute(&fixture.request(&input));
        assert!(outcome.is_pass(), "{:?}", outcome.diagnostics);
        assert_eq!(outcome.output.as_ref(), Some(&input));
        assert_eq!(outcome.receipts.len(), 3);
        assert_eq!(outcome.receipts[0].input.stage, Stage::Inspection);
        assert_eq!(outcome.receipts[1].input.stage, Stage::Instantiation);
        assert_eq!(outcome.receipts[2].input.stage, Stage::Execution);
        assert!(outcome.receipts.iter().all(|receipt| receipt.input.consumer == consumer));
    }
}

#[test]
fn identical_execution_replays_to_identical_canonical_evidence() {
    // r[verify molten.wasm_component.determinism]
    // r[verify molten.wasm_component.receipts]
    let fixture = super::support::Fixture::new(Consumer::Actor);
    let input = super::support::input_value();
    let left = super::support::execute(&fixture.request(&input));
    let right = super::support::execute(&fixture.request(&input));
    assert!(left.is_pass(), "{:?}", left.diagnostics);
    assert!(right.is_pass(), "{:?}", right.diagnostics);
    assert_eq!(left.output, right.output);
    assert!(replay_receipts_match(&left.receipts, &right.receipts));
}

#[test]
fn denies_malformed_wrong_world_and_over_resource_requests_with_receipts() {
    // r[verify molten.wasm_component.fixtures]
    // r[verify molten.wasm_component.resources]
    let mut malformed = super::support::Fixture::new(Consumer::Actor);
    malformed.replace_component_bytes(b"not-a-component".to_vec());
    let input = super::support::input_value();
    let malformed_outcome = super::support::execute(&malformed.request(&input));
    assert!(!malformed_outcome.is_pass());
    assert_eq!(malformed_outcome.receipts.len(), 1);
    assert_eq!(malformed_outcome.receipts[0].input.stage, Stage::Denial);

    let mut wrong_world = super::support::Fixture::new(Consumer::Actor);
    wrong_world.facts.declared_world = "wrong-world".to_string();
    let wrong_world_outcome = super::support::execute(&wrong_world.request(&input));
    assert!(!wrong_world_outcome.is_pass());
    assert_eq!(wrong_world_outcome.receipts.len(), 1);

    let mut exhausted = super::support::Fixture::new(Consumer::Actor);
    exhausted.facts.instances = exhausted.profile.resources.max_instances + 1;
    let exhausted_outcome = super::support::execute(&exhausted.request(&input));
    assert!(!exhausted_outcome.is_pass());
    assert_eq!(exhausted_outcome.receipts.len(), 1);
}

#[test]
fn production_loose_bytes_and_core_module_requests_never_fall_back() {
    // r[verify molten.wasm_component.materialization]
    // r[verify molten.wasm_component.migration]
    let fixture = super::support::Fixture::new(Consumer::Actor);
    let input = super::support::input_value();
    let loose_request = ExecutionRequest {
        profile: &fixture.profile,
        requested_profile: RequestedExecutionProfile::ComponentV1,
        evidence_scope: EvidenceScope::Production,
        source: ArtifactSource::TestOnlyLoose {
            component_bytes: &fixture.component_bytes,
            wit_bytes: &fixture.wit_bytes,
        },
        facts: &fixture.facts,
        import_manifest: &fixture.import_manifest,
        import_grants: &[],
        input: &input,
    };
    let loose = super::support::execute(&loose_request);
    assert!(!loose.is_pass());
    assert_eq!(loose.receipts.len(), 1);

    let mut core = super::support::Fixture::new(Consumer::Actor);
    core.replace_component_bytes(wat::parse_str("(module)").expect("core module"));
    let denied = super::support::execute(&core.request(&input));
    assert!(!denied.is_pass());
    assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("fallback is forbidden")));
}

#[test]
fn denies_invalid_preserves_output_and_fuel_exhaustion() {
    // r[verify molten.wasm_component.abi]
    // r[verify molten.wasm_component.resources]
    // r[verify molten.wasm_component.fixtures]
    let input = super::support::input_value();

    let mut invalid_output = super::support::Fixture::new(Consumer::Actor);
    invalid_output.replace_component_bytes(super::support::invalid_output_bytes());
    let invalid = super::support::execute(&invalid_output.request(&input));
    assert!(!invalid.is_pass());
    assert!(invalid.diagnostics.iter().any(|diagnostic| diagnostic.contains("canonical Preserves")));
    assert_eq!(invalid.receipts.len(), 3);
    assert_eq!(invalid.receipts[0].input.stage, Stage::Inspection);
    assert_eq!(invalid.receipts[1].input.stage, Stage::Instantiation);
    assert_eq!(invalid.receipts[2].input.stage, Stage::Denial);
    assert_eq!(invalid.receipts[2].input.trap_class.as_deref(), Some("invalid-preserves-payload"));

    let mut fuel_exhaustion = super::support::Fixture::new(Consumer::Actor);
    fuel_exhaustion.replace_component_bytes(super::support::fuel_exhaustion_bytes());
    let exhausted = super::support::execute(&fuel_exhaustion.request(&input));
    assert!(!exhausted.is_pass());
    assert!(exhausted.diagnostics.iter().any(|diagnostic| diagnostic.contains("fuel")));
    assert_eq!(exhausted.receipts.len(), 3);
    assert_eq!(exhausted.receipts[2].input.stage, Stage::Denial);
    assert_eq!(exhausted.receipts[2].input.trap_class.as_deref(), Some("fuel-exhausted"));
}

#[test]
fn reinspects_resource_facts_and_rejects_actual_growth() {
    // r[verify molten.wasm_component.determinism]
    // r[verify molten.wasm_component.resources]
    let input = super::support::input_value();

    let mut forged = super::support::Fixture::new(Consumer::Actor);
    forged.facts.memory.initial = 0;
    forged.facts.memory.maximum = Some(0);
    let forged_outcome = super::support::execute(&forged.request(&input));
    assert!(!forged_outcome.is_pass());
    assert!(forged_outcome.diagnostics.iter().any(|diagnostic| diagnostic.contains("differs")));

    let mut growing = super::support::Fixture::new(Consumer::Actor);
    growing.replace_component_bytes(super::support::dynamic_growth_bytes());
    let growing_outcome = super::support::execute(&growing.request(&input));
    assert!(!growing_outcome.is_pass());
    assert!(growing_outcome.diagnostics.iter().any(|diagnostic| diagnostic.contains("nondeterministic growth")));

    let (over_limit_bytes, declared_bytes) = super::support::over_limit_memory_bytes();
    let mut over_limit = super::support::Fixture::new(Consumer::Actor);
    over_limit.replace_component_bytes(over_limit_bytes);
    over_limit.facts.memory.initial = declared_bytes;
    over_limit.facts.memory.maximum = Some(declared_bytes);
    let over_limit_outcome = super::support::execute(&over_limit.request(&input));
    assert!(!over_limit_outcome.is_pass());
    assert!(over_limit_outcome.diagnostics.iter().any(|diagnostic| diagnostic.contains("resource bound")));
}

#[test]
fn untrusted_runtime_text_cannot_spoof_canonical_denial_classes() {
    // r[verify molten.wasm_component.receipts]
    let guest_denial =
        Denial::classified(DenialClass::GuestDenial, "component invoke denied: guest reported fuel exhaustion");
    assert_eq!(guest_denial.canonical_class(), "guest-denial");

    let trap = Denial::classified(
        DenialClass::ComponentTrap,
        "component invoke trapped and fuel observation failed (diagnostic): invalid canonical Preserves",
    );
    assert_eq!(trap.canonical_class(), "component-trap");

    let exhausted =
        Denial::classified(DenialClass::FuelExhausted, "component fuel exhausted during invoke: diagnostic text");
    assert_eq!(exhausted.canonical_class(), "fuel-exhausted");

    let unclassified = Denial::new("untrusted import name contains fuel, memory, and WIT");
    assert_eq!(unclassified.canonical_class(), "component-admission-denial");
}
