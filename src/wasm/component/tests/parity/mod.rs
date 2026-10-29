//! Frozen adoption corpus. The paired local/shared baseline that established
//! these expectations is archived with the pre-cutover local oracle under
//! `evidence/2026-10-29-cutover/`; each test now asserts the exact stages,
//! stable classes, outputs and ledgers that baseline fixed.

use super::super::*;

mod fixtures;
mod guest;
mod guestarena;
mod initialization;
mod rejection;
mod resources;

const SLICE_LIMIT: usize = 1_000;

fn run(fixture: &super::support::Fixture) -> ExecutionOutcome {
    let input = super::support::input_value();
    super::support::execute(&fixture.request(&input))
}

fn stages(outcome: &ExecutionOutcome) -> Vec<Stage> {
    outcome.receipts.iter().map(|receipt| receipt.input.stage).collect()
}

fn denial_class(outcome: &ExecutionOutcome) -> Option<&str> {
    outcome.receipts.last().and_then(|receipt| receipt.input.trap_class.as_deref())
}

fn continuation() -> ContinuationKey {
    ContinuationKey {
        service_id: "molten.shared-parity".to_string(),
        generation: 1,
        runnable_id: "callback".to_string(),
        occurrence_sequence: 1,
    }
}

fn slices(execution: &mut SlicedExecution<'_>) -> (Vec<SliceBoundary>, ExecutionOutcome) {
    let mut boundaries = Vec::with_capacity(SLICE_LIMIT);
    for _ in 0..SLICE_LIMIT {
        match execution.run_slice() {
            SliceProgress::Yielded { boundary, requeue } => {
                assert_eq!(requeue, continuation().requeue_command());
                boundaries.push(boundary);
            }
            SliceProgress::Completed(outcome) => return (boundaries, outcome),
        }
    }
    panic!("fixture exceeded bounded polls")
}

#[test]
fn success_preserves_output_cleanup_and_canonical_receipt_inputs() {
    // r[verify molten.wasm_component_adoption.parity]
    // r[verify molten.wasm_component_adoption.cutover]
    // r[verify molten.wasm_component_adoption.validation]
    for consumer in [Consumer::Actor, Consumer::SystemExtension] {
        for bytes in [
            super::support::identity_bytes(),
            super::support::cpu_heavy_bytes(3_000),
            fixtures::cleanup(),
        ] {
            let mut fixture = super::support::Fixture::new(consumer);
            fixture.replace_component_bytes(bytes);
            let outcome = run(&fixture);
            assert!(outcome.is_pass(), "{:?}", outcome.diagnostics);
            assert_eq!(outcome.output, Some(super::support::input_value()));
            assert_eq!(stages(&outcome), vec![Stage::Inspection, Stage::Instantiation, Stage::Execution]);
            validate_chain(&outcome.receipts).expect("canonical receipt chain");
            assert!(outcome.receipts.iter().all(|receipt| receipt.input.consumer == consumer));
        }
    }
}

#[test]
fn typed_failures_keep_their_phase_and_denial_receipts() {
    // r[verify molten.wasm_component_adoption.parity]
    // r[verify molten.wasm_component_adoption.validation]
    for (bytes, expected_stages, class) in [
        (fixtures::start_trap(), vec![Stage::Inspection, Stage::Denial], "component-instantiation-denied"),
        (
            fixtures::invoke_trap(),
            vec![Stage::Inspection, Stage::Instantiation, Stage::Denial],
            "component-trap",
        ),
        (
            fixtures::guest_denial(),
            vec![Stage::Inspection, Stage::Instantiation, Stage::Denial],
            "guest-denial",
        ),
        (
            super::support::invalid_output_bytes(),
            vec![Stage::Inspection, Stage::Instantiation, Stage::Denial],
            "invalid-preserves-payload",
        ),
        (
            super::support::fuel_exhaustion_bytes(),
            vec![Stage::Inspection, Stage::Instantiation, Stage::Denial],
            "fuel-exhausted",
        ),
        (
            fixtures::failed_growth(),
            vec![Stage::Inspection, Stage::Instantiation, Stage::Denial],
            "component-trap",
        ),
    ] {
        let mut fixture = super::support::Fixture::new(Consumer::Actor);
        fixture.replace_component_bytes(bytes);
        let outcome = run(&fixture);
        assert!(!outcome.is_pass());
        assert_eq!(outcome.output, None);
        assert_eq!(stages(&outcome), expected_stages);
        assert_eq!(denial_class(&outcome), Some(class));
        // Failed-run fuel is not a receipt input; do not erase successful fuel.
        assert_eq!(outcome.receipts.last().expect("denial").input.fuel_remaining, None);
    }
}

#[test]
fn admission_denies_wrong_world_stale_facts_and_core_modules_without_fallback() {
    // r[verify molten.wasm_component_adoption.parity]
    let mut wrong_world = super::support::Fixture::new(Consumer::Actor);
    wrong_world.facts.declared_world = "wrong-world".to_string();
    let mut stale_facts = super::support::Fixture::new(Consumer::Actor);
    stale_facts.facts.memory.initial = 0;
    let mut core = super::support::Fixture::new(Consumer::Actor);
    core.replace_component_bytes(wat::parse_str("(module)").expect("core fixture"));
    let mut undeclared = super::support::Fixture::new(Consumer::Actor);
    undeclared.facts.imports.push("wasi:cli/environment@0.2.6".to_string());
    for fixture in [wrong_world, stale_facts, core, undeclared] {
        let outcome = run(&fixture);
        assert!(!outcome.is_pass());
        assert_eq!(stages(&outcome), vec![Stage::Denial]);
    }
}

