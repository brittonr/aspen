// r[verify molten.compute_capacity.durable_reconciliation]
// r[verify molten.compute_capacity.offers]
// r[verify molten.compute_capacity.backing_pools]
#[test]
fn offer_round_trip_and_canonical_packed_ref() {
    let input = sample_offer();
    let canonical = offer(&input).expect("canonical offer");
    assert_eq!(canonical.reference, crate::preserves_rail::canonical_hash(&canonical.value).unwrap());
    assert_eq!(parse_offer(&canonical.value).unwrap(), input);
    assert_eq!(offer(&parse_offer(&canonical.value).unwrap()).unwrap(), canonical);

    let mut unknown = input;
    unknown.compatibility.backend = None;
    unknown.compatibility.qualification_refs.clear();
    unknown.pools[0].observed_free = None;
    unknown.views[0].partition_id = Some("partition-a".into());
    unknown.health = molten_core::compute_capacity::Health::Unknown;
    assert_eq!(parse_offer(&offer(&unknown).unwrap().value).unwrap(), unknown);
}

// r[verify molten.compute_capacity.durable_reconciliation]
// r[verify molten.compute_capacity.reservation_authority]
// r[verify molten.compute_capacity.uncertainty_release]
#[test]
fn allocation_round_trip_and_packed_ref() {
    let input = sample_allocation();
    let canonical = allocation(&input).expect("canonical allocation");
    assert_eq!(canonical.reference, crate::preserves_rail::canonical_hash(&canonical.value).unwrap());
    assert_eq!(parse_allocation(&canonical.value).unwrap(), input);
    assert_eq!(allocation(&parse_allocation(&canonical.value).unwrap()).unwrap(), canonical);

    let mut dependent = input;
    dependent.serving_allocation_id = Some("deployment-a".into());
    dependent.status = molten_core::compute_capacity::ReservationStatus::Quarantined;
    dependent.enforcement = molten_core::compute_capacity::EnforcementStrength::BackendAcknowledged;
    assert_eq!(parse_allocation(&allocation(&dependent).unwrap().value).unwrap(), dependent);
    let mut committed = sample_allocation();
    committed.status = molten_core::compute_capacity::ReservationStatus::Active;
    committed.enforcement = molten_core::compute_capacity::EnforcementStrength::BackendAcknowledged;
    committed.activation_backend_ref = Some("backend-activation-a".into());
    assert_eq!(parse_allocation(&allocation(&committed).unwrap().value).unwrap(), committed);
    committed.activation_backend_ref = None;
    assert!(allocation(&committed).is_err());
}

// r[verify molten.compute_capacity.durable_reconciliation]
// r[verify molten.compute_capacity.readback]
#[test]
fn readback_round_trip_and_golden_packed_ref() {
    let input = sample_readback();
    let canonical = readback(&input).expect("canonical readback");
    assert_eq!(canonical.reference, GOLDEN_READBACK_REF);
    assert_eq!(crate::preserves_rail::canonical_hash(&canonical.value).unwrap(), GOLDEN_READBACK_REF);
    assert_eq!(parse_readback(&canonical.value).unwrap(), input);
    assert_eq!(readback(&parse_readback(&canonical.value).unwrap()).unwrap(), canonical);

    let mut uncertain = input;
    uncertain.is_fresh = false;
    uncertain.health = molten_core::compute_capacity::Health::Degraded;
    uncertain.allocations[0].status = molten_core::compute_capacity::ReservationStatus::Quarantined;
    uncertain.allocations[0].enforcement = molten_core::compute_capacity::EnforcementStrength::BackendFenced;
    assert_eq!(parse_readback(&readback(&uncertain).unwrap().value).unwrap(), uncertain);
}