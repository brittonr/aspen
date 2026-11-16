// r[verify molten.compute_capacity.durable_reconciliation]
// r[verify molten.compute_capacity.offers]
// r[verify molten.compute_capacity.reservation_authority]
// r[verify molten.compute_capacity.readback]
#[test]
fn parser_denies_wrong_label_arity_and_schema() {
    let offer_value = offer(&sample_offer()).unwrap().value;
    let allocation_value = allocation(&sample_allocation()).unwrap().value;
    let readback_value = readback(&sample_readback()).unwrap().value;
    assert!(
        parse_offer(&crate::preserves_rail::record(
            "compute-capacity-offer-v1",
            cloned_fields(&offer_value, OFFER_RECORD, 16)
        ))
        .is_err()
    );
    assert!(
        parse_allocation(&crate::preserves_rail::record(
            "compute-capacity-allocation-v1",
            cloned_fields(&allocation_value, ALLOCATION_RECORD, 24)
        ))
        .is_err()
    );
    assert!(
        parse_readback(&crate::preserves_rail::record(
            "compute-capacity-readback-v2",
            cloned_fields(&readback_value, READBACK_RECORD, 6)
        ))
        .is_err()
    );
    assert!(parse_offer(&crate::preserves_rail::record(OFFER_RECORD, vec![])).is_err());
    assert!(parse_allocation(&crate::preserves_rail::record(ALLOCATION_RECORD, vec![])).is_err());
    assert!(parse_readback(&crate::preserves_rail::record(READBACK_RECORD, vec![])).is_err());
    assert!(
        parse_offer(&replace(
            &offer_value,
            OFFER_RECORD,
            16,
            0,
            crate::preserves_rail::string("molten.compute.offer.v2")
        ))
        .is_err()
    );
    assert!(
        parse_allocation(&replace(
            &allocation_value,
            ALLOCATION_RECORD,
            24,
            0,
            crate::preserves_rail::string("molten.compute.reservation.v2")
        ))
        .is_err()
    );
}

// r[verify molten.compute_capacity.durable_reconciliation]
// r[verify molten.compute_capacity.offers]
// r[verify molten.compute_capacity.reservation_authority]
// r[verify molten.compute_capacity.readback]
#[test]
fn parser_denies_invalid_refs() {
    let offer_value = offer(&sample_offer()).unwrap().value;
    let allocation_value = allocation(&sample_allocation()).unwrap().value;
    let readback_value = readback(&sample_readback()).unwrap().value;
    assert!(
        parse_offer(&replace(&offer_value, OFFER_RECORD, 16, 4, crate::preserves_rail::string("provider-v1"))).is_err()
    );
    assert!(
        parse_allocation(&replace(
            &allocation_value,
            ALLOCATION_RECORD,
            24,
            17,
            crate::preserves_rail::string("grant-v1")
        ))
        .is_err()
    );
    assert!(
        parse_readback(&replace(
            &readback_value,
            READBACK_RECORD,
            6,
            4,
            crate::preserves_rail::sequence(vec![crate::preserves_rail::string("qualification-v1")])
        ))
        .is_err()
    );
}

// r[verify molten.compute_capacity.durable_reconciliation]
// r[verify molten.compute_capacity.offers]
// r[verify molten.compute_capacity.reservation_authority]
// r[verify molten.compute_capacity.readback]
#[test]
fn parser_denies_nested_shapes() {
    let offer_value = offer(&sample_offer()).unwrap().value;
    let allocation_value = allocation(&sample_allocation()).unwrap().value;
    let readback_value = readback(&sample_readback()).unwrap().value;
    assert!(
        parse_offer(&replace(
            &offer_value,
            OFFER_RECORD,
            16,
            13,
            crate::preserves_rail::record("compute-compatibility-v2", vec![])
        ))
        .is_err()
    );
    assert!(
        parse_allocation(&replace(
            &allocation_value,
            ALLOCATION_RECORD,
            24,
            18,
            crate::preserves_rail::sequence(vec![crate::preserves_rail::record("compute-claim-v1", vec![])])
        ))
        .is_err()
    );
    assert!(
        parse_readback(&replace(
            &readback_value,
            READBACK_RECORD,
            6,
            5,
            crate::preserves_rail::sequence(vec![crate::preserves_rail::record(
                "compute-allocation-readback-v1",
                vec![]
            )])
        ))
        .is_err()
    );
}

// r[verify molten.compute_capacity.durable_reconciliation]
// r[verify molten.compute_capacity.offers]
// r[verify molten.compute_capacity.reservation_authority]
// r[verify molten.compute_capacity.readback]
#[test]
fn parser_denies_invalid_scalar_fields() {
    let offer_value = offer(&sample_offer()).unwrap().value;
    let allocation_value = allocation(&sample_allocation()).unwrap().value;
    let readback_value = readback(&sample_readback()).unwrap().value;
    assert!(parse_offer(&replace(&offer_value, OFFER_RECORD, 16, 11, crate::preserves_rail::string("ready"))).is_err());
    assert!(
        parse_allocation(&replace(
            &allocation_value,
            ALLOCATION_RECORD,
            24,
            21,
            crate::preserves_rail::string("committed")
        ))
        .is_err()
    );
    assert!(
        parse_readback(&replace(&readback_value, READBACK_RECORD, 6, 2, crate::preserves_rail::string("true")))
            .is_err()
    );
}

// r[verify molten.compute_capacity.durable_reconciliation]
// r[verify molten.compute_capacity.offers]
// r[verify molten.compute_capacity.reservation_authority]
// r[verify molten.compute_capacity.readback]
#[test]
fn projection_denies_invalid_refs_and_oversized_sequences() {
    let mut unbound = sample_offer();
    unbound.provider_profile_ref = "provider-v1".into();
    assert!(offer(&unbound).is_err());
    let mut unbound = sample_allocation();
    unbound.grant_ref = "grant-v1".into();
    assert!(allocation(&unbound).is_err());
    let mut unbound = sample_readback();
    unbound.qualification_refs[0] = "qualification-v1".into();
    assert!(readback(&unbound).is_err());
    let mut invalid = sample_allocation();
    invalid.id = "different-operation".into();
    assert!(allocation(&invalid).is_err());
    let mut invalid = sample_allocation();
    invalid.claims.push(invalid.claims[0].clone());
    invalid.authority_ids.push("authority-chip".into());
    assert!(allocation(&invalid).is_err());
    let mut invalid = sample_allocation();
    invalid.kind = molten_core::compute_capacity::AllocationKind::ServingDeployment;
    invalid.serving_allocation_id = Some("nested-deployment".into());
    assert!(allocation(&invalid).is_err());
    let mut invalid = sample_readback();
    invalid.allocations[0].is_logically_fenced = false;
    assert!(readback(&invalid).is_err());

    let too_many = crate::preserves_rail::sequence(vec![
        crate::preserves_rail::string("chip");
        molten_core::compute_capacity::MAX_ITEMS + 1
    ]);
    let offer_value = offer(&sample_offer()).unwrap().value;
    assert!(parse_offer(&replace(&offer_value, OFFER_RECORD, 16, 14, too_many)).is_err());
    let mut too_many_readback = sample_readback();
    too_many_readback.allocations =
        vec![too_many_readback.allocations[0].clone(); molten_core::compute_capacity::MAX_ITEMS + 1];
    assert!(readback(&too_many_readback).is_err());
}