#[test]
fn retained_slices_keep_callback_ledger_receipts_and_single_publication() {
    // r[verify molten.wasm_component_adoption.parity]
    // r[verify molten.wasm_component_adoption.cutover]
    for (bytes, class) in [
        (super::support::cpu_heavy_bytes(3_000), None),
        (super::support::fuel_exhaustion_bytes(), Some("fuel-exhausted")),
        (super::support::invalid_output_bytes(), Some("invalid-preserves-payload")),
    ] {
        let mut fixture =
            super::support::Fixture::with_profile(Consumer::Actor, time_sliced().expect("sliced profile"));
        fixture.replace_component_bytes(bytes);
        let input = super::support::input_value();
        let request = fixture.sliced_request(&input);
        let engine = super::support::engine(&fixture);
        let mut first = engine
            .start_sliced(&request, continuation())
            .unwrap_or_else(|outcome| panic!("{:?}", outcome.diagnostics));
        let (boundaries, outcome) = slices(&mut first);
        assert_eq!(outcome.is_pass(), class.is_none(), "{:?}", outcome.diagnostics);
        assert_eq!(denial_class(&outcome), class);
        assert!(first.ledger().boundaries.starts_with(&boundaries));
        validate_chain(&outcome.receipts).expect("canonical receipt chain");
        // A completed turn never publishes a second outcome.
        assert!(matches!(first.run_slice(), SliceProgress::Completed(repeated) if !repeated.is_pass()));
        let ledger = first.ledger().clone();
        drop(first);
        // The same admitted callback replays to an identical ledger and outcome.
        let mut second = engine
            .start_sliced(&request, continuation())
            .unwrap_or_else(|outcome| panic!("{:?}", outcome.diagnostics));
        let (replayed_boundaries, replayed) = slices(&mut second);
        assert_eq!(replayed_boundaries, boundaries);
        assert_eq!(replayed, outcome);
        assert_eq!(second.ledger().compare_replay(&ledger), Ok(()));
    }
}

#[test]
fn two_admitted_per_memory_bounds_preserve_complete_execution() {
    // r[verify molten.wasm_component_adoption.parity]
    // r[verify molten.wasm_component.resources]
    for consumer in [Consumer::Actor, Consumer::SystemExtension] {
        let mut fixture = super::support::Fixture::new(consumer);
        fixture.replace_component_bytes(fixtures::aggregate_memory());
        // Inspection reports the largest declaration, not the sum of memories.
        fixture.facts.memory.initial = 129 * 65_536;
        fixture.facts.memory.maximum = Some(fixture.facts.memory.initial);
        fixture.facts.memories = 2;
        fixture.facts.instances = 2;
        let outcome = run(&fixture);
        assert!(outcome.is_pass(), "{:?}", outcome.diagnostics);
        assert_eq!(outcome.output, Some(super::support::input_value()));
        assert_eq!(stages(&outcome), vec![Stage::Inspection, Stage::Instantiation, Stage::Execution]);
        let fuel_remaining =
            outcome.receipts.last().expect("execution receipt").input.fuel_remaining.expect("success fuel");
        assert!(fuel_remaining < fixture.profile.resources.fuel);
    }
}

#[test]
fn invalid_oversized_output_keeps_pointer_trap_precedence() {
    let mut fixture = super::support::Fixture::new(Consumer::Actor);
    fixture.replace_component_bytes(fixtures::oversized_invalid_span());
    let outcome = run(&fixture);
    assert!(!outcome.is_pass());
    assert_eq!(denial_class(&outcome), Some("component-trap"));
}

#[test]
fn valid_oversized_output_keeps_resource_denial_precedence() {
    let mut fixture = super::support::Fixture::new(Consumer::Actor);
    fixture.replace_component_bytes(fixtures::oversized_valid_span());
    fixture.facts.memory.initial = 2 * 65_536;
    fixture.facts.memory.maximum = Some(fixture.facts.memory.initial);
    assert_eq!(denial_class(&run(&fixture)), Some("resource-denial"));
}

#[test]
fn retained_cancellation_releases_the_concurrency_permit() {
    let mut fixture = super::support::Fixture::with_profile(Consumer::Actor, time_sliced().expect("sliced profile"));
    fixture.replace_component_bytes(super::support::cpu_heavy_bytes(3_000));
    let input = super::support::input_value();
    let request = fixture.sliced_request(&input);
    let engine = super::support::engine(&fixture);
    let mut first = engine
        .start_sliced(&request, continuation())
        .unwrap_or_else(|outcome| panic!("{:?}", outcome.diagnostics));
    assert!(matches!(first.run_slice(), SliceProgress::Yielded { .. }));
    let rejected = match engine.start_sliced(&request, continuation()) {
        Ok(_) => panic!("retained invocation must own the only admitted concurrency slot"),
        Err(outcome) => outcome,
    };
    assert_eq!(denial_class(&rejected), Some("resource-denial"));
    drop(first);
    let mut restarted = engine
        .start_sliced(&request, continuation())
        .unwrap_or_else(|outcome| panic!("{:?}", outcome.diagnostics));
    let (_, completed) = slices(&mut restarted);
    assert!(completed.is_pass(), "{:?}", completed.diagnostics);
    assert_eq!(completed.output, Some(input.clone()));
}
