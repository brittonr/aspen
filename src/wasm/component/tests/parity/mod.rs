use super::super::*;

mod fixtures;
mod initialization;
mod rejection;
mod resources;

const DEADLINE_TICKS: u64 = 1_000;
const SLICE_LIMIT: usize = 1_000;

fn runtime(fixture: &super::support::Fixture) -> super::super::runtime::shared::EngineAdapter {
    super::super::runtime::shared::EngineAdapter::build(&fixture.profile).expect("candidate runtime")
}

fn compare(local: &ExecutionOutcome, shared: &ExecutionOutcome) {
    assert_eq!(local.decision, shared.decision);
    assert_eq!(local.output, shared.output);
    // Includes whole typed inputs, canonical payload, hash, stage parent and fuel.
    // Human diagnostics are intentionally outside canonical receipt authority.
    assert_eq!(local.receipts, shared.receipts);
    assert!(replay_receipts_match(&local.receipts, &shared.receipts));
}

fn compare_synchronous(fixture: &super::support::Fixture) -> (ExecutionOutcome, ExecutionOutcome) {
    let input = super::support::input_value();
    let request = fixture.request(&input);
    let local = execute(&request);
    let shared = runtime(fixture).execute(&request, DEADLINE_TICKS);
    compare(&local, &shared);
    (local, shared)
}

#[test]
fn shared_candidate_preserves_success_output_cleanup_and_canonical_receipt_inputs() {
    // r[verify molten.wasm_component_adoption.parity]
    for consumer in [Consumer::Actor, Consumer::SystemExtension] {
        for bytes in [
            super::support::identity_bytes(),
            super::support::cpu_heavy_bytes(3_000),
            fixtures::cleanup(),
        ] {
            let mut fixture = super::support::Fixture::new(consumer);
            fixture.replace_component_bytes(bytes);
            let (local, _) = compare_synchronous(&fixture);
            assert!(local.is_pass(), "{:?}", local.diagnostics);
        }
    }
}

#[test]
fn shared_candidate_preserves_typed_failure_phase_and_denial_receipts() {
    // r[verify molten.wasm_component_adoption.parity]
    for (bytes, stages, class) in [
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
        let (local, _) = compare_synchronous(&fixture);
        assert!(!local.is_pass());
        assert_eq!(local.receipts.iter().map(|receipt| receipt.input.stage).collect::<Vec<_>>(), stages);
        let denial = local.receipts.last().expect("denial");
        assert_eq!(denial.input.trap_class.as_deref(), Some(class));
        // Failed-run fuel is not a receipt input; do not erase successful fuel.
        assert_eq!(denial.input.fuel_remaining, None);
    }
}

#[test]
fn shared_candidate_reuses_product_admission_for_wrong_world_stale_facts_and_no_fallback() {
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
        let (local, _) = compare_synchronous(&fixture);
        assert!(!local.is_pass());
        assert_eq!(local.receipts.iter().map(|receipt| receipt.input.stage).collect::<Vec<_>>(), vec![Stage::Denial]);
    }
}

fn continuation() -> ContinuationKey {
    ContinuationKey {
        service_id: "molten.shared-parity".to_string(),
        generation: 1,
        runnable_id: "callback".to_string(),
        occurrence_sequence: 1,
    }
}

fn local_slices(execution: &mut SlicedExecution<'_>) -> (Vec<SliceBoundary>, ExecutionOutcome) {
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
    panic!("local fixture exceeded bounded polls")
}

fn shared_slices(
    execution: &mut super::super::runtime::shared::sliced::PendingInvocation<'_>,
) -> (Vec<SliceBoundary>, ExecutionOutcome) {
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
    panic!("shared fixture exceeded bounded polls")
}

