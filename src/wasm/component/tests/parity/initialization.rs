use super::*;

#[test]
fn admitted_start_work_preserves_callback_boundaries_receipts_and_ledger() {
    // r[verify molten.wasm_component_adoption.parity]
    for consumer in [Consumer::Actor, Consumer::SystemExtension] {
        let mut fixture =
            super::super::support::Fixture::with_profile(consumer, time_sliced().expect("sliced profile"));
        fixture.replace_component_bytes(fixtures::expensive_start());
        let input = super::super::support::input_value();
        let request = fixture.sliced_request(&input);
        let mut local = start_sliced_execution(&request, continuation()).expect("local start");
        let mut shared = runtime(&fixture)
            .start_sliced(&request, continuation(), DEADLINE_TICKS)
            .unwrap_or_else(|outcome| panic!("{:?}", outcome.diagnostics));
        let (local_boundaries, local_outcome) = local_slices(&mut local);
        let (shared_boundaries, shared_outcome) = shared_slices(&mut shared);
        assert!(local_outcome.is_pass(), "{:?}", local_outcome.diagnostics);
        assert!(shared_outcome.is_pass(), "{:?}", shared_outcome.diagnostics);
        compare(&local_outcome, &shared_outcome);
        assert_eq!(local_boundaries, shared_boundaries);
        assert_eq!(local.ledger(), Some(shared.ledger()));
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
    let local = start_sliced_execution(&request, continuation()).err().expect("local start denial");
    let shared = runtime(&fixture)
        .start_sliced(&request, continuation(), DEADLINE_TICKS)
        .err()
        .expect("shared start denial");
    compare(&local, &shared);
    assert_eq!(local.receipts.iter().map(|receipt| receipt.input.stage).collect::<Vec<_>>(), vec![
        Stage::Inspection,
        Stage::Denial
    ]);
    assert_eq!(
        local.receipts.last().expect("denial").input.trap_class.as_deref(),
        Some("component-instantiation-denied")
    );
}
