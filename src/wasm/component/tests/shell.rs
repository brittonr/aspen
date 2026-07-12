use super::super::*;
use super::support::ComponentFixture;
use super::support::dynamic_growth_component_bytes;
use super::support::fuel_exhaustion_component_bytes;
use super::support::input_value;
use super::support::invalid_output_component_bytes;
use super::support::over_limit_memory_component_bytes;

#[test]
fn mantle_materialized_component_executes_through_generated_wit_bindings() {
    // r[verify molten.wasm_component.abi]
    // r[verify molten.wasm_component.determinism]
    // r[verify molten.wasm_component.receipts]
    // r[verify molten.wasm_component.validation]
    for consumer in [ComponentConsumer::Actor, ComponentConsumer::SystemExtension] {
        let fixture = ComponentFixture::new(consumer);
        let input = input_value();
        let outcome = execute_component(&fixture.request(&input));
        assert!(outcome.is_pass(), "{:?}", outcome.diagnostics);
        assert_eq!(outcome.output.as_ref(), Some(&input));
        assert_eq!(outcome.receipts.len(), 3);
        assert_eq!(outcome.receipts[0].input.stage, ComponentReceiptStage::Inspection);
        assert_eq!(outcome.receipts[1].input.stage, ComponentReceiptStage::Instantiation);
        assert_eq!(outcome.receipts[2].input.stage, ComponentReceiptStage::Execution);
        assert!(outcome.receipts.iter().all(|receipt| receipt.input.consumer == consumer));
    }
}

#[test]
fn identical_component_execution_replays_to_identical_canonical_evidence() {
    // r[verify molten.wasm_component.determinism]
    // r[verify molten.wasm_component.receipts]
    let fixture = ComponentFixture::new(ComponentConsumer::Actor);
    let input = input_value();
    let left = execute_component(&fixture.request(&input));
    let right = execute_component(&fixture.request(&input));
    assert!(left.is_pass(), "{:?}", left.diagnostics);
    assert!(right.is_pass(), "{:?}", right.diagnostics);
    assert_eq!(left.output, right.output);
    assert!(replay_receipts_match(&left.receipts, &right.receipts));
}

#[test]
fn component_shell_denies_malformed_wrong_world_and_over_resource_requests_with_receipts() {
    // r[verify molten.wasm_component.fixtures]
    // r[verify molten.wasm_component.resources]
    let mut malformed = ComponentFixture::new(ComponentConsumer::Actor);
    malformed.replace_component_bytes(b"not-a-component".to_vec());
    let input = input_value();
    let malformed_outcome = execute_component(&malformed.request(&input));
    assert!(!malformed_outcome.is_pass());
    assert_eq!(malformed_outcome.receipts.len(), 1);
    assert_eq!(malformed_outcome.receipts[0].input.stage, ComponentReceiptStage::Denial);

    let mut wrong_world = ComponentFixture::new(ComponentConsumer::Actor);
    wrong_world.facts.declared_world = "wrong-world".to_string();
    let wrong_world_outcome = execute_component(&wrong_world.request(&input));
    assert!(!wrong_world_outcome.is_pass());
    assert_eq!(wrong_world_outcome.receipts.len(), 1);

    let mut exhausted = ComponentFixture::new(ComponentConsumer::Actor);
    exhausted.facts.instances = exhausted.profile.resources.max_instances + 1;
    let exhausted_outcome = execute_component(&exhausted.request(&input));
    assert!(!exhausted_outcome.is_pass());
    assert_eq!(exhausted_outcome.receipts.len(), 1);
}

#[test]
fn production_loose_bytes_and_core_module_requests_never_fall_back() {
    // r[verify molten.wasm_component.materialization]
    // r[verify molten.wasm_component.migration]
    let fixture = ComponentFixture::new(ComponentConsumer::Actor);
    let input = input_value();
    let loose_request = ComponentExecutionRequest {
        profile: &fixture.profile,
        requested_profile: RequestedExecutionProfile::ComponentV1,
        evidence_scope: EvidenceScope::Production,
        source: ComponentArtifactSource::TestOnlyLoose {
            component_bytes: &fixture.component_bytes,
            wit_bytes: &fixture.wit_bytes,
        },
        facts: &fixture.facts,
        import_grants: &[],
        input: &input,
    };
    let loose = execute_component(&loose_request);
    assert!(!loose.is_pass());
    assert_eq!(loose.receipts.len(), 1);

    let mut core = ComponentFixture::new(ComponentConsumer::Actor);
    core.replace_component_bytes(wat::parse_str("(module)").expect("core module"));
    let denied = execute_component(&core.request(&input));
    assert!(!denied.is_pass());
    assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("fallback is forbidden")));
}