#[test]
fn retained_shared_slices_preserve_callback_ledger_receipts_and_single_publication() {
    // r[verify molten.wasm_component_adoption.parity]
    for bytes in [
        super::support::cpu_heavy_bytes(3_000),
        super::support::fuel_exhaustion_bytes(),
        super::support::invalid_output_bytes(),
    ] {
        let mut fixture =
            super::support::Fixture::with_profile(Consumer::Actor, time_sliced().expect("sliced profile"));
        fixture.replace_component_bytes(bytes);
        let input = super::support::input_value();
        let request = fixture.sliced_request(&input);
        let mut local = start_sliced_execution(&request, continuation()).expect("local start");
        let mut shared = runtime(&fixture)
            .start_sliced(&request, continuation(), DEADLINE_TICKS)
            .unwrap_or_else(|outcome| panic!("{:?}", outcome.diagnostics));
        let (local_boundaries, local_outcome) = local_slices(&mut local);
        let (shared_boundaries, shared_outcome) = shared_slices(&mut shared);
        assert_eq!(local_boundaries, shared_boundaries);
        assert_eq!(local.ledger(), Some(shared.ledger()));
        compare(&local_outcome, &shared_outcome);
        assert!(matches!(shared.run_slice(), SliceProgress::Completed(outcome) if !outcome.is_pass()));
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
        let (local, shared) = compare_synchronous(&fixture);
        assert!(local.is_pass(), "{:?}", local.diagnostics);
        assert!(shared.is_pass(), "{:?}", shared.diagnostics);
        assert_eq!(local.output, Some(super::support::input_value()));
        assert_eq!(local.receipts.iter().map(|receipt| receipt.input.stage).collect::<Vec<_>>(), vec![
            Stage::Inspection,
            Stage::Instantiation,
            Stage::Execution
        ]);
        let fuel_remaining =
            local.receipts.last().expect("execution receipt").input.fuel_remaining.expect("success fuel");
        assert!(fuel_remaining < fixture.profile.resources.fuel);
    }
}

#[test]
fn invalid_oversized_output_preserves_pointer_trap_precedence() {
    let mut fixture = super::support::Fixture::new(Consumer::Actor);
    fixture.replace_component_bytes(fixtures::oversized_invalid_span());
    let (local, _) = compare_synchronous(&fixture);
    assert!(!local.is_pass());
    assert_eq!(local.receipts.last().expect("denial").input.trap_class.as_deref(), Some("component-trap"));
}

#[test]
fn valid_oversized_output_preserves_resource_denial_precedence() {
    let mut fixture = super::support::Fixture::new(Consumer::Actor);
    fixture.replace_component_bytes(fixtures::oversized_valid_span());
    fixture.facts.memory.initial = 2 * 65_536;
    fixture.facts.memory.maximum = Some(fixture.facts.memory.initial);
    let (local, _) = compare_synchronous(&fixture);
    assert_eq!(local.receipts.last().expect("denial").input.trap_class.as_deref(), Some("resource-denial"));
}

#[test]
fn retained_cancellation_releases_the_shared_concurrency_permit() {
    let mut fixture = super::support::Fixture::with_profile(Consumer::Actor, time_sliced().expect("sliced profile"));
    fixture.replace_component_bytes(super::support::cpu_heavy_bytes(3_000));
    let input = super::support::input_value();
    let request = fixture.sliced_request(&input);
    let runtime = runtime(&fixture);
    let mut first = runtime
        .start_sliced(&request, continuation(), DEADLINE_TICKS)
        .unwrap_or_else(|outcome| panic!("{:?}", outcome.diagnostics));
    assert!(matches!(first.run_slice(), SliceProgress::Yielded { .. }));
    let rejected = match runtime.start_sliced(&request, continuation(), DEADLINE_TICKS) {
        Ok(_) => panic!("retained invocation must own the only admitted concurrency slot"),
        Err(outcome) => outcome,
    };
    assert_eq!(rejected.receipts.last().expect("denial").input.trap_class.as_deref(), Some("resource-denial"));
    drop(first);
    let mut restarted = runtime
        .start_sliced(&request, continuation(), DEADLINE_TICKS)
        .unwrap_or_else(|outcome| panic!("{:?}", outcome.diagnostics));
    let (_, completed) = shared_slices(&mut restarted);
    let mut local = start_sliced_execution(&request, continuation()).expect("local start");
    let (_, expected) = local_slices(&mut local);
    compare(&expected, &completed);
}
