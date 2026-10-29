use super::*;

fn assert_cleanup_denial(outcome: &ExecutionOutcome, class: &str) {
    assert!(!outcome.is_pass());
    assert_eq!(outcome.output, None);
    assert_eq!(outcome.receipts.iter().map(|record| record.input.stage).collect::<Vec<_>>(), vec![
        Stage::Inspection,
        Stage::Instantiation,
        Stage::Denial,
    ]);
    let denial = outcome.receipts.last().expect("cleanup denial");
    assert_eq!(denial.input.trap_class.as_deref(), Some(class));
    assert_eq!(denial.input.output_ref, None);
    assert_eq!(denial.input.fuel_remaining, None);
}

#[test]
fn post_return_failure_prevents_success_and_output_publication() {
    // r[verify molten.wasm_component_adoption.parity]
    // r[verify molten.wasm_component_adoption.validation]
    for consumer in [Consumer::Actor, Consumer::SystemExtension] {
        for (bytes, class) in [
            (fixtures::cleanup_trap(), "component-trap"),
            (fixtures::cleanup_exhaustion(), "fuel-exhausted"),
        ] {
            let mut fixture = super::super::support::Fixture::new(consumer);
            fixture.replace_component_bytes(bytes);
            assert_cleanup_denial(&run(&fixture), class);
        }
    }
}

#[test]
fn retained_post_return_failure_preserves_callback_accounting_without_success() {
    // r[verify molten.wasm_component_adoption.parity]
    let mut fixture =
        super::super::support::Fixture::with_profile(Consumer::SystemExtension, time_sliced().expect("sliced profile"));
    fixture.replace_component_bytes(fixtures::cleanup_exhaustion());
    let input = super::super::support::input_value();
    let request = fixture.sliced_request(&input);
    let engine = super::super::support::engine(&fixture);
    let mut execution = engine
        .start_sliced(&request, continuation())
        .unwrap_or_else(|outcome| panic!("{:?}", outcome.diagnostics));
    let (boundaries, outcome) = slices(&mut execution);
    assert!(execution.ledger().boundaries.starts_with(&boundaries));
    assert_cleanup_denial(&outcome, "fuel-exhausted");
}
