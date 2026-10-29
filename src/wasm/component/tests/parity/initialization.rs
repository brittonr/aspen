use super::*;

#[test]
fn admitted_start_work_keeps_callback_boundaries_receipts_and_ledger() {
    // r[verify molten.wasm_component_adoption.parity]
    for consumer in [Consumer::Actor, Consumer::SystemExtension] {
        let mut fixture =
            super::super::support::Fixture::with_profile(consumer, time_sliced().expect("sliced profile"));
        fixture.replace_component_bytes(fixtures::expensive_start());
        let input = super::super::support::input_value();
        let request = fixture.sliced_request(&input);
        let engine = super::super::support::engine(&fixture);
        let mut execution = engine
            .start_sliced(&request, continuation())
            .unwrap_or_else(|outcome| panic!("{:?}", outcome.diagnostics));
        let (boundaries, outcome) = slices(&mut execution);
        assert!(outcome.is_pass(), "{:?}", outcome.diagnostics);
        assert_eq!(stages(&outcome), vec![Stage::Inspection, Stage::Instantiation, Stage::Execution]);
        assert_eq!(outcome.output, Some(input.clone()));
        assert!(execution.ledger().boundaries.starts_with(&boundaries));
        let remaining = outcome.receipts.last().and_then(|receipt| receipt.input.fuel_remaining).expect("fuel");
        assert_eq!(execution.ledger().consumed_fuel, execution.ledger().total_budget_fuel - remaining);
    }
}

#[test]
fn sliced_start_failure_denies_before_exposing_a_callback() {
    // r[verify molten.wasm_component_adoption.parity]
    let mut fixture =
        super::super::support::Fixture::with_profile(Consumer::Actor, time_sliced().expect("sliced profile"));
    fixture.replace_component_bytes(fixtures::start_trap());
    let input = super::super::support::input_value();
    let request = fixture.sliced_request(&input);
    let engine = super::super::support::engine(&fixture);
    let denied = engine.start_sliced(&request, continuation()).err().expect("start denial");
    assert_eq!(stages(&denied), vec![Stage::Inspection, Stage::Denial]);
    assert_eq!(denial_class(&denied), Some("component-instantiation-denied"));
}