#[test]
fn component_shell_denies_invalid_preserves_output_and_fuel_exhaustion() {
    // r[verify molten.wasm_component.abi]
    // r[verify molten.wasm_component.resources]
    // r[verify molten.wasm_component.fixtures]
    let input = input_value();

    let mut invalid_output = ComponentFixture::new(ComponentConsumer::Actor);
    invalid_output.replace_component_bytes(invalid_output_component_bytes());
    let invalid = execute_component(&invalid_output.request(&input));
    assert!(!invalid.is_pass());
    assert!(invalid.diagnostics.iter().any(|diagnostic| diagnostic.contains("canonical Preserves")));
    assert_eq!(invalid.receipts.len(), 3);
    assert_eq!(invalid.receipts[0].input.stage, ComponentReceiptStage::Inspection);
    assert_eq!(invalid.receipts[1].input.stage, ComponentReceiptStage::Instantiation);
    assert_eq!(invalid.receipts[2].input.stage, ComponentReceiptStage::Denial);
    assert_eq!(invalid.receipts[2].input.trap_class.as_deref(), Some("invalid-preserves-payload"));

    let mut fuel_exhaustion = ComponentFixture::new(ComponentConsumer::Actor);
    fuel_exhaustion.replace_component_bytes(fuel_exhaustion_component_bytes());
    let exhausted = execute_component(&fuel_exhaustion.request(&input));
    assert!(!exhausted.is_pass());
    assert!(exhausted.diagnostics.iter().any(|diagnostic| diagnostic.contains("fuel")));
    assert_eq!(exhausted.receipts.len(), 3);
    assert_eq!(exhausted.receipts[2].input.stage, ComponentReceiptStage::Denial);
    assert_eq!(exhausted.receipts[2].input.trap_class.as_deref(), Some("fuel-exhausted"));
}

#[test]
fn component_shell_reinspects_resource_facts_and_rejects_actual_growth() {
    // r[verify molten.wasm_component.determinism]
    // r[verify molten.wasm_component.resources]
    let input = input_value();

    let mut forged = ComponentFixture::new(ComponentConsumer::Actor);
    forged.facts.memory.initial = 0;
    forged.facts.memory.maximum = Some(0);
    let forged_outcome = execute_component(&forged.request(&input));
    assert!(!forged_outcome.is_pass());
    assert!(forged_outcome.diagnostics.iter().any(|diagnostic| diagnostic.contains("differs")));

    let mut growing = ComponentFixture::new(ComponentConsumer::Actor);
    growing.replace_component_bytes(dynamic_growth_component_bytes());
    let growing_outcome = execute_component(&growing.request(&input));
    assert!(!growing_outcome.is_pass());
    assert!(growing_outcome.diagnostics.iter().any(|diagnostic| diagnostic.contains("nondeterministic growth")));

    let (over_limit_bytes, declared_bytes) = over_limit_memory_component_bytes();
    let mut over_limit = ComponentFixture::new(ComponentConsumer::Actor);
    over_limit.replace_component_bytes(over_limit_bytes);
    over_limit.facts.memory.initial = declared_bytes;
    over_limit.facts.memory.maximum = Some(declared_bytes);
    let over_limit_outcome = execute_component(&over_limit.request(&input));
    assert!(!over_limit_outcome.is_pass());
    assert!(over_limit_outcome.diagnostics.iter().any(|diagnostic| diagnostic.contains("resource bound")));
}

#[test]
fn untrusted_runtime_text_cannot_spoof_canonical_denial_classes() {
    // r[verify molten.wasm_component.receipts]
    let guest_denial = ComponentDenial::classified(
        ComponentDenialClass::GuestDenial,
        "component invoke denied: guest reported fuel exhaustion",
    );
    assert_eq!(guest_denial.canonical_class(), "guest-denial");

    let trap = ComponentDenial::classified(
        ComponentDenialClass::ComponentTrap,
        "component invoke trapped and fuel observation failed (diagnostic): invalid canonical Preserves",
    );
    assert_eq!(trap.canonical_class(), "component-trap");

    let exhausted = ComponentDenial::classified(
        ComponentDenialClass::FuelExhausted,
        "component fuel exhausted during invoke: diagnostic text",
    );
    assert_eq!(exhausted.canonical_class(), "fuel-exhausted");

    let unclassified = ComponentDenial::new("untrusted import name contains fuel, memory, and WIT");
    assert_eq!(unclassified.canonical_class(), "component-admission-denial");
}
