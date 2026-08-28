use molten_core::prolly_map::*;

use super::*;

// r[verify molten.prolly_map.storage_boundary]
#[test]
fn unknown_publication_reconciles_once_without_blind_retry() {
    let plan = initial_plan();
    let expected = ExpectedProllyRoot {
        root_ref: None,
        generation: INITIAL_GENERATION,
    };

    let mut applied = MemoryPort::new(UnknownMode::AfterApply);
    let receipt = publish_prolly_edit(&mut applied, MAP_ID, &expected, &plan).expect("applied reconciliation");
    assert_eq!(receipt.status, ProllyPublicationStatus::AppliedAfterReconciliation);
    assert_eq!(applied.compare_calls, 1);

    let mut not_applied = MemoryPort::new(UnknownMode::BeforeApply);
    let receipt = publish_prolly_edit(&mut not_applied, MAP_ID, &expected, &plan).expect("negative reconciliation");
    assert_eq!(receipt.status, ProllyPublicationStatus::NotAppliedAfterReconciliation);
    assert_eq!(not_applied.compare_calls, 1);
}

#[test]
fn crossed_expected_root_and_receipt_overclaims_fail_closed() {
    let plan = initial_plan();
    let mut port = MemoryPort::new(UnknownMode::None);
    let crossed = ExpectedProllyRoot {
        root_ref: Some(RootRef::new(
            "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
        )),
        generation: FIRST_GENERATION,
    };
    assert!(matches!(
        publish_prolly_edit(&mut port, MAP_ID, &crossed, &plan),
        Err(ProllyServiceError::Domain(issues)) if issues.contains(&ProllyIssue::RootIdentityMismatch)
    ));

    let receipt = ProllyPublicationReceipt {
        schema: PROLLY_PUBLICATION_RECEIPT_SCHEMA.to_string(),
        map_id: MAP_ID.to_string(),
        prior_root_ref: None,
        next_root_ref: plan.next.snapshot.root.root_ref,
        generation: FIRST_GENERATION,
        staged_block_refs: Vec::new(),
        status: ProllyPublicationStatus::Applied,
        authorizes_future_mutation: true,
        deletion_authorized: false,
        non_claims: prolly_receipt_non_claims(),
    };
    assert!(canonical_prolly_publication_receipt(&receipt).is_err());
}
